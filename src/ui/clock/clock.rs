use skia_safe::{font::Edging, Canvas, Color, Font, FontMgr, FontStyle, Paint};
use windows_sys::Win32::Foundation::SYSTEMTIME;
use windows_sys::Win32::System::SystemInformation::GetLocalTime;

pub struct ClockUI {
    font_compact: Font,
    font_large: Font,
    font_sub: Font,
    text_paint_primary: Paint,
    text_paint_secondary: Paint,
    scale_factor: f32,
}

impl ClockUI {
    pub fn new(scale_factor: f32) -> Self {
        let font_mgr = FontMgr::new();
        let typeface = font_mgr
            .match_family_style("Lilita One", FontStyle::normal())
            .or_else(|| font_mgr.match_family_style("Segoe UI", FontStyle::normal()))
            .or_else(|| font_mgr.legacy_make_typeface(None, FontStyle::normal()))
            .expect("Failed to load typeface for Clock UI");

        // Compact font
        let mut font_compact = Font::new(typeface.clone(), 12.5 * scale_factor);
        font_compact.set_subpixel(true);
        font_compact.set_edging(Edging::SubpixelAntiAlias);

        // Large clock font for expanded view
        let mut font_large = Font::new(typeface.clone(), 36.0 * scale_factor);
        font_large.set_subpixel(true);
        font_large.set_edging(Edging::SubpixelAntiAlias);

        // Secondary sub-label font
        let mut font_sub = Font::new(typeface, 11.5 * scale_factor);
        font_sub.set_subpixel(true);
        font_sub.set_edging(Edging::SubpixelAntiAlias);

        // Primary text paint (bright white)
        let mut text_paint_primary = Paint::default();
        text_paint_primary.set_anti_alias(true);
        text_paint_primary.set_color(Color::from_argb(245, 255, 255, 255));

        // Secondary text paint (subtle gray)
        let mut text_paint_secondary = Paint::default();
        text_paint_secondary.set_anti_alias(true);
        text_paint_secondary.set_color(Color::from_argb(160, 255, 255, 255));

        Self {
            font_compact,
            font_large,
            font_sub,
            text_paint_primary,
            text_paint_secondary,
            scale_factor,
        }
    }

    /// Query current local time formatted as "HH:MM"
    pub fn get_time_string(&self) -> String {
        unsafe {
            let mut st: SYSTEMTIME = std::mem::zeroed();
            GetLocalTime(&mut st);
            format!("{:02}:{:02}", st.wHour, st.wMinute)
        }
    }

    /// Query current date string (e.g. "Tuesday, September 22")
    pub fn get_date_string(&self) -> String {
        unsafe {
            let mut st: SYSTEMTIME = std::mem::zeroed();
            GetLocalTime(&mut st);

            let day_name = match st.wDayOfWeek {
                0 => "Sunday",
                1 => "Monday",
                2 => "Tuesday",
                3 => "Wednesday",
                4 => "Thursday",
                5 => "Friday",
                6 => "Saturday",
                _ => "",
            };

            let month_name = match st.wMonth {
                1 => "Jan",
                2 => "Feb",
                3 => "Mar",
                4 => "Apr",
                5 => "May",
                6 => "Jun",
                7 => "Jul",
                8 => "Aug",
                9 => "Sep",
                10 => "Oct",
                11 => "Nov",
                12 => "Dec",
                _ => "",
            };

            format!("{}, {} {}", day_name, month_name, st.wDay)
        }
    }

    /// Draw compact clock string centered inside the compact notch
    pub fn draw(&self, canvas: &Canvas, canvas_width: f32, pill_height: f32) {
        let time_str = self.get_time_string();

        let (text_width, _bounds) =
            self.font_compact.measure_str(&time_str, Some(&self.text_paint_primary));
        let (_, metrics) = self.font_compact.metrics();

        let text_x = (canvas_width - text_width) / 2.0;
        let font_height = -metrics.ascent + metrics.descent;
        let text_y = (pill_height - font_height) / 2.0 - metrics.ascent;

        canvas.draw_str(&time_str, (text_x, text_y), &self.font_compact, &self.text_paint_primary);
    }

    /// Draw expanded card content (Large digital clock + full date label)
    pub fn draw_expanded(
        &self,
        canvas: &Canvas,
        card_x: f32,
        card_y: f32,
        card_width: f32,
        card_height: f32,
    ) {
        let time_str = self.get_time_string();
        let date_str = self.get_date_string();

        // 1. Draw large time
        let (time_w, _) =
            self.font_large.measure_str(&time_str, Some(&self.text_paint_primary));
        let (_, time_metrics) = self.font_large.metrics();

        let time_x = card_x + (card_width - time_w) / 2.0;
        let time_y = card_y + (card_height * 0.48) - time_metrics.ascent / 2.0;

        canvas.draw_str(&time_str, (time_x, time_y), &self.font_large, &self.text_paint_primary);

        // 2. Draw date subtext below the time
        let (date_w, _) =
            self.font_sub.measure_str(&date_str, Some(&self.text_paint_secondary));
        let (_, date_metrics) = self.font_sub.metrics();

        let date_x = card_x + (card_width - date_w) / 2.0;
        let date_y = time_y + date_metrics.descent + (16.0 * self.scale_factor);

        canvas.draw_str(&date_str, (date_x, date_y), &self.font_sub, &self.text_paint_secondary);
    }
}