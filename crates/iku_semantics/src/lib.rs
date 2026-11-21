//! The value types of HTTP semantics ([RFC 9110]) that HTTP/1.1, HTTP/2 and
//! HTTP/3 share, for iku.
//!
//! The package models the head of a message: its control data and its header
//! or trailer section. Content is not here: each HTTP version carries content
//! as a byte stream of its own. Every value is checked when it is constructed,
//! so a value of these types always satisfies the rules of RFC 9110 that do not
//! depend on the HTTP version; the rules of one version (lowercase field names
//! and pseudo-header fields in HTTP/2 and HTTP/3, the request line of
//! HTTP/1.1) are checked by that version before it constructs these values.
//!
//! - [`token`]: tokens ([`token::Token`], RFC 9110 §5.6.2), which methods and
//!   field names are made of.
//! - [`field`]: field names ([`field::FieldName`]), field values
//!   ([`field::FieldValue`]) and field sections ([`field::FieldSection`]),
//!   with the members of list-based field values.
//! - [`method`]: request methods ([`method::Method`]).
//! - [`status`]: status codes ([`status::StatusCode`]).
//! - [`uri`]: the parts of a target URI that HTTP uses: [`uri::Scheme`],
//!   [`uri::Authority`] and [`uri::PathAndQuery`] ([RFC 3986]).
//! - [`target`]: request targets ([`target::RequestTarget`]).
//! - [`protocol`]: the protocol of an extended CONNECT request
//!   ([`protocol::Protocol`], [RFC 8441] §4, [RFC 9220]).
//! - [`head`]: request heads ([`head::RequestHead`]) and response heads
//!   ([`head::ResponseHead`]).
//!
//! The package does not use the standard library; it needs `alloc`.
//!
//! [RFC 3986]: https://www.rfc-editor.org/rfc/rfc3986.html
//! [RFC 8441]: https://www.rfc-editor.org/rfc/rfc8441.html
//! [RFC 9110]: https://www.rfc-editor.org/rfc/rfc9110.html
//! [RFC 9220]: https://www.rfc-editor.org/rfc/rfc9220.html
#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;

mod chars;

pub mod field;
pub mod head;
pub mod method;
pub mod protocol;
pub mod status;
pub mod target;
pub mod token;
pub mod uri;
