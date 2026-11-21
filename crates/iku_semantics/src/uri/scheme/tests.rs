use super::*;
use alloc::format;

fn new(text: &str) -> Result<Scheme, SchemeError> {
    Scheme::new(Bytes::copy_from_slice(text.as_bytes()))
}

fn invalid(error: SchemeError) -> (usize, u8) {
    match error {
        SchemeError::InvalidCharacter { source } => (source.index(), source.byte()),
        other => panic!("expected InvalidCharacter, got {other:?}"),
    }
}

#[test]
fn scheme_constants() {
    assert_eq!(Scheme::HTTP.as_bytes(), b"http");
    assert_eq!(Scheme::HTTPS.as_bytes(), b"https");
    assert_eq!(new("http").unwrap(), Scheme::HTTP);
    assert_eq!(Scheme::from_static("https").unwrap(), Scheme::HTTPS);
}

#[test]
fn scheme_accepts_a_single_letter() {
    assert_eq!(new("a").unwrap().as_bytes(), b"a");
}

#[test]
fn scheme_accepts_letters_digits_plus_minus_dot() {
    // RFC 3986 §3.1.
    assert_eq!(new("a1+b-c.d").unwrap().as_bytes(), b"a1+b-c.d");
    assert_eq!(new("coap+tcp").unwrap().as_bytes(), b"coap+tcp");
}

#[test]
fn scheme_lowercases() {
    assert_eq!(new("HTTP").unwrap(), Scheme::HTTP);
    assert_eq!(new("HtTpS").unwrap(), Scheme::HTTPS);
    assert_eq!(new("A+B").unwrap().as_bytes(), b"a+b");
}

#[test]
fn scheme_without_uppercase_shares_the_bytes() {
    let input = Bytes::from_static(b"urn");
    let scheme = Scheme::new(input.clone()).unwrap();
    assert_eq!(scheme.as_bytes().as_ptr(), input.as_ptr());
}

#[test]
fn scheme_rejects_empty_input() {
    assert_eq!(new(""), Err(SchemeError::Empty));
    assert_eq!(Scheme::from_static(""), Err(SchemeError::Empty));
}

#[test]
fn scheme_rejects_a_first_byte_that_is_not_a_letter() {
    for (text, byte) in [
        ("1http", b'1'),
        ("+a", b'+'),
        ("-a", b'-'),
        (".a", b'.'),
        (":", b':'),
    ] {
        assert_eq!(invalid(new(text).unwrap_err()), (0, byte), "{text}");
    }
}

#[test]
fn scheme_rejects_a_later_byte_it_cannot_hold() {
    assert_eq!(invalid(new("ht tp").unwrap_err()), (2, b' '));
    assert_eq!(invalid(new("http:").unwrap_err()), (4, b':'));
    assert_eq!(invalid(new("a_b").unwrap_err()), (1, b'_'));
    assert_eq!(
        invalid(Scheme::new(Bytes::from_static(b"a\xe9")).unwrap_err()),
        (1, 0xe9)
    );
}

#[test]
fn scheme_is_http_or_https() {
    assert!(Scheme::HTTP.is_http_or_https());
    assert!(Scheme::HTTPS.is_http_or_https());
    assert!(new("HTTPS").unwrap().is_http_or_https());
    assert!(!new("ftp").unwrap().is_http_or_https());
    assert!(!new("urn").unwrap().is_http_or_https());
    assert!(!new("httpx").unwrap().is_http_or_https());
    assert!(!new("ws").unwrap().is_http_or_https());
}

#[test]
fn scheme_display_and_debug() {
    assert_eq!(format!("{}", Scheme::HTTP), "http");
    assert_eq!(format!("{:?}", Scheme::HTTPS), "Scheme(https)");
}

#[test]
fn scheme_errors_display() {
    assert_eq!(format!("{}", SchemeError::Empty), "a scheme is empty");
    let SchemeError::InvalidCharacter { source } = new("1a").unwrap_err() else {
        panic!();
    };
    assert_eq!(
        format!("{source}"),
        "byte 0x31 at 0 cannot be in a scheme there"
    );
}
