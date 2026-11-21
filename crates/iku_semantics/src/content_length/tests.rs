use super::*;
use alloc::format;
use alloc::vec::Vec;
use bytes::Bytes;

fn section(lines: &[&'static str]) -> FieldSection {
    lines
        .iter()
        .map(|value| {
            (
                FieldName::CONTENT_LENGTH,
                FieldValue::new(Bytes::from_static(value.as_bytes())).unwrap(),
            )
        })
        .collect()
}

fn read(lines: &[&'static str]) -> Result<Option<ContentLength>, ContentLengthError> {
    ContentLength::from_fields(&section(lines))
}

fn length(octets: u64) -> Result<Option<ContentLength>, ContentLengthError> {
    Ok(Some(ContentLength::new(octets)))
}

#[test]
fn content_length_is_none_without_lines() {
    assert_eq!(ContentLength::from_fields(&FieldSection::new()), Ok(None));
}

#[test]
fn content_length_ignores_other_fields() {
    let mut fields = FieldSection::new();
    fields.append(FieldName::HOST, FieldValue::from_static("a").unwrap());
    assert_eq!(ContentLength::from_fields(&fields), Ok(None));
}

#[test]
fn content_length_reads_one_number() {
    assert_eq!(read(&["42"]), length(42));
    assert_eq!(read(&["0"]), length(0));
}

#[test]
fn content_length_keeps_the_value_of_a_single_line_with_leading_zeros() {
    assert_eq!(read(&["007"]), length(7));
}

#[test]
fn content_length_merges_two_lines_of_the_same_number() {
    // RFC 9110 §8.6.
    assert_eq!(read(&["42", "42"]), length(42));
}

#[test]
fn content_length_merges_a_list_of_the_same_number() {
    assert_eq!(read(&["42, 42"]), length(42));
    assert_eq!(read(&["42,42,42"]), length(42));
}

#[test]
fn content_length_merges_a_repeated_equal_number_over_lines_and_lists() {
    assert_eq!(read(&["42, 42", "42"]), length(42));
}

#[test]
fn content_length_compares_members_as_numbers() {
    assert_eq!(read(&["42, 042"]), length(42));
    assert_eq!(read(&["0, 000"]), length(0));
}

#[test]
fn content_length_rejects_different_numbers() {
    assert_eq!(read(&["42, 43"]), Err(ContentLengthError::DifferentValues));
    assert_eq!(
        read(&["42", "43"]),
        Err(ContentLengthError::DifferentValues)
    );
}

#[test]
fn content_length_rejects_a_member_that_is_not_decimal() {
    for value in ["4a", "-1", "+1", "\"42\"", "4 2", "0x2a", "4.2", "\u{e9}"] {
        assert_eq!(
            read(&[value]),
            Err(ContentLengthError::NotDecimal),
            "{value:?}"
        );
    }
}

#[test]
fn content_length_rejects_a_bad_member_after_a_good_one() {
    assert_eq!(read(&["42, 4a"]), Err(ContentLengthError::NotDecimal));
    assert_eq!(read(&["42", "-1"]), Err(ContentLengthError::NotDecimal));
}

#[test]
fn content_length_reports_the_first_broken_rule_in_order() {
    // 42 then 43 is seen before the non-decimal member.
    assert_eq!(
        read(&["42, 43, x"]),
        Err(ContentLengthError::DifferentValues)
    );
    // x is seen before the mismatch.
    assert_eq!(read(&["x, 42, 43"]), Err(ContentLengthError::NotDecimal));
}

#[test]
fn content_length_rejects_a_value_without_members() {
    assert_eq!(read(&[""]), Err(ContentLengthError::Empty));
    assert_eq!(read(&[","]), Err(ContentLengthError::Empty));
    assert_eq!(read(&[",,, ,"]), Err(ContentLengthError::Empty));
    assert_eq!(read(&["", ","]), Err(ContentLengthError::Empty));
}

#[test]
fn content_length_ignores_empty_members() {
    assert_eq!(read(&[",42,"]), length(42));
    assert_eq!(read(&["", "42"]), length(42));
}

#[test]
fn content_length_accepts_u64_max() {
    assert_eq!(read(&["18446744073709551615"]), length(u64::MAX));
}

#[test]
fn content_length_rejects_one_above_u64_max() {
    assert_eq!(
        read(&["18446744073709551616"]),
        Err(ContentLengthError::OutOfRange)
    );
    assert_eq!(
        read(&["184467440737095516150"]),
        Err(ContentLengthError::OutOfRange)
    );
    assert_eq!(
        read(&["99999999999999999999"]),
        Err(ContentLengthError::OutOfRange)
    );
}

#[test]
fn content_length_accepts_a_long_run_of_leading_zeros() {
    assert_eq!(read(&["000000000000000000000000000042"]), length(42));
}

#[test]
fn content_length_to_field_value_has_no_leading_zeros() {
    let cases: [(u64, &str); 6] = [
        (0, "0"),
        (7, "7"),
        (10, "10"),
        (u64::from(u16::MAX), "65535"),
        (u64::MAX, "18446744073709551615"),
        (1_000_000_000_000, "1000000000000"),
    ];
    for (octets, text) in cases {
        let value = ContentLength::new(octets).to_field_value();
        assert_eq!(value.as_bytes(), text.as_bytes());
    }
}

#[test]
fn content_length_round_trips_through_a_field_value() {
    for octets in [0, 1, 9, 10, 99, 100, u64::MAX - 1, u64::MAX] {
        let mut fields = FieldSection::new();
        fields.append(
            FieldName::CONTENT_LENGTH,
            ContentLength::new(octets).to_field_value(),
        );
        assert_eq!(ContentLength::from_fields(&fields), length(octets));
    }
}

#[test]
fn content_length_get_and_ordering() {
    assert_eq!(ContentLength::new(5).get(), 5);
    assert!(ContentLength::new(5) < ContentLength::new(6));
}

#[test]
fn content_length_errors_display() {
    let errors = [
        ContentLengthError::Empty,
        ContentLengthError::NotDecimal,
        ContentLengthError::OutOfRange,
        ContentLengthError::DifferentValues,
    ];
    let texts: Vec<_> = errors.iter().map(|e| format!("{e}")).collect();
    assert_eq!(texts[0], "content-length holds no number");
    assert_eq!(texts.len(), 4);
}
