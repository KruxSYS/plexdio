//! Windows-specific integration. Every item has a no-op fallback on other platforms so the
//! app (and its tests) also build and run natively on Linux.

pub mod autostart;
pub mod smtc;
pub mod single_instance;
pub mod tray;

use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// Native window handle as an integer (HWND on Windows), if available.
pub fn hwnd_of(h: &impl HasWindowHandle) -> Option<isize> {
    match h.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(w) => Some(w.hwnd.get()),
        _ => None,
    }
}

/// Use the exe's icon resource (every ICO frame) for the window and taskbar icons, so Windows can
/// pick the pixel-aligned 16/24 px frames instead of shrinking the 64 px RGBA image the window was
/// created with. No-op on other platforms.
pub fn apply_window_icons(window: &winit::window::Window) {
    #[cfg(windows)]
    {
        use winit::platform::windows::{IconExtWindows, WindowExtWindows};
        let scale = window.scale_factor();
        let px = |n: f64| {
            let s = (n * scale).round() as u32;
            winit::dpi::PhysicalSize::new(s, s)
        };
        match winit::window::Icon::from_resource(1, Some(px(16.0))) {
            Ok(icon) => window.set_window_icon(Some(icon)),
            Err(e) => log::warn!("small window icon: {e}"),
        }
        match winit::window::Icon::from_resource(1, Some(px(32.0))) {
            Ok(icon) => window.set_taskbar_icon(Some(icon)),
            Err(e) => log::warn!("taskbar icon: {e}"),
        }
    }
    #[cfg(not(windows))]
    let _ = window;
}

/// True when running under Wine (ntdll exports `wine_get_version`).
pub fn is_wine() -> bool {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
        let name: Vec<u16> = "ntdll.dll".encode_utf16().chain(Some(0)).collect();
        let h = GetModuleHandleW(name.as_ptr());
        !h.is_null() && GetProcAddress(h, c"wine_get_version".as_ptr().cast()).is_some()
    }
    #[cfg(not(windows))]
    false
}

/// Wine workaround: hide winit's "thread event target" helper window.
///
/// winit keeps that 0x0 layered popup `WS_VISIBLE` and calls `RedrawWindow(RDW_INTERNALPAINT)` on
/// it for every message it receives. Wine never delivers the resulting `WM_PAINT`, so the thread's
/// `QS_PAINT` bit stays set, `MsgWaitForMultipleObjectsEx` never blocks, and the idle UI thread
/// spins a core. A hidden window does not take part in paint scheduling. On real Windows the
/// window is left alone, because winit relies on it during modal resize loops.
pub fn fix_wine_event_target() {
    #[cfg(windows)]
    unsafe {
        if !is_wine() {
            return;
        }
        use windows_sys::Win32::Foundation::{HWND, LPARAM};
        use windows_sys::Win32::System::Threading::GetCurrentThreadId;
        use windows_sys::Win32::UI::WindowsAndMessaging::{EnumThreadWindows, GetClassNameW, ShowWindow, SW_HIDE};
        unsafe extern "system" fn cb(hwnd: HWND, _: LPARAM) -> i32 {
            let mut cls = [0u16; 64];
            let n = unsafe { GetClassNameW(hwnd, cls.as_mut_ptr(), 64) };
            if String::from_utf16_lossy(&cls[..n.max(0) as usize]) == "Winit Thread Event Target" {
                unsafe { ShowWindow(hwnd, SW_HIDE) };
                log::info!("Wine detected: hid winit event-target window to avoid idle busy loop");
            }
            1
        }
        EnumThreadWindows(GetCurrentThreadId(), Some(cb), 0);
    }
}

/// Open a URL in the default browser.
pub fn open_url(url: &str) -> bool {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
        let op: Vec<u16> = "open".encode_utf16().chain(Some(0)).collect();
        let u: Vec<u16> = url.encode_utf16().chain(Some(0)).collect();
        let r = ShellExecuteW(std::ptr::null_mut(), op.as_ptr(), u.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL);
        r as isize > 32
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("xdg-open").arg(url).spawn().is_ok()
    }
}
