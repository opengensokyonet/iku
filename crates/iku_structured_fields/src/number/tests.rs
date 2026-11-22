use super::*;
use alloc::string::ToString;
use bytes::Bytes;

fn parse(text: &'static str) -> (Result<Number, SyntaxError>, Bytes) {
    let mut parser = Parser::new(Bytes::from_static(text.as_bytes()));
    let result = parse_number(&mut parser);
    (result, Bytes::copy_from_slice(parser.rest()))
}

fn integer(text: &'static str) -> i64 {
    match parse(text).0 {
        Ok(Number::Integer(integer)) => integer.get(),
        Ok(Number::Decimal(_)) => panic!("{text} parsed as a decimal"),
        Err(error) => panic!("{text} failed: {error:?}"),
    }
}

fn decimal(text: &'static str) -> i64 {
    match parse(text).0 {
        Ok(Number::Decimal(decimal)) => decimal.thousandths(),
        Ok(Number::Integer(_)) => panic!("{text} parsed as an integer"),
        Err(error) => panic!("{text} failed: {error:?}"),
    }
}

fn error(text: &'static str) -> SyntaxError {
    match parse(text).0 {
        Err(error) => error,
        Ok(_) => panic!("{text} parsed"),
    }
}

fn serialized_decimal(thousandths: i64) -> alloc::string::String {
    let mut out = Vec::new();
    Decimal::from_thousandths(thousandths)
        .unwrap()
        .serialize(&mut out);
    alloc::string::String::from_utf8(out).unwrap()
}

#[test]
fn parses_integers() {
    assert_eq!(integer("0"), 0);
    assert_eq!(integer("42"), 42);
    assert_eq!(integer("-42"), -42);
    assert_eq!(integer("-0"), 0);
    assert_eq!(integer("0042"), 42);
    assert_eq!(integer("999999999999999"), 999_999_999_999_999);
    assert_eq!(integer("-999999999999999"), -999_999_999_999_999);
}

#[test]
fn integer_stops_at_first_other_byte() {
    let (result, rest) = parse("12abc");
    assert!(matches!(result, Ok(Number::Integer(i)) if i.get() == 12));
    assert_eq!(&rest[..], b"abc");
    let (result, rest) = parse("1,2");
    assert!(matches!(result, Ok(Number::Integer(i)) if i.get() == 1));
    assert_eq!(&rest[..], b",2");
}

#[test]
fn sixteen_digit_integer_fails_at_the_sixteenth_digit() {
    assert_eq!(
        error("1000000000000000"),
        SyntaxError::IntegerTooLong { index: 15 }
    );
    assert_eq!(
        error("-1000000000000000"),
        SyntaxError::IntegerTooLong { index: 16 }
    );
    // Leading zeros count as digits.
    assert_eq!(
        error("0000000000000001"),
        SyntaxError::IntegerTooLong { index: 15 }
    );
}

#[test]
fn parses_decimals() {
    assert_eq!(decimal("1.5"), 1500);
    assert_eq!(decimal("-1.5"), -1500);
    assert_eq!(decimal("0.0"), 0);
    assert_eq!(decimal("-0.0"), 0);
    assert_eq!(decimal("0.1"), 100);
    assert_eq!(decimal("0.01"), 10);
    assert_eq!(decimal("0.001"), 1);
    assert_eq!(decimal("1.050"), 1050);
    assert_eq!(decimal("1.000"), 1000);
    assert_eq!(decimal("4.5"), 4500);
    assert_eq!(decimal("12345678901.2"), 12_345_678_901_200);
    assert_eq!(decimal("999999999999.999"), 999_999_999_999_999);
    assert_eq!(decimal("-999999999999.999"), -999_999_999_999_999);
}

#[test]
fn decimal_stops_at_a_second_point() {
    let (result, rest) = parse("1.2.3");
    assert!(matches!(result, Ok(Number::Decimal(d)) if d.thousandths() == 1200));
    assert_eq!(&rest[..], b".3");
}

#[test]
fn decimal_with_thirteen_integer_digits_fails_at_the_point() {
    assert_eq!(
        error("1234567890123.5"),
        SyntaxError::IntegerPartTooLong { index: 13 }
    );
    assert_eq!(
        error("-1234567890123.5"),
        SyntaxError::IntegerPartTooLong { index: 14 }
    );
    assert_eq!(
        error("123456789012345.5"),
        SyntaxError::IntegerPartTooLong { index: 15 }
    );
    // Sixteen integer digits fail before the point is reached.
    assert_eq!(
        error("1234567890123456.5"),
        SyntaxError::IntegerTooLong { index: 15 }
    );
}

#[test]
fn decimal_with_four_fraction_digits_fails_at_the_fourth() {
    assert_eq!(error("1.2345"), SyntaxError::FractionTooLong { index: 5 });
    assert_eq!(error("1.0000"), SyntaxError::FractionTooLong { index: 5 });
    assert_eq!(error("-1.2345"), SyntaxError::FractionTooLong { index: 6 });
}

#[test]
fn decimal_point_needs_a_digit() {
    assert_eq!(error("1."), SyntaxError::EmptyFraction { index: 1 });
    assert_eq!(error("1.a"), SyntaxError::EmptyFraction { index: 1 });
    assert_eq!(error("-12."), SyntaxError::EmptyFraction { index: 3 });
}

#[test]
fn minus_needs_a_digit() {
    assert_eq!(error("-"), SyntaxError::MissingDigit { index: 1 });
    assert_eq!(error("-a"), SyntaxError::MissingDigit { index: 1 });
    assert_eq!(error("--1"), SyntaxError::MissingDigit { index: 1 });
    assert_eq!(error("-.5"), SyntaxError::MissingDigit { index: 1 });
}

#[test]
fn integer_new_checks_the_range() {
    assert_eq!(Integer::new(0).unwrap().get(), 0);
    assert_eq!(Integer::new(999_999_999_999_999).unwrap(), Integer::MAX);
    assert_eq!(Integer::new(-999_999_999_999_999).unwrap(), Integer::MIN);
    assert_eq!(Integer::MAX.get(), 999_999_999_999_999);
    assert_eq!(Integer::MIN.get(), -999_999_999_999_999);
    let error = Integer::new(1_000_000_000_000_000).unwrap_err();
    assert_eq!(error.value(), 1_000_000_000_000_000);
    let error = Integer::new(-1_000_000_000_000_000).unwrap_err();
    assert_eq!(error.value(), -1_000_000_000_000_000);
    assert!(Integer::new(i64::MAX).is_err());
    assert!(Integer::new(i64::MIN).is_err());
}

#[test]
fn integer_error_display() {
    let error = Integer::new(1_000_000_000_000_000).unwrap_err();
    assert_eq!(
        error.to_string(),
        "1000000000000000 has more than 15 digits"
    );
}

#[test]
fn integer_display_and_serialize() {
    for (value, text) in [
        (0, "0"),
        (7, "7"),
        (-7, "-7"),
        (-999_999_999_999_999, "-999999999999999"),
    ] {
        let integer = Integer::new(value).unwrap();
        assert_eq!(integer.to_string(), text);
        let mut out = Vec::new();
        integer.serialize(&mut out);
        assert_eq!(out, text.as_bytes());
    }
}

#[test]
fn integer_orders_by_value() {
    assert!(Integer::MIN < Integer::new(0).unwrap());
    assert!(Integer::new(0).unwrap() < Integer::MAX);
}

#[test]
fn decimal_from_thousandths_checks_the_range() {
    assert_eq!(Decimal::from_thousandths(1500).unwrap().thousandths(), 1500);
    assert_eq!(
        Decimal::from_thousandths(999_999_999_999_999).unwrap(),
        Decimal::MAX
    );
    assert_eq!(
        Decimal::from_thousandths(-999_999_999_999_999).unwrap(),
        Decimal::MIN
    );
    assert_eq!(Decimal::MAX.thousandths(), 999_999_999_999_999);
    assert_eq!(Decimal::MIN.thousandths(), -999_999_999_999_999);
    let error = Decimal::from_thousandths(1_000_000_000_000_000).unwrap_err();
    assert_eq!(error.thousandths(), 1_000_000_000_000_000);
    let error = Decimal::from_thousandths(-1_000_000_000_000_000).unwrap_err();
    assert_eq!(error.thousandths(), -1_000_000_000_000_000);
}

#[test]
fn decimal_error_display() {
    let error = Decimal::from_thousandths(1_000_000_000_000_000).unwrap_err();
    assert_eq!(
        error.to_string(),
        "1000000000000000 thousandths has more than 12 integer digits"
    );
}

#[test]
fn decimal_serializes_without_trailing_zeros() {
    for (thousandths, text) in [
        (0, "0.0"),
        (1000, "1.0"),
        (-1000, "-1.0"),
        (1500, "1.5"),
        (1050, "1.05"),
        (1005, "1.005"),
        (1230, "1.23"),
        (100, "0.1"),
        (10, "0.01"),
        (1, "0.001"),
        (-1, "-0.001"),
        (-500, "-0.5"),
        (4000, "4.0"),
        (12_345_678_901_200, "12345678901.2"),
        (999_999_999_999_999, "999999999999.999"),
        (-999_999_999_999_999, "-999999999999.999"),
    ] {
        assert_eq!(serialized_decimal(thousandths), text);
        assert_eq!(
            Decimal::from_thousandths(thousandths).unwrap().to_string(),
            text
        );
    }
}

#[test]
fn decimal_round_trips_through_the_parser() {
    for text in ["0.0", "1.5", "-0.001", "999999999999.999", "12.34"] {
        let mut parser = Parser::new(Bytes::copy_from_slice(text.as_bytes()));
        let Ok(Number::Decimal(decimal)) = parse_number(&mut parser) else {
            panic!("{text}");
        };
        assert_eq!(decimal.to_string(), text);
    }
}
