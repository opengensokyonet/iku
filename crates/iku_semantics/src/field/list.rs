//! The members of a list-based field value (RFC 9110 §5.6.1).
//!
//! A list-based field is written `#element`: members separated by commas
//! with optional whitespace, where a recipient ignores empty members
//! (§5.6.1.2). A comma inside a quoted string (§5.6.4) does not separate
//! members. The same list can be split over several field lines with the same
//! name (§5.3); [`FieldSection::list_members`](super::FieldSection::list_members)
//! reads all of them.

use crate::chars::is_whitespace;

/// The non-empty members of one list-based field value, without the
/// whitespace around them.
///
/// The members are not checked against the grammar of any field: a member
/// with an unterminated quoted string runs to the end of the value, and the
/// field that reads it rejects it.
pub fn members(value: &[u8]) -> Members<'_> {
    Members { rest: value }
}

/// The iterator of [`members`].
#[derive(Clone, Debug)]
pub struct Members<'a> {
    rest: &'a [u8],
}

impl<'a> Iterator for Members<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<&'a [u8]> {
        loop {
            if self.rest.is_empty() {
                return None;
            }
            let end = member_end(self.rest);
            let member = trim(&self.rest[..end]);
            self.rest = self.rest.get(end + 1..).unwrap_or_default();
            if !member.is_empty() {
                return Some(member);
            }
        }
    }
}

/// The index of the comma that ends the first member, or the length.
fn member_end(bytes: &[u8]) -> usize {
    let mut quoted = false;
    let mut escaped = false;
    for (index, &byte) in bytes.iter().enumerate() {
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
        } else if byte == b'"' {
            quoted = true;
        } else if byte == b',' {
            return index;
        }
    }
    bytes.len()
}

fn trim(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|&b| !is_whitespace(b))
        .unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|&b| !is_whitespace(b))
        .map_or(start, |i| i + 1);
    &bytes[start..end]
}
