use super::*;
use crate::{Decimal, Integer, Key, SfString, SfToken};
use alloc::string::{String, ToString};

fn fv(text: &'static str) -> FieldValue {
    FieldValue::from_static(text).unwrap()
}

fn text(value: &FieldValue) -> &str {
    core::str::from_utf8(value.as_bytes()).unwrap()
}

fn parse(input: &'static str) -> Item {
    Item::from_field_value(&fv(input)).unwrap()
}

fn error(input: &'static str) -> SyntaxError {
    Item::from_field_value(&fv(input)).unwrap_err()
}

fn canonical(input: &'static str) -> String {
    text(&parse(input).to_field_value()).to_string()
}

fn int(value: i64) -> BareItem {
    BareItem::Integer(Integer::new(value).unwrap())
}

fn name() -> FieldName {
    FieldName::from_static("example-item").unwrap()
}

#[test]
fn parses_the_items_of_rfc_9651() {
    assert_eq!(parse("5").bare_item(), &int(5));
    assert_eq!(
        parse("4.5").bare_item(),
        &BareItem::Decimal(Decimal::from_thousandths(4500).unwrap())
    );
    assert_eq!(
        parse("\"hello world\"").bare_item(),
        &BareItem::String(SfString::from_static("hello world").unwrap())
    );
    assert_eq!(
        parse("foo123/456").bare_item(),
        &BareItem::Token(SfToken::from_static("foo123/456").unwrap())
    );
    assert_eq!(parse("?1").bare_item(), &BareItem::Boolean(true));
    assert_eq!(
        parse(":cHJldGVuZCB0aGlzIGlzIGJpbmFyeSBjb250ZW50Lg==:").bare_item(),
        &BareItem::ByteSequence(bytes::Bytes::from_static(
            b"pretend this is binary content."
        ))
    );
    assert_eq!(
        parse("@1659578233").bare_item(),
        &BareItem::Date(crate::Date::from_seconds(
            Integer::new(1_659_578_233).unwrap()
        ))
    );
    assert_eq!(
        parse("%\"This is intended for display to %c3%bcsers.\"").bare_item(),
        &BareItem::DisplayString("This is intended for display to \u{fc}sers.".to_string())
    );
}

#[test]
fn parses_parameters() {
    let item = parse("1; a; b=?0");
    assert_eq!(item.bare_item(), &int(1));
    assert_eq!(item.parameters().len(), 2);
    assert_eq!(item.parameters().get("a"), Some(&BareItem::Boolean(true)));
    assert_eq!(item.parameters().get("b"), Some(&BareItem::Boolean(false)));
    assert_eq!(text(&item.to_field_value()), "1;a;b=?0");
}

#[test]
fn canonical_forms() {
    assert_eq!(canonical("1.50"), "1.5");
    assert_eq!(canonical("4.000"), "4.0");
    assert_eq!(canonical("abc;a=1;b=2;a=3"), "abc;a=3;b=2");
    assert_eq!(canonical(":AA:"), ":AA==:");
    assert_eq!(canonical("?1;x=?1"), "?1;x");
    assert_eq!(canonical("%\"%41\""), "%\"A\"");
}

#[test]
fn trailing_bytes_fail() {
    assert_eq!(error("1 2"), SyntaxError::TrailingCharacters { index: 2 });
    assert_eq!(error("1, 2"), SyntaxError::TrailingCharacters { index: 1 });
    assert_eq!(error("a b"), SyntaxError::TrailingCharacters { index: 2 });
    assert_eq!(
        error("1;a=1 b"),
        SyntaxError::TrailingCharacters { index: 6 }
    );
    assert_eq!(error("1)"), SyntaxError::TrailingCharacters { index: 1 });
    assert_eq!(
        error("\"a\"\"b\""),
        SyntaxError::TrailingCharacters { index: 3 }
    );
}

#[test]
fn syntax_errors_of_the_parts_pass_through() {
    assert_eq!(error(""), SyntaxError::InvalidBareItemStart { index: 0 });
    assert_eq!(error("(a)"), SyntaxError::InvalidBareItemStart { index: 0 });
    assert_eq!(error("1;"), SyntaxError::InvalidKeyStart { index: 2 });
    assert_eq!(error("1;A"), SyntaxError::InvalidKeyStart { index: 2 });
    assert_eq!(
        error("1;a="),
        SyntaxError::InvalidBareItemStart { index: 4 }
    );
    assert_eq!(error("-"), SyntaxError::MissingDigit { index: 1 });
    assert_eq!(error("\"abc"), SyntaxError::UnclosedString { index: 0 });
}

#[test]
fn from_field_value_accepts_bytes_beyond_ascii_only_where_valid() {
    let value = FieldValue::new(bytes::Bytes::from_static(b"\"\xc3\xbc\"")).unwrap();
    assert_eq!(
        Item::from_field_value(&value),
        Err(SyntaxError::InvalidStringCharacter { index: 1 })
    );
}

#[test]
fn accessors_and_constructors() {
    let item = Item::new(int(1));
    assert_eq!(item.bare_item(), &int(1));
    assert!(item.parameters().is_empty());

    let mut parameters = Parameters::new();
    parameters.insert(Key::from_static("a").unwrap(), int(2));
    let item = Item::with_parameters(int(1), parameters.clone());
    assert_eq!(item.parameters(), &parameters);
    assert_eq!(text(&item.to_field_value()), "1;a=2");
    assert_eq!(item.clone().into_parts(), (int(1), parameters));
    assert_eq!(Item::from(int(7)), Item::new(int(7)));
}

#[test]
fn to_field_value_of_each_type() {
    assert_eq!(text(&Item::new(int(-5)).to_field_value()), "-5");
    assert_eq!(
        text(&Item::new(BareItem::Boolean(false)).to_field_value()),
        "?0"
    );
    assert_eq!(
        text(&Item::new(BareItem::DisplayString("\u{fc}".to_string())).to_field_value()),
        "%\"%c3%bc\""
    );
}

#[test]
fn from_fields_without_the_field_is_none() {
    let mut fields = FieldSection::new();
    assert_eq!(Item::from_fields(&fields, &name()), Ok(None));
    fields.append(FieldName::from_static("other").unwrap(), fv("1"));
    assert_eq!(Item::from_fields(&fields, &name()), Ok(None));
}

#[test]
fn from_fields_parses_the_single_line() {
    let mut fields = FieldSection::new();
    fields.append(FieldName::from_static("other").unwrap(), fv("x y"));
    fields.append(name(), fv("5;a"));
    fields.append(FieldName::from_static("another").unwrap(), fv("z"));
    let item = Item::from_fields(&fields, &name()).unwrap().unwrap();
    assert_eq!(item.bare_item(), &int(5));
    assert_eq!(item.parameters().get("a"), Some(&BareItem::Boolean(true)));
}

#[test]
fn from_fields_rejects_several_lines() {
    let mut fields = FieldSection::new();
    fields.append(name(), fv("1"));
    fields.append(name(), fv("2"));
    assert_eq!(
        Item::from_fields(&fields, &name()),
        Err(ItemFieldError::SeveralLines)
    );
    // Several lines fail even when the first is not an item.
    let mut fields = FieldSection::new();
    fields.append(name(), fv("("));
    fields.append(name(), fv("2"));
    assert_eq!(
        Item::from_fields(&fields, &name()),
        Err(ItemFieldError::SeveralLines)
    );
}

#[test]
fn from_fields_reports_the_syntax_error() {
    let mut fields = FieldSection::new();
    fields.append(name(), fv("1 2"));
    assert_eq!(
        Item::from_fields(&fields, &name()),
        Err(ItemFieldError::Syntax {
            source: SyntaxError::TrailingCharacters { index: 2 }
        })
    );
    let mut fields = FieldSection::new();
    fields.append(name(), fv(""));
    assert_eq!(
        Item::from_fields(&fields, &name()),
        Err(ItemFieldError::Syntax {
            source: SyntaxError::InvalidBareItemStart { index: 0 }
        })
    );
}

#[test]
fn error_display_and_source() {
    use core::error::Error;
    assert_eq!(
        ItemFieldError::SeveralLines.to_string(),
        "an item field has several lines"
    );
    assert!(ItemFieldError::SeveralLines.source().is_none());
    let error = ItemFieldError::Syntax {
        source: SyntaxError::TrailingCharacters { index: 2 },
    };
    assert_eq!(error.to_string(), "the field line is not an item");
    assert_eq!(
        error.source().unwrap().to_string(),
        "unexpected bytes after the item at byte 2"
    );
}
