use super::*;
use crate::{Date, Decimal, Integer, Token};
use alloc::string::String;
use bytes::Bytes;

fn parse(input: &'static str) -> (Result<Parameters, SyntaxError>, Bytes) {
    let mut parser = Parser::new(Bytes::from_static(input.as_bytes()));
    let result = Parameters::parse(&mut parser);
    (result, Bytes::copy_from_slice(parser.rest()))
}

fn ok(input: &'static str) -> Parameters {
    parse(input).0.unwrap()
}

fn err(input: &'static str) -> SyntaxError {
    parse(input).0.unwrap_err()
}

fn serialized(parameters: &Parameters) -> String {
    let mut out = Vec::new();
    parameters.serialize(&mut out);
    String::from_utf8(out).unwrap()
}

fn key(text: &'static str) -> Key {
    Key::from_static(text).unwrap()
}

fn int(value: i64) -> BareItem {
    BareItem::Integer(Integer::new(value).unwrap())
}

fn pairs(parameters: &Parameters) -> Vec<(&str, &BareItem)> {
    parameters.iter().map(|(k, v)| (k.as_str(), v)).collect()
}

#[test]
fn no_semicolon_no_parameters() {
    let (result, rest) = parse("");
    assert!(result.unwrap().is_empty());
    assert!(rest.is_empty());
    let (result, rest) = parse(", a");
    assert!(result.unwrap().is_empty());
    assert_eq!(&rest[..], b", a");
}

#[test]
fn parses_values_of_every_type() {
    let parameters = ok(";a=1;b=1.5;c=\"s\";d=tok;e=:AA==:;f=?0;g=@5;h=%\"x\"");
    assert_eq!(parameters.len(), 8);
    assert_eq!(parameters.get("a"), Some(&int(1)));
    assert_eq!(
        parameters.get("b"),
        Some(&BareItem::Decimal(Decimal::from_thousandths(1500).unwrap()))
    );
    assert_eq!(
        parameters.get("c"),
        Some(&BareItem::String(crate::String::from_static("s").unwrap()))
    );
    assert_eq!(
        parameters.get("d"),
        Some(&BareItem::Token(Token::from_static("tok").unwrap()))
    );
    assert_eq!(
        parameters.get("e"),
        Some(&BareItem::ByteSequence(Bytes::from_static(&[0])))
    );
    assert_eq!(parameters.get("f"), Some(&BareItem::Boolean(false)));
    assert_eq!(
        parameters.get("g"),
        Some(&BareItem::Date(Date::from_seconds(
            Integer::new(5).unwrap()
        )))
    );
    assert_eq!(
        parameters.get("h"),
        Some(&BareItem::DisplayString(String::from("x")))
    );
    assert_eq!(parameters.get("z"), None);
}

#[test]
fn a_key_alone_is_true() {
    let parameters = ok(";a;b=?1;c=?0");
    assert_eq!(parameters.get("a"), Some(&BareItem::Boolean(true)));
    assert_eq!(parameters.get("b"), Some(&BareItem::Boolean(true)));
    assert_eq!(parameters.get("c"), Some(&BareItem::Boolean(false)));
    assert_eq!(serialized(&parameters), ";a;b;c=?0");
}

#[test]
fn spaces_after_the_semicolon_are_skipped() {
    let parameters = ok("; a=1;   b=2; cde_456");
    assert_eq!(
        pairs(&parameters)
            .iter()
            .map(|(k, _)| *k)
            .collect::<Vec<_>>(),
        ["a", "b", "cde_456"]
    );
    assert_eq!(serialized(&parameters), ";a=1;b=2;cde_456");
}

#[test]
fn a_space_before_the_semicolon_ends_the_parameters() {
    let (result, rest) = parse(";a=1 ;b=2");
    assert_eq!(result.unwrap().len(), 1);
    assert_eq!(&rest[..], b" ;b=2");
    let (result, rest) = parse(";a=1, b");
    assert_eq!(result.unwrap().len(), 1);
    assert_eq!(&rest[..], b", b");
}

#[test]
fn repeated_key_takes_the_last_value_in_the_first_place() {
    let parameters = ok(";a=1;b=2;a=3");
    assert_eq!(parameters.len(), 2);
    assert_eq!(pairs(&parameters), [("a", &int(3)), ("b", &int(2))]);
    assert_eq!(serialized(&parameters), ";a=3;b=2");
    let parameters = ok(";a;a=?0;b;a");
    assert_eq!(pairs(&parameters).len(), 2);
    assert_eq!(parameters.get("a"), Some(&BareItem::Boolean(true)));
}

#[test]
fn parameter_errors() {
    assert_eq!(err(";"), SyntaxError::InvalidKeyStart { index: 1 });
    assert_eq!(err("; "), SyntaxError::InvalidKeyStart { index: 2 });
    assert_eq!(err(";A=1"), SyntaxError::InvalidKeyStart { index: 1 });
    assert_eq!(err(";1"), SyntaxError::InvalidKeyStart { index: 1 });
    assert_eq!(err(";a=1;"), SyntaxError::InvalidKeyStart { index: 5 });
    assert_eq!(err(";a=1;;b"), SyntaxError::InvalidKeyStart { index: 5 });
    assert_eq!(err(";a="), SyntaxError::InvalidBareItemStart { index: 3 });
    assert_eq!(err(";a=;b"), SyntaxError::InvalidBareItemStart { index: 3 });
    assert_eq!(
        err(";a=(1)"),
        SyntaxError::InvalidBareItemStart { index: 3 }
    );
    assert_eq!(err(";a=?x"), SyntaxError::InvalidBoolean { index: 4 });
    assert_eq!(err(";a=@1.5"), SyntaxError::DecimalDate { index: 4 });
    assert_eq!(err(";a=1.2345"), SyntaxError::FractionTooLong { index: 8 });
}

#[test]
fn insert_returns_the_old_value_and_keeps_the_place() {
    let mut parameters = Parameters::new();
    assert!(parameters.is_empty());
    assert_eq!(parameters.insert(key("a"), int(1)), None);
    assert_eq!(parameters.insert(key("b"), int(2)), None);
    assert_eq!(parameters.insert(key("c"), int(3)), None);
    assert_eq!(parameters.insert(key("a"), int(10)), Some(int(1)));
    assert_eq!(
        parameters.insert(key("b"), BareItem::Boolean(false)),
        Some(int(2))
    );
    assert_eq!(parameters.len(), 3);
    assert!(!parameters.is_empty());
    assert_eq!(
        pairs(&parameters),
        [
            ("a", &int(10)),
            ("b", &BareItem::Boolean(false)),
            ("c", &int(3))
        ]
    );
    assert_eq!(parameters.iter().len(), 3);
}

#[test]
fn from_iterator_merges_repeated_keys() {
    let parameters: Parameters = [(key("a"), int(1)), (key("b"), int(2)), (key("a"), int(3))]
        .into_iter()
        .collect();
    assert_eq!(pairs(&parameters), [("a", &int(3)), ("b", &int(2))]);
}

#[test]
fn default_is_empty() {
    let parameters = Parameters::default();
    assert!(parameters.is_empty());
    assert_eq!(parameters.len(), 0);
    assert_eq!(parameters, Parameters::new());
    assert_eq!(serialized(&parameters), "");
}

#[test]
fn serializes_true_as_the_key_alone() {
    let mut parameters = Parameters::new();
    parameters.insert(key("a"), BareItem::Boolean(true));
    parameters.insert(key("b"), BareItem::Boolean(false));
    parameters.insert(key("c"), int(-1));
    parameters.insert(
        key("d"),
        BareItem::Decimal(Decimal::from_thousandths(2000).unwrap()),
    );
    parameters.insert(
        key("e"),
        BareItem::String(crate::String::from_static("x\"").unwrap()),
    );
    assert_eq!(serialized(&parameters), ";a;b=?0;c=-1;d=2.0;e=\"x\\\"\"");
}

#[test]
fn parse_then_serialize_is_canonical() {
    let rfc_example = ";a=1;b=2;cde_456";
    assert_eq!(serialized(&ok(rfc_example)), rfc_example);
    assert_eq!(
        serialized(&ok(";q=0.500;r=\"+w\";s=?1")),
        ";q=0.5;r=\"+w\";s"
    );
}
