//! Byte sequences (RFC 9651 §3.3.5).

use crate::base64::{self, DecodeError};
use crate::syntax::{
    InvalidBase64CharacterSnafu, InvalidBase64LengthSnafu, MisplacedBase64PaddingSnafu, Parser,
    SyntaxError, UnclosedByteSequenceSnafu,
};
use alloc::vec::Vec;
use bytes::Bytes;

/// Parses a byte sequence (§4.2.7); the parser is at `:`.
pub(crate) fn parse(parser: &mut Parser) -> Result<Bytes, SyntaxError> {
    let open = parser.index();
    parser.advance();
    let Some(length) = parser.rest().iter().position(|&byte| byte == b':') else {
        return UnclosedByteSequenceSnafu { index: open }.fail();
    };
    let content = open + 1;
    let decoded = base64::decode(&parser.rest()[..length]).map_err(|error| match error {
        DecodeError::InvalidCharacter(index) => InvalidBase64CharacterSnafu {
            index: content + index,
        }
        .build(),
        DecodeError::MisplacedPadding(index) => MisplacedBase64PaddingSnafu {
            index: content + index,
        }
        .build(),
        DecodeError::InvalidLength => InvalidBase64LengthSnafu { index: open }.build(),
    })?;
    parser.skip(length + 1);
    Ok(Bytes::from(decoded))
}

/// Writes `:`, the bytes in base64 with padding, and `:` (§4.1.8).
pub(crate) fn serialize(bytes: &[u8], out: &mut Vec<u8>) {
    out.push(b':');
    base64::encode(bytes, out);
    out.push(b':');
}
