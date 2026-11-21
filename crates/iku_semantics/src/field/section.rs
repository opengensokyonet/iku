//! Field sections (RFC 9110 §5.2, §5.3).

use super::{FieldName, FieldValue, list};
use alloc::vec::Vec;
use bytes::{BufMut, BytesMut};
use snafu::Snafu;

/// The field lines of a header or trailer section (RFC 9110 §5.2).
///
/// The lines keep the order they were received or appended in, and a name can
/// appear on several lines: the order of lines with the same name is
/// significant, and an intermediary forwards them as they are (§5.3).
/// Looking a name up goes through the lines in order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FieldSection {
    lines: Vec<(FieldName, FieldValue)>,
}

impl FieldSection {
    /// A section without lines.
    pub const fn new() -> Self {
        Self { lines: Vec::new() }
    }

    /// A section without lines, with room for `capacity` lines.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            lines: Vec::with_capacity(capacity),
        }
    }

    /// The number of lines.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Whether the section has no lines.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Adds a line after the others.
    pub fn append(&mut self, name: FieldName, value: FieldValue) {
        self.lines.push((name, value));
    }

    /// The lines in order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&FieldName, &FieldValue)> {
        self.lines.iter().map(|(name, value)| (name, value))
    }

    /// Whether a line has `name`.
    pub fn contains(&self, name: &FieldName) -> bool {
        self.lines.iter().any(|(line, _)| line == name)
    }

    /// The values of the lines with `name`, in order.
    pub fn get_all<'a>(&'a self, name: &'a FieldName) -> impl Iterator<Item = &'a FieldValue> {
        self.lines
            .iter()
            .filter(move |(line, _)| line == name)
            .map(|(_, value)| value)
    }

    /// The non-empty members of the list-based field `name`, over all its lines
    /// in order (RFC 9110 §5.6.1, §5.3).
    pub fn list_members<'a>(&'a self, name: &'a FieldName) -> impl Iterator<Item = &'a [u8]> {
        self.get_all(name)
            .flat_map(|value| list::members(value.as_bytes()))
    }

    /// The combined field value of `name`: the values of its lines in order,
    /// separated by commas (RFC 9110 §5.3); `None` when no line has `name`.
    ///
    /// A single line is returned without copying.
    ///
    /// # Errors
    ///
    /// [`SetCookieNotCombinableError`] for `set-cookie`, whose lines cannot be
    /// combined (§5.3).
    pub fn combined(
        &self,
        name: &FieldName,
    ) -> Result<Option<FieldValue>, SetCookieNotCombinableError> {
        if *name == FieldName::SET_COOKIE {
            return SetCookieNotCombinableSnafu.fail();
        }
        let mut values = self.get_all(name);
        let Some(first) = values.next() else {
            return Ok(None);
        };
        let Some(second) = values.next() else {
            return Ok(Some(first.clone()));
        };
        let mut combined = BytesMut::new();
        combined.put_slice(first.as_bytes());
        for value in core::iter::once(second).chain(values) {
            // Without a space after the comma, an empty last value does not
            // leave trailing whitespace.
            combined.put_u8(b',');
            combined.put_slice(value.as_bytes());
        }
        // The values hold no control character, the first starts without
        // whitespace and the last ends without it, and a comma is visible.
        Ok(Some(FieldValue::from_checked(combined.freeze())))
    }

    /// Removes the lines with `name` and gives how many there were.
    pub fn remove_all(&mut self, name: &FieldName) -> usize {
        let before = self.lines.len();
        self.lines.retain(|(line, _)| line != name);
        before - self.lines.len()
    }

    /// Keeps only the lines for which `keep` is true, in order.
    pub fn retain(&mut self, mut keep: impl FnMut(&FieldName, &FieldValue) -> bool) {
        self.lines.retain(|(name, value)| keep(name, value));
    }
}

impl IntoIterator for FieldSection {
    type Item = (FieldName, FieldValue);
    type IntoIter = alloc::vec::IntoIter<(FieldName, FieldValue)>;

    fn into_iter(self) -> Self::IntoIter {
        self.lines.into_iter()
    }
}

impl FromIterator<(FieldName, FieldValue)> for FieldSection {
    fn from_iter<I: IntoIterator<Item = (FieldName, FieldValue)>>(lines: I) -> Self {
        Self {
            lines: lines.into_iter().collect(),
        }
    }
}

impl Extend<(FieldName, FieldValue)> for FieldSection {
    fn extend<I: IntoIterator<Item = (FieldName, FieldValue)>>(&mut self, lines: I) {
        self.lines.extend(lines);
    }
}

/// The lines of `set-cookie` cannot be combined into one value (RFC 9110
/// §5.3): each is one cookie, and a cookie can hold a comma.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(display("the lines of set-cookie cannot be combined"))]
pub struct SetCookieNotCombinableError;
