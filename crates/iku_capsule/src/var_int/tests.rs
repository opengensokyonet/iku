use super::*;
use alloc::vec::Vec;

/// Reads one whole variable-length integer: the first byte, then the rest.
#[call(sync)]
async fn read_whole<F: Read, A: ReceiverAccess<Family = F>>(
    io: &mut A,
) -> Result<u64, ReadArrayError<F::Error>> {
    let [first] = io.read_array::<1>().await?;
    read_var_int_rest(io, first).await
}

fn decode(bytes: &[u8]) -> u64 {
    let mut input = bytes;
    let value = input.sync_read_whole_unpin().unwrap();
    assert!(input.is_empty(), "{bytes:02x?} left bytes unread");
    value
}

fn encode(value: u64) -> Vec<u8> {
    let mut out = [0u8; 8];
    let width = put_var_int(&mut out, value);
    out[..width].to_vec()
}

#[test]
fn max_is_two_to_the_62_minus_one() {
    assert_eq!(MAX, (1u64 << 62) - 1);
}

#[test]
fn reads_one_byte() {
    assert_eq!(decode(&[0x25]), 37);
    assert_eq!(decode(&[0x00]), 0);
    assert_eq!(decode(&[0x3f]), 63);
}

#[test]
fn reads_two_bytes() {
    assert_eq!(decode(&[0x7b, 0xbd]), 15293);
    assert_eq!(decode(&[0x40, 0x40]), 64);
    assert_eq!(decode(&[0x7f, 0xff]), 16383);
}

#[test]
fn reads_four_bytes() {
    assert_eq!(decode(&[0x9d, 0x7f, 0x3e, 0x7d]), 494_878_333);
    assert_eq!(decode(&[0x80, 0x00, 0x40, 0x00]), 16384);
    assert_eq!(decode(&[0xbf, 0xff, 0xff, 0xff]), (1 << 30) - 1);
}

#[test]
fn reads_eight_bytes() {
    assert_eq!(
        decode(&[0xc2, 0x19, 0x7c, 0x5e, 0xff, 0x14, 0xe8, 0x8c]),
        151_288_809_941_952_652
    );
    assert_eq!(decode(&[0xc0, 0, 0, 0, 0x40, 0, 0, 0]), 1 << 30);
    assert_eq!(decode(&[0xff; 8]), MAX);
}

#[test]
fn accepts_non_minimal_encodings() {
    // RFC 9000 Appendix A.1: 0x4025 is also 37.
    assert_eq!(decode(&[0x40, 0x25]), 37);
    assert_eq!(decode(&[0x80, 0x00, 0x00, 0x25]), 37);
    assert_eq!(decode(&[0xc0, 0, 0, 0, 0, 0, 0, 0x25]), 37);
}

#[test]
fn reads_only_its_own_bytes() {
    let mut input: &[u8] = &[0x40, 0x25, 0xaa, 0xbb];
    assert_eq!(input.sync_read_whole_unpin().unwrap(), 37);
    assert_eq!(input, &[0xaa, 0xbb]);
}

#[test]
fn truncated_encoding_is_incomplete() {
    for bytes in [&[0x40][..], &[0x80, 0x01, 0x02], &[0xc0, 1, 2, 3, 4, 5, 6]] {
        let mut input = bytes;
        let error = input.sync_read_whole_unpin().unwrap_err();
        assert!(
            matches!(error, ReadArrayError::Incomplete { .. }),
            "{bytes:02x?}"
        );
    }
}

#[test]
fn writes_shortest_encoding() {
    assert_eq!(encode(0), [0x00]);
    assert_eq!(encode(37), [0x25]);
    assert_eq!(encode(63), [0x3f]);
    assert_eq!(encode(64), [0x40, 0x40]);
    assert_eq!(encode(15293), [0x7b, 0xbd]);
    assert_eq!(encode(16383), [0x7f, 0xff]);
    assert_eq!(encode(16384), [0x80, 0x00, 0x40, 0x00]);
    assert_eq!(encode(494_878_333), [0x9d, 0x7f, 0x3e, 0x7d]);
    assert_eq!(encode((1 << 30) - 1), [0xbf, 0xff, 0xff, 0xff]);
    assert_eq!(encode(1 << 30), [0xc0, 0, 0, 0, 0x40, 0, 0, 0]);
    assert_eq!(
        encode(151_288_809_941_952_652),
        [0xc2, 0x19, 0x7c, 0x5e, 0xff, 0x14, 0xe8, 0x8c]
    );
    assert_eq!(encode(MAX), [0xff; 8]);
}

#[test]
fn put_returns_the_width() {
    let mut out = [0u8; 8];
    for (value, width) in [
        (0, 1),
        (63, 1),
        (64, 2),
        (16383, 2),
        (16384, 4),
        ((1 << 30) - 1, 4),
        (1 << 30, 8),
        (MAX, 8),
    ] {
        assert_eq!(put_var_int(&mut out, value), width, "{value}");
    }
}

#[test]
fn round_trips() {
    for value in [
        0,
        1,
        63,
        64,
        255,
        16383,
        16384,
        65535,
        (1 << 30) - 1,
        1 << 30,
        1 << 40,
        MAX - 1,
        MAX,
    ] {
        assert_eq!(decode(&encode(value)), value);
    }
}
