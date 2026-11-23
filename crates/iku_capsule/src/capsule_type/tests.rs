use super::*;
use alloc::string::ToString;
use sakuya_var_int::VarIntWidth;

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

fn field<T>(value: T, width: VarIntWidth) -> VarIntField<T>
where
    T: Copy + Into<VarInt> + TryFrom<VarInt>,
{
    VarIntField::new(value, width).unwrap()
}

#[test]
fn with_encoding_keeps_the_given_widths() {
    let capsule_type = field(CapsuleType::new(var_int(0x25)), VarIntWidth::Four);
    let length = field(var_int(3), VarIntWidth::Two);
    let header = CapsuleHeader::with_encoding(capsule_type, length);
    assert_eq!(header.capsule_type(), CapsuleType::new(var_int(0x25)));
    assert_eq!(header.length(), var_int(3));
    assert_eq!(header.capsule_type_field(), capsule_type);
    assert_eq!(header.length_field(), length);
    assert_eq!(header.capsule_type_field().width(), VarIntWidth::Four);
    assert_eq!(header.length_field().width(), VarIntWidth::Two);
}

#[test]
fn new_takes_the_shortest_widths() {
    let cases = [
        (0, VarIntWidth::One),
        (63, VarIntWidth::One),
        (64, VarIntWidth::Two),
        (16383, VarIntWidth::Two),
        (16384, VarIntWidth::Four),
        ((1 << 30) - 1, VarIntWidth::Four),
        (1 << 30, VarIntWidth::Eight),
        ((1 << 62) - 1, VarIntWidth::Eight),
    ];
    for (value, width) in cases {
        let header = CapsuleHeader::new(CapsuleType::new(var_int(value)), var_int(value));
        assert_eq!(header.capsule_type_field().width(), width, "{value}");
        assert_eq!(header.length_field().width(), width, "{value}");
    }
}

#[test]
fn headers_that_differ_only_in_width_are_not_equal() {
    let shortest = CapsuleHeader::new(CapsuleType::DATAGRAM, var_int(3));
    let wide_length = CapsuleHeader::with_encoding(
        field(CapsuleType::DATAGRAM, VarIntWidth::One),
        field(var_int(3), VarIntWidth::Two),
    );
    let wide_type = CapsuleHeader::with_encoding(
        field(CapsuleType::DATAGRAM, VarIntWidth::Eight),
        field(var_int(3), VarIntWidth::One),
    );
    assert_ne!(shortest, wide_length);
    assert_ne!(shortest, wide_type);
    assert_ne!(wide_length, wide_type);
    assert_eq!(wide_length.length(), shortest.length());
    assert_eq!(wide_length.capsule_type(), shortest.capsule_type());
    let same = CapsuleHeader::with_encoding(
        field(CapsuleType::DATAGRAM, VarIntWidth::One),
        field(var_int(3), VarIntWidth::One),
    );
    assert_eq!(shortest, same);
}

#[test]
fn type_converts_from_and_into_var_int() {
    let value = var_int(0x2f);
    let capsule_type = CapsuleType::from(value);
    assert_eq!(capsule_type, CapsuleType::new(value));
    assert_eq!(VarInt::from(capsule_type), value);
    let back: VarInt = CapsuleType::DATAGRAM.into();
    assert_eq!(back, VarInt::ZERO);
    let from: CapsuleType = VarInt::MAX.into();
    assert_eq!(from.get(), VarInt::MAX);
}
