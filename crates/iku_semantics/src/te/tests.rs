use super::*;
use crate::field::FieldValue;
use bytes::Bytes;

fn section(lines: &[&'static str]) -> FieldSection {
    lines
        .iter()
        .map(|value| {
            (
                FieldName::TE,
                FieldValue::new(Bytes::from_static(value.as_bytes())).unwrap(),
            )
        })
        .collect()
}

#[test]
fn te_without_lines_has_no_other_member() {
    assert!(!has_member_other_than_trailers(&FieldSection::new()));
}

#[test]
fn te_ignores_other_fields() {
    let mut fields = FieldSection::new();
    fields.append(FieldName::HOST, FieldValue::from_static("deflate").unwrap());
    assert!(!has_member_other_than_trailers(&fields));
}

#[test]
fn te_trailers_alone_is_allowed() {
    // RFC 9113 §8.2.2.
    assert!(!has_member_other_than_trailers(&section(&["trailers"])));
}

#[test]
fn te_trailers_is_case_insensitive() {
    assert!(!has_member_other_than_trailers(&section(&["Trailers"])));
    assert!(!has_member_other_than_trailers(&section(&["TRAILERS"])));
}

#[test]
fn te_trailers_with_another_coding_is_not_allowed() {
    assert!(has_member_other_than_trailers(&section(&[
        "trailers, deflate"
    ])));
    assert!(has_member_other_than_trailers(&section(&[
        "deflate, trailers"
    ])));
}

#[test]
fn te_a_member_with_a_weight_is_another_member() {
    assert!(has_member_other_than_trailers(&section(&["gzip;q=0.5"])));
    assert!(has_member_other_than_trailers(&section(&["trailers;q=1"])));
}

#[test]
fn te_an_empty_value_has_no_member() {
    assert!(!has_member_other_than_trailers(&section(&[""])));
    assert!(!has_member_other_than_trailers(&section(&[","])));
}

#[test]
fn te_looks_at_every_line() {
    assert!(has_member_other_than_trailers(&section(&[
        "trailers", "gzip"
    ])));
    assert!(!has_member_other_than_trailers(&section(&[
        "trailers", "trailers"
    ])));
}

#[test]
fn te_tolerates_whitespace_around_members() {
    assert!(!has_member_other_than_trailers(&section(&[
        "trailers\t, trailers"
    ])));
}
