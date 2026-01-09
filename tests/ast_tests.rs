use ssfmt::ast::{Condition, NamedColor};

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
