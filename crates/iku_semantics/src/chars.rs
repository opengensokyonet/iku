//! Character classes of RFC 9110 and RFC 3986, as predicates on bytes.

/// `tchar` (RFC 9110 §5.6.2).
pub(crate) const fn is_tchar(byte: u8) -> bool {
    matches!(
        byte,
        b'!' | b'#'
            | b'$'
            | b'%'
            | b'&'
            | b'\''
            | b'*'
            | b'+'
            | b'-'
            | b'.'
            | b'^'
            | b'_'
            | b'`'
            | b'|'
            | b'~'
    ) || byte.is_ascii_alphanumeric()
}

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

/// `OWS`, `RWS` and `BWS` are made of these (RFC 9110 §5.6.3).
pub(crate) const fn is_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t')
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
    fn tchar_is_the_set_of_rfc_9110() {
        let tchars: alloc::vec::Vec<u8> = (0..=u8::MAX).filter(|&b| is_tchar(b)).collect();
        let mut expected: alloc::vec::Vec<u8> = b"!#$%&'*+-.^_`|~".to_vec();
        expected.extend(b'0'..=b'9');
        expected.extend(b'A'..=b'Z');
        expected.extend(b'a'..=b'z');
        expected.sort_unstable();
        assert_eq!(tchars, expected);
    }

    #[test]
    fn pct_encoded_needs_two_hex_digits() {
        assert!(is_pct_encoded_at(b"a%2Fb", 1));
        assert!(!is_pct_encoded_at(b"a%2", 1));
        assert!(!is_pct_encoded_at(b"a%G0", 1));
    }
}
