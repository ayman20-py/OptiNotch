use super::layout::CalendarLayout;
use super::model::{CalendarState, CalendarViewMode};
use skia_safe::{Canvas, Color, Font, FontMgr, FontStyle, Paint, RRect, Rect, font::Edging};

pub fn draw_calendar(
    canvas: &Canvas,
    state: &CalendarState,
    layout: &CalendarLayout,
    scale_factor: f32,
) {
    let s = scale_factor;

    let font_mgr = FontMgr::new();
    let regular_tf = font_mgr
        .match_family_style("Google Sans", FontStyle::normal())
        .or_else(|| font_mgr.match_family_style("Google Sans Display", FontStyle::normal()))
        .or_else(|| font_mgr.match_family_style("Product Sans", FontStyle::normal()))
        .or_else(|| font_mgr.match_family_style("Segoe UI Variable Display", FontStyle::normal()))
        .or_else(|| font_mgr.match_family_style("Segoe UI Variable Text", FontStyle::normal()))
        .or_else(|| font_mgr.match_family_style("Segoe UI", FontStyle::normal()))
        .or_else(|| font_mgr.match_family_style("Inter", FontStyle::normal()))
        .or_else(|| font_mgr.legacy_make_typeface(None, FontStyle::normal()))
        .expect("Failed to load typeface for Calendar UI");

    let bold_tf = font_mgr
        .match_family_style("Google Sans", FontStyle::bold())
        .or_else(|| font_mgr.match_family_style("Google Sans Display", FontStyle::bold()))
        .or_else(|| font_mgr.match_family_style("Product Sans", FontStyle::bold()))
        .or_else(|| font_mgr.match_family_style("Segoe UI Variable Display", FontStyle::bold()))
        .or_else(|| font_mgr.match_family_style("Segoe UI Variable Text", FontStyle::bold()))
        .or_else(|| font_mgr.match_family_style("Segoe UI", FontStyle::bold()))
        .or_else(|| font_mgr.match_family_style("Inter", FontStyle::bold()))
        .unwrap_or_else(|| regular_tf.clone());

    match state.view_mode {
        CalendarViewMode::WeekAgenda => {
            let (month_str, year_num, days) = state.get_week_days();

            // =========================================================================
            // 1. Month & Year Block (Left) - Clickable to open month picker
            // =========================================================================
            let month_x = layout.bounds.left;
            let month_y = layout.bounds.top + (10.0 * s);

            let mut month_font = Font::new(bold_tf.clone(), 15.0 * s);
            month_font.set_subpixel(true);
            month_font.set_edging(Edging::SubpixelAntiAlias);

            let mut month_paint = Paint::default();
            month_paint.set_anti_alias(true);
            month_paint.set_color(Color::from_argb(245, 255, 255, 255));
            canvas.draw_str(&month_str, (month_x, month_y), &month_font, &month_paint);

            let mut year_font = Font::new(regular_tf.clone(), 11.0 * s);
            year_font.set_subpixel(true);
            year_font.set_edging(Edging::SubpixelAntiAlias);

            let mut year_paint = Paint::default();
            year_paint.set_anti_alias(true);
            year_paint.set_color(Color::from_argb(120, 255, 255, 255));
            let year_str = year_num.to_string();
            let year_y = month_y + (14.0 * s);
            canvas.draw_str(&year_str, (month_x, year_y), &year_font, &year_paint);

            // =========================================================================
            // 2. Week Navigation Arrows (< and >)
            // =========================================================================
            let mut arrow_paint = Paint::default();
            arrow_paint.set_anti_alias(true);
            arrow_paint.set_color(Color::from_argb(140, 255, 255, 255));

            let mut arrow_font = Font::new(bold_tf.clone(), 16.0 * s);
            arrow_font.set_subpixel(true);
            arrow_font.set_edging(Edging::SubpixelAntiAlias);

            let (prev_w, _) = arrow_font.measure_str("‹", Some(&arrow_paint));
            let (next_w, _) = arrow_font.measure_str("›", Some(&arrow_paint));

            let prev_cx = layout.prev_week_btn.left + (layout.prev_week_btn.width() - prev_w) / 2.0;
            let next_cx = layout.next_week_btn.left + (layout.next_week_btn.width() - next_w) / 2.0;
            let arrow_y = layout.bounds.top + (12.0 * s);

            canvas.draw_str("‹", (prev_cx, arrow_y), &arrow_font, &arrow_paint);
            canvas.draw_str("›", (next_cx, arrow_y), &arrow_font, &arrow_paint);

            // =========================================================================
            // 3. 7 Day Columns
            // =========================================================================
            let mut day_name_font = Font::new(regular_tf.clone(), 10.0 * s);
            day_name_font.set_subpixel(true);
            day_name_font.set_edging(Edging::SubpixelAntiAlias);

            let mut num_font = Font::new(bold_tf.clone(), 11.5 * s);
            num_font.set_subpixel(true);
            num_font.set_edging(Edging::SubpixelAntiAlias);

            for (i, day) in days.iter().enumerate() {
                let btn = &layout.day_buttons[i];
                let col_cx = btn.left + btn.width() / 2.0;
                let is_selected = i == state.selected_day_index;

                // Day name (Mon, Tue, ...)
                let mut name_paint = Paint::default();
                name_paint.set_anti_alias(true);
                name_paint.set_color(if is_selected {
                    Color::from_argb(255, 30, 136, 229)
                } else {
                    Color::from_argb(110, 255, 255, 255)
                });

                let (name_w, _) = day_name_font.measure_str(day.day_name, Some(&name_paint));
                let name_x = col_cx - (name_w / 2.0);
                let name_y = layout.bounds.top + (8.0 * s);
                canvas.draw_str(day.day_name, (name_x, name_y), &day_name_font, &name_paint);

                // Day number
                let num_str = day.day_number.to_string();
                let num_cy = name_y + (13.0 * s);

                let mut num_paint = Paint::default();
                num_paint.set_anti_alias(true);
                num_paint.set_color(if is_selected {
                    Color::from_argb(255, 30, 136, 229)
                } else {
                    Color::from_argb(140, 255, 255, 255)
                });

                let (num_w, _) = num_font.measure_str(&num_str, Some(&num_paint));
                let num_x = col_cx - (num_w / 2.0);
                canvas.draw_str(&num_str, (num_x, num_cy), &num_font, &num_paint);

                // Event dot
                if day.has_events && !is_selected {
                    let mut dot_paint = Paint::default();
                    dot_paint.set_anti_alias(true);
                    dot_paint.set_color(Color::from_argb(120, 255, 255, 255));
                    canvas.draw_circle((col_cx, num_cy + (5.5 * s)), 1.5 * s, &dot_paint);
                }
            }

            // =========================================================================
            // 4. Daily Agenda / Google Tasks Feed (Bottom)
            // =========================================================================
            let events = state.get_selected_day_events();
            let agenda_y_start = layout.bounds.top + (42.0 * s);
            let event_spacing = 30.0 * s;

            let mut event_title_font = Font::new(bold_tf, 12.0 * s);
            event_title_font.set_subpixel(true);
            event_title_font.set_edging(Edging::SubpixelAntiAlias);

            let mut event_time_font = Font::new(regular_tf, 10.0 * s);
            event_time_font.set_subpixel(true);
            event_time_font.set_edging(Edging::SubpixelAntiAlias);

            let mut title_paint = Paint::default();
            title_paint.set_anti_alias(true);
            title_paint.set_color(Color::from_argb(240, 255, 255, 255));

            let mut time_paint = Paint::default();
            time_paint.set_anti_alias(true);
            time_paint.set_color(Color::from_argb(120, 255, 255, 255));

            if events.is_empty() {
                let empty_y = agenda_y_start + (14.0 * s);
                let mut empty_paint = Paint::default();
                empty_paint.set_anti_alias(true);
                empty_paint.set_color(Color::from_argb(90, 255, 255, 255));
                canvas.draw_str(
                    "No events scheduled",
                    (month_x + (4.0 * s), empty_y),
                    &event_time_font,
                    &empty_paint,
                );
            } else {
                for (i, event) in events.iter().take(2).enumerate() {
                    let ey = agenda_y_start + (i as f32 * event_spacing);

                    // Left vertical accent bar
                    let bar_rect = Rect::from_xywh(month_x + (2.0 * s), ey, 2.5 * s, 22.0 * s);
                    let bar_rrect = RRect::new_rect_xy(bar_rect, 1.2 * s, 1.2 * s);
                    let mut bar_paint = Paint::default();
                    bar_paint.set_anti_alias(true);
                    bar_paint.set_color(Color::from_argb(
                        255,
                        event.color_rgb.0,
                        event.color_rgb.1,
                        event.color_rgb.2,
                    ));
                    canvas.draw_rrect(bar_rrect, &bar_paint);

                    // Event Title
                    let title_x = month_x + (12.0 * s);
                    let title_y = ey + (9.0 * s);

                    let max_title_w = layout.bounds.width() - (16.0 * s);
                    let mut display_title = event.title.clone();
                    let (mut tw, _) = event_title_font.measure_str(&display_title, Some(&title_paint));
                    if tw > max_title_w && max_title_w > 0.0 {
                        let mut chars = event.title.chars().collect::<Vec<_>>();
                        while !chars.is_empty() && tw > max_title_w {
                            chars.pop();
                            display_title = format!("{}...", chars.iter().collect::<String>());
                            let (w, _) = event_title_font.measure_str(&display_title, Some(&title_paint));
                            tw = w;
                        }
                    }
                    canvas.draw_str(
                        &display_title,
                        (title_x, title_y),
                        &event_title_font,
                        &title_paint,
                    );

                    // Event Time / Duration
                    let time_y = title_y + (13.0 * s);
                    canvas.draw_str(
                        &event.time_str,
                        (title_x, time_y),
                        &event_time_font,
                        &time_paint,
                    );
                }
            }
        }

        CalendarViewMode::MonthPicker => {
            let (month_title, year_num, cells) = state.get_month_grid();

            // =========================================================================
            // 1. Month Picker Navigation Header: [ ‹ ] Month Year [ › ]    [ ✕ ]
            // =========================================================================
            let mut header_font = Font::new(bold_tf.clone(), 13.0 * s);
            header_font.set_subpixel(true);
            header_font.set_edging(Edging::SubpixelAntiAlias);

            let mut header_paint = Paint::default();
            header_paint.set_anti_alias(true);
            header_paint.set_color(Color::from_argb(240, 255, 255, 255));

            let title_text = format!("{} {}", month_title, year_num);
            let title_y = layout.bounds.top + (12.0 * s);
            let title_x = layout.picker_prev_btn.right + (6.0 * s);
            canvas.draw_str(&title_text, (title_x, title_y), &header_font, &header_paint);

            // Month Nav Arrows
            let mut nav_font = Font::new(bold_tf.clone(), 14.0 * s);
            nav_font.set_subpixel(true);
            nav_font.set_edging(Edging::SubpixelAntiAlias);

            let mut nav_paint = Paint::default();
            nav_paint.set_anti_alias(true);
            nav_paint.set_color(Color::from_argb(140, 255, 255, 255));

            let (prev_w, _) = nav_font.measure_str("‹", Some(&nav_paint));
            let prev_x = layout.picker_prev_btn.left + (layout.picker_prev_btn.width() - prev_w) / 2.0;
            canvas.draw_str("‹", (prev_x, title_y), &nav_font, &nav_paint);

            let (next_w, _) = nav_font.measure_str("›", Some(&nav_paint));
            let next_btn_x = layout.picker_next_btn.left + (layout.picker_next_btn.width() - next_w) / 2.0;
            canvas.draw_str("›", (next_btn_x, title_y), &nav_font, &nav_paint);

            // Close / Back button (✕)
            let mut close_font = Font::new(regular_tf.clone(), 11.0 * s);
            close_font.set_subpixel(true);
            close_font.set_edging(Edging::SubpixelAntiAlias);

            let mut close_paint = Paint::default();
            close_paint.set_anti_alias(true);
            close_paint.set_color(Color::from_argb(130, 255, 255, 255));

            let (close_w, _) = close_font.measure_str("✕", Some(&close_paint));
            let close_x = layout.picker_close_btn.left + (layout.picker_close_btn.width() - close_w) / 2.0;
            canvas.draw_str("✕", (close_x, title_y - (0.5 * s)), &close_font, &close_paint);

            // =========================================================================
            // 2. Weekday Header (Mo Tu We Th Fr Sa Su)
            // =========================================================================
            let weekday_names = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];
            let mut dow_font = Font::new(bold_tf.clone(), 9.0 * s);
            dow_font.set_subpixel(true);
            dow_font.set_edging(Edging::SubpixelAntiAlias);

            let mut dow_paint = Paint::default();
            dow_paint.set_anti_alias(true);
            dow_paint.set_color(Color::from_argb(90, 255, 255, 255));

            let dow_y = layout.bounds.top + (26.0 * s);
            let grid_left = layout.bounds.left + (2.0 * s);
            let col_w = (layout.bounds.width() - (4.0 * s)) / 7.0;

            for (col, name) in weekday_names.iter().enumerate() {
                let col_cx = grid_left + (col as f32 * col_w) + (col_w / 2.0);
                let (w, _) = dow_font.measure_str(name, Some(&dow_paint));
                canvas.draw_str(name, (col_cx - w / 2.0, dow_y), &dow_font, &dow_paint);
            }

            // =========================================================================
            // 3. Month Grid Day Numbers
            // =========================================================================
            let mut day_num_font = Font::new(bold_tf.clone(), 10.5 * s);
            day_num_font.set_subpixel(true);
            day_num_font.set_edging(Edging::SubpixelAntiAlias);

            let mut dim_num_font = Font::new(regular_tf.clone(), 10.0 * s);
            dim_num_font.set_subpixel(true);
            dim_num_font.set_edging(Edging::SubpixelAntiAlias);

            for (idx, cell) in cells.iter().enumerate() {
                if idx >= layout.picker_cells.len() {
                    break;
                }
                let (cell_rect, _, _, _) = &layout.picker_cells[idx];
                let col_cx = cell_rect.left + (cell_rect.width() / 2.0);
                let text_y = cell_rect.top + (11.0 * s);

                let num_str = cell.day_number.to_string();

                let mut paint = Paint::default();
                paint.set_anti_alias(true);

                if cell.is_selected {
                    paint.set_color(Color::from_argb(255, 30, 136, 229));
                } else if !cell.is_current_month {
                    paint.set_color(Color::from_argb(55, 255, 255, 255));
                } else if cell.is_today {
                    paint.set_color(Color::from_argb(255, 255, 255, 255));
                } else {
                    paint.set_color(Color::from_argb(180, 255, 255, 255));
                }

                let active_font = if cell.is_current_month || cell.is_selected {
                    &day_num_font
                } else {
                    &dim_num_font
                };

                let (w, _) = active_font.measure_str(&num_str, Some(&paint));
                canvas.draw_str(&num_str, (col_cx - w / 2.0, text_y), active_font, &paint);

                // Activity / Event dot under the number
                if cell.has_events {
                    let mut dot_paint = Paint::default();
                    dot_paint.set_anti_alias(true);
                    if cell.is_selected {
                        dot_paint.set_color(Color::from_argb(255, 30, 136, 229));
                    } else if cell.is_current_month {
                        dot_paint.set_color(Color::from_argb(170, 245, 158, 11)); // warm gold indicator
                    } else {
                        dot_paint.set_color(Color::from_argb(60, 245, 158, 11));
                    }
                    canvas.draw_circle((col_cx, text_y + (3.2 * s)), 1.2 * s, &dot_paint);
                }
            }
        }
    }
}

