# SheetJS SSF Test Suite Coverage

This document describes the test coverage from the SheetJS SSF library.

## Test Files Overview

We have implemented tests from the following SheetJS SSF test files:

### JSON Test Files (Fully Implemented)

1. **ssf_implied_tests.rs** - `implied.json`
   - Tests Excel's 49 built-in format IDs (0-49)
   - **Status**: 588/588 (100%) ✅

2. **ssf_fraction_tests.rs** - `fraction.json`
   - Tests fraction formatting (mixed and improper fractions)
   - **Status**: 104/106 (98.1%)

3. **ssf_oddities_tests.rs** - `oddities.json`
   - Tests edge cases and unusual format combinations
   - **Status**: 246/254 applicable (96.9%)

4. **ssf_general_tests.rs** - `general.json`
   - Tests general number formatting
   - **Status**: Not yet created

5. **ssf_date_tests.rs** - `date.json`
   - Tests date value roundtripping
   - **Status**: Not yet created

6. **ssf_is_date_tests.rs** - `is_date.json`
   - Tests format string date detection
   - **Status**: Not yet created

### TSV Test Files (Newly Added)

7. **ssf_comma_tests.rs** - `comma.tsv`
   - Tests thousands separators and comma divisors
   - Formats: `#,##0`, `#.0000,,,` (divide by billion), etc.
   - **Status**: 95/105 (90.5%)

8. **ssf_exp_tests.rs** - `exp.tsv`
   - Tests scientific/exponential notation with various mantissa sizes
   - Formats: `#0.0E+0`, `##0.0E+0`, `###0.0E+0`, etc.
   - **Status**: 177/180 (98.3%)

9. **ssf_valid_tests.rs** - `valid.tsv`
   - Tests that 442 format strings can be parsed without crashing
   - **Status**: 442/442 (100%) ✅

### Large TSV Files (Sampled)

The following files are very large (100K+ test cases each) and have been sampled:

10. **dates.tsv** → `ssf_dates_sample.tsv`
    - Original: 295,849 test cases
    - Sample: 1,000 test cases
    - Tests date format codes: `y`, `yy`, `yyyy`, `m`, `mm`, `d`, `dd`, etc.
    - **Status**: Not yet implemented

11. **times.tsv** → `ssf_times_sample.tsv`
    - Original: 1,048,576 test cases
    - Sample: 1,000 test cases
    - Tests time format codes: `h`, `HH`, `mm`, `ss`, `[h]`, `[m]`, etc.
    - **Status**: Not yet implemented

12. **cal.tsv**
    - Original: 1,048,576 test cases
    - Calendar/date computation tests
    - **Status**: Not copied (disabled in SSF tests with `if(0)`)

## Overall Test Coverage

### Implemented Tests
- **Total test cases**: ~1,500
- **Overall pass rate**: ~95%

### Test Categories
1. ✅ Built-in format IDs (100%)
2. ✅ Scientific notation (98.3%)
3. ✅ Comma/thousands formatting (90.5%)
4. ✅ Format string validation (100%)
5. 🟡 Fraction formatting (98.1%)
6. 🟡 Edge cases/oddities (96.9%)
7. ⚪ Date formatting (not yet tested)
8. ⚪ Time formatting (not yet tested)
9. ⚪ General number formatting (not yet tested)

## Known Limitations

### Fraction Formatting (2 failures)
- Different approximation algorithm for best fractions
- Parser doesn't recognize formats with spaces around slash (e.g., `# ?? / ?????????`)

### Oddities (8 failures)
- 3 failures: Floating-point precision for very large numbers (>10^15)
- 2 failures: Floating-point display artifacts
- 3 failures: Buddhist calendar not implemented (B2 format code)

### Comma Formatting (10 failures)
- Needs investigation

## Next Steps

1. Investigate comma formatting failures
2. Implement tests for `general.json`, `date.json`, `is_date.json`
3. Create tests for date/time sample files
4. Consider creating a benchmark suite for the full TSV files
