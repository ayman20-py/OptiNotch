use crate::ui::clock::ClockUI;
use crate::window::NotchController;
use skia_safe::{Canvas, Color, Paint, PaintStyle, RRect, Rect};

pub fn draw_notch(
    canvas: &Canvas,
    canvas_width: f32,
    _canvas_height: f32,
    controller: &NotchController,
    clock_ui: &ClockUI,
) {
    // 1. Clear transparent background
    canvas.clear(Color::TRANSPARENT);

    let current_w = controller.current_width();
    let current_h = controller.current_height();

    // Center pill horizontally in the max canvas
    let pill_x = (canvas_width - current_w) / 2.0;
    let pill_y = 0.0;

    // Smooth corner radius transition:
    // Morph from pill corner radius (h/2) to rounded card (22px)
    let min_h = controller.config.collapsed_height;
    let max_h = controller.config.expanded_height;
    let progress = ((current_h - min_h) / (max_h - min_h)).clamp(0.0, 1.0);

    let collapsed_radius = current_h / 2.0;
    let expanded_radius = 22.0 * controller.config.scale_factor;
    let corner_radius = collapsed_radius + (expanded_radius - collapsed_radius) * progress;

    let rect = Rect::from_xywh(pill_x, pill_y, current_w, current_h);
    let rrect = RRect::new_rect_xy(rect, corner_radius, corner_radius);

    // 2. Draw dark obsidian background
    let mut bg_paint = Paint::default();
    bg_paint.set_anti_alias(true);
    bg_paint.set_style(PaintStyle::Fill);
    bg_paint.set_color(Color::from_argb(255, 16, 16, 16));
    canvas.draw_rrect(rrect, &bg_paint);

    // 3. Draw subtle border stroke
    let mut border_paint = Paint::default();
    border_paint.set_anti_alias(true);
    border_paint.set_style(PaintStyle::Stroke);
    border_paint.set_stroke_width(1.0);
    border_paint.set_color(Color::from_argb(35, 255, 255, 255));
    canvas.draw_rrect(rrect, &border_paint);

    // 4. Draw content (morph seamlessly between compact and expanded views)
    if progress < 0.45 {
        // Draw compact clock
        clock_ui.draw(canvas, canvas_width, current_h);
    } else {
        // Draw expanded card content
        clock_ui.draw_expanded(canvas, pill_x, pill_y, current_w, current_h);
    }
}
