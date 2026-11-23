use super::*;
use alloc::format;

fn new(text: &str) -> Result<Authority, AuthorityError> {
    Authority::new(text.as_bytes())
}

fn ok(text: &str) -> Authority {
    new(text).unwrap_or_else(|e| panic!("{text:?}: {e:?}"))
}

fn err(text: &str) -> AuthorityError {
    new(text).unwrap_err()
}

/// The index of the `%` that `AuthorityError::InvalidPercentEncoding` names.
fn pct_index(text: &str) -> usize {
    match err(text) {
        AuthorityError::InvalidPercentEncoding { source } => source.index(),
        other => panic!("{text:?}: expected InvalidPercentEncoding, got {other:?}"),
    }
}

/// The index that a failure with an index payload names.
fn payload_index(text: &str) -> usize {
    match err(text) {
        AuthorityError::Userinfo { source } => source.index(),
        AuthorityError::AfterIpLiteral { source } => source.index(),
        AuthorityError::PortOutOfRange { source } => source.index(),
        other => panic!("{text:?}: expected an index payload, got {other:?}"),
    }
}

#[test]
fn authority_reads_a_registered_name() {
    // RFC 3986 §3.2.2.
    let a = ok("example.com");
    assert_eq!(a.host(), Host::RegName(b"example.com"));
    assert_eq!(a.host_bytes(), b"example.com");
    assert_eq!(a.port(), None);
}

#[test]
fn authority_reads_a_name_and_port() {
    let a = ok("example.com:8080");
    assert_eq!(a.host(), Host::RegName(b"example.com"));
    assert_eq!(a.port(), Some(8080));
}

#[test]
fn authority_lowercases_the_host() {
    // RFC 3986 §3.2.2: hosts are case-insensitive.
    let a = ok("EXAMPLE.Com:80");
    assert_eq!(a.host_bytes(), b"example.com");
    assert_eq!(a, ok("example.com:80"));
}

#[test]
fn authority_accepts_a_single_byte_host() {
    assert_eq!(ok("a").host(), Host::RegName(b"a"));
}

#[test]
fn authority_reads_an_ipv4_address() {
    // The address of an example in RFC 3986 §1.1.2.
    let a = ok("192.0.2.16");
    assert_eq!(a.host(), Host::Ipv4(Ipv4Addr::new(192, 0, 2, 16)));
    assert_eq!(a.host_bytes(), b"192.0.2.16");
    let a = ok("192.0.2.16:8080");
    assert_eq!(a.host(), Host::Ipv4(Ipv4Addr::new(192, 0, 2, 16)));
    assert_eq!(a.port(), Some(8080));
}

#[test]
fn authority_reads_the_ipv4_edges() {
    assert_eq!(ok("0.0.0.0").host(), Host::Ipv4(Ipv4Addr::UNSPECIFIED));
    assert_eq!(
        ok("255.255.255.255").host(),
        Host::Ipv4(Ipv4Addr::BROADCAST)
    );
}

#[test]
fn authority_treats_numbers_that_are_not_ipv4_as_reg_names() {
    // RFC 3986 §3.2.2: text that does not match IPv4address is a reg-name.
    for text in [
        "1.2.3",
        "256.1.1.1",
        "1.2.3.4.5",
        "1.2.3.256",
        "01.2.3.4",
        "1..2.3",
        "123",
        "1.2.3.",
    ] {
        let a = ok(text);
        assert_eq!(a.host(), Host::RegName(text.as_bytes()), "{text}");
    }
}

#[test]
fn authority_reads_an_ipv6_literal() {
    // The literal of an example in RFC 3986 §1.1.2.
    let a = ok("[2001:db8::7]");
    assert_eq!(
        a.host(),
        Host::Ipv6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 7))
    );
    assert_eq!(a.host_bytes(), b"[2001:db8::7]");
    assert_eq!(a.port(), None);
}

#[test]
fn authority_reads_an_ipv6_literal_with_a_port() {
    let a = ok("[2001:db8::7]:443");
    assert_eq!(a.port(), Some(443));
    assert_eq!(a.host_bytes(), b"[2001:db8::7]");
}

#[test]
fn authority_lowercases_an_ipv6_literal() {
    assert_eq!(ok("[2001:DB8::A]").host_bytes(), b"[2001:db8::a]");
}

#[test]
fn authority_reads_loopback_and_embedded_ipv4() {
    assert_eq!(ok("[::1]").host(), Host::Ipv6(Ipv6Addr::LOCALHOST));
    assert_eq!(
        ok("[::ffff:192.0.2.128]").host(),
        Host::Ipv6(Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc000, 0x280))
    );
}

#[test]
fn authority_reads_an_ip_future_literal() {
    // RFC 3986 §3.2.2.
    let a = ok("[v7.fe80::1]");
    assert_eq!(a.host(), Host::IpFuture(b"v7.fe80::1"));
    assert_eq!(a.host_bytes(), b"[v7.fe80::1]");
    let a = ok("[V7.FE80::1]:80");
    assert_eq!(a.host(), Host::IpFuture(b"v7.fe80::1"));
    assert_eq!(a.port(), Some(80));
}

#[test]
fn authority_accepts_the_smallest_ip_future() {
    assert_eq!(ok("[vF.a]").host(), Host::IpFuture(b"vf.a"));
    assert!(new("[v1.~!$&'()*+,;=:-._]").is_ok());
}

#[test]
fn authority_accepts_percent_encoding_and_sub_delims_in_a_name() {
    let a = ok("a%41b!$&'()*+,;=-._~");
    assert_eq!(a.host(), Host::RegName(b"a%41b!$&'()*+,;=-._~"));
}

#[test]
fn authority_port_edges() {
    assert_eq!(ok("h:0").port(), Some(0));
    assert_eq!(ok("h:1").port(), Some(1));
    assert_eq!(ok("h:65535").port(), Some(u16::MAX));
    assert_eq!(ok("h:0000065535").port(), Some(u16::MAX));
    assert_eq!(ok("[::1]:65535").port(), Some(u16::MAX));
}

#[test]
fn authority_empty_port_is_no_port() {
    // RFC 3986 §3.2.3.
    assert_eq!(ok("h:").port(), None);
    assert_eq!(ok("h:"), ok("h"));
    assert_eq!(ok("[::1]:").port(), None);
    assert_eq!(ok("h:").to_bytes(), Bytes::from_static(b"h"));
}

#[test]
fn authority_rejects_userinfo() {
    // RFC 9110 §4.2.4.
    assert_eq!(payload_index("user@example.com"), 4);
    assert_eq!(payload_index("@"), 0);
    assert_eq!(payload_index("a:b@host"), 3);
    assert_eq!(payload_index("[::1]@h"), 5);
    assert_eq!(payload_index("h:80@"), 4);
    assert!(matches!(err("@"), AuthorityError::Userinfo { .. }));
}

#[test]
fn authority_rejects_an_empty_host() {
    // RFC 9110 §4.2.1.
    assert_eq!(err(""), AuthorityError::EmptyHost);
    assert_eq!(err(":80"), AuthorityError::EmptyHost);
    assert_eq!(err(":"), AuthorityError::EmptyHost);
    assert_eq!(err("::1"), AuthorityError::EmptyHost);
}

#[test]
fn authority_rejects_an_unclosed_ip_literal() {
    assert_eq!(err("["), AuthorityError::UnclosedIpLiteral);
    assert_eq!(err("[::1"), AuthorityError::UnclosedIpLiteral);
    assert_eq!(err("[::1:80"), AuthorityError::UnclosedIpLiteral);
    assert_eq!(err("[v7.a"), AuthorityError::UnclosedIpLiteral);
}

#[test]
fn authority_rejects_a_bad_ipv6_address() {
    for text in [
        "[]",
        "[1:2]",
        "[::g]",
        "[1.2.3.4]",
        "[fe80::1%25eth0]",
        "[::1::2]",
        "[ ::1]",
        "[:::]",
    ] {
        assert_eq!(err(text), AuthorityError::InvalidIpv6, "{text}");
    }
    assert_eq!(
        Authority::new(b"[\xff]").unwrap_err(),
        AuthorityError::InvalidIpv6
    );
}

#[test]
fn authority_rejects_a_bad_ip_future() {
    // "v" 1*HEXDIG "." 1*( unreserved / sub-delims / ":" ), with the index of `]`.
    for (text, close) in [
        ("[v]", 2),
        ("[v.a]", 4),
        ("[v7]", 3),
        ("[v7.]", 4),
        ("[vg.a]", 5),
        ("[v7.a b]", 7),
        ("[v7.a/b]", 7),
    ] {
        let expected = AuthorityError::InvalidIpFuture {
            source: InvalidIpFutureError { close },
        };
        assert_eq!(err(text), expected, "{text}");
    }
    let AuthorityError::InvalidIpFuture { source } = err("[v7.]") else {
        panic!();
    };
    assert_eq!(source.close(), 4);
}

#[test]
fn authority_rejects_something_other_than_a_port_after_an_ip_literal() {
    for text in ["[::1]x", "[::1]/", "[::1] ", "[::1]80", "[v7.a]:80:90x"] {
        let error = err(text);
        assert!(
            matches!(
                error,
                AuthorityError::AfterIpLiteral { .. } | AuthorityError::InvalidPortCharacter { .. }
            ),
            "{text}: {error:?}"
        );
    }
    assert_eq!(payload_index("[::1]x"), 5);
    assert_eq!(payload_index("[::1]80"), 5);
    assert_eq!(payload_index("[::1]]"), 5);
    assert!(matches!(
        err("[::1]x"),
        AuthorityError::AfterIpLiteral { .. }
    ));
}

#[test]
fn authority_rejects_a_byte_a_reg_name_cannot_hold() {
    for (text, index, byte) in [
        ("a b", 1, b' '),
        ("a/b", 1, b'/'),
        ("a?b", 1, b'?'),
        ("a#b", 1, b'#'),
        ("a[b", 1, b'['),
        ("a]b", 1, b']'),
        ("a\"", 1, b'"'),
        ("a\\", 1, b'\\'),
        ("a<", 1, b'<'),
        ("a\t", 1, b'\t'),
        ("a\x7f", 1, 0x7f),
    ] {
        let AuthorityError::InvalidRegNameCharacter { source } = err(text) else {
            panic!("{text:?}");
        };
        assert_eq!((source.index(), source.byte()), (index, byte), "{text:?}");
    }
    let AuthorityError::InvalidRegNameCharacter { source } = Authority::new(b"a\xe9").unwrap_err()
    else {
        panic!();
    };
    assert_eq!((source.index(), source.byte()), (1, 0xe9));
}

#[test]
fn authority_reg_name_character_index_counts_from_the_authority_start() {
    let AuthorityError::InvalidRegNameCharacter { source } = err("abc d:80") else {
        panic!();
    };
    assert_eq!(source.index(), 3);
}

#[test]
fn authority_rejects_a_bad_percent_encoding_in_a_name() {
    assert_eq!(pct_index("a%"), 1);
    assert_eq!(pct_index("a%2"), 1);
    assert_eq!(pct_index("%zz"), 0);
    assert_eq!(pct_index("ab%2g.c"), 2);
}

#[test]
fn authority_rejects_a_port_with_a_non_digit() {
    for (text, index, byte) in [
        ("h:80a", 4, b'a'),
        ("h:a", 2, b'a'),
        ("h:-1", 2, b'-'),
        ("h:+1", 2, b'+'),
        ("h: 1", 2, b' '),
        ("h:8 0", 3, b' '),
        ("a:b:c", 2, b'b'),
        ("h:80:90", 4, b':'),
        ("[::1]:8x", 7, b'x'),
        ("[::1]:x", 6, b'x'),
    ] {
        let AuthorityError::InvalidPortCharacter { source } = err(text) else {
            panic!("{text:?}");
        };
        assert_eq!((source.index(), source.byte()), (index, byte), "{text:?}");
    }
}

#[test]
fn authority_rejects_a_port_above_65535() {
    assert_eq!(payload_index("h:65536"), 2);
    assert_eq!(payload_index("h:99999"), 2);
    assert_eq!(payload_index("h:4294967296"), 2);
    assert_eq!(payload_index("h:99999999999999999999999"), 2);
    assert_eq!(payload_index("[::1]:65536"), 6);
}

#[test]
fn authority_checks_the_host_before_the_port() {
    assert_eq!(
        err("a b:99999"),
        AuthorityError::InvalidRegNameCharacter {
            source: InvalidRegNameCharacterError {
                index: 1,
                byte: b' '
            }
        }
    );
}

#[test]
fn authority_to_bytes_writes_the_port_in_decimal() {
    // Zero, one digit, u16::MAX.
    assert_eq!(ok("h:0").to_bytes(), Bytes::from_static(b"h:0"));
    assert_eq!(ok("h:7").to_bytes(), Bytes::from_static(b"h:7"));
    assert_eq!(ok("h:10").to_bytes(), Bytes::from_static(b"h:10"));
    assert_eq!(ok("h:0080").to_bytes(), Bytes::from_static(b"h:80"));
    assert_eq!(ok("h:65535").to_bytes(), Bytes::from_static(b"h:65535"));
}

#[test]
fn authority_to_bytes_keeps_brackets_and_lowercases() {
    assert_eq!(ok("[::1]").to_bytes(), Bytes::from_static(b"[::1]"));
    assert_eq!(ok("[V7.A]:9").to_bytes(), Bytes::from_static(b"[v7.a]:9"));
    assert_eq!(ok("Ex.COM").to_bytes(), Bytes::from_static(b"ex.com"));
}

#[test]
fn authority_to_bytes_reads_back_the_same() {
    for text in [
        "a",
        "a:1",
        "192.0.2.16:80",
        "[2001:db8::7]:8080",
        "[v7.fe80::1]",
        "a%41",
    ] {
        let a = ok(text);
        assert_eq!(Authority::new(&a.to_bytes()), Ok(a));
    }
}

#[test]
fn authority_equality_ignores_case_and_empty_port() {
    assert_eq!(ok("EXAMPLE.com:"), ok("example.com"));
    assert_ne!(ok("example.com:80"), ok("example.com"));
    assert_ne!(ok("example.com:80"), ok("example.com:81"));
}

#[test]
fn authority_display_and_debug() {
    assert_eq!(format!("{}", ok("Example.com")), "example.com");
    assert_eq!(format!("{}", ok("example.com:8080")), "example.com:8080");
    assert_eq!(format!("{}", ok("[::1]:1")), "[::1]:1");
    assert_eq!(format!("{:?}", ok("h:1")), "Authority(h:1)");
}

#[test]
fn authority_errors_display() {
    assert_eq!(format!("{}", err("a@b")), "an authority holds userinfo");
    let AuthorityError::Userinfo { source } = err("a@b") else {
        panic!("expected Userinfo");
    };
    assert_eq!(
        format!("{source}"),
        "userinfo at 1: an authority holds an `@`"
    );
    let AuthorityError::InvalidPercentEncoding { source } = err("abc%") else {
        panic!("expected InvalidPercentEncoding");
    };
    assert_eq!(
        format!("{source}"),
        "a percent sign at 3 is not followed by two hexadecimal digits"
    );
    let AuthorityError::PortOutOfRange { source } = err("h:70000") else {
        panic!("expected PortOutOfRange");
    };
    assert_eq!(format!("{source}"), "the port at 2 is above 65535");
    let AuthorityError::InvalidPortCharacter { source } = err("h:x") else {
        panic!();
    };
    assert_eq!(
        format!("{source}"),
        "byte 0x78 at 2 in a port is not a digit"
    );
    let AuthorityError::InvalidRegNameCharacter { source } = err("a b") else {
        panic!();
    };
    assert_eq!(
        format!("{source}"),
        "byte 0x20 at 1 cannot be in a registered name"
    );
    let AuthorityError::InvalidIpFuture { source } = err("[v]") else {
        panic!();
    };
    assert_eq!(
        format!("{source}"),
        "the IPvFuture literal ending at 2 is not valid"
    );
}

#[test]
fn eq_normalized_takes_the_default_port_of_the_scheme() {
    // RFC 3986 §6.2.3; RFC 9113 §8.3.1.
    assert!(ok("example.com").eq_normalized(&ok("example.com:443"), &Scheme::HTTPS));
    assert!(ok("example.com:").eq_normalized(&ok("example.com:443"), &Scheme::HTTPS));
    assert!(ok("example.com:80").eq_normalized(&ok("example.com"), &Scheme::HTTP));
    assert!(!ok("example.com").eq_normalized(&ok("example.com:80"), &Scheme::HTTPS));
    assert!(!ok("example.com:443").eq_normalized(&ok("example.com:8443"), &Scheme::HTTPS));
    // No default port is known for other schemes.
    let other = Scheme::from_static("coap").unwrap();
    assert!(!ok("example.com").eq_normalized(&ok("example.com:5683"), &other));
    assert!(ok("example.com:5683").eq_normalized(&ok("EXAMPLE.com:5683"), &other));
}

#[test]
fn eq_normalized_decodes_unreserved_characters() {
    // RFC 3986 §6.2.2.2: %61 is `a`, %2D is `-`; %2F is not unreserved.
    assert!(ok("ex%61mple.com").eq_normalized(&ok("example.com"), &Scheme::HTTPS));
    assert!(ok("a%2Db").eq_normalized(&ok("a-b"), &Scheme::HTTPS));
    assert!(ok("%41").eq_normalized(&ok("a"), &Scheme::HTTPS));
    assert!(ok("a%2fb").eq_normalized(&ok("a%2Fb"), &Scheme::HTTPS));
    assert!(!ok("a%2fb").eq_normalized(&ok("axb"), &Scheme::HTTPS));
    assert_ne!(ok("ex%61mple.com"), ok("example.com"));
}

#[test]
fn eq_normalized_compares_ip_addresses() {
    assert!(ok("[::1]").eq_normalized(&ok("[0:0::1]:443"), &Scheme::HTTPS));
    assert!(ok("127.0.0.1").eq_normalized(&ok("127.0.0.1:80"), &Scheme::HTTP));
    assert!(!ok("[::1]").eq_normalized(&ok("[::2]"), &Scheme::HTTPS));
    assert!(!ok("127.0.0.1").eq_normalized(&ok("localhost"), &Scheme::HTTP));
}

#[test]
fn scheme_default_ports() {
    assert_eq!(Scheme::HTTP.default_port(), Some(80));
    assert_eq!(Scheme::HTTPS.default_port(), Some(443));
    assert_eq!(Scheme::from_static("ws").unwrap().default_port(), None);
}
