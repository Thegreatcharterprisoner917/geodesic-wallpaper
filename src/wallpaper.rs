//! Windows desktop wallpaper window creation.
//! Uses HWND_BOTTOM + WM_WINDOWPOSCHANGING to stay pinned below all apps
//! but above the desktop background — reliable on Windows 10/11.
//! All unsafe isolated here.

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::UpdateWindow;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, PostQuitMessage, RegisterClassExW, SetWindowPos, ShowWindow,
    CS_HREDRAW, CS_VREDRAW, HWND_BOTTOM, SWP_NOACTIVATE, SWP_NOZORDER, SW_SHOW, WINDOWPOS,
    WM_DESTROY, WM_WINDOWPOSCHANGING, WNDCLASSEXW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP,
    WS_VISIBLE,
};

fn hwnd_null(h: HWND) -> bool {
    h.0 as isize == 0
}

/// Create a borderless fullscreen window that stays below all other app windows.
/// Intercepts WM_WINDOWPOSCHANGING to prevent anything from raising it.
pub fn create_wallpaper_hwnd(width: i32, height: i32) -> Option<HWND> {
    unsafe {
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
        RegisterClassExW(&wc);

        let hwnd = match CreateWindowExW(
            WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
            class_name,
            windows::core::w!(""),
            WS_POPUP | WS_VISIBLE,
            0,
            0,
            width,
            height,
            None,
            None,
            hinstance,
            None,
        ) {
            Ok(h) => h,
            Err(e) => {
                tracing::error!("CreateWindowExW failed: {e}");
                return None;
            }
        };
        if hwnd_null(hwnd) {
            return None;
        }

        // Pin to bottom of Z-order — below all normal windows, above desktop
        let _ = SetWindowPos(hwnd, HWND_BOTTOM, 0, 0, width, height, SWP_NOACTIVATE);
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = UpdateWindow(hwnd);

        tracing::info!("Wallpaper window created: {:?}", hwnd);
        Some(hwnd)
    }
}

/// Window procedure — intercepts WM_WINDOWPOSCHANGING to stay pinned at HWND_BOTTOM.
unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_DESTROY {
            PostQuitMessage(0);
            return LRESULT(0);
        }

        // Every time Windows tries to change our Z-order, force it back to BOTTOM.
        if msg == WM_WINDOWPOSCHANGING {
            let pos = &mut *(lp.0 as *mut WINDOWPOS);
            pos.hwndInsertAfter = HWND_BOTTOM;
            // Allow size/move changes but lock Z-order
            pos.flags |= SWP_NOZORDER;
            return LRESULT(0);
        }

        DefWindowProcW(hwnd, msg, wp, lp)
    }
}
