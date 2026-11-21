//! The `TE` field (RFC 9110 §10.1.4).
//!
//! HTTP/2 and HTTP/3 allow `TE` in a request only when it holds no member
//! other than `trailers` ([RFC 9113 §8.2.2], [RFC 9114 §4.2]); each version
//! checks that with [`has_member_other_than_trailers`].
//!
//! [RFC 9113 §8.2.2]: https://www.rfc-editor.org/rfc/rfc9113.html#section-8.2.2
//! [RFC 9114 §4.2]: https://www.rfc-editor.org/rfc/rfc9114.html#section-4.2

use crate::field::{FieldName, FieldSection};

/// Whether a `TE` line of `fields` holds a member other than `trailers`.
///
/// `trailers` is compared ignoring ASCII case, as a literal of the grammar is
/// (RFC 9110 §10.1.4, [RFC 5234 §2.3]); a member with a weight or parameters
/// is another member. Without `TE` lines, or with only empty ones, it is false.
///
/// [RFC 5234 §2.3]: https://www.rfc-editor.org/rfc/rfc5234.html#section-2.3
pub fn has_member_other_than_trailers(fields: &FieldSection) -> bool {
    fields
        .list_members(&FieldName::TE)
        .any(|member| !member.eq_ignore_ascii_case(b"trailers"))
}

#[cfg(test)]
mod tests;
