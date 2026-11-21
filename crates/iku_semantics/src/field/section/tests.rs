use super::*;
use alloc::format;
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

fn pairs(fields: &FieldSection) -> Vec<(Vec<u8>, Vec<u8>)> {
    fields
        .iter()
        .map(|(n, v)| (n.as_bytes().to_vec(), v.as_bytes().to_vec()))
        .collect()
}

fn pair(n: &str, v: &str) -> (Vec<u8>, Vec<u8>) {
    (n.as_bytes().to_vec(), v.as_bytes().to_vec())
}

#[test]
fn new_section_is_empty() {
    let fields = FieldSection::new();
    assert!(fields.is_empty());
    assert_eq!(fields.len(), 0);
    assert_eq!(fields.iter().len(), 0);
    assert_eq!(fields, FieldSection::default());
}

#[test]
fn new_works_in_const_context() {
    const FIELDS: FieldSection = FieldSection::new();
    assert!(FIELDS.is_empty());
}

#[test]
fn with_capacity_makes_an_empty_section() {
    let fields = FieldSection::with_capacity(16);
    assert!(fields.is_empty());
    assert_eq!(fields, FieldSection::new());
}

#[test]
fn append_keeps_order_and_repeated_names() {
    let mut fields = FieldSection::new();
    fields.append(name("b"), value("1"));
    fields.append(name("a"), value("2"));
    fields.append(name("b"), value("3"));
    assert_eq!(fields.len(), 3);
    assert!(!fields.is_empty());
    assert_eq!(
        pairs(&fields),
        vec![pair("b", "1"), pair("a", "2"), pair("b", "3")]
    );
}

#[test]
fn iter_is_exact_size() {
    let fields = section(&[("a", "1"), ("b", "2")]);
    let mut iter = fields.iter();
    assert_eq!(iter.len(), 2);
    iter.next();
    assert_eq!(iter.len(), 1);
}

#[test]
fn contains_looks_for_a_name() {
    let fields = section(&[("a", "1")]);
    assert!(fields.contains(&name("a")));
    assert!(fields.contains(&name("A")));
    assert!(!fields.contains(&name("b")));
    assert!(!FieldSection::new().contains(&name("a")));
}

#[test]
fn get_all_gives_the_values_of_a_name_in_order() {
    let fields = section(&[("a", "1"), ("b", "x"), ("a", "2"), ("a", "")]);
    let a = name("a");
    let values: Vec<&[u8]> = fields.get_all(&a).map(FieldValue::as_bytes).collect();
    assert_eq!(values, vec![&b"1"[..], b"2", b""]);
    let missing = name("c");
    assert_eq!(fields.get_all(&missing).count(), 0);
}

#[test]
fn list_members_reads_across_lines() {
    let fields = section(&[
        ("accept", "a, b"),
        ("x", "no"),
        ("Accept", ",c,"),
        ("accept", ""),
    ]);
    let accept = name("accept");
    let members: Vec<&[u8]> = fields.list_members(&accept).collect();
    assert_eq!(members, vec![&b"a"[..], b"b", b"c"]);
}

#[test]
fn list_members_does_not_split_a_quoted_comma() {
    let fields = section(&[("x", "a=\"1,2\""), ("x", "b")]);
    let x = name("x");
    let members: Vec<&[u8]> = fields.list_members(&x).collect();
    assert_eq!(members, vec![&b"a=\"1,2\""[..], b"b"]);
}

#[test]
fn list_members_of_a_missing_name_is_empty() {
    let fields = section(&[("a", "1")]);
    let b = name("b");
    assert_eq!(fields.list_members(&b).count(), 0);
}

#[test]
fn combined_is_none_without_lines() {
    assert_eq!(FieldSection::new().combined(&name("a")), Ok(None));
    assert_eq!(section(&[("b", "1")]).combined(&name("a")), Ok(None));
}

#[test]
fn combined_of_a_single_line_is_the_value() {
    let fields = section(&[("a", "x, y")]);
    assert_eq!(fields.combined(&name("a")), Ok(Some(value("x, y"))));
}

#[test]
fn combined_of_a_single_line_is_not_copied() {
    let fields = section(&[("a", "x, y")]);
    let original = fields
        .get_all(&name("a"))
        .next()
        .unwrap()
        .as_bytes()
        .as_ptr();
    let combined = fields.combined(&name("a")).unwrap().unwrap();
    assert_eq!(combined.as_bytes().as_ptr(), original);
}

#[test]
fn combined_joins_lines_with_a_bare_comma() {
    // RFC 9110 §5.3.
    let fields = section(&[("a", "1"), ("b", "x"), ("a", "2"), ("a", "3")]);
    assert_eq!(fields.combined(&name("a")), Ok(Some(value("1,2,3"))));
}

#[test]
fn combined_keeps_inner_whitespace_of_each_value() {
    let fields = section(&[("a", "x y"), ("a", "z\tw")]);
    assert_eq!(fields.combined(&name("a")), Ok(Some(value("x y,z\tw"))));
}

#[test]
fn combined_with_an_empty_last_line_has_no_trailing_whitespace() {
    let fields = section(&[("a", "1"), ("a", "")]);
    assert_eq!(fields.combined(&name("a")), Ok(Some(value("1,"))));
}

#[test]
fn combined_with_an_empty_first_line() {
    let fields = section(&[("a", ""), ("a", "1")]);
    assert_eq!(fields.combined(&name("a")), Ok(Some(value(",1"))));
}

#[test]
fn combined_with_only_empty_lines() {
    let fields = section(&[("a", ""), ("a", "")]);
    assert_eq!(fields.combined(&name("a")), Ok(Some(value(","))));
}

#[test]
fn combined_refuses_set_cookie() {
    // RFC 9110 §5.3 and RFC 6265.
    let fields = section(&[("set-cookie", "a=1"), ("set-cookie", "b=2")]);
    assert_eq!(
        fields.combined(&FieldName::SET_COOKIE),
        Err(SetCookieNotCombinableError)
    );
    // Even without lines or with a single one.
    assert_eq!(
        FieldSection::new().combined(&FieldName::SET_COOKIE),
        Err(SetCookieNotCombinableError)
    );
    assert_eq!(
        section(&[("set-cookie", "a=1")]).combined(&FieldName::SET_COOKIE),
        Err(SetCookieNotCombinableError)
    );
}

#[test]
fn remove_all_returns_how_many_lines_went() {
    let mut fields = section(&[("a", "1"), ("b", "2"), ("a", "3")]);
    assert_eq!(fields.remove_all(&name("a")), 2);
    assert_eq!(pairs(&fields), vec![pair("b", "2")]);
    assert_eq!(fields.remove_all(&name("a")), 0);
    assert_eq!(fields.remove_all(&name("b")), 1);
    assert!(fields.is_empty());
    assert_eq!(fields.remove_all(&name("b")), 0);
}

#[test]
fn retain_keeps_matching_lines_in_order() {
    let mut fields = section(&[("a", "1"), ("b", "2"), ("a", "3"), ("c", "4")]);
    fields.retain(|n, v| *n != name("b") && v.as_bytes() != b"4");
    assert_eq!(pairs(&fields), vec![pair("a", "1"), pair("a", "3")]);
}

#[test]
fn retain_calls_the_closure_once_per_line_in_order() {
    let mut fields = section(&[("a", "1"), ("b", "2"), ("c", "3")]);
    let mut seen = Vec::new();
    fields.retain(|n, _| {
        seen.push(n.as_bytes().to_vec());
        true
    });
    assert_eq!(seen, vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
    assert_eq!(fields.len(), 3);
}

#[test]
fn retain_can_drop_everything() {
    let mut fields = section(&[("a", "1")]);
    fields.retain(|_, _| false);
    assert!(fields.is_empty());
}

#[test]
fn from_iterator_into_iterator_round_trip() {
    let lines = vec![
        (name("b"), value("1")),
        (name("a"), value("2")),
        (name("b"), value("3")),
    ];
    let fields: FieldSection = lines.clone().into_iter().collect();
    assert_eq!(fields.into_iter().collect::<Vec<_>>(), lines);
}

#[test]
fn extend_appends_after_existing_lines() {
    let mut fields = section(&[("a", "1")]);
    fields.extend(vec![(name("b"), value("2")), (name("a"), value("3"))]);
    assert_eq!(
        pairs(&fields),
        vec![pair("a", "1"), pair("b", "2"), pair("a", "3")]
    );
}

#[test]
fn section_equality_depends_on_order() {
    assert_ne!(
        section(&[("a", "1"), ("b", "2")]),
        section(&[("b", "2"), ("a", "1")])
    );
    assert_eq!(section(&[("a", "1")]), section(&[("A", "1")]));
}

#[test]
fn set_cookie_error_displays() {
    assert_eq!(
        format!("{SetCookieNotCombinableError}"),
        "the lines of set-cookie cannot be combined"
    );
}
