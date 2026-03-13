//! Win32 WorkerW/Progman trick.
//! Creates our own HWND with WS_CHILD from the start, parents to WorkerW.
//! All unsafe isolated here.

use windows::Win32::Foundation::{HWND, WPARAM, LPARAM, BOOL, LRESULT};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, SendMessageTimeoutW, EnumWindows, FindWindowExW,
    SetWindowPos, ShowWindow, SetParent,
    GetWindowLongPtrW, SetWindowLongPtrW,
    CreateWindowExW, RegisterClassExW, DefWindowProcW, PostQuitMessage,
    WNDCLASSEXW, CS_HREDRAW, CS_VREDRAW,
    WS_POPUP, WS_VISIBLE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    GWL_STYLE, GWL_EXSTYLE,
    WS_CAPTION, WS_THICKFRAME, WS_OVERLAPPED,
    SMTO_NORMAL, SWP_NOACTIVATE, SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOSIZE,
    HWND_BOTTOM, SW_SHOW,
    WM_DESTROY,
};
use windows::Win32::Graphics::Gdi::UpdateWindow;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use std::sync::OnceLock;

static WORKER_W: OnceLock<isize> = OnceLock::new();

fn hwnd_null(h: HWND) -> bool { h.0 as isize == 0 }

/// Find WorkerW, create a WS_CHILD window inside it, return the HWND.
/// Returns None if WorkerW trick fails (caller should create a normal window).
pub fn create_wallpaper_hwnd(width: i32, height: i32) -> Option<HWND> {
    unsafe {
        // 1. Find Progman
        let progman = match FindWindowW(windows::core::w!("Progman"), None) {
            Ok(h) => { log::info!("Progman found: {:?}", h); h }
            Err(e) => { log::error!("FindWindowW(Progman) failed: {e}"); return None; }
        };
        if hwnd_null(progman) { log::error!("Progman HWND is null"); return None; }

        // 2. Magic spawn message
        let _ = SendMessageTimeoutW(
            progman, 0x052C,
            WPARAM(0xD), LPARAM(0x1),
            SMTO_NORMAL, 1000, None,
        );
        log::info!("Sent 0x052C to Progman");

        // 3. Find WorkerW behind icons.
        // On Windows 11, SHELLDLL_DefView may be a direct child of Progman (not of a WorkerW),
        // so we try two strategies:
        //   A) EnumWindows looking for a top-level WorkerW whose child is SHELLDLL_DefView
        //   B) Check if Progman itself contains SHELLDLL_DefView, then find any WorkerW sibling
        let _ = EnumWindows(Some(find_worker_w), LPARAM(0));

        // Strategy B: SHELLDLL_DefView inside Progman directly (Windows 11)
        if WORKER_W.get().is_none() {
            log::info!("Strategy A failed, trying strategy B (Win11 Progman-direct)");
            let shell = FindWindowExW(progman, None, windows::core::w!("SHELLDLL_DefView"), None);
            let shell_found = shell.as_ref().map(|h| !hwnd_null(*h)).unwrap_or(false);
            log::info!("Strategy B: SHELLDLL_DefView in Progman = {shell_found}");
            if shell_found {
                // The WorkerW that was spawned by 0x052C is a sibling of Progman at top level.
                // Iterate through all top-level WorkerW windows to find the right one.
                let _ = EnumWindows(Some(find_any_worker_w), LPARAM(0));
            }
        }

        // Strategy C: just enumerate all WorkerW windows and use the first
        if WORKER_W.get().is_none() {
            log::info!("Strategy C: looking for any WorkerW");
            let _ = EnumWindows(Some(find_any_worker_w), LPARAM(0));
        }

        let ww = match WORKER_W.get() {
            Some(v) => { log::info!("WorkerW found: 0x{:x}", v); v }
            None => { log::error!("WorkerW not found after all strategies"); return None; }
        };
        let worker = HWND(*ww as _);
        if hwnd_null(worker) { log::error!("WorkerW HWND is null"); return None; }

        // 4. Register a minimal window class
        let hinstance = GetModuleHandleW(None).ok()?;
        let class_name = windows::core::w!("GeodesicWallpaper");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinstance.into(),
            lpszClassName: class_name,
            ..Default::default()
        };
        RegisterClassExW(&wc); // ignore error if already registered

        // 5. Create as a top-level WS_POPUP first (cross-process CreateWindowExW with
        //    a foreign parent is denied by Windows; SetParent is allowed cross-process).
        let hwnd = match CreateWindowExW(
            WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
            class_name,
            windows::core::w!(""),
            WS_POPUP | WS_VISIBLE,
            0, 0, width, height,
            None,   // no parent yet
            None,
            hinstance,
            None,
        ) {
            Ok(h) => { log::info!("CreateWindowExW ok: {:?}", h); h }
            Err(e) => { log::error!("CreateWindowExW failed: {e}"); return None; }
        };
        if hwnd_null(hwnd) { log::error!("Created HWND is null"); return None; }

        // 6. Now reparent into WorkerW (SetParent is allowed cross-process)
        match SetParent(hwnd, worker) {
            Ok(_) => log::info!("SetParent to WorkerW ok"),
            Err(e) => { log::error!("SetParent failed: {e}"); return None; }
        }

        // 7. Strip caption/border styles left from WS_POPUP, add toolbar hint
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        let style = style & !(WS_CAPTION.0 | WS_THICKFRAME.0 | WS_OVERLAPPED.0);
        SetWindowLongPtrW(hwnd, GWL_STYLE, style as isize);

        // 8. Force position + send to bottom of WorkerW's Z-order
        let _ = SetWindowPos(
            hwnd, HWND_BOTTOM,
            0, 0, width, height,
            SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = UpdateWindow(hwnd);

        log::info!("Wallpaper attached to WorkerW successfully");
        Some(hwnd)
    }
}

/// Fallback: create a borderless fullscreen top-level window (not behind icons).
pub fn create_fallback_hwnd(width: i32, height: i32) -> Option<HWND> {
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{WS_POPUP, WS_EX_TOPMOST};
        let hinstance = GetModuleHandleW(None).ok()?;
        let class_name = windows::core::w!("GeodesicWallpaperFallback");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinstance.into(),
            lpszClassName: class_name,
            ..Default::default()
        };
        RegisterClassExW(&wc);
        let hwnd = CreateWindowExW(
            WS_EX_NOACTIVATE,
            class_name,
            windows::core::w!("Geodesic Wallpaper"),
            WS_POPUP | WS_VISIBLE,
            0, 0, width, height,
            None, None, hinstance, None,
        ).ok()?;
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = UpdateWindow(hwnd);
        Some(hwnd)
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_DESTROY {
        unsafe { PostQuitMessage(0); }
        return LRESULT(0);
    }
    unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
}

unsafe extern "system" fn find_worker_w(hwnd: HWND, _lp: LPARAM) -> BOOL {
    unsafe {
        let shell = FindWindowExW(hwnd, None, windows::core::w!("SHELLDLL_DefView"), None);
        let found = shell.as_ref().map(|h| !hwnd_null(*h)).unwrap_or(false);
        if found {
            let ww = FindWindowExW(None, hwnd, windows::core::w!("WorkerW"), None);
            if let Ok(w) = ww {
                if !hwnd_null(w) {
                    let _ = WORKER_W.set(w.0 as isize);
                }
            }
        }
        BOOL(1)
    }
}

/// Find the empty WorkerW (the one spawned by 0x052C, has no SHELLDLL_DefView child)
unsafe extern "system" fn find_any_worker_w(hwnd: HWND, _lp: LPARAM) -> BOOL {
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetWindow, GW_CHILD};
        let mut buf = [0u16; 64];
        let len = GetClassNameW(hwnd, &mut buf);
        if len > 0 {
            let name = String::from_utf16_lossy(&buf[..len as usize]);
            if name == "WorkerW" {
                // Check: does this WorkerW have a SHELLDLL_DefView child? If yes, skip it.
                let child = FindWindowExW(hwnd, None, windows::core::w!("SHELLDLL_DefView"), None);
                let has_shell = child.as_ref().map(|h| !hwnd_null(*h)).unwrap_or(false);
                log::info!("WorkerW candidate {:?} has_shell_view={}", hwnd, has_shell);
                if !has_shell && WORKER_W.get().is_none() {
                    let _ = WORKER_W.set(hwnd.0 as isize);
                    log::info!("Selected WorkerW (empty): {:?}", hwnd);
                    return BOOL(0); // stop
                }
            }
        }
        BOOL(1)
    }
}
