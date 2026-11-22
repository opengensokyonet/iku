//! Reading capsules from a data stream (RFC 9297 §3.2, §3.3).

use crate::capsule_type::{CapsuleHeader, CapsuleType};
use crate::var_int::{ReadVarIntRestExt, read_var_int_rest};
use bytes::{Bytes, BytesMut};
use core::num::NonZeroUsize;
use core::ops::ControlFlow;
use core::pin::pin;
use nitori_call::{ReceiverAccess, call};
use nitori_io::{
    calls::{ReadArrayError, ReadChunksExactError, ReadExactError},
    prelude::*,
};
use snafu::{ResultExt, Snafu};

/// Reads the header of the next capsule: `None` when the stream ends before
/// it, between two capsules.
///
/// A stream that ends inside the header is a truncated capsule, which the
/// recipient treats as a malformed or incomplete message (RFC 9297 §3.3);
/// the error then holds [`nitori_io::error::IncompleteError`].
#[call(sync)]
pub async fn read_capsule_header<F: Read, A: ReceiverAccess<Family = F>>(
    io: &mut A,
) -> Result<Option<CapsuleHeader>, ReadCapsuleHeaderError<F::Error>> {
    let first = match io.read_array::<1>().await {
        Ok([first]) => first,
        Err(ReadArrayError::Incomplete { .. }) => return Ok(None),
        Err(source) => return Err(ReadCapsuleHeaderError::Type { source }),
    };
    let capsule_type = read_var_int_rest(io, first).await.context(TypeSnafu)?;
    let [first] = io.read_array::<1>().await.context(LengthSnafu)?;
    let length = io.read_var_int_rest(first).await.context(LengthSnafu)?;
    Ok(Some(CapsuleHeader::from_var_ints(
        CapsuleType::from_var_int(capsule_type),
        length,
    )))
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
        source: ReadArrayError<E>,
    },
    /// The Capsule Length could not be read: the stream ended before or
    /// inside it, or reading failed.
    #[snafu(display("could not read the capsule length"))]
    Length {
        /// Why.
        source: ReadArrayError<E>,
    },
}

/// Reads the value of the capsule with `header`, yielding it in chunks of at
/// most `chunk_maximum` bytes, without holding the whole value (RFC 9297 §3.2
/// asks a recipient not to buffer a whole value before processing it).
///
/// A stream that ends inside the value is a truncated capsule (§3.3); the
/// error then holds [`nitori_io::error::IncompleteError`].
#[call(sync, yields = F::Chunk)]
pub async fn read_capsule_value<F: ReadChunk, A: ReceiverAccess<Family = F>>(
    io: &mut A,
    header: CapsuleHeader,
    chunk_maximum: NonZeroUsize,
) -> Result<(), ReadCapsuleValueError<F::Error>> {
    let length = header.length();
    let Ok(length) = usize::try_from(length) else {
        return LengthTooLargeSnafu { length }.fail();
    };
    let mut chunks = pin!(io.read_chunks_exact(length, chunk_maximum));
    loop {
        match chunks.as_mut().step().await {
            ControlFlow::Continue(chunk) => yield chunk,
            ControlFlow::Break(result) => return result.context(ValueSnafu),
        }
    }
}

/// The value of a capsule could not be read (RFC 9297 §3.2).
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum ReadCapsuleValueError<E> {
    /// The length does not fit in `usize` on this target.
    #[snafu(display("capsule length {length} does not fit in memory sizes"))]
    LengthTooLarge {
        /// The length, above `usize::MAX`.
        length: u64,
    },
    /// The value could not be read: the stream ended inside it, or reading
    /// failed.
    #[snafu(display("could not read the capsule value"))]
    Value {
        /// Why.
        source: ReadChunksExactError<E>,
    },
}

/// Reads the value of the capsule with `header` whole when it is at most
/// `maximum` bytes; otherwise reads it in chunks of at most `chunk_maximum`
/// bytes, drops it and returns `None`.
///
/// It is for short capsules, such as DATAGRAM, whose value is used whole; an
/// endpoint discards a DATAGRAM capsule that is too long without buffering
/// it (RFC 9297 §3.5).
#[call(sync)]
pub async fn collect_capsule_value<F: ReadChunk, A: ReceiverAccess<Family = F>>(
    io: &mut A,
    header: CapsuleHeader,
    maximum: usize,
    chunk_maximum: NonZeroUsize,
) -> Result<Option<Bytes>, CollectCapsuleValueError<F::Error>> {
    match usize::try_from(header.length()) {
        Ok(length) if length <= maximum => {
            let mut value = BytesMut::with_capacity(length);
            io.read_exact(&mut value, length)
                .await
                .context(CollectSnafu)?;
            Ok(Some(value.freeze()))
        }
        _ => {
            let mut chunks = pin!(io.read_capsule_value(header, chunk_maximum));
            loop {
                match chunks.as_mut().step().await {
                    ControlFlow::Continue(_) => {}
                    ControlFlow::Break(result) => {
                        result.context(DiscardSnafu)?;
                        return Ok(None);
                    }
                }
            }
        }
    }
}

/// The value of a capsule could not be collected or discarded (RFC 9297
/// §3.2).
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum CollectCapsuleValueError<E> {
    /// The value, at most the maximum, could not be read whole: the stream
    /// ended inside it, or reading failed.
    #[snafu(display("could not read the capsule value"))]
    Collect {
        /// Why.
        source: ReadExactError<E>,
    },
    /// The value, above the maximum, could not be read to discard it.
    #[snafu(display("could not discard the capsule value"))]
    Discard {
        /// Why.
        source: ReadCapsuleValueError<E>,
    },
}

#[cfg(test)]
mod tests;
