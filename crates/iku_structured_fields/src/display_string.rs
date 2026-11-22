//! Display strings (RFC 9651 §3.3.8).

use crate::syntax::{
    ExpectedDisplayStringQuoteSnafu, InvalidDisplayStringCharacterSnafu,
    InvalidPercentEncodingSnafu, InvalidUtf8Snafu, Parser, SyntaxError, UnclosedDisplayStringSnafu,
};
use alloc::string::String;
use alloc::vec::Vec;

/// Parses a display string (§4.2.10); the parser is at `%`.
pub(crate) fn parse(parser: &mut Parser) -> Result<String, SyntaxError> {
    let open = parser.index();
    parser.advance();
    if parser.peek() != Some(b'"') {
        return ExpectedDisplayStringQuoteSnafu {
            index: parser.index(),
        }
        .fail();
    }
    parser.advance();
    let mut bytes = Vec::new();
    loop {
        let index = parser.index();
        match parser.next_byte() {
            None => return UnclosedDisplayStringSnafu { index: open }.fail(),
            Some(b'"') => break,
            Some(b'%') => {
                let rest = parser.rest();
                let (Some(high), Some(low)) = (
                    rest.first().copied().and_then(lowercase_hex_value),
                    rest.get(1).copied().and_then(lowercase_hex_value),
                ) else {
                    return InvalidPercentEncodingSnafu { index }.fail();
                };
                bytes.push((high << 4) | low);
                parser.skip(2);
            }
            Some(byte @ 0x20..=0x7e) => bytes.push(byte),
            Some(_) => return InvalidDisplayStringCharacterSnafu { index }.fail(),
        }
    }
    String::from_utf8(bytes).map_err(|_| InvalidUtf8Snafu { index: open }.build())
}

/// Writes `%"`, the text with `%`, `"` and every byte outside visible ASCII
/// and space percent-encoded in lowercase, and `"` (§4.1.11).
pub(crate) fn serialize(text: &str, out: &mut Vec<u8>) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    out.extend_from_slice(b"%\"");
    for &byte in text.as_bytes() {
        if byte == b'%' || byte == b'"' || !(0x20..=0x7e).contains(&byte) {
            out.push(b'%');
            out.push(HEX[usize::from(byte >> 4)]);
            out.push(HEX[usize::from(byte & 0x0f)]);
        } else {
            out.push(byte);
        }
    }
    out.push(b'"');
}

/// `lc-hexdig`: a digit or a lowercase letter from `a` to `f`.
fn lowercase_hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}
