use super::*;
use crate::{CapsuleType, ReadCapsuleHeaderHostExt};
use alloc::vec::Vec;

fn header(capsule_type: u64, length: u64) -> CapsuleHeader {
    CapsuleHeader::new(CapsuleType::new(capsule_type).unwrap(), length).unwrap()
}

fn written(header: CapsuleHeader) -> Vec<u8> {
    let mut out = Vec::new();
    out.sync_write_capsule_header_unpin(header).unwrap();
    out
}

#[test]
fn writes_one_byte_type_and_length() {
    assert_eq!(written(header(0, 3)), [0x00, 0x03]);
    assert_eq!(written(header(0, 0)), [0x00, 0x00]);
    assert_eq!(written(header(63, 63)), [0x3f, 0x3f]);
}

#[test]
fn writes_two_byte_widths() {
    assert_eq!(written(header(0x40, 0)), [0x40, 0x40, 0x00]);
    assert_eq!(written(header(0, 64)), [0x00, 0x40, 0x40]);
    assert_eq!(written(header(0x3fff, 0x3fff)), [0x7f, 0xff, 0x7f, 0xff]);
}

#[test]
fn writes_four_byte_widths() {
    assert_eq!(written(header(0x4000, 0)), [0x80, 0x00, 0x40, 0x00, 0x00]);
    assert_eq!(written(header(0, 0x4000)), [0x00, 0x80, 0x00, 0x40, 0x00]);
}

#[test]
fn writes_eight_byte_widths() {
    let max = (1u64 << 62) - 1;
    assert_eq!(
        written(header(1 << 30, 0)),
        [0xc0, 0, 0, 0, 0x40, 0, 0, 0, 0x00]
    );
    let mut expected = alloc::vec![0xff; 8];
    expected.push(0x00);
    assert_eq!(written(header(max, 0)), expected);
    let mut expected = alloc::vec![0x00];
    expected.extend([0xff; 8]);
    assert_eq!(written(header(0, max)), expected);
    assert_eq!(written(header(max, max)), [0xff; 16]);
}

#[test]
fn writes_mixed_widths() {
    assert_eq!(
        written(header(0x2f, 0x1_0000)),
        [0x2f, 0x80, 0x01, 0x00, 0x00]
    );
}

#[test]
fn appends_to_existing_bytes() {
    let mut out = alloc::vec![0xaa];
    out.sync_write_capsule_header_unpin(header(0, 1)).unwrap();
    out.sync_write_capsule_header_unpin(header(1, 2)).unwrap();
    assert_eq!(out, [0xaa, 0x00, 0x01, 0x01, 0x02]);
}

#[test]
fn round_trips_through_read() {
    let max = (1u64 << 62) - 1;
    let headers = [
        header(0, 0),
        header(0, 63),
        header(0, 64),
        header(63, 16383),
        header(64, 16384),
        header(16383, (1 << 30) - 1),
        header(16384, 1 << 30),
        header((1 << 30) - 1, 1 << 30),
        header(1 << 30, max),
        header(max, 0),
        header(max, max),
    ];
    let mut out = Vec::new();
    for header in headers {
        out.sync_write_capsule_header_unpin(header).unwrap();
    }
    let mut input = out.as_slice();
    for expected in headers {
        assert_eq!(
            input.sync_read_capsule_header_unpin().unwrap(),
            Some(expected)
        );
    }
    assert_eq!(input.sync_read_capsule_header_unpin().unwrap(), None);
}

#[test]
fn writes_to_bytes_mut() {
    let mut out = bytes::BytesMut::new();
    out.sync_write_capsule_header_unpin(header(0, 3)).unwrap();
    assert_eq!(&out[..], &[0x00, 0x03]);
}

#[test]
fn full_slice_reports_a_write_error() {
    // A fixed slice that is too small cannot take the header.
    let mut storage = [0u8; 1];
    let mut out: &mut [u8] = &mut storage;
    let error = out
        .sync_write_capsule_header_unpin(header(0, 3))
        .unwrap_err();
    assert!(matches!(
        error.write_error(),
        WriteAllError::WriteZero { .. }
    ));
    assert_eq!(error.write_error().completed(), 1);
}

#[test]
fn error_display() {
    let mut storage = [0u8; 0];
    let mut out: &mut [u8] = &mut storage;
    let error = out
        .sync_write_capsule_header_unpin(header(0, 3))
        .unwrap_err();
    assert_eq!(
        alloc::string::ToString::to_string(&error),
        "could not write the capsule header"
    );
}
