//! Strings (RFC 9651 §3.3.3).

use crate::syntax::{
    InvalidStringCharacterSnafu, InvalidStringEscapeSnafu, Parser, SyntaxError, UnclosedStringSnafu,
};
use alloc::vec::Vec;
use bytes::Bytes;
use core::fmt;
use snafu::Snafu;

/// A string of visible ASCII characters and spaces (RFC 9651 §3.3.3).
///
/// Text beyond ASCII is a display string (§3.3.8).
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SfString(Bytes);

impl SfString {
    /// Checks that `bytes` are visible ASCII characters and spaces.
    ///
    /// # Errors
    ///
    /// [`InvalidSfStringCharacterError`] for the first byte that is neither.
    pub fn new(bytes: Bytes) -> Result<Self, InvalidSfStringCharacterError> {
        match bytes.iter().position(|&byte| !is_string_character(byte)) {
            Some(index) => Err(InvalidSfStringCharacterError {
                index,
                byte: bytes[index],
            }),
            None => Ok(Self(bytes)),
        }
    }

    /// As [`SfString::new`], without copying `text`.
    ///
    /// # Errors
    ///
    /// As [`SfString::new`].
    pub fn from_static(text: &'static str) -> Result<Self, InvalidSfStringCharacterError> {
        Self::new(Bytes::from_static(text.as_bytes()))
    }

    /// The characters.
    pub fn as_str(&self) -> &str {
        // Invariant: every byte is ASCII, so the bytes are UTF-8.
        core::str::from_utf8(&self.0).expect("a string holds only ASCII")
    }

    /// The characters as [`Bytes`], without copying.
    pub fn into_bytes(self) -> Bytes {
        self.0
    }

    /// Parses a string (§4.2.5); the parser is at `"`. Bytes without escapes
    /// are not copied.
    pub(crate) fn parse(parser: &mut Parser) -> Result<Self, SyntaxError> {
        let open = parser.index();
        parser.advance();
        let start = parser.index();
        // The unescaped characters, once an escape has been seen.
        let mut unescaped: Option<Vec<u8>> = None;
        loop {
            let index = parser.index();
            let Some(byte) = parser.next_byte() else {
                return UnclosedStringSnafu { index: open }.fail();
            };
            match byte {
                b'"' => {
                    return Ok(Self(match unescaped {
                        Some(characters) => Bytes::from(characters),
                        None => parser.slice_from(start).slice(..index - start),
                    }));
                }
                b'\\' => {
                    let escaped_index = parser.index();
                    let escaped = match parser.next_byte() {
                        None => return UnclosedStringSnafu { index: open }.fail(),
                        Some(escaped @ (b'"' | b'\\')) => escaped,
                        Some(_) => {
                            return InvalidStringEscapeSnafu {
                                index: escaped_index,
                            }
                            .fail();
                        }
                    };
                    unescaped
                        .get_or_insert_with(|| parser.slice_from(start)[..index - start].to_vec())
                        .push(escaped);
                }
                byte if is_string_character(byte) => {
                    if let Some(characters) = &mut unescaped {
                        characters.push(byte);
                    }
                }
                _ => return InvalidStringCharacterSnafu { index }.fail(),
            }
        }
    }

    /// Writes the string in quotes, escaping `"` and `\` (§4.1.6).
    pub(crate) fn serialize(&self, out: &mut Vec<u8>) {
        out.push(b'"');
        for &byte in self.0.iter() {
            if byte == b'"' || byte == b'\\' {
                out.push(b'\\');
            }
            out.push(byte);
        }
        out.push(b'"');
    }
}

impl fmt::Display for SfString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Debug for SfString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SfString({:?})", self.as_str())
    }
}

/// A visible ASCII character or a space: `%x20-7E` (§3.3.3).
const fn is_string_character(byte: u8) -> bool {
    matches!(byte, 0x20..=0x7e)
}

/// A byte that is not a visible ASCII character or a space, in what should
/// be a string (RFC 9651 §3.3.3).
///
/// It is constructed only when the check fails, so its byte is never one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(display("byte {byte:#04x} at {index} is not visible ASCII or a space"))]
pub struct InvalidSfStringCharacterError {
    index: usize,
    byte: u8,
}

impl InvalidSfStringCharacterError {
    /// Where the byte is, counted from the start of the string.
    pub const fn index(self) -> usize {
        self.index
    }

    /// The byte.
    pub const fn byte(self) -> u8 {
        self.byte
    }
}

#[cfg(test)]
mod tests;
