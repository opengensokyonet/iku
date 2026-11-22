//! The members of a list or a dictionary over the lines of a field (RFC 9651
//! §4.2).

use crate::syntax::{ExpectedCommaSnafu, Parser, SyntaxError, TrailingCommaSnafu};
use core::iter::Peekable;
use iku_semantics::field::FieldValue;
use snafu::Snafu;

/// Parses the next member of a list or a dictionary in `parser` (§4.2.1,
/// §4.2.2): `None` at the end of the line. `after_member` tells whether a
/// member has been read from this line, so that a comma comes first.
pub(crate) fn next_member<T>(
    parser: &mut Parser,
    after_member: &mut bool,
    parse_member: fn(&mut Parser) -> Result<T, SyntaxError>,
) -> Result<Option<T>, SyntaxError> {
    if *after_member {
        parser.skip_ows();
        if parser.is_empty() {
            return Ok(None);
        }
        if parser.peek() != Some(b',') {
            return ExpectedCommaSnafu {
                index: parser.index(),
            }
            .fail();
        }
        parser.advance();
        parser.skip_ows();
        if parser.is_empty() {
            return TrailingCommaSnafu {
                index: parser.index(),
            }
            .fail();
        }
    } else {
        parser.skip_sp();
        if parser.is_empty() {
            return Ok(None);
        }
    }
    let member = parse_member(parser)?;
    *after_member = true;
    Ok(Some(member))
}

/// The members of a list or a dictionary over several field lines, parsed
/// one at a time, as if the lines were combined with commas (§4.2).
pub(crate) struct LineMembers<'a, I: Iterator<Item = &'a FieldValue>, T> {
    lines: Peekable<I>,
    /// The index of the current line, once there is one.
    line: Option<usize>,
    current: Option<Parser>,
    after_member: bool,
    done: bool,
    parse_member: fn(&mut Parser) -> Result<T, SyntaxError>,
}

impl<'a, I: Iterator<Item = &'a FieldValue>, T> LineMembers<'a, I, T> {
    pub(crate) fn new(lines: I, parse_member: fn(&mut Parser) -> Result<T, SyntaxError>) -> Self {
        Self {
            lines: lines.peekable(),
            line: None,
            current: None,
            after_member: false,
            done: false,
            parse_member,
        }
    }
}

impl<'a, I: Iterator<Item = &'a FieldValue>, T> Iterator for LineMembers<'a, I, T> {
    type Item = Result<T, FieldLinesError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        loop {
            let Some(parser) = &mut self.current else {
                let value = self.lines.next()?;
                let line = self.line.map_or(0, |line| line + 1);
                self.line = Some(line);
                if value.is_empty() {
                    // Combined with the other lines, an empty line leaves
                    // two commas together, or one at an end (§4.2).
                    if line > 0 || self.lines.peek().is_some() {
                        self.done = true;
                        return Some(Err(FieldLinesError::EmptyLine { line }));
                    }
                    continue;
                }
                self.current = Some(Parser::new(value.clone().into_bytes()));
                self.after_member = false;
                continue;
            };
            match next_member(parser, &mut self.after_member, self.parse_member) {
                Ok(Some(member)) => return Some(Ok(member)),
                Ok(None) => self.current = None,
                Err(source) => {
                    self.done = true;
                    return Some(Err(FieldLinesError::Syntax {
                        line: self.line.unwrap_or_default(),
                        source,
                    }));
                }
            }
        }
    }
}

/// The lines of a field are not a list or a dictionary (RFC 9651 §4.2).
///
/// `line` is the index of the line among the lines of the field, from 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
pub enum FieldLinesError {
    /// A line is not well-formed.
    #[snafu(display("field line {line} is not well-formed"))]
    Syntax {
        /// Which line.
        line: usize,
        /// Which rule it breaks, and where.
        source: SyntaxError,
    },
    /// One of several lines is empty, so the combined value has a comma with
    /// no member on one side of it.
    #[snafu(display("field line {line} is empty, one of several"))]
    EmptyLine {
        /// Which line.
        line: usize,
    },
}

#[cfg(test)]
mod tests;
