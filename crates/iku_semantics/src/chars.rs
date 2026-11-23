//! Character classes of RFC 9110, as predicates on bytes.

/// `tchar` (RFC 9110 §5.6.2).
///
/// `is_token_character` in `iku_structured_fields/src/token.rs` repeats this
/// set with `:` and `/` added; a change to one is checked against the other.
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

/// `OWS`, `RWS` and `BWS` are made of these (RFC 9110 §5.6.3).
pub(crate) const fn is_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t')
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
}
