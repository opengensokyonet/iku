//! Reading and writing a capsule header on a borrowed stream are `Send`
//! when the stream is.
use iku_capsule::{CapsuleHeader, CapsuleType, read_capsule_header, write_capsule_header};
use nitori_call::{ConstructedAccess, ReceiverAccess};
use nitori_io::{Read, Write};
use sakuya_var_int::VarInt;

/// Checks that `future` is `Send`, then runs it.
fn send<T: Future + Send>(future: T) -> T::Output {
    nitori_call::__private::run_sync(future)
}

async fn read_through<I: ReceiverAccess<Family: Read<Metadata = ()>>>(
    io: &mut I,
) -> Option<CapsuleHeader> {
    read_capsule_header(io).await.ok().flatten()
}

async fn write_through<I: ReceiverAccess<Family: Write<(), Report = ()>>>(
    io: &mut I,
    header: CapsuleHeader,
) -> bool {
    write_capsule_header(io, header).await.is_ok()
}

#[test]
fn reading_a_header_from_a_borrowed_stream_is_send() {
    let mut io = ConstructedAccess::new(&[0x00, 0x05, 0x01, 0x02, 0x3f, 0x00][..]);
    let first = send(read_capsule_header(&mut io)).unwrap().unwrap();
    assert_eq!(first.length_field().get(), VarInt::from_u32(5));
    let second = send(read_through(&mut io)).unwrap();
    assert_eq!(second.length_field().get(), VarInt::from_u32(2));
    let third = send(read_through(&mut &mut io)).unwrap();
    assert_eq!(third.length_field().get(), VarInt::from_u32(0));
}

#[test]
fn writing_a_header_into_a_borrowed_stream_is_send() {
    let header = CapsuleHeader::new(CapsuleType::new(VarInt::from_u32(0)), VarInt::from_u32(5));
    let mut io = ConstructedAccess::new(Vec::new());
    send(write_capsule_header(&mut io, header)).unwrap();
    assert!(send(write_through(&mut io, header)));
    assert!(send(write_through(&mut &mut io, header)));
    assert_eq!(io.into_inner(), [0x00, 0x05].repeat(3));
}
