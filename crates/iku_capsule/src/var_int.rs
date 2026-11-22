//! QUIC variable-length integers (RFC 9000 §16), which the Capsule Type and
//! the Capsule Length use (RFC 9297 §3.2).

use nitori_call::{ReceiverAccess, call};
use nitori_io::{calls::ReadArrayError, prelude::*};

/// The largest value a variable-length integer holds, 2^62 - 1.
pub(crate) const MAX: u64 = (1 << 62) - 1;

/// Reads the rest of a variable-length integer whose first byte is `first`.
/// A longer encoding than needed is accepted (RFC 9297 §1.1).
#[call]
pub(crate) async fn read_var_int_rest<F: Read, A: ReceiverAccess<Family = F>>(
    io: &mut A,
    first: u8,
) -> Result<u64, ReadArrayError<F::Error>> {
    let value = u64::from(first & 0x3f);
    // The top two bits give the width: 1, 2, 4 or 8 bytes in all.
    Ok(match first >> 6 {
        0 => value,
        1 => extend(value, &io.read_array::<1>().await?),
        2 => extend(value, &io.read_array::<3>().await?),
        _ => extend(value, &io.read_array::<7>().await?),
    })
}

/// Appends big-endian `bytes` to the low bits of `value`.
fn extend(value: u64, bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .fold(value, |value, byte| (value << 8) | u64::from(*byte))
}

/// Writes `value`, at most [`MAX`], in the shortest encoding into `out`;
/// returns how many bytes it took.
pub(crate) fn put_var_int(out: &mut [u8; 8], value: u64) -> usize {
    let (width, prefix) = match value {
        0..0x40 => (1, 0x00),
        0x40..0x4000 => (2, 0x40),
        0x4000..0x4000_0000 => (4, 0x80),
        _ => (8, 0xc0),
    };
    let bytes = value.to_be_bytes();
    out[..width].copy_from_slice(&bytes[8 - width..]);
    out[0] |= prefix;
    width
}

#[cfg(test)]
mod tests;
