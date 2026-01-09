//! Date and time formatting

use crate::ast::{AmPmStyle, DatePart, FormatPart, Section};
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
    let has_ampm = section.parts.iter().any(|p| matches!(p, FormatPart::AmPm(_)));

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
    let (hour, minute, second) = serial_to_time(value);

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
                    &opts.locale,
                );
                result.push_str(&formatted);
            }
            FormatPart::AmPm(style) => {
                let formatted = format_ampm(*style, hour, &opts.locale);
                result.push_str(&formatted);
            }
            FormatPart::Literal(s) => {
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
    locale: &Locale,
) -> String {
    match part {
        // Year formatting
        DatePart::Year2 => format!("{:02}", year % 100),
        DatePart::Year4 => format!("{:04}", year),

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
            let h = if has_ampm {
                to_12_hour(hour)
            } else {
                hour
            };
            format!("{}", h)
        }
        DatePart::Hour2 => {
            let h = if has_ampm {
                to_12_hour(hour)
            } else {
                hour
            };
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
            // For sub-second precision, we need the fractional seconds
            // The serial_to_time function rounds, so we need to recalculate
            // to get fractional seconds
            let fraction = (second as f64).fract();
            if places == 0 {
                String::new()
            } else {
                let multiplier = 10_u32.pow(places as u32);
                let subsec = ((fraction * multiplier as f64).round() as u32) % multiplier;
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

    match style {
        AmPmStyle::Upper => {
            if is_pm {
                locale.pm_string.to_uppercase()
            } else {
                locale.am_string.to_uppercase()
            }
        }
        AmPmStyle::Lower => {
            if is_pm {
                locale.pm_string.to_lowercase()
            } else {
                locale.am_string.to_lowercase()
            }
        }
        AmPmStyle::ShortUpper => {
            if is_pm {
                "P".to_string()
            } else {
                "A".to_string()
            }
        }
        AmPmStyle::ShortLower => {
            if is_pm {
                "p".to_string()
            } else {
                "a".to_string()
            }
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
