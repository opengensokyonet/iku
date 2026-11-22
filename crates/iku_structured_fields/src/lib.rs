//! Structured field values for HTTP ([RFC 9651]), for iku.
//!
//! A structured field is a field whose value has one of three shapes: an
//! item, a list or a dictionary. This package parses field lines into those
//! shapes and serializes them back, with every value checked when it is
//! constructed. The specification of a field says which shape it has and
//! what its members mean; the types of particular fields are built on these.
//!
//! - Shapes: [`Item`], [`List`] (of [`ListMember`]s, each an [`Item`] or an
//!   [`InnerList`]) and [`Dictionary`]; items and inner lists carry
//!   [`Parameters`].
//! - Bare items ([`BareItem`]): [`Integer`], [`Decimal`], [`SfString`],
//!   [`SfToken`], byte sequences, booleans, [`Date`] and display strings;
//!   [`Key`]s name dictionary members and parameters.
//! - Reading: `from_fields` parses the lines of a field in a
//!   [`FieldSection`](iku_semantics::field::FieldSection) as if they were
//!   combined (§4.2); [`List::members`] and [`Dictionary::entries`] give the
//!   members one at a time without combining. Parsing is strict: on any
//!   error the recipient ignores the whole field (§2.2).
//! - Writing: `to_field_value` cannot fail, since the values were checked
//!   when they were constructed.
//!
//! The package does not use the standard library; it needs `alloc`.
//!
//! [RFC 9651]: https://www.rfc-editor.org/rfc/rfc9651.html
#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;

mod bare_item;
mod base64;
mod byte_sequence;
mod date;
mod dictionary;
mod display_string;
mod inner_list;
mod item;
mod key;
mod lines;
mod list;
mod number;
mod parameters;
mod string;
mod syntax;
mod token;

pub use bare_item::BareItem;
pub use date::Date;
pub use dictionary::Dictionary;
pub use inner_list::InnerList;
pub use item::{Item, ItemFieldError};
pub use key::{InvalidKeyCharacterError, InvalidKeyStartError, Key, KeyError};
pub use lines::FieldLinesError;
pub use list::{List, ListMember};
pub use number::{Decimal, DecimalOutOfRangeError, Integer, IntegerOutOfRangeError};
pub use parameters::Parameters;
pub use string::{InvalidSfStringCharacterError, SfString};
pub use syntax::SyntaxError;
pub use token::{InvalidSfTokenCharacterError, InvalidSfTokenStartError, SfToken, SfTokenError};
