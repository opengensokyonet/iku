//! Character classes of RFC 3986, as predicates on bytes.

use snafu::Snafu;

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

/// A `%` that is not followed by two hexadecimal digits, so it does not start
/// a `pct-encoded` (RFC 3986 §2.1).
///
/// It is constructed only when the check fails. Authorities and paths share
/// it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(display("a percent sign at {index} is not followed by two hexadecimal digits"))]
#[snafu(context(suffix(false)))]
pub struct InvalidPercentEncodingError {
    index: usize,
}

impl InvalidPercentEncodingError {
    /// Where the `%` is.
    pub const fn index(self) -> usize {
        self.index
    }
}

/// Checks that `bytes[index]`, a `%`, starts a `pct-encoded`.
pub(crate) fn check_pct_encoded_at(
    bytes: &[u8],
    index: usize,
) -> Result<(), InvalidPercentEncodingError> {
    if is_pct_encoded_at(bytes, index) {
        Ok(())
    } else {
        Err(InvalidPercentEncodingError { index })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pct_encoded_needs_two_hex_digits() {
        assert!(is_pct_encoded_at(b"a%2Fb", 1));
        assert!(!is_pct_encoded_at(b"a%2", 1));
        assert!(!is_pct_encoded_at(b"a%G0", 1));
        assert_eq!(check_pct_encoded_at(b"a%2Fb", 1), Ok(()));
        assert_eq!(check_pct_encoded_at(b"a%2", 1).unwrap_err().index(), 1);
    }
}
