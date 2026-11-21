//! Tokens (RFC 9110 §5.6.2).

use crate::chars::is_tchar;
use bytes::Bytes;
use core::fmt;
use snafu::{ResultExt, Snafu};

/// A token: one or more `tchar` (RFC 9110 §5.6.2).
///
/// Methods, field names and connection options are tokens. A token is ASCII.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Token(Bytes);

impl Token {
    /// Checks that `bytes` is a token.
    ///
    /// # Errors
    ///
    /// [`TokenError`] if `bytes` is empty or holds a byte that is not a
    /// `tchar`.
    pub fn new(bytes: Bytes) -> Result<Self, TokenError> {
        check(&bytes)?;
        Ok(Self(bytes))
    }

    /// Checks that `text` is a token, without copying it.
    ///
    /// # Errors
    ///
    /// As [`Token::new`].
    pub fn from_static(text: &'static str) -> Result<Self, TokenError> {
        Self::new(Bytes::from_static(text.as_bytes()))
    }

    /// A token from a constant that the tests of this package check.
    pub(crate) const fn from_checked_static(text: &'static str) -> Self {
        Self(Bytes::from_static(text.as_bytes()))
    }

    /// A token from bytes that were checked to be one.
    pub(crate) fn from_checked(bytes: Bytes) -> Self {
        debug_assert!(check(&bytes).is_ok());
        Self(bytes)
    }

    /// The bytes of the token.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// The token as [`Bytes`], without copying.
    pub fn into_bytes(self) -> Bytes {
        self.0
    }

    /// Whether `self` and `other` are the same token ignoring ASCII case, as
    /// for connection options (RFC 9110 §7.6.1) and transfer codings
    /// (§10.1.4).
    pub fn eq_ignore_ascii_case(&self, other: &[u8]) -> bool {
        self.0.eq_ignore_ascii_case(other)
    }
}

/// Checks the bytes of a token.
pub(crate) fn check(bytes: &[u8]) -> Result<(), TokenError> {
    if bytes.is_empty() {
        return EmptySnafu.fail();
    }
    check_characters(bytes).context(InvalidCharacterSnafu)
}

/// Checks that every byte is a `tchar`, for bytes known not to be empty.
pub(crate) fn check_characters(bytes: &[u8]) -> Result<(), InvalidTokenCharacterError> {
    match bytes.iter().position(|&byte| !is_tchar(byte)) {
        Some(index) => Err(InvalidTokenCharacterError {
            index,
            byte: bytes[index],
        }),
        None => Ok(()),
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // A token is ASCII, so every byte is a character.
        self.0
            .iter()
            .try_for_each(|&byte| fmt::Write::write_char(f, char::from(byte)))
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Token({self})")
    }
}

/// The bytes are not a token (RFC 9110 §5.6.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum TokenError {
    /// A token has at least one character.
    #[snafu(display("a token is empty"))]
    Empty,
    /// A byte is not a `tchar`.
    #[snafu(display("a token holds a byte that is not a tchar"))]
    InvalidCharacter {
        /// Which byte, and where.
        source: InvalidTokenCharacterError,
    },
}

/// A byte that is not a `tchar` in what should be a token.
///
/// It is constructed only when the check fails, so its byte is never a
/// `tchar`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(display("byte {byte:#04x} at {index} is not a tchar"))]
pub struct InvalidTokenCharacterError {
    index: usize,
    byte: u8,
}

impl InvalidTokenCharacterError {
    /// Where the byte is, counted from the start of the token.
    pub const fn index(self) -> usize {
        self.index
    }

    /// The byte, which is not a `tchar`.
    pub const fn byte(self) -> u8 {
        self.byte
    }
}

#[cfg(test)]
mod tests;
