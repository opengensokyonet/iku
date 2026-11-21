use super::*;
use alloc::format;

const ALL: [Method; 8] = [
    Method::GET,
    Method::HEAD,
    Method::POST,
    Method::PUT,
    Method::DELETE,
    Method::CONNECT,
    Method::OPTIONS,
    Method::TRACE,
];

#[test]
fn method_constants_are_tokens_with_their_names() {
    // RFC 9110 §9.3.
    let names = [
        "GET", "HEAD", "POST", "PUT", "DELETE", "CONNECT", "OPTIONS", "TRACE",
    ];
    for (method, name) in ALL.iter().zip(names) {
        assert_eq!(method.as_bytes(), name.as_bytes());
        assert_eq!(&Method::from_static(name).unwrap(), method);
        assert_eq!(
            &Method::new(Bytes::from_static(name.as_bytes())).unwrap(),
            method
        );
        assert!(Token::new(Bytes::from_static(name.as_bytes())).is_ok());
    }
}

#[test]
fn method_is_case_sensitive() {
    // RFC 9110 §9.1.
    assert_ne!(Method::from_static("get").unwrap(), Method::GET);
    assert_ne!(Method::from_static("Get").unwrap(), Method::GET);
}

#[test]
fn method_accepts_an_extension_token() {
    let patch = Method::from_static("PATCH").unwrap();
    assert_eq!(patch.as_bytes(), b"PATCH");
    assert_eq!(patch.as_token(), &Token::from_static("PATCH").unwrap());
}

#[test]
fn method_rejects_empty_and_non_token() {
    let token_error = |result: Result<Method, MethodError>| result.unwrap_err().token_error();
    assert_eq!(token_error(Method::new(Bytes::new())), TokenError::Empty);
    assert_eq!(token_error(Method::from_static("")), TokenError::Empty);
    assert!(matches!(
        token_error(Method::from_static("GE T")),
        TokenError::InvalidCharacter { source } if source.index() == 2 && source.byte() == b' '
    ));
    assert!(matches!(
        token_error(Method::new(Bytes::from_static(b"GET\r"))),
        TokenError::InvalidCharacter { .. }
    ));
}

#[test]
fn method_safe_set_is_get_head_options_trace() {
    // RFC 9110 §9.2.1.
    let safe: alloc::vec::Vec<bool> = ALL.iter().map(Method::is_safe).collect();
    assert_eq!(
        safe,
        alloc::vec![true, true, false, false, false, false, true, true]
    );
}

#[test]
fn method_idempotent_set_adds_put_and_delete() {
    // RFC 9110 §9.2.2.
    let idempotent: alloc::vec::Vec<bool> = ALL.iter().map(Method::is_idempotent).collect();
    assert_eq!(
        idempotent,
        alloc::vec![true, true, false, true, true, false, true, true]
    );
}

#[test]
fn method_unknown_is_neither_safe_nor_idempotent() {
    let patch = Method::from_static("PATCH").unwrap();
    assert!(!patch.is_safe());
    assert!(!patch.is_idempotent());
    let lower = Method::from_static("get").unwrap();
    assert!(!lower.is_safe());
    assert!(!lower.is_idempotent());
}

#[test]
fn method_display_and_debug() {
    assert_eq!(format!("{}", Method::GET), "GET");
    assert_eq!(format!("{:?}", Method::CONNECT), "Method(CONNECT)");
}

#[test]
fn method_error_says_what_is_not_a_token() {
    let error = Method::from_static("").unwrap_err();
    assert_eq!(format!("{error}"), "the method is not a token");
    assert_eq!(format!("{}", error.token_error()), "a token is empty");
}
