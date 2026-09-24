use std::ffi::c_void;
use std::ptr::null;
use std::sync::atomic::{AtomicBool, Ordering};
use skia_safe::{surfaces, AlphaType, Canvas, ColorType, ImageInfo};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, RegisterHotKey, UnregisterHotKey,
};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use super::state::{MediaAction, NotchController, NotchState};
use super::tray::{IDM_AUTOSTART, IDM_CHECK_UPDATES, IDM_EXIT, IDM_TOGGLE, WM_TRAY_ICON};

const HOTKEY_TOGGLE_ID: i32 = 1001;

// Windows Key + \ (VK_OEM_5 = 0xDC)
const MOD_WIN: u32 = 0x0008;
const MOD_NOREPEAT: u32 = 0x4000;
const VK_OEM_5: u32 = 0xDC;

static EXPAND_REQUESTED_FLAG: AtomicBool = AtomicBool::new(false);
static COLLAPSE_REQUESTED_FLAG: AtomicBool = AtomicBool::new(false);
static SHOW_TRAY_MENU_FLAG: AtomicBool = AtomicBool::new(false);
static CHECK_UPDATES_FLAG: AtomicBool = AtomicBool::new(false);
static EXIT_REQUESTED_FLAG: AtomicBool = AtomicBool::new(false);
static MEDIA_TOGGLE_FLAG: AtomicBool = AtomicBool::new(false);
static MEDIA_NEXT_FLAG: AtomicBool = AtomicBool::new(false);
static MEDIA_PREV_FLAG: AtomicBool = AtomicBool::new(false);
static SWITCH_MONITOR_FLAG: AtomicBool = AtomicBool::new(false);
static HIDE_NOTCH_FLAG: AtomicBool = AtomicBool::new(false);
static CALENDAR_CONNECT_FLAG: AtomicBool = AtomicBool::new(false);

static mut ACTIVE_CONTROLLER_PTR: usize = 0;
static mut ACTIVE_WINDOW_HWND: HWND = 0 as _;
static mut ACTIVE_WINDOW_X: i32 = 0;
static mut ACTIVE_WINDOW_Y: i32 = 0;
static mut MOUSE_HOOK: HHOOK = 0 as _;
static mut KEYBOARD_HOOK: HHOOK = 0 as _;

// Virtual key codes for Win & Alt
const VK_LWIN: u32 = 0x5B;
const VK_RWIN: u32 = 0x5C;
const VK_LMENU: u32 = 0xA4;
const VK_RMENU: u32 = 0xA5;
const VK_MENU: u32 = 0x12;

#[derive(Clone, Copy)]
pub struct MonitorDevice {
    pub _hmonitor: HMONITOR,
    pub rect: RECT,
    pub is_primary: bool,
}

unsafe extern "system" fn monitor_enum_proc(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _lprc: *mut RECT,
    lparam: LPARAM,
) -> i32 {
    unsafe {
        let monitors = &mut *(lparam as *mut Vec<MonitorDevice>);
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;

        if GetMonitorInfoW(hmonitor, &mut info) != 0 {
            let is_primary = (info.dwFlags & 1) != 0;
            monitors.push(MonitorDevice {
                _hmonitor: hmonitor,
                rect: info.rcMonitor,
                is_primary,
            });
        }
    }
    1
}

pub fn get_all_monitors() -> Vec<MonitorDevice> {
    let mut monitors: Vec<MonitorDevice> = Vec::new();
    unsafe {
        EnumDisplayMonitors(
            0 as _,
            std::ptr::null(),
            Some(monitor_enum_proc),
            &mut monitors as *mut Vec<MonitorDevice> as LPARAM,
        );
    }
    monitors.sort_by_key(|m| if m.is_primary { 0 } else { 1 });
    if monitors.is_empty() {
        unsafe {
            let w = GetSystemMetrics(SM_CXSCREEN);
            let h = GetSystemMetrics(SM_CYSCREEN);
            monitors.push(MonitorDevice {
                _hmonitor: 0 as _,
                rect: RECT {
                    left: 0,
                    top: 0,
                    right: w,
                    bottom: h,
                },
                is_primary: true,
            });
        }
    }
    monitors
}

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
        } else if msg == WM_MOUSEWHEEL {
            unsafe {
                let hook_struct = &*(lparam as *const MSLLHOOKSTRUCT);
                let pt = hook_struct.pt;

                if ACTIVE_CONTROLLER_PTR != 0 {
                    let controller = &mut *(ACTIVE_CONTROLLER_PTR as *mut NotchController);
                    if controller.state == NotchState::Expanded
                        && controller.is_inside_screen_rect(ACTIVE_WINDOW_X, ACTIVE_WINDOW_Y, pt.x, pt.y)
                    {
                        let wheel_delta = ((hook_struct.mouseData >> 16) & 0xFFFF) as i16 as f32;
                        let scroll_amount = (wheel_delta / 120.0) * (34.0 * controller.config.scale_factor);
                        controller.calendar.scroll_events(-scroll_amount);
                        controller.is_animating = true;
                        controller.last_frame_time = Some(std::time::Instant::now());
                    }
                }
            }
        }
    }
    unsafe { CallNextHookEx(MOUSE_HOOK, n_code, wparam, lparam) }
}

/// Global low-level keyboard hook procedure to detect Win + Alt key combinations
unsafe extern "system" fn keyboard_hook_proc(n_code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if n_code >= 0 {
        let msg = wparam as u32;
        if msg == WM_KEYDOWN || msg == WM_KEYUP || msg == WM_SYSKEYDOWN || msg == WM_SYSKEYUP {
            unsafe {
                let win_down = (GetAsyncKeyState(VK_LWIN as i32) as u16 & 0x8000 != 0)
                    || (GetAsyncKeyState(VK_RWIN as i32) as u16 & 0x8000 != 0);
                let alt_down = (GetAsyncKeyState(VK_MENU as i32) as u16 & 0x8000 != 0)
                    || (GetAsyncKeyState(VK_LMENU as i32) as u16 & 0x8000 != 0)
                    || (GetAsyncKeyState(VK_RMENU as i32) as u16 & 0x8000 != 0);

                let is_holding_win_alt = win_down && alt_down;
                let prev = HIDE_NOTCH_FLAG.swap(is_holding_win_alt, Ordering::SeqCst);

                if prev != is_holding_win_alt && ACTIVE_CONTROLLER_PTR != 0 {
                    let controller = &mut *(ACTIVE_CONTROLLER_PTR as *mut NotchController);
                    controller.set_hidden(is_holding_win_alt);
                }
            }
        }
    }
    unsafe { CallNextHookEx(KEYBOARD_HOOK, n_code, wparam, lparam) }
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
                    // Use exact screen-to-window coordinate math for subpixel accuracy
                    let mut pt: POINT = unsafe { std::mem::zeroed() };
                    unsafe { GetCursorPos(&mut pt) };

                    let mut rect: RECT = unsafe { std::mem::zeroed() };
                    unsafe { GetWindowRect(hwnd, &mut rect) };

                    let local_x = (pt.x - rect.left) as f32;
                    let local_y = (pt.y - rect.top) as f32;

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
                        MediaAction::SwitchMonitor => {
                            SWITCH_MONITOR_FLAG.store(true, Ordering::SeqCst);
                        }
                        MediaAction::CalendarSelectDay(idx) => {
                            let controller_mut = unsafe { &mut *(controller_ptr as *mut NotchController) };
                            controller_mut.calendar.select_day(idx);
                            controller_mut.is_animating = true;
                            controller_mut.last_frame_time = Some(std::time::Instant::now());
                        }
                        MediaAction::CalendarPrevWeek => {
                            let controller_mut = unsafe { &mut *(controller_ptr as *mut NotchController) };
                            controller_mut.calendar.prev_week();
                            controller_mut.is_animating = true;
                            controller_mut.last_frame_time = Some(std::time::Instant::now());
                        }
                        MediaAction::CalendarNextWeek => {
                            let controller_mut = unsafe { &mut *(controller_ptr as *mut NotchController) };
                            controller_mut.calendar.next_week();
                            controller_mut.is_animating = true;
                            controller_mut.last_frame_time = Some(std::time::Instant::now());
                        }
                        MediaAction::CalendarOpenMonthPicker => {
                            let controller_mut = unsafe { &mut *(controller_ptr as *mut NotchController) };
                            controller_mut.calendar.open_month_picker();
                            controller_mut.is_animating = true;
                            controller_mut.last_frame_time = Some(std::time::Instant::now());
                        }
                        MediaAction::CalendarCloseMonthPicker => {
                            let controller_mut = unsafe { &mut *(controller_ptr as *mut NotchController) };
                            controller_mut.calendar.close_month_picker();
                            controller_mut.is_animating = true;
                            controller_mut.last_frame_time = Some(std::time::Instant::now());
                        }
                        MediaAction::CalendarPrevMonth => {
                            let controller_mut = unsafe { &mut *(controller_ptr as *mut NotchController) };
                            controller_mut.calendar.prev_month_picker();
                            controller_mut.is_animating = true;
                            controller_mut.last_frame_time = Some(std::time::Instant::now());
                        }
                        MediaAction::CalendarNextMonth => {
                            let controller_mut = unsafe { &mut *(controller_ptr as *mut NotchController) };
                            controller_mut.calendar.next_month_picker();
                            controller_mut.is_animating = true;
                            controller_mut.last_frame_time = Some(std::time::Instant::now());
                        }
                        MediaAction::CalendarSelectPickerDate { year, month, day } => {
                            let controller_mut = unsafe { &mut *(controller_ptr as *mut NotchController) };
                            controller_mut.calendar.select_date_from_picker(year, month, day);
                            controller_mut.is_animating = true;
                            controller_mut.last_frame_time = Some(std::time::Instant::now());
                        }
                        MediaAction::CalendarConnectGoogle => {
                            CALENDAR_CONNECT_FLAG.store(true, Ordering::SeqCst);
                        }
                        MediaAction::None => {}
                    }
                }
            }
            0
        }

        WM_MOUSEWHEEL => {
            let controller_ptr = unsafe {
                GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut NotchController
            };
            if !controller_ptr.is_null() {
                let controller = unsafe { &mut *controller_ptr };
                if controller.state == NotchState::Expanded {
                    let wheel_delta = ((wparam >> 16) & 0xFFFF) as i16 as f32;
                    let scroll_amount = (wheel_delta / 120.0) * (34.0 * controller.config.scale_factor);
                    controller.calendar.scroll_events(-scroll_amount);
                    controller.is_animating = true;
                    controller.last_frame_time = Some(std::time::Instant::now());
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
            } else if cmd_id == IDM_AUTOSTART {
                let enabled = crate::autostart::is_autostart_enabled();
                let _ = crate::autostart::set_autostart(!enabled);
            } else if cmd_id == IDM_CHECK_UPDATES {
                CHECK_UPDATES_FLAG.store(true, Ordering::SeqCst);
            } else if cmd_id == IDM_EXIT {
                EXIT_REQUESTED_FLAG.store(true, Ordering::SeqCst);
                unsafe { DestroyWindow(hwnd) };
            }
            0
        }

        // Global Keyboard Shortcut: Win + \
        WM_HOTKEY => {
            if wparam as i32 == HOTKEY_TOGGLE_ID {
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
    pub top_padding: i32,
    pub monitors: Vec<MonitorDevice>,
    pub current_monitor_index: usize,
}

impl NotchWindow {
    pub fn new(controller: &mut NotchController) -> Self {
        let width = controller.config.canvas_width.round() as i32;
        let height = controller.config.canvas_height.round() as i32;
        let top_padding = controller.config.top_padding.round() as i32;

        let monitors = get_all_monitors();
        let current_monitor_index = 0;

        let primary_mon = &monitors[0];
        let mon_w = primary_mon.rect.right - primary_mon.rect.left;
        let x = primary_mon.rect.left + (mon_w - width) / 2;
        let y = primary_mon.rect.top + top_padding;

        controller.current_monitor = 0;
        controller.total_monitors = monitors.len();

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

            // Install global keyboard hook for Win + Alt hold detection
            KEYBOARD_HOOK = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(keyboard_hook_proc),
                0 as _,
                0,
            );

            // Register global hotkey: Win + \ (VK_OEM_5)
            RegisterHotKey(
                hwnd,
                HOTKEY_TOGGLE_ID,
                MOD_WIN | MOD_NOREPEAT,
                VK_OEM_5,
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
                top_padding,
                monitors,
                current_monitor_index,
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

    pub fn check_check_updates(&self) -> bool {
        CHECK_UPDATES_FLAG.swap(false, Ordering::SeqCst)
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

    pub fn check_switch_monitor(&self) -> bool {
        SWITCH_MONITOR_FLAG.swap(false, Ordering::SeqCst)
    }

    pub fn check_calendar_connect(&self) -> bool {
        CALENDAR_CONNECT_FLAG.swap(false, Ordering::SeqCst)
    }

    pub fn resize_surface(&mut self, new_w: i32, new_h: i32) {
        if self.width == new_w && self.height == new_h && !self.pixel_ptr.is_null() {
            return;
        }

        self.width = new_w;
        self.height = new_h;

        unsafe {
            if self.hbitmap != 0 as _ {
                DeleteObject(self.hbitmap);
            }

            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: new_w,
                    biHeight: -new_h,
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
            self.hbitmap = CreateDIBSection(
                self.hdc_mem,
                &bmi,
                DIB_RGB_COLORS,
                &mut pixel_ptr,
                0 as _,
                0,
            );
            self.pixel_ptr = pixel_ptr;

            SelectObject(self.hdc_mem, self.hbitmap);
        }
    }

    pub fn switch_to_next_monitor(&mut self, controller: &mut NotchController) -> f32 {
        self.monitors = get_all_monitors();
        if self.monitors.is_empty() {
            return controller.config.scale_factor;
        }
        self.current_monitor_index = (self.current_monitor_index + 1) % self.monitors.len();
        let mon = self.monitors[self.current_monitor_index];
        let mon_w = mon.rect.right - mon.rect.left;

        // 1. Move window to the target display first
        let initial_x = mon.rect.left + (mon_w - self.width) / 2;
        let initial_y = mon.rect.top + self.top_padding;

        unsafe {
            SetWindowPos(
                self.hwnd,
                0 as _,
                initial_x,
                initial_y,
                self.width,
                self.height,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }

        // 2. Query target monitor DPI
        let dpi = unsafe {
            windows_sys::Win32::UI::HiDpi::GetDpiForWindow(self.hwnd)
        } as f32;
        let scale_factor = if dpi > 0.0 { dpi / 96.0 } else { 1.0 };

        // 3. Update controller config & dimensions
        controller.update_scale(scale_factor);
        controller.current_monitor = self.current_monitor_index;
        controller.total_monitors = self.monitors.len();

        let new_w = controller.config.canvas_width.round() as i32;
        let new_h = controller.config.canvas_height.round() as i32;
        let top_padding = controller.config.top_padding.round() as i32;
        self.top_padding = top_padding;

        // 4. Resize DIB section surface if canvas dimensions changed
        self.resize_surface(new_w, new_h);

        // 5. Center with new scaled dimensions on target display
        let new_x = mon.rect.left + (mon_w - new_w) / 2;
        let new_y = mon.rect.top + top_padding;
        self.x = new_x;
        self.y = new_y;

        unsafe {
            ACTIVE_WINDOW_X = new_x;
            ACTIVE_WINDOW_Y = new_y;
            SetWindowPos(
                self.hwnd,
                0 as _,
                new_x,
                new_y,
                new_w,
                new_h,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }

        scale_factor
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
            UnregisterHotKey(self.hwnd, HOTKEY_TOGGLE_ID);
            if MOUSE_HOOK != 0 as _ {
                UnhookWindowsHookEx(MOUSE_HOOK);
            }
            if KEYBOARD_HOOK != 0 as _ {
                UnhookWindowsHookEx(KEYBOARD_HOOK);
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
