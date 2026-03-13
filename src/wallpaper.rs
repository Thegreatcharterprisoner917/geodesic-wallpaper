//! Win32 WorkerW/Progman trick to parent our window behind desktop icons.
//! All unsafe is isolated here.

use windows::Win32::Foundation::{HWND, WPARAM, LPARAM, BOOL};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, SendMessageTimeoutW, EnumWindows,
    FindWindowExW, SetParent, GetWindowLongPtrW, SetWindowLongPtrW,
    SetWindowPos,
    SMTO_NORMAL, SWP_NOACTIVATE, SWP_SHOWWINDOW,
    GWL_STYLE, GWL_EXSTYLE,
    WS_CHILD, WS_VISIBLE, WS_POPUP, WS_CAPTION, WS_THICKFRAME, WS_OVERLAPPED,
    WS_EX_TOOLWINDOW, WS_EX_APPWINDOW,
    HWND_BOTTOM,
};
use std::sync::OnceLock;

static WORKER_W: OnceLock<isize> = OnceLock::new();

fn hwnd_is_null(h: &HWND) -> bool {
    h.0 as isize == 0
}

/// Attempt to set the render window as a child of the WorkerW (behind desktop icons).
/// Also fixes window styles so it truly sits behind icons without stealing focus.
/// Returns true on success.
pub fn attach_to_desktop(hwnd: HWND, width: i32, height: i32) -> bool {
    unsafe {
        // 1. Find Progman
        let progman = match FindWindowW(windows::core::w!("Progman"), None) {
            Ok(h) if !hwnd_is_null(&h) => h,
            _ => {
                log::warn!("Could not find Progman window");
                return false;
            }
        };

        // 2. Send magic 0x052C message to Progman — spawns a WorkerW behind icons
        let _ = SendMessageTimeoutW(
            progman,
            0x052C,
            WPARAM(0xD),
            LPARAM(0x1),
            SMTO_NORMAL,
            1000,
            None,
        );

        // 3. Enumerate top-level windows to find the WorkerW that has SHELLDLL_DefView
        let _ = EnumWindows(Some(find_worker_w), LPARAM(0));

        let worker = match WORKER_W.get() {
            Some(&ww) => HWND(ww as _),
            None => {
                log::warn!("WorkerW not found, running as normal window");
                return false;
            }
        };

        // 4. Strip popup/caption styles, apply WS_CHILD | WS_VISIBLE
        let mut style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        style &= !(WS_POPUP.0 | WS_CAPTION.0 | WS_THICKFRAME.0 | WS_OVERLAPPED.0);
        style |= WS_CHILD.0 | WS_VISIBLE.0;
        SetWindowLongPtrW(hwnd, GWL_STYLE, style as isize);

        // 5. Strip WS_EX_APPWINDOW (removes taskbar entry), keep it as tool window
        let mut ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        ex_style &= !WS_EX_APPWINDOW.0;
        ex_style |= WS_EX_TOOLWINDOW.0;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style as isize);

        // 6. Re-parent to WorkerW
        match SetParent(hwnd, worker) {
            Ok(prev) if !hwnd_is_null(&prev) || true => {
                log::info!("Attached to WorkerW successfully");
            }
            _ => {
                log::warn!("SetParent failed");
                return false;
            }
        }

        // 7. Position to fill the full screen inside WorkerW
        let _ = SetWindowPos(
            hwnd,
            HWND_BOTTOM,
            0, 0,
            width, height,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );

        true
    }
}

unsafe extern "system" fn find_worker_w(hwnd: HWND, _lparam: LPARAM) -> BOOL {
    unsafe {
        let shell_view = FindWindowExW(hwnd, None, windows::core::w!("SHELLDLL_DefView"), None);
        let shell_ok = shell_view.as_ref().map(|h| !hwnd_is_null(h)).unwrap_or(false);
        if shell_ok {
            let worker_w = FindWindowExW(None, hwnd, windows::core::w!("WorkerW"), None);
            if let Ok(ww) = worker_w {
                if !hwnd_is_null(&ww) {
                    let _ = WORKER_W.set(ww.0 as isize);
                }
            }
        }
        BOOL(1)
    }
}
