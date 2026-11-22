//! Dates (RFC 9651 §3.3.7).

use crate::number::Integer;
use alloc::vec::Vec;

/// A date: seconds from 1970-01-01T00:00:00Z, leap seconds excluded (RFC 9651
/// §3.3.7). Any integer is a date.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Date(Integer);

impl Date {
    /// The date `seconds` after 1970-01-01T00:00:00Z; before it when
    /// negative.
    pub const fn from_seconds(seconds: Integer) -> Self {
        Self(seconds)
    }

    /// The seconds from 1970-01-01T00:00:00Z.
    pub const fn seconds(self) -> Integer {
        self.0
    }

    /// Writes `@` and the seconds (§4.1.10).
    pub(crate) fn serialize(self, out: &mut Vec<u8>) {
        out.push(b'@');
        self.0.serialize(out);
    }
}
