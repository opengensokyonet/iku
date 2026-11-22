use super::*;
use alloc::string::ToString;

fn parse(input: &'static [u8]) -> (Result<BareItem, SyntaxError>, Bytes) {
    let mut parser = Parser::new(Bytes::from_static(input));
    let result = BareItem::parse(&mut parser);
    (result, Bytes::copy_from_slice(parser.rest()))
}

fn ok(input: &'static str) -> BareItem {
    parse(input.as_bytes()).0.unwrap()
}

fn err(input: &'static str) -> SyntaxError {
    parse(input.as_bytes()).0.unwrap_err()
}

fn serialized(item: &BareItem) -> alloc::string::String {
    let mut out = Vec::new();
    item.serialize(&mut out);
    alloc::string::String::from_utf8(out).unwrap()
}

/// Parses `input` and writes it back.
fn canonical(input: &'static str) -> alloc::string::String {
    serialized(&ok(input))
}

fn integer(value: i64) -> BareItem {
    BareItem::Integer(Integer::new(value).unwrap())
}

fn decimal(thousandths: i64) -> BareItem {
    BareItem::Decimal(Decimal::from_thousandths(thousandths).unwrap())
}

fn date(seconds: i64) -> BareItem {
    BareItem::Date(Date::from_seconds(Integer::new(seconds).unwrap()))
}

#[test]
fn parses_each_type() {
    assert_eq!(ok("42"), integer(42));
    assert_eq!(ok("-42"), integer(-42));
    assert_eq!(ok("4.5"), decimal(4500));
    assert_eq!(
        ok("\"hello world\""),
        BareItem::String(crate::String::from_static("hello world").unwrap())
    );
    assert_eq!(
        ok("foo123/456"),
        BareItem::Token(Token::from_static("foo123/456").unwrap())
    );
    assert_eq!(ok("*"), BareItem::Token(Token::from_static("*").unwrap()));
    assert_eq!(
        ok("Foo"),
        BareItem::Token(Token::from_static("Foo").unwrap())
    );
    assert_eq!(
        ok(":aGVsbG8=:"),
        BareItem::ByteSequence(Bytes::from_static(b"hello"))
    );
    assert_eq!(ok("?1"), BareItem::Boolean(true));
    assert_eq!(ok("?0"), BareItem::Boolean(false));
    assert_eq!(ok("@1659578233"), date(1_659_578_233));
    assert_eq!(
        ok("%\"f%c3%bc%c3%bc\""),
        BareItem::DisplayString("füü".to_string())
    );
}

#[test]
fn parse_leaves_what_follows() {
    for (input, rest) in [
        ("1;a", ";a"),
        ("1, 2", ", 2"),
        ("1.5)", ")"),
        ("abc=1", "=1"),
        ("\"a\" b", " b"),
        (":AA==:x", "x"),
        ("?1;x", ";x"),
        ("@5 ", " "),
        ("%\"a\")", ")"),
    ] {
        let (result, left) = parse(input.as_bytes());
        result.unwrap();
        assert_eq!(&left[..], rest.as_bytes(), "{input}");
    }
}

#[test]
fn nothing_else_starts_a_bare_item() {
    for input in [
        "", " ", "\t", "!", "#", "(", ")", ",", ";", "=", "<", "[", "\\", "{", "~", "\x7f",
        "\u{e9}", "+1", ".5",
    ] {
        assert_eq!(
            err(input),
            SyntaxError::InvalidBareItemStart { index: 0 },
            "{input:?}"
        );
    }
}

#[test]
fn bare_item_start_index_is_where_the_parser_is() {
    let mut parser = Parser::new(Bytes::from_static(b"1 )"));
    parser.skip(2);
    assert_eq!(
        BareItem::parse(&mut parser),
        Err(SyntaxError::InvalidBareItemStart { index: 2 })
    );
}

#[test]
fn number_errors_pass_through() {
    assert_eq!(err("-"), SyntaxError::MissingDigit { index: 1 });
    assert_eq!(
        err("1234567890123456"),
        SyntaxError::IntegerTooLong { index: 15 }
    );
    assert_eq!(err("1."), SyntaxError::EmptyFraction { index: 1 });
}

#[test]
fn booleans_are_only_0_and_1() {
    assert_eq!(err("?"), SyntaxError::InvalidBoolean { index: 1 });
    for input in ["?2", "?t", "?true", "?-", "? ", "?\""] {
        assert_eq!(
            err(input),
            SyntaxError::InvalidBoolean { index: 1 },
            "{input}"
        );
    }
    let (result, rest) = parse(b"?01");
    assert_eq!(result.unwrap(), BareItem::Boolean(false));
    assert_eq!(&rest[..], b"1");
}

#[test]
fn dates_are_integers() {
    assert_eq!(ok("@0"), date(0));
    assert_eq!(ok("@-62135596800"), date(-62_135_596_800));
    assert_eq!(ok("@999999999999999"), date(999_999_999_999_999));
    assert_eq!(err("@1.5"), SyntaxError::DecimalDate { index: 1 });
    assert_eq!(err("@-1.5"), SyntaxError::DecimalDate { index: 1 });
    assert_eq!(err("@1659578233.0"), SyntaxError::DecimalDate { index: 1 });
    assert_eq!(err("@"), SyntaxError::MissingDigit { index: 1 });
    assert_eq!(err("@-"), SyntaxError::MissingDigit { index: 2 });
    assert_eq!(err("@a"), SyntaxError::MissingDigit { index: 1 });
    assert_eq!(err("@ 1"), SyntaxError::MissingDigit { index: 1 });
    assert_eq!(
        err("@1234567890123456"),
        SyntaxError::IntegerTooLong { index: 16 }
    );
    // The error of a decimal date does not depend on a later error.
    assert_eq!(err("@1."), SyntaxError::EmptyFraction { index: 2 });
}

#[test]
fn byte_sequences_decode() {
    assert_eq!(ok("::"), BareItem::ByteSequence(Bytes::new()));
    assert_eq!(
        ok(":cHJldGVuZCB0aGlzIGlzIGJpbmFyeSBjb250ZW50Lg==:"),
        BareItem::ByteSequence(Bytes::from_static(b"pretend this is binary content."))
    );
    assert_eq!(
        ok(":/w==:"),
        BareItem::ByteSequence(Bytes::from_static(&[0xff]))
    );
}

#[test]
fn byte_sequences_accept_missing_padding_and_non_zero_pad_bits() {
    let expected = BareItem::ByteSequence(Bytes::from_static(b"foob"));
    assert_eq!(ok(":Zm9vYg==:"), expected);
    assert_eq!(ok(":Zm9vYg:"), expected);
    assert_eq!(ok(":Zm9vYh:"), expected);
    assert_eq!(ok(":Zm9vYh==:"), expected);
    // The pad bits are lost, so the output is the canonical form.
    assert_eq!(canonical(":Zm9vYh:"), ":Zm9vYg==:");
}

#[test]
fn byte_sequence_errors() {
    assert_eq!(err(":abc"), SyntaxError::UnclosedByteSequence { index: 0 });
    assert_eq!(err(":"), SyntaxError::UnclosedByteSequence { index: 0 });
    assert_eq!(err(":Zg=="), SyntaxError::UnclosedByteSequence { index: 0 });
    assert_eq!(
        err(":Zm9v*:"),
        SyntaxError::InvalidBase64Character { index: 5 }
    );
    assert_eq!(
        err(":-_:"),
        SyntaxError::InvalidBase64Character { index: 1 }
    );
    assert_eq!(
        err(":Zm 9:"),
        SyntaxError::InvalidBase64Character { index: 3 }
    );
    assert_eq!(
        err(":Zg===:"),
        SyntaxError::MisplacedBase64Padding { index: 5 }
    );
    assert_eq!(
        err(":Zm9v=:"),
        SyntaxError::MisplacedBase64Padding { index: 5 }
    );
    assert_eq!(
        err(":Zg=a:"),
        SyntaxError::MisplacedBase64Padding { index: 3 }
    );
    assert_eq!(err(":Z:"), SyntaxError::InvalidBase64Length { index: 0 });
    assert_eq!(
        err(":Zm9vY:"),
        SyntaxError::InvalidBase64Length { index: 0 }
    );
    assert_eq!(err(":Z==:"), SyntaxError::InvalidBase64Length { index: 0 });
}

#[test]
fn byte_sequence_error_index_counts_from_the_line_start() {
    let mut parser = Parser::new(Bytes::from_static(b"a, :Zm9v*:"));
    parser.skip(3);
    assert_eq!(
        BareItem::parse(&mut parser),
        Err(SyntaxError::InvalidBase64Character { index: 8 })
    );
}

#[test]
fn display_strings_decode() {
    assert_eq!(
        ok("%\"\""),
        BareItem::DisplayString(alloc::string::String::new())
    );
    assert_eq!(
        ok("%\"abc def\""),
        BareItem::DisplayString("abc def".to_string())
    );
    assert_eq!(
        ok("%\"This is intended for display to %c3%bcsers.\""),
        BareItem::DisplayString("This is intended for display to \u{fc}sers.".to_string())
    );
    assert_eq!(
        ok("%\"%f0%9f%98%80\""),
        BareItem::DisplayString("\u{1f600}".to_string())
    );
    assert_eq!(
        ok("%\"%22%25\""),
        BareItem::DisplayString("\"%".to_string())
    );
    assert_eq!(
        ok("%\"%00%0a\""),
        BareItem::DisplayString("\0\n".to_string())
    );
    // A plain letter may also be percent-encoded.
    assert_eq!(ok("%\"%61\""), BareItem::DisplayString("a".to_string()));
}

#[test]
fn display_string_errors() {
    assert_eq!(
        err("%"),
        SyntaxError::ExpectedDisplayStringQuote { index: 1 }
    );
    assert_eq!(
        err("%a"),
        SyntaxError::ExpectedDisplayStringQuote { index: 1 }
    );
    assert_eq!(
        err("%'a'"),
        SyntaxError::ExpectedDisplayStringQuote { index: 1 }
    );
    assert_eq!(err("%\""), SyntaxError::UnclosedDisplayString { index: 0 });
    assert_eq!(
        err("%\"abc"),
        SyntaxError::UnclosedDisplayString { index: 0 }
    );
    assert_eq!(
        err("%\"a%C3%BC\""),
        SyntaxError::InvalidPercentEncoding { index: 3 }
    );
    assert_eq!(
        err("%\"%c\""),
        SyntaxError::InvalidPercentEncoding { index: 2 }
    );
    assert_eq!(
        err("%\"%\""),
        SyntaxError::InvalidPercentEncoding { index: 2 }
    );
    assert_eq!(
        err("%\"%g0\""),
        SyntaxError::InvalidPercentEncoding { index: 2 }
    );
    assert_eq!(
        err("%\"%0g\""),
        SyntaxError::InvalidPercentEncoding { index: 2 }
    );
    assert_eq!(
        err("%\"%C3%BC\""),
        SyntaxError::InvalidPercentEncoding { index: 2 }
    );
    assert_eq!(err("%\"%c3\""), SyntaxError::InvalidUtf8 { index: 0 });
    assert_eq!(err("%\"%c3%28\""), SyntaxError::InvalidUtf8 { index: 0 });
    assert_eq!(err("%\"%ff\""), SyntaxError::InvalidUtf8 { index: 0 });
    // An overlong encoding of `/`.
    assert_eq!(err("%\"%c0%af\""), SyntaxError::InvalidUtf8 { index: 0 });
    assert_eq!(err("%\"%ed%a0%80\""), SyntaxError::InvalidUtf8 { index: 0 });
    for (input, index) in [
        (&b"%\"a\x01\""[..], 3),
        (&b"%\"a\tb\""[..], 3),
        (&b"%\"a\x7f\""[..], 3),
        (&b"%\"\xc3\xbc\""[..], 2),
    ] {
        let mut parser = Parser::new(Bytes::copy_from_slice(input));
        assert_eq!(
            BareItem::parse(&mut parser),
            Err(SyntaxError::InvalidDisplayStringCharacter { index }),
            "{input:?}"
        );
    }
}

#[test]
fn backslash_is_an_ordinary_character_in_a_display_string() {
    assert_eq!(ok("%\"a\\b\""), BareItem::DisplayString("a\\b".to_string()));
    // So a backslash before the closing quote does not escape it.
    assert_eq!(ok("%\"a\\\""), BareItem::DisplayString("a\\".to_string()));
}

#[test]
fn serializes_each_type() {
    assert_eq!(serialized(&integer(0)), "0");
    assert_eq!(serialized(&integer(-42)), "-42");
    assert_eq!(serialized(&decimal(4500)), "4.5");
    assert_eq!(serialized(&decimal(-1)), "-0.001");
    assert_eq!(serialized(&decimal(2000)), "2.0");
    assert_eq!(
        serialized(&BareItem::String(
            crate::String::from_static("a\"b\\c").unwrap()
        )),
        "\"a\\\"b\\\\c\""
    );
    assert_eq!(
        serialized(&BareItem::Token(Token::from_static("text/html").unwrap())),
        "text/html"
    );
    assert_eq!(serialized(&BareItem::Boolean(true)), "?1");
    assert_eq!(serialized(&BareItem::Boolean(false)), "?0");
    assert_eq!(serialized(&date(1_659_578_233)), "@1659578233");
    assert_eq!(serialized(&date(-62_135_596_800)), "@-62135596800");
    assert_eq!(serialized(&date(0)), "@0");
}

#[test]
fn serializes_byte_sequences_with_padding() {
    let bytes = |b: &'static [u8]| BareItem::ByteSequence(Bytes::from_static(b));
    assert_eq!(serialized(&bytes(b"")), "::");
    assert_eq!(serialized(&bytes(b"f")), ":Zg==:");
    assert_eq!(serialized(&bytes(b"fo")), ":Zm8=:");
    assert_eq!(serialized(&bytes(b"foo")), ":Zm9v:");
    assert_eq!(
        serialized(&bytes(b"pretend this is binary content.")),
        ":cHJldGVuZCB0aGlzIGlzIGJpbmFyeSBjb250ZW50Lg==:"
    );
    assert_eq!(
        canonical(":cHJldGVuZCB0aGlzIGlzIGJpbmFyeSBjb250ZW50Lg:"),
        ":cHJldGVuZCB0aGlzIGlzIGJpbmFyeSBjb250ZW50Lg==:"
    );
}

#[test]
fn serializes_display_strings_in_lowercase_percent_encoding() {
    let text = |t: &str| serialized(&BareItem::DisplayString(t.to_string()));
    assert_eq!(text(""), "%\"\"");
    assert_eq!(text("abc def"), "%\"abc def\"");
    assert_eq!(text("f\u{fc}\u{fc}"), "%\"f%c3%bc%c3%bc\"");
    assert_eq!(text("\u{1f600}"), "%\"%f0%9f%98%80\"");
    assert_eq!(text("\u{ff}"), "%\"%c3%bf\"");
    assert_eq!(text("100%"), "%\"100%25\"");
    assert_eq!(text("say \"hi\""), "%\"say %22hi%22\"");
    assert_eq!(text("a\tb\n\0"), "%\"a%09b%0a%00\"");
    assert_eq!(text("\x7f~"), "%\"%7f~\"");
    assert_eq!(text("\\"), "%\"\\\"");
}

#[test]
fn canonical_forms() {
    assert_eq!(canonical("1.50"), "1.5");
    assert_eq!(canonical("1.000"), "1.0");
    assert_eq!(canonical("-0.0"), "0.0");
    assert_eq!(canonical("-0"), "0");
    assert_eq!(canonical("007"), "7");
    assert_eq!(canonical("0.500"), "0.5");
    assert_eq!(canonical("@0042"), "@42");
    assert_eq!(canonical("%\"%41\""), "%\"A\"");
    assert_eq!(canonical("*foo"), "*foo");
}

#[test]
fn round_trips_every_type() {
    for text in [
        "5",
        "-5",
        "4.5",
        "\"x y\"",
        "foo/bar",
        "*",
        ":AAEC:",
        "?1",
        "?0",
        "@123",
        "%\"%c3%a9\"",
    ] {
        assert_eq!(canonical(text), text);
    }
}

#[test]
fn converts_from_checked_types() {
    let integer_value = Integer::new(3).unwrap();
    let decimal_value = Decimal::from_thousandths(3500).unwrap();
    let string = crate::String::from_static("s").unwrap();
    let token = Token::from_static("t").unwrap();
    let date_value = Date::from_seconds(integer_value);
    assert_eq!(
        BareItem::from(integer_value),
        BareItem::Integer(integer_value)
    );
    assert_eq!(
        BareItem::from(decimal_value),
        BareItem::Decimal(decimal_value)
    );
    assert_eq!(BareItem::from(string.clone()), BareItem::String(string));
    assert_eq!(BareItem::from(token.clone()), BareItem::Token(token));
    assert_eq!(BareItem::from(true), BareItem::Boolean(true));
    assert_eq!(BareItem::from(date_value), BareItem::Date(date_value));
}

#[test]
fn date_accessors() {
    let seconds = Integer::new(-5).unwrap();
    let date_value = Date::from_seconds(seconds);
    assert_eq!(date_value.seconds(), seconds);
    assert_eq!(Date::from_seconds(Integer::MAX).seconds(), Integer::MAX);
    assert!(Date::from_seconds(Integer::MIN) < Date::from_seconds(Integer::MAX));
}

#[test]
fn types_do_not_mix() {
    assert_ne!(integer(1), decimal(1000));
    assert_ne!(integer(1), BareItem::Boolean(true));
    assert_ne!(integer(1), date(1));
    assert_ne!(
        BareItem::String(crate::String::from_static("a").unwrap()),
        BareItem::Token(Token::from_static("a").unwrap())
    );
}
