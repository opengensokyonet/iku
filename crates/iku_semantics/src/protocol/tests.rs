use super::*;
use alloc::format;

fn protocol(text: &'static str) -> Result<Protocol, ProtocolError> {
    Protocol::from_static(text)
}

#[test]
fn protocol_without_a_version() {
    // RFC 8441 §4.
    let p = protocol("websocket").unwrap();
    assert_eq!(p.name(), b"websocket");
    assert_eq!(p.version(), None);
    assert_eq!(p.as_bytes(), b"websocket");
}

#[test]
fn protocol_with_a_version() {
    let p = protocol("HTTP/2.0").unwrap();
    assert_eq!(p.name(), b"http");
    assert_eq!(p.version(), Some(&b"2.0"[..]));
    assert_eq!(p.as_bytes(), b"http/2.0");
}

#[test]
fn protocol_new_takes_bytes() {
    let p = Protocol::new(Bytes::from_static(b"webtransport")).unwrap();
    assert_eq!(p.name(), b"webtransport");
}

#[test]
fn protocol_rejects_empty_input_as_a_name_error() {
    assert_eq!(
        protocol(""),
        Err(ProtocolError::Name {
            source: TokenError::Empty
        })
    );
}

#[test]
fn protocol_rejects_an_empty_name_before_a_slash() {
    assert_eq!(
        protocol("/1"),
        Err(ProtocolError::Name {
            source: TokenError::Empty
        })
    );
}

#[test]
fn protocol_rejects_an_empty_version_after_a_slash() {
    assert_eq!(
        protocol("a/"),
        Err(ProtocolError::Version {
            source: TokenError::Empty
        })
    );
}

#[test]
fn protocol_rejects_a_name_that_is_not_a_token() {
    let error = protocol("web socket").unwrap_err();
    assert!(matches!(
        error,
        ProtocolError::Name { source: TokenError::InvalidCharacter { source } }
            if source.index() == 3 && source.byte() == b' '
    ));
}

#[test]
fn protocol_rejects_a_version_that_is_not_a_token() {
    let error = protocol("a/b c").unwrap_err();
    // The index is counted from the start of the version.
    assert!(matches!(
        error,
        ProtocolError::Version { source: TokenError::InvalidCharacter { source } }
            if source.index() == 1 && source.byte() == b' '
    ));
}

#[test]
fn protocol_rejects_a_second_slash_in_the_version() {
    // `/` is not a tchar, so a version cannot hold one.
    assert!(matches!(
        protocol("a/b/c"),
        Err(ProtocolError::Version { source: TokenError::InvalidCharacter { source } })
            if source.byte() == b'/'
    ));
}

#[test]
fn protocol_display_and_debug() {
    let p = protocol("a/1").unwrap();
    assert_eq!(format!("{p}"), "a/1");
    assert_eq!(format!("{p:?}"), "Protocol(a/1)");
}

#[test]
fn protocol_name_is_lowercased_and_the_version_kept() {
    assert_eq!(
        protocol("WebSocket").unwrap(),
        protocol("websocket").unwrap()
    );
    let mixed = protocol("WebTransport/Draft02").unwrap();
    assert_eq!(mixed.as_bytes(), b"webtransport/Draft02");
    assert_eq!(mixed.name(), b"webtransport");
    assert_eq!(mixed.version(), Some(&b"Draft02"[..]));
    assert_eq!(format!("{mixed}"), "webtransport/Draft02");
    assert_ne!(mixed, protocol("webtransport/draft02").unwrap());
}

#[test]
fn protocol_errors_display() {
    assert_eq!(
        format!("{}", protocol("").unwrap_err()),
        "the protocol name is not a token"
    );
    assert_eq!(
        format!("{}", protocol("a/").unwrap_err()),
        "the protocol version is not a token"
    );
}
