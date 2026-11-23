use super::*;
use crate::WriteCapsuleHeaderHostExt as _;
use alloc::{string::ToString, vec::Vec};
use bytes::Bytes;
use nitori_io::error::IncompleteError;
use sakuya_var_int::{VarInt, VarIntField, VarIntWidth};

fn header(capsule_type: u64, length: u64) -> CapsuleHeader {
    CapsuleHeader::new(
        CapsuleType::new(VarInt::new(capsule_type).unwrap()),
        VarInt::new(length).unwrap(),
    )
}

fn widths(header: CapsuleHeader) -> (VarIntWidth, VarIntWidth) {
    (
        header.capsule_type_field().width(),
        header.length_field().width(),
    )
}

/// Reads a header, reads the first two bytes of its value through
/// `value_limit`, drops the rest with `skip_capsule_value`, then reads the
/// next header.
#[call(sync)]
async fn read_part_then_skip<F: ReadChunk, A: ReceiverAccess<Family = F>>(
    io: &mut A,
) -> Result<([u8; 2], u64, Option<CapsuleHeader>), &'static str> {
    let header = io
        .read_capsule_header()
        .await
        .map_err(|_| "first header")?
        .ok_or("no first header")?;
    let mut value = io.compose(header.value_limit());
    let part = value.read_array::<2>().await.map_err(|_| "part")?;
    let rest = value.into_state().0;
    io.skip_capsule_value(rest).await.map_err(|_| "skip")?;
    let next = io.read_capsule_header().await.map_err(|_| "next header")?;
    Ok((part, rest, next))
}

macro_rules! read_past_value {
    ($name:ident, $count:literal) => {
        /// Reads a header, then reads `$count` bytes through `value_limit`: the
        /// number of bytes read before the stream of the value ends, when it
        /// does.
        #[call(sync)]
        async fn $name<F: ReadChunk, A: ReceiverAccess<Family = F>>(
            io: &mut A,
        ) -> Result<Option<usize>, &'static str> {
            let header = io
                .read_capsule_header()
                .await
                .map_err(|_| "header")?
                .ok_or("no header")?;
            let mut value = io.compose(header.value_limit());
            match value.read_array::<$count>().await {
                Ok(_) => Ok(None),
                Err(nitori_io::calls::ReadArrayError::Incomplete { source }) => {
                    Ok(Some(source.completed()))
                }
                Err(_) => Err("receiver failed"),
            }
        }
    };
}
read_past_value!(read_three_of_value, 3);
read_past_value!(read_one_of_value, 1);

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
    // The second type is read in two bytes and keeps them.
    let wide = CapsuleHeader::with_widths(
        VarIntField::new(
            CapsuleType::new(VarInt::new(0x25).unwrap()),
            VarIntWidth::Two,
        )
        .unwrap(),
        VarIntField::shortest(VarInt::new(1).unwrap()),
    );
    assert_eq!(headers, [header(0, 2), wide, header(0x4000, 0)]);
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
    let read = input.sync_read_capsule_header_unpin().unwrap().unwrap();
    assert_eq!(read.capsule_type(), CapsuleType::DATAGRAM);
    assert_eq!(read.length(), VarInt::new(3).unwrap());
    assert_eq!(widths(read), (VarIntWidth::Two, VarIntWidth::Two));
    assert_ne!(read, header(0, 3));
}

#[test]
fn header_keeps_the_width_of_each_field() {
    use VarIntWidth::{Eight, Four, One, Two};
    let cases: [(&[u8], (VarIntWidth, VarIntWidth)); 5] = [
        (&[0x00, 0x03], (One, One)),
        (&[0x40, 0x00, 0x03], (Two, One)),
        (&[0x00, 0x80, 0, 0, 0x03], (One, Four)),
        (&[0xc0, 0, 0, 0, 0, 0, 0, 0, 0x40, 0x03], (Eight, Two)),
        (&[0x80, 0, 0, 0, 0x80, 0, 0, 0x03], (Four, Four)),
    ];
    for (bytes, expected) in cases {
        let mut input = bytes;
        let read = input.sync_read_capsule_header_unpin().unwrap().unwrap();
        assert_eq!(widths(read), expected, "{bytes:02x?}");
        assert!(input.is_empty());
    }
}

#[test]
fn non_shortest_header_is_written_back_unchanged() {
    // An intermediary forwards a capsule unmodified (RFC 9297 §3.2).
    let inputs: [&[u8]; 4] = [
        &[0x40, 0x00, 0x40, 0x03],
        &[0x80, 0, 0, 0x25, 0xc0, 0, 0, 0, 0, 0, 0, 0x02],
        &[0x00, 0x80, 0, 0, 0x00],
        &[0xc0, 0, 0, 0, 0, 0, 0x3f, 0xff, 0x7f, 0xff],
    ];
    for bytes in inputs {
        let mut input = bytes;
        let read = input.sync_read_capsule_header_unpin().unwrap().unwrap();
        assert!(input.is_empty());
        let mut out = Vec::new();
        out.sync_write_capsule_header_unpin(read).unwrap();
        assert_eq!(out, bytes);
    }
}

#[test]
fn header_unknown_type_is_read() {
    let mut input: &[u8] = &[0x3f, 0x02, 0xaa, 0xbb];
    let h = input.sync_read_capsule_header_unpin().unwrap().unwrap();
    assert_eq!(h.capsule_type().get().get(), 0x3f);
    assert_eq!(h.length().get(), 2);
    assert_eq!(input, [0xaa, 0xbb]);
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

// value_limit

#[test]
fn value_limit_is_the_length() {
    assert_eq!(header(0, 0).value_limit().0, 0);
    assert_eq!(header(0, 1234).value_limit().0, 1234);
    assert_eq!(header(0, (1 << 62) - 1).value_limit().0, (1 << 62) - 1);
}

#[test]
fn value_limit_read_part_then_skip_stays_inside_the_capsule() {
    let mut input: &[u8] = &[
        0x00, 0x05, b'a', b'b', b'c', b'd', b'e', // DATAGRAM, 5
        0x01, 0x00, // type 1, empty
    ];
    let (part, rest, next) = input.sync_read_part_then_skip_unpin().unwrap();
    assert_eq!(&part, b"ab");
    assert_eq!(rest, 3);
    assert_eq!(next, Some(header(1, 0)));
    assert!(input.is_empty());
}

#[test]
fn value_limit_read_whole_value_leaves_nothing_to_skip() {
    let mut input: &[u8] = &[0x00, 0x02, b'x', b'y', 0x00, 0x00];
    let (part, rest, next) = input.sync_read_part_then_skip_unpin().unwrap();
    assert_eq!(&part, b"xy");
    assert_eq!(rest, 0);
    assert_eq!(next, Some(header(0, 0)));
}

#[test]
fn value_limit_read_part_then_skip_ends_at_the_end_of_the_stream() {
    let mut input: &[u8] = &[0x00, 0x03, 1, 2, 3];
    let (_, rest, next) = input.sync_read_part_then_skip_unpin().unwrap();
    assert_eq!(rest, 1);
    assert_eq!(next, None);
}

#[test]
fn value_limit_read_part_then_skip_on_a_bytes_host() {
    let mut input = Bytes::from_static(&[0x00, 0x04, 1, 2, 3, 4, 0x07, 0x00, 9]);
    let (part, rest, next) = input.sync_read_part_then_skip_unpin().unwrap();
    assert_eq!(part, [1, 2]);
    assert_eq!(rest, 2);
    assert_eq!(next, Some(header(7, 0)));
    assert_eq!(&input[..], [9]);
}

#[test]
fn value_limit_ends_the_value_stream_at_its_length() {
    // A parser of the value cannot read into the next capsule.
    let mut input: &[u8] = &[0x00, 0x02, 1, 2, 0x00, 0x01, 9];
    let completed = input.sync_read_three_of_value_unpin().unwrap();
    assert_eq!(completed, Some(2));
    // The bytes of the next capsule are not consumed.
    assert_eq!(input, [0x00, 0x01, 9]);
}

#[test]
fn value_limit_read_exactly_the_length_succeeds() {
    let mut input: &[u8] = &[0x00, 0x03, 1, 2, 3, 0x00];
    assert_eq!(input.sync_read_three_of_value_unpin().unwrap(), None);
    assert_eq!(input, [0x00]);
}

#[test]
fn value_limit_of_an_empty_value_ends_at_once() {
    let mut input: &[u8] = &[0x00, 0x00, 0x00, 0x01, 9];
    let completed = input.sync_read_one_of_value_unpin().unwrap();
    assert_eq!(completed, Some(0));
    assert_eq!(input, [0x00, 0x01, 9]);
}

// skip_capsule_value

#[test]
fn skip_zero_length_reads_nothing() {
    let mut input: &[u8] = &[0xaa];
    input.sync_skip_capsule_value_unpin(0).unwrap();
    assert_eq!(input, [0xaa]);
    let mut empty: &[u8] = &[];
    empty.sync_skip_capsule_value_unpin(0).unwrap();
}

#[test]
fn skip_exactly_the_whole_input() {
    let mut input: &[u8] = b"abcdef";
    input.sync_skip_capsule_value_unpin(6).unwrap();
    assert!(input.is_empty());
}

#[test]
fn skip_does_not_read_the_next_capsule() {
    let mut input: &[u8] = &[b'a', b'b', b'c', 0x00, 0x01, b'z'];
    input.sync_skip_capsule_value_unpin(3).unwrap();
    assert_eq!(input, [0x00, 0x01, b'z']);
    assert_eq!(
        input.sync_read_capsule_header_unpin().unwrap(),
        Some(header(0, 1))
    );
}

#[test]
fn skip_from_bytes_host() {
    let mut input = Bytes::from_static(b"abcdefgh");
    input.sync_skip_capsule_value_unpin(5).unwrap();
    assert_eq!(&input[..], b"fgh");
}

#[test]
fn skip_truncated_is_incomplete() {
    let mut input: &[u8] = b"ab";
    let error = input.sync_skip_capsule_value_unpin(5).unwrap_err();
    assert!(matches!(
        error,
        SkipCapsuleValueError::Incomplete {
            length: 5,
            completed: 2
        }
    ));
    assert_eq!(
        error.to_string(),
        "the stream ended after 2 of 5 bytes of a capsule value"
    );
    assert!(input.is_empty());
}

#[test]
fn skip_empty_stream_is_incomplete_at_zero() {
    let mut input: &[u8] = &[];
    let error = input.sync_skip_capsule_value_unpin(1).unwrap_err();
    assert!(matches!(
        error,
        SkipCapsuleValueError::Incomplete {
            length: 1,
            completed: 0
        }
    ));
}

#[test]
fn skip_length_beyond_u32_is_incomplete_with_the_count() {
    for length in [u64::from(u32::MAX) + 1, 1 << 33, (1 << 62) - 1] {
        let mut input: &[u8] = &[1, 2, 3, 4, 5];
        let error = input.sync_skip_capsule_value_unpin(length).unwrap_err();
        assert!(
            matches!(
                error,
                SkipCapsuleValueError::Incomplete { length: l, completed: 5 } if l == length
            ),
            "{length}: {error:?}"
        );
    }
}

#[test]
fn oversized_datagram_is_skipped_and_reading_goes_on() {
    // RFC 9297 §3.5: a DATAGRAM too long to handle is dropped.
    let mut input: &[u8] = &[
        0x00, 0x05, b'a', b'b', b'c', b'd', b'e', // too long
        0x00, 0x02, b'o', b'k', // fine
    ];
    let first = input.sync_read_capsule_header_unpin().unwrap().unwrap();
    assert_eq!(first.capsule_type(), CapsuleType::DATAGRAM);
    input
        .sync_skip_capsule_value_unpin(first.length().get())
        .unwrap();
    let next = input.sync_read_capsule_header_unpin().unwrap().unwrap();
    assert_eq!(next, header(0, 2));
    assert_eq!(input, b"ok");
    input.sync_skip_capsule_value_unpin(2).unwrap();
    assert_eq!(input.sync_read_capsule_header_unpin().unwrap(), None);
}

#[test]
fn smoke() {
    let mut input: &[u8] = &[0x00, 0x03, b'a', b'b', b'c', 0x40, 0x25, 0x02, b'x', b'y'];
    let first = input.sync_read_capsule_header_unpin().unwrap().unwrap();
    assert_eq!(first.length().get(), 3);
    input
        .sync_skip_capsule_value_unpin(first.length().get())
        .unwrap();
    let second = input.sync_read_capsule_header_unpin().unwrap().unwrap();
    assert_eq!(second.capsule_type().get().get(), 0x25);
    input
        .sync_skip_capsule_value_unpin(second.length().get())
        .unwrap();
    assert!(input.sync_read_capsule_header_unpin().unwrap().is_none());
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

/// Reading that fails once its bytes run out, one byte at a time.
mod failing {
    extern crate std;

    use super::*;
    use nitori_io::{adapters::Chunked, bridge::Std};
    use std::io;

    struct FailAfter<'a> {
        bytes: &'a [u8],
    }

    impl io::Read for FailAfter<'_> {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            let Some((&first, rest)) = self.bytes.split_first() else {
                return Err(io::Error::other("source failed"));
            };
            if buffer.is_empty() {
                return Ok(0);
            }
            buffer[0] = first;
            self.bytes = rest;
            Ok(1)
        }
    }

    #[test]
    fn header_receiver_failure_is_reported_with_its_count() {
        let mut input = Std::new(FailAfter {
            bytes: &[0x00, 0x80, 0x01],
        });
        let error = input.sync_read_capsule_header_unpin().unwrap_err();
        let ReadCapsuleHeaderError::Length { source } = &error else {
            panic!("not a length error: {error:?}");
        };
        assert!(source.host_error().is_some());
        assert_eq!(source.completed(), 2);
    }

    #[test]
    fn skip_receiver_failure_is_reported_with_its_count() {
        let chunk_capacity = NonZeroUsize::new(2).unwrap();
        let mut input = Chunked::new(Std::new(FailAfter { bytes: b"abc" }), chunk_capacity);
        let error = input.sync_skip_capsule_value_unpin(10).unwrap_err();
        let SkipCapsuleValueError::Receiver {
            length, completed, ..
        } = error
        else {
            panic!("not a receiver error: {error:?}");
        };
        assert_eq!((length, completed), (10, 3));
    }
}
