pub mod error;

mod device_context;
mod memory_device_context;

use device_context::DeviceContext;
use error::Error;
use log::trace;
use std::string::String;
use windows::{
    Win32::Foundation::{BOOL, GetLastError, HWND, LPARAM, RECT},
    Win32::System::Console::GetConsoleWindow,
    Win32::UI::WindowsAndMessaging::{
        EnumWindows, FindWindowW, GetClientRect, GetParent, GetSystemMetrics, GetWindowInfo,
        GetWindowRect, GetWindowTextLengthW, GetWindowTextW, IsWindowVisible, SM_CXSCREEN,
        SM_CYSCREEN, SYSTEM_METRICS_INDEX, WINDOWINFO, WS_POPUP,
    },
    core::{self, HSTRING, PCWSTR},
};

pub fn capture_as_image(
    fullscreen: bool,
    window: Option<&str>,
    is_desktop: bool,
) -> Result<Vec<u8>, Error> {
    let mut is_windows_terminal = false;

    let window = if fullscreen {
        DeviceContext::fullscreen()?
    } else {
        let hwnd = get_console_window_handle(window)?;
        if let Some(parent) = hwnd.1 {
            // windows terminal のデバイスコンテキストを使って内部プロセスのデバイスコンテキストに関連する
            // キャプチャは取得できないため、フルスクリーンのキャプチャから windows terminal の
            // ウィンドウの座標を切り出してキャプチャとする。
            // そのためウィンドウが重なっている場合は表示されているウィンドウがキャプチャの対象となる。
            if !is_desktop {
                return Err(Error::from(
                    "Need to specify `--desktop` or `--fullscreen` option.",
                ));
            }

            is_windows_terminal = true;
            DeviceContext::get(Some(parent), is_desktop)?
        } else {
            DeviceContext::get(Some(hwnd.0), is_desktop)?
        }
    };

    let screen = window.offscreen()?;
    if is_windows_terminal {
        let full = DeviceContext::fullscreen()?;
        let rect = get_client_rect(window.window(), is_desktop)?;
        screen.clip_from(full.handle(), (rect.0, rect.1))?;
    } else {
        screen.copy_from(window.handle())?;
    }
    Ok(screen.as_bytes()?)
}

fn get_console_window_handle(
    window_name: Option<&str>,
) -> Result<(HWND, Option<HWND>), core::Error> {
    match window_name {
        Some(name) => {
            // https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-findwindoww
            trace!("{}({})", "FindWindowW", name);
            let h_name = HSTRING::from(name);
            let w_name = PCWSTR::from_raw(h_name.as_ptr());
            let hwnd = unsafe { FindWindowW(None, w_name) };
            hwnd.map(|h| (h, None))
        }
        _ => {
            // https://docs.microsoft.com/en-us/windows/console/getconsolewindow
            trace!("{}", "GetConsoleWindow");
            let hwnd = unsafe { GetConsoleWindow() };

            // https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getparent
            trace!("{}", "GetParent");
            let phwnd = unsafe { GetParent(hwnd) };

            if phwnd.is_ok() {
                phwnd.map(|p| (hwnd, Some(p)))
            } else {
                trace!("GetParent {:?}", phwnd.err());
                Ok((hwnd, None))
            }
        }
    }
}

fn get_client_rect(
    hwnd: Option<HWND>,
    is_desktop: bool,
) -> Result<(i32, i32, i32, i32), core::Error> {
    if hwnd.is_none() {
        let full_x = get_system_metrics(SM_CXSCREEN)?;
        let full_y = get_system_metrics(SM_CYSCREEN)?;
        return Ok((0, 0, full_x, full_y));
    }

    let mut rect = RECT::default();

    if is_desktop {
        // https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowrect
        trace!("{}", "GetWindowRect");
        unsafe { GetWindowRect(hwnd.unwrap(), &mut rect) }
    } else {
        // https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getclientrect
        trace!("{}", "GetClientRect");
        unsafe { GetClientRect(hwnd.unwrap(), &mut rect) }
    }?;

    Ok((rect.left, rect.top, rect.right, rect.bottom))
}

fn get_system_metrics(index: SYSTEM_METRICS_INDEX) -> Result<i32, core::Error> {
    // https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getsystemmetrics
    trace!("{}({:?})", "GetSystemMetrics", index);
    let ret = unsafe { GetSystemMetrics(index) };
    if ret == 0 {
        return Err(core::Error::from_win32());
    }

    Ok(ret)
}

pub fn print_window_name() -> Result<(), core::Error> {
    // https://docs.microsoft.com/ja-jp/windows/win32/api/winuser/nf-winuser-enumwindows
    trace!("{}", "EnumWindows");
    unsafe { EnumWindows(Some(print_window_name_proc), None) }
}

unsafe extern "system" fn print_window_name_proc(hwnd: HWND, _: LPARAM) -> BOOL {
    unsafe {
        // https://docs.microsoft.com/ja-jp/windows/win32/api/winuser/nf-winuser-iswindowvisible
        if !IsWindowVisible(hwnd).as_bool() {
            return true.into();
        }

        // https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowinfo
        let mut info = WINDOWINFO::default();
        if GetWindowInfo(hwnd, &mut info).is_err() {
            return true.into();
        }

        if (info.dwStyle & WS_POPUP) == WS_POPUP {
            return true.into();
        }

        // https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowtextlengthw
        let size = GetWindowTextLengthW(hwnd);
        if size == 0 {
            let err = GetLastError();
            if err.is_ok() {
                return true.into();
            }

            // 「Program Manager」というWindowタイトルをもつプログラムが
            // GetWindowInfo を実行した後の場合はエラーになる。
            // GetWindowInfo を実行しない場合はエラーは発生しない。
            return false.into();
        }

        let mut buffer: Vec<u16> = vec![0; (size + 1) as usize];

        // https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowtextw
        let ret = GetWindowTextW(hwnd, &mut buffer);
        if ret == 0 {
            return false.into();
        }

        println!("{}", String::from_utf16_lossy(&buffer[..ret as usize]));
        true.into()
    }
}
