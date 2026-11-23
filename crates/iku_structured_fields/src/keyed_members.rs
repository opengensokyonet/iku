//! The members of an ordered map from keys, collected from members that can
//! repeat a key: dictionaries (RFC 9651 §4.2.2) and parameters (§4.2.3.2).

use crate::key::Key;
use alloc::collections::BTreeMap;
use alloc::collections::btree_map::Entry;
use alloc::vec::Vec;

/// Members in the order of their first key, a later value of a key replacing
/// the earlier one in its place.
///
/// The members come from the remote peer, so finding a key goes through an
/// index from each key to its place instead of a search of the members, and
/// collecting `n` members takes `O(n log n)` time.
pub(crate) struct KeyedMembers<V> {
    members: Vec<(Key, V)>,
    /// The place of each key in `members`.
    places: BTreeMap<Key, usize>,
}

impl<V> KeyedMembers<V> {
    pub(crate) const fn new() -> Self {
        Self {
            members: Vec::new(),
            places: BTreeMap::new(),
        }
    }

    /// Sets `key` to `value`: in its place when `key` is there, and last
    /// otherwise.
    pub(crate) fn push(&mut self, key: Key, value: V) {
        match self.places.entry(key) {
            Entry::Occupied(place) => {
                // Invariant: every place in `places` is the index of a member,
                // since members are only pushed.
                self.members[*place.get()].1 = value;
            }
            Entry::Vacant(place) => {
                self.members.push((place.key().clone(), value));
                place.insert(self.members.len() - 1);
            }
        }
    }

    /// The members in order.
    pub(crate) fn into_members(self) -> Vec<(Key, V)> {
        self.members
    }
}

impl<V> FromIterator<(Key, V)> for KeyedMembers<V> {
    fn from_iter<I: IntoIterator<Item = (Key, V)>>(iter: I) -> Self {
        let mut members = Self::new();
        for (key, value) in iter {
            members.push(key, value);
        }
        members
    }
}
