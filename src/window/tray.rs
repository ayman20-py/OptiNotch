use std::ptr::null;
use skia_safe::{surfaces, AlphaType, Color, ColorType, ImageInfo, Paint, PaintStyle, RRect, Rect};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::Shell::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub const WM_TRAY_ICON: u32 = WM_USER + 100;
pub const IDM_TOGGLE: usize = 1001;
pub const IDM_EXIT: usize = 1002;

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Renders the OptiNotch vector icon using Skia into an in-memory 32-bit Win32 HICON
fn create_skia_tray_icon(size: i32) -> HICON {
    let width = size;
    let height = size;

    let image_info = ImageInfo::new(
        (width, height),
        ColorType::BGRA8888,
        AlphaType::Premul,
        None,
    );

    let mut surface = surfaces::raster(&image_info, None, None)
        .expect("Failed to create Skia raster surface for tray icon");

    let canvas = surface.canvas();
    canvas.clear(Color::TRANSPARENT);

    // Coordinate scale relative to 32x32 viewBox
    let s = size as f32 / 32.0;

    // 1. Top Edge Guide line
    let mut line_paint = Paint::default();
    line_paint.set_anti_alias(true);
    line_paint.set_style(PaintStyle::Stroke);
    line_paint.set_stroke_width(2.0 * s);
    line_paint.set_color(Color::from_argb(255, 113, 113, 122)); // #71717A
    canvas.draw_line(
        (4.0 * s, 6.0 * s),
        (28.0 * s, 6.0 * s),
        &line_paint,
    );

    // 2. Light Grey Rounded Notch Capsule
    let capsule_rect = Rect::from_xywh(8.0 * s, 9.0 * s, 16.0 * s, 8.0 * s);
    let capsule_rrect = RRect::new_rect_xy(capsule_rect, 4.0 * s, 4.0 * s);

    let mut capsule_fill = Paint::default();
    capsule_fill.set_anti_alias(true);
    capsule_fill.set_style(PaintStyle::Fill);
    capsule_fill.set_color(Color::from_argb(255, 228, 228, 231)); // #E4E4E7
    canvas.draw_rrect(capsule_rrect, &capsule_fill);

    let mut capsule_stroke = Paint::default();
    capsule_stroke.set_anti_alias(true);
    capsule_stroke.set_style(PaintStyle::Stroke);
    capsule_stroke.set_stroke_width(1.5 * s);
    capsule_stroke.set_color(Color::from_argb(255, 113, 113, 122)); // #71717A
    canvas.draw_rrect(capsule_rrect, &capsule_stroke);

    // 3. Active Amber Indicator Dot
    let mut dot_paint = Paint::default();
    dot_paint.set_anti_alias(true);
    dot_paint.set_style(PaintStyle::Fill);
    dot_paint.set_color(Color::from_argb(255, 245, 158, 11)); // #F59E0B
    canvas.draw_circle((16.0 * s, 13.0 * s), 1.5 * s, &dot_paint);

    // Read out raw BGRA pixels
    let row_bytes = (width * 4) as usize;
    let mut pixel_bytes = vec![0u8; row_bytes * height as usize];
    let pixmap = surface.peek_pixels().expect("Failed to peek pixels");
    pixmap.read_pixels(
        &image_info,
        &mut pixel_bytes,
        row_bytes,
        (0, 0),
    );

    // Create Win32 32-bit ARGB HICON
    unsafe {
        let hdc_screen = GetDC(0 as _);
        let hbm_color = CreateBitmap(width, height, 1, 32, pixel_bytes.as_ptr() as *const _);
        let hbm_mask = CreateCompatibleBitmap(hdc_screen, width, height);
        ReleaseDC(0 as _, hdc_screen);

        let icon_info = ICONINFO {
            fIcon: 1, // TRUE for icon
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: hbm_mask,
            hbmColor: hbm_color,
        };

        let hicon = CreateIconIndirect(&icon_info);

        DeleteObject(hbm_color);
        DeleteObject(hbm_mask);

        hicon
    }
}

pub struct TrayIcon {
    hwnd: HWND,
    uid: u32,
    hicon: HICON,
}

impl TrayIcon {
    pub fn new(hwnd: HWND, uid: u32, tooltip: &str) -> Self {
        unsafe {
            let hicon = create_skia_tray_icon(32);

            let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
            nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = hwnd;
            nid.uID = uid;
            nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
            nid.uCallbackMessage = WM_TRAY_ICON;
            nid.hIcon = hicon;

            let tip_wide = to_wide(tooltip);
            let len = tip_wide.len().min(nid.szTip.len() - 1);
            nid.szTip[..len].copy_from_slice(&tip_wide[..len]);

            Shell_NotifyIconW(NIM_ADD, &nid);

            Self { hwnd, uid, hicon }
        }
    }

    /// Show right-click context menu at current cursor position
    pub fn show_context_menu(&self) {
        unsafe {
            let mut pt: POINT = std::mem::zeroed();
            GetCursorPos(&mut pt);

            let hmenu = CreatePopupMenu();
            let exit_text = to_wide("Exit OptiNotch");

            AppendMenuW(hmenu, MF_SEPARATOR, 0, null());
            AppendMenuW(hmenu, MF_STRING, IDM_EXIT, exit_text.as_ptr());

            SetForegroundWindow(self.hwnd);

            TrackPopupMenu(
                hmenu,
                TPM_RIGHTBUTTON | TPM_BOTTOMALIGN,
                pt.x,
                pt.y,
                0,
                self.hwnd,
                null(),
            );

            DestroyMenu(hmenu);
        }
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        unsafe {
            let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
            nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = self.hwnd;
            nid.uID = self.uid;

            Shell_NotifyIconW(NIM_DELETE, &nid);

            if self.hicon != 0 as _ {
                DestroyIcon(self.hicon);
            }
        }
    }
}
