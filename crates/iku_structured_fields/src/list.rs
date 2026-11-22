//! Lists (RFC 9651 §3.1).

use crate::inner_list::InnerList;
use crate::item::Item;
use crate::lines::{FieldLinesError, LineMembers, next_member};
use crate::syntax::{Parser, SyntaxError};
use alloc::vec::Vec;
use bytes::Bytes;
use iku_semantics::field::{FieldName, FieldSection, FieldValue};

/// A member of a list or a dictionary: an item or an inner list (RFC 9651
/// §3.1, §3.2).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ListMember {
    /// An item.
    Item(Item),
    /// An inner list.
    InnerList(InnerList),
}

impl ListMember {
    /// Parses an item or an inner list (§4.2.1.1).
    pub(crate) fn parse(parser: &mut Parser) -> Result<Self, SyntaxError> {
        if parser.peek() == Some(b'(') {
            InnerList::parse(parser).map(Self::InnerList)
        } else {
            Item::parse(parser).map(Self::Item)
        }
    }

    pub(crate) fn serialize(&self, out: &mut Vec<u8>) {
        match self {
            Self::Item(item) => item.serialize(out),
            Self::InnerList(inner_list) => inner_list.serialize(out),
        }
    }
}

impl From<Item> for ListMember {
    fn from(item: Item) -> Self {
        Self::Item(item)
    }
}

impl From<InnerList> for ListMember {
    fn from(inner_list: InnerList) -> Self {
        Self::InnerList(inner_list)
    }
}

/// A list: items and inner lists in order (RFC 9651 §3.1). A field whose value
/// is a list is a list field.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct List(Vec<ListMember>);

impl List {
    /// The empty list.
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// The members in order.
    pub fn as_slice(&self) -> &[ListMember] {
        &self.0
    }

    /// The members.
    pub fn into_vec(self) -> Vec<ListMember> {
        self.0
    }

    /// Appends `member`.
    pub fn push(&mut self, member: ListMember) {
        self.0.push(member);
    }

    /// How many members there are.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The members of the list field `name` in `fields`, parsed one at a time
    /// over its lines in order, as if they were combined (RFC 9651 §4.2).
    ///
    /// Parsing stops at the first error. When any member fails to parse, the
    /// recipient ignores the whole field (§2.2), so a caller acts on the
    /// members only after the iterator has ended without an error;
    /// [`List::from_fields`] does that.
    ///
    /// A member split over two lines fails to parse, which §4.2 allows.
    pub fn members<'a>(
        fields: &'a FieldSection,
        name: &'a FieldName,
    ) -> impl Iterator<Item = Result<ListMember, FieldLinesError>> + 'a {
        LineMembers::new(fields.get_all(name), ListMember::parse)
    }

    /// Parses the list field `name` in `fields`; `None` when it has no line.
    ///
    /// # Errors
    ///
    /// [`FieldLinesError`] if the lines are not a list. The recipient ignores
    /// the field (§2.2).
    pub fn from_fields(
        fields: &FieldSection,
        name: &FieldName,
    ) -> Result<Option<Self>, FieldLinesError> {
        if !fields.contains(name) {
            return Ok(None);
        }
        Self::members(fields, name)
            .collect::<Result<Vec<_>, _>>()
            .map(|members| Some(Self(members)))
    }

    /// Parses `value` as a list field (§4.2).
    ///
    /// # Errors
    ///
    /// [`SyntaxError`] if `value` is not a list.
    pub fn from_field_value(value: &FieldValue) -> Result<Self, SyntaxError> {
        let mut parser = Parser::new(value.clone().into_bytes());
        let mut after_member = false;
        let mut members = Vec::new();
        while let Some(member) = next_member(&mut parser, &mut after_member, ListMember::parse)? {
            members.push(member);
        }
        Ok(Self(members))
    }

    /// The list as a field value (§4.1); `None` when it is empty, since an
    /// empty list is sent by leaving the field out.
    pub fn to_field_value(&self) -> Option<FieldValue> {
        if self.0.is_empty() {
            return None;
        }
        let mut out = Vec::new();
        for (index, member) in self.0.iter().enumerate() {
            if index > 0 {
                out.extend_from_slice(b", ");
            }
            member.serialize(&mut out);
        }
        // Invariant: a serialized list is visible ASCII and spaces, and starts
        // and ends with neither a space nor a tab.
        Some(FieldValue::new(Bytes::from(out)).expect("a serialized list is a field value"))
    }
}

impl From<Vec<ListMember>> for List {
    fn from(members: Vec<ListMember>) -> Self {
        Self(members)
    }
}

impl FromIterator<ListMember> for List {
    fn from_iter<I: IntoIterator<Item = ListMember>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

#[cfg(test)]
mod tests;
