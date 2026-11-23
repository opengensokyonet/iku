//! Capsule types and headers (RFC 9297 §3.2).

use core::fmt;
use sakuya_var_int::VarInt;

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

impl fmt::Display for CapsuleType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", self.0.get())
    }
}

/// The header of a capsule: its type and the length of its value in bytes
/// (RFC 9297 §3.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CapsuleHeader {
    capsule_type: CapsuleType,
    length: VarInt,
}

impl CapsuleHeader {
    /// The header of a capsule of `capsule_type` whose value is `length`
    /// bytes.
    pub const fn new(capsule_type: CapsuleType, length: VarInt) -> Self {
        Self {
            capsule_type,
            length,
        }
    }

    /// The type.
    pub const fn capsule_type(self) -> CapsuleType {
        self.capsule_type
    }

    /// The length of the value in bytes.
    pub const fn length(self) -> VarInt {
        self.length
    }
}

#[cfg(test)]
mod tests;
