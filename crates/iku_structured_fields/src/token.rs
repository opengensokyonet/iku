//! Tokens (RFC 9651 §3.3.4).

use crate::syntax::Parser;
use alloc::vec::Vec;
use bytes::Bytes;
use core::fmt;
use snafu::Snafu;

/// A token of a structured field (RFC 9651 §3.3.4): a letter or `*`, then
/// `tchar`s, `:` and `/`.
///
/// It is not the token of RFC 9110 §5.6.2: it cannot start with a digit or
/// most symbols, and it can hold `:` and `/`.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SfToken(Bytes);

impl SfToken {
    /// Checks that `bytes` is a token.
    ///
    /// # Errors
    ///
    /// [`SfTokenError`] for the rule that `bytes` breaks.
    pub fn new(bytes: Bytes) -> Result<Self, SfTokenError> {
        let Some(&first) = bytes.first() else {
            return Err(SfTokenError::Empty);
        };
        if !is_token_start(first) {
            return Err(SfTokenError::InvalidStart {
                source: InvalidSfTokenStartError { byte: first },
            });
        }
        match bytes.iter().position(|&byte| !is_token_character(byte)) {
            Some(index) => Err(SfTokenError::InvalidCharacter {
                source: InvalidSfTokenCharacterError {
                    index,
                    byte: bytes[index],
                },
            }),
            None => Ok(Self(bytes)),
        }
    }

    /// As [`SfToken::new`], without copying `text`.
    ///
    /// # Errors
    ///
    /// As [`SfToken::new`].
    pub fn from_static(text: &'static str) -> Result<Self, SfTokenError> {
        Self::new(Bytes::from_static(text.as_bytes()))
    }

    /// The characters.
    pub fn as_str(&self) -> &str {
        // Invariant: every byte is ASCII, so the bytes are UTF-8.
        core::str::from_utf8(&self.0).expect("a token holds only ASCII")
    }

    /// The characters as [`Bytes`], without copying.
    pub fn into_bytes(self) -> Bytes {
        self.0
    }

    /// Parses a token (§4.2.6); the parser is at a letter or `*`.
    pub(crate) fn parse(parser: &mut Parser) -> Self {
        let start = parser.index();
        parser.advance();
        parser.advance_while(is_token_character);
        Self(parser.slice_from(start))
    }

    pub(crate) fn serialize(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.0);
    }
}

impl fmt::Display for SfToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Debug for SfToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SfToken({})", self.as_str())
    }
}

/// A byte that can start a token: `ALPHA` or `*`.
pub(crate) const fn is_token_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'*'
}

/// A byte that can follow the first one: `tchar` (RFC 9110 §5.6.2), `:` or
/// `/`.
const fn is_token_character(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'^'
                | b'_'
                | b'`'
                | b'|'
                | b'~'
                | b':'
                | b'/'
        )
}

/// The bytes are not a token (RFC 9651 §3.3.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
pub enum SfTokenError {
    /// A token has at least one character.
    #[snafu(display("a token is empty"))]
    Empty,
    /// The first byte cannot start a token.
    #[snafu(display("a token starts with a byte that cannot start it"))]
    InvalidStart {
        /// Which byte.
        source: InvalidSfTokenStartError,
    },
    /// A later byte cannot be in a token.
    #[snafu(display("a token holds a byte that cannot be in it"))]
    InvalidCharacter {
        /// Which byte, and where.
        source: InvalidSfTokenCharacterError,
    },
}

/// The first byte of what should be a token is not a letter or `*`.
///
/// It is constructed only when the check fails, so its byte is never one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(display("byte {byte:#04x} is not a letter or `*`"))]
pub struct InvalidSfTokenStartError {
    byte: u8,
}

impl InvalidSfTokenStartError {
    /// The first byte, which is not a letter or `*`.
    pub const fn byte(self) -> u8 {
        self.byte
    }
}

/// A byte after the first one in what should be a token is not
/// a `tchar`, `:` or `/`.
///
/// It is constructed only when the check fails, so its byte is never one and
/// its index is never 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(display("byte {byte:#04x} at {index} is not a `tchar`, `:` or `/`"))]
pub struct InvalidSfTokenCharacterError {
    index: usize,
    byte: u8,
}

impl InvalidSfTokenCharacterError {
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
