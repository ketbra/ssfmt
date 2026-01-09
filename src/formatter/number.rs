//! Number formatting (integers, decimals, percentages, scientific notation)

use crate::ast::{DigitPlaceholder, FormatPart, Section};
use crate::error::FormatError;
use crate::options::FormatOptions;

/// Analysis of a format section's numeric structure.
#[derive(Debug, Clone)]
pub struct FormatAnalysis {
    /// Number of integer digit placeholders
    pub integer_placeholders: Vec<DigitPlaceholder>,
    /// Number of decimal digit placeholders
    pub decimal_placeholders: Vec<DigitPlaceholder>,
    /// Whether the format has a thousands separator
    pub has_thousands_separator: bool,
    /// Number of percent signs (each multiplies by 100)
    pub percent_count: usize,
    /// Parts before the number (literals, etc.)
    pub prefix_parts: Vec<FormatPart>,
    /// Parts after the number (literals, percent, etc.)
    pub suffix_parts: Vec<FormatPart>,
}

impl FormatAnalysis {
    /// Get the number of required decimal places
    pub fn decimal_places(&self) -> usize {
        self.decimal_placeholders.len()
    }

    /// Get the minimum integer digits (count of Zero placeholders)
    #[allow(dead_code)]
    pub fn min_integer_digits(&self) -> usize {
        self.integer_placeholders
            .iter()
            .filter(|p| p.is_required())
            .count()
    }
}

/// Analyze a format section to extract its numeric structure.
pub fn analyze_format(section: &Section) -> FormatAnalysis {
    let mut integer_placeholders = Vec::new();
    let mut decimal_placeholders = Vec::new();
    let mut has_thousands_separator = false;
    let mut percent_count = 0;
    let mut prefix_parts = Vec::new();
    let mut suffix_parts = Vec::new();

    let mut seen_digit = false;
    let mut after_decimal = false;
    let mut after_digits = false;

    for part in &section.parts {
        match part {
            FormatPart::Digit(placeholder) => {
                seen_digit = true;
                after_digits = false;
                if after_decimal {
                    decimal_placeholders.push(*placeholder);
                } else {
                    integer_placeholders.push(*placeholder);
                }
            }
            FormatPart::DecimalPoint => {
                after_decimal = true;
                seen_digit = true;
            }
            FormatPart::ThousandsSeparator => {
                has_thousands_separator = true;
            }
            FormatPart::Percent => {
                percent_count += 1;
                if seen_digit {
                    after_digits = true;
                    suffix_parts.push(part.clone());
                } else {
                    prefix_parts.push(part.clone());
                }
            }
            FormatPart::Literal(_) | FormatPart::Locale(_) => {
                if !seen_digit {
                    prefix_parts.push(part.clone());
                } else {
                    after_digits = true;
                    suffix_parts.push(part.clone());
                }
            }
            FormatPart::Skip(c) => {
                // Skip adds space equivalent to character width
                if !seen_digit {
                    prefix_parts.push(FormatPart::Literal(" ".to_string()));
                } else {
                    suffix_parts.push(FormatPart::Literal(" ".to_string()));
                }
                let _ = c; // suppress unused warning
            }
            _ => {
                // Handle other parts as literals in prefix/suffix
                if !seen_digit {
                    prefix_parts.push(part.clone());
                } else if after_digits {
                    suffix_parts.push(part.clone());
                }
            }
        }
    }

    // Ensure we have at least one integer placeholder for output
    if integer_placeholders.is_empty() && !after_decimal {
        integer_placeholders.push(DigitPlaceholder::Hash);
    }

    FormatAnalysis {
        integer_placeholders,
        decimal_placeholders,
        has_thousands_separator,
        percent_count,
        prefix_parts,
        suffix_parts,
    }
}

/// Format a number according to a section.
pub fn format_number(
    value: f64,
    section: &Section,
    opts: &FormatOptions,
) -> Result<String, FormatError> {
    // Check if section has any numeric placeholders
    let has_numeric_parts = section
        .parts
        .iter()
        .any(|p| matches!(p, FormatPart::Digit(_) | FormatPart::DecimalPoint));

    // If no numeric parts, just return the literals
    if !has_numeric_parts {
        let mut result = String::new();
        for part in &section.parts {
            match part {
                FormatPart::Literal(s) => result.push_str(s),
                FormatPart::Locale(locale_code) => {
                    if let Some(ref currency) = locale_code.currency {
                        result.push_str(currency);
                    }
                }
                FormatPart::Percent => result.push('%'),
                _ => {}
            }
        }
        return Ok(result);
    }

    let analysis = analyze_format(section);

    // Apply percent multiplication
    let mut adjusted_value = value.abs();
    for _ in 0..analysis.percent_count {
        adjusted_value *= 100.0;
    }

    // Round to the required decimal places
    let decimal_places = analysis.decimal_places();
    let multiplier = 10_f64.powi(decimal_places as i32);
    let rounded = (adjusted_value * multiplier).round() / multiplier;

    // Format the number with placeholders
    let formatted = format_with_placeholders(rounded, &analysis, opts);

    // Build the final result with prefix and suffix
    let result = build_result(&analysis, &formatted, opts);

    Ok(result)
}

/// Format a number according to the analysis.
fn format_with_placeholders(value: f64, analysis: &FormatAnalysis, opts: &FormatOptions) -> String {
    let decimal_places = analysis.decimal_places();

    // Split into integer and decimal parts
    let integer_part = value.trunc() as u64;
    let decimal_part = value.fract();

    // Format integer part
    let integer_str = format_integer(
        integer_part,
        &analysis.integer_placeholders,
        analysis.has_thousands_separator,
        opts,
    );

    // Format decimal part
    if decimal_places > 0 {
        let decimal_str = format_decimal(decimal_part, &analysis.decimal_placeholders, opts);
        format!(
            "{}{}{}",
            integer_str,
            opts.locale.decimal_separator,
            decimal_str
        )
    } else {
        integer_str
    }
}

/// Format the integer part with placeholders and thousands separator.
fn format_integer(
    value: u64,
    placeholders: &[DigitPlaceholder],
    use_thousands: bool,
    opts: &FormatOptions,
) -> String {
    let value_str = value.to_string();
    let value_digits: Vec<char> = value_str.chars().collect();

    let min_digits = placeholders.iter().filter(|p| p.is_required()).count();
    let output_len = value_digits.len().max(min_digits);

    let mut result = String::new();

    // Process from right to left (least significant first)
    for (digit_count, pos_from_right) in (0..output_len).enumerate() {
        let digit_index = value_digits.len() as isize - 1 - pos_from_right as isize;

        // Add thousands separator if needed (but not at position 0)
        if use_thousands && digit_count > 0 && digit_count % 3 == 0 {
            result.insert(0, opts.locale.thousands_separator);
        }

        if digit_index >= 0 {
            // We have a digit from the value
            result.insert(0, value_digits[digit_index as usize]);
        } else {
            // We need to check placeholder for padding
            let placeholder_index = placeholders.len() as isize - 1 - pos_from_right as isize;
            if placeholder_index >= 0 {
                let placeholder = placeholders[placeholder_index as usize];
                if let Some(c) = placeholder.empty_char() {
                    result.insert(0, c);
                }
            }
        }
    }

    // Handle the case where we have no digits but need at least one
    if result.is_empty() {
        // Check if we have any required placeholders
        if placeholders.iter().any(|p| p.is_required()) {
            result.push('0');
        }
    }

    result
}

/// Format the decimal part with placeholders.
fn format_decimal(value: f64, placeholders: &[DigitPlaceholder], _opts: &FormatOptions) -> String {
    if placeholders.is_empty() {
        return String::new();
    }

    // Get the decimal digits by multiplying and truncating
    let multiplier = 10_f64.powi(placeholders.len() as i32);
    let decimal_int = (value * multiplier).round() as u64;
    let decimal_str = format!("{:0>width$}", decimal_int, width = placeholders.len());
    let decimal_chars: Vec<char> = decimal_str.chars().collect();

    let mut result = String::new();
    let mut trailing_zeros_start = placeholders.len();

    // Find where trailing zeros start (for # placeholders)
    for i in (0..placeholders.len()).rev() {
        if decimal_chars.get(i) == Some(&'0') {
            if !placeholders[i].is_required() {
                trailing_zeros_start = i;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // Build result, respecting placeholder rules
    for (i, placeholder) in placeholders.iter().enumerate() {
        let ch = decimal_chars.get(i).copied().unwrap_or('0');

        if i >= trailing_zeros_start && ch == '0' && !placeholder.is_required() {
            // Skip trailing zeros for # placeholders
            if matches!(placeholder, DigitPlaceholder::Question) {
                result.push(' ');
            }
            // For Hash, we don't add anything
        } else {
            result.push(ch);
        }
    }

    result
}

/// Build the final result string with prefix and suffix parts.
fn build_result(analysis: &FormatAnalysis, formatted_number: &str, _opts: &FormatOptions) -> String {
    let mut result = String::new();

    // Add prefix parts
    for part in &analysis.prefix_parts {
        match part {
            FormatPart::Literal(s) => result.push_str(s),
            FormatPart::Locale(locale_code) => {
                if let Some(ref currency) = locale_code.currency {
                    result.push_str(currency);
                }
            }
            FormatPart::Percent => result.push('%'),
            _ => {}
        }
    }

    // Add the formatted number
    result.push_str(formatted_number);

    // Add suffix parts
    for part in &analysis.suffix_parts {
        match part {
            FormatPart::Literal(s) => result.push_str(s),
            FormatPart::Locale(locale_code) => {
                if let Some(ref currency) = locale_code.currency {
                    result.push_str(currency);
                }
            }
            FormatPart::Percent => result.push('%'),
            _ => {}
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Section;

    fn make_section(parts: Vec<FormatPart>) -> Section {
        Section {
            condition: None,
            color: None,
            parts,
        }
    }

    #[test]
    fn test_analyze_simple_integer() {
        let section = make_section(vec![FormatPart::Digit(DigitPlaceholder::Zero)]);
        let analysis = analyze_format(&section);

        assert_eq!(analysis.integer_placeholders.len(), 1);
        assert_eq!(analysis.decimal_placeholders.len(), 0);
        assert!(!analysis.has_thousands_separator);
        assert_eq!(analysis.percent_count, 0);
    }

    #[test]
    fn test_analyze_decimal_format() {
        let section = make_section(vec![
            FormatPart::Digit(DigitPlaceholder::Zero),
            FormatPart::DecimalPoint,
            FormatPart::Digit(DigitPlaceholder::Zero),
            FormatPart::Digit(DigitPlaceholder::Zero),
        ]);
        let analysis = analyze_format(&section);

        assert_eq!(analysis.integer_placeholders.len(), 1);
        assert_eq!(analysis.decimal_placeholders.len(), 2);
    }

    #[test]
    fn test_analyze_thousands() {
        let section = make_section(vec![
            FormatPart::Digit(DigitPlaceholder::Hash),
            FormatPart::ThousandsSeparator,
            FormatPart::Digit(DigitPlaceholder::Hash),
            FormatPart::Digit(DigitPlaceholder::Hash),
            FormatPart::Digit(DigitPlaceholder::Zero),
        ]);
        let analysis = analyze_format(&section);

        assert!(analysis.has_thousands_separator);
        assert_eq!(analysis.integer_placeholders.len(), 4);
    }

    #[test]
    fn test_analyze_percent() {
        let section = make_section(vec![
            FormatPart::Digit(DigitPlaceholder::Zero),
            FormatPart::Percent,
        ]);
        let analysis = analyze_format(&section);

        assert_eq!(analysis.percent_count, 1);
        assert_eq!(analysis.suffix_parts.len(), 1);
    }
}
