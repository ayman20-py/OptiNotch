use crate::media::{
    MediaInfo, draw_album_art, draw_media_info, draw_playback_controls, draw_progress_bar,
};
use crate::ui::clock::ClockUI;
use crate::window::NotchController;
use skia_safe::{Canvas, Color, Image, Paint, PaintStyle, RRect, Rect};

pub fn draw_notch(
    canvas: &Canvas,
    canvas_width: f32,
    _canvas_height: f32,
    controller: &NotchController,
    clock_ui: &ClockUI,
    media_info: &MediaInfo,
    album_art: Option<&Image>,
) {
    // 1. Clear transparent background
    canvas.clear(Color::TRANSPARENT);

    let current_w = controller.current_width();
    let current_h = controller.current_height();

    // Center pill horizontally in the max canvas
    let pill_x = (canvas_width - current_w) / 2.0;
    let pill_y = 0.0;

    // Smooth corner radius transition:
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

    // 4. Draw content based on animation progress
    if progress < 0.45 {
        // Collapsed View (Clock)
        clock_ui.draw(canvas, canvas_width, current_h);
    } else {
        // Expanded View (Clock & Date)
        clock_ui.draw_expanded(canvas, pill_x, pill_y, current_w, current_h);

        // Media Widget (when media is detected)
        if media_info.has_media {
            let scale = controller.config.scale_factor;
            let art_size = 70.0 * scale;
            let art_x = pill_x + (current_w * 0.05);
            let art_y = pill_y + (current_h - art_size) / 1.6;

            // Draw Album Art (Preserving user position)
            draw_album_art(canvas, album_art, art_x, art_y, art_size);

            // Draw Track Title & Artist (Preserving user position)
            let info_x = art_x + (art_size * 1.15);
            let info_y = art_y + (art_size * 0.02);
            let max_w = pill_x + current_w - info_x - (20.0 * scale);
            draw_media_info(
                canvas,
                &media_info.title,
                &media_info.artist,
                info_x,
                info_y,
                max_w,
                scale,
            );

            // Draw Media Progress Bar
            let bar_y = info_y + (47.0 * scale);
            let bar_w = (160.0 * scale).min(max_w);
            draw_progress_bar(
                canvas,
                info_x,
                bar_y,
                bar_w,
                media_info.position_secs,
                media_info.duration_secs,
                scale,
            );

            // Draw Playback Controls (Previous, Play/Pause, Next)
            let controls_cx = info_x + bar_w / 2.0;
            let controls_cy = bar_y + (30.0 * scale);
            draw_playback_controls(
                canvas,
                controls_cx,
                controls_cy,
                media_info.is_playing,
                scale,
            );
        }
    }
}
