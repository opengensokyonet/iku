//! Absolute paths with an optional query (RFC 3986 §3.3, §3.4).

use crate::chars::{is_pct_encoded_at, is_sub_delim, is_unreserved};
use bytes::Bytes;
use core::fmt;
use snafu::Snafu;

/// An absolute path and an optional query: `absolute-path [ "?" query ]`
/// (RFC 9110 §4.1, RFC 3986 §3.3, §3.4).
///
/// It always starts with `/`: an `http` or `https` URI without a path has
/// the path `/` (RFC 9110 §4.2.3), and HTTP/2 and HTTP/3 send `/` in `:path`
/// for it ([RFC 9114 §4.3.1]). It holds no fragment.
///
/// [RFC 9114 §4.3.1]: https://www.rfc-editor.org/rfc/rfc9114.html#section-4.3.1
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PathAndQuery {
    bytes: Bytes,
    /// Where the `?` is, if there is a query.
    question: Option<usize>,
}

impl PathAndQuery {
    /// `/`.
    pub const ROOT: Self = Self {
        bytes: Bytes::from_static(b"/"),
        question: None,
    };

    /// Checks that `bytes` is an absolute path with an optional query.
    ///
    /// A path holds `pchar` and `/`; a query also holds `?`. A `pchar` is an
    /// unreserved character, a sub-delimiter, `:`, `@`, or a percent-encoding.
    ///
    /// # Errors
    ///
    /// [`PathAndQueryError`] if `bytes` does not start with `/`, holds a byte
    /// that neither a path nor a query can hold (including `#`), or holds a
    /// `%` that two hexadecimal digits do not follow.
    pub fn new(bytes: Bytes) -> Result<Self, PathAndQueryError> {
        if bytes.first() != Some(&b'/') {
            return NotAbsoluteSnafu.fail();
        }
        let mut question = None;
        let mut index = 0;
        while index < bytes.len() {
            let byte = bytes[index];
            if byte == b'%' {
                if !is_pct_encoded_at(&bytes, index) {
                    return InvalidPercentEncodingSnafu { index }.fail();
                }
                index += 3;
                continue;
            }
            if byte == b'?' {
                question.get_or_insert(index);
            } else if !(is_unreserved(byte)
                || is_sub_delim(byte)
                || matches!(byte, b':' | b'@' | b'/'))
            {
                return Err(PathAndQueryError::InvalidCharacter {
                    source: InvalidPathCharacterError { index, byte },
                });
            }
            index += 1;
        }
        Ok(Self { bytes, question })
    }

    /// As [`PathAndQuery::new`], without copying `text`.
    ///
    /// # Errors
    ///
    /// As [`PathAndQuery::new`].
    pub fn from_static(text: &'static str) -> Result<Self, PathAndQueryError> {
        Self::new(Bytes::from_static(text.as_bytes()))
    }

    /// The path and the query as written.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The path and the query as [`Bytes`], without copying.
    pub fn into_bytes(self) -> Bytes {
        self.bytes
    }

    /// The path, without the query.
    pub fn path(&self) -> &[u8] {
        &self.bytes[..self.question.unwrap_or(self.bytes.len())]
    }

    /// The query, without the `?`, if there is one.
    pub fn query(&self) -> Option<&[u8]> {
        self.question.map(|question| &self.bytes[question + 1..])
    }
}

impl fmt::Display for PathAndQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The bytes are ASCII.
        self.bytes
            .iter()
            .try_for_each(|&byte| fmt::Write::write_char(f, char::from(byte)))
    }
}

impl fmt::Debug for PathAndQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PathAndQuery({self})")
    }
}

/// The bytes are not an absolute path with an optional query.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum PathAndQueryError {
    /// The bytes do not start with `/`; an empty path is written `/`.
    #[snafu(display("a path does not start with a slash"))]
    NotAbsolute,
    /// A byte that neither a path nor a query can hold.
    #[snafu(display("a path or query holds a byte it cannot"))]
    InvalidCharacter {
        /// Which byte, and where.
        source: InvalidPathCharacterError,
    },
    /// A `%` is not followed by two hexadecimal digits.
    #[snafu(display("a percent sign at {index} is not followed by two hexadecimal digits"))]
    InvalidPercentEncoding {
        /// Where the `%` is.
        index: usize,
    },
}

/// A byte that neither a path nor a query can hold.
///
/// It is constructed only when the check fails.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(display("byte {byte:#04x} at {index} cannot be in a path or query"))]
pub struct InvalidPathCharacterError {
    index: usize,
    byte: u8,
}

impl InvalidPathCharacterError {
    /// Where the byte is.
    pub const fn index(self) -> usize {
        self.index
    }

    /// The byte.
    pub const fn byte(self) -> u8 {
        self.byte
    }
}
