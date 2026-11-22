use super::*;
use alloc::format;
use alloc::string::ToString;
use core::error::Error;

fn token(text: &'static str) -> Token {
    Token::from_static(text).unwrap()
}

fn parse(input: &'static [u8]) -> (Token, Bytes) {
    let mut parser = Parser::new(Bytes::from_static(input));
    let token = Token::parse(&mut parser);
    (token, Bytes::copy_from_slice(parser.rest()))
}

#[test]
fn new_accepts_tokens() {
    for text in [
        "a",
        "A",
        "*",
        "*foo",
        "foo",
        "foo123/456",
        "text/html",
        "a:b",
        "a:",
        "a/",
        "Zz09",
        "a!#$%&'*+-.^_`|~",
        "a!#$%&'*+-.^_`|~:/",
        "a*b",
        "a%41",
    ] {
        assert_eq!(token(text).as_str(), text);
    }
}

#[test]
fn token_may_hold_colon_and_slash() {
    assert_eq!(token("foo:bar/baz").as_str(), "foo:bar/baz");
}

#[test]
fn empty_token_fails() {
    assert_eq!(Token::new(Bytes::new()), Err(TokenError::Empty));
}

#[test]
fn token_start_must_be_a_letter_or_star() {
    for (text, byte) in [
        ("1a", b'1'),
        ("-a", b'-'),
        (":a", b':'),
        ("/a", b'/'),
        ("_a", b'_'),
        (".a", b'.'),
        ("%a", b'%'),
        (" a", b' '),
        ("\"a", b'"'),
        ("~a", b'~'),
    ] {
        match Token::from_static(text) {
            Err(TokenError::InvalidStart { source }) => assert_eq!(source.byte(), byte, "{text}"),
            other => panic!("{text}: {other:?}"),
        }
    }
    assert!(matches!(
        Token::new(Bytes::from_static(b"\xc3a")),
        Err(TokenError::InvalidStart { source }) if source.byte() == 0xc3
    ));
}

#[test]
fn token_character_must_be_tchar_colon_or_slash() {
    for (text, index, byte) in [
        ("a b", 1, b' '),
        ("a\"b", 1, b'"'),
        ("a,b", 1, b','),
        ("a;b", 1, b';'),
        ("a=b", 1, b'='),
        ("a(b", 1, b'('),
        ("ab)", 2, b')'),
        ("a<b", 1, b'<'),
        ("a>b", 1, b'>'),
        ("a?b", 1, b'?'),
        ("a@b", 1, b'@'),
        ("a[b", 1, b'['),
        ("a\\b", 1, b'\\'),
        ("a]b", 1, b']'),
        ("a{b", 1, b'{'),
        ("a}b", 1, b'}'),
        ("abc\t", 3, b'\t'),
    ] {
        match Token::from_static(text) {
            Err(TokenError::InvalidCharacter { source }) => {
                assert_eq!((source.index(), source.byte()), (index, byte), "{text}");
            }
            other => panic!("{text}: {other:?}"),
        }
    }
    assert!(matches!(
        Token::new(Bytes::from_static(b"ab\xff")),
        Err(TokenError::InvalidCharacter { source }) if source.index() == 2 && source.byte() == 0xff
    ));
}

#[test]
fn error_display_and_source() {
    assert_eq!(TokenError::Empty.to_string(), "a token is empty");
    assert!(TokenError::Empty.source().is_none());

    let start = Token::from_static("1").unwrap_err();
    assert_eq!(
        start.to_string(),
        "a token starts with a byte that cannot start it"
    );
    assert_eq!(
        start.source().unwrap().to_string(),
        "byte 0x31 is not a letter or `*`"
    );

    let character = Token::from_static("a b").unwrap_err();
    assert_eq!(
        character.to_string(),
        "a token holds a byte that cannot be in it"
    );
    assert_eq!(
        character.source().unwrap().to_string(),
        "byte 0x20 at 1 is not a `tchar`, `:` or `/`"
    );
}

#[test]
fn accessors_and_formatting() {
    let token = token("text/html");
    assert_eq!(token.as_str(), "text/html");
    assert_eq!(token.to_string(), "text/html");
    assert_eq!(format!("{token:?}"), "Token(text/html)");
    assert_eq!(&token.clone().into_bytes()[..], b"text/html");
    let mut out = Vec::new();
    token.serialize(&mut out);
    assert_eq!(out, b"text/html");
}

#[test]
fn parse_takes_the_longest_token() {
    let (token, rest) = parse(b"foo123/456, bar");
    assert_eq!(token.as_str(), "foo123/456");
    assert_eq!(&rest[..], b", bar");
    let (token, rest) = parse(b"*;a=1");
    assert_eq!(token.as_str(), "*");
    assert_eq!(&rest[..], b";a=1");
    let (token, rest) = parse(b"a:b/c=d");
    assert_eq!(token.as_str(), "a:b/c");
    assert_eq!(&rest[..], b"=d");
    let (token, rest) = parse(b"abc\"x");
    assert_eq!(token.as_str(), "abc");
    assert_eq!(&rest[..], b"\"x");
    let (token, rest) = parse(b"a(b)");
    assert_eq!(token.as_str(), "a");
    assert_eq!(&rest[..], b"(b)");
}
