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

        // Format as a number
        format_number(value, section, opts)
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
/// This provides a reasonable default representation of the value.
pub fn fallback_format(value: f64) -> String {
    // Use general number formatting with up to 10 decimal places
    let formatted = format!("{:.10}", value);

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
        // All values should use the same section
        assert_eq!(fmt.format(42.0, &opts), "42");
        assert_eq!(fmt.format(-42.0, &opts), "42");
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
