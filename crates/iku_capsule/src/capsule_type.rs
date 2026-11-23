//! Capsule types and headers (RFC 9297 §3.2).

use core::fmt;
use sakuya_var_int::{VarInt, VarIntField};

/// The type of a capsule: a variable-length integer (RFC 9297 §3.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CapsuleType(VarInt);

impl CapsuleType {
    /// `DATAGRAM` (0x00), which carries an HTTP datagram (RFC 9297 §3.5).
    pub const DATAGRAM: Self = Self(VarInt::ZERO);

    /// The type whose value is `value`.
    pub const fn new(value: VarInt) -> Self {
        Self(value)
    }

    /// The value.
    pub const fn get(self) -> VarInt {
        self.0
    }
}

impl From<VarInt> for CapsuleType {
    fn from(value: VarInt) -> Self {
        Self(value)
    }
}

impl From<CapsuleType> for VarInt {
    fn from(capsule_type: CapsuleType) -> Self {
        capsule_type.0
    }
}

impl fmt::Display for CapsuleType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", self.0.get())
    }
}

/// The header of a capsule: its type and the length of its value in bytes
/// (RFC 9297 §3.2), each with the width of its encoding.
///
/// A header that is read keeps the widths it was read with, and writing it
/// writes them, so an intermediary forwards the header unmodified (§3.2).
/// [`CapsuleHeader::new`] takes the shortest widths, and
/// [`CapsuleHeader::with_widths`] the widths of its fields. Equality compares the
/// widths too.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CapsuleHeader {
    capsule_type: VarIntField<CapsuleType>,
    length: VarIntField<VarInt>,
}

impl CapsuleHeader {
    /// The header of a capsule of `capsule_type` whose value is `length`
    /// bytes, both in their shortest encodings.
    pub fn new(capsule_type: CapsuleType, length: VarInt) -> Self {
        Self {
            capsule_type: VarIntField::shortest(capsule_type),
            length: VarIntField::shortest(length),
        }
    }

    /// The header with the type and the length in the widths that their
    /// fields hold.
    pub const fn with_widths(
        capsule_type: VarIntField<CapsuleType>,
        length: VarIntField<VarInt>,
    ) -> Self {
        Self {
            capsule_type,
            length,
        }
    }

    /// The type.
    pub const fn capsule_type(self) -> CapsuleType {
        self.capsule_type.get()
    }

    /// The length of the value in bytes.
    pub const fn length(self) -> VarInt {
        self.length.get()
    }

    /// The type with the width of its encoding.
    pub const fn capsule_type_field(self) -> VarIntField<CapsuleType> {
        self.capsule_type
    }

    /// The length with the width of its encoding.
    pub const fn length_field(self) -> VarIntField<VarInt> {
        self.length
    }
}

#[cfg(test)]
mod tests;
