use super::*;
use crate::{BareItem, Dictionary, Integer};
use alloc::string::ToString;

fn value(text: &'static str) -> FieldValue {
    FieldValue::from_static(text).unwrap()
}

fn round_trip_list(text: &'static str, expected: &str) {
    let list = List::from_field_value(&value(text)).unwrap();
    let out = list
        .to_field_value()
        .map(|v| core::str::from_utf8(v.as_bytes()).unwrap().to_string());
    assert_eq!(out.as_deref(), Some(expected), "{text}");
}

#[test]
fn smoke() {
    round_trip_list("sugar, tea, rum", "sugar, tea, rum");
    round_trip_list(
        "(\"foo\" \"bar\");lvl=5, (\"baz\");lvl=1",
        "(\"foo\" \"bar\");lvl=5, (\"baz\");lvl=1",
    );
    round_trip_list(
        "1.50, -0.5, 4.000, 12345678901.2",
        "1.5, -0.5, 4.0, 12345678901.2",
    );
    round_trip_list(
        ":cHJldGVuZCB0aGlzIGlzIGJpbmFyeSBjb250ZW50Lg==:",
        ":cHJldGVuZCB0aGlzIGlzIGJpbmFyeSBjb250ZW50Lg==:",
    );
    round_trip_list(
        ":cHJldGVuZCB0aGlzIGlzIGJpbmFyeSBjb250ZW50Lg:",
        ":cHJldGVuZCB0aGlzIGlzIGJpbmFyeSBjb250ZW50Lg==:",
    );
    round_trip_list(
        "%\"This is intended for display to %c3%bcsers.\"",
        "%\"This is intended for display to %c3%bcsers.\"",
    );
    round_trip_list(
        "@1659578233, ?0, \"a\\\"b\\\\c\"",
        "@1659578233, ?0, \"a\\\"b\\\\c\"",
    );
    round_trip_list("abc;a=1;b=2;a=3", "abc;a=3;b=2");
    let d = Dictionary::from_field_value(&value(
        "en=\"Applepie\", da=:w4ZibGV0w6ZydGUK:, a=?1;x, b=1, a=2",
    ))
    .unwrap();
    assert_eq!(
        core::str::from_utf8(d.to_field_value().unwrap().as_bytes()).unwrap(),
        "en=\"Applepie\", da=:w4ZibGV0w6ZydGUK:, a=2, b=1"
    );
    assert_eq!(
        d.get("b"),
        Some(&ListMember::Item(Item::new(BareItem::Integer(
            Integer::new(1).unwrap()
        ))))
    );
    assert!(List::from_field_value(&value("a,")).is_err());
    assert!(List::from_field_value(&value("1234567890123456")).is_err());
    assert!(List::from_field_value(&value("1.2345")).is_err());
    let mut fields = FieldSection::new();
    let name = FieldName::from_static("example-list").unwrap();
    fields.append(name.clone(), value("a, b"));
    fields.append(name.clone(), value("c"));
    assert_eq!(List::from_fields(&fields, &name).unwrap().unwrap().len(), 3);
    fields.append(name.clone(), FieldValue::from_static("").unwrap());
    assert!(matches!(
        List::from_fields(&fields, &name),
        Err(FieldLinesError::EmptyLine { line: 2 })
    ));
}

fn text(value: &FieldValue) -> &str {
    core::str::from_utf8(value.as_bytes()).unwrap()
}

fn parse(input: &'static str) -> List {
    List::from_field_value(&value(input)).unwrap()
}

fn canonical(input: &'static str) -> alloc::string::String {
    text(&parse(input).to_field_value().unwrap()).to_string()
}

fn error(input: &'static str) -> SyntaxError {
    List::from_field_value(&value(input)).unwrap_err()
}

fn name() -> FieldName {
    FieldName::from_static("example-list").unwrap()
}

fn section(lines: &[&'static str]) -> FieldSection {
    let mut fields = FieldSection::new();
    for line in lines {
        fields.append(name(), value(line));
    }
    fields
}

fn int_item(n: i64) -> ListMember {
    ListMember::Item(Item::new(BareItem::Integer(Integer::new(n).unwrap())))
}

#[test]
fn rfc_9651_lists_are_canonical() {
    for text in [
        "sugar, tea, rum",
        "(\"foo\" \"bar\"), (\"baz\"), (\"bat\" \"one\"), ()",
        "(\"foo\";a=1;b=2);lvl=5, (\"bar\" \"baz\");lvl=1",
        "abc;a=1;b=2;cde_456, (ghi;jkl=4 mno);q=9;r=\"+w\"",
        "1, 42",
        "a",
    ] {
        assert_eq!(canonical(text), text);
    }
}

#[test]
fn rfc_9651_lists_with_loose_spacing() {
    assert_eq!(
        canonical("(\"foo\"; a=1;b=2);lvl=5, (\"bar\" \"baz\");lvl=1"),
        "(\"foo\";a=1;b=2);lvl=5, (\"bar\" \"baz\");lvl=1"
    );
    assert_eq!(
        canonical("abc;a=1;b=2; cde_456, (ghi;jkl=4 mno);q=9;r=\"+w\""),
        "abc;a=1;b=2;cde_456, (ghi;jkl=4 mno);q=9;r=\"+w\""
    );
    assert_eq!(canonical("a,b,c"), "a, b, c");
    assert_eq!(canonical("a ,\tb  ,   c"), "a, b, c");
    assert_eq!(canonical("a,\t\tb"), "a, b");
}

#[test]
fn members_are_items_and_inner_lists() {
    let list = parse("a, (b c);x, 3");
    assert_eq!(list.len(), 3);
    assert!(matches!(list.as_slice()[0], ListMember::Item(_)));
    match &list.as_slice()[1] {
        ListMember::InnerList(inner) => {
            assert_eq!(inner.items().len(), 2);
            assert_eq!(inner.parameters().len(), 1);
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(list.as_slice()[2], int_item(3));
}

#[test]
fn an_empty_value_is_the_empty_list() {
    let list = parse("");
    assert!(list.is_empty());
    assert_eq!(list.len(), 0);
    assert_eq!(list, List::new());
    assert_eq!(list.to_field_value(), None);
}

#[test]
fn list_syntax_errors() {
    assert_eq!(error("a,"), SyntaxError::TrailingComma { index: 2 });
    assert_eq!(error("a, b,"), SyntaxError::TrailingComma { index: 5 });
    assert_eq!(error("a b"), SyntaxError::ExpectedComma { index: 2 });
    assert_eq!(error("a;x=1 b"), SyntaxError::ExpectedComma { index: 6 });
    assert_eq!(error("(a)b"), SyntaxError::ExpectedComma { index: 3 });
    assert_eq!(error("a;b=2c"), SyntaxError::ExpectedComma { index: 5 });
    assert_eq!(error(",a"), SyntaxError::InvalidBareItemStart { index: 0 });
    assert_eq!(
        error("a,,b"),
        SyntaxError::InvalidBareItemStart { index: 2 }
    );
    assert_eq!(
        error("a, ,b"),
        SyntaxError::InvalidBareItemStart { index: 3 }
    );
    assert_eq!(error("a, (b"), SyntaxError::UnclosedInnerList { index: 3 });
    assert_eq!(
        error("1234567890123456"),
        SyntaxError::IntegerTooLong { index: 15 }
    );
    assert_eq!(error("1.2345"), SyntaxError::FractionTooLong { index: 5 });
    assert_eq!(error("1, \"a"), SyntaxError::UnclosedString { index: 3 });
}

#[test]
fn constructors_and_accessors() {
    let mut list = List::new();
    assert!(list.is_empty());
    list.push(int_item(1));
    list.push(ListMember::InnerList(InnerList::new(alloc::vec![
        Item::new(BareItem::Boolean(true))
    ])));
    assert_eq!(list.len(), 2);
    assert!(!list.is_empty());
    assert_eq!(text(&list.to_field_value().unwrap()), "1, (?1)");
    assert_eq!(list.clone().into_vec().len(), 2);
    assert_eq!(List::from(list.clone().into_vec()), list);
    let collected: List = [int_item(1), int_item(2)].into_iter().collect();
    assert_eq!(text(&collected.to_field_value().unwrap()), "1, 2");
    assert_eq!(
        ListMember::from(Item::new(BareItem::Boolean(false))).clone(),
        ListMember::Item(Item::new(BareItem::Boolean(false)))
    );
    assert_eq!(
        ListMember::from(InnerList::default()),
        ListMember::InnerList(InnerList::default())
    );
}

#[test]
fn empty_list_has_no_field_value() {
    assert_eq!(List::new().to_field_value(), None);
    assert_eq!(List::default().to_field_value(), None);
}

#[test]
fn from_fields_without_the_field_is_none() {
    assert_eq!(List::from_fields(&FieldSection::new(), &name()), Ok(None));
    let mut fields = FieldSection::new();
    fields.append(FieldName::from_static("other").unwrap(), value("a"));
    assert_eq!(List::from_fields(&fields, &name()), Ok(None));
}

#[test]
fn from_fields_with_one_empty_line_is_the_empty_list() {
    assert_eq!(
        List::from_fields(&section(&[""]), &name()),
        Ok(Some(List::new()))
    );
}

#[test]
fn from_fields_combines_the_lines() {
    let list = List::from_fields(&section(&["1, 2", "3", "(4 5), 6"]), &name())
        .unwrap()
        .unwrap();
    assert_eq!(list.len(), 5);
    assert_eq!(list.as_slice()[0], int_item(1));
    assert_eq!(list.as_slice()[2], int_item(3));
    assert_eq!(text(&list.to_field_value().unwrap()), "1, 2, 3, (4 5), 6");
}

#[test]
fn from_fields_fails_on_a_bad_line() {
    assert_eq!(
        List::from_fields(&section(&["1", "2 3"]), &name()),
        Err(FieldLinesError::Syntax {
            line: 1,
            source: SyntaxError::ExpectedComma { index: 2 }
        })
    );
    assert_eq!(
        List::from_fields(&section(&["1", "", "2"]), &name()),
        Err(FieldLinesError::EmptyLine { line: 1 })
    );
    assert_eq!(
        List::from_fields(&section(&["", "1"]), &name()),
        Err(FieldLinesError::EmptyLine { line: 0 })
    );
    assert_eq!(
        List::from_fields(&section(&["1", ""]), &name()),
        Err(FieldLinesError::EmptyLine { line: 1 })
    );
}

#[test]
fn members_across_lines_in_order() {
    let fields = section(&["a, b", "(c d);x", "5"]);
    let name = name();
    let members: Vec<_> = List::members(&fields, &name).map(Result::unwrap).collect();
    assert_eq!(members.len(), 4);
    assert!(matches!(members[0], ListMember::Item(_)));
    assert!(matches!(members[2], ListMember::InnerList(_)));
    assert_eq!(members[3], int_item(5));
}

#[test]
fn members_end_after_an_error() {
    let fields = section(&["a", "b c", "d"]);
    let name = name();
    let mut members = List::members(&fields, &name);
    assert!(matches!(members.next(), Some(Ok(_))));
    assert!(matches!(members.next(), Some(Ok(_))));
    assert_eq!(
        members.next(),
        Some(Err(FieldLinesError::Syntax {
            line: 1,
            source: SyntaxError::ExpectedComma { index: 2 }
        }))
    );
    assert_eq!(members.next(), None);
    assert_eq!(members.next(), None);
}

#[test]
fn members_of_one_empty_line_are_none() {
    let fields = section(&[""]);
    let name = name();
    assert_eq!(List::members(&fields, &name).count(), 0);
}

#[test]
fn members_of_a_list_with_an_empty_line_among_several() {
    let fields = section(&["a", "", "b"]);
    let name = name();
    let all: Vec<_> = List::members(&fields, &name).collect();
    assert_eq!(all.len(), 2);
    assert!(all[0].is_ok());
    assert_eq!(all[1], Err(FieldLinesError::EmptyLine { line: 1 }));
}
