//! Capsule types and headers (RFC 9297 §3.2).

use crate::var_int;
use core::fmt;
use snafu::Snafu;

/// The type of a capsule: a variable-length integer, so at most 2^62 - 1
/// (RFC 9297 §3.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CapsuleType(u64);

impl CapsuleType {
    /// `DATAGRAM` (0x00), which carries an HTTP datagram (RFC 9297 §3.5).
    pub const DATAGRAM: Self = Self(0x00);

    /// Checks that `value` is at most 2^62 - 1.
    ///
    /// # Errors
    ///
    /// [`CapsuleTypeOutOfRangeError`] if it is larger.
    pub const fn new(value: u64) -> Result<Self, CapsuleTypeOutOfRangeError> {
        if value > var_int::MAX {
            return Err(CapsuleTypeOutOfRangeError { value });
        }
        Ok(Self(value))
    }

    /// A type read from a variable-length integer.
    pub(crate) const fn from_var_int(value: u64) -> Self {
        Self(value)
    }

    /// The value.
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for CapsuleType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

/// The value is above 2^62 - 1, so it is not a capsule type (RFC 9297 §3.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(display("capsule type {value:#x} is above 2^62 - 1"))]
pub struct CapsuleTypeOutOfRangeError {
    value: u64,
}

impl CapsuleTypeOutOfRangeError {
    /// The value, above 2^62 - 1.
    pub const fn value(self) -> u64 {
        self.value
    }
}

/// The header of a capsule: its type and the length of its value in bytes
/// (RFC 9297 §3.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CapsuleHeader {
    capsule_type: CapsuleType,
    length: u64,
}

impl CapsuleHeader {
    /// The header of a capsule of `capsule_type` whose value is `length`
    /// bytes.
    ///
    /// # Errors
    ///
    /// [`CapsuleLengthOutOfRangeError`] if `length` is above 2^62 - 1.
    pub const fn new(
        capsule_type: CapsuleType,
        length: u64,
    ) -> Result<Self, CapsuleLengthOutOfRangeError> {
        if length > var_int::MAX {
            return Err(CapsuleLengthOutOfRangeError { length });
        }
        Ok(Self {
            capsule_type,
            length,
        })
    }

    /// A header read from two variable-length integers.
    pub(crate) const fn from_var_ints(capsule_type: CapsuleType, length: u64) -> Self {
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
    pub const fn length(self) -> u64 {
        self.length
    }
}

/// The length is above 2^62 - 1, so no capsule has it (RFC 9297 §3.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(display("capsule length {length} is above 2^62 - 1"))]
pub struct CapsuleLengthOutOfRangeError {
    length: u64,
}

impl CapsuleLengthOutOfRangeError {
    /// The length, above 2^62 - 1.
    pub const fn length(self) -> u64 {
        self.length
    }
}

#[cfg(test)]
mod tests;
