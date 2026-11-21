//! Character classes of RFC 3986, as predicates on bytes.

/// `unreserved` (RFC 3986 §2.3).
pub(crate) const fn is_unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
}

/// `sub-delims` (RFC 3986 §2.2).
pub(crate) const fn is_sub_delim(byte: u8) -> bool {
    matches!(
        byte,
        b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'='
    )
}

/// Whether `bytes[index]` starts a `pct-encoded` (RFC 3986 §2.1): `%`
/// followed by two hexadecimal digits.
pub(crate) fn is_pct_encoded_at(bytes: &[u8], index: usize) -> bool {
    matches!(
        bytes.get(index..index + 3),
        Some([b'%', high, low]) if high.is_ascii_hexdigit() && low.is_ascii_hexdigit()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pct_encoded_needs_two_hex_digits() {
        assert!(is_pct_encoded_at(b"a%2Fb", 1));
        assert!(!is_pct_encoded_at(b"a%2", 1));
        assert!(!is_pct_encoded_at(b"a%G0", 1));
    }
}
