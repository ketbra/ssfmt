//! Fraction formatting

use crate::ast::{DigitPlaceholder, FormatPart, FractionDenom, Section};
use crate::error::FormatError;
use crate::options::FormatOptions;

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
        if integer_part > 0 || integer_digits.len() > 0 {
            result.push_str(&format!("{}", integer_part));
        }
        // Add space between integer and fraction
        result.push(' ');
    }

    if num > 0 {
        // Format numerator with padding
        let num_str = format!("{}", num);
        let num_width = numerator_digits.len();
        for _ in 0..(num_width.saturating_sub(num_str.len())) {
            result.push(' ');
        }
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
    } else {
        // No fractional part, pad to match format width
        // Need to pad: numerator + "/" + denominator
        let denom_width = match denominator {
            FractionDenom::UpToDigits(d) => *d as usize,
            FractionDenom::Fixed(d) => d.to_string().len(),
        };
        let total_frac_width = numerator_digits.len() + 1 + denom_width; // num + "/" + denom
        for _ in 0..total_frac_width {
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
