//! The protocol of an extended CONNECT request ([RFC 8441] §4, [RFC 9220]).
//!
//! [RFC 8441]: https://www.rfc-editor.org/rfc/rfc8441.html
//! [RFC 9220]: https://www.rfc-editor.org/rfc/rfc9220.html

use crate::token::{self, TokenError};
use bytes::Bytes;
use core::fmt;
use snafu::{ResultExt, Snafu};

/// The value of the `:protocol` pseudo-header field of an extended CONNECT
/// request: an upgrade token, `protocol-name [ "/" protocol-version ]`, both
/// tokens (RFC 8441 §4, RFC 9110 §7.8), such as `websocket` or
/// `webtransport`.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Protocol {
    bytes: Bytes,
    /// Where the `/` is, if there is a version.
    slash: Option<usize>,
}

impl Protocol {
    /// Checks that `bytes` is an upgrade token.
    ///
    /// # Errors
    ///
    /// [`ProtocolError`] if the name, or the version after a `/`, is not a
    /// token.
    pub fn new(bytes: Bytes) -> Result<Self, ProtocolError> {
        let slash = bytes.iter().position(|&b| b == b'/');
        let (name, version) = match slash {
            Some(slash) => (&bytes[..slash], Some(&bytes[slash + 1..])),
            None => (&bytes[..], None),
        };
        token::check(name).context(NameSnafu)?;
        if let Some(version) = version {
            token::check(version).context(VersionSnafu)?;
        }
        Ok(Self { bytes, slash })
    }

    /// As [`Protocol::new`], without copying `text`.
    ///
    /// # Errors
    ///
    /// As [`Protocol::new`].
    pub fn from_static(text: &'static str) -> Result<Self, ProtocolError> {
        Self::new(Bytes::from_static(text.as_bytes()))
    }

    /// The protocol as written.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The name of the protocol.
    pub fn name(&self) -> &[u8] {
        &self.bytes[..self.slash.unwrap_or(self.bytes.len())]
    }

    /// The version of the protocol, if there is one.
    pub fn version(&self) -> Option<&[u8]> {
        self.slash.map(|slash| &self.bytes[slash + 1..])
    }
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Tokens and `/` are ASCII.
        self.bytes
            .iter()
            .try_for_each(|&byte| fmt::Write::write_char(f, char::from(byte)))
    }
}

impl fmt::Debug for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Protocol({self})")
    }
}

/// The bytes are not an upgrade token (RFC 9110 §7.8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum ProtocolError {
    /// The name is not a token.
    #[snafu(display("the protocol name is not a token"))]
    Name {
        /// Why.
        source: TokenError,
    },
    /// The version after `/` is not a token.
    #[snafu(display("the protocol version is not a token"))]
    Version {
        /// Why.
        source: TokenError,
    },
}

#[cfg(test)]
mod tests;
