use crate::ui::font_cache::FontCache;
use crate::ui::system_status::{BatteryStatus, VolumeStatus};
use crate::window::state::ButtonAnimations;
use crate::window::NotchController;
use skia_safe::{
    font::Edging, Canvas, Color, Font, Paint, PaintStyle, PathBuilder, Point, RRect, Rect,
};

pub fn draw_header_system_bar(
    canvas: &Canvas,
    pill_x: f32,
    pill_y: f32,
    current_w: f32,
    scale: f32,
    controller: &NotchController,
    battery: &BatteryStatus,
    volume: &VolumeStatus,
) {
    let s = scale;
    let mut right_cursor = pill_x + current_w - (16.0 * s);
    let cy = pill_y + (22.0 * s);

    // 1. Redesigned Modern Monitor Switch Button (Rightmost)
    let mon_w = 38.0 * s;
    let mon_cx = right_cursor - (mon_w / 2.0);
    draw_modern_monitor_button(
        canvas,
        mon_cx,
        cy,
        s,
        controller.current_monitor,
        &controller.btn_anims,
    );
    right_cursor -= mon_w + (6.0 * s);

    // 2. Live Volume Capsule
    let vol_w = draw_volume_status(canvas, right_cursor, cy, s, volume);
    right_cursor -= vol_w + (6.0 * s);

    // 3. Battery Capsule (Only shown on devices with batteries)
    if battery.has_battery {
        draw_battery_status(canvas, right_cursor, cy, s, battery);
    }
}

/// Redesigned Modern Monitor Switch Pill Button with tactile micro-effects
fn draw_modern_monitor_button(
    canvas: &Canvas,
    cx: f32,
    cy: f32,
    s: f32,
    current_monitor: usize,
    anims: &ButtonAnimations,
) {
    let scale = anims.monitor_scale.current;
    let glow = anims.monitor_glow.current.clamp(0.0, 1.0);
    let rot = anims.monitor_rotate.current;

    canvas.save();
    canvas.translate((cx, cy));
    canvas.scale((scale, scale));
    if rot.abs() > 0.01 {
        canvas.rotate(rot, Some(Point::new(0.0, 0.0)));
    }

    let pill_w = 38.0 * s;
    let pill_h = 22.0 * s;
    let pill_rect = Rect::from_xywh(-pill_w / 2.0, -pill_h / 2.0, pill_w, pill_h);
    let pill_rrect = RRect::new_rect_xy(pill_rect, 7.0 * s, 7.0 * s);

    // 1. Dynamic Glow Halo on Click
    if glow > 0.01 {
        let mut glow_paint = Paint::default();
        glow_paint.set_anti_alias(true);
        glow_paint.set_style(PaintStyle::Stroke);
        glow_paint.set_stroke_width(2.0 * s);
        let alpha = (glow * 180.0) as u8;
        glow_paint.set_color(Color::from_argb(alpha, 59, 130, 246)); // Cyan/Blue pulse
        let halo_expand = (1.0 - glow) * 4.0 * s;
        let halo_rect = Rect::from_xywh(
            -pill_w / 2.0 - halo_expand,
            -pill_h / 2.0 - halo_expand,
            pill_w + (halo_expand * 2.0),
            pill_h + (halo_expand * 2.0),
        );
        let halo_rrect = RRect::new_rect_xy(halo_rect, 8.0 * s, 8.0 * s);
        canvas.draw_rrect(halo_rrect, &glow_paint);
    }

    // 2. Translucent Glass Capsule Fill
    let mut bg_paint = Paint::default();
    bg_paint.set_anti_alias(true);
    bg_paint.set_color(Color::from_argb(28, 255, 255, 255));
    canvas.draw_rrect(pill_rrect, &bg_paint);

    let mut border_paint = Paint::default();
    border_paint.set_anti_alias(true);
    border_paint.set_style(PaintStyle::Stroke);
    border_paint.set_stroke_width(1.0);
    border_paint.set_color(Color::from_argb(45, 255, 255, 255));
    canvas.draw_rrect(pill_rrect, &border_paint);

    // 3. Modern Display Frame Vector
    let mut mon_paint = Paint::default();
    mon_paint.set_anti_alias(true);
    mon_paint.set_style(PaintStyle::Stroke);
    mon_paint.set_stroke_width(1.1 * s);
    mon_paint.set_color(Color::from_argb(240, 255, 255, 255));

    let mon_w_box = 13.0 * s;
    let mon_h_box = 9.0 * s;
    let mon_rect = Rect::from_xywh(-pill_w / 2.0 + (6.0 * s), -mon_h_box / 2.0 - (1.0 * s), mon_w_box, mon_h_box);
    let mon_rrect = RRect::new_rect_xy(mon_rect, 1.8 * s, 1.8 * s);
    canvas.draw_rrect(mon_rrect, &mon_paint);

    // Monitor Screen Accent Fill
    let mut screen_bg = Paint::default();
    screen_bg.set_anti_alias(true);
    screen_bg.set_color(Color::from_argb(140, 59, 130, 246)); // Glowing blue screen
    let inner_rect = Rect::from_xywh(mon_rect.left + (1.2 * s), mon_rect.top + (1.2 * s), mon_w_box - (2.4 * s), mon_h_box - (2.4 * s));
    canvas.draw_rrect(RRect::new_rect_xy(inner_rect, 1.0 * s, 1.0 * s), &screen_bg);

    // Stand base
    let stand_cx = mon_rect.left + (mon_w_box / 2.0);
    let stand_top = mon_rect.bottom;
    canvas.draw_line((stand_cx, stand_top), (stand_cx, stand_top + (2.0 * s)), &mon_paint);
    canvas.draw_line((stand_cx - (2.5 * s), stand_top + (2.0 * s)), (stand_cx + (2.5 * s), stand_top + (2.0 * s)), &mon_paint);

    // 4. Display Number Badge
    let fonts = FontCache::get();
    let mut num_font = Font::new(fonts.bold.clone(), 9.5 * s);
    num_font.set_subpixel(true);
    num_font.set_edging(Edging::SubpixelAntiAlias);

    let mut num_paint = Paint::default();
    num_paint.set_anti_alias(true);
    num_paint.set_color(Color::from_argb(255, 255, 255, 255));

    let num_str = (current_monitor + 1).to_string();
    let (num_w, _) = num_font.measure_str(&num_str, Some(&num_paint));
    let (_, num_metrics) = num_font.metrics();
    let num_x = (pill_w / 2.0) - (8.5 * s) - (num_w / 2.0);
    let num_y = -(num_metrics.ascent + num_metrics.descent) / 2.0;
    canvas.draw_str(&num_str, (num_x, num_y), &num_font, &num_paint);

    canvas.restore();
}

/// Live Volume Capsule with dynamic speaker sound waves
fn draw_volume_status(canvas: &Canvas, right_x: f32, cy: f32, s: f32, volume: &VolumeStatus) -> f32 {
    let fonts = FontCache::get();
    let mut vol_font = Font::new(fonts.regular.clone(), 9.5 * s);
    vol_font.set_subpixel(true);
    vol_font.set_edging(Edging::SubpixelAntiAlias);

    let mut text_paint = Paint::default();
    text_paint.set_anti_alias(true);
    text_paint.set_color(if volume.is_muted {
        Color::from_argb(210, 239, 68, 68) // Red when muted
    } else {
        Color::from_argb(220, 255, 255, 255)
    });

    let vol_text = if volume.is_muted {
        "Mute".to_string()
    } else {
        format!("{}%", volume.percentage)
    };

    let (text_w, _) = vol_font.measure_str(&vol_text, Some(&text_paint));
    let capsule_w = text_w + (28.0 * s);
    let capsule_h = 22.0 * s;
    let capsule_rect = Rect::from_xywh(right_x - capsule_w, cy - (capsule_h / 2.0), capsule_w, capsule_h);
    let capsule_rrect = RRect::new_rect_xy(capsule_rect, 7.0 * s, 7.0 * s);

    // Glass Background
    let mut bg_paint = Paint::default();
    bg_paint.set_anti_alias(true);
    bg_paint.set_color(Color::from_argb(22, 255, 255, 255));
    canvas.draw_rrect(capsule_rrect, &bg_paint);

    let mut border_paint = Paint::default();
    border_paint.set_anti_alias(true);
    border_paint.set_style(PaintStyle::Stroke);
    border_paint.set_stroke_width(1.0);
    border_paint.set_color(Color::from_argb(35, 255, 255, 255));
    canvas.draw_rrect(capsule_rrect, &border_paint);

    // Vector Speaker Icon
    let icon_left = capsule_rect.left + (6.5 * s);
    let mut icon_paint = Paint::default();
    icon_paint.set_anti_alias(true);
    icon_paint.set_color(if volume.is_muted {
        Color::from_argb(210, 239, 68, 68)
    } else {
        Color::from_argb(240, 255, 255, 255)
    });

    // Speaker Body (Back trapezoid)
    let body_w = 4.0 * s;
    let body_h = 6.0 * s;
    let body_rect = Rect::from_xywh(icon_left, cy - (body_h / 2.0), body_w, body_h);
    canvas.draw_rrect(RRect::new_rect_xy(body_rect, 1.0 * s, 1.0 * s), &icon_paint);

    // Speaker Cone (Front triangle flared out)
    let mut cone = PathBuilder::new();
    cone.move_to(Point::new(icon_left + body_w, cy - (body_h / 2.0)));
    cone.line_to(Point::new(icon_left + body_w + (4.0 * s), cy - (6.0 * s)));
    cone.line_to(Point::new(icon_left + body_w + (4.0 * s), cy + (6.0 * s)));
    cone.line_to(Point::new(icon_left + body_w, cy + (body_h / 2.0)));
    cone.close();
    canvas.draw_path(&cone.detach(), &icon_paint);

    // Sound Wave Arcs or Mute Strike
    if volume.is_muted || volume.percentage == 0 {
        // Red Mute Strike (✕)
        let mut x_paint = Paint::default();
        x_paint.set_anti_alias(true);
        x_paint.set_style(PaintStyle::Stroke);
        x_paint.set_stroke_width(1.2 * s);
        x_paint.set_color(Color::from_argb(240, 239, 68, 68));

        let cross_cx = icon_left + (10.5 * s);
        let cross_sz = 3.0 * s;
        canvas.draw_line((cross_cx - cross_sz, cy - cross_sz), (cross_cx + cross_sz, cy + cross_sz), &x_paint);
        canvas.draw_line((cross_cx + cross_sz, cy - cross_sz), (cross_cx - cross_sz, cy + cross_sz), &x_paint);
    } else {
        let mut wave_paint = Paint::default();
        wave_paint.set_anti_alias(true);
        wave_paint.set_style(PaintStyle::Stroke);
        wave_paint.set_stroke_width(1.1 * s);
        wave_paint.set_color(Color::from_argb(210, 255, 255, 255));

        // Wave 1 (Low)
        if volume.percentage > 0 {
            let mut w1 = PathBuilder::new();
            w1.add_arc(
                Rect::from_xywh(icon_left + (4.0 * s), cy - (2.5 * s), 5.0 * s, 5.0 * s),
                -45.0,
                90.0,
            );
            canvas.draw_path(&w1.detach(), &wave_paint);
        }
        // Wave 2 (Medium)
        if volume.percentage >= 34 {
            let mut w2 = PathBuilder::new();
            w2.add_arc(
                Rect::from_xywh(icon_left + (4.0 * s), cy - (4.5 * s), 9.0 * s, 9.0 * s),
                -45.0,
                90.0,
            );
            canvas.draw_path(&w2.detach(), &wave_paint);
        }
        // Wave 3 (High)
        if volume.percentage >= 67 {
            let mut w3 = PathBuilder::new();
            w3.add_arc(
                Rect::from_xywh(icon_left + (4.0 * s), cy - (6.5 * s), 13.0 * s, 13.0 * s),
                -45.0,
                90.0,
            );
            canvas.draw_path(&w3.detach(), &wave_paint);
        }
    }

    // Text Label
    let (_, metrics) = vol_font.metrics();
    let text_x = icon_left + (16.0 * s);
    let text_y = cy - (metrics.ascent + metrics.descent) / 2.0;
    canvas.draw_str(&vol_text, (text_x, text_y), &vol_font, &text_paint);

    capsule_w
}

/// Battery Status Capsule with dynamic level fill & charging indicator
fn draw_battery_status(canvas: &Canvas, right_x: f32, cy: f32, s: f32, battery: &BatteryStatus) -> f32 {
    let fonts = FontCache::get();
    let mut bat_font = Font::new(fonts.regular.clone(), 9.5 * s);
    bat_font.set_subpixel(true);
    bat_font.set_edging(Edging::SubpixelAntiAlias);

    let bat_text = format!("{}%", battery.percentage);

    let mut text_paint = Paint::default();
    text_paint.set_anti_alias(true);
    text_paint.set_color(if battery.is_charging {
        Color::from_argb(255, 74, 222, 128) // Vibrant emerald green when charging
    } else if battery.percentage <= 20 {
        Color::from_argb(240, 239, 68, 68) // Low battery red
    } else {
        Color::from_argb(220, 255, 255, 255)
    });

    let (text_w, _) = bat_font.measure_str(&bat_text, Some(&text_paint));
    let capsule_w = text_w + (28.0 * s);
    let capsule_h = 22.0 * s;
    let capsule_rect = Rect::from_xywh(right_x - capsule_w, cy - (capsule_h / 2.0), capsule_w, capsule_h);
    let capsule_rrect = RRect::new_rect_xy(capsule_rect, 7.0 * s, 7.0 * s);

    // Glass Background (emerald tint if charging)
    let mut bg_paint = Paint::default();
    bg_paint.set_anti_alias(true);
    bg_paint.set_color(if battery.is_charging {
        Color::from_argb(35, 34, 197, 94)
    } else {
        Color::from_argb(22, 255, 255, 255)
    });
    canvas.draw_rrect(capsule_rrect, &bg_paint);

    let mut border_paint = Paint::default();
    border_paint.set_anti_alias(true);
    border_paint.set_style(PaintStyle::Stroke);
    border_paint.set_stroke_width(1.0);
    border_paint.set_color(if battery.is_charging {
        Color::from_argb(75, 34, 197, 94)
    } else {
        Color::from_argb(35, 255, 255, 255)
    });
    canvas.draw_rrect(capsule_rrect, &border_paint);

    // Vector Battery Icon
    let bx = capsule_rect.left + (6.0 * s);
    let by = cy - (4.5 * s);
    let bw = 13.0 * s;
    let bh = 9.0 * s;

    let mut shell_paint = Paint::default();
    shell_paint.set_anti_alias(true);
    shell_paint.set_style(PaintStyle::Stroke);
    shell_paint.set_stroke_width(1.1 * s);
    shell_paint.set_color(if battery.is_charging {
        Color::from_argb(240, 74, 222, 128)
    } else if battery.percentage <= 20 {
        Color::from_argb(240, 239, 68, 68)
    } else {
        Color::from_argb(210, 255, 255, 255)
    });

    // Outer Battery Body
    let b_rect = Rect::from_xywh(bx, by, bw, bh);
    canvas.draw_rrect(RRect::new_rect_xy(b_rect, 2.0 * s, 2.0 * s), &shell_paint);

    // Positive Terminal Nub on Right
    let mut nub_paint = Paint::default();
    nub_paint.set_anti_alias(true);
    nub_paint.set_color(shell_paint.color());
    let nub_rect = Rect::from_xywh(bx + bw, by + (2.5 * s), 1.5 * s, 4.0 * s);
    canvas.draw_rrect(RRect::new_rect_xy(nub_rect, 0.7 * s, 0.7 * s), &nub_paint);

    // Proportional Battery Level Fill
    let inner_w = (bw - (3.0 * s)) * (battery.percentage as f32 / 100.0).clamp(0.0, 1.0);
    if inner_w > 0.5 {
        let mut fill_paint = Paint::default();
        fill_paint.set_anti_alias(true);
        fill_paint.set_color(if battery.is_charging {
            Color::from_argb(255, 74, 222, 128)
        } else if battery.percentage <= 20 {
            Color::from_argb(240, 239, 68, 68)
        } else {
            Color::from_argb(220, 255, 255, 255)
        });
        let fill_rect = Rect::from_xywh(bx + (1.5 * s), by + (1.5 * s), inner_w, bh - (3.0 * s));
        canvas.draw_rrect(RRect::new_rect_xy(fill_rect, 1.0 * s, 1.0 * s), &fill_paint);
    }

    // Charging Lightning Bolt Icon Overlay
    if battery.is_charging {
        let mut bolt_paint = Paint::default();
        bolt_paint.set_anti_alias(true);
        bolt_paint.set_color(Color::from_argb(255, 255, 255, 255));

        let bolt_cx = bx + (bw / 2.0);
        let mut bolt = PathBuilder::new();
        bolt.move_to(Point::new(bolt_cx + (0.5 * s), by + (1.0 * s)));
        bolt.line_to(Point::new(bolt_cx - (2.0 * s), cy + (0.5 * s)));
        bolt.line_to(Point::new(bolt_cx, cy + (0.5 * s)));
        bolt.line_to(Point::new(bolt_cx - (0.5 * s), by + bh - (1.0 * s)));
        bolt.line_to(Point::new(bolt_cx + (2.0 * s), cy - (0.5 * s)));
        bolt.line_to(Point::new(bolt_cx, cy - (0.5 * s)));
        bolt.close();
        canvas.draw_path(&bolt.detach(), &bolt_paint);
    }

    // Text Label
    let (_, metrics) = bat_font.metrics();
    let text_x = bx + bw + (5.0 * s);
    let text_y = cy - (metrics.ascent + metrics.descent) / 2.0;
    canvas.draw_str(&bat_text, (text_x, text_y), &bat_font, &text_paint);

    capsule_w
}
