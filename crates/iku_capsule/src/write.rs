//! Writing capsules to a data stream (RFC 9297 §3.2).

use crate::capsule_type::CapsuleHeader;
use bytes::BufMut;
use nitori_call::{ReceiverAccess, call};
use nitori_io::{calls::WriteAllError, prelude::*};
use sakuya_var_int::{VarInt, codec::write_var_int};
use snafu::{ResultExt, Snafu};

/// Writes the header of a capsule: its type and length, each in the shortest
/// encoding. The value follows, written by the caller.
#[call(sync)]
pub async fn write_capsule_header<F: Write, A: ReceiverAccess<Family = F>>(
    io: &mut A,
    header: CapsuleHeader,
) -> Result<(), WriteCapsuleHeaderError<F::Error>> {
    let mut bytes = [0u8; 2 * VarInt::MAX_ENCODED_LEN];
    let mut rest = &mut bytes[..];
    write_var_int(&mut rest, header.capsule_type().get());
    write_var_int(&mut rest, header.length());
    let unused = rest.remaining_mut();
    let encoded_len = bytes.len() - unused;
    io.write_all(&bytes[..encoded_len])
        .await
        .context(WriteCapsuleHeaderSnafu)
}

/// The header of a capsule could not be written.
#[derive(Debug, Snafu)]
#[snafu(display("could not write the capsule header"))]
pub struct WriteCapsuleHeaderError<E> {
    source: WriteAllError<E>,
}

impl<E> WriteCapsuleHeaderError<E> {
    /// Why.
    pub fn write_error(&self) -> &WriteAllError<E> {
        &self.source
    }
}

#[cfg(test)]
mod tests;
