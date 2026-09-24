use std::collections::HashMap;
use windows_sys::Win32::Foundation::SYSTEMTIME;
use windows_sys::Win32::System::SystemInformation::GetLocalTime;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CalendarEvent {
    pub title: String,
    pub time_str: String,
    pub color_rgb: (u8, u8, u8), // RGB accent bar
    pub is_all_day: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CalendarDay {
    pub day_name: &'static str,
    pub day_number: u32,
    pub month: u32,
    pub year: u32,
    pub is_today: bool,
    pub has_events: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalendarViewMode {
    WeekAgenda,
    MonthPicker,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CalendarMonthCell {
    pub day_number: u32,
    pub month: u32,
    pub year: u32,
    pub is_current_month: bool,
    pub is_selected: bool,
    pub is_today: bool,
    pub has_events: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CalendarState {
    pub current_year: u32,
    pub current_month: u32,
    pub current_day: u32,
    pub selected_year: u32,
    pub selected_month: u32,
    pub selected_day: u32,
    pub selected_day_index: usize, // 0..6 in week strip
    pub week_offset_days: i32,     // 0 = current week, -7 = prev week, +7 = next week
    pub picker_year: u32,          // Year currently displayed in month picker
    pub picker_month: u32,         // Month currently displayed in month picker
    pub view_mode: CalendarViewMode,
    pub is_google_connected: bool,
    pub mock_events: HashMap<(u32, u32, u32), Vec<CalendarEvent>>, // (Y, M, D) -> Events
    pub scroll_offset: f32,
    pub target_scroll: f32,
    pub max_scroll: f32,
}

impl CalendarState {
    pub fn new() -> Self {
        let mut st: SYSTEMTIME = unsafe { std::mem::zeroed() };
        unsafe { GetLocalTime(&mut st) };

        let year = st.wYear as u32;
        let month = st.wMonth as u32;
        let day = st.wDay as u32;

        let mut mock_events = HashMap::new();

        // Populate sample mock events matching the inspo reference
        mock_events.insert(
            (year, month, day),
            vec![
                CalendarEvent {
                    title: "Co-operative work term ends".to_string(),
                    time_str: "All-day".to_string(),
                    color_rgb: (245, 158, 11), // Amber gold
                    is_all_day: true,
                },
                CalendarEvent {
                    title: "Final examination".to_string(),
                    time_str: "02:00 PM - 04:30 PM".to_string(),
                    color_rgb: (245, 158, 11),
                    is_all_day: false,
                },
            ],
        );

        // Events for tomorrow / other days
        mock_events.insert(
            (year, month, (day + 1).min(28)),
            vec![
                CalendarEvent {
                    title: "Team Sprint Review".to_string(),
                    time_str: "10:00 AM - 11:00 AM".to_string(),
                    color_rgb: (59, 130, 246), // Blue
                    is_all_day: false,
                },
                CalendarEvent {
                    title: "Project OptiNotch v1.0 Launch".to_string(),
                    time_str: "04:00 PM".to_string(),
                    color_rgb: (16, 185, 129), // Emerald Green
                    is_all_day: false,
                },
            ],
        );

        // Additional sample mock event for day 15
        mock_events.insert(
            (year, month, 15),
            vec![CalendarEvent {
                title: "Design Sync & Demo".to_string(),
                time_str: "03:00 PM - 04:00 PM".to_string(),
                color_rgb: (168, 85, 247), // Purple
                is_all_day: false,
            }],
        );

        // Determine which day of the 7-day week today is (0 = Mon, ..., 6 = Sun)
        let day_of_week = if st.wDayOfWeek == 0 {
            6
        } else {
            (st.wDayOfWeek - 1) as usize
        };

        Self {
            current_year: year,
            current_month: month,
            current_day: day,
            selected_year: year,
            selected_month: month,
            selected_day: day,
            selected_day_index: day_of_week.min(6),
            week_offset_days: 0,
            picker_year: year,
            picker_month: month,
            view_mode: CalendarViewMode::WeekAgenda,
            is_google_connected: false,
            mock_events,
            scroll_offset: 0.0,
            target_scroll: 0.0,
            max_scroll: 0.0,
        }
    }

    /// Reset calendar to today's date, current week, and default WeekAgenda view
    pub fn reset_to_today(&mut self) {
        let mut st: SYSTEMTIME = unsafe { std::mem::zeroed() };
        unsafe { GetLocalTime(&mut st) };

        let year = st.wYear as u32;
        let month = st.wMonth as u32;
        let day = st.wDay as u32;
        let dow = if st.wDayOfWeek == 0 {
            6
        } else {
            (st.wDayOfWeek - 1) as usize
        };

        self.current_year = year;
        self.current_month = month;
        self.current_day = day;
        self.selected_year = year;
        self.selected_month = month;
        self.selected_day = day;
        self.selected_day_index = dow.min(6);
        self.week_offset_days = 0;
        self.picker_year = year;
        self.picker_month = month;
        self.view_mode = CalendarViewMode::WeekAgenda;
        self.scroll_offset = 0.0;
        self.target_scroll = 0.0;
    }

    /// Update live events from Google Calendar service
    pub fn update_from_google(
        &mut self,
        events: HashMap<(u32, u32, u32), Vec<CalendarEvent>>,
        is_connected: bool,
    ) {
        self.is_google_connected = is_connected;
        if is_connected {
            self.mock_events = events;
        }
    }

    /// Calculate the 7 days for the active week strip (Monday through Sunday)
    pub fn get_week_days(&self) -> (String, u32, Vec<CalendarDay>) {
        let mut st: SYSTEMTIME = unsafe { std::mem::zeroed() };
        unsafe { GetLocalTime(&mut st) };

        // 0 = Sunday in Win32, so Monday = 1
        let dow = if st.wDayOfWeek == 0 {
            6
        } else {
            (st.wDayOfWeek - 1) as i32
        };
        let monday_offset = -dow + self.week_offset_days;

        let day_names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        let mut days = Vec::with_capacity(7);

        let base_day = st.wDay as i32 + monday_offset;
        let mut display_month = st.wMonth as u32;
        let mut display_year = st.wYear as u32;

        for i in 0..7 {
            let offset_from_mon = base_day + i;
            let (d, m, y) = normalize_date(offset_from_mon, st.wMonth as i32, st.wYear as i32);
            if i == 3 {
                display_month = m;
                display_year = y;
            }

            let is_today = y == st.wYear as u32 && m == st.wMonth as u32 && d == st.wDay as u32;
            let has_events = self.mock_events.contains_key(&(y, m, d));

            days.push(CalendarDay {
                day_name: day_names[i as usize],
                day_number: d,
                month: m,
                year: y,
                is_today,
                has_events,
            });
        }

        let month_name = get_month_abbr(display_month).to_string();
        (month_name, display_year, days)
    }

    /// Calculate the 35 or 42 cells for the full month picker grid
    pub fn get_month_grid(&self) -> (String, u32, Vec<CalendarMonthCell>) {
        let y = self.picker_year;
        let m = self.picker_month;
        let first_dow = day_of_week(1, m, y); // 0 = Mon, 6 = Sun
        let total_days_in_m = days_in_month(m, y);

        let mut cells = Vec::with_capacity(42);

        // Leading days from previous month
        let (prev_m, prev_y) = if m == 1 { (12, y - 1) } else { (m - 1, y) };
        let prev_m_days = days_in_month(prev_m, prev_y);
        for i in (0..first_dow).rev() {
            let d = prev_m_days - i as u32;
            let is_today = prev_y == self.current_year
                && prev_m == self.current_month
                && d == self.current_day;
            let is_selected = prev_y == self.selected_year
                && prev_m == self.selected_month
                && d == self.selected_day;
            let has_events = self.mock_events.contains_key(&(prev_y, prev_m, d));
            cells.push(CalendarMonthCell {
                day_number: d,
                month: prev_m,
                year: prev_y,
                is_current_month: false,
                is_selected,
                is_today,
                has_events,
            });
        }

        // Days in current month
        for d in 1..=total_days_in_m {
            let is_today =
                y == self.current_year && m == self.current_month && d == self.current_day;
            let is_selected =
                y == self.selected_year && m == self.selected_month && d == self.selected_day;
            let has_events = self.mock_events.contains_key(&(y, m, d));
            cells.push(CalendarMonthCell {
                day_number: d,
                month: m,
                year: y,
                is_current_month: true,
                is_selected,
                is_today,
                has_events,
            });
        }

        // Trailing days from next month to fill complete rows (multiples of 7)
        let rem = cells.len() % 7;
        if rem != 0 {
            let needed = 7 - rem;
            let (next_m, next_y) = if m == 12 { (1, y + 1) } else { (m + 1, y) };
            for d in 1..=(needed as u32) {
                let is_today = next_y == self.current_year
                    && next_m == self.current_month
                    && d == self.current_day;
                let is_selected = next_y == self.selected_year
                    && next_m == self.selected_month
                    && d == self.selected_day;
                let has_events = self.mock_events.contains_key(&(next_y, next_m, d));
                cells.push(CalendarMonthCell {
                    day_number: d,
                    month: next_m,
                    year: next_y,
                    is_current_month: false,
                    is_selected,
                    is_today,
                    has_events,
                });
            }
        }

        let full_month_name = get_month_full_name(m).to_string();
        (full_month_name, y, cells)
    }

    /// Get the events for the currently selected day
    pub fn get_selected_day_events(&self) -> Vec<CalendarEvent> {
        let (_, _, week_days) = self.get_week_days();
        if let Some(day) = week_days.get(self.selected_day_index) {
            self.mock_events
                .get(&(day.year, day.month, day.day_number))
                .cloned()
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    pub fn select_day(&mut self, index: usize) {
        if index < 7 {
            self.selected_day_index = index;
            let (_, _, week_days) = self.get_week_days();
            if let Some(day) = week_days.get(index) {
                self.selected_year = day.year;
                self.selected_month = day.month;
                self.selected_day = day.day_number;
            }
            self.scroll_offset = 0.0;
            self.target_scroll = 0.0;
        }
    }

    pub fn select_date_from_picker(&mut self, year: u32, month: u32, day: u32) {
        self.selected_year = year;
        self.selected_month = month;
        self.selected_day = day;

        // Calculate days between today (system time) and the chosen target date
        let mut st: SYSTEMTIME = unsafe { std::mem::zeroed() };
        unsafe { GetLocalTime(&mut st) };

        let today_days = days_from_epoch(st.wYear as u32, st.wMonth as u32, st.wDay as u32);
        let target_days = days_from_epoch(year, month, day);
        let diff = target_days - today_days;

        let today_dow = if st.wDayOfWeek == 0 {
            6
        } else {
            (st.wDayOfWeek - 1) as i32
        };
        let target_dow = day_of_week(day, month, year) as i32;

        let target_mon_offset = diff - target_dow;
        let today_mon_offset = -today_dow;
        self.week_offset_days = target_mon_offset - today_mon_offset;
        self.selected_day_index = target_dow as usize;
        self.scroll_offset = 0.0;
        self.target_scroll = 0.0;

        // Switch back to WeekAgenda view
        self.view_mode = CalendarViewMode::WeekAgenda;
    }

    pub fn scroll_events(&mut self, delta: f32) {
        let count = self.get_selected_day_events().len();
        let total_h = (count as f32) * 29.0;
        let viewport_h = 75.0;
        self.max_scroll = (total_h - viewport_h).max(0.0);
        self.target_scroll = (self.target_scroll + delta).clamp(0.0, self.max_scroll);
    }

    pub fn update_scroll(&mut self, dt: f32) -> bool {
        let diff = self.target_scroll - self.scroll_offset;
        if diff.abs() > 0.5 {
            self.scroll_offset += diff * (16.0 * dt).min(1.0);
            true
        } else if (self.scroll_offset - self.target_scroll).abs() > 0.001 {
            self.scroll_offset = self.target_scroll;
            true
        } else {
            false
        }
    }

    pub fn open_month_picker(&mut self) {
        self.picker_year = self.selected_year;
        self.picker_month = self.selected_month;
        self.view_mode = CalendarViewMode::MonthPicker;
    }

    pub fn close_month_picker(&mut self) {
        self.view_mode = CalendarViewMode::WeekAgenda;
    }

    #[allow(dead_code)]
    pub fn toggle_month_picker(&mut self) {
        if self.view_mode == CalendarViewMode::MonthPicker {
            self.close_month_picker();
        } else {
            self.open_month_picker();
        }
    }

    pub fn prev_month_picker(&mut self) {
        if self.picker_month == 1 {
            self.picker_month = 12;
            self.picker_year = self.picker_year.saturating_sub(1);
        } else {
            self.picker_month -= 1;
        }
    }

    pub fn next_month_picker(&mut self) {
        if self.picker_month == 12 {
            self.picker_month = 1;
            self.picker_year += 1;
        } else {
            self.picker_month += 1;
        }
    }

    pub fn prev_week(&mut self) {
        self.week_offset_days -= 7;
        self.scroll_offset = 0.0;
        self.target_scroll = 0.0;
        self.sync_selected_date_from_week();
    }

    pub fn next_week(&mut self) {
        self.week_offset_days += 7;
        self.scroll_offset = 0.0;
        self.target_scroll = 0.0;
        self.sync_selected_date_from_week();
    }

    fn sync_selected_date_from_week(&mut self) {
        let (_, _, week_days) = self.get_week_days();
        if let Some(day) = week_days.get(self.selected_day_index) {
            self.selected_year = day.year;
            self.selected_month = day.month;
            self.selected_day = day.day_number;
        }
    }
}

fn normalize_date(day: i32, month: i32, year: i32) -> (u32, u32, u32) {
    let mut d = day;
    let mut m = month;
    let mut y = year;

    while d < 1 {
        m -= 1;
        if m < 1 {
            m = 12;
            y -= 1;
        }
        d += days_in_month(m as u32, y as u32) as i32;
    }

    while d > days_in_month(m as u32, y as u32) as i32 {
        d -= days_in_month(m as u32, y as u32) as i32;
        m += 1;
        if m > 12 {
            m = 1;
            y += 1;
        }
    }

    (d as u32, m as u32, y as u32)
}

fn days_in_month(month: u32, year: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

pub fn get_month_abbr(month: u32) -> &'static str {
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "",
    }
}

pub fn get_month_full_name(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "",
    }
}

/// Zeller-like formula to calculate day of week for a given Gregorian date: 0 = Mon, 6 = Sun
pub fn day_of_week(day: u32, month: u32, year: u32) -> usize {
    let d = day as i32;
    let mut m = month as i32;
    let mut y = year as i32;
    if m < 3 {
        m += 12;
        y -= 1;
    }
    let k = y % 100;
    let j = y / 100;
    // Zeller formula for Sunday=1..Saturday=0/7
    let h = (d + (13 * (m + 1)) / 5 + k + k / 4 + j / 4 + 5 * j) % 7;
    // Convert: h=0 -> Sat (5), h=1 -> Sun (6), h=2 -> Mon (0), h=3 -> Tue (1), etc.
    let dow = (h + 5) % 7;
    dow as usize
}

/// Total days since 0000-03-01 (Rata Die algorithm) for difference calculations
pub fn days_from_epoch(year: u32, month: u32, day: u32) -> i32 {
    let mut y = year as i32;
    let mut m = month as i32;
    if m <= 2 {
        y -= 1;
        m += 12;
    }
    let era = y / 400;
    let yoe = y - era * 400;
    let doy = (153 * (m - 3) + 2) / 5 + day as i32 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe
}
