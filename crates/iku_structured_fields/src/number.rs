//! Integers and decimals (RFC 9651 §3.3.1, §3.3.2), which one algorithm
//! parses (§4.2.4).

use crate::syntax::{
    EmptyFractionSnafu, FractionTooLongSnafu, IntegerPartTooLongSnafu, IntegerTooLongSnafu,
    MissingDigitSnafu, Parser, SyntaxError, put_decimal,
};
use alloc::vec::Vec;
use core::fmt;
use snafu::Snafu;

/// The largest magnitude of an integer: fifteen nines (§3.3.1).
const INTEGER_LIMIT: i64 = 999_999_999_999_999;

/// An integer from -999,999,999,999,999 to 999,999,999,999,999 (RFC 9651
/// §3.3.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Integer(i64);

impl Integer {
    /// The smallest integer, -999,999,999,999,999.
    pub const MIN: Self = Self(-INTEGER_LIMIT);
    /// The largest integer, 999,999,999,999,999.
    pub const MAX: Self = Self(INTEGER_LIMIT);

    /// Checks that `value` has at most fifteen digits.
    ///
    /// # Errors
    ///
    /// [`IntegerOutOfRangeError`] if it has more.
    pub const fn new(value: i64) -> Result<Self, IntegerOutOfRangeError> {
        if value < -INTEGER_LIMIT || value > INTEGER_LIMIT {
            return Err(IntegerOutOfRangeError { value });
        }
        Ok(Self(value))
    }

    /// The value.
    pub const fn get(self) -> i64 {
        self.0
    }

    pub(crate) fn serialize(self, out: &mut Vec<u8>) {
        if self.0 < 0 {
            out.push(b'-');
        }
        put_decimal(out, self.0.unsigned_abs());
    }
}

impl fmt::Display for Integer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// The value has more than fifteen digits, so it is not an integer (RFC 9651
/// §3.3.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(display("{value} has more than 15 digits"))]
pub struct IntegerOutOfRangeError {
    value: i64,
}

impl IntegerOutOfRangeError {
    /// The value, whose magnitude is above 999,999,999,999,999.
    pub const fn value(self) -> i64 {
        self.value
    }
}

/// A decimal with at most twelve digits before its point and three after it
/// (RFC 9651 §3.3.2), held exactly as a count of thousandths.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Decimal(i64);

impl Decimal {
    /// The smallest decimal, -999,999,999,999.999.
    pub const MIN: Self = Self(-INTEGER_LIMIT);
    /// The largest decimal, 999,999,999,999.999.
    pub const MAX: Self = Self(INTEGER_LIMIT);

    /// The decimal `thousandths / 1000`: `1500` is 1.5.
    ///
    /// # Errors
    ///
    /// [`DecimalOutOfRangeError`] if it has more than twelve digits before its
    /// point.
    pub const fn from_thousandths(thousandths: i64) -> Result<Self, DecimalOutOfRangeError> {
        if thousandths < -INTEGER_LIMIT || thousandths > INTEGER_LIMIT {
            return Err(DecimalOutOfRangeError { thousandths });
        }
        Ok(Self(thousandths))
    }

    /// The value in thousandths.
    pub const fn thousandths(self) -> i64 {
        self.0
    }

    /// Writes the integer component, `.`, and the fractional component without
    /// trailing zeros, or `0` when it is zero (§4.1.5).
    pub(crate) fn serialize(self, out: &mut Vec<u8>) {
        if self.0 < 0 {
            out.push(b'-');
        }
        let magnitude = self.0.unsigned_abs();
        put_decimal(out, magnitude / 1000);
        out.push(b'.');
        let mut fraction = magnitude % 1000;
        if fraction == 0 {
            out.push(b'0');
            return;
        }
        let mut digits = 3;
        while fraction % 10 == 0 {
            fraction /= 10;
            digits -= 1;
        }
        let mut written = [b'0'; 3];
        for slot in written[..digits].iter_mut().rev() {
            // `fraction % 10` is a single digit.
            *slot = b'0' + (fraction % 10) as u8;
            fraction /= 10;
        }
        out.extend_from_slice(&written[..digits]);
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = Vec::new();
        self.serialize(&mut out);
        // The serialization is ASCII.
        out.iter()
            .try_for_each(|&byte| fmt::Write::write_char(f, char::from(byte)))
    }
}

/// The value has more than twelve digits before its point, so it is not a
/// decimal (RFC 9651 §3.3.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(display("{thousandths} thousandths has more than 12 integer digits"))]
pub struct DecimalOutOfRangeError {
    thousandths: i64,
}

impl DecimalOutOfRangeError {
    /// The value in thousandths, whose magnitude is above
    /// 999,999,999,999,999.
    pub const fn thousandths(self) -> i64 {
        self.thousandths
    }
}

/// An integer or a decimal, as §4.2.4 parses them.
pub(crate) enum Number {
    Integer(Integer),
    Decimal(Decimal),
}

/// Parses an integer or a decimal (§4.2.4); the parser is at `-` or a digit.
pub(crate) fn parse_number(parser: &mut Parser) -> Result<Number, SyntaxError> {
    let negative = parser.peek() == Some(b'-');
    if negative {
        parser.advance();
    }
    if !parser.peek().is_some_and(|byte| byte.is_ascii_digit()) {
        return MissingDigitSnafu {
            index: parser.index(),
        }
        .fail();
    }
    let mut integer: i64 = 0;
    let mut integer_digits = 0;
    // The fractional digits read so far and how many there are, once there is
    // a decimal point.
    let mut fraction: Option<(i64, u32)> = None;
    loop {
        let index = parser.index();
        match parser.peek() {
            Some(byte) if byte.is_ascii_digit() => {
                let digit = i64::from(byte - b'0');
                match &mut fraction {
                    None => {
                        integer_digits += 1;
                        if integer_digits > 15 {
                            return IntegerTooLongSnafu { index }.fail();
                        }
                        // At most fifteen digits, so this stays below 10^15.
                        integer = integer * 10 + digit;
                    }
                    Some((value, digits)) => {
                        *digits += 1;
                        if *digits > 3 {
                            return FractionTooLongSnafu { index }.fail();
                        }
                        *value = *value * 10 + digit;
                    }
                }
                parser.advance();
            }
            Some(b'.') if fraction.is_none() => {
                if integer_digits > 12 {
                    return IntegerPartTooLongSnafu { index }.fail();
                }
                fraction = Some((0, 0));
                parser.advance();
            }
            _ => break,
        }
    }
    let sign = if negative { -1 } else { 1 };
    match fraction {
        None => Ok(Number::Integer(Integer(sign * integer))),
        Some((_, 0)) => EmptyFractionSnafu {
            index: parser.index() - 1,
        }
        .fail(),
        Some((value, digits)) => {
            // Twelve integer digits and three fractional digits fit in 10^15.
            let thousandths = integer * 1000 + value * 10_i64.pow(3 - digits);
            Ok(Number::Decimal(Decimal(sign * thousandths)))
        }
    }
}

#[cfg(test)]
mod tests;
