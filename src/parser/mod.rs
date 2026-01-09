//! Parser for ECMA-376 number format codes.

pub mod lexer;
pub mod tokens;

use crate::ast::{
    AmPmStyle, Color, Condition, DatePart, DigitPlaceholder, ElapsedPart, FormatPart, LocaleCode,
    NamedColor, NumberFormat, Section,
};
use crate::error::ParseError;
use lexer::Lexer;
use tokens::{SpannedToken, Token};

/// Parse a format code string into a NumberFormat.
pub fn parse(format_code: &str) -> Result<NumberFormat, ParseError> {
    if format_code.is_empty() {
        return Err(ParseError::EmptyFormat);
    }

    let mut parser = Parser::new(format_code);
    parser.parse()
}

/// Parser for format code strings.
struct Parser<'a> {
    lexer: Lexer<'a>,
    /// Current token
    current: SpannedToken,
    /// Whether we've seen an hour token in the current section (for minute vs month disambiguation)
    seen_hour: bool,
}

impl<'a> Parser<'a> {
    /// Create a new parser for the given format code.
    fn new(format_code: &'a str) -> Self {
        let mut lexer = Lexer::new(format_code);
        // Get the first token
        let current = lexer.next_token().unwrap_or(SpannedToken {
            token: Token::Eof,
            start: 0,
            end: 0,
        });
        Self {
            lexer,
            current,
            seen_hour: false,
        }
    }

    /// Advance to the next token.
    fn advance(&mut self) -> Result<(), ParseError> {
        self.current = self.lexer.next_token()?;
        Ok(())
    }

    /// Parse the format code into a NumberFormat.
    fn parse(&mut self) -> Result<NumberFormat, ParseError> {
        let mut sections = Vec::new();

        loop {
            let section = self.parse_section()?;
            sections.push(section);

            // Check for section separator or end
            if matches!(self.current.token, Token::Eof) {
                break;
            }

            if matches!(self.current.token, Token::SectionSep) {
                self.advance()?;
                // Continue to next section
            } else {
                break;
            }
        }

        Ok(NumberFormat::from_sections(sections))
    }

    /// Parse a single section of the format.
    fn parse_section(&mut self) -> Result<Section, ParseError> {
        let mut builder = SectionBuilder::new();
        self.seen_hour = false;

        loop {
            match &self.current.token {
                Token::Eof | Token::SectionSep => break,

                // Bracket content - could be color, condition, elapsed time, or locale
                Token::OpenBracket => {
                    let bracket_start = self.current.start;
                    self.advance()?;
                    self.parse_bracket_content(&mut builder, bracket_start)?;
                }

                // Digit placeholders
                Token::Zero => {
                    builder.add_part(FormatPart::Digit(DigitPlaceholder::Zero));
                    self.advance()?;
                }
                Token::Hash => {
                    builder.add_part(FormatPart::Digit(DigitPlaceholder::Hash));
                    self.advance()?;
                }
                Token::Question => {
                    builder.add_part(FormatPart::Digit(DigitPlaceholder::Question));
                    self.advance()?;
                }

                // Separators
                Token::DecimalPoint => {
                    builder.add_part(FormatPart::DecimalPoint);
                    self.advance()?;
                }
                Token::ThousandsSep => {
                    builder.add_part(FormatPart::ThousandsSeparator);
                    self.advance()?;
                }

                // Special characters
                Token::Percent => {
                    builder.add_part(FormatPart::Percent);
                    self.advance()?;
                }
                Token::At => {
                    builder.add_part(FormatPart::TextPlaceholder);
                    self.advance()?;
                }
                Token::Asterisk => {
                    // Fill character - next char is the fill
                    self.advance()?;
                    if let Some(ch) = self.get_literal_char() {
                        builder.add_part(FormatPart::Fill(ch));
                        self.advance()?;
                    }
                }
                Token::Underscore => {
                    // Skip character - next char is the skip width
                    self.advance()?;
                    if let Some(ch) = self.get_literal_char() {
                        builder.add_part(FormatPart::Skip(ch));
                        self.advance()?;
                    }
                }

                // Scientific notation
                Token::ExponentUpper | Token::ExponentLower => {
                    let upper = matches!(self.current.token, Token::ExponentUpper);
                    self.advance()?;
                    let show_plus = matches!(self.current.token, Token::Plus);
                    if matches!(self.current.token, Token::Plus | Token::Minus) {
                        self.advance()?;
                    }
                    builder.add_part(FormatPart::Scientific { upper, show_plus });
                }

                // Signs become literals in format context (when not part of scientific notation)
                Token::Plus => {
                    builder.add_part(FormatPart::Literal("+".to_string()));
                    self.advance()?;
                }
                Token::Minus => {
                    builder.add_part(FormatPart::Literal("-".to_string()));
                    self.advance()?;
                }

                // Fraction
                Token::Slash => {
                    builder.add_part(FormatPart::Literal("/".to_string()));
                    self.advance()?;
                }

                // Date/time tokens
                Token::Year => {
                    let count = self.count_consecutive(&Token::Year)?;
                    let part = if count >= 4 {
                        DatePart::Year4
                    } else {
                        DatePart::Year2
                    };
                    builder.add_part(FormatPart::DatePart(part));
                }
                Token::Month => {
                    let count = self.count_consecutive(&Token::Month)?;
                    // Check if this should be minute (after hour) or month
                    let part = if self.seen_hour {
                        // This is minute
                        if count >= 2 {
                            DatePart::Minute2
                        } else {
                            DatePart::Minute
                        }
                    } else {
                        // This is month
                        match count {
                            1 => DatePart::Month,
                            2 => DatePart::Month2,
                            3 => DatePart::MonthAbbr,
                            4 => DatePart::MonthFull,
                            _ => DatePart::MonthLetter,
                        }
                    };
                    builder.add_part(FormatPart::DatePart(part));
                }
                Token::Day => {
                    let count = self.count_consecutive(&Token::Day)?;
                    let part = match count {
                        1 => DatePart::Day,
                        2 => DatePart::Day2,
                        3 => DatePart::DayAbbr,
                        _ => DatePart::DayFull,
                    };
                    builder.add_part(FormatPart::DatePart(part));
                }
                Token::Hour => {
                    self.seen_hour = true;
                    let count = self.count_consecutive(&Token::Hour)?;
                    let part = if count >= 2 {
                        DatePart::Hour2
                    } else {
                        DatePart::Hour
                    };
                    builder.add_part(FormatPart::DatePart(part));
                }
                Token::Second => {
                    let count = self.count_consecutive(&Token::Second)?;
                    let part = if count >= 2 {
                        DatePart::Second2
                    } else {
                        DatePart::Second
                    };
                    builder.add_part(FormatPart::DatePart(part));
                }

                // AM/PM
                Token::AmPm(s) => {
                    let style = parse_am_pm_style(s);
                    builder.add_part(FormatPart::AmPm(style));
                    self.advance()?;
                }

                // Literals
                Token::Literal(ch) => {
                    builder.add_part(FormatPart::Literal(ch.to_string()));
                    self.advance()?;
                }
                Token::EscapedChar(ch) => {
                    builder.add_part(FormatPart::Literal(ch.to_string()));
                    self.advance()?;
                }
                Token::QuotedString(s) => {
                    builder.add_part(FormatPart::Literal(s.clone()));
                    self.advance()?;
                }

                Token::CloseBracket => {
                    // Unexpected close bracket - treat as literal
                    builder.add_part(FormatPart::Literal("]".to_string()));
                    self.advance()?;
                }
            }
        }

        Ok(builder.build())
    }

    /// Parse bracket content: [Red], [>100], [h], [$-409], etc.
    fn parse_bracket_content(
        &mut self,
        builder: &mut SectionBuilder,
        bracket_start: usize,
    ) -> Result<(), ParseError> {
        // Collect all content until we hit the close bracket
        let mut content = String::new();

        loop {
            match &self.current.token {
                Token::CloseBracket => {
                    self.advance()?;
                    break;
                }
                Token::Eof => {
                    return Err(ParseError::UnterminatedBracket {
                        position: bracket_start,
                    });
                }
                Token::Literal(ch) => {
                    content.push(*ch);
                    self.advance()?;
                }
                // Other tokens that might appear inside brackets
                Token::Zero => {
                    content.push('0');
                    self.advance()?;
                }
                Token::Hash => {
                    content.push('#');
                    self.advance()?;
                }
                Token::Question => {
                    content.push('?');
                    self.advance()?;
                }
                Token::DecimalPoint => {
                    content.push('.');
                    self.advance()?;
                }
                Token::ThousandsSep => {
                    content.push(',');
                    self.advance()?;
                }
                Token::Percent => {
                    content.push('%');
                    self.advance()?;
                }
                Token::At => {
                    content.push('@');
                    self.advance()?;
                }
                Token::Asterisk => {
                    content.push('*');
                    self.advance()?;
                }
                Token::Underscore => {
                    content.push('_');
                    self.advance()?;
                }
                Token::Plus => {
                    content.push('+');
                    self.advance()?;
                }
                Token::Minus => {
                    content.push('-');
                    self.advance()?;
                }
                Token::Slash => {
                    content.push('/');
                    self.advance()?;
                }
                Token::ExponentUpper => {
                    content.push('E');
                    self.advance()?;
                }
                Token::ExponentLower => {
                    content.push('e');
                    self.advance()?;
                }
                _ => {
                    // Skip other tokens inside brackets
                    self.advance()?;
                }
            }
        }

        // Now parse the bracket content
        let content = content.trim();

        // Try to parse as color
        if let Some(color) = try_parse_color(content) {
            builder.color = Some(color);
            return Ok(());
        }

        // Try to parse as condition
        if let Some(condition) = try_parse_condition(content) {
            builder.condition = Some(condition);
            return Ok(());
        }

        // Try to parse as elapsed time
        if let Some(elapsed) = try_parse_elapsed(content) {
            builder.add_part(FormatPart::Elapsed(elapsed));
            return Ok(());
        }

        // Try to parse as locale code
        if let Some(locale) = try_parse_locale(content) {
            builder.add_part(FormatPart::Locale(locale));
            return Ok(());
        }

        // Unknown bracket content - treat as literal (or ignore)
        Ok(())
    }

    /// Count consecutive tokens of the same type and advance past them.
    fn count_consecutive(&mut self, token_type: &Token) -> Result<usize, ParseError> {
        let mut count = 0;
        while self.token_matches(token_type) {
            count += 1;
            self.advance()?;
        }
        Ok(count)
    }

    /// Check if current token matches the given token type (ignoring content).
    fn token_matches(&self, token_type: &Token) -> bool {
        std::mem::discriminant(&self.current.token) == std::mem::discriminant(token_type)
    }

    /// Get the literal character from the current token.
    fn get_literal_char(&self) -> Option<char> {
        match &self.current.token {
            Token::Literal(ch) => Some(*ch),
            Token::Zero => Some('0'),
            Token::Hash => Some('#'),
            Token::Question => Some('?'),
            Token::DecimalPoint => Some('.'),
            Token::ThousandsSep => Some(','),
            Token::Percent => Some('%'),
            Token::At => Some('@'),
            Token::Asterisk => Some('*'),
            Token::Underscore => Some('_'),
            Token::Plus => Some('+'),
            Token::Minus => Some('-'),
            Token::Slash => Some('/'),
            Token::EscapedChar(ch) => Some(*ch),
            _ => None,
        }
    }
}

/// Helper struct for building sections.
struct SectionBuilder {
    condition: Option<Condition>,
    color: Option<Color>,
    parts: Vec<FormatPart>,
}

impl SectionBuilder {
    fn new() -> Self {
        Self {
            condition: None,
            color: None,
            parts: Vec::new(),
        }
    }

    fn add_part(&mut self, part: FormatPart) {
        self.parts.push(part);
    }

    fn build(self) -> Section {
        Section {
            condition: self.condition,
            color: self.color,
            parts: self.parts,
        }
    }
}

/// Parse AM/PM style from the matched string.
fn parse_am_pm_style(s: &str) -> AmPmStyle {
    match s {
        "AM/PM" => AmPmStyle::Upper,
        "am/pm" => AmPmStyle::Lower,
        "A/P" => AmPmStyle::ShortUpper,
        "a/p" => AmPmStyle::ShortLower,
        // Default to upper for mixed case
        _ => {
            if s.len() <= 3 {
                if s.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                    AmPmStyle::ShortUpper
                } else {
                    AmPmStyle::ShortLower
                }
            } else if s.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                AmPmStyle::Upper
            } else {
                AmPmStyle::Lower
            }
        }
    }
}

/// Try to parse bracket content as a color.
fn try_parse_color(content: &str) -> Option<Color> {
    // Check for named colors
    if let Ok(named) = content.parse::<NamedColor>() {
        return Some(Color::Named(named));
    }

    // Check for indexed colors: Color1 through Color56
    let lower = content.to_lowercase();
    if lower.starts_with("color") {
        if let Ok(index) = content[5..].parse::<u8>() {
            if (1..=56).contains(&index) {
                return Some(Color::Indexed(index));
            }
        }
    }

    None
}

/// Try to parse bracket content as a condition.
fn try_parse_condition(content: &str) -> Option<Condition> {
    let content = content.trim();

    // Parse conditions like >=, <=, <>, >, <, =
    if let Some(value_str) = content.strip_prefix(">=") {
        if let Ok(value) = value_str.trim().parse::<f64>() {
            return Some(Condition::GreaterOrEqual(value));
        }
    } else if let Some(value_str) = content.strip_prefix("<=") {
        if let Ok(value) = value_str.trim().parse::<f64>() {
            return Some(Condition::LessOrEqual(value));
        }
    } else if let Some(value_str) = content.strip_prefix("<>") {
        if let Ok(value) = value_str.trim().parse::<f64>() {
            return Some(Condition::NotEqual(value));
        }
    } else if let Some(value_str) = content.strip_prefix('>') {
        if let Ok(value) = value_str.trim().parse::<f64>() {
            return Some(Condition::GreaterThan(value));
        }
    } else if let Some(value_str) = content.strip_prefix('<') {
        if let Ok(value) = value_str.trim().parse::<f64>() {
            return Some(Condition::LessThan(value));
        }
    } else if let Some(value_str) = content.strip_prefix('=') {
        if let Ok(value) = value_str.trim().parse::<f64>() {
            return Some(Condition::Equal(value));
        }
    }

    None
}

/// Try to parse bracket content as elapsed time.
fn try_parse_elapsed(content: &str) -> Option<ElapsedPart> {
    let lower = content.to_lowercase();
    match lower.as_str() {
        "h" | "hh" => Some(ElapsedPart::Hours),
        "m" | "mm" => Some(ElapsedPart::Minutes),
        "s" | "ss" => Some(ElapsedPart::Seconds),
        _ => None,
    }
}

/// Try to parse bracket content as a locale code.
fn try_parse_locale(content: &str) -> Option<LocaleCode> {
    // Locale codes start with $ e.g., [$-409], [$€-407]
    if !content.starts_with('$') {
        return None;
    }

    let rest = &content[1..];

    // Parse [$currency-lcid] format
    if let Some(dash_pos) = rest.find('-') {
        let currency_part = &rest[..dash_pos];
        let lcid_part = &rest[dash_pos + 1..];

        let currency = if currency_part.is_empty() {
            None
        } else {
            Some(currency_part.to_string())
        };

        let lcid = u32::from_str_radix(lcid_part, 16).ok();

        Some(LocaleCode { currency, lcid })
    } else {
        // Just a currency symbol
        Some(LocaleCode {
            currency: if rest.is_empty() {
                None
            } else {
                Some(rest.to_string())
            },
            lcid: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        let result = parse("");
        assert!(matches!(result, Err(ParseError::EmptyFormat)));
    }

    #[test]
    fn test_parse_single_zero() {
        let fmt = parse("0").unwrap();
        assert_eq!(fmt.sections().len(), 1);
        assert_eq!(fmt.sections()[0].parts.len(), 1);
    }

    #[test]
    fn test_try_parse_color_named() {
        assert!(matches!(
            try_parse_color("Red"),
            Some(Color::Named(NamedColor::Red))
        ));
        assert!(matches!(
            try_parse_color("blue"),
            Some(Color::Named(NamedColor::Blue))
        ));
    }

    #[test]
    fn test_try_parse_color_indexed() {
        assert!(matches!(try_parse_color("Color1"), Some(Color::Indexed(1))));
        assert!(matches!(
            try_parse_color("Color56"),
            Some(Color::Indexed(56))
        ));
        assert!(try_parse_color("Color0").is_none());
        assert!(try_parse_color("Color57").is_none());
    }

    #[test]
    fn test_try_parse_condition() {
        assert!(matches!(
            try_parse_condition(">100"),
            Some(Condition::GreaterThan(n)) if (n - 100.0).abs() < f64::EPSILON
        ));
        assert!(matches!(
            try_parse_condition("<0"),
            Some(Condition::LessThan(n)) if n.abs() < f64::EPSILON
        ));
        assert!(matches!(
            try_parse_condition(">=50"),
            Some(Condition::GreaterOrEqual(n)) if (n - 50.0).abs() < f64::EPSILON
        ));
        assert!(matches!(
            try_parse_condition("<=10"),
            Some(Condition::LessOrEqual(n)) if (n - 10.0).abs() < f64::EPSILON
        ));
        assert!(matches!(
            try_parse_condition("=5"),
            Some(Condition::Equal(n)) if (n - 5.0).abs() < f64::EPSILON
        ));
        assert!(matches!(
            try_parse_condition("<>0"),
            Some(Condition::NotEqual(n)) if n.abs() < f64::EPSILON
        ));
    }

    #[test]
    fn test_try_parse_elapsed() {
        assert!(matches!(try_parse_elapsed("h"), Some(ElapsedPart::Hours)));
        assert!(matches!(try_parse_elapsed("hh"), Some(ElapsedPart::Hours)));
        assert!(matches!(try_parse_elapsed("m"), Some(ElapsedPart::Minutes)));
        assert!(matches!(
            try_parse_elapsed("mm"),
            Some(ElapsedPart::Minutes)
        ));
        assert!(matches!(try_parse_elapsed("s"), Some(ElapsedPart::Seconds)));
        assert!(matches!(
            try_parse_elapsed("ss"),
            Some(ElapsedPart::Seconds)
        ));
    }

    #[test]
    fn test_try_parse_locale() {
        let locale = try_parse_locale("$-409").unwrap();
        assert!(locale.currency.is_none());
        assert_eq!(locale.lcid, Some(0x409));

        let locale = try_parse_locale("$€-407").unwrap();
        assert_eq!(locale.currency, Some("€".to_string()));
        assert_eq!(locale.lcid, Some(0x407));

        let locale = try_parse_locale("$$").unwrap();
        assert_eq!(locale.currency, Some("$".to_string()));
        assert!(locale.lcid.is_none());
    }
}
