//! Items (RFC 9651 §3.3).

use crate::bare_item::BareItem;
use crate::parameters::Parameters;
use crate::syntax::{Parser, SyntaxError, TrailingCharactersSnafu};
use alloc::vec::Vec;
use bytes::Bytes;
use iku_semantics::field::{FieldName, FieldSection, FieldValue};
use snafu::{ResultExt, Snafu};

/// An item: a bare item with parameters (RFC 9651 §3.3). A field whose value
/// is one item is an item field.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Item {
    bare_item: BareItem,
    parameters: Parameters,
}

impl Item {
    /// The item `bare_item` without parameters.
    pub fn new(bare_item: BareItem) -> Self {
        Self::with_parameters(bare_item, Parameters::new())
    }

    /// The item `bare_item` with `parameters`.
    pub const fn with_parameters(bare_item: BareItem, parameters: Parameters) -> Self {
        Self {
            bare_item,
            parameters,
        }
    }

    /// The bare item.
    pub fn bare_item(&self) -> &BareItem {
        &self.bare_item
    }

    /// The parameters.
    pub fn parameters(&self) -> &Parameters {
        &self.parameters
    }

    /// The bare item and the parameters.
    pub fn into_parts(self) -> (BareItem, Parameters) {
        (self.bare_item, self.parameters)
    }

    /// Parses `value` as an item field (RFC 9651 §4.2).
    ///
    /// # Errors
    ///
    /// [`SyntaxError`] if `value` is not one item. The recipient ignores the
    /// field (§2.2).
    pub fn from_field_value(value: &FieldValue) -> Result<Self, SyntaxError> {
        let mut parser = Parser::new(value.clone().into_bytes());
        parser.skip_sp();
        let item = Self::parse(&mut parser)?;
        parser.skip_sp();
        if !parser.is_empty() {
            return TrailingCharactersSnafu {
                index: parser.index(),
            }
            .fail();
        }
        Ok(item)
    }

    /// Parses the line `name` of `fields` as an item field; `None` when there
    /// is no such line.
    ///
    /// # Errors
    ///
    /// [`ItemFieldError`] if there are several lines or the line is not one
    /// item. The recipient ignores the field (§2.2).
    pub fn from_fields(
        fields: &FieldSection,
        name: &FieldName,
    ) -> Result<Option<Self>, ItemFieldError> {
        let mut lines = fields.get_all(name);
        let Some(value) = lines.next() else {
            return Ok(None);
        };
        if lines.next().is_some() {
            return SeveralLinesSnafu.fail();
        }
        Self::from_field_value(value).map(Some).context(SyntaxSnafu)
    }

    /// The item as a field value (§4.1).
    pub fn to_field_value(&self) -> FieldValue {
        let mut out = Vec::new();
        self.serialize(&mut out);
        // Invariant: a serialized item is visible ASCII and spaces, and starts
        // and ends with neither a space nor a tab.
        FieldValue::new(Bytes::from(out)).expect("a serialized item is a field value")
    }

    /// Parses an item (§4.2.3).
    pub(crate) fn parse(parser: &mut Parser) -> Result<Self, SyntaxError> {
        let bare_item = BareItem::parse(parser)?;
        let parameters = Parameters::parse(parser)?;
        Ok(Self::with_parameters(bare_item, parameters))
    }

    /// Writes the item (§4.1.3).
    pub(crate) fn serialize(&self, out: &mut Vec<u8>) {
        self.bare_item.serialize(out);
        self.parameters.serialize(out);
    }
}

impl From<BareItem> for Item {
    fn from(bare_item: BareItem) -> Self {
        Self::new(bare_item)
    }
}

/// The lines of a field are not an item field (RFC 9651 §4.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum ItemFieldError {
    /// There are several lines. Combined, they are separated by commas, and
    /// a comma ends an item (§4.2).
    #[snafu(display("an item field has several lines"))]
    SeveralLines,
    /// The line is not one item.
    #[snafu(display("the field line is not an item"))]
    Syntax {
        /// Which rule it breaks, and where.
        source: SyntaxError,
    },
}

#[cfg(test)]
mod tests;
