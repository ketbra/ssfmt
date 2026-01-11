# Comparison: ssfmt vs numfmt-rs

This document compares `ssfmt` with `numfmt-rs`, another Rust implementation of Excel number format codes. Both crates were created in response to [umya-spreadsheet issue #277](https://github.com/MathNya/umya-spreadsheet/issues/277).

## Overview

| Crate | Author | Approach |
|-------|--------|----------|
| ssfmt | ketbra | Compile format to typed AST, format many values |
| numfmt-rs | hongjr03 | Parse to flat struct, cache internally |

## Excel Compliance

Both crates target SheetJS SSF compatibility. ssfmt has been validated against the full SSF test suite:

| Test Suite | Tests | ssfmt Pass Rate |
|------------|-------|-----------------|
| SSF Dates | 3,846,024 | 100.0% |
| SSF Times | 15,728,625 | 83.6%* |
| SSF General | 493 | 100.0% |
| SSF Fractions | 106 | 100.0% |
| SSF Valid | 442 | 100.0% |
| SSF Oddities | 275 | 89.5% |
| SSF Implied | 672 | 78.7% |

\* Time failures are edge cases with subsecond rounding at precision boundaries.

When run against the same test fixtures, both libraries produce identical results for core formatting operations.

## Performance

Benchmarks were run with 100,000 iterations per test on a release build.

### Scenario 1: One-Shot Formatting (Parse + Format)

This measures the time to parse a format string and format a single value. This is relevant when formats are used only once.

| Format Type | ssfmt (ns/op) | numfmt-rs (ns/op) | Winner |
|-------------|---------------|-------------------|--------|
| General (integer) | 391.8 | 250.4 | numfmt-rs 1.56x |
| General (decimal) | 256.1 | 673.5 | **ssfmt 2.63x** |
| Number with decimals | 577.1 | 803.8 | **ssfmt 1.39x** |
| Percentage | 541.7 | 763.3 | **ssfmt 1.41x** |
| Scientific notation | 390.1 | 812.0 | **ssfmt 2.08x** |
| Fraction | 359.5 | 571.3 | **ssfmt 1.59x** |
| Date format | 392.6 | 377.1 | ~tied (numfmt 1.04x) |
| Time format | 480.0 | 418.5 | numfmt-rs 1.15x |
| Complex date/time | 689.1 | 537.0 | numfmt-rs 1.28x |
| Elapsed time | 406.7 | 383.3 | numfmt-rs 1.06x |

### Scenario 2: Pre-Compiled AST (ssfmt) vs Internal Cache (numfmt-rs)

This measures the time when the format has already been parsed. For ssfmt, this uses `NumberFormat::parse()` once and then `format()` repeatedly. For numfmt-rs, this relies on its internal `HashMap` cache.

This is the typical spreadsheet use case where the same format is applied to thousands of cells.

| Format Type | ssfmt (ns/op) | numfmt-rs (ns/op) | Winner |
|-------------|---------------|-------------------|--------|
| General (integer) | 320.4 | 245.1 | numfmt-rs 1.31x |
| Number with decimals | 445.2 | 802.4 | **ssfmt 1.80x** |
| Date format | 238.5 | 385.5 | **ssfmt 1.62x** |
| Time format | 284.3 | 369.1 | **ssfmt 1.30x** |

### Analysis

- **ssfmt is faster for number formatting** in both scenarios (1.4-2.6x one-shot, 1.8x cached)
- **For date/time formatting**: Nearly tied in one-shot scenario, ssfmt significantly faster when pre-compiled (1.3-1.6x)
- **Pre-compilation matters**: ssfmt's explicit AST approach shows its advantage in the cached scenario

### Performance Note

ssfmt uses O(1) algorithms for date serial number conversion (Fliegel & Van Flandern, 1968), avoiding the O(years) iteration that would be required for modern dates. This provides consistent performance regardless of the date being formatted.

### Caching Strategies

| Aspect | ssfmt | numfmt-rs |
|--------|-------|-----------|
| Cache type | LRU (bounded, 100 entries) | HashMap (unbounded) |
| Pre-compile API | `NumberFormat::parse()` | Not exposed |
| Memory safety | Bounded, won't grow | Can grow indefinitely |
| Zero-copy reuse | Yes, via `&NumberFormat` | Cache lookup each call |

## Code Idiomaticity

### ssfmt

**Strongly-typed AST:**
```rust
pub enum FormatPart {
    Literal(String),
    Digit(DigitPlaceholder),
    DecimalPoint,
    ThousandsSeparator,
    DatePart(DatePart),
    Scientific { upper: bool, show_plus: bool },
    Fraction {
        integer_digits: Vec<DigitPlaceholder>,
        numerator_digits: Vec<DigitPlaceholder>,
        denominator: FractionDenom,
        ...
    },
    ...
}
```

**Pre-computed metadata:**
```rust
pub struct SectionMetadata {
    pub has_ampm: bool,
    pub is_hijri: bool,
    pub max_subsecond_precision: Option<u8>,
    pub smallest_time_unit: TimeUnit,
    pub format_type: FormatType,
}
```

**Structured errors with thiserror:**
```rust
#[derive(Debug, Clone, PartialEq, Error)]
pub enum ParseError {
    #[error("unexpected token at position {position}: found '{found}'")]
    UnexpectedToken { position: usize, found: char },
    #[error("unterminated bracket at position {position}")]
    UnterminatedBracket { position: usize },
    ...
}
```

### numfmt-rs

**Flat struct with many fields:**
```rust
pub struct Section {
    pub scale: f64,
    pub percent: bool,
    pub text: bool,
    pub date: DateUnits,
    pub int_pattern: Vec<String>,  // String-based patterns
    pub frac_pattern: Vec<String>,
    pub man_pattern: Vec<String>,
    pub den_pattern: Vec<String>,
    pub num_pattern: Vec<String>,
    pub grouping: bool,
    pub fractions: bool,
    // ... 20+ more fields
}
```

**String-based errors:**
```rust
pub struct ParseError {
    message: String,  // Less structured
}
```

**Builder pattern for options:**
```rust
FormatterOptions::default()
    .with_locale("de")
    .with_nbsp(true)
```

## Feature Comparison

| Feature | ssfmt | numfmt-rs |
|---------|-------|-----------|
| Parse once, format many | Yes (explicit API) | Yes (internal cache) |
| Bounded cache | Yes (LRU, 100) | No (HashMap) |
| Typed AST | Yes | No (flat struct) |
| Structured errors | Yes | No |
| Locale support | Basic (en-US) | Extensive (many locales) |
| BigInt support | No | Yes |
| WASM ready | Possible | Built-in |
| Chrono integration | Yes (optional) | No (uses serial numbers) |
| Dependencies | Lighter | Heavier |

## Dependencies

### ssfmt
- `lru` - LRU cache
- `thiserror` - Error derive macro
- `chrono` (optional) - Date/time types

### numfmt-rs
- `winnow` - Parser combinator library
- `bitflags` - Bit flag macros
- `num-bigint` - Arbitrary precision integers
- `num-traits` - Numeric traits
- `wasm-bindgen` - WASM bindings
- `serde` + `serde_json` - Serialization

## Recommendations

### For umya-spreadsheet
**ssfmt is recommended** because:
1. Faster number formatting (most common operation in spreadsheets)
2. Pre-compiled AST matches spreadsheet use case (same format, many cells)
3. Bounded LRU cache prevents memory issues
4. Lighter dependency footprint
5. More maintainable typed code

### When to choose numfmt-rs
- Need BigInt support for very large numbers
- Need extensive multi-locale formatting
- Building a WASM application
- Need the builder pattern for options

## Conclusion

Both crates are solid implementations of Excel number formatting. ssfmt follows a more traditional compiler design (lexer → parser → AST → formatter) with idiomatic Rust patterns, while numfmt-rs is a more direct port of the JavaScript numfmt library with Rust-specific additions.

For typical spreadsheet processing, ssfmt's pre-compiled AST approach provides better performance and memory characteristics, while numfmt-rs offers more features for specialized use cases.
