//! Request and response heads: control data and the header section (RFC
//! 9110 §6).

use crate::field::FieldSection;
use crate::method::Method;
use crate::protocol::Protocol;
use crate::status::StatusCode;
use crate::target::RequestTarget;
use snafu::Snafu;

/// The head of a request: its method, its target, the protocol of an
/// extended CONNECT, and its header section; not its content or trailers.
///
/// Construction checks that the method and the target agree (RFC 9110
/// §7.1): the authority form only with CONNECT, `*` only with OPTIONS, and a
/// CONNECT without a protocol only with the authority form (§9.3.6). A
/// protocol makes the request an extended CONNECT, whose target is a full
/// URI ([RFC 8441] §4, [RFC 9220]).
///
/// [RFC 8441]: https://www.rfc-editor.org/rfc/rfc8441.html
/// [RFC 9220]: https://www.rfc-editor.org/rfc/rfc9220.html
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestHead {
    method: Method,
    target: RequestTarget,
    protocol: Option<Protocol>,
    fields: FieldSection,
}

impl RequestHead {
    /// Puts a request head together.
    ///
    /// # Errors
    ///
    /// [`RequestHeadError`] for the rule that the method, the target and the
    /// protocol break.
    pub fn new(
        method: Method,
        target: RequestTarget,
        protocol: Option<Protocol>,
        fields: FieldSection,
    ) -> Result<Self, RequestHeadError> {
        let connect = method == Method::CONNECT;
        match (&target, &protocol) {
            (RequestTarget::Authority(_), _) if !connect => {
                return AuthorityFormWithoutConnectSnafu.fail();
            }
            (RequestTarget::Authority(_), Some(_)) => return ProtocolWithoutUriSnafu.fail(),
            (RequestTarget::Asterisk, _) if method != Method::OPTIONS => {
                return AsteriskWithoutOptionsSnafu.fail();
            }
            (RequestTarget::Asterisk, Some(_)) => return ProtocolWithoutConnectSnafu.fail(),
            (RequestTarget::Uri(_), Some(_)) if !connect => {
                return ProtocolWithoutConnectSnafu.fail();
            }
            (RequestTarget::Uri(_), None) if connect => {
                return ConnectWithoutAuthorityFormSnafu.fail();
            }
            _ => {}
        }
        Ok(Self {
            method,
            target,
            protocol,
            fields,
        })
    }

    /// The method.
    pub fn method(&self) -> &Method {
        &self.method
    }

    /// The target.
    pub fn target(&self) -> &RequestTarget {
        &self.target
    }

    /// The protocol of an extended CONNECT.
    pub fn protocol(&self) -> Option<&Protocol> {
        self.protocol.as_ref()
    }

    /// The header section.
    pub fn fields(&self) -> &FieldSection {
        &self.fields
    }

    /// The header section, to change.
    pub fn fields_mut(&mut self) -> &mut FieldSection {
        &mut self.fields
    }

    /// The parts, taken apart.
    pub fn into_parts(self) -> (Method, RequestTarget, Option<Protocol>, FieldSection) {
        (self.method, self.target, self.protocol, self.fields)
    }
}

/// The method, the target and the protocol of a request do not agree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum RequestHeadError {
    /// The authority form is only for CONNECT (RFC 9110 §7.1).
    #[snafu(display("the authority form is used with a method other than CONNECT"))]
    AuthorityFormWithoutConnect,
    /// `*` is only for OPTIONS (§7.1).
    #[snafu(display("the asterisk form is used with a method other than OPTIONS"))]
    AsteriskWithoutOptions,
    /// A protocol is only for CONNECT (RFC 8441 §4).
    #[snafu(display("a protocol is given with a method other than CONNECT"))]
    ProtocolWithoutConnect,
    /// An extended CONNECT targets a full URI, not the authority form (RFC
    /// 8441 §4).
    #[snafu(display("an extended CONNECT targets the authority form"))]
    ProtocolWithoutUri,
    /// A CONNECT without a protocol targets the authority form only (RFC 9110
    /// §9.3.6).
    #[snafu(display("a CONNECT without a protocol targets a URI"))]
    ConnectWithoutAuthorityForm,
}

/// The head of a response: its status code and its header section; not its
/// content or trailers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResponseHead {
    status: StatusCode,
    fields: FieldSection,
}

impl ResponseHead {
    /// Puts a response head together.
    pub const fn new(status: StatusCode, fields: FieldSection) -> Self {
        Self { status, fields }
    }

    /// The status code.
    pub const fn status(&self) -> StatusCode {
        self.status
    }

    /// The header section.
    pub fn fields(&self) -> &FieldSection {
        &self.fields
    }

    /// The header section, to change.
    pub fn fields_mut(&mut self) -> &mut FieldSection {
        &mut self.fields
    }

    /// The parts, taken apart.
    pub fn into_parts(self) -> (StatusCode, FieldSection) {
        (self.status, self.fields)
    }
}
