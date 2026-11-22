use super::*;
use alloc::vec;

fn encoded(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    encode(input, &mut out);
    out
}

#[test]
fn encodes_the_vectors_of_rfc_4648() {
    for (plain, text) in [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("fooba", "Zm9vYmE="),
        ("foobar", "Zm9vYmFy"),
    ] {
        assert_eq!(encoded(plain.as_bytes()), text.as_bytes(), "{plain}");
    }
}

#[test]
fn encodes_with_the_standard_alphabet() {
    assert_eq!(encoded(&[0xfb, 0xff, 0xbf]), b"+/+/");
    assert_eq!(encoded(&[0x00]), b"AA==");
    assert_eq!(encoded(&[0xff]), b"/w==");
    assert_eq!(encoded(&[0x00, 0x10, 0x83]), b"ABCD");
}

#[test]
fn encode_appends_to_the_output() {
    let mut out = b"x".to_vec();
    encode(b"f", &mut out);
    assert_eq!(out, b"xZg==");
}

#[test]
fn decodes_the_vectors_of_rfc_4648() {
    for (plain, text) in [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("fooba", "Zm9vYmE="),
        ("foobar", "Zm9vYmFy"),
    ] {
        assert_eq!(decode(text.as_bytes()).unwrap(), plain.as_bytes(), "{text}");
    }
}

#[test]
fn decodes_the_standard_alphabet() {
    assert_eq!(decode(b"+/+/").unwrap(), vec![0xfb, 0xff, 0xbf]);
    assert_eq!(decode(b"/w==").unwrap(), vec![0xff]);
}

#[test]
fn missing_padding_is_accepted() {
    assert_eq!(decode(b"Zg").unwrap(), b"f");
    assert_eq!(decode(b"Zm8").unwrap(), b"fo");
    assert_eq!(decode(b"Zm9vYg").unwrap(), b"foob");
    assert_eq!(decode(b"Zm9vYmE").unwrap(), b"fooba");
    // Partial padding too.
    assert_eq!(decode(b"Zg=").unwrap(), b"f");
}

#[test]
fn non_zero_pad_bits_are_accepted() {
    // `Zh` is `Zg` with a one in the unused bits.
    assert_eq!(decode(b"Zh==").unwrap(), b"f");
    assert_eq!(decode(b"Zh").unwrap(), b"f");
    assert_eq!(decode(b"Zm9=").unwrap(), b"fo");
    assert_eq!(decode(b"/").unwrap_err(), DecodeError::InvalidLength);
    assert_eq!(decode(b"//").unwrap(), vec![0xff]);
    assert_eq!(decode(b"///").unwrap(), vec![0xff, 0xff]);
}

#[test]
fn characters_outside_the_alphabet_fail() {
    assert_eq!(
        decode(b"Zm9v*").unwrap_err(),
        DecodeError::InvalidCharacter(4)
    );
    assert_eq!(decode(b"-_").unwrap_err(), DecodeError::InvalidCharacter(0));
    assert_eq!(
        decode(b"Zm 9v").unwrap_err(),
        DecodeError::InvalidCharacter(2)
    );
    assert_eq!(
        decode(b"Zm9v\n").unwrap_err(),
        DecodeError::InvalidCharacter(4)
    );
    assert_eq!(
        decode(b"Zm\xc3").unwrap_err(),
        DecodeError::InvalidCharacter(2)
    );
    // A bad character after padding is reported where it is.
    assert_eq!(
        decode(b"Zg=*").unwrap_err(),
        DecodeError::InvalidCharacter(3)
    );
}

#[test]
fn padding_in_the_wrong_place_fails() {
    assert_eq!(
        decode(b"Zg=a").unwrap_err(),
        DecodeError::MisplacedPadding(2)
    );
    assert_eq!(
        decode(b"Zg===").unwrap_err(),
        DecodeError::MisplacedPadding(4)
    );
    assert_eq!(
        decode(b"Zm9v=").unwrap_err(),
        DecodeError::MisplacedPadding(4)
    );
    assert_eq!(
        decode(b"Zm8==").unwrap_err(),
        DecodeError::MisplacedPadding(4)
    );
    assert_eq!(
        decode(b"====").unwrap_err(),
        DecodeError::MisplacedPadding(0)
    );
    assert_eq!(
        decode(b"=Zg").unwrap_err(),
        DecodeError::MisplacedPadding(0)
    );
    assert_eq!(
        decode(b"Zg==Zg==").unwrap_err(),
        DecodeError::MisplacedPadding(2)
    );
}

#[test]
fn one_digit_too_many_fails() {
    for text in ["Z", "Zm9vY", "Zm9vYmFyY", "Z=", "Z==", "Z==="] {
        assert_eq!(
            decode(text.as_bytes()).unwrap_err(),
            DecodeError::InvalidLength,
            "{text}"
        );
    }
}

#[test]
fn round_trips_every_length() {
    for length in 0..=40usize {
        let input: Vec<u8> = (0..length).map(|n| (n * 37 + 11) as u8).collect();
        let text = encoded(&input);
        assert_eq!(text.len() % 4, 0);
        assert_eq!(decode(&text).unwrap(), input, "{length}");
    }
}

#[test]
fn round_trips_every_byte_value() {
    let input: Vec<u8> = (0..=255).collect();
    assert_eq!(decode(&encoded(&input)).unwrap(), input);
}
