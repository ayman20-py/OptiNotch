use crate::media::{
    MediaInfo, MediaLayout, draw_album_art, draw_media_info, draw_no_media_placeholder,
    draw_playback_controls, draw_progress_bar,
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

        let scale = controller.config.scale_factor;

        // Draw Monitor Switch Button (Header Top-Right)
        let mon_cx = pill_x + current_w - (25.0 * scale);
        let mon_cy = pill_y + (24.0 * scale);
        let mon_scale = controller.btn_anims.monitor_scale.current;

        draw_monitor_button(
            canvas,
            mon_cx,
            mon_cy,
            mon_scale,
            scale,
            controller.current_monitor,
        );

        // Media Widget (when media is detected)
        if media_info.has_media {
            let layout = MediaLayout::compute(pill_x, pill_y, current_w, current_h, scale);

            // Draw Album Art
            draw_album_art(canvas, album_art, layout.art_rect);

            // Draw Track Title & Artist
            let max_w = (pill_x + (current_w * 0.5) - layout.info_x - (10.0 * scale)).max(60.0 * scale);
            draw_media_info(
                canvas,
                &media_info.title,
                &media_info.artist,
                layout.info_x,
                layout.info_y,
                max_w,
                scale,
            );

            // Draw Media Progress Bar
            draw_progress_bar(
                canvas,
                layout.bar_rect,
                media_info.position_secs,
                media_info.duration_secs,
                scale,
            );

            // Draw Playback Controls
            draw_playback_controls(canvas, &layout, &controller.btn_anims, scale);
        } else {
            // Modern No Media Placeholder Card
            draw_no_media_placeholder(canvas, pill_x, pill_y, current_w, current_h, scale);
        }

        // Calendar Widget (Right 50% Zone)
        let cal_layout = crate::calendar::CalendarLayout::compute(
            pill_x,
            pill_y,
            current_w,
            current_h,
            scale,
            &controller.calendar,
        );
        crate::calendar::draw_calendar(canvas, &controller.calendar, &cal_layout, scale);
    }
}

fn draw_monitor_button(
    canvas: &Canvas,
    cx: f32,
    cy: f32,
    scale: f32,
    scale_factor: f32,
    current_monitor: usize,
) {
    canvas.save();
    canvas.translate((cx, cy));
    canvas.scale((scale, scale));

    let s = scale_factor;

    // 1. Subtle glass hover backdrop
    let mut bg_paint = Paint::default();
    bg_paint.set_anti_alias(true);
    bg_paint.set_color(Color::from_argb(25, 255, 255, 255));
    let pill_w = 30.0 * s;
    let pill_h = 22.0 * s;
    let pill_rect = Rect::from_xywh(-pill_w / 2.0, -pill_h / 2.0, pill_w, pill_h);
    let pill_rrect = RRect::new_rect_xy(pill_rect, 6.0 * s, 6.0 * s);
    canvas.draw_rrect(pill_rrect, &bg_paint);

    let mut border_paint = Paint::default();
    border_paint.set_anti_alias(true);
    border_paint.set_style(PaintStyle::Stroke);
    border_paint.set_stroke_width(1.0);
    border_paint.set_color(Color::from_argb(40, 255, 255, 255));
    canvas.draw_rrect(pill_rrect, &border_paint);

    // 2. Monitor Screen Vector
    let mut screen_paint = Paint::default();
    screen_paint.set_anti_alias(true);
    screen_paint.set_style(PaintStyle::Stroke);
    screen_paint.set_stroke_width(1.2 * s);
    screen_paint.set_color(Color::from_argb(230, 255, 255, 255));

    let screen_w = 14.0 * s;
    let screen_h = 9.5 * s;
    let screen_rect = Rect::from_xywh(-screen_w / 2.0 - (4.0 * s), -screen_h / 2.0 - (1.0 * s), screen_w, screen_h);
    let screen_rrect = RRect::new_rect_xy(screen_rect, 1.5 * s, 1.5 * s);
    canvas.draw_rrect(screen_rrect, &screen_paint);

    // Stand neck & base
    let neck_x = -4.0 * s;
    let neck_top = screen_rect.bottom;
    let neck_bot = neck_top + (2.0 * s);
    canvas.draw_line((neck_x, neck_top), (neck_x, neck_bot), &screen_paint);
    canvas.draw_line((neck_x - 3.0 * s, neck_bot), (neck_x + 3.0 * s, neck_bot), &screen_paint);

    // 3. Monitor number text (e.g. "1", "2")
    let font_mgr = skia_safe::FontMgr::new();
    let typeface = font_mgr
        .match_family_style("Lilita One", skia_safe::FontStyle::normal())
        .or_else(|| font_mgr.match_family_style("Segoe UI Variable Display", skia_safe::FontStyle::normal()))
        .or_else(|| font_mgr.legacy_make_typeface(None, skia_safe::FontStyle::normal()))
        .expect("Failed to load font for monitor button");

    let mut num_font = skia_safe::Font::new(typeface, 9.5 * s);
    num_font.set_subpixel(true);
    num_font.set_edging(skia_safe::font::Edging::SubpixelAntiAlias);

    let mut num_paint = Paint::default();
    num_paint.set_anti_alias(true);
    num_paint.set_color(Color::from_argb(240, 255, 255, 255));

    let num_str = (current_monitor + 1).to_string();
    let (num_w, _) = num_font.measure_str(&num_str, Some(&num_paint));
    let (_, num_metrics) = num_font.metrics();
    let num_x = 6.0 * s - (num_w / 2.0);
    let num_y = -num_metrics.ascent / 2.0 - (1.0 * s);
    canvas.draw_str(&num_str, (num_x, num_y), &num_font, &num_paint);

    canvas.restore();
}
