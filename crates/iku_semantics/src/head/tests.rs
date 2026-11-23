use super::*;
use crate::field::{FieldName, FieldValue};
use crate::target::{AsteriskTarget, TargetUri};
use bytes::Bytes;
use iku_uri::{Authority, PathAndQuery, Scheme};

fn uri() -> RequestTarget {
    RequestTarget::Uri(
        TargetUri::new(
            Scheme::HTTPS,
            Some(Authority::new(b"example.com").unwrap()),
            PathAndQuery::ROOT,
        )
        .unwrap(),
    )
}

fn authority_form() -> RequestTarget {
    RequestTarget::Authority(Authority::new(b"example.com:443").unwrap())
}

fn asterisk() -> RequestTarget {
    RequestTarget::Asterisk(
        AsteriskTarget::new(Scheme::HTTPS, Some(Authority::new(b"example.com").unwrap())).unwrap(),
    )
}

fn websocket() -> Option<Protocol> {
    Some(Protocol::from_static("websocket").unwrap())
}

fn new(
    method: Method,
    target: RequestTarget,
    protocol: Option<Protocol>,
) -> Result<RequestHead, RequestHeadError> {
    RequestHead::new(method, target, protocol, FieldSection::new())
}

#[test]
fn request_head_accepts_get_with_a_uri() {
    let head = new(Method::GET, uri(), None).unwrap();
    assert_eq!(head.method(), &Method::GET);
    assert_eq!(head.target(), &uri());
    assert_eq!(head.protocol(), None);
    assert!(head.fields().is_empty());
}

#[test]
fn request_head_accepts_connect_with_the_authority_form() {
    // RFC 9110 §9.3.6.
    assert!(new(Method::CONNECT, authority_form(), None).is_ok());
}

#[test]
fn request_head_accepts_options_with_an_asterisk() {
    // RFC 9110 §7.1, §9.3.7.
    assert!(new(Method::OPTIONS, asterisk(), None).is_ok());
}

#[test]
fn request_head_accepts_options_with_a_uri() {
    assert!(new(Method::OPTIONS, uri(), None).is_ok());
}

#[test]
fn request_head_accepts_an_extended_connect() {
    // RFC 8441 §4.
    let head = new(Method::CONNECT, uri(), websocket()).unwrap();
    assert_eq!(head.protocol(), websocket().as_ref());
}

#[test]
fn request_head_accepts_an_extension_method_with_a_uri() {
    let patch = Method::from_static("PATCH").unwrap();
    assert!(new(patch, uri(), None).is_ok());
}

#[test]
fn request_head_rejects_the_authority_form_without_connect() {
    for method in [Method::GET, Method::OPTIONS, Method::POST] {
        assert_eq!(
            new(method, authority_form(), None),
            Err(RequestHeadError::AuthorityFormWithoutConnect)
        );
    }
    // The method check comes before the protocol check.
    assert_eq!(
        new(Method::GET, authority_form(), websocket()),
        Err(RequestHeadError::AuthorityFormWithoutConnect)
    );
}

#[test]
fn request_head_rejects_an_asterisk_without_options() {
    for method in [Method::GET, Method::CONNECT, Method::POST] {
        assert_eq!(
            new(method, asterisk(), None),
            Err(RequestHeadError::AsteriskWithoutOptions)
        );
    }
}

#[test]
fn request_head_rejects_a_protocol_with_an_asterisk() {
    assert_eq!(
        new(Method::OPTIONS, asterisk(), websocket()),
        Err(RequestHeadError::ProtocolWithoutConnect)
    );
}

#[test]
fn request_head_rejects_a_protocol_with_a_uri_and_another_method() {
    assert_eq!(
        new(Method::GET, uri(), websocket()),
        Err(RequestHeadError::ProtocolWithoutConnect)
    );
}

#[test]
fn request_head_rejects_a_protocol_with_the_authority_form() {
    assert_eq!(
        new(Method::CONNECT, authority_form(), websocket()),
        Err(RequestHeadError::ProtocolWithoutUri)
    );
}

#[test]
fn request_head_rejects_connect_with_a_uri_and_no_protocol() {
    assert_eq!(
        new(Method::CONNECT, uri(), None),
        Err(RequestHeadError::ConnectWithoutAuthorityForm)
    );
}

#[test]
fn request_head_rejects_connect_without_a_port() {
    // No port, and an empty one (RFC 9110 §9.3.6).
    for authority in [&b"example.com"[..], b"example.com:", b"[::1]"] {
        let target = RequestTarget::Authority(Authority::new(authority).unwrap());
        assert_eq!(
            new(Method::CONNECT, target, None),
            Err(RequestHeadError::ConnectWithoutPort),
            "{authority:?}"
        );
    }
    // Port 0 is a port.
    let target = RequestTarget::Authority(Authority::new(b"example.com:0").unwrap());
    assert!(new(Method::CONNECT, target, None).is_ok());
}

#[test]
fn request_head_rejects_connect_with_an_asterisk() {
    assert_eq!(
        new(Method::CONNECT, asterisk(), None),
        Err(RequestHeadError::AsteriskWithoutOptions)
    );
}

#[test]
fn request_head_fields_can_be_changed_and_taken_apart() {
    let mut head = new(Method::GET, uri(), None).unwrap();
    head.fields_mut().append(
        FieldName::HOST,
        FieldValue::new(Bytes::from_static(b"example.com")).unwrap(),
    );
    assert_eq!(head.fields().len(), 1);
    let (method, target, protocol, fields) = head.into_parts();
    assert_eq!(method, Method::GET);
    assert_eq!(target, uri());
    assert_eq!(protocol, None);
    assert_eq!(fields.len(), 1);
}

#[test]
fn request_head_keeps_the_given_fields() {
    let mut fields = FieldSection::new();
    fields.append(FieldName::TE, FieldValue::from_static("trailers").unwrap());
    let head = RequestHead::new(Method::GET, uri(), None, fields.clone()).unwrap();
    assert_eq!(head.fields(), &fields);
}

#[test]
fn request_head_errors_display() {
    use alloc::string::ToString;
    for error in [
        RequestHeadError::AuthorityFormWithoutConnect,
        RequestHeadError::AsteriskWithoutOptions,
        RequestHeadError::ProtocolWithoutConnect,
        RequestHeadError::ProtocolWithoutUri,
        RequestHeadError::ConnectWithoutAuthorityForm,
        RequestHeadError::ConnectWithoutPort,
    ] {
        assert!(!error.to_string().is_empty());
    }
}

#[test]
fn response_head_gives_back_what_it_was_given() {
    let status = StatusCode::new(204).unwrap();
    let mut head = ResponseHead::new(status, FieldSection::new());
    assert_eq!(head.status(), status);
    assert!(head.fields().is_empty());
    head.fields_mut().append(
        FieldName::CONTENT_LENGTH,
        FieldValue::from_static("0").unwrap(),
    );
    assert_eq!(head.fields().len(), 1);
    let (status_back, fields) = head.into_parts();
    assert_eq!(status_back, status);
    assert_eq!(fields.len(), 1);
}

#[test]
fn response_head_new_works_in_const_context() {
    const HEAD: ResponseHead = ResponseHead::new(
        match StatusCode::new(200) {
            Ok(status) => status,
            Err(_) => panic!(),
        },
        FieldSection::new(),
    );
    assert_eq!(HEAD.status().get(), 200);
}
