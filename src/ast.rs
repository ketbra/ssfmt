//! AST types for parsed format codes.

use std::str::FromStr;

/// Named colors supported in format codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedColor {
    Black,
    Blue,
    Cyan,
    Green,
    Magenta,
    Red,
    White,
    Yellow,
}

impl FromStr for NamedColor {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "black" => Ok(NamedColor::Black),
            "blue" => Ok(NamedColor::Blue),
            "cyan" => Ok(NamedColor::Cyan),
            "green" => Ok(NamedColor::Green),
            "magenta" => Ok(NamedColor::Magenta),
            "red" => Ok(NamedColor::Red),
            "white" => Ok(NamedColor::White),
            "yellow" => Ok(NamedColor::Yellow),
            _ => Err(()),
        }
    }
}

/// Color specification in a format section.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Color {
    Named(NamedColor),
    Indexed(u8),
}

/// Conditional expression for section selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Condition {
    GreaterThan(f64),
    LessThan(f64),
    Equal(f64),
    GreaterOrEqual(f64),
    LessOrEqual(f64),
    NotEqual(f64),
}

impl Condition {
    /// Evaluate this condition against a value.
    pub fn evaluate(&self, value: f64) -> bool {
        match self {
            Condition::GreaterThan(n) => value > *n,
            Condition::LessThan(n) => value < *n,
            Condition::Equal(n) => (value - n).abs() < f64::EPSILON,
            Condition::GreaterOrEqual(n) => value >= *n,
            Condition::LessOrEqual(n) => value <= *n,
            Condition::NotEqual(n) => (value - n).abs() >= f64::EPSILON,
        }
    }
}

/// Digit placeholder type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigitPlaceholder {
    /// `0` - Display digit or zero
    Zero,
    /// `#` - Display digit or nothing
    Hash,
    /// `?` - Display digit or space
    Question,
}

impl DigitPlaceholder {
    /// Returns true if this placeholder requires a digit (shows 0 for missing).
    pub fn is_required(&self) -> bool {
        matches!(self, DigitPlaceholder::Zero)
    }

    /// Returns the character to display when no digit is present.
    pub fn empty_char(&self) -> Option<char> {
        match self {
            DigitPlaceholder::Zero => Some('0'),
            DigitPlaceholder::Hash => None,
            DigitPlaceholder::Question => Some(' '),
        }
    }
}

/// Date/time format parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatePart {
    Year2,
    Year4,
    Month,
    Month2,
    MonthAbbr,
    MonthFull,
    MonthLetter,
    Day,
    Day2,
    DayAbbr,
    DayFull,
    Hour,
    Hour2,
    Minute,
    Minute2,
    Second,
    Second2,
    SubSecond(u8),
}

/// AM/PM format style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmPmStyle {
    Upper,
    Lower,
    ShortUpper,
    ShortLower,
}

/// Elapsed time format part (for durations).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElapsedPart {
    Hours,
    Minutes,
    Seconds,
}

/// Fraction denominator specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FractionDenom {
    UpToDigits(u8),
    Fixed(u32),
}

/// Locale code from format string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocaleCode {
    pub currency: Option<String>,
    pub lcid: Option<u32>,
}

/// A single part of a format section.
#[derive(Debug, Clone, PartialEq)]
pub enum FormatPart {
    Literal(String),
    Digit(DigitPlaceholder),
    DecimalPoint,
    ThousandsSeparator,
    Percent,
    Scientific { upper: bool, show_plus: bool },
    Fraction {
        integer_digits: Vec<DigitPlaceholder>,
        numerator_digits: Vec<DigitPlaceholder>,
        denominator: FractionDenom,
    },
    DatePart(DatePart),
    AmPm(AmPmStyle),
    Elapsed(ElapsedPart),
    TextPlaceholder,
    Fill(char),
    Skip(char),
    Locale(LocaleCode),
}

impl FormatPart {
    pub fn is_date_part(&self) -> bool {
        matches!(
            self,
            FormatPart::DatePart(_) | FormatPart::AmPm(_) | FormatPart::Elapsed(_)
        )
    }

    pub fn is_numeric_part(&self) -> bool {
        matches!(
            self,
            FormatPart::Digit(_)
                | FormatPart::DecimalPoint
                | FormatPart::ThousandsSeparator
                | FormatPart::Percent
                | FormatPart::Scientific { .. }
                | FormatPart::Fraction { .. }
        )
    }
}
