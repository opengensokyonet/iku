//! Writing capsules to a data stream (RFC 9297 §3.2).

use crate::capsule_type::CapsuleHeader;
use bytes::BufMut;
use nitori_call::{ReceiverAccess, call};
use nitori_io::{Combine, calls::WriteAllError, prelude::*};
use sakuya_var_int::{VarInt, codec::write_var_int_field};
use snafu::{ResultExt, Snafu};

/// Writes the header of a capsule: its type and length, each in the width
/// that the header holds, so a header that was read is written back as it
/// was. The value follows, written by the caller. Returns the sink's merged
/// report of the bytes written.
#[call(sync)]
pub async fn write_capsule_header<F: Write<Report: Combine>, A: ReceiverAccess<Family = F>>(
    io: &mut A,
    header: CapsuleHeader,
) -> Result<F::Report, WriteCapsuleHeaderError<F::Error>> {
    let mut bytes = [0u8; 2 * VarInt::MAX_ENCODED_LEN];
    let mut rest = &mut bytes[..];
    // invariant: each encoding takes at most `VarInt::MAX_ENCODED_LEN`
    // bytes, so both fit.
    write_var_int_field(&mut rest, header.capsule_type_field());
    write_var_int_field(&mut rest, header.length_field());
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
