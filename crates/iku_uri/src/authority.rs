//! Authorities (RFC 3986 §3.2) as HTTP uses them.

use crate::chars::{
    InvalidPercentEncodingError, check_pct_encoded_at, is_pct_encoded_at, is_sub_delim,
    is_unreserved,
};
use crate::scheme::Scheme;
use alloc::vec::Vec;
use bytes::{BufMut, Bytes, BytesMut};
use core::fmt::{self, Write};
use core::net::{Ipv4Addr, Ipv6Addr};
use core::str::FromStr;
use snafu::Snafu;

/// An authority: a host and an optional port (RFC 3986 §3.2), as the target
/// URI of an `http` or `https` request, the target of CONNECT and the `Host`
/// field carry it.
///
/// It holds no userinfo: a sender must not generate it in an `http` or
/// `https` URI and a recipient should treat it as an error (RFC 9110 §4.2.4).
/// The host is never empty (§4.2.1). Hosts are case-insensitive (RFC 3986
/// §3.2.2), so construction lowercases them; an empty port is the same as no
/// port (§3.2.3), and two authorities are equal when their hosts and port
/// numbers are.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Authority {
    /// The host in lowercase; an IP literal keeps its brackets.
    host: Bytes,
    kind: HostKind,
    port: Option<u16>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum HostKind {
    RegName,
    Ipv4(Ipv4Addr),
    Ipv6(Ipv6Addr),
    IpFuture,
}

/// The host of an [`Authority`] (RFC 3986 §3.2.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Host<'a> {
    /// A registered name, such as a DNS name, in lowercase, possibly
    /// percent-encoded.
    RegName(&'a [u8]),
    /// An IPv4 address.
    Ipv4(Ipv4Addr),
    /// An IPv6 address, written in brackets.
    Ipv6(Ipv6Addr),
    /// An `IPvFuture` literal, without its brackets, in lowercase.
    IpFuture(&'a [u8]),
}

impl Authority {
    /// Reads an authority, `host [ ":" port ]`.
    ///
    /// The host is an IP literal in brackets (an IPv6 address or an
    /// `IPvFuture`), an IPv4 address, or a registered name; text that matches
    /// the IPv4 grammar is an IPv4 address, other text a registered name
    /// (RFC 3986 §3.2.2).
    ///
    /// # Errors
    ///
    /// [`AuthorityError`] for the rule that `bytes` breaks.
    pub fn new(bytes: &[u8]) -> Result<Self, AuthorityError> {
        if let Some(index) = bytes.iter().position(|&b| b == b'@') {
            return Err(AuthorityError::Userinfo {
                source: UserinfoNotAllowedError { index },
            });
        }
        let (host, kind, port) = if bytes.first() == Some(&b'[') {
            let close = bytes
                .iter()
                .position(|&b| b == b']')
                .ok_or(AuthorityError::UnclosedIpLiteral)?;
            let inner = &bytes[1..close];
            let kind = ip_literal(inner, close)?;
            let port = match &bytes[close + 1..] {
                [] => &[][..],
                [b':', port @ ..] => port,
                _ => {
                    return Err(AuthorityError::AfterIpLiteral {
                        source: TextAfterIpLiteralError { index: close + 1 },
                    });
                }
            };
            (&bytes[..=close], kind, port_at(port, close + 2)?)
        } else {
            let (host, port, port_start) = match bytes.iter().position(|&b| b == b':') {
                Some(colon) => (&bytes[..colon], &bytes[colon + 1..], colon + 1),
                None => (bytes, &[][..], bytes.len()),
            };
            if host.is_empty() {
                return EmptyHostSnafu.fail();
            }
            (host, reg_name_or_ipv4(host)?, port_at(port, port_start)?)
        };
        Ok(Self {
            host: Bytes::from(host.to_ascii_lowercase()),
            kind,
            port,
        })
    }

    /// The host.
    pub fn host(&self) -> Host<'_> {
        match self.kind {
            HostKind::RegName => Host::RegName(&self.host),
            HostKind::Ipv4(address) => Host::Ipv4(address),
            HostKind::Ipv6(address) => Host::Ipv6(address),
            HostKind::IpFuture => Host::IpFuture(&self.host[1..self.host.len() - 1]),
        }
    }

    /// The bytes of the host as written in the authority, in lowercase; an
    /// IP literal with its brackets.
    pub fn host_bytes(&self) -> &[u8] {
        &self.host
    }

    /// Whether `self` and `other` are the same authority in a URI of
    /// `scheme`, compared after the normalizations of RFC 3986 §6.2.2 and
    /// §6.2.3, as comparing `Host` with `:authority` needs (RFC 9113 §8.3.1,
    /// RFC 9114 §4.3.1).
    ///
    /// No port, and the default port of the scheme, are the same
    /// ([`Scheme::default_port`]). In a registered name, a percent-encoded
    /// unreserved character is the same as the character (§6.2.2.2); case
    /// is already ignored, since hosts are held in lowercase. IP addresses
    /// are compared as addresses.
    ///
    /// `==` compares the hosts and the port numbers as they are, without a
    /// scheme.
    pub fn eq_normalized(&self, other: &Self, scheme: &Scheme) -> bool {
        let default_port = scheme.default_port();
        if self.port.or(default_port) != other.port.or(default_port) {
            return false;
        }
        match (self.kind, other.kind) {
            (HostKind::Ipv4(a), HostKind::Ipv4(b)) => a == b,
            (HostKind::Ipv6(a), HostKind::Ipv6(b)) => a == b,
            (HostKind::RegName, HostKind::RegName) | (HostKind::IpFuture, HostKind::IpFuture) => {
                decode_unreserved(&self.host) == decode_unreserved(&other.host)
            }
            _ => false,
        }
    }

    /// The port, if the authority has a non-empty one.
    pub const fn port(&self) -> Option<u16> {
        self.port
    }

    /// The authority written out: the host, then `:` and the port in decimal
    /// if it has one.
    pub fn to_bytes(&self) -> Bytes {
        match self.port {
            None => self.host.clone(),
            Some(port) => {
                let mut bytes = BytesMut::with_capacity(self.host.len() + 6);
                bytes.put_slice(&self.host);
                bytes.put_u8(b':');
                // Invariant: writing to a `BytesMut` grows it and never
                // fails.
                write!(bytes, "{port}").expect("a BytesMut takes any text");
                bytes.freeze()
            }
        }
    }
}

/// The kind of an IP literal's content, `close` being the index of `]`.
fn ip_literal(inner: &[u8], close: usize) -> Result<HostKind, AuthorityError> {
    if let [b'v' | b'V', rest @ ..] = inner {
        // IPvFuture = "v" 1*HEXDIG "." 1*( unreserved / sub-delims / ":" )
        let dot = rest.iter().position(|&b| b == b'.');
        let valid = dot.is_some_and(|dot| {
            let (version, address) = (&rest[..dot], &rest[dot + 1..]);
            !version.is_empty()
                && version.iter().all(u8::is_ascii_hexdigit)
                && !address.is_empty()
                && address
                    .iter()
                    .all(|&b| is_unreserved(b) || is_sub_delim(b) || b == b':')
        });
        return if valid {
            Ok(HostKind::IpFuture)
        } else {
            Err(AuthorityError::InvalidIpFuture {
                source: InvalidIpFutureError { close },
            })
        };
    }
    core::str::from_utf8(inner)
        .ok()
        .and_then(|text| Ipv6Addr::from_str(text).ok())
        .map(HostKind::Ipv6)
        .ok_or(AuthorityError::InvalidIpv6)
}

fn reg_name_or_ipv4(host: &[u8]) -> Result<HostKind, AuthorityError> {
    if host.iter().all(|&b| b.is_ascii_digit() || b == b'.')
        && let Some(address) = core::str::from_utf8(host)
            .ok()
            .and_then(|t| Ipv4Addr::from_str(t).ok())
    {
        return Ok(HostKind::Ipv4(address));
    }
    let mut index = 0;
    while index < host.len() {
        let byte = host[index];
        if byte == b'%' {
            check_pct_encoded_at(host, index)
                .map_err(|source| AuthorityError::InvalidPercentEncoding { source })?;
            index += 3;
        } else if is_unreserved(byte) || is_sub_delim(byte) {
            index += 1;
        } else {
            return Err(AuthorityError::InvalidRegNameCharacter {
                source: InvalidRegNameCharacterError { index, byte },
            });
        }
    }
    Ok(HostKind::RegName)
}

/// The port number of `digits`, which starts at `start` in the authority.
fn port_at(digits: &[u8], start: usize) -> Result<Option<u16>, AuthorityError> {
    if let Some(offset) = digits.iter().position(|b| !b.is_ascii_digit()) {
        return Err(AuthorityError::InvalidPortCharacter {
            source: InvalidPortCharacterError {
                index: start + offset,
                byte: digits[offset],
            },
        });
    }
    if digits.is_empty() {
        return Ok(None);
    }
    let mut port: u32 = 0;
    for &digit in digits {
        port = port * 10 + u32::from(digit - b'0');
        if port > u32::from(u16::MAX) {
            return Err(AuthorityError::PortOutOfRange {
                source: PortOutOfRangeError { index: start },
            });
        }
    }
    // The loop stops above u16::MAX.
    Ok(u16::try_from(port).ok())
}

impl fmt::Display for Authority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.host.escape_ascii())?;
        if let Some(port) = self.port {
            write!(f, ":{port}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for Authority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Authority({self})")
    }
}

/// `host`, which is checked and in lowercase, with each percent-encoded
/// unreserved character decoded (RFC 3986 §6.2.2.2) and in lowercase.
fn decode_unreserved(host: &[u8]) -> Vec<u8> {
    let mut decoded = Vec::with_capacity(host.len());
    let mut index = 0;
    while index < host.len() {
        if host[index] == b'%' && is_pct_encoded_at(host, index) {
            // invariant: `is_pct_encoded_at` checked two hexadecimal digits.
            let byte = (hex_value(host[index + 1]) << 4) | hex_value(host[index + 2]);
            if is_unreserved(byte) {
                decoded.push(byte.to_ascii_lowercase());
            } else {
                decoded.extend_from_slice(&host[index..index + 3]);
            }
            index += 3;
        } else {
            decoded.push(host[index]);
            index += 1;
        }
    }
    decoded
}

/// The value of a hexadecimal digit.
const fn hex_value(digit: u8) -> u8 {
    match digit {
        b'0'..=b'9' => digit - b'0',
        b'a'..=b'f' => digit - b'a' + 10,
        _ => digit - b'A' + 10,
    }
}

/// The bytes are not an authority that HTTP accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum AuthorityError {
    /// The authority holds userinfo (`@`), which RFC 9110 §4.2.4 deprecates.
    #[snafu(display("an authority holds userinfo"))]
    Userinfo {
        /// Where the first `@` is.
        source: UserinfoNotAllowedError,
    },
    /// The host is empty (RFC 9110 §4.2.1).
    #[snafu(display("an authority has an empty host"))]
    EmptyHost,
    /// An IP literal has no closing bracket.
    #[snafu(display("an IP literal has no closing bracket"))]
    UnclosedIpLiteral,
    /// An IP literal holds neither an IPv6 address nor an `IPvFuture`.
    #[snafu(display("an IP literal is not an IPv6 address"))]
    InvalidIpv6,
    /// An IP literal that starts with `v` is not an `IPvFuture`.
    #[snafu(display("an IP literal is not a valid IPvFuture"))]
    InvalidIpFuture {
        /// Where the literal ends.
        source: InvalidIpFutureError,
    },
    /// Something other than `:` and a port follows an IP literal.
    #[snafu(display("an IP literal is followed by something other than a port"))]
    AfterIpLiteral {
        /// Where the offending text starts.
        source: TextAfterIpLiteralError,
    },
    /// A byte that a registered name cannot hold.
    #[snafu(display("a registered name holds a byte it cannot"))]
    InvalidRegNameCharacter {
        /// Which byte, and where.
        source: InvalidRegNameCharacterError,
    },
    /// A `%` is not followed by two hexadecimal digits.
    #[snafu(display("a registered name holds a malformed percent-encoding"))]
    InvalidPercentEncoding {
        /// Where the `%` is in the authority.
        source: InvalidPercentEncodingError,
    },
    /// A port holds something other than digits.
    #[snafu(display("a port holds a byte that is not a digit"))]
    InvalidPortCharacter {
        /// Which byte, and where.
        source: InvalidPortCharacterError,
    },
    /// The port is above 65535.
    #[snafu(display("a port is above 65535"))]
    PortOutOfRange {
        /// Where the port starts.
        source: PortOutOfRangeError,
    },
}

/// An authority holds userinfo (`@`), which RFC 9110 §4.2.4 deprecates.
///
/// It is constructed only when the check fails.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(display("userinfo at {index}: an authority holds an `@`"))]
#[snafu(context(suffix(false)))]
pub struct UserinfoNotAllowedError {
    index: usize,
}

impl UserinfoNotAllowedError {
    /// Where the first `@` is in the authority.
    pub const fn index(self) -> usize {
        self.index
    }
}

/// Something other than `:` and a port follows the closing bracket of an IP
/// literal.
///
/// It is constructed only when the check fails.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(display("an IP literal is followed at {index} by something other than a port"))]
#[snafu(context(suffix(false)))]
pub struct TextAfterIpLiteralError {
    index: usize,
}

impl TextAfterIpLiteralError {
    /// Where the text after the closing bracket starts in the authority.
    pub const fn index(self) -> usize {
        self.index
    }
}

/// A port is above 65535, the largest number that fits in two octets.
///
/// It is constructed only when the check fails.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(display("the port at {index} is above 65535"))]
#[snafu(context(suffix(false)))]
pub struct PortOutOfRangeError {
    index: usize,
}

impl PortOutOfRangeError {
    /// Where the first digit of the port is in the authority.
    pub const fn index(self) -> usize {
        self.index
    }
}

/// An `IPvFuture` literal breaks its grammar, `"v" 1*HEXDIG "." 1*(
/// unreserved / sub-delims / ":" )`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(display("the IPvFuture literal ending at {close} is not valid"))]
#[snafu(context(suffix(false)))]
pub struct InvalidIpFutureError {
    close: usize,
}

impl InvalidIpFutureError {
    /// Where the closing bracket of the literal is.
    pub const fn close(self) -> usize {
        self.close
    }
}

/// A byte that a registered name cannot hold: one that is neither unreserved,
/// a sub-delimiter, nor the start of a percent-encoding (RFC 3986 §3.2.2).
///
/// It is constructed only when the check fails.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(display("byte {byte:#04x} at {index} cannot be in a registered name"))]
#[snafu(context(suffix(false)))]
pub struct InvalidRegNameCharacterError {
    index: usize,
    byte: u8,
}

impl InvalidRegNameCharacterError {
    /// Where the byte is in the authority.
    pub const fn index(self) -> usize {
        self.index
    }

    /// The byte.
    pub const fn byte(self) -> u8 {
        self.byte
    }
}

/// A byte in a port that is not a digit.
///
/// It is constructed only when the check fails, so its byte is never a digit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(display("byte {byte:#04x} at {index} in a port is not a digit"))]
#[snafu(context(suffix(false)))]
pub struct InvalidPortCharacterError {
    index: usize,
    byte: u8,
}

impl InvalidPortCharacterError {
    /// Where the byte is in the authority.
    pub const fn index(self) -> usize {
        self.index
    }

    /// The byte, not a digit.
    pub const fn byte(self) -> u8 {
        self.byte
    }
}

#[cfg(test)]
mod tests;
