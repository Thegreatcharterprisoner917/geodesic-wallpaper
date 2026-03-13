//! Win32 WorkerW/Progman trick.
//! Creates our own HWND with WS_CHILD from the start, parents to WorkerW.
//! All unsafe isolated here.

use windows::Win32::Foundation::{HWND, WPARAM, LPARAM, BOOL, LRESULT};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, SendMessageTimeoutW, EnumWindows, FindWindowExW,
    SetWindowPos, ShowWindow,
    CreateWindowExW, RegisterClassExW, DefWindowProcW, PostQuitMessage,
    WNDCLASSEXW, CS_HREDRAW, CS_VREDRAW,
    WS_CHILD, WS_VISIBLE, WS_EX_NOACTIVATE, WS_EX_TRANSPARENT,
    SMTO_NORMAL, SWP_NOACTIVATE, SWP_FRAMECHANGED,
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
        let progman = FindWindowW(windows::core::w!("Progman"), None).ok()?;
        if hwnd_null(progman) { return None; }

        // 2. Magic spawn message
        let _ = SendMessageTimeoutW(
            progman, 0x052C,
            WPARAM(0xD), LPARAM(0x1),
            SMTO_NORMAL, 1000, None,
        );

        // 3. Find WorkerW behind icons
        let _ = EnumWindows(Some(find_worker_w), LPARAM(0));
        let ww = WORKER_W.get()?;
        let worker = HWND(*ww as _);
        if hwnd_null(worker) { return None; }

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

        // 5. Create window as WS_CHILD of WorkerW from the start
        let hwnd = CreateWindowExW(
            WS_EX_NOACTIVATE | WS_EX_TRANSPARENT,
            class_name,
            windows::core::w!(""),
            WS_CHILD | WS_VISIBLE,
            0, 0, width, height,
            worker,   // parent = WorkerW
            None,
            hinstance,
            None,
        ).ok()?;
        if hwnd_null(hwnd) { return None; }

        // 6. Pin to bottom of Z-order, force repaint
        let _ = SetWindowPos(
            hwnd, HWND_BOTTOM,
            0, 0, width, height,
            SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = UpdateWindow(hwnd);

        log::info!("Wallpaper HWND created inside WorkerW: {:?}", hwnd);
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
