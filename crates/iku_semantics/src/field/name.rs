//! Field names (RFC 9110 §5.1).

use crate::token::{self, TokenError};
use bytes::Bytes;
use core::fmt;

/// A field name: a token, held in lowercase (RFC 9110 §5.1).
///
/// Field names are case-insensitive, and HTTP/2 and HTTP/3 send them in
/// lowercase ([RFC 9113 §8.2], [RFC 9114 §4.2]), so construction lowercases
/// the ASCII letters and two field names are the same name exactly when their
/// bytes are equal. An HTTP/1.1 writer that wants another case writes it out
/// in that case; the value keeps no other case.
///
/// [RFC 9113 §8.2]: https://www.rfc-editor.org/rfc/rfc9113.html#section-8.2
/// [RFC 9114 §4.2]: https://www.rfc-editor.org/rfc/rfc9114.html#section-4.2
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FieldName(Bytes);

impl FieldName {
    /// `connection` (RFC 9110 §7.6.1).
    pub const CONNECTION: Self = Self::from_checked_static("connection");
    /// `content-length` (RFC 9110 §8.6).
    pub const CONTENT_LENGTH: Self = Self::from_checked_static("content-length");
    /// `host` (RFC 9110 §7.2).
    pub const HOST: Self = Self::from_checked_static("host");
    /// `keep-alive` (RFC 2068 §19.7.1), removed before forwarding (RFC 9110
    /// §7.6.1).
    pub const KEEP_ALIVE: Self = Self::from_checked_static("keep-alive");
    /// `proxy-connection` (RFC 9112 Appendix C.2.2), removed before forwarding
    /// (RFC 9110 §7.6.1).
    pub const PROXY_CONNECTION: Self = Self::from_checked_static("proxy-connection");
    /// `set-cookie` ([RFC 6265]), whose lines cannot be combined (RFC 9110
    /// §5.3).
    ///
    /// [RFC 6265]: https://www.rfc-editor.org/rfc/rfc6265.html
    pub const SET_COOKIE: Self = Self::from_checked_static("set-cookie");
    /// `te` (RFC 9110 §10.1.4).
    pub const TE: Self = Self::from_checked_static("te");
    /// `transfer-encoding` (RFC 9112 §6.1).
    pub const TRANSFER_ENCODING: Self = Self::from_checked_static("transfer-encoding");
    /// `upgrade` (RFC 9110 §7.8).
    pub const UPGRADE: Self = Self::from_checked_static("upgrade");

    /// Checks that `bytes` is a token and lowercases it.
    ///
    /// The bytes are copied only when they hold an uppercase letter.
    ///
    /// # Errors
    ///
    /// [`TokenError`] if `bytes` is not a token.
    pub fn new(bytes: Bytes) -> Result<Self, TokenError> {
        token::check(&bytes)?;
        if bytes.iter().any(u8::is_ascii_uppercase) {
            Ok(Self(Bytes::from(bytes.to_ascii_lowercase())))
        } else {
            Ok(Self(bytes))
        }
    }

    /// As [`FieldName::new`], without copying `text` when it is already
    /// lowercase.
    ///
    /// # Errors
    ///
    /// As [`FieldName::new`].
    pub fn from_static(text: &'static str) -> Result<Self, TokenError> {
        Self::new(Bytes::from_static(text.as_bytes()))
    }

    /// A name from a lowercase constant that the tests of this package check.
    const fn from_checked_static(text: &'static str) -> Self {
        Self(Bytes::from_static(text.as_bytes()))
    }

    /// The bytes of the name, in lowercase.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// The name as [`Bytes`], without copying.
    pub fn into_bytes(self) -> Bytes {
        self.0
    }
}

impl fmt::Display for FieldName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // A field name is a token, so every byte is an ASCII character.
        self.0
            .iter()
            .try_for_each(|&byte| fmt::Write::write_char(f, char::from(byte)))
    }
}

impl fmt::Debug for FieldName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FieldName({self})")
    }
}

#[cfg(test)]
mod tests;
