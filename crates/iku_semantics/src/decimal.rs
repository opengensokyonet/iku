//! Decimal numbers written out as ASCII digits.

use bytes::{BufMut, BytesMut};

/// Writes `number` in decimal, without leading zeros.
pub(crate) fn put_decimal(dst: &mut BytesMut, number: u64) {
    let mut digits = [0u8; 20];
    let mut start = digits.len();
    let mut rest = number;
    loop {
        start -= 1;
        // `rest % 10` is a single digit.
        digits[start] = b'0' + (rest % 10) as u8;
        rest /= 10;
        if rest == 0 {
            break;
        }
    }
    dst.put_slice(&digits[start..]);
}
