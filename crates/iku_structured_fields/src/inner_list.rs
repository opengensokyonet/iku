//! Inner lists (RFC 9651 §3.1.1).

use crate::item::Item;
use crate::parameters::Parameters;
use crate::syntax::{ExpectedInnerListDelimiterSnafu, Parser, SyntaxError, UnclosedInnerListSnafu};
use alloc::vec::Vec;

/// An inner list: items in parentheses, with parameters of its own (RFC 9651
/// §3.1.1).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct InnerList {
    items: Vec<Item>,
    parameters: Parameters,
}

impl InnerList {
    /// The inner list of `items` without parameters.
    pub fn new(items: Vec<Item>) -> Self {
        Self::with_parameters(items, Parameters::new())
    }

    /// The inner list of `items` with `parameters`.
    pub const fn with_parameters(items: Vec<Item>, parameters: Parameters) -> Self {
        Self { items, parameters }
    }

    /// The items in order.
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// The parameters of the inner list.
    pub fn parameters(&self) -> &Parameters {
        &self.parameters
    }

    /// The items and the parameters.
    pub fn into_parts(self) -> (Vec<Item>, Parameters) {
        (self.items, self.parameters)
    }

    /// Parses an inner list (§4.2.1.2); the parser is at `(`.
    pub(crate) fn parse(parser: &mut Parser) -> Result<Self, SyntaxError> {
        let open = parser.index();
        parser.advance();
        let mut items = Vec::new();
        loop {
            parser.skip_sp();
            match parser.peek() {
                None => return UnclosedInnerListSnafu { index: open }.fail(),
                Some(b')') => {
                    parser.advance();
                    let parameters = Parameters::parse(parser)?;
                    return Ok(Self::with_parameters(items, parameters));
                }
                Some(_) => {
                    items.push(Item::parse(parser)?);
                    match parser.peek() {
                        Some(b' ' | b')') => {}
                        None => return UnclosedInnerListSnafu { index: open }.fail(),
                        Some(_) => {
                            return ExpectedInnerListDelimiterSnafu {
                                index: parser.index(),
                            }
                            .fail();
                        }
                    }
                }
            }
        }
    }

    /// Writes `(`, the items separated by spaces, `)` and the parameters
    /// (§4.1.1.1).
    pub(crate) fn serialize(&self, out: &mut Vec<u8>) {
        out.push(b'(');
        for (index, item) in self.items.iter().enumerate() {
            if index > 0 {
                out.push(b' ');
            }
            item.serialize(out);
        }
        out.push(b')');
        self.parameters.serialize(out);
    }
}

#[cfg(test)]
mod tests;
