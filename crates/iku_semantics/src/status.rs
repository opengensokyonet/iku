//! Status codes (RFC 9110 §15).

use core::fmt;
use snafu::{ResultExt, Snafu};

/// A status code: a three-digit integer from 100 to 599 (RFC 9110 §15).
///
/// The codes are an open set with a registry (§16.2); the first digit gives
/// the class.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StatusCode(u16);

impl StatusCode {
    /// Checks that `code` is from 100 to 599.
    ///
    /// # Errors
    ///
    /// [`StatusCodeError`] if it is not.
    pub const fn new(code: u16) -> Result<Self, StatusCodeError> {
        if 100 <= code && code <= 599 {
            Ok(Self(code))
        } else {
            Err(StatusCodeError { code })
        }
    }

    /// Reads a status code written as three ASCII digits, as the status line
    /// of HTTP/1.1 and the `:status` pseudo-header field of HTTP/2 and HTTP/3
    /// carry it.
    ///
    /// A client that receives a code outside 100 to 599 should process the
    /// response as a 5xx (§15); [`StatusCodeError::code`] gives the code for
    /// that.
    ///
    /// # Errors
    ///
    /// [`ParseStatusCodeError`] if `digits` is not three ASCII digits, or the
    /// code is not from 100 to 599.
    pub fn parse(digits: &[u8]) -> Result<Self, ParseStatusCodeError> {
        let [hundreds, tens, ones] = digits else {
            return NotThreeDigitsSnafu.fail();
        };
        if !(hundreds.is_ascii_digit() && tens.is_ascii_digit() && ones.is_ascii_digit()) {
            return NotThreeDigitsSnafu.fail();
        }
        let code =
            u16::from(hundreds - b'0') * 100 + u16::from(tens - b'0') * 10 + u16::from(ones - b'0');
        Self::new(code).context(OutOfRangeSnafu)
    }

    /// The code.
    pub const fn get(self) -> u16 {
        self.0
    }

    /// The class, from the first digit (§15).
    pub const fn class(self) -> StatusClass {
        match self.0 / 100 {
            1 => StatusClass::Informational,
            2 => StatusClass::Successful,
            3 => StatusClass::Redirection,
            4 => StatusClass::ClientError,
            // The code is at most 599.
            _ => StatusClass::ServerError,
        }
    }

    /// The code as three ASCII digits.
    pub const fn to_ascii_digits(self) -> [u8; 3] {
        // The code has three digits, so each quotient is below 10.
        [
            b'0' + (self.0 / 100) as u8,
            b'0' + (self.0 / 10 % 10) as u8,
            b'0' + (self.0 % 10) as u8,
        ]
    }
}

impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Debug for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StatusCode({})", self.0)
    }
}

/// The class of a status code (RFC 9110 §15).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StatusClass {
    /// 1xx (§15.2).
    Informational,
    /// 2xx (§15.3).
    Successful,
    /// 3xx (§15.4).
    Redirection,
    /// 4xx (§15.5).
    ClientError,
    /// 5xx (§15.6).
    ServerError,
}

/// A status code is not from 100 to 599 (RFC 9110 §15).
///
/// It is constructed only when the check fails, so its code is always outside
/// that range.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(display("status code {code} is not from 100 to 599"))]
pub struct StatusCodeError {
    code: u16,
}

impl StatusCodeError {
    /// The code, outside 100 to 599.
    pub const fn code(self) -> u16 {
        self.code
    }
}

/// Bytes do not hold a status code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum ParseStatusCodeError {
    /// A status code is written as exactly three ASCII digits (§15).
    #[snafu(display("a status code is not three digits"))]
    NotThreeDigits,
    /// The three digits are a code outside 100 to 599.
    #[snafu(display("the status code is out of range"))]
    OutOfRange {
        /// The code.
        source: StatusCodeError,
    },
}

#[cfg(test)]
mod tests;
