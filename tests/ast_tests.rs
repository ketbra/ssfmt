use ssfmt::ast::{Condition, DatePart, DigitPlaceholder, FormatPart, NamedColor};

#[test]
fn test_named_color_from_str() {
    assert_eq!("Red".parse::<NamedColor>().unwrap(), NamedColor::Red);
    assert_eq!("blue".parse::<NamedColor>().unwrap(), NamedColor::Blue);
    assert!("invalid".parse::<NamedColor>().is_err());
}

#[test]
fn test_condition_evaluate() {
    let cond = Condition::GreaterThan(100.0);
    assert!(cond.evaluate(150.0));
    assert!(!cond.evaluate(50.0));
    assert!(!cond.evaluate(100.0));
}

#[test]
fn test_digit_placeholder_properties() {
    assert!(DigitPlaceholder::Zero.is_required());
    assert!(!DigitPlaceholder::Hash.is_required());
    assert!(!DigitPlaceholder::Question.is_required());
}

#[test]
fn test_format_part_is_date_part() {
    let year = FormatPart::DatePart(DatePart::Year4);
    let digit = FormatPart::Digit(DigitPlaceholder::Zero);

    assert!(year.is_date_part());
    assert!(!digit.is_date_part());
}
