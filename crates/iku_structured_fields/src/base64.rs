//! Base64 (RFC 4648 §4), as byte sequences use it (RFC 9651 §3.3.5).

use alloc::vec::Vec;

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Why bytes do not decode.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DecodeError {
    /// The byte at this index is outside the alphabet and is not `=`.
    InvalidCharacter(usize),
    /// The `=` at this index is followed by a digit, or is one too many.
    MisplacedPadding(usize),
    /// The number of digits is one more than a multiple of four.
    InvalidLength,
}

/// Writes `input` in base64 with `=` padding.
pub(crate) fn encode(input: &[u8], out: &mut Vec<u8>) {
    for group in input.chunks(3) {
        let bits = group.iter().enumerate().fold(0u32, |bits, (index, &byte)| {
            bits | (u32::from(byte) << (16 - 8 * index))
        });
        for index in 0..4 {
            if index <= group.len() {
                // Six bits index the alphabet.
                out.push(ALPHABET[((bits >> (18 - 6 * index)) & 0x3f) as usize]);
            } else {
                out.push(b'=');
            }
        }
    }
}

/// Decodes `input`. Missing `=` padding and non-zero pad bits are accepted
/// (RFC 9651 §4.2.7).
pub(crate) fn decode(input: &[u8]) -> Result<Vec<u8>, DecodeError> {
    let digits = input
        .iter()
        .position(|&byte| byte == b'=')
        .unwrap_or(input.len());
    let mut values = Vec::with_capacity(digits);
    for (index, &byte) in input[..digits].iter().enumerate() {
        values.push(digit_value(byte).ok_or(DecodeError::InvalidCharacter(index))?);
    }
    let needed = (4 - digits % 4) % 4;
    for (offset, &byte) in input[digits..].iter().enumerate() {
        let index = digits + offset;
        if byte != b'=' {
            return Err(match digit_value(byte) {
                Some(_) => DecodeError::MisplacedPadding(digits),
                None => DecodeError::InvalidCharacter(index),
            });
        }
        if offset >= needed {
            return Err(DecodeError::MisplacedPadding(index));
        }
    }
    if digits % 4 == 1 {
        return Err(DecodeError::InvalidLength);
    }
    let mut out = Vec::with_capacity(digits * 3 / 4);
    for group in values.chunks(4) {
        let bits = group
            .iter()
            .enumerate()
            .fold(0u32, |bits, (index, &value)| {
                bits | (u32::from(value) << (18 - 6 * index))
            });
        // A group of n digits holds n - 1 whole bytes; the rest are pad bits.
        for index in 0..group.len() - 1 {
            out.push((bits >> (16 - 8 * index)) as u8);
        }
    }
    Ok(out)
}

fn digit_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
