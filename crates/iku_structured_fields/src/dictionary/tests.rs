use super::*;
use crate::{InnerList, Integer, SfString, SfToken};
use alloc::string::{String, ToString};

fn fv(text: &'static str) -> FieldValue {
    FieldValue::from_static(text).unwrap()
}

fn text(value: &FieldValue) -> &str {
    core::str::from_utf8(value.as_bytes()).unwrap()
}

fn parse(input: &'static str) -> Dictionary {
    Dictionary::from_field_value(&fv(input)).unwrap()
}

fn error(input: &'static str) -> SyntaxError {
    Dictionary::from_field_value(&fv(input)).unwrap_err()
}

fn canonical(input: &'static str) -> String {
    text(&parse(input).to_field_value().unwrap()).to_string()
}

fn name() -> FieldName {
    FieldName::from_static("example-dict").unwrap()
}

fn section(lines: &[&'static str]) -> FieldSection {
    let mut fields = FieldSection::new();
    for line in lines {
        fields.append(name(), fv(line));
    }
    fields
}

fn key(text: &'static str) -> Key {
    Key::from_static(text).unwrap()
}

fn int(value: i64) -> BareItem {
    BareItem::Integer(Integer::new(value).unwrap())
}

fn int_member(value: i64) -> ListMember {
    ListMember::Item(Item::new(int(value)))
}

fn keys(dictionary: &Dictionary) -> Vec<&str> {
    dictionary.iter().map(|(key, _)| key.as_str()).collect()
}

#[test]
fn rfc_9651_dictionaries() {
    let dictionary = parse("en=\"Applepie\", da=:w4ZibGV0w6ZydGUK:");
    assert_eq!(
        dictionary.get("en"),
        Some(&ListMember::Item(Item::new(BareItem::String(
            SfString::from_static("Applepie").unwrap()
        ))))
    );
    assert_eq!(
        dictionary.get("da"),
        Some(&ListMember::Item(Item::new(BareItem::ByteSequence(
            bytes::Bytes::from_static(b"\xc3\x86blet\xc3\xa6rte\n")
        ))))
    );
    for text in [
        "en=\"Applepie\", da=:w4ZibGV0w6ZydGUK:",
        "rating=1.5, feelings=(joy sadness)",
        "a=(1 2), b=3, c=4;aa=bb, d=(5 6);valid",
        "foo=1, bar=2",
        "a=?0, b, c;foo=bar",
    ] {
        assert_eq!(canonical(text), text);
    }
}

#[test]
fn true_members_are_written_as_the_key_alone() {
    let dictionary = parse("a=?1, b, c;foo=bar, d=?1;x=1");
    assert_eq!(
        dictionary.get("a"),
        Some(&ListMember::Item(Item::new(BareItem::Boolean(true))))
    );
    assert_eq!(
        dictionary.get("b"),
        Some(&ListMember::Item(Item::new(BareItem::Boolean(true))))
    );
    assert_eq!(
        text(&dictionary.to_field_value().unwrap()),
        "a, b, c;foo=bar, d;x=1"
    );
    assert_eq!(canonical("a=?0"), "a=?0");
}

#[test]
fn a_true_inner_list_member_is_not_shortened() {
    assert_eq!(canonical("a=(?1), b=(c)"), "a=(?1), b=(c)");
}

#[test]
fn loose_spacing_is_normalized() {
    assert_eq!(canonical("a=1,b=2"), "a=1, b=2");
    assert_eq!(canonical("a=1 ,\tb=2 ,   c"), "a=1, b=2, c");
    assert_eq!(canonical("a=1; b=2; c, d"), "a=1;b=2;c, d");
}

#[test]
fn members_are_items_and_inner_lists() {
    let dictionary = parse("a=1, b=(c d);x=?0, e=:AA==:");
    assert_eq!(dictionary.len(), 3);
    assert_eq!(dictionary.get("a"), Some(&int_member(1)));
    match dictionary.get("b") {
        Some(ListMember::InnerList(inner)) => {
            assert_eq!(inner.items().len(), 2);
            assert_eq!(inner.parameters().get("x"), Some(&BareItem::Boolean(false)));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn repeated_key_takes_the_last_value_in_the_first_place() {
    let dictionary = parse("a=1, b=2, a=3");
    assert_eq!(keys(&dictionary), ["a", "b"]);
    assert_eq!(dictionary.get("a"), Some(&int_member(3)));
    assert_eq!(text(&dictionary.to_field_value().unwrap()), "a=3, b=2");
    let dictionary = parse("a=1, b=2, a=3, c, b, a");
    assert_eq!(keys(&dictionary), ["a", "b", "c"]);
    assert_eq!(
        dictionary.get("b"),
        Some(&ListMember::Item(Item::new(BareItem::Boolean(true))))
    );
    assert_eq!(
        dictionary.get("a"),
        Some(&ListMember::Item(Item::new(BareItem::Boolean(true))))
    );
    assert_eq!(canonical("a=1, b=2, a=(3)"), "a=(3), b=2");
}

#[test]
fn empty_value_is_the_empty_dictionary() {
    let dictionary = parse("");
    assert!(dictionary.is_empty());
    assert_eq!(dictionary, Dictionary::new());
    assert_eq!(dictionary.to_field_value(), None);
    assert_eq!(Dictionary::default().to_field_value(), None);
}

#[test]
fn dictionary_syntax_errors() {
    assert_eq!(error(","), SyntaxError::InvalidKeyStart { index: 0 });
    assert_eq!(error("A=1"), SyntaxError::InvalidKeyStart { index: 0 });
    assert_eq!(error("1=1"), SyntaxError::InvalidKeyStart { index: 0 });
    assert_eq!(error("a=1, B=2"), SyntaxError::InvalidKeyStart { index: 5 });
    assert_eq!(error("a=1,,b=2"), SyntaxError::InvalidKeyStart { index: 4 });
    assert_eq!(error("a="), SyntaxError::InvalidBareItemStart { index: 2 });
    assert_eq!(
        error("a=, b"),
        SyntaxError::InvalidBareItemStart { index: 2 }
    );
    assert_eq!(error("a=1,"), SyntaxError::TrailingComma { index: 4 });
    assert_eq!(error("a=1 b=2"), SyntaxError::ExpectedComma { index: 4 });
    assert_eq!(error("a b"), SyntaxError::ExpectedComma { index: 2 });
    assert_eq!(error("a=1;"), SyntaxError::InvalidKeyStart { index: 4 });
    assert_eq!(error("a=(1"), SyntaxError::UnclosedInnerList { index: 2 });
    assert_eq!(error("a=1.2345"), SyntaxError::FractionTooLong { index: 7 });
    assert_eq!(error("a=@1.5"), SyntaxError::DecimalDate { index: 3 });
    assert_eq!(error("a:1"), SyntaxError::ExpectedComma { index: 1 });
}

#[test]
fn insert_returns_the_old_value_and_keeps_the_place() {
    let mut dictionary = Dictionary::new();
    assert_eq!(dictionary.insert(key("a"), int_member(1)), None);
    assert_eq!(dictionary.insert(key("b"), int_member(2)), None);
    assert_eq!(
        dictionary.insert(key("a"), int_member(3)),
        Some(int_member(1))
    );
    assert_eq!(dictionary.len(), 2);
    assert!(!dictionary.is_empty());
    assert_eq!(keys(&dictionary), ["a", "b"]);
    assert_eq!(dictionary.get("a"), Some(&int_member(3)));
    assert_eq!(dictionary.get("b"), Some(&int_member(2)));
    assert_eq!(dictionary.get("c"), None);
    assert_eq!(dictionary.iter().len(), 2);
    assert_eq!(text(&dictionary.to_field_value().unwrap()), "a=3, b=2");
}

#[test]
fn from_iterator_merges_repeated_keys() {
    let dictionary: Dictionary = [
        (key("a"), int_member(1)),
        (key("b"), int_member(2)),
        (key("a"), int_member(3)),
    ]
    .into_iter()
    .collect();
    assert_eq!(keys(&dictionary), ["a", "b"]);
    assert_eq!(dictionary.get("a"), Some(&int_member(3)));
}

#[test]
fn serializes_inner_lists_and_tokens() {
    let mut dictionary = Dictionary::new();
    dictionary.insert(
        key("a"),
        ListMember::InnerList(InnerList::new(alloc::vec![
            Item::new(int(1)),
            Item::new(BareItem::Token(SfToken::from_static("t").unwrap())),
        ])),
    );
    dictionary.insert(
        key("b"),
        ListMember::Item(Item::new(BareItem::Boolean(true))),
    );
    assert_eq!(text(&dictionary.to_field_value().unwrap()), "a=(1 t), b");
}

#[test]
fn from_fields_without_the_field_is_none() {
    assert_eq!(
        Dictionary::from_fields(&FieldSection::new(), &name()),
        Ok(None)
    );
    let mut fields = FieldSection::new();
    fields.append(FieldName::from_static("other").unwrap(), fv("a=1"));
    assert_eq!(Dictionary::from_fields(&fields, &name()), Ok(None));
}

#[test]
fn from_fields_with_one_empty_line_is_the_empty_dictionary() {
    assert_eq!(
        Dictionary::from_fields(&section(&[""]), &name()),
        Ok(Some(Dictionary::new()))
    );
}

#[test]
fn from_fields_combines_the_lines() {
    let dictionary = Dictionary::from_fields(&section(&["a=1, b=2", "c", "d=(1 2), a=9"]), &name())
        .unwrap()
        .unwrap();
    assert_eq!(keys(&dictionary), ["a", "b", "c", "d"]);
    assert_eq!(dictionary.get("a"), Some(&int_member(9)));
    assert_eq!(
        text(&dictionary.to_field_value().unwrap()),
        "a=9, b=2, c, d=(1 2)"
    );
}

#[test]
fn from_fields_fails_on_a_bad_line() {
    assert_eq!(
        Dictionary::from_fields(&section(&["a=1", "B=2"]), &name()),
        Err(FieldLinesError::Syntax {
            line: 1,
            source: SyntaxError::InvalidKeyStart { index: 0 }
        })
    );
    assert_eq!(
        Dictionary::from_fields(&section(&["a=1", "", "b=2"]), &name()),
        Err(FieldLinesError::EmptyLine { line: 1 })
    );
    assert_eq!(
        Dictionary::from_fields(&section(&["", "a=1"]), &name()),
        Err(FieldLinesError::EmptyLine { line: 0 })
    );
}

#[test]
fn entries_give_every_occurrence_of_a_key() {
    let fields = section(&["a=1, b=2", "a=3"]);
    let name = name();
    let entries: Vec<_> = Dictionary::entries(&fields, &name)
        .map(Result::unwrap)
        .collect();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0], (key("a"), int_member(1)));
    assert_eq!(entries[1], (key("b"), int_member(2)));
    assert_eq!(entries[2], (key("a"), int_member(3)));
}

#[test]
fn entries_end_after_an_error() {
    let fields = section(&["a=1", "b=2 c=3", "d=4"]);
    let name = name();
    let mut entries = Dictionary::entries(&fields, &name);
    assert_eq!(entries.next(), Some(Ok((key("a"), int_member(1)))));
    assert_eq!(entries.next(), Some(Ok((key("b"), int_member(2)))));
    assert_eq!(
        entries.next(),
        Some(Err(FieldLinesError::Syntax {
            line: 1,
            source: SyntaxError::ExpectedComma { index: 4 }
        }))
    );
    assert_eq!(entries.next(), None);
}

#[test]
fn entries_report_an_empty_line_among_several() {
    let fields = section(&["a=1", "", "b=2"]);
    let name = name();
    let all: Vec<_> = Dictionary::entries(&fields, &name).collect();
    assert_eq!(all.len(), 2);
    assert!(all[0].is_ok());
    assert_eq!(all[1], Err(FieldLinesError::EmptyLine { line: 1 }));
}

#[test]
fn entries_of_one_empty_line_are_none() {
    let fields = section(&[""]);
    let name = name();
    assert_eq!(Dictionary::entries(&fields, &name).count(), 0);
}

#[test]
fn entries_of_a_missing_field_are_none() {
    let fields = FieldSection::new();
    let name = name();
    assert_eq!(Dictionary::entries(&fields, &name).count(), 0);
}
