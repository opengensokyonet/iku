//! The fields that the Capsule Protocol reads and forbids (RFC 9297 §3.2,
//! §3.4).

use iku_semantics::field::{FieldName, FieldSection, FieldValue};
use iku_semantics::head::ResponseHead;
use iku_semantics::status::{StatusClass, StatusCode};
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
///
/// This is the whole rule for a request. A response also depends on its
/// status code: use [`response_uses_capsule_protocol`].
pub fn uses_capsule_protocol(fields: &FieldSection) -> bool {
    matches!(
        Item::from_fields(fields, &capsule_protocol_field_name()),
        Ok(Some(item)) if *item.bare_item() == BareItem::Boolean(true)
    )
}

/// Whether the response `head` uses the Capsule Protocol (RFC 9297 §3.2,
/// §3.4): its status code is 2xx or 101, and its fields say so as
/// [`uses_capsule_protocol`] reads them.
///
/// A response with any other status code does not use the protocol, whatever
/// its fields say.
///
/// # Errors
///
/// [`CapsuleResponseError`] if the status code is 204, 205 or 206 and the
/// fields say that the protocol is in use: those responses must not use it,
/// and the recipient treats the message as malformed (§3.2).
pub fn response_uses_capsule_protocol(head: &ResponseHead) -> Result<bool, CapsuleResponseError> {
    let status = head.status();
    let allowed = status.get() == 101 || status.class() == StatusClass::Successful;
    if !allowed || !uses_capsule_protocol(head.fields()) {
        return Ok(false);
    }
    if matches!(status.get(), 204..=206) {
        return Err(CapsuleResponseError { status });
    }
    Ok(true)
}

/// A response with status code 204, 205 or 206 says that it uses the Capsule
/// Protocol, which those responses must not do (RFC 9297 §3.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(display("a response with status {status} uses the capsule protocol"))]
pub struct CapsuleResponseError {
    status: StatusCode,
}

impl CapsuleResponseError {
    /// The status code: 204, 205 or 206.
    pub const fn status(self) -> StatusCode {
        self.status
    }
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
