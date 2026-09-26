use super::model::{CalendarState, CalendarViewMode};
use skia_safe::{Contains, Point, Rect};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CalendarAction {
    SelectDay(usize),
    PrevWeek,
    NextWeek,
    OpenMonthPicker,
    CloseMonthPicker,
    PrevMonth,
    NextMonth,
    SelectPickerDate { year: u32, month: u32, day: u32 },
    ConnectGoogle,
    None,
}

#[derive(Debug, Clone, Default)]
pub struct CalendarLayout {
    pub bounds: Rect,
    // WeekAgenda Mode
    pub month_header_btn: Rect,
    pub prev_week_btn: Rect,
    pub next_week_btn: Rect,
    pub day_buttons: [Rect; 7],
    pub connect_google_btn: Rect,
    // MonthPicker Mode
    pub picker_prev_btn: Rect,
    pub picker_next_btn: Rect,
    pub picker_close_btn: Rect,
    pub picker_cells: Vec<(Rect, u32, u32, u32)>, // (Rect, Year, Month, Day)
}

impl CalendarLayout {
    pub fn compute(
        pill_x: f32,
        pill_y: f32,
        current_w: f32,
        current_h: f32,
        scale: f32,
        state: &CalendarState,
    ) -> Self {
        let left = pill_x + (current_w * 0.45);
        let top = pill_y + (current_h * 0.25);
        let width = (current_w * 0.55) - (20.0 * scale);
        let height = current_h * 0.70;
        let bounds = Rect::from_xywh(left, top, width, height);

        match state.view_mode {
            CalendarViewMode::WeekAgenda => {
                let arrow_btn_size = 18.0 * scale;
                let col_h = 32.0 * scale;

                // 1. Month & Year Header Area (strictly covers the month/year text without touching prev arrow)
                let month_header_btn = Rect::from_xywh(
                    left - (2.0 * scale),
                    top - (2.0 * scale),
                    34.0 * scale,
                    col_h,
                );

                // 2. Previous week arrow button (between month header and day strip)
                let prev_week_btn = Rect::from_xywh(
                    left + (34.0 * scale),
                    top - (2.0 * scale),
                    arrow_btn_size,
                    col_h,
                );

                // 3. 7 Day column buttons
                let day_strip_x = left + (54.0 * scale);
                let day_strip_w = width - (54.0 * scale) - arrow_btn_size - (2.0 * scale);
                let col_w = day_strip_w / 7.0;

                let mut day_buttons = [Rect::default(); 7];
                for i in 0..7 {
                    let cx = day_strip_x + (i as f32 * col_w);
                    day_buttons[i] = Rect::from_xywh(cx, top - (2.0 * scale), col_w, col_h);
                }

                // 4. Next week arrow button (right of day strip)
                let next_week_btn = Rect::from_xywh(
                    day_strip_x + day_strip_w,
                    top - (2.0 * scale),
                    arrow_btn_size,
                    col_h,
                );

                // 5. Connect Google button (active when not connected)
                let connect_google_btn = if !state.is_google_connected {
                    let card_y = top + (42.0 * scale);
                    let btn_w = 160.0 * scale;
                    let btn_h = 24.0 * scale;
                    let btn_x = left + (width - btn_w) / 2.0;
                    let btn_y = card_y + (38.0 * scale);
                    Rect::from_xywh(btn_x, btn_y, btn_w, btn_h)
                } else {
                    Rect::default()
                };

                Self {
                    bounds,
                    month_header_btn,
                    prev_week_btn,
                    next_week_btn,
                    day_buttons,
                    connect_google_btn,
                    picker_prev_btn: Rect::default(),
                    picker_next_btn: Rect::default(),
                    picker_close_btn: Rect::default(),
                    picker_cells: Vec::new(),
                }
            }

            CalendarViewMode::MonthPicker => {
                let s = scale;
                let nav_btn_size = 20.0 * s;

                // Month Picker Navigation Header: [ < ] [ Month Year ] [ > ] ... [ Back/✕ ]
                let picker_prev_btn = Rect::from_xywh(left + (2.0 * s), top - (2.0 * s), nav_btn_size, nav_btn_size);
                let picker_next_btn = Rect::from_xywh(left + (134.0 * s), top - (2.0 * s), nav_btn_size, nav_btn_size);
                let picker_close_btn = Rect::from_xywh(left + width - (24.0 * s), top - (2.0 * s), nav_btn_size, nav_btn_size);

                // 7 Columns x 5 or 6 Rows grid for all days in month
                let grid_top = top + (32.0 * s);
                let grid_w = width - (4.0 * s);
                let col_w = grid_w / 7.0;
                let row_h = 15.5 * s;

                let (_, _, cells_data) = state.get_month_grid();
                let mut picker_cells = Vec::with_capacity(cells_data.len());

                for (idx, cell) in cells_data.iter().enumerate() {
                    let col = idx % 7;
                    let row = idx / 7;
                    let cx = left + (2.0 * s) + (col as f32 * col_w);
                    let cy = grid_top + (row as f32 * row_h);
                    let cell_rect = Rect::from_xywh(cx, cy, col_w, row_h);
                    picker_cells.push((cell_rect, cell.year, cell.month, cell.day_number));
                }

                Self {
                    bounds,
                    month_header_btn: Rect::default(),
                    prev_week_btn: Rect::default(),
                    next_week_btn: Rect::default(),
                    day_buttons: [Rect::default(); 7],
                    connect_google_btn: Rect::default(),
                    picker_prev_btn,
                    picker_next_btn,
                    picker_close_btn,
                    picker_cells,
                }
            }
        }
    }

    pub fn hit_test(&self, local_x: f32, local_y: f32, mode: CalendarViewMode) -> CalendarAction {
        let pt = Point::new(local_x, local_y);

        match mode {
            CalendarViewMode::WeekAgenda => {
                if self.prev_week_btn.contains(pt) {
                    return CalendarAction::PrevWeek;
                }
                if self.next_week_btn.contains(pt) {
                    return CalendarAction::NextWeek;
                }
                if self.month_header_btn.contains(pt) {
                    return CalendarAction::OpenMonthPicker;
                }
                if !self.connect_google_btn.is_empty() && self.connect_google_btn.contains(pt) {
                    return CalendarAction::ConnectGoogle;
                }

                for (i, btn) in self.day_buttons.iter().enumerate() {
                    if btn.contains(pt) {
                        return CalendarAction::SelectDay(i);
                    }
                }
            }

            CalendarViewMode::MonthPicker => {
                if self.picker_prev_btn.contains(pt) {
                    return CalendarAction::PrevMonth;
                }
                if self.picker_next_btn.contains(pt) {
                    return CalendarAction::NextMonth;
                }
                if self.picker_close_btn.contains(pt) {
                    return CalendarAction::CloseMonthPicker;
                }

                for (rect, y, m, d) in &self.picker_cells {
                    if rect.contains(pt) {
                        return CalendarAction::SelectPickerDate {
                            year: *y,
                            month: *m,
                            day: *d,
                        };
                    }
                }
            }
        }

        CalendarAction::None
    }
}
