use std::ffi::c_void;
use std::ptr::null;
use skia_safe::{surfaces, AlphaType, Canvas, ColorType, ImageInfo};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

pub struct NotchWindow {
    pub hwnd: HWND,
    pub hdc_mem: HDC,
    pub hbitmap: HBITMAP,
    pub pixel_ptr: *mut c_void,
    pub width: i32,
    pub height: i32,
    pub x: i32,
    pub y: i32,
}

impl NotchWindow {
    pub fn new(width: i32, height: i32, top_padding: i32) -> Self {
        unsafe {
            let class_name = to_wide("optinotch");
            let window_title = to_wide("OptiNotch");

            let wnd_class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(wnd_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: 0 as _,
                hIcon: 0 as _,
                hCursor: LoadCursorW(0 as _, IDC_ARROW),
                hbrBackground: 0 as _, // Transparent layered window
                lpszMenuName: null(),
                lpszClassName: class_name.as_ptr(),
                hIconSm: 0 as _,
            };

            RegisterClassExW(&wnd_class);

            let screen_width = GetSystemMetrics(SM_CXSCREEN);
            let x = (screen_width - width) / 2;
            let y = top_padding;

            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                class_name.as_ptr(),
                window_title.as_ptr(),
                WS_POPUP | WS_VISIBLE,
                x,
                y,
                width,
                height,
                0 as _,
                0 as _,
                0 as _,
                null(),
            );

            if hwnd == 0 as _ {
                panic!("[!] Failed to create window");
            }

            // 1. Create In-Memory Device Context
            let hdc_screen = GetDC(0 as _);
            let hdc_mem = CreateCompatibleDC(hdc_screen);
            ReleaseDC(0 as _, hdc_screen);

            // 2. Setup 32-bit BGRA Top-Down DIB
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height, // Negative = top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB,
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [RGBQUAD {
                    rgbBlue: 0,
                    rgbGreen: 0,
                    rgbRed: 0,
                    rgbReserved: 0,
                }],
            };

            let mut pixel_ptr: *mut c_void = std::ptr::null_mut();
            let hbitmap = CreateDIBSection(
                hdc_mem,
                &bmi,
                DIB_RGB_COLORS,
                &mut pixel_ptr,
                0 as _,
                0,
            );

            if hbitmap == 0 as _ || pixel_ptr.is_null() {
                panic!("[!] Failed to create DIB section");
            }

            SelectObject(hdc_mem, hbitmap);

            Self {
                hwnd,
                hdc_mem,
                hbitmap,
                pixel_ptr,
                width,
                height,
                x,
                y,
            }
        }
    }

    pub fn render<F>(&mut self, draw_fn: F)
    where
        F: FnOnce(&Canvas),
    {
        let image_info = ImageInfo::new(
            (self.width, self.height),
            ColorType::BGRA8888,
            AlphaType::Premul,
            None,
        );

        let row_bytes = (self.width * 4) as usize;
        let total_bytes = row_bytes * (self.height as usize);
        let pixels_slice = unsafe {
            std::slice::from_raw_parts_mut(self.pixel_ptr as *mut u8, total_bytes)
        };

        let mut surface = surfaces::wrap_pixels(
            &image_info,
            pixels_slice,
            row_bytes,
            None,
        )
        .expect("[!] Failed to wrap Skia surface");

        // Execute drawing closure on Skia canvas
        draw_fn(surface.canvas());

        // Push pixels to Windows Desktop Window Manager (DWM)
        unsafe {
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };

            let mut pt_src = POINT { x: 0, y: 0 };
            let mut size = SIZE {
                cx: self.width,
                cy: self.height,
            };
            let mut pt_dst = POINT {
                x: self.x,
                y: self.y,
            };

            UpdateLayeredWindow(
                self.hwnd,
                0 as _,
                &mut pt_dst,
                &mut size,
                self.hdc_mem,
                &mut pt_src,
                0,
                &blend,
                ULW_ALPHA,
            );
        }
    }
}

impl Drop for NotchWindow {
    fn drop(&mut self) {
        unsafe {
            if self.hbitmap != 0 as _ {
                DeleteObject(self.hbitmap);
            }
            if self.hdc_mem != 0 as _ {
                DeleteDC(self.hdc_mem);
            }
            if self.hwnd != 0 as _ {
                DestroyWindow(self.hwnd);
            }
        }
    }
}