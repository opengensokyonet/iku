//! Schemes (RFC 3986 §3.1).

use bytes::Bytes;
use core::fmt;
use snafu::Snafu;

/// A URI scheme, in lowercase (RFC 3986 §3.1).
///
/// Schemes are case-insensitive and their canonical form is lowercase, so
/// construction lowercases them.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Scheme(Bytes);

impl Scheme {
    /// `http` (RFC 9110 §4.2.1).
    pub const HTTP: Self = Self(Bytes::from_static(b"http"));
    /// `https` (RFC 9110 §4.2.2).
    pub const HTTPS: Self = Self(Bytes::from_static(b"https"));

    /// Checks that `bytes` is a scheme, `ALPHA *( ALPHA / DIGIT / "+" / "-" /
    /// "." )`, and lowercases it; the bytes are copied only when they hold an
    /// uppercase letter.
    ///
    /// # Errors
    ///
    /// [`SchemeError`] if `bytes` is empty, starts with something other than a
    /// letter, or holds a byte that a scheme cannot.
    pub fn new(bytes: Bytes) -> Result<Self, SchemeError> {
        let Some(&first) = bytes.first() else {
            return EmptySnafu.fail();
        };
        if !first.is_ascii_alphabetic() {
            return Err(SchemeError::InvalidCharacter {
                source: InvalidSchemeCharacterError {
                    index: 0,
                    byte: first,
                },
            });
        }
        if let Some(index) = bytes
            .iter()
            .position(|&b| !(b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.')))
        {
            return Err(SchemeError::InvalidCharacter {
                source: InvalidSchemeCharacterError {
                    index,
                    byte: bytes[index],
                },
            });
        }
        if bytes.iter().any(u8::is_ascii_uppercase) {
            Ok(Self(Bytes::from(bytes.to_ascii_lowercase())))
        } else {
            Ok(Self(bytes))
        }
    }

    /// As [`Scheme::new`], without copying `text` when it is lowercase.
    ///
    /// # Errors
    ///
    /// As [`Scheme::new`].
    pub fn from_static(text: &'static str) -> Result<Self, SchemeError> {
        Self::new(Bytes::from_static(text.as_bytes()))
    }

    /// The bytes of the scheme, in lowercase.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Whether the scheme is `http` or `https`, whose URIs need a non-empty
    /// host (RFC 9110 §4.2.1, §4.2.2).
    pub fn is_http_or_https(&self) -> bool {
        *self == Self::HTTP || *self == Self::HTTPS
    }
}

impl fmt::Display for Scheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // A scheme is ASCII.
        self.0
            .iter()
            .try_for_each(|&byte| fmt::Write::write_char(f, char::from(byte)))
    }
}

impl fmt::Debug for Scheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Scheme({self})")
    }
}

/// The bytes are not a scheme (RFC 3986 §3.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum SchemeError {
    /// A scheme has at least one character.
    #[snafu(display("a scheme is empty"))]
    Empty,
    /// A byte is not allowed where it is.
    #[snafu(display("a scheme holds a byte it cannot"))]
    InvalidCharacter {
        /// Which byte, and where.
        source: InvalidSchemeCharacterError,
    },
}

/// A byte that a scheme cannot hold where it is: the first is a letter, the
/// others are letters, digits, `+`, `-` or `.`.
///
/// It is constructed only when the check fails.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(display("byte {byte:#04x} at {index} cannot be in a scheme there"))]
pub struct InvalidSchemeCharacterError {
    index: usize,
    byte: u8,
}

impl InvalidSchemeCharacterError {
    /// Where the byte is; 0 when the first byte is not a letter.
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
