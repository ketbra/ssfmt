//! Value types that can be formatted.

/// A value that can be formatted using a number format code.
#[derive(Debug, Clone, PartialEq)]
pub enum Value<'a> {
    /// A numeric value (including Excel serial dates)
    Number(f64),
    /// A text value
    Text(&'a str),
    /// A boolean value
    Bool(bool),
    /// An empty cell
    Empty,
    /// A chrono DateTime (requires `chrono` feature)
    #[cfg(feature = "chrono")]
    DateTime(chrono::NaiveDateTime),
    /// A chrono Date (requires `chrono` feature)
    #[cfg(feature = "chrono")]
    Date(chrono::NaiveDate),
    /// A chrono Time (requires `chrono` feature)
    #[cfg(feature = "chrono")]
    Time(chrono::NaiveTime),
}

impl<'a> From<f64> for Value<'a> {
    fn from(n: f64) -> Self {
        Value::Number(n)
    }
}

impl<'a> From<f32> for Value<'a> {
    fn from(n: f32) -> Self {
        Value::Number(n as f64)
    }
}

impl<'a> From<i64> for Value<'a> {
    fn from(n: i64) -> Self {
        Value::Number(n as f64)
    }
}

impl<'a> From<i32> for Value<'a> {
    fn from(n: i32) -> Self {
        Value::Number(n as f64)
    }
}

impl<'a> From<&'a str> for Value<'a> {
    fn from(s: &'a str) -> Self {
        Value::Text(s)
    }
}

impl<'a> From<bool> for Value<'a> {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl<'a> From<()> for Value<'a> {
    fn from(_: ()) -> Self {
        Value::Empty
    }
}

#[cfg(feature = "chrono")]
impl<'a> From<chrono::NaiveDateTime> for Value<'a> {
    fn from(dt: chrono::NaiveDateTime) -> Self {
        Value::DateTime(dt)
    }
}

#[cfg(feature = "chrono")]
impl<'a> From<chrono::NaiveDate> for Value<'a> {
    fn from(d: chrono::NaiveDate) -> Self {
        Value::Date(d)
    }
}

#[cfg(feature = "chrono")]
impl<'a> From<chrono::NaiveTime> for Value<'a> {
    fn from(t: chrono::NaiveTime) -> Self {
        Value::Time(t)
    }
}

impl<'a> Value<'a> {
    /// Returns the value as a number if possible.
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            Value::Bool(true) => Some(1.0),
            Value::Bool(false) => Some(0.0),
            _ => None,
        }
    }

    /// Returns the value as text if it is text.
    pub fn as_text(&self) -> Option<&'a str> {
        match self {
            Value::Text(s) => Some(s),
            _ => None,
        }
    }

    /// Returns true if this value is empty.
    pub fn is_empty(&self) -> bool {
        matches!(self, Value::Empty)
    }

    /// Returns a type name for error messages.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Number(_) => "number",
            Value::Text(_) => "text",
            Value::Bool(_) => "boolean",
            Value::Empty => "empty",
            #[cfg(feature = "chrono")]
            Value::DateTime(_) => "datetime",
            #[cfg(feature = "chrono")]
            Value::Date(_) => "date",
            #[cfg(feature = "chrono")]
            Value::Time(_) => "time",
        }
    }
}
