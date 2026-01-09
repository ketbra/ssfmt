//! ssfmt - Excel-compatible ECMA-376 number format codes
//!
//! This crate provides parsing and formatting of spreadsheet number format codes,
//! matching Excel's actual behavior including undocumented quirks.

pub mod ast;
pub mod error;
pub mod options;
pub mod value;

pub mod date_serial;

mod cache;
mod formatter;
mod locale;
pub mod parser;

// Re-exports will be added once types are defined:
pub use ast::{NumberFormat, Section};
pub use error::{FormatError, ParseError};
pub use locale::Locale;
pub use options::{DateSystem, FormatOptions};
pub use value::Value;

// Convenience functions

/// Parse and format a value in one call.
///
/// This function caches recently used format codes for efficiency.
pub fn format(
    value: f64,
    format_code: &str,
    opts: &FormatOptions,
) -> Result<String, ParseError> {
    let fmt = cache::get_or_parse(format_code)?;
    Ok(fmt.format(value, opts))
}

/// Format a value with default options (1900 date system, en-US locale).
///
/// This function caches recently used format codes for efficiency.
pub fn format_default(
    value: f64,
    format_code: &str,
) -> Result<String, ParseError> {
    let opts = FormatOptions::default();
    format(value, format_code, &opts)
}
