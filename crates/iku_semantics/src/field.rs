//! Fields: names, values and sections (RFC 9110 §5).
//!
//! - [`FieldName`]: a token, lowercased when it is constructed.
//! - [`FieldValue`]: bytes without control characters other than HTAB and
//!   without leading or trailing whitespace.
//! - [`FieldSection`]: the field lines of a header or trailer section, in
//!   their order, with repeated names kept.
//! - [`list`]: the members of a list-based field value (§5.6.1).

pub mod list;
mod name;
mod section;
mod value;

pub use name::FieldName;
pub use section::{FieldSection, SetCookieNotCombinableError};
pub use value::{ControlCharacter, FieldValue, FieldValueError};
