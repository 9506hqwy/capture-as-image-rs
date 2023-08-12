use log::trace;
use std::ffi::c_void;
use std::mem::size_of;
use std::ops::Drop;
use std::slice::from_raw_parts;
use windows::{
    core::{self, IntoParam},
    Win32::Foundation::{HWND, RECT},
    Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreatedHDC, DeleteDC, DeleteObject,
        GetDIBits, SelectObject, BITMAPFILEHEADER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
        CAPTUREBLT, DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ, ROP_CODE, SRCCOPY,
    },
    Win32::UI::WindowsAndMessaging::{
        GetClientRect, GetSystemMetrics, GetWindowRect, SM_CXSCREEN, SM_CYSCREEN,
        SYSTEM_METRICS_INDEX,
    },
};

#[derive(Debug)]
pub struct MemoryDeviceContext {
    height: i32,
    width: i32,
    memory: CreatedHDC,
    bitmap: HBITMAP,
    preobj: HGDIOBJ,
}

impl MemoryDeviceContext {
    pub fn bitmap(window: Option<HWND>, hdc: HDC, is_desktop: bool) -> Result<Self, core::Error> {
        let (height, width) = size(window, is_desktop)?;

        // https://docs.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-createcompatibledc
        trace!("{}", "CreateCompatibleDC");
        let memory = unsafe { CreateCompatibleDC(hdc) };
        if memory.is_invalid() {
            return Err(core::Error::from_win32());
        }

        // https://docs.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-createcompatiblebitmap
        trace!("{}", "CreateCompatibleBitmap");
        let bitmap = unsafe { CreateCompatibleBitmap(hdc, width, height) };
        if bitmap.is_invalid() {
            return Err(core::Error::from_win32());
        }

        let preobj = assign(memory, bitmap)?;

        Ok(MemoryDeviceContext {
            height,
            width,
            memory,
            bitmap,
            preobj,
        })
    }

    pub fn copy_from(&self, hdcsrc: HDC) -> Result<(), core::Error> {
        // https://docs.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-bitblt
        trace!("{}({:?})", "BitBlt", hdcsrc);
        unsafe {
            BitBlt(
                self.memory,
                0,
                0,
                self.width,
                self.height,
                hdcsrc,
                0,
                0,
                ROP_CODE(SRCCOPY.0 | CAPTUREBLT.0),
            )
            .ok()
        }?;

        Ok(())
    }

    pub fn as_bytes(&self) -> Result<Vec<u8>, core::Error> {
        let img_size = (self.height as u32) * (self.width as u32) * 4;
        let mut buffer: Vec<u8> = vec![0; img_size as usize];

        let mut info = BITMAPINFO::default();
        info.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = self.width;
        info.bmiHeader.biHeight = self.height;
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        info.bmiHeader.biSizeImage = 0;
        info.bmiHeader.biCompression = BI_RGB as u32;

        // https://docs.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-getdibits
        trace!("{}", "GetDIBits");
        let ret = unsafe {
            GetDIBits(
                self.memory,
                self.bitmap,
                0,
                self.height as u32,
                buffer.as_mut_ptr() as *mut c_void,
                &mut info,
                DIB_RGB_COLORS,
            )
        };
        if ret == 0 {
            return Err(core::Error::from_win32());
        }

        // https://docs.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-bitmapfileheader
        let header_size = (size_of::<BITMAPFILEHEADER>() + size_of::<BITMAPINFOHEADER>()) as u32;
        let file_header = BITMAPFILEHEADER {
            bfType: 0x4D42,
            bfSize: header_size + img_size,
            bfReserved1: 0,
            bfReserved2: 0,
            bfOffBits: header_size,
        };

        let mut bmp_bytes = vec![];

        let file_header_bytes = unsafe { bytes(&file_header) };
        bmp_bytes.extend_from_slice(file_header_bytes);

        let info_header_bytes = unsafe { bytes(&info) };
        bmp_bytes.extend_from_slice(info_header_bytes);

        bmp_bytes.extend_from_slice(&buffer);

        Ok(bmp_bytes)
    }
}

impl Drop for MemoryDeviceContext {
    fn drop(&mut self) {
        assign(self.memory, self.preobj).unwrap();

        // https://docs.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-deleteobject
        trace!("{}", "DeleteObject");
        let ret = unsafe { DeleteObject(self.bitmap) };
        if !ret.as_bool() {
            panic!("DeleteObject");
        }

        // https://docs.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-deletedc
        trace!("{}", "DeleteDC");
        let ret = unsafe { DeleteDC(self.memory) };
        if !ret.as_bool() {
            panic!("DeleteDC");
        }
    }
}

fn assign<'a>(
    hdc: impl IntoParam<'a, HDC>,
    h: impl IntoParam<'a, HGDIOBJ>,
) -> Result<HGDIOBJ, core::Error> {
    // https://docs.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-selectobject
    trace!("{}", "SelectObject");
    let obj = unsafe { SelectObject(hdc, h) };
    if obj.is_invalid() {
        return Err(core::Error::from_win32());
    }

    Ok(obj)
}

unsafe fn bytes<T: Sized>(p: &T) -> &[u8] {
    from_raw_parts((p as *const T) as *const u8, size_of::<T>())
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
        unsafe { GetWindowRect(hwnd, &mut rect).ok() }
    } else {
        // https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getclientrect
        trace!("{}", "GetClientRect");
        unsafe { GetClientRect(hwnd, &mut rect).ok() }
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

fn size(window: Option<HWND>, is_desktop: bool) -> Result<(i32, i32), core::Error> {
    let rect = get_client_rect(window, is_desktop)?;
    let width = rect.2 - rect.0;
    let height = rect.3 - rect.1;
    Ok((height, width))
}
