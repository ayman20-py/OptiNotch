use std::ffi::c_void;
use std::ptr::null;
use std::sync::atomic::{AtomicBool, Ordering};
use skia_safe::{surfaces, AlphaType, Canvas, ColorType, ImageInfo};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use super::state::{MediaAction, NotchController, NotchState};
use super::tray::{IDM_EXIT, IDM_TOGGLE, WM_TRAY_ICON};

static EXPAND_REQUESTED_FLAG: AtomicBool = AtomicBool::new(false);
static COLLAPSE_REQUESTED_FLAG: AtomicBool = AtomicBool::new(false);
static SHOW_TRAY_MENU_FLAG: AtomicBool = AtomicBool::new(false);
static EXIT_REQUESTED_FLAG: AtomicBool = AtomicBool::new(false);
static MEDIA_TOGGLE_FLAG: AtomicBool = AtomicBool::new(false);
static MEDIA_NEXT_FLAG: AtomicBool = AtomicBool::new(false);
static MEDIA_PREV_FLAG: AtomicBool = AtomicBool::new(false);

static mut ACTIVE_CONTROLLER_PTR: usize = 0;
static mut ACTIVE_WINDOW_HWND: HWND = 0 as _;
static mut ACTIVE_WINDOW_X: i32 = 0;
static mut ACTIVE_WINDOW_Y: i32 = 0;
static mut MOUSE_HOOK: HHOOK = 0 as _;

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Global low-level mouse hook procedure to detect clicks outside the expanded notch
unsafe extern "system" fn mouse_hook_proc(n_code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if n_code >= 0 {
        let msg = wparam as u32;
        if msg == WM_LBUTTONDOWN || msg == WM_RBUTTONDOWN || msg == WM_NCLBUTTONDOWN {
            unsafe {
                let hook_struct = &*(lparam as *const MSLLHOOKSTRUCT);
                let pt = hook_struct.pt;

                if ACTIVE_CONTROLLER_PTR != 0 {
                    let controller = &*(ACTIVE_CONTROLLER_PTR as *const NotchController);
                    if controller.state == NotchState::Expanded {
                        // Check if click was outside the expanded card
                        if !controller.is_inside_screen_rect(
                            ACTIVE_WINDOW_X,
                            ACTIVE_WINDOW_Y,
                            pt.x,
                            pt.y,
                        ) {
                            COLLAPSE_REQUESTED_FLAG.store(true, Ordering::SeqCst);
                        }
                    }
                }
            }
        }
    }
    unsafe { CallNextHookEx(MOUSE_HOOK, n_code, wparam, lparam) }
}

pub unsafe extern "system" fn wnd_proc(
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

        // Notch direct click on the window
        WM_LBUTTONUP => {
            let controller_ptr = unsafe {
                GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const NotchController
            };

            if !controller_ptr.is_null() {
                let controller = unsafe { &*controller_ptr };

                if controller.state == NotchState::Collapsed {
                    // Click on compact pill expands it
                    EXPAND_REQUESTED_FLAG.store(true, Ordering::SeqCst);
                } else if controller.state == NotchState::Expanded {
                    // In expanded mode: check if user clicked a media control button!
                    let local_x = (lparam & 0xFFFF) as i16 as f32;
                    let local_y = ((lparam >> 16) & 0xFFFF) as i16 as f32;

                    match controller.check_media_click(local_x, local_y) {
                        MediaAction::TogglePlayPause => {
                            MEDIA_TOGGLE_FLAG.store(true, Ordering::SeqCst);
                        }
                        MediaAction::SkipNext => {
                            MEDIA_NEXT_FLAG.store(true, Ordering::SeqCst);
                        }
                        MediaAction::SkipPrevious => {
                            MEDIA_PREV_FLAG.store(true, Ordering::SeqCst);
                        }
                        MediaAction::None => {}
                    }
                }
            }
            0
        }

        // Tray Icon events
        WM_TRAY_ICON => {
            let event = lparam as u32;
            if event == WM_RBUTTONUP {
                SHOW_TRAY_MENU_FLAG.store(true, Ordering::SeqCst);
            } else if event == WM_LBUTTONUP {
                let controller_ptr = unsafe {
                    GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const NotchController
                };
                if !controller_ptr.is_null() {
                    let controller = unsafe { &*controller_ptr };
                    if controller.state == NotchState::Collapsed {
                        EXPAND_REQUESTED_FLAG.store(true, Ordering::SeqCst);
                    } else {
                        COLLAPSE_REQUESTED_FLAG.store(true, Ordering::SeqCst);
                    }
                }
            }
            0
        }

        // Context menu commands
        WM_COMMAND => {
            let cmd_id = (wparam & 0xFFFF) as usize;
            if cmd_id == IDM_TOGGLE {
                let controller_ptr = unsafe {
                    GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const NotchController
                };
                if !controller_ptr.is_null() {
                    let controller = unsafe { &*controller_ptr };
                    if controller.state == NotchState::Collapsed {
                        EXPAND_REQUESTED_FLAG.store(true, Ordering::SeqCst);
                    } else {
                        COLLAPSE_REQUESTED_FLAG.store(true, Ordering::SeqCst);
                    }
                }
            } else if cmd_id == IDM_EXIT {
                EXIT_REQUESTED_FLAG.store(true, Ordering::SeqCst);
                unsafe { DestroyWindow(hwnd) };
            }
            0
        }

        // Dynamic Click-Through Hit Testing
        WM_NCHITTEST => {
            let controller_ptr = unsafe {
                GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const NotchController
            };

            if !controller_ptr.is_null() {
                let controller = unsafe { &*controller_ptr };

                let screen_x = (lparam & 0xFFFF) as i16 as i32;
                let screen_y = ((lparam >> 16) & 0xFFFF) as i16 as i32;

                let mut rect: RECT = unsafe { std::mem::zeroed() };
                unsafe { GetWindowRect(hwnd, &mut rect) };

                let local_x = (screen_x - rect.left) as f32;
                let local_y = (screen_y - rect.top) as f32;

                if controller.is_inside_pill(local_x, local_y) {
                    return HTCLIENT as LRESULT;
                } else {
                    return HTTRANSPARENT as LRESULT;
                }
            }

            HTCLIENT as LRESULT
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
    pub fn new(controller: &mut NotchController) -> Self {
        let width = controller.config.canvas_width.round() as i32;
        let height = controller.config.canvas_height.round() as i32;
        let top_padding = controller.config.top_padding.round() as i32;

        unsafe {
            let class_name = to_wide("OptiNotchClass");
            let window_title = to_wide("OptiNotch");

            let wnd_class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(wnd_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: 0 as _,
                hIcon: LoadIconW(0 as _, IDI_APPLICATION),
                hCursor: LoadCursorW(0 as _, IDC_ARROW),
                hbrBackground: 0 as _,
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
                panic!("[!] Failed to create OptiNotch window");
            }

            // Save pointers for global hook & hit testing
            ACTIVE_CONTROLLER_PTR = controller as *mut NotchController as usize;
            ACTIVE_WINDOW_HWND = hwnd;
            ACTIVE_WINDOW_X = x;
            ACTIVE_WINDOW_Y = y;

            SetWindowLongPtrW(hwnd, GWLP_USERDATA, controller as *mut NotchController as isize);

            // Install global mouse hook for click-outside detection
            MOUSE_HOOK = SetWindowsHookExW(
                WH_MOUSE_LL,
                Some(mouse_hook_proc),
                0 as _,
                0,
            );

            // Create Memory DC
            let hdc_screen = GetDC(0 as _);
            let hdc_mem = CreateCompatibleDC(hdc_screen);
            ReleaseDC(0 as _, hdc_screen);

            // 32-bit Top-down BGRA DIB
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height,
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

    pub fn check_expand_requested(&self) -> bool {
        EXPAND_REQUESTED_FLAG.swap(false, Ordering::SeqCst)
    }

    pub fn check_collapse_requested(&self) -> bool {
        COLLAPSE_REQUESTED_FLAG.swap(false, Ordering::SeqCst)
    }

    pub fn check_show_tray_menu(&self) -> bool {
        SHOW_TRAY_MENU_FLAG.swap(false, Ordering::SeqCst)
    }

    pub fn check_exit_requested(&self) -> bool {
        EXIT_REQUESTED_FLAG.load(Ordering::SeqCst)
    }

    pub fn check_media_toggle(&self) -> bool {
        MEDIA_TOGGLE_FLAG.swap(false, Ordering::SeqCst)
    }

    pub fn check_media_next(&self) -> bool {
        MEDIA_NEXT_FLAG.swap(false, Ordering::SeqCst)
    }

    pub fn check_media_prev(&self) -> bool {
        MEDIA_PREV_FLAG.swap(false, Ordering::SeqCst)
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
        .expect("[!] Failed to create Skia surface");

        draw_fn(surface.canvas());

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
            if MOUSE_HOOK != 0 as _ {
                UnhookWindowsHookEx(MOUSE_HOOK);
            }
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
