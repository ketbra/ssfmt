//! Fraction formatting

use crate::ast::{DigitPlaceholder, FormatPart, FractionDenom, Section};
use crate::error::FormatError;
use crate::options::FormatOptions;

/// Format a fraction part (numerator or denominator) with digit placeholders.
fn format_fraction_part(value: u32, placeholders: &[DigitPlaceholder]) -> String {
    if placeholders.is_empty() {
        return value.to_string();
    }

    let value_str = value.to_string();
    let value_digits: Vec<char> = value_str.chars().collect();

    // If we have more digits than placeholders, show all digits
    if value_digits.len() > placeholders.len() {
        return value_str;
    }

    let mut result = String::new();

    // Process from right to left
    for (pos_from_right, _) in (0..placeholders.len()).enumerate() {
        let digit_index = value_digits.len() as isize - 1 - pos_from_right as isize;
        let placeholder_index = placeholders.len() - 1 - pos_from_right;
        let placeholder = placeholders[placeholder_index];

        if digit_index >= 0 {
            // We have a digit from the value
            result.insert(0, value_digits[digit_index as usize]);
        } else {
            // Use placeholder's empty character
            if let Some(c) = placeholder.empty_char() {
                result.insert(0, c);
            }
        }
    }

    result
}

/// Format a number as a fraction according to the format section.
pub fn format_fraction(
    value: f64,
    section: &Section,
    _opts: &FormatOptions,
) -> Result<String, FormatError> {
    // Find the fraction part in the section
    let fraction_part = section.parts.iter().find_map(|p| {
        if let FormatPart::Fraction {
            integer_digits,
            numerator_digits,
            denominator,
        } = p
        {
            Some((integer_digits, numerator_digits, denominator))
        } else {
            None
        }
    });

    let Some((integer_digits, numerator_digits, denominator)) = fraction_part else {
        return Err(FormatError::TypeMismatch {
            expected: "fraction format",
            got: "no fraction part found",
        });
    };

    // Separate integer and fractional parts
    let abs_value = value.abs();
    let mut integer_part = abs_value.trunc() as i64;
    let frac_part = abs_value.fract();

    // Determine if this is a mixed fraction or improper fraction
    let is_mixed = !integer_digits.is_empty();

    // Find best fraction approximation
    let (mut num, denom) = if is_mixed {
        // Mixed fraction: approximate the fractional part only
        match denominator {
            FractionDenom::UpToDigits(digits) => {
                let max_denom = 10_u32.pow(*digits as u32) - 1;
                find_best_fraction(frac_part, max_denom)
            }
            FractionDenom::Fixed(d) => {
                let num = (frac_part * (*d as f64)).round() as u32;
                (num, *d)
            }
        }
    } else {
        // Improper fraction: approximate the entire value
        match denominator {
            FractionDenom::UpToDigits(digits) => {
                let max_denom = 10_u32.pow(*digits as u32) - 1;
                find_best_fraction(abs_value, max_denom)
            }
            FractionDenom::Fixed(d) => {
                let num = (abs_value * (*d as f64)).round() as u32;
                (num, *d)
            }
        }
    };

    // If fraction rounds to 1 or more (mixed fraction only), add to integer part
    if is_mixed && num >= denom && denom > 0 {
        let whole = num / denom;
        integer_part += whole as i64;
        num %= denom;
    }

    // Format the result
    let mut result = String::new();

    // Add sign for negative values
    if value < 0.0 {
        result.push('-');
    }

    // Format integer part (mixed fractions only)
    if is_mixed {
        if integer_part > 0 {
            // Non-zero integer: show the number
            result.push_str(&format!("{}", integer_part));
        } else if !integer_digits.is_empty() {
            // Zero integer with placeholders: show spaces (Excel behavior)
            for placeholder in integer_digits {
                // For fractions, Excel treats all placeholder types as spaces when value is 0
                let c = placeholder.empty_char().unwrap_or(' ');
                result.push(c);
            }
        }
        // Add space between integer and fraction
        result.push(' ');
    }

    // Format the fraction part
    // For mixed fractions with no fractional part (num=0), use spaces instead of "0/1"
    if is_mixed && num == 0 {
        // Add spaces instead of "0/X"
        // Space for numerator
        for _ in numerator_digits {
            result.push(' ');
        }
        result.push(' '); // Space for the slash
        // Space for denominator
        let denom_width = match denominator {
            FractionDenom::UpToDigits(d) => *d as usize,
            FractionDenom::Fixed(_) => format!("{}", denom).len(),
        };
        for _ in 0..denom_width {
            result.push(' ');
        }
    } else {
        // Format numerator with proper placeholder handling
        let num_str = format_fraction_part(num, numerator_digits);
        result.push_str(&num_str);

        result.push('/');

        // Format denominator with padding
        let denom_str = format!("{}", denom);
        let denom_width = match denominator {
            FractionDenom::UpToDigits(d) => *d as usize,
            FractionDenom::Fixed(_) => denom_str.len(),
        };
        result.push_str(&denom_str);
        for _ in 0..(denom_width.saturating_sub(denom_str.len())) {
            result.push(' ');
        }
    }

    Ok(result)
}

/// Find the best fraction approximation for a decimal value.
/// Uses continued fractions algorithm for best rational approximation.
fn find_best_fraction(value: f64, max_denom: u32) -> (u32, u32) {
    if value == 0.0 || max_denom == 0 {
        return (0, 1);
    }

    // Handle values very close to 0
    if value.abs() < 1e-10 {
        return (0, 1);
    }

    // Use continued fractions algorithm
    let mut x = value;
    let mut a = x.floor();
    let mut h = [a as i64, 1];
    let mut k = [1_i64, 0];

    let mut n = 0;
    while n < 20 {
        // Limit iterations
        if (x - a).abs() < 1e-10 {
            break;
        }

        x = 1.0 / (x - a);
        a = x.floor();

        let h_next = a as i64 * h[0] + h[1];
        let k_next = a as i64 * k[0] + k[1];

        // Check if denominator exceeds limit
        if k_next > max_denom as i64 {
            // Return previous convergent
            break;
        }

        h[1] = h[0];
        h[0] = h_next;
        k[1] = k[0];
        k[0] = k_next;

        n += 1;
    }

    // Ensure we don't exceed max denominator
    if k[0] > max_denom as i64 {
        // Fall back to simple rounding
        let denom = max_denom.min(10);
        let num = (value * denom as f64).round() as u32;
        return (num, denom);
    }

    (h[0].max(0) as u32, k[0].max(1) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_best_fraction() {
        // Test 1/5
        let (num, denom) = find_best_fraction(0.2, 9);
        assert_eq!((num, denom), (1, 5));

        // Test 1/3
        let (num, denom) = find_best_fraction(0.333333, 9);
        assert_eq!((num, denom), (1, 3));

        // Test 2/3
        let (num, denom) = find_best_fraction(0.666666, 9);
        assert_eq!((num, denom), (2, 3));
    }
}
