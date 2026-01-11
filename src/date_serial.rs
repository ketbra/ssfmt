//! Date serial number conversion utilities
//!
//! Excel stores dates as serial numbers representing days since a base date:
//! - 1900 system: Day 1 = January 1, 1900 (Windows default)
//! - 1904 system: Day 1 = January 2, 1904 (Mac legacy)
//!
//! The 1900 system includes the infamous leap year bug: Excel treats 1900 as a
//! leap year (it wasn't), so day 60 is February 29, 1900 (which didn't exist).
//! Days after 60 are effectively shifted by 1 to compensate.
//!
//! Time is stored as the fractional part of the serial number:
//! - 0.5 = 12:00:00 (noon)
//! - 0.75 = 18:00:00 (6 PM)

use crate::options::DateSystem;

/// Days in each month for non-leap years
const DAYS_IN_MONTH: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

/// Returns true if the given year is a leap year
fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Returns the number of days in a given month/year
fn days_in_month(year: i32, month: u32) -> u32 {
    if month == 2 && is_leap_year(year) {
        29
    } else {
        DAYS_IN_MONTH[(month - 1) as usize]
    }
}

/// Convert an Excel serial number to a date (year, month, day).
///
/// Returns `None` if the serial number is invalid (negative or zero for some systems).
///
/// # Arguments
/// * `serial` - The Excel serial number (integer part is the date)
/// * `system` - The date system to use
///
/// # Returns
/// * `Some((year, month, day))` on success
/// * `None` if the serial number is out of range
///
/// # Excel's Leap Year Bug
/// In the 1900 system, day 60 returns (1900, 2, 29) even though February 29, 1900
/// didn't actually exist. This matches Excel's behavior.
pub fn serial_to_date(serial: f64, system: DateSystem) -> Option<(i32, u32, u32)> {
    let days = serial.floor() as i64;

    if days < 1 {
        return None;
    }

    match system {
        DateSystem::Date1900 => serial_to_date_1900(days),
        DateSystem::Date1904 => serial_to_date_1904(days),
    }
}

/// Convert serial number to date using the 1900 system
fn serial_to_date_1900(days: i64) -> Option<(i32, u32, u32)> {
    // Handle Excel's leap year bug: day 60 = Feb 29, 1900 (doesn't exist)
    if days == 60 {
        return Some((1900, 2, 29));
    }

    // For days > 60, subtract 1 to account for the phantom Feb 29, 1900
    let adjusted_days = if days > 60 { days - 1 } else { days };

    // Day 1 = Jan 1, 1900
    // Convert to days since day 0 (Dec 31, 1899)
    let mut remaining = adjusted_days;

    let mut year = 1900i32;
    let mut month = 1u32;

    // Find the year
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining <= days_in_year as i64 {
            break;
        }
        remaining -= days_in_year as i64;
        year += 1;
    }

    // Find the month
    while remaining > days_in_month(year, month) as i64 {
        remaining -= days_in_month(year, month) as i64;
        month += 1;
    }

    let day = remaining as u32;

    Some((year, month, day))
}

/// Convert serial number to date using the 1904 system
fn serial_to_date_1904(days: i64) -> Option<(i32, u32, u32)> {
    // In the 1904 system, day 1 = January 2, 1904
    // (day 0 = January 1, 1904)
    let mut year = 1904i32;
    let mut month = 1u32;

    // Start from Jan 1, 1904 (day 0)
    // Day 1 = Jan 2, 1904
    let mut day = 1u32 + days as u32;

    // Normalize the date
    loop {
        let dim = days_in_month(year, month);
        if day <= dim {
            break;
        }
        day -= dim;
        month += 1;
        if month > 12 {
            month = 1;
            year += 1;
        }
    }

    Some((year, month, day))
}

/// Extract the time components (hours, minutes, seconds) from a serial number.
///
/// The time is the fractional part of the serial number:
/// - 0.0 = 00:00:00
/// - 0.5 = 12:00:00
/// - 0.75 = 18:00:00
///
/// # Arguments
/// * `serial` - The Excel serial number (fractional part is the time)
///
/// # Returns
/// * `(hours, minutes, seconds)` where hours is 0-23, minutes and seconds are 0-59
pub fn serial_to_time(serial: f64) -> (u32, u32, u32) {
    serial_to_time_impl(serial, true)
}

/// Convert Excel serial number to time components (hour, minute, second).
/// The `round_seconds` parameter controls whether to round fractional seconds.
/// Set to false when the format includes subsecond display (.0, .00, etc.).
pub fn serial_to_time_with_rounding(serial: f64, round_seconds: bool) -> (u32, u32, u32) {
    serial_to_time_impl(serial, round_seconds)
}

fn serial_to_time_impl(serial: f64, round_seconds: bool) -> (u32, u32, u32) {
    // Get the fractional part (time component)
    let fraction = serial.fract().abs();

    // Convert to total seconds in a day (86400 seconds)
    let total_seconds = if round_seconds {
        // Round to handle fractional seconds close to the next second
        // Excel rounds seconds when displaying time without subseconds
        (fraction * 86400.0).round() as u32
    } else {
        // Truncate when format includes subsecond display
        // This allows .0, .00, etc. to show the fractional part correctly
        (fraction * 86400.0) as u32
    };

    let hours = (total_seconds / 3600) % 24;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    (hours, minutes, seconds)
}

/// Convert a date (year, month, day) to an Excel serial number.
///
/// # Arguments
/// * `year` - The year (e.g., 2026)
/// * `month` - The month (1-12)
/// * `day` - The day of month (1-31)
/// * `system` - The date system to use
///
/// # Returns
/// The Excel serial number for the given date
pub fn date_to_serial(year: i32, month: u32, day: u32, system: DateSystem) -> f64 {
    match system {
        DateSystem::Date1900 => date_to_serial_1900(year, month, day),
        DateSystem::Date1904 => date_to_serial_1904(year, month, day),
    }
}

/// Convert date to serial using the 1900 system
fn date_to_serial_1900(year: i32, month: u32, day: u32) -> f64 {
    // Special case for the phantom Feb 29, 1900
    if year == 1900 && month == 2 && day == 29 {
        return 60.0;
    }

    let mut serial: i64 = 0;

    // Add days for complete years from 1900
    for y in 1900..year {
        serial += if is_leap_year(y) { 366 } else { 365 };
    }

    // Add days for complete months in the current year
    for m in 1..month {
        serial += days_in_month(year, m) as i64;
    }

    // Add the day of month
    serial += day as i64;

    // Add 1 for the leap year bug (for dates after Feb 28, 1900)
    if serial >= 60 {
        serial += 1;
    }

    serial as f64
}

/// Convert date to serial using the 1904 system
fn date_to_serial_1904(year: i32, month: u32, day: u32) -> f64 {
    // Day 0 = January 1, 1904
    // Day 1 = January 2, 1904
    let mut serial: i64 = 0;

    // Add days for complete years from 1904
    for y in 1904..year {
        serial += if is_leap_year(y) { 366 } else { 365 };
    }

    // Add days for complete months in the current year
    for m in 1..month {
        serial += days_in_month(year, m) as i64;
    }

    // Add the day of month minus 1 (since day 0 = Jan 1)
    serial += (day - 1) as i64;

    serial as f64
}

/// Get the day of the week from a serial number.
///
/// # Arguments
/// * `serial` - The Excel serial number
/// * `system` - The date system to use
///
/// # Returns
/// Day of week: 1 = Sunday, 2 = Monday, ..., 7 = Saturday
/// (matches Excel's WEEKDAY function with return_type=1)
pub fn serial_to_weekday(serial: f64, system: DateSystem) -> u32 {
    let days = serial.floor() as i64;

    match system {
        DateSystem::Date1900 => {
            // Day 1 (Jan 1, 1900) was a Sunday (day 1)
            // But we need to account for the leap year bug
            ((days - 1) % 7 + 1) as u32
        }
        DateSystem::Date1904 => {
            // Day 0 (Jan 1, 1904) was a Friday
            // Day 1 (Jan 2, 1904) was a Saturday
            let adjusted = (days + 5) % 7 + 1; // +5 because Friday=6, and we want Sunday=1
            adjusted as u32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_leap_year() {
        assert!(!is_leap_year(1900)); // Not a leap year (divisible by 100 but not 400)
        assert!(is_leap_year(2000)); // Leap year (divisible by 400)
        assert!(is_leap_year(2024)); // Leap year (divisible by 4)
        assert!(!is_leap_year(2023)); // Not a leap year
    }

    #[test]
    fn test_days_in_month() {
        assert_eq!(days_in_month(2024, 2), 29); // Leap year
        assert_eq!(days_in_month(2023, 2), 28); // Non-leap year
        assert_eq!(days_in_month(2024, 1), 31);
        assert_eq!(days_in_month(2024, 4), 30);
    }

    #[test]
    fn test_serial_to_date_1900_early() {
        // Day 1 = Jan 1, 1900
        assert_eq!(
            serial_to_date(1.0, DateSystem::Date1900),
            Some((1900, 1, 1))
        );
        // Day 2 = Jan 2, 1900
        assert_eq!(
            serial_to_date(2.0, DateSystem::Date1900),
            Some((1900, 1, 2))
        );
        // Day 31 = Jan 31, 1900
        assert_eq!(
            serial_to_date(31.0, DateSystem::Date1900),
            Some((1900, 1, 31))
        );
        // Day 32 = Feb 1, 1900
        assert_eq!(
            serial_to_date(32.0, DateSystem::Date1900),
            Some((1900, 2, 1))
        );
    }

    #[test]
    fn test_serial_to_date_leap_year_bug() {
        // Day 59 = Feb 28, 1900
        assert_eq!(
            serial_to_date(59.0, DateSystem::Date1900),
            Some((1900, 2, 28))
        );
        // Day 60 = Feb 29, 1900 (Excel's bug - this date doesn't exist)
        assert_eq!(
            serial_to_date(60.0, DateSystem::Date1900),
            Some((1900, 2, 29))
        );
        // Day 61 = Mar 1, 1900
        assert_eq!(
            serial_to_date(61.0, DateSystem::Date1900),
            Some((1900, 3, 1))
        );
    }

    #[test]
    fn test_roundtrip_1900() {
        // Test roundtrip for various dates
        for &(y, m, d) in &[
            (1900, 1, 1),
            (1900, 3, 1),
            (2000, 2, 29), // Leap year
            (2024, 12, 31),
            (2026, 1, 9),
        ] {
            let serial = date_to_serial(y, m, d, DateSystem::Date1900);
            let (y2, m2, d2) = serial_to_date(serial, DateSystem::Date1900).unwrap();
            assert_eq!(
                (y, m, d),
                (y2, m2, d2),
                "Roundtrip failed for {}-{}-{}",
                y,
                m,
                d
            );
        }
    }
}
