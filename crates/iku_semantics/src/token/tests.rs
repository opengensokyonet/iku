use super::*;
use alloc::format;

fn bytes(text: &'static str) -> Bytes {
    Bytes::from_static(text.as_bytes())
}

#[test]
fn token_accepts_a_single_tchar() {
    assert_eq!(Token::new(bytes("a")).unwrap().as_bytes(), b"a");
}

#[test]
fn token_accepts_every_punctuation_tchar() {
    let token = Token::new(bytes("!#$%&'*+-.^_`|~")).unwrap();
    assert_eq!(token.as_bytes(), b"!#$%&'*+-.^_`|~");
}

#[test]
fn token_keeps_case() {
    assert_eq!(Token::new(bytes("GeT")).unwrap().as_bytes(), b"GeT");
}

#[test]
fn token_rejects_empty_input() {
    assert_eq!(Token::new(Bytes::new()), Err(TokenError::Empty));
}

#[test]
fn token_rejects_a_space_and_reports_where() {
    let error = Token::new(bytes("ab cd")).unwrap_err();
    let TokenError::InvalidCharacter { source } = error else {
        panic!("expected InvalidCharacter, got {error:?}");
    };
    assert_eq!(source.index(), 2);
    assert_eq!(source.byte(), b' ');
}

#[test]
fn token_rejects_a_first_byte_that_is_not_a_tchar() {
    let error = Token::new(bytes("(a")).unwrap_err();
    assert!(matches!(
        error,
        TokenError::InvalidCharacter { source } if source.index() == 0 && source.byte() == b'('
    ));
}

#[test]
fn token_rejects_a_last_byte_that_is_not_a_tchar() {
    let error = Token::new(bytes("a,")).unwrap_err();
    assert!(matches!(
        error,
        TokenError::InvalidCharacter { source } if source.index() == 1 && source.byte() == b','
    ));
}

#[test]
fn token_rejects_htab_del_and_obs_text() {
    for byte in [b'\t', 0x00, 0x1f, 0x7f, 0x80, 0xff] {
        let error = Token::new(Bytes::from(alloc::vec![b'a', byte])).unwrap_err();
        assert!(
            matches!(
                error,
                TokenError::InvalidCharacter { source } if source.index() == 1 && source.byte() == byte
            ),
            "byte {byte:#04x}: {error:?}"
        );
    }
}

#[test]
fn token_rejects_every_delimiter_of_rfc_9110_section_5_6_2() {
    for byte in b"\"(),/:;<=>?@[\\]{}" {
        let error = Token::new(Bytes::from(alloc::vec![*byte])).unwrap_err();
        assert!(
            matches!(error, TokenError::InvalidCharacter { .. }),
            "{byte}"
        );
    }
}

#[test]
fn token_from_static_checks_like_new() {
    assert_eq!(Token::from_static("close").unwrap().as_bytes(), b"close");
    assert_eq!(Token::from_static(""), Err(TokenError::Empty));
    assert!(matches!(
        Token::from_static("a b"),
        Err(TokenError::InvalidCharacter { .. })
    ));
}

#[test]
fn token_into_bytes_gives_the_same_bytes() {
    let token = Token::from_static("keep-alive").unwrap();
    assert_eq!(token.into_bytes(), Bytes::from_static(b"keep-alive"));
}

#[test]
fn token_compares_ignoring_ascii_case_on_request() {
    // RFC 9110 §7.6.1: connection options are case-insensitive.
    let token = Token::from_static("Close").unwrap();
    assert!(token.eq_ignore_ascii_case(b"close"));
    assert!(token.eq_ignore_ascii_case(b"CLOSE"));
    assert!(!token.eq_ignore_ascii_case(b"closed"));
    assert!(!token.eq_ignore_ascii_case(b""));
    assert_ne!(token, Token::from_static("close").unwrap());
}

#[test]
fn token_display_and_debug() {
    let token = Token::from_static("GET").unwrap();
    assert_eq!(format!("{token}"), "GET");
    assert_eq!(format!("{token:?}"), "Token(GET)");
}

#[test]
fn token_error_display_mentions_the_byte() {
    let error = Token::new(bytes("a b")).unwrap_err();
    assert_eq!(
        format!("{error}"),
        "a token holds a byte that is not a tchar"
    );
    let TokenError::InvalidCharacter { source } = error else {
        panic!();
    };
    assert_eq!(format!("{source}"), "byte 0x20 at 1 is not a tchar");
    assert_eq!(format!("{}", TokenError::Empty), "a token is empty");
}

#[test]
fn check_and_check_characters_agree_with_new() {
    assert_eq!(check(b""), Err(TokenError::Empty));
    assert!(check(b"x").is_ok());
    assert!(check_characters(b"x-y").is_ok());
    let error = check_characters(b"x y").unwrap_err();
    assert_eq!((error.index(), error.byte()), (1, b' '));
}
