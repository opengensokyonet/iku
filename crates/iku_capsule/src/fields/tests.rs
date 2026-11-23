use super::*;
use alloc::string::ToString;
use iku_semantics::head::ResponseHead;
use iku_semantics::status::StatusCode;

fn name(text: &'static str) -> FieldName {
    FieldName::from_static(text).unwrap()
}

fn value(text: &'static str) -> FieldValue {
    FieldValue::from_static(text).unwrap()
}

fn section(lines: &[(&'static str, &'static str)]) -> FieldSection {
    let mut fields = FieldSection::new();
    for (n, v) in lines {
        fields.append(name(n), value(v));
    }
    fields
}

fn protocol(lines: &[&'static str]) -> bool {
    let lines: alloc::vec::Vec<_> = lines.iter().map(|v| ("capsule-protocol", *v)).collect();
    uses_capsule_protocol(&section(&lines))
}

#[test]
fn field_name_is_capsule_protocol() {
    assert_eq!(capsule_protocol_field_name(), name("capsule-protocol"));
    assert_eq!(
        capsule_protocol_field_name().as_bytes(),
        b"capsule-protocol"
    );
}

#[test]
fn field_value_is_true_boolean() {
    assert_eq!(capsule_protocol_field_value().as_bytes(), b"?1");
}

#[test]
fn true_means_in_use() {
    assert!(protocol(&["?1"]));
}

#[test]
fn own_field_value_means_in_use() {
    let mut fields = FieldSection::new();
    fields.append(
        capsule_protocol_field_name(),
        capsule_protocol_field_value(),
    );
    assert!(uses_capsule_protocol(&fields));
}

#[test]
fn parameters_are_ignored() {
    assert!(protocol(&["?1;a=1"]));
    assert!(protocol(&["?1; a; b=\"x\""]));
    assert!(!protocol(&["?0;a=1"]));
}

#[test]
fn false_is_not_in_use() {
    assert!(!protocol(&["?0"]));
}

#[test]
fn missing_field_is_not_in_use() {
    assert!(!uses_capsule_protocol(&FieldSection::new()));
    assert!(!uses_capsule_protocol(&section(&[("other", "?1")])));
}

#[test]
fn non_boolean_is_not_in_use() {
    assert!(!protocol(&["1"]));
    assert!(!protocol(&["\"x\""]));
    assert!(!protocol(&["token"]));
    assert!(!protocol(&[":aGk=:"]));
    assert!(!protocol(&["1.5"]));
}

#[test]
fn several_lines_are_not_in_use() {
    assert!(!protocol(&["?1", "?1"]));
    assert!(!protocol(&["?1", "?0"]));
}

#[test]
fn unparsable_is_not_in_use() {
    assert!(!protocol(&[""]));
    assert!(!protocol(&["?2"]));
    assert!(!protocol(&["?"]));
    assert!(!protocol(&["?1 x"]));
    assert!(!protocol(&["?1, ?1"]));
    assert!(!protocol(&["(?1)"]));
}

#[test]
fn no_forbidden_fields_is_ok() {
    assert_eq!(check_capsule_message_fields(&FieldSection::new()), Ok(()));
    assert_eq!(
        check_capsule_message_fields(&section(&[
            ("capsule-protocol", "?1"),
            ("host", "example.com"),
        ])),
        Ok(())
    );
}

#[test]
fn content_length_is_reported() {
    assert_eq!(
        check_capsule_message_fields(&section(&[("content-length", "0")])),
        Err(CapsuleMessageFieldError::ContentLength)
    );
}

#[test]
fn content_type_is_reported() {
    assert_eq!(
        check_capsule_message_fields(&section(&[("content-type", "text/plain")])),
        Err(CapsuleMessageFieldError::ContentType)
    );
}

#[test]
fn transfer_encoding_is_reported() {
    assert_eq!(
        check_capsule_message_fields(&section(&[("transfer-encoding", "chunked")])),
        Err(CapsuleMessageFieldError::TransferEncoding)
    );
}

#[test]
fn order_is_length_then_type_then_encoding() {
    let all = [
        ("transfer-encoding", "chunked"),
        ("content-type", "text/plain"),
        ("content-length", "0"),
    ];
    assert_eq!(
        check_capsule_message_fields(&section(&all)),
        Err(CapsuleMessageFieldError::ContentLength)
    );
    assert_eq!(
        check_capsule_message_fields(&section(&all[..2])),
        Err(CapsuleMessageFieldError::ContentType)
    );
    assert_eq!(
        check_capsule_message_fields(&section(&[all[0], all[2]])),
        Err(CapsuleMessageFieldError::ContentLength)
    );
}

#[test]
fn forbidden_field_is_reported_among_others() {
    assert_eq!(
        check_capsule_message_fields(&section(&[
            ("capsule-protocol", "?1"),
            ("content-type", "x/y"),
            ("host", "example.com"),
        ])),
        Err(CapsuleMessageFieldError::ContentType)
    );
}

#[test]
fn error_display() {
    assert_eq!(
        CapsuleMessageFieldError::ContentLength.to_string(),
        "a capsule message has content-length"
    );
    assert_eq!(
        CapsuleMessageFieldError::ContentType.to_string(),
        "a capsule message has content-type"
    );
    assert_eq!(
        CapsuleMessageFieldError::TransferEncoding.to_string(),
        "a capsule message has transfer-encoding"
    );
}

fn response(status: u16, lines: &[&'static str]) -> ResponseHead {
    let mut fields = FieldSection::new();
    for line in lines {
        fields.append(capsule_protocol_field_name(), value(line));
    }
    ResponseHead::new(StatusCode::new(status).unwrap(), fields)
}

#[test]
fn response_uses_the_protocol_with_2xx_or_101_and_true() {
    for status in [101, 200, 202, 299] {
        assert_eq!(
            response_uses_capsule_protocol(&response(status, &["?1"])),
            Ok(true),
            "{status}"
        );
        assert_eq!(
            response_uses_capsule_protocol(&response(status, &["?0"])),
            Ok(false),
            "{status}"
        );
        assert_eq!(
            response_uses_capsule_protocol(&response(status, &[])),
            Ok(false),
            "{status}"
        );
    }
}

#[test]
fn response_with_another_status_does_not_use_the_protocol() {
    // RFC 9297 §3.2: not in use unless 2xx or 101.
    for status in [100, 102, 301, 404, 500] {
        assert_eq!(
            response_uses_capsule_protocol(&response(status, &["?1"])),
            Ok(false),
            "{status}"
        );
    }
}

#[test]
fn response_204_205_206_with_true_is_malformed() {
    for status in [204, 205, 206] {
        let error = response_uses_capsule_protocol(&response(status, &["?1"])).unwrap_err();
        assert_eq!(error.status().get(), status);
        assert_eq!(
            error.to_string(),
            alloc::format!("a response with status {status} uses the capsule protocol")
        );
        assert_eq!(
            response_uses_capsule_protocol(&response(status, &["?0"])),
            Ok(false)
        );
    }
}
