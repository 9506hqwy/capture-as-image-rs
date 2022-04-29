use super::memory_device_context::MemoryDeviceContext;
use log::trace;
use std::ops::Drop;
use windows::{
    core::{self},
    Win32::Foundation::HWND,
    Win32::Graphics::Gdi::{GetDC, GetWindowDC, ReleaseDC, HDC},
};

#[derive(Debug)]
pub struct DeviceContext(Option<HWND>, HDC, bool);

impl DeviceContext {
    pub fn fullscreen() -> Result<DeviceContext, core::Error> {
        Self::get(None, false)
    }

    pub fn get(hwnd: Option<HWND>, is_desktop: bool) -> Result<DeviceContext, core::Error> {
        // https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getdc
        let hdc = if is_desktop {
            trace!("{}", "GetWindowDC");
            unsafe { GetWindowDC(hwnd) }
        } else {
            trace!("{}", "GetDC");
            unsafe { GetDC(hwnd) }
        };
        if hdc.is_invalid() {
            return Err(core::Error::from_win32());
        }

        Ok(DeviceContext(hwnd, hdc, is_desktop))
    }

    pub fn handle(&self) -> HDC {
        self.1
    }

    pub fn offscreen(&self) -> Result<MemoryDeviceContext, core::Error> {
        MemoryDeviceContext::bitmap(self.0, self.1, self.2)
    }
}

impl Drop for DeviceContext {
    fn drop(&mut self) {
        // https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-releasedc
        trace!("{}", "ReleaseDC");
        let ret = unsafe { ReleaseDC(self.0, self.1) };
        if ret == 0 {
            panic!("ReleaseDC");
        }
    }
}
