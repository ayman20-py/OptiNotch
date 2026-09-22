use crate::ui::clock::ClockUI;
use skia_safe::{Canvas, Color, Paint, PaintStyle, RRect, Rect};

pub fn draw_notch(canvas: &Canvas, width: f32, height: f32, clock_ui: &ClockUI) {
    // 1. Clear everything to 100% transparent
    canvas.clear(Color::TRANSPARENT);

    // 2. Draw rounded notch pill
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

    // 3. Render the Live Clock UI
    clock_ui.draw(canvas, width, height);
}