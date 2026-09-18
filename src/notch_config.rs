use display_info::DisplayInfo;
use floem::kurbo::Point;
use windows_sys::Win32::Graphics::Dwm::DwmExtendFrameIntoClientArea;
use windows_sys::Win32::UI::Controls::MARGINS;
use windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW;

pub const NOTCH_RADIUS: f64 = 14.0;

pub fn calculate_notch_layout() -> (f64, f64, Point) {
    let mut logical_w: f64 = 1920.0;
    let mut logical_h: f64 = 1080.0;

    // Retrieve all displays and find the primary one
    if let Ok(displays) = DisplayInfo::all() {
        if let Some(primary) = displays.into_iter().find(|d| d.is_primary) {
            let scale = primary.scale_factor as f64;
            logical_h = primary.height as f64 / scale;
            logical_w = primary.width as f64 / scale;
        }
    }

    // Calculating the size relative to the screen 
    let raw_notch_w = logical_w * 0.14;

    // Clamp width between sensible minimum and maximum values
    let notch_w = raw_notch_w.clamp(200.0, 450.0);

    // Notch height (either fixed or proportion of height)
    let notch_h = (logical_h * 0.03).clamp(28.0, 40.0);

    let pos_x = (logical_w - notch_w) / 2.0; // Centering the notch horizontally
    let pos_y = 0.0;
    
    (notch_w, notch_h, Point::new(pos_x, pos_y))
}

pub fn enable_smooth_transparency() {
    unsafe {
        let title: Vec<u16> = "OptiNotch\0".encode_utf16().collect();
        let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());

        if !hwnd.is_null() {
            let margins = MARGINS {
                cxLeftWidth: -1,
                cxRightWidth: -1,
                cyTopHeight: -1,
                cyBottomHeight: -1,
            };
            DwmExtendFrameIntoClientArea(hwnd, &margins);
        }
    }
}