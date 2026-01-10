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
    /// Thousands scaling factor (trailing commas divide by 1000 each)
    pub thousands_scale: usize,
    /// Literals that appear inline with integer digits (position -> literal)
    /// Position is counted from the right (0 = ones place, 1 = tens, etc.)
    pub inline_literals: Vec<(usize, String)>,
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
    let mut inline_literals = Vec::new();
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
                after_digits = true;  // Mark that integer digit sequence is complete
            }
            FormatPart::ThousandsSeparator => {
                // Regular thousands separator
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
            FormatPart::Literal(s) | FormatPart::EscapedLiteral(s) | FormatPart::Locale(crate::ast::LocaleCode { currency: Some(s), .. }) => {
                let literal_str = if let FormatPart::Literal(s) = part {
                    s.clone()
                } else if let FormatPart::EscapedLiteral(s) = part {
                    s.clone()
                } else if let FormatPart::Locale(loc) = part {
                    loc.currency.clone().unwrap_or_default()
                } else {
                    String::new()
                };

                if !seen_digit {
                    // Before any digits - prefix
                    prefix_parts.push(part.clone());
                } else if after_digits {
                    // After all digits (after decimal or after digit sequence ended) - suffix
                    suffix_parts.push(part.clone());
                } else {
                    // Among integer digits - inline literal
                    // Store the current placeholder count - we'll convert to position later
                    inline_literals.push((integer_placeholders.len(), literal_str));
                }
            }
            FormatPart::Locale(loc) if loc.currency.is_none() => {
                // Locale without currency - treat as before
                if !seen_digit {
                    prefix_parts.push(part.clone());
                } else if after_digits {
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

    // Count trailing commas by scanning backwards from the end
    // Any ThousandsSeparator after the last Digit/DecimalPoint is a trailing comma
    let mut trailing_commas = 0;
    for part in section.parts.iter().rev() {
        match part {
            FormatPart::ThousandsSeparator => {
                trailing_commas += 1;
            }
            FormatPart::Digit(_) | FormatPart::DecimalPoint => {
                // Found a digit or decimal, stop counting trailing commas
                break;
            }
            _ => {
                // Other parts (Fill, Skip, Literal) - continue scanning
            }
        }
    }
    let thousands_scale = trailing_commas;

    // Convert inline_literals from placeholder indices to positions from right
    // Inline literals are stored as (placeholder_count, string) where placeholder_count
    // is the number of placeholders added BEFORE seeing the literal.
    // This means the literal appears before placeholder at index=placeholder_count.
    // When formatting right-to-left, placeholder at index I is at position (total-1-I) from right.
    let total_placeholders = integer_placeholders.len();
    let inline_literals_converted: Vec<(usize, String)> = inline_literals
        .into_iter()
        .map(|(placeholder_count, literal)| {
            // Literal appears before placeholder[placeholder_count]
            // That placeholder is at position (total - 1 - placeholder_count) from right
            // Insert the literal AT that position (before that placeholder's digit)
            let pos_from_right = total_placeholders - placeholder_count;
            (pos_from_right, literal)
        })
        .collect();

    FormatAnalysis {
        integer_placeholders,
        decimal_placeholders,
        has_thousands_separator,
        percent_count,
        thousands_scale,
        inline_literals: inline_literals_converted,
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
    // Check if this is scientific notation
    let scientific_part = section.parts.iter().find_map(|p| {
        if let FormatPart::Scientific { upper, show_plus } = p {
            Some((*upper, *show_plus))
        } else {
            None
        }
    });

    if let Some((upper, show_plus)) = scientific_part {
        return format_scientific(value, section, upper, show_plus, opts);
    }

    // Check if this is a fraction format
    let has_fraction = section
        .parts
        .iter()
        .any(|p| matches!(p, FormatPart::Fraction { .. }));

    if has_fraction {
        return crate::formatter::fraction::format_fraction(value, section, opts);
    }

    // Check if section has any numeric placeholders
    let has_numeric_parts = section
        .parts
        .iter()
        .any(|p| matches!(p, FormatPart::Digit(_) | FormatPart::DecimalPoint));

    // Check if section has a text placeholder
    let has_text_placeholder = section
        .parts
        .iter()
        .any(|p| matches!(p, FormatPart::TextPlaceholder));

    // If we have a text placeholder and we're formatting a number,
    // use General format (fallback formatting)
    if has_text_placeholder && !has_numeric_parts {
        return Ok(crate::formatter::fallback_format(value));
    }

    // If no numeric parts and no text placeholder, check if GeneralNumber is present
    if !has_numeric_parts {
        let has_general_number = section
            .parts
            .iter()
            .any(|p| matches!(p, FormatPart::GeneralNumber));

        if has_general_number {
            // Section has GeneralNumber part - use General format + append literals
            // This handles cases like "General " where we want to format the number and add a suffix
            let mut result = crate::formatter::fallback_format(value);
            for part in &section.parts {
                match part {
                    FormatPart::Literal(s) | FormatPart::EscapedLiteral(s) => result.push_str(s),
                    FormatPart::Locale(locale_code) => {
                        if let Some(ref currency) = locale_code.currency {
                            result.push_str(currency);
                        }
                    }
                    FormatPart::Percent => result.push('%'),
                    FormatPart::Skip(_) => result.push(' '),
                    FormatPart::Fill(_) => {
                        // Fill character - for now just skip it
                    }
                    FormatPart::GeneralNumber => {
                        // Already handled - skip
                    }
                    _ => {}
                }
            }
            return Ok(result);
        } else {
            // No GeneralNumber - just return the literals without formatting the number
            let mut result = String::new();
            for part in &section.parts {
                match part {
                    FormatPart::Literal(s) | FormatPart::EscapedLiteral(s) => result.push_str(s),
                    FormatPart::Locale(locale_code) => {
                        if let Some(ref currency) = locale_code.currency {
                            result.push_str(currency);
                        }
                    }
                    FormatPart::Percent => result.push('%'),
                    FormatPart::Skip(_) => result.push(' '),
                    FormatPart::Fill(_) => {
                        // Fill character - for now just skip it in literal-only formats
                        // TODO: implement proper fill behavior with available width
                    }
                    _ => {}
                }
            }
            return Ok(result);
        }
    }

    let analysis = analyze_format(section);

    // Apply percent multiplication
    let mut adjusted_value = value.abs();
    for _ in 0..analysis.percent_count {
        adjusted_value *= 100.0;
    }

    // Apply thousands scaling (trailing commas divide by 1000 each)
    for _ in 0..analysis.thousands_scale {
        adjusted_value /= 1000.0;
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
        &analysis.inline_literals,
        opts,
    );

    // Format decimal part
    if decimal_places > 0 {
        let decimal_str = format_decimal(decimal_part, &analysis.decimal_placeholders, opts);
        format!(
            "{}{}{}",
            integer_str, opts.locale.decimal_separator, decimal_str
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
    inline_literals: &[(usize, String)],
    opts: &FormatOptions,
) -> String {
    let value_str = value.to_string();
    let value_digits: Vec<char> = value_str.chars().collect();

    let min_digits = placeholders.iter().filter(|p| p.is_required()).count();

    // Special case: if value is 0 and all placeholders are optional, return empty
    // BUT still include any inline literals
    if value == 0 && min_digits == 0 {
        let mut result = String::new();
        // Add any inline literals that would be in the optional placeholder region
        // Sort by position (descending) to add them left-to-right
        let mut sorted_literals: Vec<_> = inline_literals.iter().collect();
        sorted_literals.sort_by(|a, b| b.0.cmp(&a.0)); // Descending order

        for (_literal_pos, literal_str) in sorted_literals {
            // Add literals in order (left to right)
            result.push_str(literal_str);
        }
        return result;
    }

    // Check if placeholder types are heavily interspersed (e.g., 000#0#0#0##00##)
    // by counting transitions between different placeholder types
    // Only treat as interspersed if there are many transitions (complex format)
    let mut transitions = 0;
    for i in 1..placeholders.len() {
        if placeholders[i] != placeholders[i - 1] {
            transitions += 1;
        }
    }
    let has_interspersed_placeholders = transitions > 1;

    let output_len = if has_interspersed_placeholders {
        // Interspersed placeholders: add zero count to value digits
        // This handles formats like #0####### where placeholders are mixed
        value_digits.len() + min_digits
    } else {
        // Consecutive placeholders of same type: use maximum of value digits and required placeholders
        value_digits.len().max(min_digits)
    };

    let mut result = String::new();

    // Process from right to left (least significant first)
    for (digit_count, pos_from_right) in (0..output_len).enumerate() {
        let digit_index = value_digits.len() as isize - 1 - pos_from_right as isize;

        // Add thousands separator if needed (but not at position 0)
        if use_thousands && digit_count > 0 && digit_count % 3 == 0 {
            result.insert(0, opts.locale.thousands_separator);
        }

        // Check if there's an inline literal at this position
        // Position is from the right (0 = ones place, 1 = tens, etc.)
        // Collect all literals at this position and insert them in reverse order
        // (since we're building the string from right to left with insert(0))
        let literals_at_pos: Vec<&str> = inline_literals
            .iter()
            .filter(|(pos, _)| *pos == pos_from_right)
            .map(|(_, s)| s.as_str())
            .collect();

        // Insert in reverse order so they appear in correct order in final result
        for literal_str in literals_at_pos.iter().rev() {
            // Insert each character of this literal
            for ch in literal_str.chars().rev() {
                result.insert(0, ch);
            }
        }

        if digit_index >= 0 {
            // We have a digit from the value
            result.insert(0, value_digits[digit_index as usize]);
        } else if has_interspersed_placeholders {
            // For interspersed placeholders, padding positions use zeros
            // We've already calculated output_len = value_digits + zero_count
            // So all padding positions get '0'
            result.insert(0, '0');
        } else {
            // For consecutive placeholders, check placeholder for padding character
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

    // Insert any inline literals that are at positions beyond what we formatted
    // (literals in the leftmost optional placeholder region)
    for (literal_pos, literal_str) in inline_literals {
        if *literal_pos >= output_len {
            // This literal is to the left of all displayed digits
            // Insert it at the beginning
            for ch in literal_str.chars().rev() {
                result.insert(0, ch);
            }
        }
    }

    result
}

/// Format the decimal part with placeholders.
fn format_decimal(value: f64, placeholders: &[DigitPlaceholder], _opts: &FormatOptions) -> String {
    if placeholders.is_empty() {
        return String::new();
    }

    // Clamp to maximum precision to avoid overflow (f64 has ~15-16 significant digits)
    // This prevents overflow when casting to u64 with excessive decimal placeholders
    let effective_places = placeholders.len().min(15);

    // Get the decimal digits by multiplying and truncating
    let multiplier = 10_f64.powi(effective_places as i32);
    let decimal_int = (value * multiplier).round() as u64;
    let decimal_str = format!("{:0>width$}", decimal_int, width = effective_places);
    let decimal_chars: Vec<char> = decimal_str.chars().collect();

    let mut result = String::new();
    let mut trailing_zeros_start = placeholders.len();

    // Find where trailing zeros start (for # placeholders)
    // Only scan within effective_places to avoid index out of bounds
    for i in (0..placeholders.len().min(effective_places)).rev() {
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
        // For placeholders beyond effective precision, use '0'
        let ch = if i < effective_places {
            decimal_chars.get(i).copied().unwrap_or('0')
        } else {
            '0'
        };

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
fn build_result(
    analysis: &FormatAnalysis,
    formatted_number: &str,
    _opts: &FormatOptions,
) -> String {
    let mut result = String::new();

    // Add prefix parts
    for part in &analysis.prefix_parts {
        match part {
            FormatPart::Literal(s) | FormatPart::EscapedLiteral(s) => result.push_str(s),
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
            FormatPart::Literal(s) | FormatPart::EscapedLiteral(s) => result.push_str(s),
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

/// Format a number in scientific notation according to a format section.
fn format_scientific(
    value: f64,
    section: &Section,
    upper: bool,
    show_plus: bool,
    _opts: &FormatOptions,
) -> Result<String, FormatError> {
    // Count digits before and after decimal in mantissa, and exponent digits
    let mut mantissa_integer_places = 0;
    let mut mantissa_decimal_places = 0;
    let mut exponent_digits = 0;
    let mut seen_decimal = false;
    let mut after_exponent = false;

    for part in &section.parts {
        match part {
            FormatPart::Digit(_) if !seen_decimal && !after_exponent => {
                mantissa_integer_places += 1;
            }
            FormatPart::DecimalPoint if !after_exponent => {
                seen_decimal = true;
            }
            FormatPart::Digit(_) if seen_decimal && !after_exponent => {
                mantissa_decimal_places += 1;
            }
            FormatPart::Scientific { .. } => {
                after_exponent = true;
            }
            FormatPart::Digit(_) if after_exponent => {
                exponent_digits += 1;
            }
            _ => {}
        }
    }

    // Convert value to scientific notation
    let abs_value = value.abs();

    // Handle zero specially
    if abs_value == 0.0 {
        let zeros = "0".repeat(mantissa_decimal_places);
        let decimal_part = if mantissa_decimal_places > 0 {
            format!(".{}", zeros)
        } else {
            String::new()
        };
        let exp_char = if upper { 'E' } else { 'e' };
        let sign = if show_plus { "+" } else { "" };
        return Ok(format!("0{}{}{sign}00", decimal_part, exp_char));
    }

    // Calculate exponent based on integer placeholder count
    // Standard format (0) or minimal format (no placeholder): mantissa 1-10, exponent = log10(value)
    // Format with multiple placeholders (##0): adjust exponent to use more mantissa digits
    let base_exponent = abs_value.log10().floor() as i32;

    let exponent = if mantissa_integer_places > 1 {
        // For ##0 (3 places), we want mantissa to be in range [1, 1000)
        // Adjust exponent to be a multiple of (places-1) to group digits
        // For ##0: exponent should be multiple of 3, giving mantissa like 123.5E+6, not 1.235E+8
        let group_size = (mantissa_integer_places as i32).max(1);
        // Round exponent down to nearest multiple of group_size
        (base_exponent / group_size) * group_size
    } else {
        base_exponent
    };

    let mantissa = abs_value / 10_f64.powi(exponent);

    // Format mantissa with appropriate decimal places
    let mantissa_str = if mantissa_decimal_places > 0 {
        format!("{:.prec$}", mantissa, prec = mantissa_decimal_places)
    } else {
        format!("{:.0}", mantissa)
    };

    // Format exponent
    let exp_char = if upper { 'E' } else { 'e' };
    let exp_sign = if exponent >= 0 {
        if show_plus { "+" } else { "" }
    } else {
        "-"
    };
    let exp_abs = exponent.abs();

    // Format exponent with appropriate zero padding
    let exp_str = if exponent_digits >= 2 {
        // 0.00E+00 format uses 2-digit exponents
        format!("{:02}", exp_abs)
    } else {
        // ##0.0E+0 format uses minimal digits
        format!("{}", exp_abs)
    };
    let formatted = format!("{}{}{}{}", mantissa_str, exp_char, exp_sign, exp_str);

    // Apply sign for negative values
    if value < 0.0 {
        Ok(format!("-{}", formatted))
    } else {
        Ok(formatted)
    }
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
