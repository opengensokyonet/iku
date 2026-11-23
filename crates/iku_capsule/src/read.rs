//! Reading capsules from a data stream (RFC 9297 §3.2, §3.3).

use crate::capsule_type::{CapsuleHeader, CapsuleType};
use bytes::Buf;
use core::num::NonZeroUsize;
use nitori_call::{ReceiverAccess, call};
use nitori_io::{adapters::ReadLimit, prelude::*};
use sakuya_var_int::call::{ReadVarIntError, ReadVarIntFieldExt as _};
use snafu::{ResultExt, Snafu};

/// Reads the header of the next capsule: `None` when the stream ends before
/// it, between two capsules.
///
/// The header keeps the widths of the encodings it was read from. The value
/// follows: compose the stream with [`CapsuleHeader::value_limit`] to read it
/// as a stream of its own, which ends where the value does, or drop it with
/// [`skip_capsule_value`].
///
/// A stream that ends inside the header is a truncated capsule, which the
/// recipient treats as a malformed or incomplete message (RFC 9297 §3.3);
/// the error then holds [`nitori_io::error::IncompleteError`].
#[call(sync)]
pub async fn read_capsule_header<F: Read, A: ReceiverAccess<Family = F>>(
    io: &mut A,
) -> Result<Option<CapsuleHeader>, ReadCapsuleHeaderError<F::Error>> {
    let capsule_type = match io.read_var_int_field::<CapsuleType>().await {
        Ok(capsule_type) => capsule_type,
        // The stream ended before the first byte of the header.
        Err(error) if error.is_incomplete() && error.completed() == 0 => return Ok(None),
        Err(source) => return Err(ReadCapsuleHeaderError::Type { source }),
    };
    let length = io.read_var_int_field().await.context(LengthSnafu)?;
    Ok(Some(CapsuleHeader::with_widths(capsule_type, length)))
}

/// The header of a capsule could not be read (RFC 9297 §3.2).
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum ReadCapsuleHeaderError<E> {
    /// The Capsule Type could not be read: the stream ended inside it, or
    /// reading failed.
    #[snafu(display("could not read the capsule type"))]
    Type {
        /// Why.
        source: ReadVarIntError<E>,
    },
    /// The Capsule Length could not be read: the stream ended before or
    /// inside it, or reading failed.
    #[snafu(display("could not read the capsule length"))]
    Length {
        /// Why.
        source: ReadVarIntError<E>,
    },
}

impl CapsuleHeader {
    /// The read budget of the value: `io.compose(header.value_limit())` reads
    /// the value and then ends, so that what reads it cannot read into the
    /// next capsule.
    ///
    /// The budget left afterwards, from `into_state`, is what of the value
    /// is still unread. A recipient that parses a capsule type it knows
    /// treats bytes left after the fields that the type defines as a
    /// malformed capsule (RFC 9297 §3.3); only a capsule that is dropped
    /// whole, such as one of an unknown type, has its rest skipped with
    /// [`skip_capsule_value`].
    pub fn value_limit(self) -> ReadLimit<u64> {
        ReadLimit(self.length().get())
    }
}

/// Reads `length` bytes and drops them, without holding them: the value of a
/// capsule that is dropped (RFC 9297 §3.2: a capsule of an unknown type; §3.5:
/// a DATAGRAM that is too long), or what is left of such a value after
/// reading part of it. Bytes left after the fields of a capsule type that
/// the recipient knows make the capsule malformed (§3.3) and are not for
/// skipping.
///
/// The length is a `u64`, so a value longer than memory sizes on a 32-bit
/// target is dropped like any other. Each read asks for at most 16 KiB, so
/// the length that the peer states never becomes the size of one request.
///
/// # Errors
///
/// [`SkipCapsuleValueError::Incomplete`] if the stream ends first: a
/// truncated capsule (§3.3). [`SkipCapsuleValueError::Receiver`] if reading
/// fails. Both say how many bytes were dropped before.
#[call(sync)]
pub async fn skip_capsule_value<F: ReadChunk, A: ReceiverAccess<Family = F>>(
    io: &mut A,
    length: u64,
) -> Result<(), SkipCapsuleValueError<F::Error>> {
    let mut completed = 0u64;
    let mut value = io.compose(ReadLimit(length));
    while completed < length {
        match value.read_chunk(SKIP_CHUNK_MAXIMUM).await {
            Ok(Some(chunk)) => {
                // A chunk is at most `usize::MAX` bytes, and the budget keeps
                // the total at most `length`.
                completed += chunk.remaining() as u64;
            }
            Ok(None) => return IncompleteSnafu { length, completed }.fail(),
            Err(source) => {
                return Err(SkipCapsuleValueError::Receiver {
                    source,
                    length,
                    completed,
                });
            }
        }
    }
    Ok(())
}

/// The largest number of bytes that [`skip_capsule_value`] asks for at once.
const SKIP_CHUNK_MAXIMUM: NonZeroUsize = NonZeroUsize::new(16 * 1024).unwrap();

/// The value of a capsule could not be read to the end to drop it (RFC 9297
/// §3.2).
///
/// The counts are `u64`, as the length of a capsule is, rather than the
/// `usize` of `nitori_io::error::IncompleteError`, which does not hold every
/// length on a 32-bit target.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum SkipCapsuleValueError<E> {
    /// The stream ended after `completed` of the `length` bytes: the capsule
    /// is truncated (§3.3).
    #[snafu(display("the stream ended after {completed} of {length} bytes of a capsule value"))]
    Incomplete {
        /// The bytes to drop.
        length: u64,
        /// The bytes dropped before the end, fewer than `length`.
        completed: u64,
    },
    /// Reading failed after `completed` of the `length` bytes.
    #[snafu(display("reading failed after {completed} of {length} bytes of a capsule value"))]
    Receiver {
        /// The receiver's error.
        source: E,
        /// The bytes to drop.
        length: u64,
        /// The bytes dropped before the failure.
        completed: u64,
    },
}

#[cfg(test)]
mod tests;
