use super::*;
use alloc::format;

fn new(bytes: &[u8]) -> Result<FieldValue, FieldValueError> {
    FieldValue::new(Bytes::copy_from_slice(bytes))
}

#[test]
fn field_value_accepts_an_empty_value() {
    let value = new(b"").unwrap();
    assert!(value.is_empty());
    assert_eq!(value.len(), 0);
}

#[test]
fn field_value_accepts_a_single_byte() {
    assert_eq!(new(b"a").unwrap().as_bytes(), b"a");
}

#[test]
fn field_value_accepts_inner_space_and_htab() {
    let value = new(b"a b\tc").unwrap();
    assert_eq!(value.as_bytes(), b"a b\tc");
    assert_eq!(value.len(), 5);
    assert!(!value.is_empty());
}

#[test]
fn field_value_accepts_visible_ascii() {
    let all: alloc::vec::Vec<u8> = (0x21..=0x7e).collect();
    assert!(new(&all).is_ok());
}

#[test]
fn field_value_accepts_obs_text() {
    // RFC 9110 §5.5: obs-text is opaque data.
    let all: alloc::vec::Vec<u8> = (0x80..=0xff).collect();
    assert_eq!(new(&all).unwrap().as_bytes(), &all[..]);
}

#[test]
fn field_value_rejects_leading_space_and_htab() {
    assert_eq!(new(b" a"), Err(FieldValueError::LeadingWhitespace));
    assert_eq!(new(b"\ta"), Err(FieldValueError::LeadingWhitespace));
    assert_eq!(new(b" "), Err(FieldValueError::LeadingWhitespace));
    assert_eq!(new(b"\t"), Err(FieldValueError::LeadingWhitespace));
}

#[test]
fn field_value_rejects_trailing_space_and_htab() {
    assert_eq!(new(b"a "), Err(FieldValueError::TrailingWhitespace));
    assert_eq!(new(b"a\t"), Err(FieldValueError::TrailingWhitespace));
}

#[test]
fn field_value_rejects_every_control_character_but_htab() {
    for byte in (0x00..0x20u8).filter(|&b| b != b'\t').chain([0x7f]) {
        let error = new(&[b'a', byte, b'b']).unwrap_err();
        let FieldValueError::ControlCharacter { index, character } = error else {
            panic!("byte {byte:#04x}: {error:?}");
        };
        assert_eq!(index, 1);
        assert_eq!(character.byte(), byte);
    }
}

#[test]
fn field_value_reports_the_first_control_character() {
    let error = new(b"ab\r\ncd").unwrap_err();
    assert!(matches!(
        error,
        FieldValueError::ControlCharacter { index: 2, character } if character.byte() == b'\r'
    ));
}

#[test]
fn field_value_checks_leading_whitespace_before_control_characters() {
    assert_eq!(new(b" \n"), Err(FieldValueError::LeadingWhitespace));
}

#[test]
fn field_value_checks_control_characters_before_trailing_whitespace() {
    let error = new(b"a\n ").unwrap_err();
    assert!(matches!(
        error,
        FieldValueError::ControlCharacter { index: 1, .. }
    ));
}

#[test]
fn field_value_a_lone_control_character_is_a_control_error() {
    assert!(matches!(
        new(b"\x00"),
        Err(FieldValueError::ControlCharacter { index: 0, .. })
    ));
}

#[test]
fn control_character_knows_cr_lf_and_nul() {
    // RFC 9110 §5.5.
    for byte in [b'\r', b'\n', 0u8] {
        let FieldValueError::ControlCharacter { character, .. } = new(&[byte]).unwrap_err() else {
            panic!();
        };
        assert!(character.is_cr_lf_or_nul(), "{byte:#04x}");
    }
    for byte in [0x01u8, 0x0b, 0x1f, 0x7f] {
        let FieldValueError::ControlCharacter { character, .. } = new(&[byte]).unwrap_err() else {
            panic!();
        };
        assert!(!character.is_cr_lf_or_nul(), "{byte:#04x}");
    }
}

#[test]
fn field_value_from_static_checks_like_new() {
    assert_eq!(FieldValue::from_static("gzip").unwrap().as_bytes(), b"gzip");
    assert_eq!(
        FieldValue::from_static(" x"),
        Err(FieldValueError::LeadingWhitespace)
    );
    assert_eq!(
        FieldValue::from_static("x "),
        Err(FieldValueError::TrailingWhitespace)
    );
}

#[test]
fn field_value_into_bytes_shares_the_input() {
    let input = Bytes::from_static(b"abc");
    let value = FieldValue::new(input.clone()).unwrap();
    assert_eq!(value.into_bytes().as_ptr(), input.as_ptr());
}

#[test]
fn field_value_debug_escapes_bytes() {
    let value = new(b"a\tb\xe9\"").unwrap();
    assert_eq!(format!("{value:?}"), "FieldValue(\"a\\tb\\xe9\\\"\")");
}

#[test]
fn field_value_orders_by_bytes() {
    assert!(new(b"a").unwrap() < new(b"b").unwrap());
}

#[test]
fn field_value_error_display() {
    assert_eq!(
        format!("{}", FieldValueError::LeadingWhitespace),
        "a field value starts with whitespace"
    );
    assert_eq!(
        format!("{}", FieldValueError::TrailingWhitespace),
        "a field value ends with whitespace"
    );
    let error = new(b"a\n").unwrap_err();
    assert_eq!(
        format!("{error}"),
        "a field value holds the control character 0x0a at 1"
    );
}
