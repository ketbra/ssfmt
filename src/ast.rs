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
