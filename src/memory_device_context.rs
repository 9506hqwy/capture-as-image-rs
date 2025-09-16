use super::get_client_rect;
use log::trace;
use std::ffi::c_void;
use std::mem::size_of;
use std::ops::Drop;
use std::slice::from_raw_parts;
use windows::{
    Win32::Foundation::HWND,
    Win32::Graphics::Gdi::{
        BI_RGB, BITMAPFILEHEADER, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CAPTUREBLT,
        CreateCompatibleBitmap, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject,
        GetDIBits, HBITMAP, HDC, HGDIOBJ, ROP_CODE, SRCCOPY, SelectObject,
    },
    core::{self, Param},
};

#[derive(Debug)]
pub struct MemoryDeviceContext {
    height: i32,
    width: i32,
    memory: HDC,
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
        }?;

        Ok(())
    }

    pub fn clip_from(&self, hdcsrc: HDC, coordinate: (i32, i32)) -> Result<(), core::Error> {
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
                coordinate.0,
                coordinate.1,
                ROP_CODE(SRCCOPY.0 | CAPTUREBLT.0),
            )
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
        info.bmiHeader.biCompression = BI_RGB.0;

        // https://docs.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-getdibits
        trace!("{}", "GetDIBits");
        let ret = unsafe {
            GetDIBits(
                self.memory,
                self.bitmap,
                0,
                self.height as u32,
                Some(buffer.as_mut_ptr() as *mut c_void),
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

fn assign(hdc: impl Param<HDC>, h: impl Param<HGDIOBJ>) -> Result<HGDIOBJ, core::Error> {
    // https://docs.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-selectobject
    trace!("{}", "SelectObject");
    let obj = unsafe { SelectObject(hdc, h) };
    if obj.is_invalid() {
        return Err(core::Error::from_win32());
    }

    Ok(obj)
}

unsafe fn bytes<T: Sized>(p: &T) -> &[u8] {
    unsafe { from_raw_parts((p as *const T) as *const u8, size_of::<T>()) }
}

fn size(window: Option<HWND>, is_desktop: bool) -> Result<(i32, i32), core::Error> {
    let rect = get_client_rect(window, is_desktop)?;
    let width = rect.2 - rect.0;
    let height = rect.3 - rect.1;
    Ok((height, width))
}
