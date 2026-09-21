use skia_safe::{
    font::Edging, Canvas, Color, Font, FontMgr, FontStyle, Paint, PaintStyle, RRect, Rect,
};

use windows_sys::Win32::UI::HiDpi::{ GetDpiForSystem };

pub fn draw_notch(canvas: &Canvas, width: f32, height: f32) {
    let dpi  = unsafe { GetDpiForSystem() } as f32;
    let scale_factor = dpi/ 96.0;
    // Clear everything to 100% transparent
    canvas.clear(Color::TRANSPARENT);

    //Draw rounded notch pill
    let corner_radius = height / 2.0;
    let rect = Rect::from_xywh(0.0, 0.0, width, height);
    let rrect = RRect::new_rect_xy(rect, corner_radius, corner_radius);

    // Pill dark background
    let mut bg_paint = Paint::default();
    bg_paint.set_anti_alias(true);
    bg_paint.set_style(PaintStyle::Fill);
    bg_paint.set_color(Color::from_argb(255, 18, 18, 18));
    canvas.draw_rrect(rrect, &bg_paint);

    // Subtle border glow / stroke
    let mut border_paint = Paint::default();
    border_paint.set_anti_alias(true);
    border_paint.set_style(PaintStyle::Stroke);
    border_paint.set_stroke_width(1.0);
    border_paint.set_color(Color::from_argb(40, 255, 255, 255));
    canvas.draw_rrect(rrect, &border_paint);

    //  Render centered text
    let font_mgr = FontMgr::new();
    let typeface = font_mgr
        .match_family_style("Segoe UI Variable Display", FontStyle::normal())
        .or_else(|| font_mgr.match_family_style("Segoe UI", FontStyle::normal()))
        .or_else(|| font_mgr.legacy_make_typeface(None, FontStyle::normal()))
        .expect("Failed to load typeface");


    let font_size = 11.0 * scale_factor;
    let mut font = Font::new(typeface, font_size);
    font.set_subpixel(true);
    font.set_edging(Edging::SubpixelAntiAlias);


    let mut text_paint = Paint::default();
    text_paint.set_anti_alias(true);
    text_paint.set_color(Color::from_argb(230, 255, 255, 255));

    let text = "OptiNotch";
    let (text_width, _bounds) = font.measure_str(text, Some(&text_paint));
    let (_, metrics) = font.metrics();

    let text_x = (width - text_width) / 2.0;
    let font_height = -metrics.ascent + metrics.descent;
    let text_y = (height - font_height) / 2.0 - metrics.ascent;

    canvas.draw_str(text, (text_x, text_y), &font, &text_paint);
}