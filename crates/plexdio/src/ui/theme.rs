//! KruxSYS look: dark navy panels, thin blue-grey lines, a cyan signal accent and square corners.
//! Every colour comes from kruxsys.com's stylesheet; docs/brand.md lists the source of each token
//! and the contrast checks. Use these tokens instead of literal colours anywhere in the UI.

use egui::{Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle};

// ---- Surfaces ----
/// Darkest surface: sidebar, title bar, now-playing bar, input fields (site `--bg`).
pub const SIDEBAR: Color32 = Color32::from_rgb(0x07, 0x09, 0x0c);
/// Content background (site `--panel`).
pub const BG: Color32 = Color32::from_rgb(0x0c, 0x10, 0x16);
/// Windows, menus and popups (site `--panel`).
pub const BAR: Color32 = Color32::from_rgb(0x0c, 0x10, 0x16);
/// Cards and tiles; always drawn with a `LINE` border (site `--panel-2`).
pub const CARD: Color32 = Color32::from_rgb(0x11, 0x16, 0x20);
/// Hovered rows, cards and tiles (site `--line`). `TEXT_DIM` stays readable on it.
pub const CARD_HOVER: Color32 = Color32::from_rgb(0x1a, 0x21, 0x2c);
/// Resting fill of controls: buttons, chips, the library switcher (site `--line`).
pub const CONTROL: Color32 = Color32::from_rgb(0x1a, 0x21, 0x2c);
/// Hovered/pressed controls (site `--line-2`). Only `TEXT` labels on it, never `TEXT_DIM`.
pub const CONTROL_HOVER: Color32 = Color32::from_rgb(0x2a, 0x34, 0x42);
/// Selected rows: cyan at 10 % over `BG`. Always paired with [`selection_marker`].
pub const SELECTED: Color32 = Color32::from_rgb(0x0e, 0x24, 0x2c);

// ---- Lines ----
/// Panel and card borders, separators (site `--line`).
pub const LINE: Color32 = Color32::from_rgb(0x1a, 0x21, 0x2c);
/// Control borders, chart grid lines (site `--line-2`).
pub const LINE_2: Color32 = Color32::from_rgb(0x2a, 0x34, 0x42);

// ---- Text ----
pub const TEXT: Color32 = Color32::from_rgb(0xe8, 0xec, 0xf2);
/// Long secondary text (site `.prose p`).
pub const TEXT_SOFT: Color32 = Color32::from_rgb(0xc9, 0xd1, 0xdd);
/// Secondary text and idle icons (site `--muted`).
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x8e, 0x9b, 0xb0);

// ---- Accent ----
/// The KruxSYS signal cyan (site `--signal`).
pub const ACCENT: Color32 = Color32::from_rgb(0x22, 0xd3, 0xee);
/// Hovered accent fills (site `.button.primary:hover`).
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(0x67, 0xe8, 0xf9);
/// Faint cyan fill (site `--signal-dim`, opaque over `BG`). Carries `TEXT` or `ACCENT`, never `ON_ACCENT`.
pub const ACCENT_DIM: Color32 = Color32::from_rgb(0x0f, 0x2b, 0x34);
/// Text and glyphs on accent fills (site `.button.primary`).
pub const ON_ACCENT: Color32 = Color32::from_rgb(0x03, 0x10, 0x14);

// ---- Status (not in the KruxSYS stylesheet; from the Krunk site's dark scheme) ----
pub const ERROR: Color32 = Color32::from_rgb(0xff, 0x8a, 0x80);
pub const ON_ERROR: Color32 = Color32::from_rgb(0x2a, 0x07, 0x16);
pub const OK: Color32 = Color32::from_rgb(0x6f, 0xdc, 0x95);
/// Title-bar close button hover: the Windows 11 convention, not a brand colour.
pub const CLOSE_HOVER: Color32 = Color32::from_rgb(0xc4, 0x2b, 0x1c);

/// Darkened backdrop for text over the visualiser and Ride: `SIDEBAR` at `alpha` (0..1).
pub fn scrim(alpha: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(SIDEBAR.r(), SIDEBAR.g(), SIDEBAR.b(), (alpha.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// A square HUD panel over the visualiser or Ride: dark scrim with a thin border.
pub fn hud_panel(painter: &egui::Painter, rect: egui::Rect, alpha: f32) {
    painter.rect(rect, 0.0, scrim(0.72 * alpha), Stroke::new(1.0, LINE_2.gamma_multiply(alpha)), egui::StrokeKind::Inside);
}

// ---- Shape ----
/// Corner radius of controls, rows and art. Panels and cards are square (`0`).
pub const RADIUS: f32 = 2.0;
/// [`RADIUS`] for APIs that take whole-pixel radii.
pub const RADIUS_U8: u8 = 2;

pub fn apply(ctx: &egui::Context) {
    let mut style = (*ctx.global_style()).clone();
    style.text_styles = [
        (TextStyle::Heading, FontId::new(26.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(14.0, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(11.5, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace)),
    ]
    .into();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    style.spacing.scroll.bar_width = 8.0;

    let v = &mut style.visuals;
    *v = egui::Visuals::dark();
    v.panel_fill = BG;
    v.window_fill = BAR;
    v.window_stroke = Stroke::new(1.0, LINE_2);
    v.extreme_bg_color = SIDEBAR;
    v.faint_bg_color = CARD;
    v.code_bg_color = CARD;
    v.override_text_color = Some(TEXT);
    v.weak_text_color = Some(TEXT_DIM);
    v.hyperlink_color = ACCENT;
    v.error_fg_color = ERROR;
    v.selection.bg_fill = ACCENT.linear_multiply(0.45);
    v.selection.stroke = Stroke::new(1.0, ACCENT);
    v.text_cursor.stroke = Stroke::new(2.0, ACCENT);
    v.window_corner_radius = CornerRadius::same(RADIUS_U8);
    v.menu_corner_radius = CornerRadius::same(RADIUS_U8);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = BG;
    w.noninteractive.weak_bg_fill = BG;
    w.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
    w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_DIM);
    w.inactive.bg_fill = CONTROL;
    w.inactive.weak_bg_fill = CONTROL;
    w.inactive.bg_stroke = Stroke::new(1.0, LINE_2);
    w.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    w.hovered.bg_fill = CONTROL_HOVER;
    w.hovered.weak_bg_fill = CONTROL_HOVER;
    w.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    w.hovered.fg_stroke = Stroke::new(1.5, TEXT);
    w.active.bg_fill = CONTROL_HOVER;
    w.active.weak_bg_fill = CONTROL_HOVER;
    w.active.bg_stroke = Stroke::new(1.0, ACCENT);
    w.active.fg_stroke = Stroke::new(1.5, TEXT);
    w.open.bg_fill = CONTROL_HOVER;
    w.open.weak_bg_fill = CONTROL_HOVER;
    w.open.bg_stroke = Stroke::new(1.0, LINE_2);
    w.open.fg_stroke = Stroke::new(1.0, TEXT);
    for wv in [&mut w.noninteractive, &mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
        wv.corner_radius = CornerRadius::same(RADIUS_U8);
        wv.expansion = 0.0;
    }
    ctx.set_theme(egui::Theme::Dark);
    ctx.set_style_of(egui::Theme::Dark, style);
}

/// Paint a button's background and centred label (shared by the custom buttons below).
fn paint_button(ui: &egui::Ui, rect: egui::Rect, galley: std::sync::Arc<egui::Galley>, fill: Color32, stroke: Stroke, text: Color32) {
    let p = ui.painter();
    p.rect(rect, RADIUS, fill, stroke, egui::StrokeKind::Inside);
    p.galley_with_override_text_color(rect.center() - galley.size() / 2.0, galley, text);
}

fn button_rect(ui: &mut egui::Ui, text: &str, size: f32, strong: bool, min: egui::Vec2) -> (egui::Response, std::sync::Arc<egui::Galley>) {
    let mut rt = egui::RichText::new(text).size(size);
    if strong {
        rt = rt.strong();
    }
    let galley = egui::WidgetText::from(rt).into_galley(ui, Some(egui::TextWrapMode::Extend), f32::INFINITY, TextStyle::Button);
    let want = (galley.size() + egui::vec2(28.0, 12.0)).max(min);
    let (_, resp) = ui.allocate_exact_size(want, egui::Sense::click());
    (resp, galley)
}

/// Accent-filled button (dark text on cyan, as the site's primary button).
pub fn primary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let (resp, galley) = button_rect(ui, text, 15.0, true, egui::vec2(160.0, 38.0));
    let fill = if !ui.is_enabled() {
        ACCENT.gamma_multiply(0.4)
    } else if resp.hovered() {
        ACCENT_HOVER
    } else {
        ACCENT
    };
    paint_button(ui, resp.rect, galley, fill, Stroke::NONE, ON_ACCENT);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Outlined button: thin `LINE_2` border that turns cyan on hover.
pub fn secondary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let (resp, galley) = button_rect(ui, text, 14.0, false, egui::vec2(120.0, 34.0));
    let enabled = ui.is_enabled();
    let border = if enabled && resp.hovered() { ACCENT } else { LINE_2 };
    let fill = if enabled && resp.is_pointer_button_down_on() { CONTROL } else { Color32::TRANSPARENT };
    let text = if enabled { TEXT } else { TEXT_DIM };
    paint_button(ui, resp.rect, galley, fill, Stroke::new(1.0, border), text);
    resp
}

/// On/off switch: square track, square knob. Marks the response changed when flipped.
pub fn toggle(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let (rect, mut resp) = ui.allocate_exact_size(egui::vec2(38.0, 22.0), egui::Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let t = ui.ctx().animate_bool_responsive(resp.id, *on);
    let enabled = ui.is_enabled();
    let dim = |c: Color32| if enabled { c } else { c.gamma_multiply(0.4) };
    let (track, border, knob) = if *on {
        (if resp.hovered() { ACCENT_HOVER } else { ACCENT }, Stroke::NONE, ON_ACCENT)
    } else {
        let border = if resp.hovered() { ACCENT } else { LINE_2 };
        (CONTROL, Stroke::new(1.0, dim(border)), if resp.hovered() { TEXT } else { TEXT_DIM })
    };
    ui.painter().rect(rect, RADIUS, dim(track), border, egui::StrokeKind::Inside);
    let k = rect.height() - 8.0;
    let x = egui::lerp((rect.left() + 4.0 + k / 2.0)..=(rect.right() - 4.0 - k / 2.0), t);
    let knob_rect = egui::Rect::from_center_size(egui::pos2(x, rect.center().y), egui::vec2(k, k));
    ui.painter().rect_filled(knob_rect, 1.0, dim(knob));
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Square card frame with a thin border.
pub fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0, LINE))
        .corner_radius(CornerRadius::ZERO)
        .inner_margin(egui::Margin::same(24))
}

/// Border of a painted card or tile (see [`card`]).
pub fn card_border(painter: &egui::Painter, rect: egui::Rect) {
    painter.rect_stroke(rect, 0.0, Stroke::new(1.0, LINE), egui::StrokeKind::Inside);
}

/// Mark a selected/current row or nav item: a 2 px cyan bar on its left edge, so the state never
/// relies on the faint `SELECTED` fill alone.
pub fn selection_marker(painter: &egui::Painter, rect: egui::Rect) {
    let bar = egui::Rect::from_min_size(rect.left_top(), egui::vec2(2.0, rect.height()));
    painter.rect_filled(bar, 0.0, ACCENT);
}

/// The KruxSYS corner brackets: cyan L-shapes on the top-left and bottom-right corners.
pub fn brackets(painter: &egui::Painter, rect: egui::Rect, len: f32) {
    let s = Stroke::new(1.0, ACCENT);
    let (a, b) = (rect.left_top() + egui::vec2(0.5, 0.5), rect.right_bottom() - egui::vec2(0.5, 0.5));
    painter.line_segment([a, a + egui::vec2(len, 0.0)], s);
    painter.line_segment([a, a + egui::vec2(0.0, len)], s);
    painter.line_segment([b, b - egui::vec2(len, 0.0)], s);
    painter.line_segment([b, b - egui::vec2(0.0, len)], s);
}

/// Small rotating arc used as a busy indicator.
pub fn spinner(ui: &mut egui::Ui, size: f32) {
    ui.add(egui::Spinner::new().size(size).color(ACCENT));
}
