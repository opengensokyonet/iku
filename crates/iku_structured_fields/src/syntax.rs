//! The cursor that the parsers share (RFC 9651 §4.2), and the syntax errors
//! they report.

use alloc::vec::Vec;
use bytes::Bytes;
use snafu::Snafu;

/// A position in the bytes of one field line.
pub(crate) struct Parser {
    bytes: Bytes,
    index: usize,
}

impl Parser {
    pub(crate) fn new(bytes: Bytes) -> Self {
        Self { bytes, index: 0 }
    }

    /// How many bytes have been consumed.
    pub(crate) fn index(&self) -> usize {
        self.index
    }

    pub(crate) fn peek(&self) -> Option<u8> {
        self.bytes.get(self.index).copied()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.index >= self.bytes.len()
    }

    /// Consumes one byte; at the end, nothing.
    pub(crate) fn advance(&mut self) {
        if !self.is_empty() {
            self.index += 1;
        }
    }

    /// Consumes and returns the next byte.
    pub(crate) fn next_byte(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.index += 1;
        Some(byte)
    }

    /// Consumes bytes while `keep` holds.
    pub(crate) fn advance_while(&mut self, keep: impl Fn(u8) -> bool) {
        while self.peek().is_some_and(&keep) {
            self.index += 1;
        }
    }

    /// Discards leading spaces (SP).
    pub(crate) fn skip_sp(&mut self) {
        self.advance_while(|byte| byte == b' ');
    }

    /// Discards leading OWS: spaces and horizontal tabs (RFC 9110 §5.6.3).
    pub(crate) fn skip_ows(&mut self) {
        self.advance_while(|byte| byte == b' ' || byte == b'\t');
    }

    /// The bytes from `start` to the current position, without copying.
    pub(crate) fn slice_from(&self, start: usize) -> Bytes {
        self.bytes.slice(start..self.index)
    }

    /// The bytes not consumed yet.
    pub(crate) fn rest(&self) -> &[u8] {
        self.bytes.get(self.index..).unwrap_or_default()
    }

    /// Consumes `count` bytes; the caller has checked that they are there.
    pub(crate) fn skip(&mut self, count: usize) {
        self.index = (self.index + count).min(self.bytes.len());
    }
}

/// Writes `number` in decimal, without leading zeros.
pub(crate) fn put_decimal(out: &mut Vec<u8>, number: u64) {
    let mut digits = [0u8; 20];
    let mut start = digits.len();
    let mut rest = number;
    loop {
        start -= 1;
        // `rest % 10` is a single digit.
        digits[start] = b'0' + (rest % 10) as u8;
        rest /= 10;
        if rest == 0 {
            break;
        }
    }
    out.extend_from_slice(&digits[start..]);
}

/// A field line is not a structured field of the expected type (RFC 9651
/// §4.2).
///
/// Each variant holds `index`: where in the line the rule breaks, counted in
/// bytes from the start of the line. An index equal to the length of the line
/// means that the line ends too soon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum SyntaxError {
    /// A list or a dictionary ends right after a comma (§4.2.1, §4.2.2).
    #[snafu(display("the list ends after a comma at byte {index}"))]
    TrailingComma {
        /// Where the line ends.
        index: usize,
    },
    /// A member of a list or a dictionary is followed by something other than
    /// a comma (§4.2.1, §4.2.2).
    #[snafu(display("expected a comma at byte {index}"))]
    ExpectedComma {
        /// The byte that is not a comma.
        index: usize,
    },
    /// An item is followed by more than spaces (§4.2).
    #[snafu(display("unexpected bytes after the item at byte {index}"))]
    TrailingCharacters {
        /// The first byte after the item.
        index: usize,
    },
    /// No bare item starts here (§4.2.3.1).
    #[snafu(display("no bare item starts at byte {index}"))]
    InvalidBareItemStart {
        /// The byte, or the end of the line.
        index: usize,
    },
    /// No key starts here: a key starts with a lowercase letter or `*`
    /// (§4.2.3.3).
    #[snafu(display("no key starts at byte {index}"))]
    InvalidKeyStart {
        /// The byte, or the end of the line.
        index: usize,
    },
    /// An inner list has no closing parenthesis (§4.2.1.2).
    #[snafu(display("the inner list that opens at byte {index} is not closed"))]
    UnclosedInnerList {
        /// The opening parenthesis.
        index: usize,
    },
    /// An item of an inner list is followed by something other than a space
    /// or the closing parenthesis (§4.2.1.2).
    #[snafu(display("expected a space or `)` in the inner list at byte {index}"))]
    ExpectedInnerListDelimiter {
        /// The byte that is neither.
        index: usize,
    },
    /// A `-` is not followed by a digit (§4.2.4).
    #[snafu(display("expected a digit at byte {index}"))]
    MissingDigit {
        /// The byte that is not a digit, or the end of the line.
        index: usize,
    },
    /// A number has more than 15 digits before any decimal point (§4.2.4):
    /// too long for an integer, and for a decimal too.
    #[snafu(display("the number has more than 15 digits at byte {index}"))]
    IntegerTooLong {
        /// The sixteenth digit.
        index: usize,
    },
    /// A decimal has more than 12 digits before its point (§4.2.4).
    #[snafu(display("the decimal has more than 12 integer digits at byte {index}"))]
    IntegerPartTooLong {
        /// The decimal point.
        index: usize,
    },
    /// A decimal has more than 3 digits after its point (§4.2.4).
    #[snafu(display("the decimal has more than 3 fractional digits at byte {index}"))]
    FractionTooLong {
        /// The fourth fractional digit.
        index: usize,
    },
    /// A decimal point is not followed by a digit (§4.2.4).
    #[snafu(display("the decimal point at byte {index} has no digit after it"))]
    EmptyFraction {
        /// The decimal point.
        index: usize,
    },
    /// A string has no closing quote (§4.2.5).
    #[snafu(display("the string that opens at byte {index} is not closed"))]
    UnclosedString {
        /// The opening quote.
        index: usize,
    },
    /// A backslash in a string escapes something other than `"` or `\`
    /// (§4.2.5).
    #[snafu(display("invalid escape in a string at byte {index}"))]
    InvalidStringEscape {
        /// The byte after the backslash.
        index: usize,
    },
    /// A string holds a byte that is not visible ASCII or a space (§4.2.5).
    #[snafu(display("invalid byte in a string at byte {index}"))]
    InvalidStringCharacter {
        /// The byte.
        index: usize,
    },
    /// A byte sequence has no closing colon (§4.2.7).
    #[snafu(display("the byte sequence that opens at byte {index} is not closed"))]
    UnclosedByteSequence {
        /// The opening colon.
        index: usize,
    },
    /// A byte sequence holds a byte outside the base64 alphabet (§4.2.7).
    #[snafu(display("invalid base64 byte at byte {index}"))]
    InvalidBase64Character {
        /// The byte.
        index: usize,
    },
    /// A byte sequence holds `=` before its end, or more than two (§4.2.7,
    /// RFC 4648 §4).
    #[snafu(display("misplaced base64 padding at byte {index}"))]
    MisplacedBase64Padding {
        /// The first `=` that is out of place.
        index: usize,
    },
    /// A byte sequence holds a number of base64 digits that no bytes encode
    /// to: one more than a multiple of four (RFC 4648 §4).
    #[snafu(display("the byte sequence at byte {index} has an impossible length"))]
    InvalidBase64Length {
        /// The opening colon.
        index: usize,
    },
    /// A `?` is not followed by `0` or `1` (§4.2.8).
    #[snafu(display("expected `0` or `1` at byte {index}"))]
    InvalidBoolean {
        /// The byte after `?`, or the end of the line.
        index: usize,
    },
    /// A date is a decimal, not an integer (§4.2.9).
    #[snafu(display("the date at byte {index} is not an integer"))]
    DecimalDate {
        /// The first byte of the number.
        index: usize,
    },
    /// A `%` that starts a display string is not followed by `"` (§4.2.10).
    #[snafu(display("expected `\"` at byte {index}"))]
    ExpectedDisplayStringQuote {
        /// The byte after `%`, or the end of the line.
        index: usize,
    },
    /// A display string has no closing quote (§4.2.10).
    #[snafu(display("the display string that opens at byte {index} is not closed"))]
    UnclosedDisplayString {
        /// The `%` that starts it.
        index: usize,
    },
    /// A display string holds a byte that is not visible ASCII or a space
    /// (§4.2.10).
    #[snafu(display("invalid byte in a display string at byte {index}"))]
    InvalidDisplayStringCharacter {
        /// The byte.
        index: usize,
    },
    /// A `%` in a display string is not followed by two lowercase
    /// hexadecimal digits (§4.2.10).
    #[snafu(display("invalid percent-encoding at byte {index}"))]
    InvalidPercentEncoding {
        /// The `%`.
        index: usize,
    },
    /// A display string decodes to bytes that are not UTF-8 (§4.2.10).
    #[snafu(display("the display string at byte {index} is not UTF-8"))]
    InvalidUtf8 {
        /// The `%` that starts it.
        index: usize,
    },
}

impl SyntaxError {
    /// Where in the line the rule breaks, in bytes from its start.
    pub const fn index(self) -> usize {
        match self {
            Self::TrailingComma { index }
            | Self::ExpectedComma { index }
            | Self::TrailingCharacters { index }
            | Self::InvalidBareItemStart { index }
            | Self::InvalidKeyStart { index }
            | Self::UnclosedInnerList { index }
            | Self::ExpectedInnerListDelimiter { index }
            | Self::MissingDigit { index }
            | Self::IntegerTooLong { index }
            | Self::IntegerPartTooLong { index }
            | Self::FractionTooLong { index }
            | Self::EmptyFraction { index }
            | Self::UnclosedString { index }
            | Self::InvalidStringEscape { index }
            | Self::InvalidStringCharacter { index }
            | Self::UnclosedByteSequence { index }
            | Self::InvalidBase64Character { index }
            | Self::MisplacedBase64Padding { index }
            | Self::InvalidBase64Length { index }
            | Self::InvalidBoolean { index }
            | Self::DecimalDate { index }
            | Self::ExpectedDisplayStringQuote { index }
            | Self::UnclosedDisplayString { index }
            | Self::InvalidDisplayStringCharacter { index }
            | Self::InvalidPercentEncoding { index }
            | Self::InvalidUtf8 { index } => index,
        }
    }
}
