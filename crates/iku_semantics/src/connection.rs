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
/// case-insensitive; compare them with [`Token::eq_ignore_ascii_case`].
pub fn options(
    fields: &FieldSection,
) -> impl Iterator<Item = Result<Token, InvalidTokenCharacterError>> + '_ {
    let lines = fields
        .iter()
        .filter(|(name, _)| **name == FieldName::CONNECTION);
    lines.flat_map(|(_, value)| {
        let bytes = value.clone().into_bytes();
        // A member is never empty, so only its characters need checking.
        members(value.as_bytes()).map(move |member| {
            token::check_characters(member).map(|()| Token::from_checked(bytes.slice_ref(member)))
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
pub fn remove_connection_specific(fields: &mut FieldSection) {
    let options: Vec<Token> = options(fields).filter_map(Result::ok).collect();
    fields.retain(|name, _| {
        !is_connection_specific(name)
            && !options
                .iter()
                .any(|option| option.eq_ignore_ascii_case(name.as_bytes()))
    });
}

#[cfg(test)]
mod tests;
