//! Request methods (RFC 9110 §9).

use crate::token::{Token, TokenError};
use bytes::Bytes;
use core::fmt;

/// A request method: a token, case-sensitive (RFC 9110 §9.1).
///
/// The methods are an open set with a registry (§16.1); the eight that RFC
/// 9110 defines are constants. Whether a method is safe or idempotent is
/// given by its definition (§9.2.1, §9.2.2); a method that this type does not
/// know is neither.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Method(Token);

impl Method {
    /// `GET` (§9.3.1).
    pub const GET: Self = Self(Token::from_checked_static("GET"));
    /// `HEAD` (§9.3.2).
    pub const HEAD: Self = Self(Token::from_checked_static("HEAD"));
    /// `POST` (§9.3.3).
    pub const POST: Self = Self(Token::from_checked_static("POST"));
    /// `PUT` (§9.3.4).
    pub const PUT: Self = Self(Token::from_checked_static("PUT"));
    /// `DELETE` (§9.3.5).
    pub const DELETE: Self = Self(Token::from_checked_static("DELETE"));
    /// `CONNECT` (§9.3.6).
    pub const CONNECT: Self = Self(Token::from_checked_static("CONNECT"));
    /// `OPTIONS` (§9.3.7).
    pub const OPTIONS: Self = Self(Token::from_checked_static("OPTIONS"));
    /// `TRACE` (§9.3.8).
    pub const TRACE: Self = Self(Token::from_checked_static("TRACE"));

    /// Checks that `bytes` is a token.
    ///
    /// # Errors
    ///
    /// [`TokenError`] if `bytes` is not a token.
    pub fn new(bytes: Bytes) -> Result<Self, TokenError> {
        Token::new(bytes).map(Self)
    }

    /// As [`Method::new`], without copying `text`.
    ///
    /// # Errors
    ///
    /// As [`Method::new`].
    pub fn from_static(text: &'static str) -> Result<Self, TokenError> {
        Token::from_static(text).map(Self)
    }

    /// The method as a token.
    pub fn as_token(&self) -> &Token {
        &self.0
    }

    /// The bytes of the method.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Whether the method is safe: `GET`, `HEAD`, `OPTIONS` or `TRACE`
    /// (§9.2.1).
    pub fn is_safe(&self) -> bool {
        [Self::GET, Self::HEAD, Self::OPTIONS, Self::TRACE].contains(self)
    }

    /// Whether the method is idempotent: a safe method, `PUT` or `DELETE`
    /// (§9.2.2).
    pub fn is_idempotent(&self) -> bool {
        self.is_safe() || *self == Self::PUT || *self == Self::DELETE
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Debug for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Method({})", self.0)
    }
}

#[cfg(test)]
mod tests;
