use super::*;
use alloc::format;
use alloc::string::ToString;

fn parse(input: &'static [u8]) -> (Result<SfString, SyntaxError>, Bytes) {
    let mut parser = Parser::new(Bytes::from_static(input));
    let result = SfString::parse(&mut parser);
    (result, Bytes::copy_from_slice(parser.rest()))
}

fn parsed(input: &'static [u8]) -> alloc::string::String {
    parse(input).0.unwrap().as_str().to_string()
}

fn serialized(text: &'static str) -> alloc::string::String {
    let mut out = Vec::new();
    SfString::from_static(text).unwrap().serialize(&mut out);
    alloc::string::String::from_utf8(out).unwrap()
}

#[test]
fn new_accepts_visible_ascii_and_space() {
    let all: Vec<u8> = (0x20..=0x7e).collect();
    let string = SfString::new(Bytes::from(all.clone())).unwrap();
    assert_eq!(string.as_str().as_bytes(), &all[..]);
    assert_eq!(SfString::from_static("").unwrap().as_str(), "");
    assert_eq!(SfString::from_static(" ").unwrap().as_str(), " ");
    assert_eq!(
        SfString::from_static("hello world").unwrap().as_str(),
        "hello world"
    );
    assert_eq!(
        SfString::from_static("a\"b\\c").unwrap().as_str(),
        "a\"b\\c"
    );
}

#[test]
fn new_rejects_other_bytes() {
    for byte in [0x00, 0x09, 0x0a, 0x0d, 0x1f, 0x7f, 0x80, 0xc3, 0xff] {
        let error = SfString::new(Bytes::from(alloc::vec![b'a', byte])).unwrap_err();
        assert_eq!(error.index(), 1, "{byte:#x}");
        assert_eq!(error.byte(), byte);
    }
    let error = SfString::new(Bytes::from_static(b"ab\x00cd\x01")).unwrap_err();
    assert_eq!((error.index(), error.byte()), (2, 0));
    assert!(SfString::from_static("café").is_err());
}

#[test]
fn character_error_display() {
    let error = SfString::new(Bytes::from_static(b"a\x7f")).unwrap_err();
    assert_eq!(
        error.to_string(),
        "byte 0x7f at 1 is not visible ASCII or a space"
    );
}

#[test]
fn accessors_and_formatting() {
    let string = SfString::from_static("a b").unwrap();
    assert_eq!(string.as_str(), "a b");
    assert_eq!(string.to_string(), "a b");
    assert_eq!(format!("{string:?}"), "SfString(\"a b\")");
    assert_eq!(&string.clone().into_bytes()[..], b"a b");
    assert_eq!(string, SfString::from_static("a b").unwrap());
    assert!(SfString::from_static("a").unwrap() < SfString::from_static("b").unwrap());
}

#[test]
fn parses_strings() {
    assert_eq!(parsed(b"\"hello world\""), "hello world");
    assert_eq!(parsed(b"\"\""), "");
    assert_eq!(parsed(b"\" \""), " ");
    assert_eq!(
        parsed(b"\"!#$%&'()*+,-./:;<=>?@[]^_`{|}~\""),
        "!#$%&'()*+,-./:;<=>?@[]^_`{|}~"
    );
}

#[test]
fn parses_escapes() {
    assert_eq!(parsed(b"\"a\\\"b\""), "a\"b");
    assert_eq!(parsed(b"\"a\\\\b\""), "a\\b");
    assert_eq!(parsed(b"\"\\\"\""), "\"");
    assert_eq!(parsed(b"\"\\\\\""), "\\");
    assert_eq!(parsed(b"\"ab\\\"cd\\\\ef\""), "ab\"cd\\ef");
    assert_eq!(parsed(b"\"\\\\\\\"\""), "\\\"");
}

#[test]
fn parse_stops_after_the_closing_quote() {
    let (result, rest) = parse(b"\"a\"b, c");
    assert_eq!(result.unwrap().as_str(), "a");
    assert_eq!(&rest[..], b"b, c");
    let (result, rest) = parse(b"\"a\\\"\";x");
    assert_eq!(result.unwrap().as_str(), "a\"");
    assert_eq!(&rest[..], b";x");
}

#[test]
fn unclosed_string_reports_the_opening_quote() {
    assert_eq!(
        parse(b"\"abc").0,
        Err(SyntaxError::UnclosedString { index: 0 })
    );
    assert_eq!(
        parse(b"\"").0,
        Err(SyntaxError::UnclosedString { index: 0 })
    );
    // A backslash at the end escapes nothing.
    assert_eq!(
        parse(b"\"abc\\").0,
        Err(SyntaxError::UnclosedString { index: 0 })
    );
    assert_eq!(
        parse(b"\"a\\\"").0,
        Err(SyntaxError::UnclosedString { index: 0 })
    );
}

#[test]
fn only_quote_and_backslash_can_be_escaped() {
    assert_eq!(
        parse(b"\"a\\nb\"").0,
        Err(SyntaxError::InvalidStringEscape { index: 3 })
    );
    assert_eq!(
        parse(b"\"\\a\"").0,
        Err(SyntaxError::InvalidStringEscape { index: 2 })
    );
    assert_eq!(
        parse(b"\"x\\ \"").0,
        Err(SyntaxError::InvalidStringEscape { index: 3 })
    );
    assert_eq!(
        parse(b"\"x\\\"y\\z\"").0,
        Err(SyntaxError::InvalidStringEscape { index: 6 })
    );
}

#[test]
fn string_rejects_bytes_outside_0x20_to_0x7e() {
    assert_eq!(
        parse(b"\"a\x01\"").0,
        Err(SyntaxError::InvalidStringCharacter { index: 2 })
    );
    assert_eq!(
        parse(b"\"a\tb\"").0,
        Err(SyntaxError::InvalidStringCharacter { index: 2 })
    );
    assert_eq!(
        parse(b"\"a\x7f\"").0,
        Err(SyntaxError::InvalidStringCharacter { index: 2 })
    );
    assert_eq!(
        parse(b"\"\xc3\xbc\"").0,
        Err(SyntaxError::InvalidStringCharacter { index: 1 })
    );
    // After an escape the characters are still checked.
    assert_eq!(
        parse(b"\"a\\\"\x80\"").0,
        Err(SyntaxError::InvalidStringCharacter { index: 4 })
    );
}

#[test]
fn serializes_with_escapes() {
    assert_eq!(serialized(""), "\"\"");
    assert_eq!(serialized("hello world"), "\"hello world\"");
    assert_eq!(serialized("a\"b"), "\"a\\\"b\"");
    assert_eq!(serialized("a\\b"), "\"a\\\\b\"");
    assert_eq!(serialized("\\\""), "\"\\\\\\\"\"");
    assert_eq!(serialized("%\x7e"), "\"%~\"");
}

#[test]
fn serialize_then_parse_round_trips() {
    let string = SfString::from_static("it's \"quoted\" and \\ slashed").unwrap();
    let mut out = Vec::new();
    string.serialize(&mut out);
    let mut parser = Parser::new(Bytes::from(out));
    assert_eq!(SfString::parse(&mut parser).unwrap(), string);
    assert!(parser.is_empty());
}
