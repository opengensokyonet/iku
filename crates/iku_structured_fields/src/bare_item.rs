//! Bare items: the values of items and parameters (RFC 9651 §3.3).

use crate::date::Date;
use crate::number::{Decimal, Integer, Number, parse_number};
use crate::string;
use crate::syntax::{
    DecimalDateSnafu, InvalidBareItemStartSnafu, InvalidBooleanSnafu, Parser, SyntaxError,
};
use crate::token::{Token, is_token_start};
use crate::{byte_sequence, display_string};
use alloc::vec::Vec;
use bytes::Bytes;

/// The value of an item or a parameter, one of the eight types of RFC 9651
/// §3.3.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BareItem {
    /// An integer (§3.3.1).
    Integer(Integer),
    /// A decimal (§3.3.2).
    Decimal(Decimal),
    /// A string of visible ASCII (§3.3.3).
    String(string::String),
    /// A token (§3.3.4).
    Token(Token),
    /// A byte sequence (§3.3.5), decoded.
    ByteSequence(Bytes),
    /// A boolean (§3.3.6).
    Boolean(bool),
    /// A date (§3.3.7).
    Date(Date),
    /// A display string (§3.3.8), decoded: any Unicode text.
    DisplayString(alloc::string::String),
}

impl BareItem {
    /// Parses a bare item (§4.2.3.1).
    pub(crate) fn parse(parser: &mut Parser) -> Result<Self, SyntaxError> {
        let index = parser.index();
        match parser.peek() {
            Some(b'-' | b'0'..=b'9') => Ok(match parse_number(parser)? {
                Number::Integer(integer) => Self::Integer(integer),
                Number::Decimal(decimal) => Self::Decimal(decimal),
            }),
            Some(b'"') => string::String::parse(parser).map(Self::String),
            Some(byte) if is_token_start(byte) => Ok(Self::Token(Token::parse(parser))),
            Some(b':') => byte_sequence::parse(parser).map(Self::ByteSequence),
            Some(b'?') => {
                parser.advance();
                let value = match parser.peek() {
                    Some(b'1') => true,
                    Some(b'0') => false,
                    _ => {
                        return InvalidBooleanSnafu {
                            index: parser.index(),
                        }
                        .fail();
                    }
                };
                parser.advance();
                Ok(Self::Boolean(value))
            }
            Some(b'@') => {
                parser.advance();
                let start = parser.index();
                match parse_number(parser)? {
                    Number::Integer(seconds) => Ok(Self::Date(Date::from_seconds(seconds))),
                    Number::Decimal(_) => DecimalDateSnafu { index: start }.fail(),
                }
            }
            Some(b'%') => display_string::parse(parser).map(Self::DisplayString),
            _ => InvalidBareItemStartSnafu { index }.fail(),
        }
    }

    /// Writes the bare item (§4.1.3.1).
    pub(crate) fn serialize(&self, out: &mut Vec<u8>) {
        match self {
            Self::Integer(integer) => integer.serialize(out),
            Self::Decimal(decimal) => decimal.serialize(out),
            Self::String(string) => string.serialize(out),
            Self::Token(token) => token.serialize(out),
            Self::ByteSequence(bytes) => byte_sequence::serialize(bytes, out),
            Self::Boolean(value) => out.extend_from_slice(if *value { b"?1" } else { b"?0" }),
            Self::Date(date) => date.serialize(out),
            Self::DisplayString(text) => display_string::serialize(text, out),
        }
    }
}

impl From<Integer> for BareItem {
    fn from(integer: Integer) -> Self {
        Self::Integer(integer)
    }
}

impl From<Decimal> for BareItem {
    fn from(decimal: Decimal) -> Self {
        Self::Decimal(decimal)
    }
}

impl From<string::String> for BareItem {
    fn from(string: string::String) -> Self {
        Self::String(string)
    }
}

impl From<Token> for BareItem {
    fn from(token: Token) -> Self {
        Self::Token(token)
    }
}

impl From<bool> for BareItem {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl From<Date> for BareItem {
    fn from(date: Date) -> Self {
        Self::Date(date)
    }
}

#[cfg(test)]
mod tests;
