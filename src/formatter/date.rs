//! Date and time formatting

use crate::ast::{AmPmStyle, DatePart, ElapsedPart, FormatPart, Section};
use crate::date_serial::{serial_to_date, serial_to_time, serial_to_weekday};
use crate::error::FormatError;
use crate::locale::Locale;
use crate::options::FormatOptions;

/// Format a value as a date/time using the given section.
pub fn format_date(
    value: f64,
    section: &Section,
    opts: &FormatOptions,
) -> Result<String, FormatError> {
    // Check if there's an AM/PM indicator in the format
    let has_ampm = section
        .parts
        .iter()
        .any(|p| matches!(p, FormatPart::AmPm(_)));

    // Check if there are multiple SubSecond parts (affects rounding strategy)
    let subsecond_count = section
        .parts
        .iter()
        .filter(|p| matches!(p, FormatPart::DatePart(DatePart::SubSecond(_))))
        .count();
    let has_multiple_subseconds = subsecond_count > 1;

    // Round the serial value if it's very close to an integer
    // This handles floating point precision errors like 2.9999999999999996 -> 3.0
    // which should display as 72:00:00 not 72:01:00
    let adjusted_value = if (value - value.round()).abs() < 1e-10 {
        value.round()
    } else {
        value
    };

    // Get date components
    // For time-only values (serial < 1), use a default date since we only need time
    let (year, month, day) = if value >= 1.0 {
        serial_to_date(value, opts.date_system)
            .ok_or(FormatError::DateOutOfRange { serial: value })?
    } else {
        // For time-only formatting, use epoch date as placeholder
        // The date components won't be used if format only has time parts
        (1900, 1, 1)
    };

    // Get time components
    let (hour, minute, second) = serial_to_time(adjusted_value);

    // Get weekday (1=Sunday...7=Saturday)
    // For time-only values, use Sunday as default
    let weekday = if value >= 1.0 {
        serial_to_weekday(value, opts.date_system)
    } else {
        1 // Sunday
    };

    // Build the formatted string
    let mut result = String::new();

    for part in &section.parts {
        match part {
            FormatPart::DatePart(date_part) => {
                let formatted = format_date_part(
                    *date_part,
                    year,
                    month,
                    day,
                    hour,
                    minute,
                    second,
                    weekday,
                    has_ampm,
                    value, // Pass the original serial value for fractional seconds
                    has_multiple_subseconds,
                    &opts.locale,
                );
                result.push_str(&formatted);
            }
            FormatPart::AmPm(style) => {
                let formatted = format_ampm(*style, hour, &opts.locale);
                result.push_str(&formatted);
            }
            FormatPart::Elapsed(elapsed_part) => {
                let formatted = format_elapsed(*elapsed_part, adjusted_value);
                result.push_str(&formatted);
            }
            FormatPart::Literal(s) | FormatPart::EscapedLiteral(s) => {
                result.push_str(s);
            }
            FormatPart::Skip(c) => {
                // Skip width of character - add a space for alignment
                result.push(*c);
            }
            FormatPart::Fill(_) => {
                // Fill characters are handled at a higher level
                // For now, just skip
            }
            FormatPart::ThousandsSeparator => {
                // In date formats, the thousands separator (,) is just a literal comma
                result.push(opts.locale.thousands_separator);
            }
            FormatPart::DecimalPoint => {
                // In date formats, the decimal point is just a literal
                result.push(opts.locale.decimal_separator);
            }
            _ => {
                // Other parts (e.g., numeric) are not expected in date formats
                // but we'll ignore them silently
            }
        }
    }

    Ok(result)
}

/// Format a single date/time part.
#[allow(clippy::too_many_arguments)]
fn format_date_part(
    part: DatePart,
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    weekday: u32,
    has_ampm: bool,
    serial: f64,
    has_multiple_subseconds: bool,
    locale: &Locale,
) -> String {
    match part {
        // Year formatting
        DatePart::Year2 => format!("{:02}", year % 100),
        DatePart::Year4 => format!("{:04}", year),

        // Buddhist calendar (Thai Buddhist Era)
        DatePart::BuddhistYear2 => {
            // Thai Buddhist calendar: Gregorian year + 543
            let buddhist_year = year + 543;
            format!("{:02}", buddhist_year % 100)
        }
        DatePart::BuddhistYear4 => {
            // Thai Buddhist calendar: Gregorian year + 543
            let buddhist_year = year + 543;
            format!("{:04}", buddhist_year)
        }
        DatePart::BuddhistYear4Alt => {
            // Alternative Buddhist calendar era: Gregorian year - 582
            // Used with B2yyyy prefix
            let buddhist_year = year - 582;
            format!("{:04}", buddhist_year)
        }
        DatePart::BuddhistYear2Alt => {
            // Alternative Buddhist calendar era: Gregorian year - 582
            // Used with B2yy prefix
            let buddhist_year = year - 582;
            format!("{:02}", buddhist_year % 100)
        }

        // Month formatting
        DatePart::Month => format!("{}", month),
        DatePart::Month2 => format!("{:02}", month),
        DatePart::MonthAbbr => locale.month_names_short[(month - 1) as usize].to_string(),
        DatePart::MonthFull => locale.month_names_full[(month - 1) as usize].to_string(),
        DatePart::MonthLetter => {
            // First letter of the month name
            locale.month_names_full[(month - 1) as usize]
                .chars()
                .next()
                .unwrap_or('?')
                .to_string()
        }

        // Day formatting
        DatePart::Day => format!("{}", day),
        DatePart::Day2 => format!("{:02}", day),
        DatePart::DayAbbr => {
            // weekday is 1=Sunday...7=Saturday, array is 0-indexed
            locale.day_names_short[(weekday - 1) as usize].to_string()
        }
        DatePart::DayFull => locale.day_names_full[(weekday - 1) as usize].to_string(),

        // Hour formatting
        DatePart::Hour => {
            let h = if has_ampm { to_12_hour(hour) } else { hour };
            format!("{}", h)
        }
        DatePart::Hour2 => {
            let h = if has_ampm { to_12_hour(hour) } else { hour };
            format!("{:02}", h)
        }

        // Minute formatting
        DatePart::Minute => format!("{}", minute),
        DatePart::Minute2 => format!("{:02}", minute),

        // Second formatting
        DatePart::Second => format!("{}", second),
        DatePart::Second2 => format!("{:02}", second),

        // Sub-second formatting
        DatePart::SubSecond(places) => {
            // For sub-second precision, we need the fractional seconds from the original serial
            // Calculate total seconds with fractional part
            let time_fraction = serial.fract().abs();
            let total_seconds = time_fraction * 86400.0; // seconds in a day
            let subsecond_fraction = total_seconds.fract();

            if places == 0 {
                String::new()
            } else {
                let multiplier = 10_u32.pow(places as u32);
                // Round to high precision first to handle floating point errors
                let high_precision = (subsecond_fraction * 10000.0).round() / 10000.0;

                // Use different rounding strategies based on whether there are multiple subsecond displays
                let subsec = if has_multiple_subseconds {
                    // Multiple subsecond displays: truncate for consistency
                    (high_precision * multiplier as f64) as u32 % multiplier
                } else {
                    // Single subsecond display: round
                    ((high_precision * multiplier as f64).round() as u32) % multiplier
                };
                format!("{:0width$}", subsec, width = places as usize)
            }
        }
    }
}

/// Convert 24-hour time to 12-hour format.
/// 0 -> 12, 1-12 -> 1-12, 13-23 -> 1-11
fn to_12_hour(hour: u32) -> u32 {
    match hour {
        0 => 12,
        1..=12 => hour,
        _ => hour - 12,
    }
}

/// Format AM/PM indicator.
fn format_ampm(style: AmPmStyle, hour: u32, locale: &Locale) -> String {
    let is_pm = hour >= 12;

    // Excel always outputs uppercase AM/PM regardless of format case
    match style {
        AmPmStyle::Upper | AmPmStyle::Lower => {
            if is_pm {
                locale.pm_string.to_uppercase()
            } else {
                locale.am_string.to_uppercase()
            }
        }
        AmPmStyle::ShortUpper | AmPmStyle::ShortLower => {
            if is_pm {
                "P".to_string()
            } else {
                "A".to_string()
            }
        }
    }
}

/// Format elapsed time (total hours, minutes, or seconds).
fn format_elapsed(part: ElapsedPart, serial_value: f64) -> String {
    match part {
        ElapsedPart::Hours => {
            // Total hours = serial_value * 24
            let total_hours = (serial_value * 24.0).round() as i64;
            format!("{}", total_hours)
        }
        ElapsedPart::Minutes => {
            // Total minutes = serial_value * 24 * 60
            let total_minutes = (serial_value * 24.0 * 60.0).round() as i64;
            format!("{}", total_minutes)
        }
        ElapsedPart::Seconds => {
            // Total seconds = serial_value * 24 * 60 * 60
            let total_seconds = (serial_value * 24.0 * 60.0 * 60.0).round() as i64;
            format!("{}", total_seconds)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_12_hour() {
        assert_eq!(to_12_hour(0), 12);
        assert_eq!(to_12_hour(1), 1);
        assert_eq!(to_12_hour(11), 11);
        assert_eq!(to_12_hour(12), 12);
        assert_eq!(to_12_hour(13), 1);
        assert_eq!(to_12_hour(23), 11);
    }
}
