//! The Capsule Protocol ([RFC 9297] §3), for iku.
//!
//! A capsule is a type, a length and a value, carried on the data stream of
//! an HTTP request that upgrades or uses extended CONNECT: the content of the
//! DATA frames in HTTP/2 and HTTP/3, and the bytes after the upgrade in
//! HTTP/1.1. Capsules are read and written as `nitori_call` operations on a
//! `nitori_io` byte stream, so each HTTP version lends its own stream:
//!
//! - [`read_capsule_header`] reads a [`CapsuleHeader`] ([`CapsuleType`] and
//!   length), or ends at the end of the stream;
//! - [`read_capsule_value`] yields the value in chunks, and
//!   [`collect_capsule_value`] reads a short one whole;
//! - [`write_capsule_header`] writes a header; the caller writes the value.
//!
//! A capsule of an unknown type is read like any other; an endpoint drops it
//! and an intermediary forwards it unmodified (§3.2). The fields that say the
//! protocol is in use and the ones that it forbids are in
//! [`uses_capsule_protocol`] and [`check_capsule_message_fields`].
//!
//! The package does not use the standard library; it needs `alloc`.
//!
//! [RFC 9297]: https://www.rfc-editor.org/rfc/rfc9297.html
#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![feature(impl_trait_in_assoc_type, yield_expr)]

extern crate alloc;

mod capsule_type;
mod fields;
// `#[call]` generates an operation struct, its fields and extension traits
// without documentation; the functions they come from are documented.
#[allow(missing_docs)]
mod read;
mod var_int;
#[allow(missing_docs)]
mod write;

pub use capsule_type::{
    CapsuleHeader, CapsuleLengthOutOfRangeError, CapsuleType, CapsuleTypeOutOfRangeError,
};
pub use fields::{
    CapsuleMessageFieldError, capsule_protocol_field_name, capsule_protocol_field_value,
    check_capsule_message_fields, uses_capsule_protocol,
};
pub use read::{
    CollectCapsuleValue, CollectCapsuleValueError, CollectCapsuleValueExt,
    CollectCapsuleValueHostExt, ReadCapsuleHeader, ReadCapsuleHeaderError, ReadCapsuleHeaderExt,
    ReadCapsuleHeaderHostExt, ReadCapsuleValue, ReadCapsuleValueError, ReadCapsuleValueExt,
    ReadCapsuleValueHostExt, collect_capsule_value, read_capsule_header, read_capsule_value,
};
pub use write::{
    WriteCapsuleHeader, WriteCapsuleHeaderError, WriteCapsuleHeaderExt, WriteCapsuleHeaderHostExt,
    write_capsule_header,
};
