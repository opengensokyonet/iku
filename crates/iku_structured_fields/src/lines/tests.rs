use super::*;
use crate::bare_item::BareItem;
use crate::list::ListMember;
use alloc::string::ToString;
use alloc::vec::Vec;
use bytes::Bytes;
use core::error::Error;
use iku_semantics::field::{FieldName, FieldSection};

fn fv(text: &'static str) -> FieldValue {
    FieldValue::from_static(text).unwrap()
}

fn name() -> FieldName {
    FieldName::from_static("example-list").unwrap()
}

fn section(lines: &[&'static str]) -> FieldSection {
    let mut fields = FieldSection::new();
    for line in lines {
        fields.append(name(), fv(line));
    }
    fields
}

fn members(fields: &FieldSection) -> Vec<Result<ListMember, FieldLinesError>> {
    let name = name();
    LineMembers::new(fields.get_all(&name), ListMember::parse).collect()
}

fn tokens(fields: &FieldSection) -> Vec<alloc::string::String> {
    members(fields)
        .into_iter()
        .map(|member| match member.unwrap() {
            ListMember::Item(item) => match item.bare_item() {
                BareItem::Token(token) => token.to_string(),
                other => panic!("{other:?}"),
            },
            ListMember::InnerList(_) => panic!("inner list"),
        })
        .collect()
}

fn next(
    input: &'static str,
    parser_position: usize,
    after_member: &mut bool,
) -> (Result<Option<ListMember>, SyntaxError>, usize) {
    let mut parser = Parser::new(Bytes::from_static(input.as_bytes()));
    parser.skip(parser_position);
    let result = next_member(&mut parser, after_member, ListMember::parse);
    (result, parser.index())
}

#[test]
fn next_member_reads_the_first_member_after_spaces() {
    let mut after = false;
    let (result, _) = next("   a", 0, &mut after);
    assert!(matches!(result, Ok(Some(ListMember::Item(_)))));
    assert!(after);
}

#[test]
fn next_member_on_an_empty_or_blank_line_is_none() {
    for input in ["", "   "] {
        let mut after = false;
        let (result, _) = next(input, 0, &mut after);
        assert_eq!(result.map(|member| member.is_none()), Ok(true));
        assert!(!after);
    }
}

#[test]
fn next_member_needs_a_comma_after_a_member() {
    let mut after = true;
    assert_eq!(next(" , a", 0, &mut after).0.map(|m| m.is_some()), Ok(true));
    let mut after = true;
    assert_eq!(next(",a", 0, &mut after).0.map(|m| m.is_some()), Ok(true));
    let mut after = true;
    assert_eq!(
        next("\t,\ta", 0, &mut after).0.map(|m| m.is_some()),
        Ok(true)
    );
    let mut after = true;
    assert_eq!(
        next("x", 0, &mut after).0.map(|m| m.is_some()),
        Err(SyntaxError::ExpectedComma { index: 0 })
    );
    let mut after = true;
    assert_eq!(
        next("  x", 0, &mut after).0.map(|m| m.is_some()),
        Err(SyntaxError::ExpectedComma { index: 2 })
    );
}

#[test]
fn next_member_after_the_last_member_is_none() {
    for input in ["", "  ", "\t \t"] {
        let mut after = true;
        let (result, _) = next(input, 0, &mut after);
        assert_eq!(result.map(|member| member.is_none()), Ok(true));
    }
}

#[test]
fn a_comma_at_the_end_fails() {
    let mut after = true;
    assert_eq!(
        next(",", 0, &mut after).0.map(|m| m.is_some()),
        Err(SyntaxError::TrailingComma { index: 1 })
    );
    let mut after = true;
    assert_eq!(
        next(", \t", 0, &mut after).0.map(|m| m.is_some()),
        Err(SyntaxError::TrailingComma { index: 3 })
    );
}

#[test]
fn a_tab_is_not_skipped_before_the_first_member() {
    let mut after = false;
    assert_eq!(
        next("\ta", 0, &mut after).0.map(|m| m.is_some()),
        Err(SyntaxError::InvalidBareItemStart { index: 0 })
    );
}

#[test]
fn members_are_given_across_lines_in_order() {
    assert_eq!(
        tokens(&section(&["a, b", "c", "d, e, f"])),
        ["a", "b", "c", "d", "e", "f"]
    );
}

#[test]
fn other_fields_are_ignored() {
    let mut fields = FieldSection::new();
    fields.append(FieldName::from_static("other").unwrap(), fv("!bad"));
    fields.append(name(), fv("a"));
    fields.append(FieldName::from_static("other").unwrap(), fv(""));
    fields.append(name(), fv("b"));
    assert_eq!(tokens(&fields), ["a", "b"]);
}

#[test]
fn no_line_gives_no_members() {
    assert!(members(&FieldSection::new()).is_empty());
}

#[test]
fn one_empty_line_gives_no_members() {
    assert!(members(&section(&[""])).is_empty());
}

#[test]
fn an_empty_line_among_several_fails() {
    for (lines, line) in [
        (&["", "a"][..], 0),
        (&["a", "", "b"][..], 1),
        (&["a", ""][..], 1),
        (&["", ""][..], 0),
        (&["a", "b", ""][..], 2),
    ] {
        let result = members(&section(lines));
        let last = result.last().unwrap();
        assert_eq!(last, &Err(FieldLinesError::EmptyLine { line }), "{lines:?}");
    }
}

#[test]
fn members_before_an_empty_line_are_still_given() {
    let result = members(&section(&["a, b", "", "c"]));
    assert_eq!(result.len(), 3);
    assert!(result[0].is_ok() && result[1].is_ok());
    assert_eq!(result[2], Err(FieldLinesError::EmptyLine { line: 1 }));
}

#[test]
fn line_numbers_count_only_the_lines_of_the_field() {
    let mut fields = FieldSection::new();
    fields.append(FieldName::from_static("other").unwrap(), fv("x"));
    fields.append(name(), fv("a"));
    fields.append(FieldName::from_static("other").unwrap(), fv("y"));
    fields.append(name(), fv(""));
    let result = members(&fields);
    assert_eq!(
        result.last().unwrap(),
        &Err(FieldLinesError::EmptyLine { line: 1 })
    );
}

#[test]
fn the_iterator_ends_after_an_error() {
    let fields = section(&["a", "!", "b"]);
    let name = name();
    let mut iterator = LineMembers::new(fields.get_all(&name), ListMember::parse);
    assert!(matches!(iterator.next(), Some(Ok(_))));
    assert_eq!(
        iterator.next().unwrap().map(|_| ()),
        Err(FieldLinesError::Syntax {
            line: 1,
            source: SyntaxError::InvalidBareItemStart { index: 0 }
        })
    );
    assert!(iterator.next().is_none());
    assert!(iterator.next().is_none());

    let fields = section(&["a", "", "b"]);
    let mut iterator = LineMembers::new(fields.get_all(&name), ListMember::parse);
    assert!(matches!(iterator.next(), Some(Ok(_))));
    assert!(matches!(
        iterator.next(),
        Some(Err(FieldLinesError::EmptyLine { line: 1 }))
    ));
    assert!(iterator.next().is_none());
}

#[test]
fn syntax_errors_name_the_line_and_the_index() {
    let result = members(&section(&["a, b", "c d"]));
    assert_eq!(
        result.last().unwrap(),
        &Err(FieldLinesError::Syntax {
            line: 1,
            source: SyntaxError::ExpectedComma { index: 2 }
        })
    );
    let result = members(&section(&["a,"]));
    assert_eq!(
        result.last().unwrap(),
        &Err(FieldLinesError::Syntax {
            line: 0,
            source: SyntaxError::TrailingComma { index: 2 }
        })
    );
}

#[test]
fn a_line_that_starts_with_a_comma_fails() {
    let result = members(&section(&["a", ", b"]));
    assert_eq!(
        result.last().unwrap(),
        &Err(FieldLinesError::Syntax {
            line: 1,
            source: SyntaxError::InvalidBareItemStart { index: 0 }
        })
    );
}

#[test]
fn a_member_cannot_span_two_lines() {
    let result = members(&section(&["(a", "b)"]));
    assert_eq!(
        result[0],
        Err(FieldLinesError::Syntax {
            line: 0,
            source: SyntaxError::UnclosedInnerList { index: 0 }
        })
    );
    assert_eq!(result.len(), 1);
}

#[test]
fn each_line_starts_afresh() {
    // The second line needs no comma before its first member.
    let result = members(&section(&["a", "b"]));
    assert_eq!(result.len(), 2);
    assert!(result.iter().all(Result::is_ok));
    assert!(result.iter().all(|r| matches!(r, Ok(ListMember::Item(_)))));
}

#[test]
fn error_display_and_source() {
    let syntax = FieldLinesError::Syntax {
        line: 3,
        source: SyntaxError::ExpectedComma { index: 4 },
    };
    assert_eq!(syntax.to_string(), "field line 3 is not well-formed");
    assert_eq!(
        syntax.source().unwrap().to_string(),
        "expected a comma at byte 4"
    );
    let empty = FieldLinesError::EmptyLine { line: 2 };
    assert_eq!(empty.to_string(), "field line 2 is empty, one of several");
    assert!(empty.source().is_none());
}
