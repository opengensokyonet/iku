use super::*;
use alloc::format;
use alloc::string::ToString;
use core::error::Error;

fn parse(input: &'static [u8]) -> (Result<Key, SyntaxError>, Bytes) {
    let mut parser = Parser::new(Bytes::from_static(input));
    let key = Key::parse(&mut parser);
    (key, Bytes::copy_from_slice(parser.rest()))
}

#[test]
fn new_accepts_keys() {
    for text in [
        "a",
        "*",
        "*a",
        "a1",
        "abc",
        "a_b-c.d*e1",
        "a0123456789",
        "*-_.*",
    ] {
        assert_eq!(Key::from_static(text).unwrap().as_str(), text);
    }
}

#[test]
fn empty_key_fails() {
    assert_eq!(Key::new(Bytes::new()), Err(KeyError::Empty));
}

#[test]
fn key_start_must_be_a_lowercase_letter_or_star() {
    for (text, byte) in [
        ("A", b'A'),
        ("1a", b'1'),
        ("_a", b'_'),
        ("-a", b'-'),
        (".a", b'.'),
        (":a", b':'),
        (" a", b' '),
        ("Za", b'Z'),
    ] {
        match Key::from_static(text) {
            Err(KeyError::InvalidStart { source }) => assert_eq!(source.byte(), byte, "{text}"),
            other => panic!("{text}: {other:?}"),
        }
    }
}

#[test]
fn key_character_must_be_lowercase_digit_or_punctuation() {
    for (text, index, byte) in [
        ("aB", 1, b'B'),
        ("abC", 2, b'C'),
        ("a:b", 1, b':'),
        ("a b", 1, b' '),
        ("a/b", 1, b'/'),
        ("a=b", 1, b'='),
        ("a;b", 1, b';'),
        ("a,b", 1, b','),
        ("ab~", 2, b'~'),
    ] {
        match Key::from_static(text) {
            Err(KeyError::InvalidCharacter { source }) => {
                assert_eq!((source.index(), source.byte()), (index, byte), "{text}");
            }
            other => panic!("{text}: {other:?}"),
        }
    }
    assert!(matches!(
        Key::new(Bytes::from_static(b"a\xc3")),
        Err(KeyError::InvalidCharacter { source }) if source.index() == 1 && source.byte() == 0xc3
    ));
}

#[test]
fn error_display_and_source() {
    assert_eq!(KeyError::Empty.to_string(), "a key is empty");
    assert!(KeyError::Empty.source().is_none());

    let start = Key::from_static("A").unwrap_err();
    assert_eq!(
        start.to_string(),
        "a key starts with a byte that cannot start it"
    );
    assert_eq!(
        start.source().unwrap().to_string(),
        "byte 0x41 is not a lowercase letter or `*`"
    );

    let character = Key::from_static("aB").unwrap_err();
    assert_eq!(
        character.to_string(),
        "a key holds a byte that cannot be in it"
    );
    assert_eq!(
        character.source().unwrap().to_string(),
        "byte 0x42 at 1 is not a lowercase letter, a digit, `_`, `-`, `.` or `*`"
    );
}

#[test]
fn accessors_and_formatting() {
    let key = Key::from_static("a-b").unwrap();
    assert_eq!(key.as_str(), "a-b");
    assert_eq!(key.to_string(), "a-b");
    assert_eq!(format!("{key:?}"), "Key(a-b)");
    assert_eq!(&key.clone().into_bytes()[..], b"a-b");
    let mut out = Vec::new();
    key.serialize(&mut out);
    assert_eq!(out, b"a-b");
    assert!(Key::from_static("a").unwrap() < Key::from_static("b").unwrap());
}

#[test]
fn parse_takes_the_longest_key() {
    let (key, rest) = parse(b"abc=1");
    assert_eq!(key.unwrap().as_str(), "abc");
    assert_eq!(&rest[..], b"=1");
    let (key, rest) = parse(b"*a_b.c-d9;x");
    assert_eq!(key.unwrap().as_str(), "*a_b.c-d9");
    assert_eq!(&rest[..], b";x");
    // An uppercase letter ends the key, which the caller then rejects.
    let (key, rest) = parse(b"abC");
    assert_eq!(key.unwrap().as_str(), "ab");
    assert_eq!(&rest[..], b"C");
}

#[test]
fn parse_rejects_a_bad_start() {
    assert_eq!(
        parse(b"Abc").0,
        Err(SyntaxError::InvalidKeyStart { index: 0 })
    );
    assert_eq!(
        parse(b"1").0,
        Err(SyntaxError::InvalidKeyStart { index: 0 })
    );
    assert_eq!(parse(b"").0, Err(SyntaxError::InvalidKeyStart { index: 0 }));
    assert_eq!(
        parse(b"=a").0,
        Err(SyntaxError::InvalidKeyStart { index: 0 })
    );
    assert_eq!(
        parse(b" a").0,
        Err(SyntaxError::InvalidKeyStart { index: 0 })
    );
}

#[test]
fn parse_reports_the_index_of_a_later_start() {
    let mut parser = Parser::new(Bytes::from_static(b"ab;C"));
    parser.skip(3);
    assert_eq!(
        Key::parse(&mut parser),
        Err(SyntaxError::InvalidKeyStart { index: 3 })
    );
}
