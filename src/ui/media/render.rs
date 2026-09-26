use super::layout::MediaLayout;
use crate::window::state::ButtonAnimations;
use skia_safe::{
    Canvas, Color, Font, Image, Paint, PaintStyle, PathBuilder, Point, RRect, Rect, font::Edging,
};

pub fn draw_no_media_placeholder(
    canvas: &Canvas,
    pill_x: f32,
    pill_y: f32,
    _current_w: f32,
    current_h: f32,
    scale_factor: f32,
) {
    let s = scale_factor;
    let card_x = pill_x + (20.0 * s);
    let card_y = pill_y + (current_h * 0.38);
    let card_h = current_h * 0.52;

    // 1. Modern Equalizer Waveform Glyph
    let icon_cx = card_x + (14.0 * s);
    let icon_cy = card_y + card_h / 2.0;

    let mut icon_paint = Paint::default();
    icon_paint.set_anti_alias(true);
    icon_paint.set_color(Color::from_argb(140, 255, 255, 255));

    // 4 vertical audio bars
    let bar_w = 3.0 * s;
    let heights = [10.0 * s, 20.0 * s, 15.0 * s, 9.0 * s];
    let bar_spacing = 5.5 * s;
    let start_x = icon_cx - (1.5 * bar_spacing);

    for (i, &bh) in heights.iter().enumerate() {
        let bx = start_x + (i as f32 * bar_spacing);
        let by = icon_cy - bh / 2.0;
        let bar_rrect =
            RRect::new_rect_xy(Rect::from_xywh(bx, by, bar_w, bh), bar_w / 2.0, bar_w / 2.0);
        canvas.draw_rrect(bar_rrect, &icon_paint);
    }

    // 2. Typography
    let fonts = crate::ui::font_cache::FontCache::get();
    let text_x = icon_cx + (22.0 * s);

    let mut title_font = Font::new(fonts.regular.clone(), 13.0 * s);
    title_font.set_subpixel(true);
    title_font.set_edging(Edging::SubpixelAntiAlias);

    let mut title_paint = Paint::default();
    title_paint.set_anti_alias(true);
    title_paint.set_color(Color::from_argb(180, 255, 255, 255));

    let (_, metrics) = title_font.metrics();
    let title_y = icon_cy - (metrics.ascent + metrics.descent) / 2.0;
    canvas.draw_str(
        "No Media Playing...",
        (text_x, title_y),
        &title_font,
        &title_paint,
    );
}

pub fn draw_album_art(canvas: &Canvas, image: Option<&Image>, dest_rect: Rect) {
    let rrect = RRect::new_rect_xy(dest_rect, 12.0, 12.0);

    canvas.save();
    canvas.clip_rrect(rrect, None, true);

    if let Some(img) = image {
        let src_rect = Rect::from_wh(img.width() as f32, img.height() as f32);
        canvas.draw_image_rect(
            img,
            Some((&src_rect, skia_safe::canvas::SrcRectConstraint::Fast)),
            dest_rect,
            &Paint::default(),
        );
    } else {
        let mut placeholder = Paint::default();
        placeholder.set_color(Color::from_argb(255, 30, 30, 30));
        canvas.draw_rect(dest_rect, &placeholder);

        let mut dot_paint = Paint::default();
        dot_paint.set_color(Color::from_argb(120, 255, 255, 255));
        let size = dest_rect.width();
        canvas.draw_circle(
            (dest_rect.left + size / 2.0, dest_rect.top + size / 2.0),
            size * 0.2,
            &dot_paint,
        );
    }

    canvas.restore();
}

pub fn draw_media_info(
    canvas: &Canvas,
    title: &str,
    artist: &str,
    x: f32,
    y: f32,
    max_width: f32,
    scale_factor: f32,
) {
    let fonts = crate::ui::font_cache::FontCache::get();

    // 1. Title
    let mut title_font = Font::new(fonts.bold.clone(), 14.0 * scale_factor);
    title_font.set_subpixel(true);
    title_font.set_edging(Edging::SubpixelAntiAlias);

    let mut title_paint = Paint::default();
    title_paint.set_anti_alias(true);
    title_paint.set_color(Color::from_argb(245, 255, 255, 255));

    let mut display_title = title.to_string();
    let (mut title_w, _) = title_font.measure_str(&display_title, Some(&title_paint));
    if title_w > max_width && max_width > 0.0 {
        let mut chars = title.chars().collect::<Vec<_>>();
        while !chars.is_empty() && title_w > max_width {
            chars.pop();
            display_title = format!("{}...", chars.iter().collect::<String>());
            let (w, _) = title_font.measure_str(&display_title, Some(&title_paint));
            title_w = w;
        }
    }

    let (_, title_metrics) = title_font.metrics();
    let title_y = y - title_metrics.ascent;
    canvas.draw_str(&display_title, (x, title_y), &title_font, &title_paint);

    // 2. Artist
    let mut artist_font = Font::new(fonts.regular.clone(), 12.0 * scale_factor);
    artist_font.set_subpixel(true);
    artist_font.set_edging(Edging::SubpixelAntiAlias);

    let mut artist_paint = Paint::default();
    artist_paint.set_anti_alias(true);
    artist_paint.set_color(Color::from_argb(150, 255, 255, 255));

    let mut display_artist = artist.to_string();
    let (mut artist_w, _) = artist_font.measure_str(&display_artist, Some(&artist_paint));
    if artist_w > max_width && max_width > 0.0 {
        let mut chars = artist.chars().collect::<Vec<_>>();
        while !chars.is_empty() && artist_w > max_width {
            chars.pop();
            display_artist = format!("{}...", chars.iter().collect::<String>());
            let (w, _) = artist_font.measure_str(&display_artist, Some(&artist_paint));
            artist_w = w;
        }
    }

    let artist_y = title_y + (18.0 * scale_factor);
    canvas.draw_str(&display_artist, (x, artist_y), &artist_font, &artist_paint);
}

fn format_duration(seconds: f32) -> String {
    let total_secs = seconds.max(0.0) as u32;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{:02}:{:02}", mins, secs)
}

pub fn draw_progress_bar(
    canvas: &Canvas,
    bar_rect: Rect,
    position_secs: f32,
    duration_secs: f32,
    scale_factor: f32,
) {
    let x = bar_rect.left;
    let y = bar_rect.top;
    let width = bar_rect.width();
    let bar_height = bar_rect.height();

    let progress = if duration_secs > 0.0 {
        (position_secs / duration_secs).clamp(0.0, 1.0)
    } else {
        0.0
    };

    // 1. Background track
    let track_rrect = RRect::new_rect_xy(bar_rect, bar_height / 2.0, bar_height / 2.0);
    let mut track_paint = Paint::default();
    track_paint.set_anti_alias(true);
    track_paint.set_color(Color::from_argb(60, 255, 255, 255));
    canvas.draw_rrect(track_rrect, &track_paint);

    // 2. Filled progress
    let fill_w = width * progress;
    if fill_w > 0.0 {
        let fill_rect = Rect::from_xywh(x, y, fill_w, bar_height);
        let fill_rrect = RRect::new_rect_xy(fill_rect, bar_height / 2.0, bar_height / 2.0);

        let mut fill_paint = Paint::default();
        fill_paint.set_anti_alias(true);
        fill_paint.set_color(Color::from_argb(230, 255, 255, 255));
        canvas.draw_rrect(fill_rrect, &fill_paint);
    }

    // 3. Thumb dot
    let mut thumb_paint = Paint::default();
    thumb_paint.set_anti_alias(true);
    thumb_paint.set_color(Color::from_argb(255, 255, 255, 255));
    canvas.draw_circle(
        (x + fill_w, y + bar_height / 2.0),
        4.0 * scale_factor,
        &thumb_paint,
    );

    // 4. Timestamps
    let fonts = crate::ui::font_cache::FontCache::get();
    let mut time_font = Font::new(fonts.regular.clone(), 10.0 * scale_factor);
    time_font.set_subpixel(true);
    time_font.set_edging(Edging::SubpixelAntiAlias);

    let mut time_paint = Paint::default();
    time_paint.set_anti_alias(true);
    time_paint.set_color(Color::from_argb(140, 255, 255, 255));

    let pos_str = format_duration(position_secs);
    let dur_str = format_duration(duration_secs);

    let text_y = y + bar_height + (12.0 * scale_factor);
    canvas.draw_str(&pos_str, (x, text_y), &time_font, &time_paint);

    let (dur_w, _) = time_font.measure_str(&dur_str, Some(&time_paint));
    canvas.draw_str(
        &dur_str,
        (x + width - dur_w, text_y),
        &time_font,
        &time_paint,
    );
}

/// Draw Modern Animated Playback Controls with scale bounce & smooth morph
pub fn draw_playback_controls(
    canvas: &Canvas,
    layout: &MediaLayout,
    anims: &ButtonAnimations,
    scale_factor: f32,
) {
    let mut btn_paint = Paint::default();
    btn_paint.set_anti_alias(true);
    btn_paint.set_style(PaintStyle::Fill);
    btn_paint.set_color(Color::from_argb(245, 255, 255, 255));

    // A. Previous Button (with click scale & left nudge animation)
    let prev_cx = layout.prev_btn.left + layout.prev_btn.width() / 2.0 + anims.prev_nudge.current;
    let prev_cy = layout.prev_btn.top + layout.prev_btn.height() / 2.0;
    let prev_scale = anims.prev_scale.current;

    canvas.save();
    canvas.translate((prev_cx, prev_cy));
    canvas.scale((prev_scale, prev_scale));
    draw_previous_icon(canvas, 0.0, 0.0, scale_factor, &btn_paint);
    canvas.restore();

    // B. Play / Pause Button (with scale pulse + smooth morphing)
    let play_cx = layout.play_btn.left + layout.play_btn.width() / 2.0;
    let play_cy = layout.play_btn.top + layout.play_btn.height() / 2.0;
    let play_scale = anims.play_scale.current;
    let morph = anims.play_morph.current.clamp(0.0, 1.0);

    canvas.save();
    canvas.translate((play_cx, play_cy));
    canvas.scale((play_scale, play_scale));
    // Morph between Play and Pause seamlessly
    draw_morphed_play_pause(canvas, 0.0, 0.0, morph, scale_factor, &btn_paint);
    canvas.restore();

    // C. Next Button (with click scale & right nudge animation)
    let next_cx = layout.next_btn.left + layout.next_btn.width() / 2.0 + anims.next_nudge.current;
    let next_cy = layout.next_btn.top + layout.next_btn.height() / 2.0;
    let next_scale = anims.next_scale.current;

    canvas.save();
    canvas.translate((next_cx, next_cy));
    canvas.scale((next_scale, next_scale));
    draw_next_icon(canvas, 0.0, 0.0, scale_factor, &btn_paint);
    canvas.restore();
}

/// Smoothly morphs icon from Play triangle (morph = 0.0) to Pause bars (morph = 1.0)
fn draw_morphed_play_pause(canvas: &Canvas, cx: f32, cy: f32, morph: f32, s: f32, paint: &Paint) {
    if morph <= 0.01 {
        // Fast path: Pure Play triangle
        draw_play_icon(canvas, cx, cy, s, paint);
    } else if morph >= 0.99 {
        // Fast path: Pure Pause bars
        draw_pause_icon(canvas, cx, cy, s, paint);
    } else {
        // Smooth interpolated cross-fade & rotation
        canvas.save();
        let rotation = morph * 90.0; // Dynamic 90-degree twist during toggle
        canvas.rotate(rotation, Some(Point::new(cx, cy)));

        if morph < 0.5 {
            let alpha = ((1.0 - morph * 2.0) * 245.0) as u8;
            let mut p = paint.clone();
            p.set_color(Color::from_argb(alpha, 255, 255, 255));
            draw_play_icon(canvas, cx, cy, s, &p);
        } else {
            let alpha = (((morph - 0.5) * 2.0) * 245.0) as u8;
            let mut p = paint.clone();
            p.set_color(Color::from_argb(alpha, 255, 255, 255));
            draw_pause_icon(canvas, cx, cy, s, &p);
        }
        canvas.restore();
    }
}

fn draw_play_icon(canvas: &Canvas, cx: f32, cy: f32, s: f32, paint: &Paint) {
    let size = 7.5 * s;
    let x_offset = 1.2 * s;
    let r = 2.4 * s;

    let p1 = Point::new(cx - size * 0.8 + x_offset, cy - size);
    let p2 = Point::new(cx + size * 1.0 + x_offset, cy);
    let p3 = Point::new(cx - size * 0.8 + x_offset, cy + size);

    let mut builder = PathBuilder::new();
    let mid_y = (p1.y + p3.y) / 2.0;
    builder.move_to(Point::new(p1.x, mid_y));
    builder.arc_to_tangent(p1, p2, r);
    builder.arc_to_tangent(p2, p3, r);
    builder.arc_to_tangent(p3, p1, r);
    builder.close();

    canvas.draw_path(&builder.detach(), paint);
}

fn draw_pause_icon(canvas: &Canvas, cx: f32, cy: f32, s: f32, paint: &Paint) {
    let bar_w = 4.0 * s;
    let bar_h = 14.0 * s;
    let gap = 5.0 * s;
    let radius = bar_w / 2.0;

    let left_bar = Rect::from_xywh(cx - gap / 2.0 - bar_w, cy - bar_h / 2.0, bar_w, bar_h);
    let right_bar = Rect::from_xywh(cx + gap / 2.0, cy - bar_h / 2.0, bar_w, bar_h);

    let r_left = RRect::new_rect_xy(left_bar, radius, radius);
    let r_right = RRect::new_rect_xy(right_bar, radius, radius);

    canvas.draw_rrect(r_left, paint);
    canvas.draw_rrect(r_right, paint);
}

fn draw_previous_icon(canvas: &Canvas, cx: f32, cy: f32, s: f32, paint: &Paint) {
    let size = 6.0 * s;
    let r = 1.8 * s;

    let p1 = Point::new(cx + size * 0.85, cy - size);
    let p2 = Point::new(cx - size * 0.85, cy);
    let p3 = Point::new(cx + size * 0.85, cy + size);

    let mut builder = PathBuilder::new();
    let mid_y = (p1.y + p3.y) / 2.0;
    builder.move_to(Point::new(p1.x, mid_y));
    builder.arc_to_tangent(p1, p2, r);
    builder.arc_to_tangent(p2, p3, r);
    builder.arc_to_tangent(p3, p1, r);
    builder.close();
    canvas.draw_path(&builder.detach(), paint);

    let bar_w = 2.5 * s;
    let bar_h = size * 2.0;
    let bar_rect = Rect::from_xywh(
        cx - size * 0.85 - bar_w - (2.0 * s),
        cy - bar_h / 2.0,
        bar_w,
        bar_h,
    );
    let r_bar = RRect::new_rect_xy(bar_rect, bar_w / 2.0, bar_w / 2.0);
    canvas.draw_rrect(r_bar, paint);
}

fn draw_next_icon(canvas: &Canvas, cx: f32, cy: f32, s: f32, paint: &Paint) {
    let size = 6.0 * s;
    let r = 1.8 * s;

    let p1 = Point::new(cx - size * 0.85, cy - size);
    let p2 = Point::new(cx + size * 0.85, cy);
    let p3 = Point::new(cx - size * 0.85, cy + size);

    let mut builder = PathBuilder::new();
    let mid_y = (p1.y + p3.y) / 2.0;
    builder.move_to(Point::new(p1.x, mid_y));
    builder.arc_to_tangent(p1, p2, r);
    builder.arc_to_tangent(p2, p3, r);
    builder.arc_to_tangent(p3, p1, r);
    builder.close();
    canvas.draw_path(&builder.detach(), paint);

    let bar_w = 2.5 * s;
    let bar_h = size * 2.0;
    let bar_rect = Rect::from_xywh(cx + size * 0.85 + (2.0 * s), cy - bar_h / 2.0, bar_w, bar_h);
    let r_bar = RRect::new_rect_xy(bar_rect, bar_w / 2.0, bar_w / 2.0);
    canvas.draw_rrect(r_bar, paint);
}
