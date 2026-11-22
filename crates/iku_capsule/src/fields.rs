//! The fields that the Capsule Protocol reads and forbids (RFC 9297 §3.2,
//! §3.4).

use iku_semantics::field::{FieldName, FieldSection, FieldValue};
use iku_structured_fields::{BareItem, Item};
use snafu::Snafu;

/// `capsule-protocol` (RFC 9297 §3.4).
pub fn capsule_protocol_field_name() -> FieldName {
    // Invariant: the text is a lowercase token.
    FieldName::from_static("capsule-protocol").expect("capsule-protocol is a field name")
}

/// `?1`, the value of `Capsule-Protocol` that says the Capsule Protocol is in
/// use (RFC 9297 §3.4).
pub fn capsule_protocol_field_value() -> FieldValue {
    Item::new(BareItem::Boolean(true)).to_field_value()
}

/// Whether `fields` say that the Capsule Protocol is in use: `Capsule-Protocol`
/// is an item field whose value is the boolean true (RFC 9297 §3.4).
///
/// Its parameters are ignored. A value that is not a boolean, a field that
/// does not parse and a field of several lines count as no field, as does the
/// boolean false.
pub fn uses_capsule_protocol(fields: &FieldSection) -> bool {
    matches!(
        Item::from_fields(fields, &capsule_protocol_field_name()),
        Ok(Some(item)) if *item.bare_item() == BareItem::Boolean(true)
    )
}

/// Checks that `fields` hold none of the fields that a message using the
/// Capsule Protocol must not have: `Content-Length`, `Content-Type` and
/// `Transfer-Encoding` (RFC 9297 §3.2).
///
/// # Errors
///
/// [`CapsuleMessageFieldError`] for the first of them, in that order. The
/// recipient treats the message as malformed (§3.2).
pub fn check_capsule_message_fields(fields: &FieldSection) -> Result<(), CapsuleMessageFieldError> {
    if fields.contains(&FieldName::CONTENT_LENGTH) {
        return Err(CapsuleMessageFieldError::ContentLength);
    }
    // Invariant: the text is a lowercase token.
    let content_type =
        FieldName::from_static("content-type").expect("content-type is a field name");
    if fields.contains(&content_type) {
        return Err(CapsuleMessageFieldError::ContentType);
    }
    if fields.contains(&FieldName::TRANSFER_ENCODING) {
        return Err(CapsuleMessageFieldError::TransferEncoding);
    }
    Ok(())
}

/// A message that uses the Capsule Protocol has a field it must not have
/// (RFC 9297 §3.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
pub enum CapsuleMessageFieldError {
    /// It has `Content-Length`.
    #[snafu(display("a capsule message has content-length"))]
    ContentLength,
    /// It has `Content-Type`.
    #[snafu(display("a capsule message has content-type"))]
    ContentType,
    /// It has `Transfer-Encoding`.
    #[snafu(display("a capsule message has transfer-encoding"))]
    TransferEncoding,
}

#[cfg(test)]
mod tests;
