//! The `Host` field (RFC 9110 §7.2).

use crate::field::{FieldName, FieldSection, FieldValue};
use crate::uri::{Authority, AuthorityError};
use bytes::Bytes;
use snafu::{ResultExt, Snafu};

/// The value of a `Host` field: the authority of the target URI, or empty
/// (RFC 9110 §7.2).
///
/// HTTP/1.1 reconstructs the target URI from it ([RFC 9112 §3.3]); in HTTP/2
/// and HTTP/3 it must agree with `:authority` (RFC 9110 §7.2). An HTTP/1.1
/// client sends it empty when the target URI has no authority ([RFC 9112
/// §3.2]).
///
/// [RFC 9112 §3.2]: https://www.rfc-editor.org/rfc/rfc9112.html#section-3.2
/// [RFC 9112 §3.3]: https://www.rfc-editor.org/rfc/rfc9112.html#section-3.3
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum HostField {
    /// The field value is empty.
    Empty,
    /// The field value is an authority, `uri-host [ ":" port ]`.
    Authority(Authority),
}

impl HostField {
    /// Reads the `Host` line of `fields`; `None` when there is none.
    ///
    /// # Errors
    ///
    /// [`HostFieldError`] if there are several `Host` lines or the value is not
    /// an authority.
    pub fn from_fields(fields: &FieldSection) -> Result<Option<Self>, HostFieldError> {
        let name = FieldName::HOST;
        let mut values = fields.get_all(&name);
        let Some(value) = values.next() else {
            return Ok(None);
        };
        if values.next().is_some() {
            return RepeatedSnafu.fail();
        }
        if value.is_empty() {
            return Ok(Some(Self::Empty));
        }
        Authority::new(value.as_bytes())
            .map(|authority| Some(Self::Authority(authority)))
            .context(InvalidAuthoritySnafu)
    }

    /// The authority; `None` when the value is empty.
    pub fn authority(&self) -> Option<&Authority> {
        match self {
            Self::Empty => None,
            Self::Authority(authority) => Some(authority),
        }
    }

    /// The field value.
    pub fn to_field_value(&self) -> FieldValue {
        match self {
            Self::Empty => FieldValue::from_checked(Bytes::new()),
            // An authority holds no whitespace or control character.
            Self::Authority(authority) => FieldValue::from_checked(authority.to_bytes()),
        }
    }
}

/// The `Host` lines do not give one host (RFC 9110 §7.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum HostFieldError {
    /// There are several `Host` lines. `Host` is not a list (RFC 9110 §7.2,
    /// §5.3), and an HTTP/1.1 server must reject such a request ([RFC 9112
    /// §3.2]).
    ///
    /// [RFC 9112 §3.2]: https://www.rfc-editor.org/rfc/rfc9112.html#section-3.2
    #[snafu(display("there are several host lines"))]
    Repeated,
    /// The value is neither empty nor an authority.
    #[snafu(display("the host value is not an authority"))]
    InvalidAuthority {
        /// Why it is not.
        source: AuthorityError,
    },
}

#[cfg(test)]
mod tests;
