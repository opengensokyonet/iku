use super::*;
use alloc::{string::ToString, vec, vec::Vec};
use bytes::Buf;
use nitori_io::error::IncompleteError;
use sakuya_var_int::VarInt;

fn nz(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).unwrap()
}

fn header(capsule_type: u64, length: u64) -> CapsuleHeader {
    CapsuleHeader::new(
        CapsuleType::new(VarInt::new(capsule_type).unwrap()),
        VarInt::new(length).unwrap(),
    )
}

/// Collects the chunks that `read_capsule_value` yields, each as bytes.
#[call(sync)]
async fn value_chunks<F: ReadChunk, A: ReceiverAccess<Family = F>>(
    io: &mut A,
    header: CapsuleHeader,
    chunk_maximum: NonZeroUsize,
) -> Result<Vec<Vec<u8>>, ReadCapsuleValueError<F::Error>> {
    let mut all = Vec::new();
    let mut chunks = pin!(io.read_capsule_value(header, chunk_maximum));
    loop {
        match chunks.as_mut().step().await {
            ControlFlow::Continue(chunk) => all.push(chunk.chunk().to_vec()),
            ControlFlow::Break(result) => return result.map(|()| all),
        }
    }
}

#[test]
fn smoke() {
    let bytes: &[u8] = &[0x00, 0x03, b'a', b'b', b'c', 0x40, 0x25, 0x02, b'x', b'y'];
    let mut input = bytes;
    let header = input.sync_read_capsule_header_unpin().unwrap().unwrap();
    assert_eq!(header.capsule_type(), CapsuleType::DATAGRAM);
    assert_eq!(header.length().get(), 3);
    let value = input
        .sync_collect_capsule_value_unpin(header, 16, NonZeroUsize::new(2).unwrap())
        .unwrap();
    assert_eq!(value.as_deref(), Some(&b"abc"[..]));
    let header = input.sync_read_capsule_header_unpin().unwrap().unwrap();
    assert_eq!(header.capsule_type().get().get(), 0x25);
    let value = input
        .sync_collect_capsule_value_unpin(header, 1, NonZeroUsize::new(1).unwrap())
        .unwrap();
    assert_eq!(value, None);
    assert!(input.sync_read_capsule_header_unpin().unwrap().is_none());
    let mut truncated: &[u8] = &[0x00, 0x05, b'a'];
    let header = truncated.sync_read_capsule_header_unpin().unwrap().unwrap();
    assert!(
        truncated
            .sync_collect_capsule_value_unpin(header, 16, NonZeroUsize::new(2).unwrap())
            .is_err()
    );
    let mut half: &[u8] = &[0x40];
    assert!(half.sync_read_capsule_header_unpin().is_err());
}

// read_capsule_header

#[test]
fn header_reads_consecutive_capsules() {
    let mut input: &[u8] = &[
        0x00, 0x02, 1, 2, // DATAGRAM, 2
        0x40, 0x25, 0x01, 9, // type 0x25 in two bytes, 1
        0x80, 0x00, 0x40, 0x00, 0x00, // type 0x4000, length 0
    ];
    let mut headers = Vec::new();
    while let Some(h) = input.sync_read_capsule_header_unpin().unwrap() {
        headers.push(h);
        // skip the value
        let n = h.length().get() as usize;
        input = &input[n..];
    }
    assert_eq!(headers, [header(0, 2), header(0x25, 1), header(0x4000, 0)]);
}

#[test]
fn header_empty_stream_is_none() {
    let mut input: &[u8] = &[];
    assert_eq!(input.sync_read_capsule_header_unpin().unwrap(), None);
    // And it stays at the end.
    assert_eq!(input.sync_read_capsule_header_unpin().unwrap(), None);
}

#[test]
fn header_end_between_capsules_is_none() {
    let mut input: &[u8] = &[0x01, 0x00];
    assert_eq!(
        input.sync_read_capsule_header_unpin().unwrap(),
        Some(header(1, 0))
    );
    assert_eq!(input.sync_read_capsule_header_unpin().unwrap(), None);
}

#[test]
fn header_all_widths() {
    let mut input: &[u8] = &[
        0x7f, 0xff, 0x80, 0x00, 0x40, 0x00, // 16383, 16384
        0xc0, 0, 0, 0, 0x40, 0, 0, 0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    ];
    assert_eq!(
        input.sync_read_capsule_header_unpin().unwrap(),
        Some(header(16383, 16384))
    );
    assert_eq!(
        input.sync_read_capsule_header_unpin().unwrap(),
        Some(header(1 << 30, (1 << 62) - 1))
    );
}

#[test]
fn header_non_minimal_encodings_accepted() {
    let mut input: &[u8] = &[0x40, 0x00, 0x40, 0x03];
    assert_eq!(
        input.sync_read_capsule_header_unpin().unwrap(),
        Some(header(0, 3))
    );
}

#[test]
fn header_unknown_type_is_read() {
    let mut input: &[u8] = &[0x3f, 0x02, 0xaa, 0xbb];
    let h = input.sync_read_capsule_header_unpin().unwrap().unwrap();
    assert_eq!(h.capsule_type().get().get(), 0x3f);
    assert_eq!(h.length().get(), 2);
    let chunks = input.sync_value_chunks_unpin(h, nz(8)).unwrap();
    assert_eq!(chunks, [vec![0xaa, 0xbb]]);
}

#[test]
fn header_truncated_type_is_type_error() {
    for bytes in [&[0x40][..], &[0x80, 0x00], &[0xc0, 0, 0, 0, 0, 0, 0]] {
        let mut input = bytes;
        let error = input.sync_read_capsule_header_unpin().unwrap_err();
        assert!(
            matches!(
                error,
                ReadCapsuleHeaderError::Type {
                    source: ReadVarIntError::Incomplete { .. }
                }
            ),
            "{bytes:02x?}"
        );
    }
}

#[test]
fn header_end_before_length_is_length_error() {
    let mut input: &[u8] = &[0x00];
    let error = input.sync_read_capsule_header_unpin().unwrap_err();
    let ReadCapsuleHeaderError::Length {
        source: ReadVarIntError::Incomplete { source },
    } = error
    else {
        panic!("not an incomplete length: {error:?}");
    };
    assert_eq!(source.completed(), 0);
}

#[test]
fn header_truncated_length_is_length_error() {
    for bytes in [
        &[0x00, 0x40][..],
        &[0x00, 0x80, 0x01, 0x02],
        &[0x00, 0xc0, 1, 2, 3, 4, 5, 6],
    ] {
        let mut input = bytes;
        let error = input.sync_read_capsule_header_unpin().unwrap_err();
        assert!(
            matches!(
                error,
                ReadCapsuleHeaderError::Length {
                    source: ReadVarIntError::Incomplete { .. }
                }
            ),
            "{bytes:02x?}"
        );
    }
}

#[test]
fn header_error_display() {
    let mut input: &[u8] = &[0x40];
    let error = input.sync_read_capsule_header_unpin().unwrap_err();
    assert_eq!(error.to_string(), "could not read the capsule type");
    let mut input: &[u8] = &[0x00];
    let error = input.sync_read_capsule_header_unpin().unwrap_err();
    assert_eq!(error.to_string(), "could not read the capsule length");
}

#[test]
fn header_leaves_value_unread() {
    let mut input: &[u8] = &[0x00, 0x03, b'a', b'b', b'c'];
    input.sync_read_capsule_header_unpin().unwrap().unwrap();
    assert_eq!(input, b"abc");
}

// read_capsule_value

#[test]
fn value_zero_length_yields_nothing() {
    let mut input: &[u8] = &[0xaa];
    let chunks = input.sync_value_chunks_unpin(header(0, 0), nz(4)).unwrap();
    assert!(chunks.is_empty());
    assert_eq!(input, &[0xaa]);
}

#[test]
fn value_is_chunked_by_maximum() {
    let mut input: &[u8] = b"abcdefg";
    let chunks = input.sync_value_chunks_unpin(header(0, 7), nz(3)).unwrap();
    assert_eq!(chunks, [b"abc".to_vec(), b"def".to_vec(), b"g".to_vec()]);
    assert!(input.is_empty());
}

#[test]
fn value_exact_multiple_of_maximum() {
    let mut input: &[u8] = b"abcdef";
    let chunks = input.sync_value_chunks_unpin(header(0, 6), nz(3)).unwrap();
    assert_eq!(chunks, [b"abc".to_vec(), b"def".to_vec()]);
}

#[test]
fn value_maximum_above_length_is_one_chunk() {
    let mut input: &[u8] = b"abcd";
    let chunks = input
        .sync_value_chunks_unpin(header(0, 4), nz(1024))
        .unwrap();
    assert_eq!(chunks, [b"abcd".to_vec()]);
}

#[test]
fn value_maximum_one() {
    let mut input: &[u8] = b"xyz";
    let chunks = input.sync_value_chunks_unpin(header(0, 3), nz(1)).unwrap();
    assert_eq!(chunks, [b"x".to_vec(), b"y".to_vec(), b"z".to_vec()]);
}

#[test]
fn value_does_not_read_the_next_capsule() {
    let mut input: &[u8] = &[b'a', b'b', b'c', 0x00, 0x01, b'z'];
    let chunks = input.sync_value_chunks_unpin(header(0, 3), nz(2)).unwrap();
    let joined: Vec<u8> = chunks.concat();
    assert_eq!(joined, b"abc");
    assert_eq!(input, &[0x00, 0x01, b'z']);
}

#[test]
fn value_chunks_never_exceed_maximum() {
    let data: Vec<u8> = (0..=99).collect();
    for maximum in [1, 2, 7, 10, 99, 100, 101] {
        let mut input = data.as_slice();
        let chunks = input
            .sync_value_chunks_unpin(header(0, 100), nz(maximum))
            .unwrap();
        assert!(chunks.iter().all(|c| !c.is_empty() && c.len() <= maximum));
        assert_eq!(chunks.concat(), data);
    }
}

#[test]
fn value_from_bytes_host() {
    let mut input = Bytes::from_static(b"abcdefgh");
    let chunks = input.sync_value_chunks_unpin(header(0, 5), nz(2)).unwrap();
    assert_eq!(chunks.concat(), b"abcde");
    assert_eq!(&input[..], b"fgh");
}

#[test]
fn value_truncated_is_value_error() {
    let mut input: &[u8] = b"ab";
    let error = input
        .sync_value_chunks_unpin(header(0, 5), nz(4))
        .unwrap_err();
    let ReadCapsuleValueError::Value {
        source: ReadChunksExactError::Incomplete { source },
    } = &error
    else {
        panic!("not an incomplete value: {error:?}");
    };
    assert_eq!(source.completed(), 2);
    assert_eq!(error.to_string(), "could not read the capsule value");
}

#[test]
fn value_empty_stream_is_value_error() {
    let mut input: &[u8] = &[];
    let error = input
        .sync_value_chunks_unpin(header(0, 1), nz(4))
        .unwrap_err();
    assert!(matches!(error, ReadCapsuleValueError::Value { .. }));
}

#[test]
fn value_length_too_large_is_reported_on_small_targets_only() {
    // On targets where usize is 64 bits every valid length fits, so
    // LengthTooLarge cannot be produced here.
    if usize::BITS < 64 {
        let mut input: &[u8] = &[];
        let h = header(0, 1 << 40);
        let error = input.sync_value_chunks_unpin(h, nz(4)).unwrap_err();
        assert!(matches!(
            error,
            ReadCapsuleValueError::LengthTooLarge { length } if length == 1 << 40
        ));
    }
}

// collect_capsule_value

#[test]
fn collect_returns_whole_value_within_maximum() {
    let mut input: &[u8] = b"abcdef";
    let value = input
        .sync_collect_capsule_value_unpin(header(0, 4), 4, nz(2))
        .unwrap();
    assert_eq!(value.as_deref(), Some(&b"abcd"[..]));
    assert_eq!(input, b"ef");
}

#[test]
fn collect_zero_length_is_empty_bytes() {
    let mut input: &[u8] = b"x";
    let value = input
        .sync_collect_capsule_value_unpin(header(0, 0), 0, nz(2))
        .unwrap();
    assert_eq!(value.as_deref(), Some(&b""[..]));
    assert_eq!(input, b"x");
}

#[test]
fn collect_below_maximum() {
    let mut input: &[u8] = b"abc";
    let value = input
        .sync_collect_capsule_value_unpin(header(0, 3), 100, nz(1))
        .unwrap();
    assert_eq!(value.as_deref(), Some(&b"abc"[..]));
}

#[test]
fn collect_above_maximum_discards_and_continues() {
    let mut input: &[u8] = &[b'a', b'b', b'c', b'd', b'e', 0x00, 0x02, b'o', b'k'];
    let h = header(0, 5);
    let value = input.sync_collect_capsule_value_unpin(h, 4, nz(2)).unwrap();
    assert_eq!(value, None);
    let next = input.sync_read_capsule_header_unpin().unwrap().unwrap();
    assert_eq!(next, header(0, 2));
    let value = input
        .sync_collect_capsule_value_unpin(next, 2, nz(2))
        .unwrap();
    assert_eq!(value.as_deref(), Some(&b"ok"[..]));
    assert_eq!(input.sync_read_capsule_header_unpin().unwrap(), None);
}

#[test]
fn collect_maximum_zero_discards_nonempty() {
    let mut input: &[u8] = b"ab";
    let value = input
        .sync_collect_capsule_value_unpin(header(0, 2), 0, nz(1))
        .unwrap();
    assert_eq!(value, None);
    assert!(input.is_empty());
}

#[test]
fn collect_truncated_whole_is_collect_error() {
    let mut input: &[u8] = b"ab";
    let error = input
        .sync_collect_capsule_value_unpin(header(0, 5), 16, nz(2))
        .unwrap_err();
    let CollectCapsuleValueError::Collect {
        source: ReadExactError::Incomplete { source },
    } = &error
    else {
        panic!("not an incomplete collect: {error:?}");
    };
    assert_eq!(source.completed(), 2);
    assert_eq!(error.to_string(), "could not read the capsule value");
}

#[test]
fn collect_truncated_discard_is_discard_error() {
    let mut input: &[u8] = b"ab";
    let error = input
        .sync_collect_capsule_value_unpin(header(0, 5), 3, nz(2))
        .unwrap_err();
    assert!(
        matches!(
            error,
            CollectCapsuleValueError::Discard {
                source: ReadCapsuleValueError::Value { .. }
            }
        ),
        "{error:?}"
    );
    assert_eq!(error.to_string(), "could not discard the capsule value");
}

#[test]
fn collect_unknown_type_is_read_like_any() {
    let mut input: &[u8] = &[0x7f, 0xff, 0x02, 1, 2];
    let h = input.sync_read_capsule_header_unpin().unwrap().unwrap();
    assert_eq!(h.capsule_type().get().get(), 0x3fff);
    let value = input.sync_collect_capsule_value_unpin(h, 8, nz(8)).unwrap();
    assert_eq!(value.as_deref(), Some(&[1u8, 2][..]));
}

#[test]
fn collect_from_bytes_host() {
    let mut input = Bytes::from_static(b"abcxyz");
    let value = input
        .sync_collect_capsule_value_unpin(header(0, 3), 3, nz(2))
        .unwrap();
    assert_eq!(value, Some(Bytes::from_static(b"abc")));
    assert_eq!(&input[..], b"xyz");
}

#[test]
fn incomplete_error_type_is_nitori_io() {
    // The documentation promises nitori_io::IncompleteError inside.
    let mut input: &[u8] = &[0x40];
    let error = input.sync_read_capsule_header_unpin().unwrap_err();
    let ReadCapsuleHeaderError::Type {
        source: ReadVarIntError::Incomplete { source },
    } = error
    else {
        panic!();
    };
    let _: IncompleteError = source;
}

#[test]
fn header_truncated_length_counts_the_whole_encoding() {
    // The length announces four bytes; two of them are there.
    let mut input: &[u8] = &[0x00, 0x80, 0x01];
    let error = input.sync_read_capsule_header_unpin().unwrap_err();
    let ReadCapsuleHeaderError::Length {
        source: ReadVarIntError::Incomplete { source },
    } = error
    else {
        panic!("not an incomplete length: {error:?}");
    };
    assert_eq!((source.expected(), source.completed()), (4, 2));
}
