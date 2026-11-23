//! The `Connection` field and connection-specific fields (RFC 9110 §7.6.1).
//!
//! An intermediary removes the connection-specific fields before it forwards
//! a message ([`remove_connection_specific`]). HTTP/2 and HTTP/3 treat a
//! message that holds them as malformed, `TE: trailers` excepted ([RFC 9113
//! §8.2.2], [RFC 9114 §4.2]); each version checks that with
//! [`is_connection_specific`] and [`crate::te`].
//!
//! [RFC 9113 §8.2.2]: https://www.rfc-editor.org/rfc/rfc9113.html#section-8.2.2
//! [RFC 9114 §4.2]: https://www.rfc-editor.org/rfc/rfc9114.html#section-4.2

use crate::field::list::members;
use crate::field::{FieldName, FieldSection};
use crate::token::{self, InvalidTokenCharacterError, Token};
use alloc::vec::Vec;
use bytes::Bytes;
use snafu::{ResultExt, Snafu};

/// The fields that are connection-specific whatever `Connection` lists:
/// `Connection` itself and the ones RFC 9110 §7.6.1 names, `Proxy-Connection`,
/// `Keep-Alive`, `TE`, `Transfer-Encoding` and `Upgrade`.
const CONNECTION_SPECIFIC: [FieldName; 6] = [
    FieldName::CONNECTION,
    FieldName::PROXY_CONNECTION,
    FieldName::KEEP_ALIVE,
    FieldName::TE,
    FieldName::TRANSFER_ENCODING,
    FieldName::UPGRADE,
];

/// Whether `name` is connection-specific whatever `Connection` lists:
/// `connection`, `proxy-connection`, `keep-alive`, `te`, `transfer-encoding`
/// or `upgrade` (RFC 9110 §7.6.1).
///
/// The fields that a `Connection` line lists are connection-specific too, in
/// that message.
pub fn is_connection_specific(name: &FieldName) -> bool {
    CONNECTION_SPECIFIC.contains(name)
}

/// The options of the `Connection` lines of `fields`, in order: their
/// non-empty members, each a token (RFC 9110 §7.6.1).
///
/// The tokens share the bytes of the field values. Options are
/// case-insensitive; compare them with [`Token::eq_ignore_ascii_case`]. A
/// member that is not a token is a [`ConnectionOptionError`].
pub fn options(
    fields: &FieldSection,
) -> impl Iterator<Item = Result<Token, ConnectionOptionError>> + '_ {
    let lines = fields
        .iter()
        .filter(|(name, _)| **name == FieldName::CONNECTION);
    lines.enumerate().flat_map(|(line, (_, value))| {
        let bytes = value.clone().into_bytes();
        // A member is never empty, so only its characters need checking.
        members(value.as_bytes())
            .enumerate()
            .map(move |(position, member)| {
                token::check_characters(member)
                    .map(|()| Token::from_checked(bytes.slice_ref(member)))
                    .context(ConnectionOptionSnafu {
                        line,
                        member: position,
                    })
            })
    })
}

/// Removes the connection-specific fields from `fields`, as an intermediary
/// does before it forwards a message (RFC 9110 §7.6.1): the fields named by an
/// option of a `Connection` line, `Connection` itself, and the fields that
/// [`is_connection_specific`] names.
///
/// Options are matched ignoring ASCII case. A member that is not a token
/// cannot name a field, so it removes nothing. RFC 9110 §7.6.1 lets an
/// intermediary add its own `Connection` afterwards.
///
/// The options come from the sender, and nothing here limits which fields
/// they name: a client that lists an end-to-end field, such as
/// `Authorization` or `X-Forwarded-For`, has an intermediary remove it.
/// RFC 9110 §7.6.1 says that a sender MUST NOT list a field that is meant for
/// every recipient on the chain; this function does not hold the sender to
/// that. Whether some fields are kept whatever the options say is for the
/// caller to decide.
///
/// The options are lowercased, sorted and deduplicated once, and each line is
/// looked up among them, so the time grows as `n log n` with the number of
/// options and lines rather than as their product.
pub fn remove_connection_specific(fields: &mut FieldSection) {
    // Field names are held in lowercase, so a lowercased option matches a
    // name ignoring ASCII case exactly when their bytes are equal.
    let mut options: Vec<Bytes> = options(fields)
        .filter_map(Result::ok)
        .map(|option| {
            let bytes = option.into_bytes();
            if bytes.iter().any(u8::is_ascii_uppercase) {
                Bytes::from(bytes.to_ascii_lowercase())
            } else {
                bytes
            }
        })
        .collect();
    options.sort_unstable();
    options.dedup();
    fields.retain(|name, _| {
        !is_connection_specific(name)
            && options
                .binary_search_by(|option| option.as_ref().cmp(name.as_bytes()))
                .is_err()
    });
}

/// A member of a `Connection` line is not a connection option: an option is
/// a token (RFC 9110 §7.6.1).
///
/// A member is never empty, so the only way to fail is a byte that is not a
/// `tchar`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(display("connection option {member} of line {line} is not a token"))]
pub struct ConnectionOptionError {
    source: InvalidTokenCharacterError,
    line: usize,
    member: usize,
}

impl ConnectionOptionError {
    /// Which `Connection` line, counting the `Connection` lines from 0.
    pub const fn line(self) -> usize {
        self.line
    }

    /// Which member of the line, counting its non-empty members from 0.
    pub const fn member(self) -> usize {
        self.member
    }

    /// The byte that is not a `tchar`, and where it is in the member.
    pub const fn character_error(self) -> InvalidTokenCharacterError {
        self.source
    }
}

#[cfg(test)]
mod tests;
