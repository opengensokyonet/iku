//! The parts of a target URI that HTTP uses ([RFC 3986]), checked when they
//! are constructed, for iku.
//!
//! HTTP identifies the target of a request with a URI (RFC 9110 §4, §7.1),
//! and HTTP/1.1, HTTP/2 and HTTP/3 carry its parts: the scheme, the authority,
//! and the path with the query. This package parses those parts and nothing
//! else of RFC 3986: no fragments, no relative references, no resolution.
//!
//! - [`Scheme`]: `http`, `https` or another scheme, in lowercase.
//! - [`Authority`]: a host ([`Host`]) and an optional port, without userinfo
//!   (RFC 9110 §4.2.4).
//! - [`PathAndQuery`]: an absolute path and an optional query, as HTTP/2 and
//!   HTTP/3 carry them in `:path` and HTTP/1.1 in the origin form.
//!
//! The package does not use the standard library; it needs `alloc`.
//!
//! [RFC 3986]: https://www.rfc-editor.org/rfc/rfc3986.html
#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;

mod authority;
mod chars;
mod decimal;
mod path_and_query;
mod scheme;

pub use authority::{
    Authority, AuthorityError, Host, InvalidIpFutureError, InvalidPortCharacterError,
    InvalidRegNameCharacterError,
};
pub use path_and_query::{InvalidPathCharacterError, PathAndQuery, PathAndQueryError};
pub use scheme::{InvalidSchemeCharacterError, Scheme, SchemeError};
