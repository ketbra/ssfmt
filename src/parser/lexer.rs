//! Lexer for tokenizing format code strings using logos.
//!
//! This lexer uses the logos crate for high-performance DFA-based tokenization.
//! Bracket content is captured as a single token and parsed separately to handle
//! context-sensitive parsing cleanly.

use crate::error::ParseError;
use crate::parser::tokens::{SpannedToken, Token};
use logos::Logos;

/// Raw tokens produced by logos lexer.
/// These are then converted to the Token type used by the parser.
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"")]  // Don't skip anything - we handle all characters
pub enum RawToken {
    // Bracket content captured as single token (handles context-sensitivity)
    #[regex(r"\[[^\]]*\]", priority = 10)]
    BracketContent,

    // Quoted string
    #[regex(r#""[^"]*""#, priority = 10)]
    QuotedString,

    // Escaped character
    #[regex(r"\\.", priority = 10)]
    Escaped,

    // General keyword (case-insensitive)
    #[regex(r"(?i)General", priority = 10)]
    General,

    // AM/PM patterns (case-insensitive, longest match first)
    #[regex(r"(?i)AM/PM", priority = 10)]
    AmPmFull,

    #[regex(r"(?i)A/P", priority = 10)]
    AmPmShort,

    // Date/time character runs (batched for efficiency)
    #[regex(r"[yY]+", priority = 3)]
    YearRun,

    #[regex(r"[mM]+", priority = 3)]
    MonthMinuteRun,

    #[regex(r"[dD]+", priority = 3)]
    DayRun,

    #[regex(r"[hH]+", priority = 3)]
    HourRun,

    #[regex(r"[sS]+", priority = 3)]
    SecondRun,

    // Buddhist year
    #[regex(r"[bB]+", priority = 3)]
    BuddhistYearRun,

    // Digit placeholders
    #[token("0", priority = 3)]
    Zero,

    #[token("#", priority = 3)]
    Hash,

    #[token("?", priority = 3)]
    Question,

    // Separators
    #[token(".", priority = 3)]
    DecimalPoint,

    #[token(",", priority = 3)]
    ThousandsSep,

    #[token(";", priority = 3)]
    SectionSep,

    // Special characters
    #[token("%", priority = 3)]
    Percent,

    #[token("@", priority = 3)]
    At,

    #[token("*", priority = 3)]
    Asterisk,

    #[token("_", priority = 3)]
    Underscore,

    // Scientific notation
    #[token("E", priority = 3)]
    ExponentUpper,

    #[token("e", priority = 3)]
    ExponentLower,

    #[token("+", priority = 3)]
    Plus,

    #[token("-", priority = 3)]
    Minus,

    // Fraction
    #[token("/", priority = 3)]
    Slash,

    // Parentheses (treated as literals)
    #[token("(", priority = 3)]
    OpenParen,

    #[token(")", priority = 3)]
    CloseParen,

    // Space (explicit token for handling)
    #[token(" ", priority = 3)]
    Space,

    // Any other single character becomes a literal (lowest priority)
    #[regex(r".", priority = 1)]
    Other,
}

/// A lexer for format code strings using logos.
pub struct Lexer<'a> {
    /// The input string being tokenized.
    pub(crate) input: &'a str,
    /// Logos lexer iterator
    logos_lexer: logos::Lexer<'a, RawToken>,
    /// Buffered tokens (only for bracket content expansion)
    buffer: Vec<SpannedToken>,
    /// Current position for buffer consumption
    buffer_pos: usize,
    /// Whether we've reached EOF
    eof_returned: bool,
    /// Pending run: (token_type_discriminant, remaining_count, current_position)
    /// Using u8 discriminant to avoid Token Clone overhead
    pending_run: Option<(u8, usize, usize)>,
}

// Token type discriminants for pending runs
const RUN_YEAR: u8 = 0;
const RUN_MONTH: u8 = 1;
const RUN_DAY: u8 = 2;
const RUN_HOUR: u8 = 3;
const RUN_SECOND: u8 = 4;
const RUN_BUDDHIST: u8 = 5;
const RUN_BUDDHIST_UPPER: u8 = 6;

impl<'a> Lexer<'a> {
    /// Creates a new lexer for the given input string.
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            logos_lexer: RawToken::lexer(input),
            buffer: Vec::new(),
            buffer_pos: 0,
            eof_returned: false,
            pending_run: None,
        }
    }

    #[inline]
    fn token_from_run_type(run_type: u8) -> Token {
        match run_type {
            RUN_YEAR => Token::Year,
            RUN_MONTH => Token::Month,
            RUN_DAY => Token::Day,
            RUN_HOUR => Token::Hour,
            RUN_SECOND => Token::Second,
            RUN_BUDDHIST => Token::BuddhistYear,
            RUN_BUDDHIST_UPPER => Token::BuddhistYearUpper,
            _ => unreachable!(),
        }
    }

    /// Returns the next token from the input.
    pub fn next_token(&mut self) -> Result<SpannedToken, ParseError> {
        // Check for pending run tokens first (no allocation needed)
        if let Some((run_type, remaining, pos)) = self.pending_run {
            let token = Self::token_from_run_type(run_type);
            if remaining > 1 {
                self.pending_run = Some((run_type, remaining - 1, pos + 1));
            } else {
                self.pending_run = None;
            }
            return Ok(SpannedToken {
                token,
                start: pos,
                end: pos + 1,
            });
        }

        // Return buffered tokens (for bracket content)
        if self.buffer_pos < self.buffer.len() {
            let token = self.buffer[self.buffer_pos].clone();
            self.buffer_pos += 1;
            return Ok(token);
        }

        // Clear buffer when exhausted
        if !self.buffer.is_empty() {
            self.buffer.clear();
            self.buffer_pos = 0;
        }

        // Get next token from logos
        match self.logos_lexer.next() {
            Some(Ok(raw_token)) => {
                let span = self.logos_lexer.span();
                let slice = self.logos_lexer.slice();
                self.convert_raw_token(raw_token, slice, span.start, span.end)
            }
            Some(Err(())) => {
                // Logos error - shouldn't happen with our catch-all regex
                let span = self.logos_lexer.span();
                Err(ParseError::UnexpectedToken {
                    position: span.start,
                    found: self.input[span.start..].chars().next().unwrap_or('\0'),
                })
            }
            None => {
                if self.eof_returned {
                    Ok(SpannedToken {
                        token: Token::Eof,
                        start: self.input.len(),
                        end: self.input.len(),
                    })
                } else {
                    self.eof_returned = true;
                    Ok(SpannedToken {
                        token: Token::Eof,
                        start: self.input.len(),
                        end: self.input.len(),
                    })
                }
            }
        }
    }

    /// Convert a raw logos token to our Token type.
    fn convert_raw_token(
        &mut self,
        raw: RawToken,
        slice: &str,
        start: usize,
        end: usize,
    ) -> Result<SpannedToken, ParseError> {
        let token = match raw {
            RawToken::BracketContent => {
                // Expand bracket content into OpenBracket + content tokens + CloseBracket
                self.expand_bracket_content(slice, start, end)?;
                // Return the first buffered token
                if !self.buffer.is_empty() {
                    let token = self.buffer[0].clone();
                    self.buffer_pos = 1;
                    return Ok(token);
                }
                // Empty brackets - just return open and close
                return Ok(SpannedToken {
                    token: Token::OpenBracket,
                    start,
                    end: start + 1,
                });
            }

            RawToken::QuotedString => {
                // Remove quotes
                let content = &slice[1..slice.len() - 1];
                Token::QuotedString(content.to_string())
            }

            RawToken::Escaped => {
                // Get the escaped character (after backslash)
                let ch = slice.chars().nth(1).unwrap_or('\\');
                Token::EscapedChar(ch)
            }

            RawToken::General => Token::General,

            RawToken::AmPmFull => Token::AmPm(slice.to_string()),
            RawToken::AmPmShort => Token::AmPm(slice.to_string()),

            // Date/time runs - use pending_run for efficient expansion
            RawToken::YearRun => {
                let len = slice.len();
                if len > 1 {
                    self.pending_run = Some((RUN_YEAR, len - 1, start + 1));
                }
                return Ok(SpannedToken {
                    token: Token::Year,
                    start,
                    end: start + 1,
                });
            }
            RawToken::MonthMinuteRun => {
                let len = slice.len();
                if len > 1 {
                    self.pending_run = Some((RUN_MONTH, len - 1, start + 1));
                }
                return Ok(SpannedToken {
                    token: Token::Month,
                    start,
                    end: start + 1,
                });
            }
            RawToken::DayRun => {
                let len = slice.len();
                if len > 1 {
                    self.pending_run = Some((RUN_DAY, len - 1, start + 1));
                }
                return Ok(SpannedToken {
                    token: Token::Day,
                    start,
                    end: start + 1,
                });
            }
            RawToken::HourRun => {
                let len = slice.len();
                if len > 1 {
                    self.pending_run = Some((RUN_HOUR, len - 1, start + 1));
                }
                return Ok(SpannedToken {
                    token: Token::Hour,
                    start,
                    end: start + 1,
                });
            }
            RawToken::SecondRun => {
                let len = slice.len();
                if len > 1 {
                    self.pending_run = Some((RUN_SECOND, len - 1, start + 1));
                }
                return Ok(SpannedToken {
                    token: Token::Second,
                    start,
                    end: start + 1,
                });
            }
            RawToken::BuddhistYearRun => {
                let len = slice.len();
                let first_char = slice.as_bytes()[0];
                let (run_type, token) = if first_char == b'B' {
                    (RUN_BUDDHIST_UPPER, Token::BuddhistYearUpper)
                } else {
                    (RUN_BUDDHIST, Token::BuddhistYear)
                };
                if len > 1 {
                    self.pending_run = Some((run_type, len - 1, start + 1));
                }
                return Ok(SpannedToken {
                    token,
                    start,
                    end: start + 1,
                });
            }

            RawToken::Zero => Token::Zero,
            RawToken::Hash => Token::Hash,
            RawToken::Question => Token::Question,
            RawToken::DecimalPoint => Token::DecimalPoint,
            RawToken::ThousandsSep => Token::ThousandsSep,
            RawToken::SectionSep => Token::SectionSep,
            RawToken::Percent => Token::Percent,
            RawToken::At => Token::At,
            RawToken::Asterisk => Token::Asterisk,
            RawToken::Underscore => Token::Underscore,
            RawToken::ExponentUpper => Token::ExponentUpper,
            RawToken::ExponentLower => Token::ExponentLower,
            RawToken::Plus => Token::Plus,
            RawToken::Minus => Token::Minus,
            RawToken::Slash => Token::Slash,
            RawToken::OpenParen => Token::Literal('('),
            RawToken::CloseParen => Token::Literal(')'),
            RawToken::Space => Token::Literal(' '),
            RawToken::Other => {
                let ch = slice.chars().next().unwrap();
                Token::Literal(ch)
            }
        };

        Ok(SpannedToken { token, start, end })
    }

    /// Expand a bracket content token into individual tokens.
    fn expand_bracket_content(
        &mut self,
        slice: &str,
        start: usize,
        end: usize,
    ) -> Result<(), ParseError> {
        // Add opening bracket
        self.buffer.push(SpannedToken {
            token: Token::OpenBracket,
            start,
            end: start + 1,
        });

        // Parse content between brackets
        let content = &slice[1..slice.len() - 1];
        let content_start = start + 1;

        // Add each character as a token
        // Inside brackets, most characters are literals, but some have special meaning
        // for the parser (like digits, operators for conditions)
        for (i, ch) in content.char_indices() {
            let char_start = content_start + i;
            let char_end = char_start + ch.len_utf8();

            // Inside brackets: use limited token set
            // The parser will interpret these for colors, conditions, elapsed time, locale
            let token = match ch {
                '0' => Token::Zero,
                '#' => Token::Hash,
                '?' => Token::Question,
                '.' => Token::DecimalPoint,
                ',' => Token::ThousandsSep,
                '%' => Token::Percent,
                '@' => Token::At,
                '*' => Token::Asterisk,
                '_' => Token::Underscore,
                '+' => Token::Plus,
                '-' => Token::Minus,
                '/' => Token::Slash,
                // Inside brackets, E and e are just literals (not exponent markers)
                _ => Token::Literal(ch),
            };

            self.buffer.push(SpannedToken {
                token,
                start: char_start,
                end: char_end,
            });
        }

        // Add closing bracket
        self.buffer.push(SpannedToken {
            token: Token::CloseBracket,
            start: end - 1,
            end,
        });

        Ok(())
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

    #[test]
    fn test_year_run() {
        let mut lexer = Lexer::new("yyyy");
        assert!(matches!(lexer.next_token().unwrap().token, Token::Year));
        assert!(matches!(lexer.next_token().unwrap().token, Token::Year));
        assert!(matches!(lexer.next_token().unwrap().token, Token::Year));
        assert!(matches!(lexer.next_token().unwrap().token, Token::Year));
        assert!(matches!(lexer.next_token().unwrap().token, Token::Eof));
    }

    #[test]
    fn test_bracket_content() {
        let mut lexer = Lexer::new("[Red]");
        assert!(matches!(lexer.next_token().unwrap().token, Token::OpenBracket));
        assert!(matches!(lexer.next_token().unwrap().token, Token::Literal('R')));
        assert!(matches!(lexer.next_token().unwrap().token, Token::Literal('e')));
        assert!(matches!(lexer.next_token().unwrap().token, Token::Literal('d')));
        assert!(matches!(lexer.next_token().unwrap().token, Token::CloseBracket));
        assert!(matches!(lexer.next_token().unwrap().token, Token::Eof));
    }

    #[test]
    fn test_general() {
        let mut lexer = Lexer::new("General");
        assert!(matches!(lexer.next_token().unwrap().token, Token::General));
        assert!(matches!(lexer.next_token().unwrap().token, Token::Eof));
    }

    #[test]
    fn test_ampm() {
        let mut lexer = Lexer::new("AM/PM");
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token, Token::AmPm(_)));
        assert!(matches!(lexer.next_token().unwrap().token, Token::Eof));
    }

    #[test]
    fn test_date_format() {
        let mut lexer = Lexer::new("yyyy-mm-dd");
        // yyyy
        assert!(matches!(lexer.next_token().unwrap().token, Token::Year));
        assert!(matches!(lexer.next_token().unwrap().token, Token::Year));
        assert!(matches!(lexer.next_token().unwrap().token, Token::Year));
        assert!(matches!(lexer.next_token().unwrap().token, Token::Year));
        // -
        assert!(matches!(lexer.next_token().unwrap().token, Token::Minus));
        // mm
        assert!(matches!(lexer.next_token().unwrap().token, Token::Month));
        assert!(matches!(lexer.next_token().unwrap().token, Token::Month));
        // -
        assert!(matches!(lexer.next_token().unwrap().token, Token::Minus));
        // dd
        assert!(matches!(lexer.next_token().unwrap().token, Token::Day));
        assert!(matches!(lexer.next_token().unwrap().token, Token::Day));
        // EOF
        assert!(matches!(lexer.next_token().unwrap().token, Token::Eof));
    }
}
