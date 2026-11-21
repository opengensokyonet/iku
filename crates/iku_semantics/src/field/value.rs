//! Field values (RFC 9110 §5.5).

use crate::chars::is_whitespace;
use bytes::Bytes;
use core::fmt;
use snafu::Snafu;

/// A field value (RFC 9110 §5.5).
///
/// It holds bytes, not text: besides visible ASCII, space and HTAB it can hold
/// `obs-text` (0x80 to 0xFF), which a recipient treats as opaque data. It holds
/// no other control character and no leading or trailing space or HTAB, and
/// it can be empty.
///
/// RFC 9110 lets a recipient replace CR, LF and NUL with spaces instead of
/// rejecting them, and retain the other control characters in a safe context;
/// this type rejects all of them, as HTTP/2 and HTTP/3 must ([RFC 9114 §4.2]).
/// An HTTP/1.1 reader that replaces CR, LF and NUL does so before it constructs
/// the value.
///
/// [RFC 9114 §4.2]: https://www.rfc-editor.org/rfc/rfc9114.html#section-4.2
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FieldValue(Bytes);

impl FieldValue {
    /// Checks that `bytes` is a field value.
    ///
    /// # Errors
    ///
    /// [`FieldValueError`] for the first rule that `bytes` breaks, looking from
    /// the start: leading whitespace, then each byte, then trailing whitespace.
    pub fn new(bytes: Bytes) -> Result<Self, FieldValueError> {
        check(&bytes)?;
        Ok(Self(bytes))
    }

    /// As [`FieldValue::new`], without copying `text`.
    ///
    /// # Errors
    ///
    /// As [`FieldValue::new`].
    pub fn from_static(text: &'static str) -> Result<Self, FieldValueError> {
        Self::new(Bytes::from_static(text.as_bytes()))
    }

    /// A value made of other values' bytes that cannot break the rules.
    pub(crate) fn from_checked(bytes: Bytes) -> Self {
        debug_assert!(check(&bytes).is_ok());
        Self(bytes)
    }

    /// The bytes of the value.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// The value as [`Bytes`], without copying.
    pub fn into_bytes(self) -> Bytes {
        self.0
    }

    /// The number of bytes.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the value is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

fn check(bytes: &[u8]) -> Result<(), FieldValueError> {
    if bytes.first().copied().is_some_and(is_whitespace) {
        return LeadingWhitespaceSnafu.fail();
    }
    if let Some(index) = bytes.iter().position(|&byte| is_control(byte)) {
        return Err(FieldValueError::ControlCharacter {
            index,
            character: ControlCharacter(bytes[index]),
        });
    }
    if bytes.last().copied().is_some_and(is_whitespace) {
        return TrailingWhitespaceSnafu.fail();
    }
    Ok(())
}

/// A control character other than HTAB: 0x00 to 0x1F except 0x09, and 0x7F.
const fn is_control(byte: u8) -> bool {
    (byte < 0x20 && byte != b'\t') || byte == 0x7f
}

impl fmt::Debug for FieldValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FieldValue(\"{}\")", self.0.escape_ascii())
    }
}

/// The bytes are not a field value (RFC 9110 §5.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum FieldValueError {
    /// The value starts with a space or HTAB.
    #[snafu(display("a field value starts with whitespace"))]
    LeadingWhitespace,
    /// The value holds a control character other than HTAB.
    #[snafu(display("a field value holds the control character {character} at {index}"))]
    ControlCharacter {
        /// Where the character is, counted from the start of the value.
        index: usize,
        /// The character.
        character: ControlCharacter,
    },
    /// The value ends with a space or HTAB.
    #[snafu(display("a field value ends with whitespace"))]
    TrailingWhitespace,
}

/// A control character that a field value cannot hold: 0x00 to 0x1F except
/// HTAB (0x09), and DEL (0x7F).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ControlCharacter(u8);

impl ControlCharacter {
    /// The byte.
    pub const fn byte(self) -> u8 {
        self.0
    }

    /// Whether it is CR, LF or NUL, which RFC 9110 §5.5 lets a recipient
    /// replace with a space instead of rejecting the message.
    pub const fn is_cr_lf_or_nul(self) -> bool {
        matches!(self.0, b'\r' | b'\n' | 0)
    }
}

impl fmt::Display for ControlCharacter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#04x}", self.0)
    }
}
