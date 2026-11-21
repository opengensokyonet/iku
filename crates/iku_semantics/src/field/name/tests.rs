use super::*;
use alloc::format;

const CONSTANTS: [(FieldName, &str); 9] = [
    (FieldName::CONNECTION, "connection"),
    (FieldName::CONTENT_LENGTH, "content-length"),
    (FieldName::HOST, "host"),
    (FieldName::KEEP_ALIVE, "keep-alive"),
    (FieldName::PROXY_CONNECTION, "proxy-connection"),
    (FieldName::SET_COOKIE, "set-cookie"),
    (FieldName::TE, "te"),
    (FieldName::TRANSFER_ENCODING, "transfer-encoding"),
    (FieldName::UPGRADE, "upgrade"),
];

#[test]
fn field_name_constants_are_lowercase_tokens() {
    for (constant, text) in CONSTANTS {
        assert_eq!(constant.as_bytes(), text.as_bytes());
        assert!(token::check(constant.as_bytes()).is_ok(), "{text}");
        assert!(!constant.as_bytes().iter().any(u8::is_ascii_uppercase));
        assert_eq!(FieldName::from_static(text).unwrap(), constant);
        assert_eq!(
            FieldName::new(Bytes::from_static(text.as_bytes())).unwrap(),
            constant
        );
    }
}

#[test]
fn field_name_lowercases_on_construction() {
    // RFC 9110 §5.1: field names are case-insensitive.
    let name = FieldName::from_static("Content-Length").unwrap();
    assert_eq!(name.as_bytes(), b"content-length");
    assert_eq!(name, FieldName::CONTENT_LENGTH);
    assert_eq!(
        FieldName::new(Bytes::from_static(b"HOST")).unwrap(),
        FieldName::HOST
    );
}

#[test]
fn field_name_lowercases_only_ascii_letters() {
    let name = FieldName::from_static("X-A_B.1~").unwrap();
    assert_eq!(name.as_bytes(), b"x-a_b.1~");
}

#[test]
fn field_name_without_uppercase_shares_the_input_bytes() {
    let input = Bytes::from_static(b"x-foo");
    let name = FieldName::new(input.clone()).unwrap();
    assert_eq!(name.as_bytes().as_ptr(), input.as_ptr());
}

#[test]
fn field_name_into_bytes() {
    let name = FieldName::from_static("X-Foo").unwrap();
    assert_eq!(name.into_bytes(), Bytes::from_static(b"x-foo"));
}

#[test]
fn field_name_accepts_a_single_byte() {
    assert_eq!(FieldName::from_static("A").unwrap().as_bytes(), b"a");
}

#[test]
fn field_name_rejects_empty_input() {
    assert_eq!(
        FieldName::new(Bytes::new()).unwrap_err().token_error(),
        TokenError::Empty
    );
    assert_eq!(
        FieldName::from_static("").unwrap_err().token_error(),
        TokenError::Empty
    );
}

#[test]
fn field_name_rejects_a_colon_space_and_control_bytes() {
    for (text, index, byte) in [
        (&b"host:"[..], 4, b':'),
        (b"x foo", 1, b' '),
        (b"x\tfoo", 1, b'\t'),
        (b"\x7f", 0, 0x7f),
        (b"a\xe9", 1, 0xe9),
    ] {
        let error = FieldName::new(Bytes::copy_from_slice(text))
            .unwrap_err()
            .token_error();
        assert!(
            matches!(
                error,
                TokenError::InvalidCharacter { source }
                    if source.index() == index && source.byte() == byte
            ),
            "{text:?}: {error:?}"
        );
    }
}

#[test]
fn field_name_orders_by_lowercase_bytes() {
    assert!(FieldName::from_static("A").unwrap() < FieldName::from_static("b").unwrap());
}

#[test]
fn field_name_display_and_debug() {
    let name = FieldName::from_static("Accept").unwrap();
    assert_eq!(format!("{name}"), "accept");
    assert_eq!(format!("{name:?}"), "FieldName(accept)");
}

#[test]
fn field_name_error_says_what_is_not_a_token() {
    let error = FieldName::from_static("a b").unwrap_err();
    assert_eq!(alloc::format!("{error}"), "the field name is not a token");
}
