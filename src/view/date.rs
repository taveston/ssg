//! A small hand-rolled proleptic Gregorian calendar, just enough to turn a
//! day-count into a readable date for the top bar (CLK-02) and the inset
//! status lines (INS-04). No date/time crate needed for this.

const EPOCH_YEAR: i64 = 2341;
const EPOCH_MONTH: u32 = 3;
const EPOCH_DAY: i64 = 14;

/// The mean Gregorian month, 365.2425 / 12 days.
const MEAN_DAYS_PER_MONTH: f64 = 30.4368;

const MONTH_NAMES: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i64, month: u32) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => unreachable!("month out of range"),
    }
}

pub struct CalendarDate {
    pub year: i64,
    pub month: u32,
    pub day: i64,
}

/// The calendar date `elapsed_days` (whole or fractional - the fraction is
/// dropped) after the campaign epoch, 14 Mar 2341.
pub fn date_from_elapsed_days(elapsed_days: f64) -> CalendarDate {
    let mut year = EPOCH_YEAR;
    let mut month = EPOCH_MONTH;
    let mut day = EPOCH_DAY + elapsed_days.floor() as i64;

    loop {
        if day < 1 {
            month = if month == 1 { 12 } else { month - 1 };
            if month == 12 {
                year -= 1;
            }
            day += days_in_month(year, month);
        } else {
            let dim = days_in_month(year, month);
            if day > dim {
                day -= dim;
                month = if month == 12 { 1 } else { month + 1 };
                if month == 1 {
                    year += 1;
                }
            } else {
                break;
            }
        }
    }

    CalendarDate { year, month, day }
}

/// The top bar's calendar date, day first: "07 Mar 2847" (CLK-02).
pub fn format_date(date: &CalendarDate) -> String {
    format!(
        "{:02} {} {}",
        date.day,
        MONTH_NAMES[(date.month - 1) as usize],
        date.year
    )
}

/// "2 Y 9 M AGO" style, from a number of days in the past. Approximate
/// (average month length) - this is flavour text, not a precise duration.
pub fn format_time_ago(days_ago: f64) -> String {
    let total_months = (days_ago.max(0.0) / MEAN_DAYS_PER_MONTH).floor() as i64;
    let years = total_months / 12;
    let months = total_months % 12;
    if years > 0 {
        format!("{} Y {} M AGO", years, months)
    } else {
        format!("{} M AGO", months)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(elapsed_days: f64) -> String {
        format_date(&date_from_elapsed_days(elapsed_days))
    }

    /// CLK-02: the clock's day count reads as a calendar date from the
    /// epoch, across month and year ends and leap days.
    #[test]
    fn day_counts_read_as_calendar_dates() {
        assert_eq!(date(0.0), "14 Mar 2341");
        assert_eq!(date(0.9), "14 Mar 2341", "the fraction of a day is dropped");
        assert_eq!(date(17.0), "31 Mar 2341");
        assert_eq!(date(18.0), "01 Apr 2341");
        assert_eq!(date(-14.0), "28 Feb 2341");
        assert_eq!(date(1082.0), "29 Feb 2344");
        assert_eq!(date(1083.0), "01 Mar 2344");
        assert_eq!(date(292.0), "31 Dec 2341");
        assert_eq!(date(293.0), "01 Jan 2342");
    }

    /// INS-04: a stale record's age reads in whole years and months.
    #[test]
    fn time_ago_reads_in_years_and_months() {
        assert_eq!(format_time_ago(1_006.0), "2 Y 9 M AGO");
        assert_eq!(format_time_ago(40.0), "1 M AGO");
        assert_eq!(format_time_ago(365.25), "1 Y 0 M AGO");
        assert_eq!(format_time_ago(-5.0), "0 M AGO", "never in the future");
    }
}
