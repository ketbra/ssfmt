# Implementation Improvements

Based on analysis of SSF source code, these improvements would make the codebase more efficient and maintainable.

## High Priority (Significant Impact)

### 1. Add Section Metadata to Eliminate Repeated Scanning
**Status**: ✅ COMPLETED
**Impact**: Major performance improvement + cleaner code
**Effort**: Medium

**Problem**: We currently scan format parts multiple times during formatting:
```rust
// In format_date(), we scan multiple times:
let has_ampm = section.parts.iter().any(|p| matches!(p, FormatPart::AmPm(_)));
let is_hijri = section.parts.iter().any(|p| matches!(p, FormatPart::DatePart(...)));
let subsecond_places: Vec<u8> = section.parts.iter().filter_map(...).collect();
```

**SSF Approach**: Parse once, set flags during parsing (bits/82_eval.js)

**Solution**:
```rust
pub struct Section {
    pub condition: Option<Condition>,
    pub parts: Vec<FormatPart>,
    pub metadata: SectionMetadata,  // ADD THIS
}

pub struct SectionMetadata {
    pub has_ampm: bool,
    pub is_hijri: bool,
    pub max_subsecond_precision: Option<u8>,
    pub has_elapsed_time: bool,
    pub smallest_time_unit: TimeUnit,  // For pre-rounding
    pub format_type: FormatType,  // Date, Time, Number, Fraction, etc.
}

pub enum TimeUnit {
    None,
    Hours,
    Minutes,
    Seconds,
    Subseconds,
}
```

**Files to modify**:
- `src/ast.rs` - Add metadata structs
- `src/parser/mod.rs` - Compute metadata during parsing
- `src/formatter/date.rs` - Use metadata instead of scanning
- `src/formatter/number.rs` - Use metadata for format type detection

---

### 2. Apply Pre-Rounding Uniformly to All Time Formatting
**Status**: ✅ COMPLETED
**Impact**: Fix potential correctness issues
**Effort**: Low

**Problem**: We only apply SSF's pre-rounding in `format_elapsed()`, not in regular time formatting.

**SSF Approach**: Pre-round based on smallest displayed time unit (bits/82_eval.js lines 102-115):
```javascript
switch(bt) {
    case 1: // Hours displayed - round subseconds->seconds->minutes->hours
    case 2: // Minutes displayed - round subseconds->seconds->minutes
    case 3: // Seconds displayed - round subseconds->seconds
}
```

**Solution**: In `format_date()`, after getting time components, apply pre-rounding based on `metadata.smallest_time_unit`:
```rust
// After: let (hour, minute, second) = serial_to_time(adjusted_value);
// Add: apply_time_prerounding(&mut hour, &mut minute, &mut second, &subseconds, metadata.smallest_time_unit);
```

**Files to modify**:
- `src/formatter/date.rs` - Add pre-rounding logic
- Tests to verify behavior matches SSF

---

### 3. Add Integer Fast Path for Large Numbers
**Status**: ✅ COMPLETED
**Impact**: Fix precision issues for large integers (fixes oddities tests #130, #133, #134)
**Effort**: Medium

**Problem**: Everything goes through f64, causing precision loss for large integers.

**SSF Approach**: Separate code paths (bits/66_numint.js vs bits/63_numflt.js)

**Solution**:
```rust
pub fn format_number(value: f64, section: &Section, opts: &FormatOptions) -> Result<String> {
    // Check if value is actually an integer within safe range
    if value.fract() == 0.0 && value.abs() < (1i64 << 53) as f64 {
        // Use integer-only path - no precision loss
        format_integer(value as i64, section, opts)
    } else {
        // Use floating-point path
        format_float(value, section, opts)
    }
}
```

**Files to modify**:
- `src/formatter/number.rs` - Split into integer and float paths
- Add `format_integer()` function
- Update tests

---

## Medium Priority (Code Quality)

### 4. Unified Placeholder Formatting
**Status**: ✅ COMPLETED
**Impact**: Reduce code duplication
**Effort**: Medium

**Problem**: `format_fraction_part()` duplicates logic found elsewhere.

**SSF Approach**: Single `write_num()` function (bits/59_numhelp.js)

**Solution**: Create `src/formatter/placeholder.rs`:
```rust
pub enum Alignment {
    Left,
    Right,
}

pub fn format_with_placeholders(
    value: u32,
    placeholders: &[DigitPlaceholder],
    alignment: Alignment,
) -> String {
    // Unified implementation
}
```

**Files to modify**:
- Create `src/formatter/placeholder.rs`
- Update `src/formatter/fraction.rs` to use it
- Update `src/formatter/number.rs` to use it
- Update `src/formatter/date.rs` for numeric date components

---

### 5. Calendar Mode in Format Context
**Status**: ✅ COMPLETED (covered by #1)
**Impact**: Cleaner design
**Effort**: Low

**Problem**: We detect Hijri calendar by scanning format parts during formatting.

**SSF Approach**: Pass `b2` boolean parameter through entire chain

**Solution**: Store in SectionMetadata (completed as part of #1)

---

## Low Priority (Micro-optimizations)

### 6. String Building Optimization
**Status**: ⏳ Not started
**Impact**: Minor performance improvement
**Effort**: Low

**Current**:
```rust
result.push(' ');
result.push_str(&num_str);
result.push('/');
```

**Better**:
```rust
// Pre-allocate when possible
let capacity = estimate_output_size(section);
let mut result = String::with_capacity(capacity);

// Or use write! for clarity
write!(result, " {}/{}", num_str, denom_str)?;
```

---

## Completed Items

### ✅ Task #1: Section Metadata to Eliminate Repeated Scanning
- Added `SectionMetadata` struct with pre-computed format characteristics
- Created `TimeUnit` and `FormatType` enums
- Modified parser to compute metadata once during parsing
- Updated formatters to use metadata instead of repeated scans
- **Commit**: f32d4d2

### ✅ Task #2: Apply Pre-Rounding Uniformly to All Time Formatting
- Added `apply_time_prerounding()` function
- Implements SSF's pre-rounding algorithm based on smallest displayed time unit
- Ensures consistent rounding across all date/time formats
- **Commit**: 2610aee

### ✅ Task #3: Add Integer Fast Path for Large Numbers
- Added `format_number_as_integer()` for integer-only arithmetic
- Modified `format_number()` to detect and dispatch safe integers
- Avoids precision loss for large integers
- **Commit**: 93573ad

### ✅ Task #4: Unified Placeholder Formatting
- Added `format_simple_with_placeholders()` helper in number.rs
- Unified logic for mapping digits to placeholders
- Updated fraction formatting to use the helper
- Eliminated 32 lines of duplicated code
- **Commit**: 2659ce4

### ✅ Task #5: Calendar Mode in Format Context
- Completed as part of Task #1
- `is_hijri` field stored in SectionMetadata
- No longer need to scan parts during formatting

---

## Notes

- Priority order based on impact vs effort
- Items #1, #2, #3 would give the most value
- Item #1 (metadata) would make #2 and #5 easier to implement
- Item #4 is independent and can be done anytime
