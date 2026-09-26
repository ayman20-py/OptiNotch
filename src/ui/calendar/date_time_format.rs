pub fn normalize_date(day: i32, month: i32, year: i32) -> (u32, u32, u32) {
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

pub fn days_in_month(month: u32, year: u32) -> u32 {
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
