//! The Capsule Protocol ([RFC 9297] §3), for iku.
//!
//! A capsule is a type, a length and a value, carried on the data stream of
//! an HTTP request that upgrades or uses extended CONNECT: the content of the
//! DATA frames in HTTP/2 and HTTP/3, and the bytes after the upgrade in
//! HTTP/1.1. Capsules are read and written as `nitori_call` operations on a
//! `nitori_io` byte stream, so each HTTP version lends its own stream:
//!
//! - [`read_capsule_header`] reads a [`CapsuleHeader`] ([`CapsuleType`] and
//!   length, with the widths of their encodings), or ends at the end of the
//!   stream;
//! - the value is the stream itself, composed with
//!   [`CapsuleHeader::value_limit`] so that it ends where the value does: a
//!   reader of the value, such as the parser of a capsule type, reads it at
//!   its own pace, and the stream waits for it;
//! - [`skip_capsule_value`] drops a value, or what is left of one;
//! - [`write_capsule_header`] writes a header; the caller writes the value.
//!
//! A capsule of an unknown type is read like any other; an endpoint drops it
//! and an intermediary forwards it unmodified (§3.2). The fields that say the
//! protocol is in use and the ones that it forbids are in
//! [`uses_capsule_protocol`], [`response_uses_capsule_protocol`] and
//! [`check_capsule_message_fields`].
//!
//! The package does not use the standard library; it needs `alloc`.
//!
//! [RFC 9297]: https://www.rfc-editor.org/rfc/rfc9297.html
#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![feature(impl_trait_in_assoc_type)]

extern crate alloc;

mod capsule_type;
mod fields;
// `#[call]` generates an operation struct, its fields and extension traits
// without documentation; the functions they come from are documented.
#[allow(missing_docs)]
mod read;
#[allow(missing_docs)]
mod write;

pub use capsule_type::{CapsuleHeader, CapsuleType};
pub use fields::{
    CapsuleMessageFieldError, CapsuleProtocolStatusError, capsule_protocol_field_name,
    capsule_protocol_field_value, check_capsule_message_fields, response_uses_capsule_protocol,
    uses_capsule_protocol,
};
pub use read::{
    ReadCapsuleHeader, ReadCapsuleHeaderError, ReadCapsuleHeaderExt, ReadCapsuleHeaderHostExt,
    SkipCapsuleValue, SkipCapsuleValueError, SkipCapsuleValueExt, SkipCapsuleValueHostExt,
    read_capsule_header, skip_capsule_value,
};
pub use write::{
    WriteCapsuleHeader, WriteCapsuleHeaderError, WriteCapsuleHeaderExt, WriteCapsuleHeaderHostExt,
    write_capsule_header,
};
