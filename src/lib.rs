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
