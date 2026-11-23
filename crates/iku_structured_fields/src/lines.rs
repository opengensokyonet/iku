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
    state: State,
    parse_member: fn(&mut Parser) -> Result<T, SyntaxError>,
}

/// Where a [`LineMembers`] is in the lines.
enum State {
    /// Before a line: the first one, or the one after a line that has ended.
    BeforeLine {
        /// The index that the next line has, from 0.
        line: usize,
    },
    /// In a line that is not empty.
    InLine {
        /// The index of the line.
        line: usize,
        /// The rest of the line.
        parser: Parser,
        /// Whether a member has been read from this line, so that a comma
        /// comes before the next one.
        after_member: bool,
    },
    /// After the last line, or after an error.
    Done,
}

impl<'a, I: Iterator<Item = &'a FieldValue>, T> LineMembers<'a, I, T> {
    pub(crate) fn new(lines: I, parse_member: fn(&mut Parser) -> Result<T, SyntaxError>) -> Self {
        Self {
            lines: lines.peekable(),
            state: State::BeforeLine { line: 0 },
            parse_member,
        }
    }
}

impl<'a, I: Iterator<Item = &'a FieldValue>, T> Iterator for LineMembers<'a, I, T> {
    type Item = Result<T, FieldLinesError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match &mut self.state {
                State::Done => return None,
                State::BeforeLine { line } => {
                    let Some(value) = self.lines.next() else {
                        self.state = State::Done;
                        return None;
                    };
                    let line = *line;
                    if value.is_empty() {
                        // Combined with the other lines, an empty line leaves
                        // two commas together, or one at an end (§4.2).
                        if line > 0 || self.lines.peek().is_some() {
                            self.state = State::Done;
                            return Some(Err(FieldLinesError::EmptyLine { line }));
                        }
                        self.state = State::BeforeLine { line: line + 1 };
                        continue;
                    }
                    self.state = State::InLine {
                        line,
                        parser: Parser::new(value.clone().into_bytes()),
                        after_member: false,
                    };
                }
                State::InLine {
                    line,
                    parser,
                    after_member,
                } => {
                    let line = *line;
                    match next_member(parser, after_member, self.parse_member) {
                        Ok(Some(member)) => return Some(Ok(member)),
                        Ok(None) => self.state = State::BeforeLine { line: line + 1 },
                        Err(source) => {
                            self.state = State::Done;
                            return Some(Err(FieldLinesError::Syntax { line, source }));
                        }
                    }
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
