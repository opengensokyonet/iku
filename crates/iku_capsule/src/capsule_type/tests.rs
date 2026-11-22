use super::*;
use alloc::string::ToString;

const MAX: u64 = (1 << 62) - 1;

#[test]
fn datagram_is_zero() {
    assert_eq!(CapsuleType::DATAGRAM.get(), 0);
    assert_eq!(CapsuleType::new(0), Ok(CapsuleType::DATAGRAM));
}

#[test]
fn type_accepts_up_to_the_maximum() {
    assert_eq!(CapsuleType::new(1).unwrap().get(), 1);
    assert_eq!(CapsuleType::new(0x2f).unwrap().get(), 0x2f);
    assert_eq!(CapsuleType::new(MAX).unwrap().get(), MAX);
}

#[test]
fn type_rejects_above_the_maximum() {
    let error = CapsuleType::new(MAX + 1).unwrap_err();
    assert_eq!(error.value(), 1 << 62);
    assert_eq!(
        error.to_string(),
        "capsule type 0x4000000000000000 is above 2^62 - 1"
    );
    assert_eq!(CapsuleType::new(u64::MAX).unwrap_err().value(), u64::MAX);
}

#[test]
fn type_display_is_hexadecimal() {
    assert_eq!(CapsuleType::DATAGRAM.to_string(), "0x0");
    assert_eq!(CapsuleType::new(0x2f).unwrap().to_string(), "0x2f");
}

#[test]
fn type_orders_by_value() {
    assert!(CapsuleType::DATAGRAM < CapsuleType::new(1).unwrap());
    assert_eq!(CapsuleType::new(5).unwrap(), CapsuleType::new(5).unwrap());
}

#[test]
fn header_holds_type_and_length() {
    let header = CapsuleHeader::new(CapsuleType::new(0x25).unwrap(), 1234).unwrap();
    assert_eq!(header.capsule_type().get(), 0x25);
    assert_eq!(header.length(), 1234);
}

#[test]
fn header_accepts_zero_and_maximum_length() {
    assert_eq!(
        CapsuleHeader::new(CapsuleType::DATAGRAM, 0)
            .unwrap()
            .length(),
        0
    );
    assert_eq!(
        CapsuleHeader::new(CapsuleType::DATAGRAM, MAX)
            .unwrap()
            .length(),
        MAX
    );
}

#[test]
fn header_rejects_length_above_the_maximum() {
    let error = CapsuleHeader::new(CapsuleType::DATAGRAM, MAX + 1).unwrap_err();
    assert_eq!(error.length(), 1 << 62);
    assert_eq!(
        error.to_string(),
        "capsule length 4611686018427387904 is above 2^62 - 1"
    );
}

#[test]
fn header_equality() {
    let a = CapsuleHeader::new(CapsuleType::DATAGRAM, 3).unwrap();
    assert_eq!(a, CapsuleHeader::new(CapsuleType::DATAGRAM, 3).unwrap());
    assert_ne!(a, CapsuleHeader::new(CapsuleType::DATAGRAM, 4).unwrap());
    assert_ne!(
        a,
        CapsuleHeader::new(CapsuleType::new(1).unwrap(), 3).unwrap()
    );
}
