//! Dictionaries (RFC 9651 §3.2).

use crate::bare_item::BareItem;
use crate::item::Item;
use crate::key::Key;
use crate::lines::{FieldLinesError, LineMembers, next_member};
use crate::list::ListMember;
use crate::parameters::Parameters;
use crate::syntax::{Parser, SyntaxError};
use alloc::vec::Vec;
use bytes::Bytes;
use iku_semantics::field::{FieldName, FieldSection, FieldValue};

/// A dictionary: an ordered map from keys to items and inner lists (RFC 9651
/// §3.2). A field whose value is a dictionary is a dictionary field.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Dictionary(Vec<(Key, ListMember)>);

impl Dictionary {
    /// The empty dictionary.
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// How many members there are.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The value of `key`.
    pub fn get(&self, key: &str) -> Option<&ListMember> {
        self.0
            .iter()
            .find(|(name, _)| name.as_str() == key)
            .map(|(_, value)| value)
    }

    /// Sets `key` to `value`: in its place when `key` is there, returning the
    /// old value, and last otherwise.
    pub fn insert(&mut self, key: Key, value: ListMember) -> Option<ListMember> {
        match self.0.iter_mut().find(|(name, _)| *name == key) {
            Some((_, old)) => Some(core::mem::replace(old, value)),
            None => {
                self.0.push((key, value));
                None
            }
        }
    }

    /// The members in order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&Key, &ListMember)> {
        self.0.iter().map(|(key, value)| (key, value))
    }

    /// The members of the dictionary field `name` in `fields`, parsed one at a
    /// time over its lines in order, as if they were combined (RFC 9651
    /// §4.2). A key that appears more than once is given each time.
    ///
    /// Parsing stops at the first error. When any member fails to parse, the
    /// recipient ignores the whole field (§2.2), so a caller acts on the
    /// members only after the iterator has ended without an error;
    /// [`Dictionary::from_fields`] does that.
    ///
    /// A member split over two lines fails to parse, which §4.2 allows.
    pub fn entries<'a>(
        fields: &'a FieldSection,
        name: &'a FieldName,
    ) -> impl Iterator<Item = Result<(Key, ListMember), FieldLinesError>> + 'a {
        LineMembers::new(fields.get_all(name), parse_entry)
    }

    /// Parses the dictionary field `name` in `fields`; `None` when it has no
    /// line. A key that appears more than once gets its last value and keeps
    /// its first place (§4.2.2).
    ///
    /// # Errors
    ///
    /// [`FieldLinesError`] if the lines are not a dictionary. The recipient
    /// ignores the field (§2.2).
    pub fn from_fields(
        fields: &FieldSection,
        name: &FieldName,
    ) -> Result<Option<Self>, FieldLinesError> {
        if !fields.contains(name) {
            return Ok(None);
        }
        Self::entries(fields, name)
            .collect::<Result<Self, _>>()
            .map(Some)
    }

    /// Parses `value` as a dictionary field (§4.2). A key that appears more
    /// than once gets its last value and keeps its first place.
    ///
    /// # Errors
    ///
    /// [`SyntaxError`] if `value` is not a dictionary.
    pub fn from_field_value(value: &FieldValue) -> Result<Self, SyntaxError> {
        let mut parser = Parser::new(value.clone().into_bytes());
        let mut after_member = false;
        let mut dictionary = Self::new();
        while let Some((key, value)) = next_member(&mut parser, &mut after_member, parse_entry)? {
            dictionary.insert(key, value);
        }
        Ok(dictionary)
    }

    /// The dictionary as a field value (§4.1); `None` when it is empty, since
    /// an empty dictionary is sent by leaving the field out.
    pub fn to_field_value(&self) -> Option<FieldValue> {
        if self.0.is_empty() {
            return None;
        }
        let mut out = Vec::new();
        for (index, (key, value)) in self.0.iter().enumerate() {
            if index > 0 {
                out.extend_from_slice(b", ");
            }
            key.serialize(&mut out);
            match value {
                // A true boolean is written as the key alone (§4.1.2).
                ListMember::Item(item) if *item.bare_item() == BareItem::Boolean(true) => {
                    item.parameters().serialize(&mut out);
                }
                _ => {
                    out.push(b'=');
                    value.serialize(&mut out);
                }
            }
        }
        // Invariant: a serialized dictionary is visible ASCII and spaces, and
        // starts and ends with neither a space nor a tab.
        Some(FieldValue::new(Bytes::from(out)).expect("a serialized dictionary is a field value"))
    }
}

/// Parses a member of a dictionary (§4.2.2): a key, then `=` and an item or
/// an inner list, or parameters of a true boolean.
fn parse_entry(parser: &mut Parser) -> Result<(Key, ListMember), SyntaxError> {
    let key = Key::parse(parser)?;
    let value = if parser.peek() == Some(b'=') {
        parser.advance();
        ListMember::parse(parser)?
    } else {
        let parameters = Parameters::parse(parser)?;
        ListMember::Item(Item::with_parameters(BareItem::Boolean(true), parameters))
    };
    Ok((key, value))
}

impl FromIterator<(Key, ListMember)> for Dictionary {
    /// Collects the members; a later value of a key replaces the earlier one
    /// in its place.
    fn from_iter<I: IntoIterator<Item = (Key, ListMember)>>(iter: I) -> Self {
        let mut dictionary = Self::new();
        for (key, value) in iter {
            dictionary.insert(key, value);
        }
        dictionary
    }
}

#[cfg(test)]
mod tests;
