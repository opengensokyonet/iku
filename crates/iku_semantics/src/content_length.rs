//! The `Content-Length` field (RFC 9110 §8.6).

use crate::decimal::put_decimal;
use crate::field::{FieldName, FieldSection, FieldValue};
use bytes::BytesMut;
use snafu::Snafu;

/// The length of a message's content in octets, as the `Content-Length` field
/// gives it (RFC 9110 §8.6).
///
/// HTTP/3 checks it against the length of the DATA frames ([RFC 9114
/// §4.1.2]); HTTP/1.1 frames the content with it.
///
/// [RFC 9114 §4.1.2]: https://www.rfc-editor.org/rfc/rfc9114.html#section-4.1.2
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContentLength(u64);

impl ContentLength {
    /// The length `octets`.
    pub const fn new(octets: u64) -> Self {
        Self(octets)
    }

    /// Reads the `Content-Length` lines of `fields`; `None` when there are
    /// none.
    ///
    /// The value is `1*DIGIT`. When it is a comma-separated list of the same
    /// number, over one line or several, it is read as that number: RFC 9110
    /// §8.6 lets a recipient replace such a list with a single instance, since
    /// an upstream processor likely duplicated or combined the field. Empty
    /// members of the list are ignored, as in any list (§5.6.1). The members
    /// are compared as numbers, so `42, 042` is 42.
    ///
    /// # Errors
    ///
    /// [`ContentLengthError`] for the first rule that the lines break, going
    /// through the members in order.
    pub fn from_fields(fields: &FieldSection) -> Result<Option<Self>, ContentLengthError> {
        if !fields.contains(&FieldName::CONTENT_LENGTH) {
            return Ok(None);
        }
        let mut length = None;
        for member in fields.list_members(&FieldName::CONTENT_LENGTH) {
            let number = parse_decimal(member)?;
            match length {
                None => length = Some(number),
                Some(first) if first != number => return DifferentValuesSnafu.fail(),
                Some(_) => {}
            }
        }
        match length {
            Some(octets) => Ok(Some(Self(octets))),
            None => EmptySnafu.fail(),
        }
    }

    /// The length in octets.
    pub const fn get(self) -> u64 {
        self.0
    }

    /// The length as a field value: decimal digits without leading zeros.
    pub fn to_field_value(self) -> FieldValue {
        let mut digits = BytesMut::with_capacity(20);
        put_decimal(&mut digits, self.0);
        // Digits are visible ASCII.
        FieldValue::from_checked(digits.freeze())
    }
}

/// The number that `digits`, a non-empty member, is in decimal.
fn parse_decimal(digits: &[u8]) -> Result<u64, ContentLengthError> {
    let mut number: u64 = 0;
    for &digit in digits {
        if !digit.is_ascii_digit() {
            return NotDecimalSnafu.fail();
        }
        number = number
            .checked_mul(10)
            .and_then(|number| number.checked_add(u64::from(digit - b'0')))
            .ok_or(ContentLengthError::OutOfRange)?;
    }
    Ok(number)
}

/// The `Content-Length` lines do not give one length (RFC 9110 §8.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum ContentLengthError {
    /// The lines hold no number, only empty values or commas.
    #[snafu(display("content-length holds no number"))]
    Empty,
    /// A member holds a byte that is not a decimal digit.
    #[snafu(display("content-length holds a member that is not a decimal number"))]
    NotDecimal,
    /// A member is above `u64::MAX`; RFC 9110 §8.6 requires a recipient to
    /// guard against this overflow.
    #[snafu(display("content-length is above the largest length this type holds"))]
    OutOfRange,
    /// Two members are different numbers, so the length is unknown.
    #[snafu(display("content-length holds different numbers"))]
    DifferentValues,
}
