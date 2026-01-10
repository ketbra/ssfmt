//! Lexer for tokenizing format code strings.
//!
//! The lexer converts format code strings into a stream of tokens that can be
//! processed by the parser. It handles special cases like:
//! - Date/time letters (y, m, d, h, s) are tokens outside brackets but literals inside brackets
//! - Quoted strings ("text") become QuotedString tokens
//! - Escaped characters (\$) become EscapedChar tokens
//! - AM/PM patterns are detected and returned as single tokens

use crate::error::ParseError;
use crate::parser::tokens::{SpannedToken, Token};

/// A lexer for format code strings.
pub struct Lexer<'a> {
    /// The input string being tokenized.
    input: &'a str,
    /// The current position in the input.
    position: usize,
    /// Whether we are currently inside brackets.
    in_bracket: bool,
}

impl<'a> Lexer<'a> {
    /// Creates a new lexer for the given input string.
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            position: 0,
            in_bracket: false,
        }
    }

    /// Returns the next token from the input.
    pub fn next_token(&mut self) -> Result<SpannedToken, ParseError> {
        if self.position >= self.input.len() {
            return Ok(SpannedToken {
                token: Token::Eof,
                start: self.position,
                end: self.position,
            });
        }

        let start = self.position;
        let ch = self.current_char().unwrap();

        // Try to match special keywords first (before consuming individual characters)
        if !self.in_bracket {
            // Try to match "General" keyword
            if let Some(general_token) = self.try_match_general() {
                return Ok(general_token);
            }

            // Try to match AM/PM patterns
            if let Some(am_pm_token) = self.try_match_am_pm() {
                return Ok(am_pm_token);
            }
        }

        let token = match ch {
            // Quoted string
            '"' => self.lex_quoted_string()?,

            // Escaped character
            '\\' => self.lex_escaped_char()?,

            // Digit placeholders
            '0' => {
                self.advance();
                Token::Zero
            }
            '#' => {
                self.advance();
                Token::Hash
            }
            '?' => {
                self.advance();
                Token::Question
            }

            // Separators
            '.' => {
                self.advance();
                Token::DecimalPoint
            }
            ',' => {
                self.advance();
                Token::ThousandsSep
            }
            ';' => {
                self.advance();
                Token::SectionSep
            }

            // Special characters
            '%' => {
                self.advance();
                Token::Percent
            }
            '@' => {
                self.advance();
                Token::At
            }
            '*' => {
                self.advance();
                Token::Asterisk
            }
            '_' => {
                self.advance();
                Token::Underscore
            }

            // Scientific notation
            'E' => {
                self.advance();
                Token::ExponentUpper
            }
            'e' if !self.in_bracket => {
                self.advance();
                Token::ExponentLower
            }

            // Signs
            '+' => {
                self.advance();
                Token::Plus
            }
            '-' => {
                self.advance();
                Token::Minus
            }

            // Fraction
            '/' => {
                self.advance();
                Token::Slash
            }

            // Brackets
            '[' => {
                self.advance();
                self.in_bracket = true;
                Token::OpenBracket
            }
            ']' => {
                self.advance();
                self.in_bracket = false;
                Token::CloseBracket
            }

            // Date/time characters (only outside brackets)
            'y' | 'Y' if !self.in_bracket => {
                self.advance();
                Token::Year
            }
            'm' | 'M' if !self.in_bracket => {
                self.advance();
                Token::Month
            }
            'd' | 'D' if !self.in_bracket => {
                self.advance();
                Token::Day
            }
            'h' | 'H' if !self.in_bracket => {
                self.advance();
                Token::Hour
            }
            's' | 'S' if !self.in_bracket => {
                self.advance();
                Token::Second
            }
            'b' if !self.in_bracket => {
                self.advance();
                Token::BuddhistYear
            }
            'B' if !self.in_bracket => {
                self.advance();
                Token::BuddhistYearUpper
            }

            // Everything else is a literal
            _ => {
                self.advance();
                Token::Literal(ch)
            }
        };

        Ok(SpannedToken {
            token,
            start,
            end: self.position,
        })
    }

    /// Returns the character at the current position, if any.
    fn current_char(&self) -> Option<char> {
        self.input[self.position..].chars().next()
    }

    /// Returns the remaining input as a string slice.
    fn remaining(&self) -> &str {
        &self.input[self.position..]
    }

    /// Advances the position by one character.
    fn advance(&mut self) {
        if let Some(ch) = self.current_char() {
            self.position += ch.len_utf8();
        }
    }

    /// Lexes a quoted string ("...").
    fn lex_quoted_string(&mut self) -> Result<Token, ParseError> {
        let start = self.position;
        self.advance(); // Skip the opening quote

        let mut content = String::new();
        loop {
            match self.current_char() {
                Some('"') => {
                    self.advance(); // Skip the closing quote
                    return Ok(Token::QuotedString(content));
                }
                Some(ch) => {
                    content.push(ch);
                    self.advance();
                }
                None => {
                    return Err(ParseError::UnexpectedToken {
                        position: start,
                        found: '"',
                    });
                }
            }
        }
    }

    /// Lexes an escaped character (\x).
    fn lex_escaped_char(&mut self) -> Result<Token, ParseError> {
        let start = self.position;
        self.advance(); // Skip the backslash

        match self.current_char() {
            Some(ch) => {
                self.advance();
                Ok(Token::EscapedChar(ch))
            }
            None => Err(ParseError::UnexpectedToken {
                position: start,
                found: '\\',
            }),
        }
    }

    /// Tries to match "General" keyword at the current position.
    /// Returns Some(SpannedToken) if a match is found, None otherwise.
    fn try_match_general(&mut self) -> Option<SpannedToken> {
        let remaining = self.remaining();
        let start = self.position;
        let upper = remaining.to_uppercase();

        if upper.starts_with("GENERAL") {
            // Match "General" keyword regardless of what follows
            // The parser will handle "General" followed by literals correctly
            self.position += 7; // len("General")
            return Some(SpannedToken {
                token: Token::General,
                start,
                end: self.position,
            });
        }
        None
    }

    /// Tries to match an AM/PM pattern at the current position.
    /// Returns Some(SpannedToken) if a match is found, None otherwise.
    fn try_match_am_pm(&mut self) -> Option<SpannedToken> {
        let remaining = self.remaining();
        let start = self.position;
        let upper = remaining.to_uppercase();

        if upper.starts_with("AM/PM") {
            let matched: String = remaining.chars().take(5).collect();
            self.position += matched.len();
            return Some(SpannedToken {
                token: Token::AmPm(matched),
                start,
                end: self.position,
            });
        }
        // Malformed AM/P pattern (4 chars) - must check before A/P
        if upper.starts_with("AM/P") {
            let matched: String = remaining.chars().take(4).collect();
            self.position += matched.len();
            return Some(SpannedToken {
                token: Token::AmPm(matched),
                start,
                end: self.position,
            });
        }
        if upper.starts_with("A/P") {
            let matched: String = remaining.chars().take(3).collect();
            self.position += matched.len();
            return Some(SpannedToken {
                token: Token::AmPm(matched),
                start,
                end: self.position,
            });
        }
        None
    }

    /// Returns all remaining tokens as a vector.
    /// This consumes the lexer.
    pub fn tokenize(mut self) -> Result<Vec<SpannedToken>, ParseError> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token()?;
            let is_eof = matches!(token.token, Token::Eof);
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        let mut lexer = Lexer::new("");
        assert!(matches!(lexer.next_token().unwrap().token, Token::Eof));
    }

    #[test]
    fn test_single_zero() {
        let mut lexer = Lexer::new("0");
        assert!(matches!(lexer.next_token().unwrap().token, Token::Zero));
        assert!(matches!(lexer.next_token().unwrap().token, Token::Eof));
    }
}
