use crate::media::{
    draw_album_art, draw_media_info, draw_no_media_placeholder, draw_playback_controls,
    draw_progress_bar, MediaInfo, MediaLayout,
};
use crate::ui::clock::ClockUI;
use crate::ui::header::draw_header_system_bar;
use crate::ui::system_status::{BatteryStatus, VolumeStatus};
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
    battery: &BatteryStatus,
    volume: &VolumeStatus,
) {
    // 1. Clear transparent background
    canvas.clear(Color::TRANSPARENT);

    let opacity = controller.opacity_spring.current.clamp(0.0, 1.0);
    if opacity <= 0.005 {
        // Completely invisible
        return;
    }

    let alpha_u8 = (opacity * 255.0).round() as u8;
    let scale_factor_anim = controller.scale_spring.current;

    canvas.save();
    let center_x = canvas_width / 2.0;
    let top_y = 0.0;
    canvas.translate((center_x, top_y));
    canvas.scale((scale_factor_anim, scale_factor_anim));
    canvas.translate((-center_x, -top_y));

    canvas.save_layer_alpha(
        Rect::from_xywh(0.0, 0.0, canvas_width, controller.config.canvas_height),
        alpha_u8 as u32,
    );

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
    let expanded_radius = 28.0 * controller.config.scale_factor;
    let corner_radius = collapsed_radius + (expanded_radius - collapsed_radius) * progress;

    let rect = Rect::from_xywh(pill_x, pill_y, current_w, current_h);
    let rrect = RRect::new_rect_xy(rect, corner_radius, corner_radius);

    // 2. Draw black glass pill body
    let mut bg_paint = Paint::default();
    bg_paint.set_anti_alias(true);
    bg_paint.set_color(Color::from_argb(250, 0, 0, 0));
    canvas.draw_rrect(rrect, &bg_paint);

    // 3. Draw subtle border stroke (frosted edge)
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

        // Draw Header System Bar (Battery, Live Volume, Modern Monitor Switch)
        draw_header_system_bar(
            canvas,
            pill_x,
            pill_y,
            current_w,
            scale,
            controller,
            battery,
            volume,
        );

        // Media Widget (when media is detected)
        if media_info.has_media {
            let layout = MediaLayout::compute(pill_x, pill_y, current_w, current_h, scale);

            // Draw Album Art
            draw_album_art(canvas, album_art, layout.art_rect);

            // Draw Track Title & Artist
            let max_w = (pill_x + (current_w * 0.45) - layout.info_x - (8.0 * scale)).max(60.0 * scale);
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

        // Calendar Widget (Right 55% Zone)
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

    canvas.restore(); // restore layer
    canvas.restore(); // restore scale transform
}
