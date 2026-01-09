//! Date and time formatting

use crate::ast::Section;
use crate::error::FormatError;
use crate::options::FormatOptions;

pub fn format_date(
    _value: f64,
    _section: &Section,
    _opts: &FormatOptions,
) -> Result<String, FormatError> {
    Ok("TODO".to_string())
}
