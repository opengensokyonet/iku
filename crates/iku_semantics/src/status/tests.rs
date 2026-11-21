use super::*;
use alloc::format;

#[test]
fn status_code_new_accepts_the_range_edges() {
    assert_eq!(StatusCode::new(100).unwrap().get(), 100);
    assert_eq!(StatusCode::new(599).unwrap().get(), 599);
    assert_eq!(StatusCode::new(200).unwrap().get(), 200);
}

#[test]
fn status_code_new_rejects_just_outside_the_range() {
    for code in [0, 1, 99, 600, 999, 1000, u16::MAX] {
        let error = StatusCode::new(code).unwrap_err();
        assert_eq!(error.code(), code);
    }
}

#[test]
fn status_code_new_works_in_const_context() {
    const OK: Result<StatusCode, StatusCodeError> = StatusCode::new(204);
    assert_eq!(OK.unwrap().get(), 204);
}

#[test]
fn status_code_class_follows_the_first_digit() {
    // RFC 9110 §15.
    let cases = [
        (100, StatusClass::Informational),
        (199, StatusClass::Informational),
        (200, StatusClass::Successful),
        (299, StatusClass::Successful),
        (300, StatusClass::Redirection),
        (399, StatusClass::Redirection),
        (400, StatusClass::ClientError),
        (499, StatusClass::ClientError),
        (500, StatusClass::ServerError),
        (599, StatusClass::ServerError),
    ];
    for (code, class) in cases {
        assert_eq!(StatusCode::new(code).unwrap().class(), class, "{code}");
    }
}

#[test]
fn status_code_to_ascii_digits() {
    assert_eq!(StatusCode::new(100).unwrap().to_ascii_digits(), *b"100");
    assert_eq!(StatusCode::new(404).unwrap().to_ascii_digits(), *b"404");
    assert_eq!(StatusCode::new(599).unwrap().to_ascii_digits(), *b"599");
}

#[test]
fn status_code_parse_reads_three_digits() {
    assert_eq!(StatusCode::parse(b"200").unwrap().get(), 200);
    assert_eq!(StatusCode::parse(b"100").unwrap().get(), 100);
    assert_eq!(StatusCode::parse(b"599").unwrap().get(), 599);
}

#[test]
fn status_code_parse_round_trips_with_to_ascii_digits() {
    for code in 100..=599 {
        let status = StatusCode::new(code).unwrap();
        assert_eq!(StatusCode::parse(&status.to_ascii_digits()), Ok(status));
    }
}

#[test]
fn status_code_parse_rejects_the_wrong_number_of_digits() {
    for input in [&b""[..], b"2", b"20", b"2000", b"0200"] {
        assert_eq!(
            StatusCode::parse(input),
            Err(ParseStatusCodeError::NotThreeDigits),
            "{input:?}"
        );
    }
}

#[test]
fn status_code_parse_rejects_non_digits() {
    for input in [
        &b"2x0"[..],
        b" 20",
        b"20 ",
        b"+20",
        b"-20",
        b"2.0",
        b"\xd9\xa0a",
    ] {
        assert_eq!(
            StatusCode::parse(input),
            Err(ParseStatusCodeError::NotThreeDigits),
            "{input:?}"
        );
    }
}

#[test]
fn status_code_parse_reports_out_of_range_codes() {
    for (input, code) in [(&b"099"[..], 99), (b"600", 600), (b"000", 0), (b"999", 999)] {
        let error = StatusCode::parse(input).unwrap_err();
        assert!(
            matches!(error, ParseStatusCodeError::OutOfRange { source } if source.code() == code),
            "{input:?}: {error:?}"
        );
    }
}

#[test]
fn status_code_orders_by_number_and_displays() {
    assert!(StatusCode::new(200).unwrap() < StatusCode::new(404).unwrap());
    assert_eq!(format!("{}", StatusCode::new(404).unwrap()), "404");
    assert_eq!(
        format!("{:?}", StatusCode::new(404).unwrap()),
        "StatusCode(404)"
    );
}

#[test]
fn status_errors_display() {
    assert_eq!(
        format!("{}", StatusCode::new(600).unwrap_err()),
        "status code 600 is not from 100 to 599"
    );
    assert_eq!(
        format!("{}", ParseStatusCodeError::NotThreeDigits),
        "a status code is not three digits"
    );
}
