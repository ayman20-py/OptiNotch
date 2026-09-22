use skia_safe::{font::Edging, Canvas, Color, Font, FontMgr, FontStyle, Paint};
use windows_sys::Win32::Foundation::SYSTEMTIME;
use windows_sys::Win32::System::SystemInformation::GetLocalTime;

pub struct ClockUI {
    font: Font,
    text_paint: Paint,
}

impl ClockUI {
    pub fn new(scale_factor: f32) -> Self {
        // Load Segoe UI Variable or fallback font
        let font_mgr = FontMgr::new();
        let typeface = font_mgr
            .match_family_style("Lilita One", FontStyle::normal())
            .or_else(|| font_mgr.match_family_style("Segoe UI", FontStyle::normal()))
            .or_else(|| font_mgr.legacy_make_typeface(None, FontStyle::normal()))
            .expect("Failed to load typeface for Clock UI");

        // Scale font size according to monitor DPI
        let font_size = 15.0 * scale_factor;
        let mut font = Font::new(typeface, font_size);
        font.set_subpixel(true);
        font.set_edging(Edging::SubpixelAntiAlias);

        let mut text_paint = Paint::default();
        text_paint.set_anti_alias(true);
        text_paint.set_color(Color::from_argb(240, 255, 255, 255));

        Self { font, text_paint }
    }

    /// Query current local time formatted as "HH:MM" (no seconds)
    pub fn get_time_string(&self) -> String {
        unsafe {
            let mut st: SYSTEMTIME = std::mem::zeroed();
            GetLocalTime(&mut st);
            format!("{:02}:{:02}", st.wHour, st.wMinute)
        }
    }

    /// Draw the clock string centered inside the notch pill
    pub fn draw(&self, canvas: &Canvas, width: f32, height: f32) {
        let time_str = self.get_time_string();

        let (text_width, _bounds) = self.font.measure_str(&time_str, Some(&self.text_paint));
        let (_, metrics) = self.font.metrics();

        // Horizontal and vertical centering calculations
        let text_x = (width - text_width) / 2.0;
        let font_height = -metrics.ascent + metrics.descent;
        let text_y = (height - font_height) / 2.0 - metrics.ascent;

        canvas.draw_str(&time_str, (text_x, text_y), &self.font, &self.text_paint);
    }
}