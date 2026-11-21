use super::*;
use crate::field::FieldValue;
use alloc::vec;
use bytes::Bytes;

fn name(text: &'static str) -> FieldName {
    FieldName::from_static(text).unwrap()
}

fn value(text: &'static str) -> FieldValue {
    FieldValue::new(Bytes::from_static(text.as_bytes())).unwrap()
}

fn section(lines: &[(&'static str, &'static str)]) -> FieldSection {
    lines.iter().map(|&(n, v)| (name(n), value(v))).collect()
}

fn names(fields: &FieldSection) -> Vec<Vec<u8>> {
    fields.iter().map(|(n, _)| n.as_bytes().to_vec()).collect()
}

fn names_of(expected: &[&str]) -> Vec<Vec<u8>> {
    expected.iter().map(|n| n.as_bytes().to_vec()).collect()
}

#[test]
fn is_connection_specific_is_true_for_the_six_names() {
    // RFC 9110 §7.6.1.
    for n in [
        "connection",
        "proxy-connection",
        "keep-alive",
        "te",
        "transfer-encoding",
        "upgrade",
    ] {
        assert!(is_connection_specific(&name(n)), "{n}");
    }
}

#[test]
fn is_connection_specific_is_case_insensitive_through_field_name() {
    assert!(is_connection_specific(&name("Keep-Alive")));
    assert!(is_connection_specific(&name("TRANSFER-ENCODING")));
}

#[test]
fn is_connection_specific_is_false_for_other_names() {
    for n in [
        "host",
        "content-length",
        "set-cookie",
        "x-foo",
        "connections",
        "trailer",
    ] {
        assert!(!is_connection_specific(&name(n)), "{n}");
    }
}

fn collect(fields: &FieldSection) -> Vec<Result<Token, ConnectionOptionError>> {
    options(fields).collect()
}

fn token(text: &'static str) -> Token {
    Token::from_static(text).unwrap()
}

#[test]
fn options_is_empty_without_connection_lines() {
    assert!(collect(&FieldSection::new()).is_empty());
    assert!(collect(&section(&[("host", "close")])).is_empty());
}

#[test]
fn options_reads_connection_close() {
    // RFC 9110 §7.6.1.
    assert_eq!(
        collect(&section(&[("connection", "close")])),
        vec![Ok(token("close"))]
    );
}

#[test]
fn options_reads_a_list_and_skips_empty_members() {
    let fields = section(&[("connection", "keep-alive, , upgrade,")]);
    assert_eq!(
        collect(&fields),
        vec![Ok(token("keep-alive")), Ok(token("upgrade"))]
    );
}

#[test]
fn options_reads_over_two_lines_in_order() {
    let fields = section(&[
        ("connection", "a"),
        ("host", "ignored"),
        ("Connection", "b, c"),
    ]);
    assert_eq!(
        collect(&fields),
        vec![Ok(token("a")), Ok(token("b")), Ok(token("c"))]
    );
}

#[test]
fn options_keeps_the_case_of_the_value() {
    let fields = section(&[("connection", "Close, Keep-ALIVE")]);
    let got = collect(&fields);
    assert_eq!(got[0].as_ref().unwrap().as_bytes(), b"Close");
    assert_eq!(got[1].as_ref().unwrap().as_bytes(), b"Keep-ALIVE");
    assert!(got[0].as_ref().unwrap().eq_ignore_ascii_case(b"close"));
}

#[test]
fn options_reports_a_member_that_is_not_a_token() {
    let fields = section(&[("connection", "ok, a/b")]);
    let got = collect(&fields);
    assert_eq!(got.len(), 2);
    assert_eq!(got[0], Ok(token("ok")));
    let error = got[1].clone().unwrap_err().character_error();
    assert_eq!((error.index(), error.byte()), (1, b'/'));
}

#[test]
fn options_reports_a_quoted_member() {
    let fields = section(&[("connection", "\"x\"")]);
    let got = collect(&fields);
    assert_eq!(got.len(), 1);
    let error = got[0].clone().unwrap_err().character_error();
    assert_eq!((error.index(), error.byte()), (0, b'"'));
}

#[test]
fn options_reports_a_member_with_inner_whitespace() {
    let fields = section(&[("connection", "a b")]);
    let got = collect(&fields);
    let error = got[0].clone().unwrap_err().character_error();
    assert_eq!((error.index(), error.byte()), (1, b' '));
}

#[test]
fn options_shares_the_bytes_of_the_field_value() {
    let fields = section(&[("connection", "close")]);
    let (_, line) = fields.iter().next().unwrap();
    let got = collect(&fields);
    let option = got[0].as_ref().unwrap();
    assert_eq!(option.as_bytes().as_ptr(), line.as_bytes().as_ptr());
}

#[test]
fn remove_connection_specific_removes_connection_and_the_fixed_five() {
    let mut fields = section(&[
        ("connection", "close"),
        ("proxy-connection", "keep-alive"),
        ("keep-alive", "timeout=5"),
        ("te", "trailers"),
        ("transfer-encoding", "chunked"),
        ("upgrade", "websocket"),
        ("host", "example.com"),
    ]);
    remove_connection_specific(&mut fields);
    assert_eq!(names(&fields), names_of(&["host"]));
}

#[test]
fn remove_connection_specific_removes_the_fields_connection_lists() {
    let mut fields = section(&[
        ("host", "example.com"),
        ("connection", "x-foo, x-bar"),
        ("x-foo", "1"),
        ("accept", "*/*"),
        ("x-bar", "2"),
        ("x-foo", "3"),
    ]);
    remove_connection_specific(&mut fields);
    assert_eq!(names(&fields), names_of(&["host", "accept"]));
}

#[test]
fn remove_connection_specific_matches_options_ignoring_case() {
    let mut fields = section(&[("Connection", "X-Foo"), ("x-foo", "1"), ("x-fooo", "2")]);
    remove_connection_specific(&mut fields);
    assert_eq!(names(&fields), names_of(&["x-fooo"]));
}

#[test]
fn remove_connection_specific_reads_options_from_every_connection_line() {
    let mut fields = section(&[
        ("connection", "a"),
        ("a", "1"),
        ("b", "2"),
        ("connection", "b"),
        ("c", "3"),
    ]);
    remove_connection_specific(&mut fields);
    assert_eq!(names(&fields), names_of(&["c"]));
}

#[test]
fn remove_connection_specific_keeps_order_of_the_rest() {
    let mut fields = section(&[
        ("z", "1"),
        ("connection", "close"),
        ("a", "2"),
        ("z", "3"),
        ("m", "4"),
    ]);
    remove_connection_specific(&mut fields);
    assert_eq!(names(&fields), names_of(&["z", "a", "z", "m"]));
    let values: Vec<&[u8]> = fields.iter().map(|(_, v)| v.as_bytes()).collect();
    assert_eq!(values, vec![&b"1"[..], b"2", b"3", b"4"]);
}

#[test]
fn remove_connection_specific_removes_nothing_for_a_non_token_member() {
    let mut fields = section(&[("connection", "\"x\""), ("x", "1"), ("y", "2")]);
    remove_connection_specific(&mut fields);
    // Only Connection itself goes.
    assert_eq!(names(&fields), names_of(&["x", "y"]));

    let mut fields = section(&[("connection", "a/b"), ("a", "1"), ("b", "2")]);
    remove_connection_specific(&mut fields);
    assert_eq!(names(&fields), names_of(&["a", "b"]));
}

#[test]
fn remove_connection_specific_still_uses_the_token_members_beside_a_bad_one() {
    let mut fields = section(&[("connection", "a/b, y"), ("x", "1"), ("y", "2")]);
    remove_connection_specific(&mut fields);
    assert_eq!(names(&fields), names_of(&["x"]));
}

#[test]
fn remove_connection_specific_leaves_a_section_without_connection_alone() {
    let mut fields = section(&[("host", "a"), ("x", "1")]);
    let before = fields.clone();
    remove_connection_specific(&mut fields);
    assert_eq!(fields, before);
    let mut empty = FieldSection::new();
    remove_connection_specific(&mut empty);
    assert!(empty.is_empty());
}

#[test]
fn connection_option_error_says_what_is_not_a_token() {
    let fields = section(&[("connection", "a/b")]);
    let error = collect(&fields)[0].clone().unwrap_err();
    assert_eq!(
        alloc::format!("{error}"),
        "a connection option is not a token"
    );
    assert_eq!(
        alloc::format!("{}", error.character_error()),
        "byte 0x2f at 1 is not a tchar"
    );
}
