//! Update checks in the app: the automatic daily check, "Check for updates" in Settings → About,
//! the update banner and the update action (see [`crate::update`]).
//!
//! - Automatic checks run off the UI thread, [`schedule::STARTUP_DELAY_SECS`] after start and
//!   then at most once a day; their failures are only logged.
//! - Manual checks show their result (or error) in About.
//! - Installed copies download the installer, verify it, run it silently and exit. Portable copies
//!   swap their exe and relaunch. A bare exe opens the download page. A failed update offers
//!   the download page instead.

use super::{setting_row, Msg, PlexdioApp};
use crate::paths::InstallMode;
use crate::ui::theme;
use crate::update::check::{CheckOutcome, Status};
use crate::update::{self, schedule, Channel, UpdateAction};
use egui::RichText;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// A newer release the server offered (`url` is `info.download_url` resolved against the
/// manifest URL).
pub type Found = update::PendingUpdate;

/// Bytes downloaded so far (shared with the worker).
#[derive(Default)]
pub struct Progress {
    done: AtomicU64,
    total: AtomicU64,
}

/// What a finished update worker prepared.
pub enum Applied {
    /// The verified installer, ready to run.
    Installer(PathBuf, String),
    /// The new exe is in place (portable); relaunch it.
    Swapped(PathBuf),
}

pub struct UpdateState {
    /// Base URL of the update server (`--update-url`, or plexdio.kruxsys.com).
    pub base_url: String,
    pub mode: InstallMode,
    started: Instant,
    /// A check is running (`true` = started by the user).
    checking: Option<bool>,
    /// After a failed automatic check: don't try again before this.
    retry_at: Option<Instant>,
    pub found: Option<Found>,
    /// Version whose banner the user closed.
    dismissed: Option<String>,
    /// An update is downloading or being applied.
    progress: Option<Arc<Progress>>,
    /// Result of the last manual check or update attempt for About: (text, is error).
    message: Option<(String, bool)>,
    /// Offer the download page (an in-app update failed or isn't possible).
    offer_page: bool,
}

impl UpdateState {
    pub fn new(base_url: Option<String>) -> Self {
        let base_url = base_url
            .map(|u| u.trim().trim_end_matches('/').to_string())
            .filter(|u| !u.is_empty())
            .unwrap_or_else(|| update::DEFAULT_BASE_URL.to_string());
        Self {
            base_url,
            mode: crate::paths::install_mode(),
            started: Instant::now(),
            checking: None,
            retry_at: None,
            found: None,
            dismissed: None,
            progress: None,
            message: None,
            offer_page: false,
        }
    }

    fn action(&self) -> UpdateAction {
        update::action_for(self.mode)
    }
}

/// "just now", "5 minutes ago", "3 hours ago", "2 days ago".
pub fn ago(now: u64, then: u64) -> String {
    let s = now.saturating_sub(then);
    let plural = |n: u64, unit: &str| format!("{n} {unit}{} ago", if n == 1 { "" } else { "s" });
    match s {
        0..=59 => "just now".into(),
        60..=3599 => plural(s / 60, "minute"),
        3600..=86_399 => plural(s / 3600, "hour"),
        _ => plural(s / 86_400, "day"),
    }
}

/// The About text for a check outcome.
fn outcome_text(o: &CheckOutcome) -> String {
    let current = env!("CARGO_PKG_VERSION");
    match (&o.status, &o.release) {
        (Status::NoReleasePublished, _) => "No release has been published yet".into(),
        (Status::UpToDate, _) => format!("Up to date ({current})"),
        (Status::UpdateAvailable, Some(r)) => format!("Plexdio {} is available", r.version),
        (Status::UpdateRequired, Some(r)) => format!("Plexdio {} is available (this version is no longer supported)", r.version),
        _ => "Up to date".into(),
    }
}

impl PlexdioApp {
    /// Called every frame: starts the automatic check when it is due and keeps the app waking
    /// up for the next one (also while it sits hidden in the tray).
    pub(super) fn poll_updates(&mut self, ctx: &egui::Context) {
        if self.updates.progress.is_some() {
            ctx.request_repaint_after(Duration::from_millis(200));
        }
        if !self.config.update_check || self.updates.checking.is_some() || self.updates.progress.is_some() {
            return;
        }
        let startup = Duration::from_secs(schedule::STARTUP_DELAY_SECS);
        let since_start = self.updates.started.elapsed();
        if since_start < startup {
            ctx.request_repaint_after(startup - since_start);
            return;
        }
        if let Some(at) = self.updates.retry_at {
            if Instant::now() < at {
                ctx.request_repaint_after(at - Instant::now());
                return;
            }
        }
        let now = schedule::now();
        if schedule::is_due(true, self.config.last_update_check, now) {
            self.start_update_check(false);
        } else {
            let wait = schedule::secs_until_due(self.config.last_update_check, now).clamp(1, schedule::POLL_SECS);
            ctx.request_repaint_after(Duration::from_secs(wait));
        }
    }

    pub(super) fn start_update_check(&mut self, manual: bool) {
        if self.updates.checking.is_some() {
            return;
        }
        self.updates.checking = Some(manual);
        if manual {
            self.updates.message = None;
            self.updates.offer_page = false;
        }
        let base = self.updates.base_url.clone();
        let channel = self.config.update_channel;
        let arch = update::arch_for(self.updates.mode);
        log::info!("update check ({}) at {base}, channel {}, package {arch}", if manual { "manual" } else { "automatic" }, channel.as_str());
        self.jobs.spawn("update-check", move || {
            let r = update::check::check(&base, channel, &update::version::current(), arch).map_err(|e| e.to_string());
            Msg::UpdateChecked(manual, r)
        });
    }

    pub(super) fn on_update_checked(&mut self, manual: bool, result: Result<CheckOutcome, String>) {
        self.updates.checking = None;
        match result {
            Ok(o) => {
                let text = outcome_text(&o);
                log::info!("update check: {text}");
                self.config.last_update_check = Some(schedule::now());
                self.config.last_update_result = Some(text.clone());
                self.save_config();
                self.updates.retry_at = None;
                self.updates.found = match (o.has_update(), o.release, o.download_url) {
                    (true, Some(info), Some(url)) => Some(Found { info, url, required: o.status == Status::UpdateRequired }),
                    _ => None,
                };
                self.config.pending_update = self.updates.found.clone();
                self.save_config();
                if manual {
                    // The result shows in the Updates row; a manual check also brings back a
                    // banner that was closed before.
                    self.updates.dismissed = None;
                }
            }
            Err(e) => {
                log::warn!("update check failed: {e}");
                if manual {
                    self.updates.message = Some((format!("Couldn't check for updates. {e}"), true));
                } else {
                    // Silent; try again in an hour.
                    self.updates.retry_at = Some(Instant::now() + Duration::from_secs(schedule::POLL_SECS));
                }
            }
        }
    }

    /// "Update now" / "Download".
    pub(super) fn start_update(&mut self) {
        let Some(found) = self.updates.found.clone() else { return };
        if self.updates.progress.is_some() {
            return;
        }
        self.updates.message = None;
        self.updates.offer_page = false;
        let action = self.updates.action();
        if action == UpdateAction::OpenDownloadPage {
            self.open_download_page();
            return;
        }
        let progress = Arc::new(Progress::default());
        progress.total.store(found.info.size, Ordering::Relaxed);
        self.updates.progress = Some(progress.clone());
        log::info!("update: downloading {} from {}", found.info.version, found.url);
        let (tx, ctx) = self.jobs.sender();
        let spawned = std::thread::Builder::new().name("update-apply".into()).spawn(move || {
            let mut last_paint = Instant::now();
            let mut report = |done: u64, _total: u64| {
                progress.done.store(done, Ordering::Relaxed);
                if last_paint.elapsed() > Duration::from_millis(150) {
                    last_paint = Instant::now();
                    ctx.request_repaint();
                }
            };
            let r = match action {
                UpdateAction::RunInstaller => update::installer::fetch(&found.info, &found.url, &mut report)
                    .map(|p| Applied::Installer(p, found.info.version.clone())),
                _ => match std::env::current_exe() {
                    Ok(exe) => update::portable::apply(&found.info, &found.url, &exe, &mut report).map(|_| Applied::Swapped(exe)),
                    Err(e) => Err(e.into()),
                },
            };
            let _ = tx.send(Msg::UpdateApplied(r.map_err(|e| e.to_string())));
            ctx.request_repaint();
        });
        if let Err(e) = spawned {
            self.updates.progress = None;
            self.updates.message = Some((format!("Couldn't start the update: {e}"), true));
        }
    }

    pub(super) fn on_update_applied(&mut self, result: Result<Applied, String>) {
        self.updates.progress = None;
        match result {
            Ok(Applied::Installer(setup, version)) => match update::installer::launch(&setup, &version) {
                Ok(()) => {
                    log::info!("update: installer started, exiting");
                    self.quitting = true;
                }
                Err(e) => self.update_failed(e.to_string()),
            },
            Ok(Applied::Swapped(exe)) => {
                let args = update::portable::relaunch_args(std::env::args().skip(1));
                match std::process::Command::new(&exe).args(&args).spawn() {
                    Ok(_) => {
                        log::info!("update: new exe started, exiting");
                        self.quitting = true;
                    }
                    Err(e) => {
                        let undo = update::portable::rollback(&exe);
                        log::warn!("update: relaunch failed ({e}); rollback: {undo:?}");
                        self.update_failed(format!("Couldn't start the new version: {e}"));
                    }
                }
            }
            Err(e) => self.update_failed(e),
        }
    }

    fn update_failed(&mut self, e: String) {
        log::warn!("update failed: {e}");
        self.updates.message = Some((format!("Update failed. {e}"), true));
        self.updates.offer_page = true;
    }

    fn open_download_page(&mut self) {
        let url = update::download_page(&self.updates.base_url);
        if !crate::win::open_url(&url) {
            self.updates.message = Some((format!("Couldn't open the browser. The download page is {url}"), true));
        }
    }

    fn open_release_notes(&mut self, version: &str) {
        let url = update::release_notes_page(&self.updates.base_url, version);
        if !crate::win::open_url(&url) {
            self.updates.message = Some((format!("Couldn't open the browser. The release notes are at {url}"), true));
        }
    }

    fn update_button_label(&self) -> &'static str {
        match self.updates.action() {
            UpdateAction::OpenDownloadPage => "Download",
            _ => "Update now",
        }
    }

    fn progress_text(&self) -> Option<String> {
        let p = self.updates.progress.as_ref()?;
        let (done, total) = (p.done.load(Ordering::Relaxed), p.total.load(Ordering::Relaxed).max(1));
        let version = self.updates.found.as_ref().map(|f| f.info.version.as_str()).unwrap_or("");
        Some(if done >= total {
            format!("Verifying and installing Plexdio {version}…")
        } else {
            format!("Downloading Plexdio {version}… {} %", done * 100 / total)
        })
    }

    /// The slim notice at the top of the content area while an update is available.
    pub(super) fn update_banner(&mut self, ui: &mut egui::Ui) {
        let Some(found) = self.updates.found.clone() else { return };
        if self.updates.dismissed.as_deref() == Some(found.info.version.as_str()) && self.updates.progress.is_none() {
            return;
        }
        let mut close = false;
        theme::card().inner_margin(egui::Margin::symmetric(12, 6)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (dot, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                ui.painter().rect_filled(dot, 0.0, theme::ACCENT);
                if let Some(text) = self.progress_text() {
                    ui.label(RichText::new(text).color(theme::TEXT));
                    return;
                }
                let required = if found.required { " This version is no longer supported." } else { "" };
                ui.label(RichText::new(format!("Plexdio {} is available.{required}", found.info.version)).color(theme::TEXT));
                if ui.link("Release notes").clicked() {
                    self.open_release_notes(&found.info.version);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if super::icons::button(ui, super::Icon::Close, 16.0, false).on_hover_text("Not now").clicked() {
                        close = true;
                    }
                    if theme::secondary_button(ui, self.update_button_label()).clicked() {
                        self.start_update();
                    }
                });
            });
        });
        if close {
            self.updates.dismissed = Some(found.info.version.clone());
        }
        ui.add_space(8.0);
    }

    /// The update rows of Settings → About.
    pub(super) fn settings_updates(&mut self, ui: &mut egui::Ui) {
        let mode = self.updates.mode;
        let where_text = match mode {
            InstallMode::Portable => format!("Settings and caches are in {}", crate::paths::config_dir().parent().map(|p| p.display().to_string()).unwrap_or_default()),
            InstallMode::Installed => "Installed for this user; settings are in your Windows profile".into(),
            InstallMode::Standalone => "Not installed: updates are downloaded from the website".into(),
            InstallMode::Custom => crate::paths::config_dir().display().to_string(),
        };
        setting_row(ui, "Mode", Some(&where_text), |ui| {
            ui.label(RichText::new(mode.label()).color(theme::TEXT_SOFT));
        });

        let last = match (self.config.last_update_check, &self.config.last_update_result) {
            (Some(t), Some(r)) => format!("{r} · checked {}", ago(schedule::now(), t)),
            _ => "Not checked yet".into(),
        };
        let checking = self.updates.checking == Some(true);
        let mut check = false;
        setting_row(ui, "Updates", Some(&last), |ui| {
            ui.add_enabled_ui(!checking && self.updates.progress.is_none(), |ui| {
                check = theme::secondary_button(ui, if checking { "Checking…" } else { "Check for updates" }).clicked();
            });
        });
        if check {
            self.start_update_check(true);
        }

        if let Some(found) = self.updates.found.clone() {
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                let (dot, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                ui.painter().rect_filled(dot, 0.0, theme::ACCENT);
                let date = found.info.release_date().map(|d| format!(" ({d})")).unwrap_or_default();
                ui.label(RichText::new(format!("Plexdio {}{date} is available", found.info.version)).strong().color(theme::ACCENT));
                if ui.link("Release notes").clicked() {
                    self.open_release_notes(&found.info.version);
                }
            });
            if let Some(text) = self.progress_text() {
                let p = self.updates.progress.as_ref().map(|p| p.done.load(Ordering::Relaxed) as f32 / p.total.load(Ordering::Relaxed).max(1) as f32);
                ui.add(egui::ProgressBar::new(p.unwrap_or(0.0)).desired_height(6.0).corner_radius(0.0));
                ui.label(RichText::new(text).small().color(theme::TEXT_DIM));
            } else {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if theme::primary_button(ui, self.update_button_label()).clicked() {
                        self.start_update();
                    }
                });
            }
        }
        if let Some((text, err)) = self.updates.message.clone() {
            ui.add_space(4.0);
            ui.add(egui::Label::new(RichText::new(text).small().color(if err { theme::ERROR } else { theme::TEXT_DIM })).wrap());
        }
        if self.updates.offer_page && ui.link("Open the download page").clicked() {
            self.open_download_page();
        }

        let mut auto = self.config.update_check;
        setting_row(ui, "Automatically check for updates", Some("Once a day. Sends only the version, channel and package type."), |ui| {
            theme::toggle(ui, &mut auto);
        });
        if auto != self.config.update_check {
            self.config.update_check = auto;
            self.save_config();
        }
        let mut channel = self.config.update_channel;
        setting_row(ui, "Update channel", Some("Beta also gets stable releases."), |ui| {
            egui::ComboBox::from_id_salt("update-channel").selected_text(channel.label()).width(90.0).show_ui(ui, |ui| {
                for c in [Channel::Stable, Channel::Beta] {
                    ui.selectable_value(&mut channel, c, c.label());
                }
            });
        });
        if channel != self.config.update_channel {
            self.config.update_channel = channel;
            self.updates.found = None;
            self.config.pending_update = None;
            self.save_config();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_times() {
        let t = 1_000_000;
        assert_eq!(ago(t, t), "just now");
        assert_eq!(ago(t + 59, t), "just now");
        assert_eq!(ago(t + 60, t), "1 minute ago");
        assert_eq!(ago(t + 300, t), "5 minutes ago");
        assert_eq!(ago(t + 7200, t), "2 hours ago");
        assert_eq!(ago(t + 86_400, t), "1 day ago");
        // A clock set back reads as "just now", not a huge number.
        assert_eq!(ago(t, t + 500), "just now");
    }

    #[test]
    fn about_texts() {
        let o = CheckOutcome { status: Status::NoReleasePublished, release: None, download_url: None };
        assert_eq!(outcome_text(&o), "No release has been published yet");
        let o = CheckOutcome { status: Status::UpToDate, release: None, download_url: None };
        assert!(outcome_text(&o).starts_with("Up to date"));
    }
}

#[cfg(test)]
mod launch_tests {
    use crate::ui::app::LaunchOptions;

    fn args(a: &[&str]) -> LaunchOptions {
        LaunchOptions::from_args(a.iter().map(|s| s.to_string()))
    }

    #[test]
    fn update_url_override_from_the_command_line() {
        assert_eq!(args(&["--update-url", "http://10.0.0.2:8080"]).update_url.as_deref(), Some("http://10.0.0.2:8080"));
        assert_eq!(args(&["--update-url=http://h:1/"]).update_url.as_deref(), Some("http://h:1/"));
        // The relaunch flag of a portable update is ignored by the option parser.
        assert_eq!(args(&["--after-update", "--update-url", "http://h"]).update_url.as_deref(), Some("http://h"));
    }

    #[test]
    fn the_base_url_defaults_to_the_website_and_is_normalised() {
        assert_eq!(super::UpdateState::new(None).base_url, "https://plexdio.kruxsys.com");
        assert_eq!(super::UpdateState::new(Some("http://h:1/ ".into())).base_url, "http://h:1");
        assert_eq!(super::UpdateState::new(Some("  ".into())).base_url, "https://plexdio.kruxsys.com");
    }
}
