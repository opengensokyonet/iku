use super::*;
use alloc::string::ToString;

fn var_int(value: u64) -> VarInt {
    VarInt::new(value).unwrap()
}

#[test]
fn datagram_is_zero() {
    assert_eq!(CapsuleType::DATAGRAM.get(), VarInt::ZERO);
    assert_eq!(CapsuleType::new(VarInt::ZERO), CapsuleType::DATAGRAM);
}

#[test]
fn type_holds_its_value() {
    assert_eq!(CapsuleType::new(var_int(0x2f)).get(), var_int(0x2f));
    assert_eq!(CapsuleType::new(VarInt::MAX).get(), VarInt::MAX);
}

#[test]
fn type_display_is_hexadecimal() {
    assert_eq!(CapsuleType::DATAGRAM.to_string(), "0x0");
    assert_eq!(CapsuleType::new(var_int(0x2f)).to_string(), "0x2f");
    assert_eq!(
        CapsuleType::new(VarInt::MAX).to_string(),
        "0x3fffffffffffffff"
    );
}

#[test]
fn type_orders_by_value() {
    assert!(CapsuleType::DATAGRAM < CapsuleType::new(var_int(1)));
    assert_eq!(CapsuleType::new(var_int(5)), CapsuleType::new(var_int(5)));
}

#[test]
fn header_holds_type_and_length() {
    let header = CapsuleHeader::new(CapsuleType::new(var_int(0x25)), var_int(1234));
    assert_eq!(header.capsule_type().get(), var_int(0x25));
    assert_eq!(header.length(), var_int(1234));
    assert_eq!(
        CapsuleHeader::new(CapsuleType::DATAGRAM, VarInt::MAX).length(),
        VarInt::MAX
    );
}

#[test]
fn header_equality() {
    let a = CapsuleHeader::new(CapsuleType::DATAGRAM, var_int(3));
    assert_eq!(a, CapsuleHeader::new(CapsuleType::DATAGRAM, var_int(3)));
    assert_ne!(a, CapsuleHeader::new(CapsuleType::DATAGRAM, var_int(4)));
    assert_ne!(
        a,
        CapsuleHeader::new(CapsuleType::new(var_int(1)), var_int(3))
    );
}
