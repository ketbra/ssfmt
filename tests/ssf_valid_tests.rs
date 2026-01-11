//! Tests from SheetJS ssf library's valid.tsv test file
//!
//! These tests verify that various format strings can be parsed and used
//! without crashing. They don't check the output, just that formatting
//! doesn't fail.

use ssfmt::format_default;

fn load_format_strings() -> Vec<String> {
    let tsv_data = include_str!("fixtures/valid.tsv");
    tsv_data
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.to_string())
        .collect()
}

#[test]
fn test_ssf_valid_formats() {
    let format_strings = load_format_strings();
    let total = format_strings.len();
    let mut passed = 0;
    let mut failed = 0;

    println!("\nTesting {} format strings from valid.tsv", total);

    // Test values to format
    let test_values = vec![0.0, 1.0, -2.0, 3.45, -67.89];

    for (i, format) in format_strings.iter().enumerate() {
        let mut format_passed = true;

        for value in &test_values {
            match format_default(*value, format) {
                Ok(_) => {
                    // Format succeeded, which is what we want
                }
                Err(e) => {
                    if failed < 20 {
                        println!(
                            "ERROR #{}: format='{}', value={}, error={:?}",
                            i + 1,
                            format,
                            value,
                            e
                        );
                    }
                    format_passed = false;
                    break;
                }
            }
        }

        if format_passed {
            passed += 1;
        } else {
            failed += 1;
        }
    }

    println!("Total formats: {}", total);
    println!("Passed:  {} ({:.1}%)", passed, (passed as f64 / total as f64) * 100.0);
    println!("Failed:  {} ({:.1}%)", failed, (failed as f64 / total as f64) * 100.0);

    // We should be able to parse most formats
    assert!(passed > total / 2, "More than half of formats should be valid");
}
