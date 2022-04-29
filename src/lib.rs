pub mod error;

mod device_context;
mod memory_device_context;

use device_context::DeviceContext;
use error::Error;
use log::trace;
use std::string::String;
use windows::{
    core::{self},
    Win32::Foundation::{GetLastError, BOOL, HWND, LPARAM},
    Win32::System::Console::GetConsoleWindow,
    Win32::UI::WindowsAndMessaging::{
        EnumWindows, FindWindowW, GetWindowInfo, GetWindowTextLengthW, GetWindowTextW,
        IsWindowVisible, WINDOWINFO, WINDOW_STYLE, WS_POPUP,
    },
};

pub fn capture_as_image(
    fullscreen: bool,
    window: Option<&str>,
    is_desktop: bool,
) -> Result<Vec<u8>, Error> {
    let window = if fullscreen {
        DeviceContext::fullscreen()?
    } else {
        let hwnd = get_console_window_handle(window)?;
        DeviceContext::get(Some(hwnd), is_desktop)?
    };

    let screen = window.offscreen()?;
    screen.copy_from(window.handle())?;
    Ok(screen.as_bytes()?)
}

fn get_console_window_handle(window_name: Option<&str>) -> Result<HWND, core::Error> {
    let hwnd = match window_name {
        Some(name) => {
            // https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-findwindoww
            trace!("{}({})", "FindWindowW", name);
            unsafe { FindWindowW(None, name) }
        }
        _ => {
            // https://docs.microsoft.com/en-us/windows/console/getconsolewindow
            trace!("{}", "GetConsoleWindow");
            unsafe { GetConsoleWindow() }
        }
    };
    if hwnd.0 == 0 {
        return Err(core::Error::from_win32());
    }

    Ok(hwnd)
}

pub fn print_window_name() -> Result<(), core::Error> {
    // https://docs.microsoft.com/ja-jp/windows/win32/api/winuser/nf-winuser-enumwindows
    trace!("{}", "EnumWindows");
    unsafe { EnumWindows(Some(print_window_name_proc), None).ok() }
}

unsafe extern "system" fn print_window_name_proc(hwnd: HWND, _: LPARAM) -> BOOL {
    // https://docs.microsoft.com/ja-jp/windows/win32/api/winuser/nf-winuser-iswindowvisible
    if !IsWindowVisible(hwnd).as_bool() {
        return true.into();
    }

    // https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowinfo
    let mut info = WINDOWINFO::default();
    if !GetWindowInfo(hwnd, &mut info).as_bool() {
        return true.into();
    }

    if (WINDOW_STYLE(info.dwStyle) & WS_POPUP) == WS_POPUP {
        return true.into();
    }

    // https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowtextlengthw
    let size = GetWindowTextLengthW(hwnd);
    if size == 0 {
        let err = GetLastError();
        if err.0 == 0 {
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
