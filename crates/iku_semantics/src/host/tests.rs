use super::*;
use crate::uri::Host;
use alloc::format;

fn section(lines: &[&'static str]) -> FieldSection {
    lines
        .iter()
        .map(|value| {
            (
                FieldName::HOST,
                FieldValue::new(Bytes::from_static(value.as_bytes())).unwrap(),
            )
        })
        .collect()
}

fn read(lines: &[&'static str]) -> Result<Option<HostField>, HostFieldError> {
    HostField::from_fields(&section(lines))
}

#[test]
fn host_field_is_none_without_a_line() {
    assert_eq!(HostField::from_fields(&FieldSection::new()), Ok(None));
    let mut fields = FieldSection::new();
    fields.append(FieldName::TE, FieldValue::from_static("x").unwrap());
    assert_eq!(HostField::from_fields(&fields), Ok(None));
}

#[test]
fn host_field_reads_an_empty_value() {
    // RFC 9112 §3.2: a client sends an empty Host for a URI without authority.
    let host = read(&[""]).unwrap().unwrap();
    assert_eq!(host, HostField::Empty);
    assert_eq!(host.authority(), None);
    assert_eq!(host.to_field_value().as_bytes(), b"");
}

#[test]
fn host_field_reads_a_host_name() {
    // RFC 9112 §3.2 example, `Host: www.example.org`.
    let host = read(&["www.example.org"]).unwrap().unwrap();
    let authority = host.authority().unwrap();
    assert_eq!(authority.host(), Host::RegName(b"www.example.org"));
    assert_eq!(authority.port(), None);
    assert_eq!(host.to_field_value().as_bytes(), b"www.example.org");
}

#[test]
fn host_field_reads_a_host_and_port() {
    let host = read(&["example.com:8080"]).unwrap().unwrap();
    let authority = host.authority().unwrap();
    assert_eq!(authority.host_bytes(), b"example.com");
    assert_eq!(authority.port(), Some(8080));
    assert_eq!(host.to_field_value().as_bytes(), b"example.com:8080");
}

#[test]
fn host_field_lowercases_the_host_on_the_way_back() {
    let host = read(&["Example.COM"]).unwrap().unwrap();
    assert_eq!(host.to_field_value().as_bytes(), b"example.com");
}

#[test]
fn host_field_reads_an_ip_literal() {
    let host = read(&["[2001:db8::7]:80"]).unwrap().unwrap();
    assert_eq!(host.to_field_value().as_bytes(), b"[2001:db8::7]:80");
}

#[test]
fn host_field_round_trips_through_a_field_value() {
    for text in [
        "example.com",
        "example.com:8080",
        "192.0.2.16:1",
        "[::1]",
        "",
    ] {
        let value = FieldValue::new(Bytes::copy_from_slice(text.as_bytes())).unwrap();
        let mut fields = FieldSection::new();
        fields.append(FieldName::HOST, value);
        let host = HostField::from_fields(&fields).unwrap().unwrap();
        assert_eq!(host.to_field_value().as_bytes(), text.as_bytes());
    }
}

#[test]
fn host_field_rejects_two_lines() {
    assert_eq!(read(&["a", "b"]), Err(HostFieldError::Repeated));
    // Even two equal lines, and even when empty.
    assert_eq!(read(&["a", "a"]), Err(HostFieldError::Repeated));
    assert_eq!(read(&["", ""]), Err(HostFieldError::Repeated));
}

#[test]
fn host_field_rejects_userinfo() {
    let error = read(&["user@example.com"]).unwrap_err();
    assert!(matches!(
        error,
        HostFieldError::InvalidAuthority {
            source: AuthorityError::Userinfo
        }
    ));
}

#[test]
fn host_field_rejects_a_bad_port() {
    assert!(matches!(
        read(&["example.com:65536"]),
        Err(HostFieldError::InvalidAuthority {
            source: AuthorityError::PortOutOfRange
        })
    ));
}

#[test]
fn host_field_rejects_a_value_with_inner_space() {
    assert!(matches!(
        read(&["a b"]),
        Err(HostFieldError::InvalidAuthority {
            source: AuthorityError::InvalidRegNameCharacter { .. }
        })
    ));
}

#[test]
fn host_field_errors_display() {
    assert_eq!(
        format!("{}", HostFieldError::Repeated),
        "there are several host lines"
    );
    let error = read(&["a@b"]).unwrap_err();
    assert_eq!(format!("{error}"), "the host value is not an authority");
}
