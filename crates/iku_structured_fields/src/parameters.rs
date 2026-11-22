//! Parameters (RFC 9651 §3.1.2).

use crate::bare_item::BareItem;
use crate::key::Key;
use crate::syntax::{Parser, SyntaxError};
use alloc::vec::Vec;

/// The parameters of an item or an inner list: an ordered map from keys to
/// bare items (RFC 9651 §3.1.2).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Parameters(Vec<(Key, BareItem)>);

impl Parameters {
    /// No parameters.
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// How many parameters there are.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The value of `key`.
    pub fn get(&self, key: &str) -> Option<&BareItem> {
        self.0
            .iter()
            .find(|(name, _)| name.as_str() == key)
            .map(|(_, value)| value)
    }

    /// Sets `key` to `value`: in its place when `key` is there, returning the
    /// old value, and last otherwise.
    pub fn insert(&mut self, key: Key, value: BareItem) -> Option<BareItem> {
        match self.0.iter_mut().find(|(name, _)| *name == key) {
            Some((_, old)) => Some(core::mem::replace(old, value)),
            None => {
                self.0.push((key, value));
                None
            }
        }
    }

    /// The parameters in order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&Key, &BareItem)> {
        self.0.iter().map(|(key, value)| (key, value))
    }

    /// Parses the parameters (§4.2.3.2). A key that is already there gets the
    /// later value and keeps its place.
    pub(crate) fn parse(parser: &mut Parser) -> Result<Self, SyntaxError> {
        let mut parameters = Self::new();
        while parser.peek() == Some(b';') {
            parser.advance();
            parser.skip_sp();
            let key = Key::parse(parser)?;
            let value = if parser.peek() == Some(b'=') {
                parser.advance();
                BareItem::parse(parser)?
            } else {
                BareItem::Boolean(true)
            };
            parameters.insert(key, value);
        }
        Ok(parameters)
    }

    /// Writes `;key` for a true boolean and `;key=value` otherwise (§4.1.1.2).
    pub(crate) fn serialize(&self, out: &mut Vec<u8>) {
        for (key, value) in &self.0 {
            out.push(b';');
            key.serialize(out);
            if *value != BareItem::Boolean(true) {
                out.push(b'=');
                value.serialize(out);
            }
        }
    }
}

impl FromIterator<(Key, BareItem)> for Parameters {
    /// Collects the parameters; a later value of a key replaces the earlier
    /// one in its place.
    fn from_iter<I: IntoIterator<Item = (Key, BareItem)>>(iter: I) -> Self {
        let mut parameters = Self::new();
        for (key, value) in iter {
            parameters.insert(key, value);
        }
        parameters
    }
}

#[cfg(test)]
mod tests;
