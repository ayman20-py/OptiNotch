use skia_safe::{
    font::Edging, Canvas, Color, Font, FontMgr, FontStyle, Image, Paint, PaintStyle, Path, Point,
    RRect, Rect,
};

pub fn draw_album_art(canvas: &Canvas, image: Option<&Image>, x: f32, y: f32, size: f32) {
    let dest_rect = Rect::from_xywh(x, y, size, size);
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
        // Placeholder dark square if no album art
        let mut placeholder = Paint::default();
        placeholder.set_color(Color::from_argb(255, 30, 30, 30));
        canvas.draw_rect(dest_rect, &placeholder);

        // Music disc placeholder icon dot
        let mut dot_paint = Paint::default();
        dot_paint.set_color(Color::from_argb(120, 255, 255, 255));
        canvas.draw_circle((x + size / 2.0, y + size / 2.0), size * 0.2, &dot_paint);
    }

    canvas.restore();
}

pub fn draw_media_info(
    canvas: &Canvas,
    title: &str,
    artist: &str,
    x: f32,
    y: f32,
    _max_width: f32,
    scale_factor: f32,
) {
    let font_mgr = FontMgr::new();
    let typeface = font_mgr
        .match_family_style("Lilita One", FontStyle::normal())
        .or_else(|| font_mgr.match_family_style("Segoe UI Variable Display", FontStyle::normal()))
        .or_else(|| font_mgr.match_family_style("Segoe UI", FontStyle::normal()))
        .or_else(|| font_mgr.legacy_make_typeface(None, FontStyle::normal()))
        .expect("Failed to load typeface for media info");

    // 1. Title font & paint
    let mut title_font = Font::new(typeface.clone(), 15.0 * scale_factor);
    title_font.set_subpixel(true);
    title_font.set_edging(Edging::SubpixelAntiAlias);

    let mut title_paint = Paint::default();
    title_paint.set_anti_alias(true);
    title_paint.set_color(Color::from_argb(245, 255, 255, 255));

    // Truncate title if too long
    let display_title = if title.len() > 30 {
        format!("{}...", &title[..27])
    } else {
        title.to_string()
    };

    let (_, title_metrics) = title_font.metrics();
    let title_y = y - title_metrics.ascent;
    canvas.draw_str(&display_title, (x, title_y), &title_font, &title_paint);

    // 2. Artist font & paint
    let mut artist_font = Font::new(typeface, 12.0 * scale_factor);
    artist_font.set_subpixel(true);
    artist_font.set_edging(Edging::SubpixelAntiAlias);

    let mut artist_paint = Paint::default();
    artist_paint.set_anti_alias(true);
    artist_paint.set_color(Color::from_argb(160, 255, 255, 255));

    let display_artist = if artist.len() > 35 {
        format!("{}...", &artist[..32])
    } else {
        artist.to_string()
    };

    let artist_y = title_y + (18.0 * scale_factor);
    canvas.draw_str(&display_artist, (x, artist_y), &artist_font, &artist_paint);
}

fn format_duration(seconds: f32) -> String {
    let total_secs = seconds.max(0.0) as u32;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{:02}:{:02}", mins, secs)
}

/// Draw modern thin progress bar with timestamps
pub fn draw_progress_bar(
    canvas: &Canvas,
    x: f32,
    y: f32,
    width: f32,
    position_secs: f32,
    duration_secs: f32,
    scale_factor: f32,
) {
    let bar_height = 4.0 * scale_factor;
    let progress = if duration_secs > 0.0 {
        (position_secs / duration_secs).clamp(0.0, 1.0)
    } else {
        0.0
    };

    // 1. Background track
    let track_rect = Rect::from_xywh(x, y, width, bar_height);
    let track_rrect = RRect::new_rect_xy(track_rect, bar_height / 2.0, bar_height / 2.0);

    let mut track_paint = Paint::default();
    track_paint.set_anti_alias(true);
    track_paint.set_color(Color::from_argb(60, 255, 255, 255));
    canvas.draw_rrect(track_rrect, &track_paint);

    // 2. Active filled progress
    let fill_w = width * progress;
    if fill_w > 0.0 {
        let fill_rect = Rect::from_xywh(x, y, fill_w, bar_height);
        let fill_rrect = RRect::new_rect_xy(fill_rect, bar_height / 2.0, bar_height / 2.0);

        let mut fill_paint = Paint::default();
        fill_paint.set_anti_alias(true);
        fill_paint.set_color(Color::from_argb(230, 255, 255, 255));
        canvas.draw_rrect(fill_rrect, &fill_paint);
    }

    // 3. Thumb dot at current position
    let mut thumb_paint = Paint::default();
    thumb_paint.set_anti_alias(true);
    thumb_paint.set_color(Color::from_argb(255, 255, 255, 255));
    canvas.draw_circle((x + fill_w, y + bar_height / 2.0), 4.0 * scale_factor, &thumb_paint);

    // 4. Timestamps
    let font_mgr = FontMgr::new();
    let typeface = font_mgr
        .match_family_style("Segoe UI Variable Display", FontStyle::normal())
        .or_else(|| font_mgr.match_family_style("Segoe UI", FontStyle::normal()))
        .or_else(|| font_mgr.legacy_make_typeface(None, FontStyle::normal()))
        .expect("Failed to load timestamp font");

    let mut time_font = Font::new(typeface, 10.0 * scale_factor);
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
    canvas.draw_str(&dur_str, (x + width - dur_w, text_y), &time_font, &time_paint);
}

/// Draw Playback Controls: Previous, Play/Pause, Next
pub fn draw_playback_controls(
    canvas: &Canvas,
    center_x: f32,
    center_y: f32,
    is_playing: bool,
    scale_factor: f32,
) {
    let mut btn_paint = Paint::default();
    btn_paint.set_anti_alias(true);
    btn_paint.set_style(PaintStyle::Fill);
    btn_paint.set_color(Color::from_argb(240, 255, 255, 255));

    let spacing = 36.0 * scale_factor;

    // A. Previous Button (left) |◀
    let prev_x = center_x - spacing;
    draw_previous_icon(canvas, prev_x, center_y, scale_factor, &btn_paint);

    // B. Play / Pause Button (center)
    let play_btn_radius = 16.0 * scale_factor;
    let mut play_bg_paint = Paint::default();
    play_bg_paint.set_anti_alias(true);
    play_bg_paint.set_color(Color::from_argb(40, 255, 255, 255));
    canvas.draw_circle((center_x, center_y), play_btn_radius, &play_bg_paint);

    if is_playing {
        draw_pause_icon(canvas, center_x, center_y, scale_factor, &btn_paint);
    } else {
        draw_play_icon(canvas, center_x, center_y, scale_factor, &btn_paint);
    }

    // C. Next Button (right) ▶|
    let next_x = center_x + spacing;
    draw_next_icon(canvas, next_x, center_y, scale_factor, &btn_paint);
}

fn draw_play_icon(canvas: &Canvas, cx: f32, cy: f32, s: f32, paint: &Paint) {
    let size = 6.0 * s;
    let x_offset = 1.0 * s;
    let pts = [
        Point::new(cx - size + x_offset, cy - size * 1.2),
        Point::new(cx + size * 1.2 + x_offset, cy),
        Point::new(cx - size + x_offset, cy + size * 1.2),
    ];
    let path = Path::polygon(&pts, true, None, None);
    canvas.draw_path(&path, paint);
}

fn draw_pause_icon(canvas: &Canvas, cx: f32, cy: f32, s: f32, paint: &Paint) {
    let bar_w = 3.5 * s;
    let bar_h = 12.0 * s;
    let gap = 4.0 * s;

    let left_bar = Rect::from_xywh(cx - gap / 2.0 - bar_w, cy - bar_h / 2.0, bar_w, bar_h);
    let right_bar = Rect::from_xywh(cx + gap / 2.0, cy - bar_h / 2.0, bar_w, bar_h);

    let r_left = RRect::new_rect_xy(left_bar, 1.5 * s, 1.5 * s);
    let r_right = RRect::new_rect_xy(right_bar, 1.5 * s, 1.5 * s);

    canvas.draw_rrect(r_left, paint);
    canvas.draw_rrect(r_right, paint);
}

fn draw_previous_icon(canvas: &Canvas, cx: f32, cy: f32, s: f32, paint: &Paint) {
    let size = 5.0 * s;
    let pts = [
        Point::new(cx + size, cy - size),
        Point::new(cx - size, cy),
        Point::new(cx + size, cy + size),
    ];
    let path = Path::polygon(&pts, true, None, None);
    canvas.draw_path(&path, paint);

    let bar_rect = Rect::from_xywh(cx - size - 2.5 * s, cy - size, 2.0 * s, size * 2.0);
    let r_bar = RRect::new_rect_xy(bar_rect, 1.0 * s, 1.0 * s);
    canvas.draw_rrect(r_bar, paint);
}

fn draw_next_icon(canvas: &Canvas, cx: f32, cy: f32, s: f32, paint: &Paint) {
    let size = 5.0 * s;
    let pts = [
        Point::new(cx - size, cy - size),
        Point::new(cx + size, cy),
        Point::new(cx - size, cy + size),
    ];
    let path = Path::polygon(&pts, true, None, None);
    canvas.draw_path(&path, paint);

    let bar_rect = Rect::from_xywh(cx + size + 0.5 * s, cy - size, 2.0 * s, size * 2.0);
    let r_bar = RRect::new_rect_xy(bar_rect, 1.0 * s, 1.0 * s);
    canvas.draw_rrect(r_bar, paint);
}
