use super::*;
use alloc::format;

fn new(text: &str) -> Result<PathAndQuery, PathAndQueryError> {
    PathAndQuery::new(Bytes::copy_from_slice(text.as_bytes()))
}

fn invalid(error: PathAndQueryError) -> (usize, u8) {
    match error {
        PathAndQueryError::InvalidCharacter { source } => (source.index(), source.byte()),
        other => panic!("expected InvalidCharacter, got {other:?}"),
    }
}

#[test]
fn root_is_a_slash() {
    assert_eq!(PathAndQuery::ROOT.as_bytes(), b"/");
    assert_eq!(PathAndQuery::ROOT.path(), b"/");
    assert_eq!(PathAndQuery::ROOT.query(), None);
    assert_eq!(new("/").unwrap(), PathAndQuery::ROOT);
}

#[test]
fn path_without_a_query() {
    let p = new("/a/b/c").unwrap();
    assert_eq!(p.path(), b"/a/b/c");
    assert_eq!(p.query(), None);
    assert_eq!(p.as_bytes(), b"/a/b/c");
}

#[test]
fn path_with_a_query() {
    let p = new("/where?q=now").unwrap();
    assert_eq!(p.path(), b"/where");
    assert_eq!(p.query(), Some(&b"q=now"[..]));
    assert_eq!(p.as_bytes(), b"/where?q=now");
}

#[test]
fn empty_query_is_a_query() {
    let p = new("/a?").unwrap();
    assert_eq!(p.path(), b"/a");
    assert_eq!(p.query(), Some(&b""[..]));
}

#[test]
fn query_may_hold_question_marks_and_slashes() {
    // RFC 3986 §3.4: query = *( pchar / "/" / "?" ).
    let p = new("/a?b?c/d").unwrap();
    assert_eq!(p.path(), b"/a");
    assert_eq!(p.query(), Some(&b"b?c/d"[..]));
}

#[test]
fn path_may_hold_pchar() {
    // RFC 3986 §3.3: unreserved, sub-delims, ":" and "@".
    assert!(new("/aZ09-._~!$&'()*+,;=:@/").is_ok());
}

#[test]
fn path_accepts_pct_encoded() {
    let p = new("/a%2Fb%7e?x=%20").unwrap();
    assert_eq!(p.path(), b"/a%2Fb%7e");
    assert_eq!(p.query(), Some(&b"x=%20"[..]));
    // At the very end.
    assert!(new("/%41").is_ok());
}

#[test]
fn path_rejects_empty_and_relative_input() {
    assert_eq!(new(""), Err(PathAndQueryError::NotAbsolute));
    assert_eq!(new("a"), Err(PathAndQueryError::NotAbsolute));
    assert_eq!(new("?a"), Err(PathAndQueryError::NotAbsolute));
    assert_eq!(new("*"), Err(PathAndQueryError::NotAbsolute));
    assert_eq!(
        PathAndQuery::from_static("http://a/"),
        Err(PathAndQueryError::NotAbsolute)
    );
}

#[test]
fn path_rejects_a_fragment() {
    assert_eq!(invalid(new("/a#b").unwrap_err()), (2, b'#'));
    assert_eq!(invalid(new("/a?b#").unwrap_err()), (4, b'#'));
}

#[test]
fn path_rejects_bytes_it_cannot_hold() {
    for (text, index, byte) in [
        ("/a b", 2, b' '),
        ("/a\tb", 2, b'\t'),
        ("/a\"", 2, b'"'),
        ("/a<", 2, b'<'),
        ("/a>", 2, b'>'),
        ("/a[", 2, b'['),
        ("/a]", 2, b']'),
        ("/a\\", 2, b'\\'),
        ("/a^", 2, b'^'),
        ("/a`", 2, b'`'),
        ("/a{", 2, b'{'),
        ("/a|", 2, b'|'),
        ("/a}", 2, b'}'),
        ("/a\x7f", 2, 0x7f),
        ("/a?b c", 4, b' '),
    ] {
        assert_eq!(invalid(new(text).unwrap_err()), (index, byte), "{text:?}");
    }
    assert_eq!(
        invalid(PathAndQuery::new(Bytes::from_static(b"/\xe9")).unwrap_err()),
        (1, 0xe9)
    );
    assert_eq!(
        invalid(PathAndQuery::new(Bytes::from_static(b"/\x00")).unwrap_err()),
        (1, 0)
    );
}

#[test]
fn path_rejects_a_bad_percent_encoding() {
    for (text, index) in [
        ("/%", 1),
        ("/%2", 1),
        ("/%zz", 1),
        ("/%2g", 1),
        ("/a?%", 3),
        ("/a?b%1", 4),
    ] {
        assert_eq!(
            new(text),
            Err(PathAndQueryError::InvalidPercentEncoding { index }),
            "{text:?}"
        );
    }
}

#[test]
fn path_from_static_and_into_bytes() {
    let p = PathAndQuery::from_static("/a?b").unwrap();
    assert_eq!(p.into_bytes(), Bytes::from_static(b"/a?b"));
}

#[test]
fn path_into_bytes_shares_the_input() {
    let input = Bytes::from_static(b"/abc");
    let p = PathAndQuery::new(input.clone()).unwrap();
    assert_eq!(p.into_bytes().as_ptr(), input.as_ptr());
}

#[test]
fn path_display_and_debug() {
    let p = new("/a?b").unwrap();
    assert_eq!(format!("{p}"), "/a?b");
    assert_eq!(format!("{p:?}"), "PathAndQuery(/a?b)");
}

#[test]
fn path_errors_display() {
    assert_eq!(
        format!("{}", PathAndQueryError::NotAbsolute),
        "a path does not start with a slash"
    );
    assert_eq!(
        format!("{}", new("/%").unwrap_err()),
        "a percent sign at 1 is not followed by two hexadecimal digits"
    );
    let PathAndQueryError::InvalidCharacter { source } = new("/ ").unwrap_err() else {
        panic!();
    };
    assert_eq!(
        format!("{source}"),
        "byte 0x20 at 1 cannot be in a path or query"
    );
}
