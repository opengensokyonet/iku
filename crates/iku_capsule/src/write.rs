//! Writing capsules to a data stream (RFC 9297 §3.2).

use crate::capsule_type::CapsuleHeader;
use crate::var_int::put_var_int;
use nitori_call::{ReceiverAccess, call};
use nitori_io::{calls::WriteAllError, prelude::*};
use snafu::{ResultExt, Snafu};

/// Writes the header of a capsule: its type and length, each in the shortest
/// encoding. The value follows, written by the caller.
#[call(sync)]
pub async fn write_capsule_header<F: Write, A: ReceiverAccess<Family = F>>(
    io: &mut A,
    header: CapsuleHeader,
) -> Result<(), WriteCapsuleHeaderError<F::Error>> {
    let mut bytes = [0u8; 16];
    let mut type_bytes = [0u8; 8];
    let mut length_bytes = [0u8; 8];
    let type_width = put_var_int(&mut type_bytes, header.capsule_type().get());
    let length_width = put_var_int(&mut length_bytes, header.length());
    bytes[..type_width].copy_from_slice(&type_bytes[..type_width]);
    bytes[type_width..type_width + length_width].copy_from_slice(&length_bytes[..length_width]);
    io.write_all(&bytes[..type_width + length_width])
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
