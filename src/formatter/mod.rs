//! Format value formatting engine

mod date;
mod fraction;
mod number;
mod text;

pub use number::format_number;

use crate::ast::{FormatPart, NumberFormat, Section};
use crate::error::FormatError;
use crate::options::FormatOptions;

impl NumberFormat {
    /// Format a numeric value using this format code.
    ///
    /// This is an infallible method that returns a formatted string.
    /// For date formats or when precise error handling is needed,
    /// use `try_format()` instead.
    pub fn format(&self, value: f64, opts: &FormatOptions) -> String {
        match self.try_format(value, opts) {
            Ok(result) => result,
            Err(_) => fallback_format(value),
        }
    }

    /// Try to format a numeric value using this format code.
    ///
    /// Returns an error if the format cannot be applied to the value.
    pub fn try_format(&self, value: f64, opts: &FormatOptions) -> Result<String, FormatError> {
        // Handle special float values
        if value.is_nan() {
            return Ok("NaN".to_string());
        }
        if value.is_infinite() {
            return Ok(if value.is_sign_positive() {
                "Infinity"
            } else {
                "-Infinity"
            }
            .to_string());
        }

        // Select the appropriate section based on value
        let section = self.select_section(value);

        // Handle "General" format (empty section with no parts)
        // This uses fallback formatting which matches Excel's General behavior
        if section.parts.is_empty() && section.condition.is_none() {
            return Ok(fallback_format(value));
        }

        // Check if this is a date format
        if section.has_date_parts() {
            return date::format_date(value, section, opts);
        }

        // Determine if we need to add a minus sign
        // For single-section formats, we add the minus sign ourselves
        // For multi-section formats, the section handles it
        let sections = self.sections();
        let num_sections = sections.len();
        let need_minus_sign = num_sections == 1 && value < 0.0;

        // Format as a number
        let mut result = format_number(value, section, opts)?;

        // Add minus sign for single-section formats with negative values
        // But only if the result doesn't already start with a minus sign
        // (fallback_format and some other cases already include the sign)
        if need_minus_sign && !result.starts_with('-') {
            result.insert(0, '-');
        }

        Ok(result)
    }

    /// Select the appropriate format section based on the value.
    ///
    /// Section selection rules:
    /// - 1 section: used for all values
    /// - 2 sections: first for positive/zero, second for negative
    /// - 3 sections: positive, negative, zero
    /// - 4 sections: positive, negative, zero, text
    fn select_section(&self, value: f64) -> &Section {
        let sections = self.sections();

        // Check if any section has conditions
        let has_conditions = sections.iter().any(|s| s.condition.is_some());

        if has_conditions {
            // With conditions: find matching conditional, or first non-conditional
            for section in sections {
                if let Some(ref condition) = section.condition {
                    if condition.evaluate(value) {
                        return section;
                    }
                } else {
                    // No condition on this section - use it as fallback
                    return section;
                }
            }
            // Fallback to last section if nothing matched
            return sections.last().unwrap();
        }

        // Standard section selection based on value sign (no conditions)
        match sections.len() {
            0 => unreachable!("NumberFormat should always have at least one section"),
            1 => &sections[0],
            2 => {
                if value < 0.0 {
                    &sections[1]
                } else {
                    &sections[0]
                }
            }
            3 | 4 => {
                if value > 0.0 {
                    &sections[0]
                } else if value < 0.0 {
                    &sections[1]
                } else {
                    &sections[2]
                }
            }
            _ => &sections[0],
        }
    }

    /// Format a text value using this format code.
    ///
    /// If this format has a text section (4th section), it will be used.
    /// Otherwise, the text is returned as-is.
    pub fn format_text(&self, text: &str, _opts: &FormatOptions) -> String {
        let sections = self.sections();

        // Text section is the 4th section if present
        if sections.len() >= 4 {
            let text_section = &sections[3];
            let mut result = String::new();

            for part in &text_section.parts {
                match part {
                    FormatPart::TextPlaceholder => result.push_str(text),
                    FormatPart::Literal(s) => result.push_str(s),
                    _ => {}
                }
            }

            return result;
        }

        // Default: return text as-is
        text.to_string()
    }
}

/// Fallback formatting for when the format code cannot be applied.
///
/// Implements Excel's "General" number format behavior:
/// - Very small numbers (0 < |x| < 1E-4) use scientific notation
/// - Very large numbers (|x| >= 1E11) use scientific notation
/// - Up to 11 significant digits total (including decimal point)
/// - No trailing zeros after decimal point
pub fn fallback_format(value: f64) -> String {
    // Handle zero
    if value == 0.0 {
        return "0".to_string();
    }

    let abs_value = value.abs();

    // Excel's General format uses scientific notation based on magnitude and precision:
    // 1. Very small: 0 < |x| < 1E-9 -> scientific
    // 2. Very large: |x| >= 1E11 -> scientific
    // 3. In between but many sig figs: also scientific
    // The rule seems to be: values < 1E-9 OR values with >11 significant figures

    // Check if we should use scientific notation
    let use_scientific = if abs_value >= 1e11 {
        true
    } else if abs_value != 0.0 && abs_value < 1e-9 {
        // Very small numbers use scientific
        true
    } else if abs_value != 0.0 && abs_value < 1e-4 {
        // For numbers in the 1E-9 to 1E-4 range, check significant figures
        // Excel uses scientific if there are 3+ significant digits
        let test_str = format!("{:.15}", abs_value);
        // Remove leading "0." and count significant digits
        let after_decimal = test_str.trim_start_matches("0.");
        let sig_figs = after_decimal
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .filter(|c| *c != '0')
            .count();
        // Use scientific notation if 3 or more significant figures
        sig_figs >= 3
    } else {
        false
    };

    if use_scientific {
        // Format in scientific notation with up to 5 decimal places
        // Excel shows "1.23457E+12" format
        let formatted = format!("{:.5E}", value);

        // Excel uses specific scientific notation format:
        // Remove trailing zeros from mantissa, but keep at least one decimal place
        if let Some(e_pos) = formatted.find('E') {
            let (mantissa, exponent) = formatted.split_at(e_pos);
            let trimmed_mantissa = mantissa.trim_end_matches('0');
            let final_mantissa = if trimmed_mantissa.ends_with('.') {
                &trimmed_mantissa[..trimmed_mantissa.len() - 1]
            } else {
                trimmed_mantissa
            };

            // Format exponent to match Excel: E+12, E-05, etc.
            let exp_str = &exponent[1..]; // Skip 'E'
            let exp_value: i32 = exp_str.parse().unwrap_or(0);
            format!("{}E{:+03}", final_mantissa, exp_value)
        } else {
            formatted
        }
    } else {
        // Use decimal notation
        // Excel's General format shows up to 11 characters total (including decimal point)
        // but we need to be smart about significant figures

        // Try to format with enough precision to show the value accurately
        // but within Excel's 11-digit display limit
        let formatted = if abs_value >= 1.0 {
            // For numbers >= 1, format with appropriate decimal places
            let integer_digits = abs_value.log10().floor() as usize + 1;
            let decimal_places = if integer_digits >= 10 {
                0
            } else {
                (10 - integer_digits).min(10)
            };
            format!("{:.prec$}", value, prec = decimal_places)
        } else {
            // For numbers < 1, format with up to 10 decimal places
            format!("{:.10}", value)
        };

        // Trim trailing zeros after decimal point
        if formatted.contains('.') {
            let trimmed = formatted.trim_end_matches('0');
            if trimmed.ends_with('.') {
                trimmed.trim_end_matches('.').to_string()
            } else {
                trimmed.to_string()
            }
        } else {
            formatted
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Condition, DigitPlaceholder, Section};

    fn make_format(sections: Vec<Section>) -> NumberFormat {
        NumberFormat::from_sections(sections)
    }

    fn make_section(parts: Vec<FormatPart>) -> Section {
        Section {
            condition: None,
            color: None,
            parts,
        }
    }

    #[test]
    fn test_select_section_single() {
        let fmt = make_format(vec![make_section(vec![FormatPart::Digit(
            DigitPlaceholder::Zero,
        )])]);

        let opts = FormatOptions::default();
        // Single-section formats: negative values get a minus sign prefix
        assert_eq!(fmt.format(42.0, &opts), "42");
        assert_eq!(fmt.format(-42.0, &opts), "-42");
        assert_eq!(fmt.format(0.0, &opts), "0");
    }

    #[test]
    fn test_select_section_two_sections() {
        let fmt = make_format(vec![
            make_section(vec![FormatPart::Digit(DigitPlaceholder::Zero)]),
            make_section(vec![
                FormatPart::Literal("-".to_string()),
                FormatPart::Digit(DigitPlaceholder::Zero),
            ]),
        ]);

        let opts = FormatOptions::default();
        assert_eq!(fmt.format(42.0, &opts), "42");
        assert_eq!(fmt.format(-42.0, &opts), "-42");
        assert_eq!(fmt.format(0.0, &opts), "0");
    }

    #[test]
    fn test_select_section_three_sections() {
        let fmt = make_format(vec![
            make_section(vec![
                FormatPart::Literal("+".to_string()),
                FormatPart::Digit(DigitPlaceholder::Zero),
            ]),
            make_section(vec![
                FormatPart::Literal("-".to_string()),
                FormatPart::Digit(DigitPlaceholder::Zero),
            ]),
            make_section(vec![FormatPart::Literal("ZERO".to_string())]),
        ]);

        let opts = FormatOptions::default();
        assert_eq!(fmt.format(42.0, &opts), "+42");
        assert_eq!(fmt.format(-42.0, &opts), "-42");
        assert_eq!(fmt.format(0.0, &opts), "ZERO");
    }

    #[test]
    fn test_select_section_with_condition() {
        let fmt = make_format(vec![
            Section {
                condition: Some(Condition::GreaterThan(100.0)),
                color: None,
                parts: vec![FormatPart::Literal("BIG".to_string())],
            },
            make_section(vec![FormatPart::Digit(DigitPlaceholder::Zero)]),
        ]);

        let opts = FormatOptions::default();
        assert_eq!(fmt.format(150.0, &opts), "BIG");
        assert_eq!(fmt.format(50.0, &opts), "50");
    }

    #[test]
    fn test_fallback_format() {
        assert_eq!(fallback_format(42.0), "42");
        assert_eq!(fallback_format(42.5), "42.5");
        assert_eq!(fallback_format(42.123456), "42.123456");
    }

    #[test]
    fn test_format_text() {
        let fmt = make_format(vec![
            make_section(vec![FormatPart::Digit(DigitPlaceholder::Zero)]),
            make_section(vec![FormatPart::Digit(DigitPlaceholder::Zero)]),
            make_section(vec![FormatPart::Digit(DigitPlaceholder::Zero)]),
            make_section(vec![
                FormatPart::Literal("<<".to_string()),
                FormatPart::TextPlaceholder,
                FormatPart::Literal(">>".to_string()),
            ]),
        ]);

        let opts = FormatOptions::default();
        assert_eq!(fmt.format_text("hello", &opts), "<<hello>>");
    }
}
