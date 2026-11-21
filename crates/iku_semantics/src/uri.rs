//! The parts of a target URI that HTTP uses ([RFC 3986]).
//!
//! - [`Scheme`]: `http`, `https` or another scheme, in lowercase.
//! - [`Authority`]: a host and an optional port, without userinfo (RFC 9110
//!   §4.2.4).
//! - [`PathAndQuery`]: an absolute path and an optional query, as HTTP/2 and
//!   HTTP/3 carry them in `:path` and HTTP/1.1 in the origin form.
//!
//! [RFC 3986]: https://www.rfc-editor.org/rfc/rfc3986.html

mod authority;
mod path_and_query;
mod scheme;

pub use authority::{
    Authority, AuthorityError, Host, InvalidIpFutureError, InvalidPortCharacterError,
    InvalidRegNameCharacterError,
};
pub use path_and_query::{InvalidPathCharacterError, PathAndQuery, PathAndQueryError};
pub use scheme::{InvalidSchemeCharacterError, Scheme, SchemeError};
