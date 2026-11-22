//! Keys of dictionaries and parameters (RFC 9651 §3.1.2, §3.2).

use crate::syntax::{self, Parser, SyntaxError};
use alloc::vec::Vec;
use bytes::Bytes;
use core::fmt;
use snafu::Snafu;

/// The key of a dictionary member or a parameter (RFC 9651 §3.1.2, §3.2): a
/// lowercase letter or `*`, then lowercase letters, digits, `_`, `-`, `.` and
/// `*`.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Key(Bytes);

impl Key {
    /// Checks that `bytes` is a key.
    ///
    /// # Errors
    ///
    /// [`KeyError`] for the rule that `bytes` breaks.
    pub fn new(bytes: Bytes) -> Result<Self, KeyError> {
        let Some(&first) = bytes.first() else {
            return Err(KeyError::Empty);
        };
        if !is_key_start(first) {
            return Err(KeyError::InvalidStart {
                source: InvalidKeyStartError { byte: first },
            });
        }
        match bytes.iter().position(|&byte| !is_key_character(byte)) {
            Some(index) => Err(KeyError::InvalidCharacter {
                source: InvalidKeyCharacterError {
                    index,
                    byte: bytes[index],
                },
            }),
            None => Ok(Self(bytes)),
        }
    }

    /// As [`Key::new`], without copying `text`.
    ///
    /// # Errors
    ///
    /// As [`Key::new`].
    pub fn from_static(text: &'static str) -> Result<Self, KeyError> {
        Self::new(Bytes::from_static(text.as_bytes()))
    }

    /// The characters.
    pub fn as_str(&self) -> &str {
        // Invariant: every byte is ASCII, so the bytes are UTF-8.
        core::str::from_utf8(&self.0).expect("a key holds only ASCII")
    }

    /// The characters as [`Bytes`], without copying.
    pub fn into_bytes(self) -> Bytes {
        self.0
    }

    /// Parses a key (§4.2.3.3).
    pub(crate) fn parse(parser: &mut Parser) -> Result<Self, SyntaxError> {
        let start = parser.index();
        if !parser.peek().is_some_and(is_key_start) {
            return syntax::InvalidKeyStartSnafu { index: start }.fail();
        }
        parser.advance_while(is_key_character);
        Ok(Self(parser.slice_from(start)))
    }

    pub(crate) fn serialize(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.0);
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Debug for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Key({})", self.as_str())
    }
}

/// `lcalpha` or `*`.
const fn is_key_start(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte == b'*'
}

/// `lcalpha`, `DIGIT`, `_`, `-`, `.` or `*`.
const fn is_key_character(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-' | b'.' | b'*')
}

/// The bytes are not a key (RFC 9651 §3.1.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
pub enum KeyError {
    /// A key has at least one character.
    #[snafu(display("a key is empty"))]
    Empty,
    /// The first byte cannot start a key.
    #[snafu(display("a key starts with a byte that cannot start it"))]
    InvalidStart {
        /// Which byte.
        source: InvalidKeyStartError,
    },
    /// A later byte cannot be in a key.
    #[snafu(display("a key holds a byte that cannot be in it"))]
    InvalidCharacter {
        /// Which byte, and where.
        source: InvalidKeyCharacterError,
    },
}

/// The first byte of what should be a key is not a lowercase letter or `*`.
///
/// It is constructed only when the check fails, so its byte is never one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(display("byte {byte:#04x} is not a lowercase letter or `*`"))]
pub struct InvalidKeyStartError {
    byte: u8,
}

impl InvalidKeyStartError {
    /// The first byte, which is not a lowercase letter or `*`.
    pub const fn byte(self) -> u8 {
        self.byte
    }
}

/// A byte after the first one in what should be a key is not
/// a lowercase letter, a digit, `_`, `-`, `.` or `*`.
///
/// It is constructed only when the check fails, so its byte is never one and
/// its index is never 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(display(
    "byte {byte:#04x} at {index} is not a lowercase letter, a digit, `_`, `-`, `.` or `*`"
))]
pub struct InvalidKeyCharacterError {
    index: usize,
    byte: u8,
}

impl InvalidKeyCharacterError {
    /// Where the byte is, counted from the start; never 0.
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
