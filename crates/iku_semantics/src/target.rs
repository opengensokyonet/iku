//! Request targets (RFC 9110 §7.1).

use iku_uri::{Authority, PathAndQuery, Scheme};
use snafu::Snafu;

/// The target of a request (RFC 9110 §7.1).
///
/// It is the parsed components of the target URI, or one of the two forms
/// that only one method uses: the host and port of a CONNECT tunnel, and the
/// `*` of OPTIONS. Reconstructing the target URI from what a message carries
/// (the origin form and `Host` of HTTP/1.1, the `:scheme`, `:authority` and
/// `:path` of HTTP/2 and HTTP/3) belongs to each HTTP version.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RequestTarget {
    /// The components of a target URI.
    Uri(TargetUri),
    /// The destination of a CONNECT tunnel (§9.3.6).
    Authority(Authority),
    /// `*`: the server rather than a resource, for OPTIONS (§9.3.7).
    Asterisk,
}

/// The components of a target URI that HTTP uses: scheme, authority, path
/// and query (RFC 9110 §7.1).
///
/// An `http` or `https` URI has an authority (§4.2.1, §4.2.2); a URI of
/// another scheme can lack one.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TargetUri {
    scheme: Scheme,
    authority: Option<Authority>,
    path_and_query: PathAndQuery,
}

impl TargetUri {
    /// Puts the components together.
    ///
    /// # Errors
    ///
    /// [`MissingAuthorityError`] if the scheme is `http` or `https` and there
    /// is no authority.
    pub fn new(
        scheme: Scheme,
        authority: Option<Authority>,
        path_and_query: PathAndQuery,
    ) -> Result<Self, MissingAuthorityError> {
        if authority.is_none() && scheme.is_http_or_https() {
            return MissingAuthoritySnafu.fail();
        }
        Ok(Self {
            scheme,
            authority,
            path_and_query,
        })
    }

    /// The scheme.
    pub fn scheme(&self) -> &Scheme {
        &self.scheme
    }

    /// The authority, if there is one.
    pub fn authority(&self) -> Option<&Authority> {
        self.authority.as_ref()
    }

    /// The path and the query.
    pub fn path_and_query(&self) -> &PathAndQuery {
        &self.path_and_query
    }

    /// The components, taken apart.
    pub fn into_parts(self) -> (Scheme, Option<Authority>, PathAndQuery) {
        (self.scheme, self.authority, self.path_and_query)
    }
}

/// An `http` or `https` URI has no authority (RFC 9110 §4.2.1, §4.2.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(display("an http or https target URI has no authority"))]
pub struct MissingAuthorityError;

#[cfg(test)]
mod tests;
