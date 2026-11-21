use super::*;
use alloc::vec;
use alloc::vec::Vec;

fn collect(value: &[u8]) -> Vec<&[u8]> {
    members(value).collect()
}

#[test]
fn members_of_an_empty_value_is_empty() {
    assert!(collect(b"").is_empty());
}

#[test]
fn members_of_a_single_member() {
    assert_eq!(collect(b"a"), vec![&b"a"[..]]);
}

#[test]
fn members_splits_at_commas() {
    assert_eq!(collect(b"a,b,c"), vec![&b"a"[..], b"b", b"c"]);
}

#[test]
fn members_trims_spaces_and_htab() {
    assert_eq!(collect(b" a ,\tb\t, c d "), vec![&b"a"[..], b"b", b"c d"]);
}

#[test]
fn members_skips_empty_members() {
    // RFC 9110 §5.6.1.2.
    assert_eq!(collect(b",,a,, ,\t,b,"), vec![&b"a"[..], b"b"]);
    assert!(collect(b",").is_empty());
    assert!(collect(b", ,").is_empty());
    assert!(collect(b" ").is_empty());
}

#[test]
fn members_does_not_split_inside_a_quoted_string() {
    // RFC 9110 §5.6.4.
    assert_eq!(collect(b"a=\"x,y\", b"), vec![&b"a=\"x,y\""[..], b"b"]);
}

#[test]
fn members_honours_a_quoted_pair() {
    // The escaped quote does not end the quoted string.
    assert_eq!(collect(b"\"a\\\",b\",c"), vec![&b"\"a\\\",b\""[..], b"c"]);
}

#[test]
fn members_an_unterminated_quote_runs_to_the_end() {
    assert_eq!(collect(b"a, \"b,c"), vec![&b"a"[..], b"\"b,c"]);
}

#[test]
fn members_a_backslash_outside_quotes_is_plain() {
    assert_eq!(collect(b"a\\,b"), vec![&b"a\\"[..], b"b"]);
}

#[test]
fn members_keeps_obs_text() {
    assert_eq!(collect(b"\xe9, \xff"), vec![&b"\xe9"[..], b"\xff"]);
}

#[test]
fn members_is_cloneable_and_resumable() {
    let mut iter = members(b"a, b, c");
    assert_eq!(iter.next(), Some(&b"a"[..]));
    let rest: Vec<_> = iter.clone().collect();
    assert_eq!(rest, vec![&b"b"[..], b"c"]);
    assert_eq!(iter.next(), Some(&b"b"[..]));
    assert_eq!(iter.next(), Some(&b"c"[..]));
    assert_eq!(iter.next(), None);
    assert_eq!(iter.next(), None);
}

#[test]
fn members_reads_an_accept_encoding_example() {
    // RFC 9110 §12.5.3 style list with weights.
    assert_eq!(
        collect(b"gzip;q=1.0, identity; q=0.5, *;q=0"),
        vec![&b"gzip;q=1.0"[..], b"identity; q=0.5", b"*;q=0"]
    );
}
