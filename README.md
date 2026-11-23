# iku

The parts of HTTP that every HTTP version shares, for Open Gensokyo Network, administratively supported by Starspun Works: the semantics of RFC 9110, the parts of a URI that HTTP uses (RFC 3986), structured fields (RFC 9651) and the Capsule Protocol (RFC 9297), each in a package of its own. The HTTP/3, HTTP/2 and HTTP/1.1 implementations of Open Gensokyo Network build on it.

The repository is at an early stage. The APIs are experimental and are not stable production APIs.

## Layout

Each package has its own documentation in its `src/lib.rs`; `cargo doc --workspace --no-deps --open` builds it.

- `crates/iku_semantics`: the value types of HTTP semantics (RFC 9110) for the head of a message, checked when they are constructed: tokens, field names, field values and field sections, methods, status codes, request targets, the protocol of an extended CONNECT (RFC 8441, RFC 9220), request and response heads, and the `Content-Length`, `Host`, `TE` and `Connection` fields.
- `crates/iku_uri`: the parts of a target URI that HTTP uses (RFC 3986), checked when they are constructed: the scheme, the authority, and the path with the query.
- `crates/iku_structured_fields`: structured field values (RFC 9651): items, lists and dictionaries with their parameters and bare items, parsed from the lines of a field and serialized back; lists and dictionaries can also be read one member at a time.
- `crates/iku_capsule`: the Capsule Protocol (RFC 9297 §3): capsule headers and values read and written as `nitori_call` operations on a `nitori_io` byte stream, and the fields that the protocol reads and forbids.

No package uses the standard library; all need `alloc`.

## Development

Install [rustup](https://rustup.rs/). The repository pins `nightly-2026-11-03` with rustfmt, Clippy and the `x86_64-unknown-none` target in `rust-toolchain.toml`, the toolchain that the other Open Gensokyo Network repositories use; run commands from the repository root so rustup selects this toolchain. `iku_capsule` uses the nightly feature `impl_trait_in_assoc_type`, which crates that define `nitori_call` operations enable; the other packages use none. Packages remain unpublished (`publish = false`).

`snafu`, `nitori_call`, `nitori_io` and `sakuya_var_int` (the QUIC variable-length integers that the Capsule Type and Length use, from the sakuya repository) are Git dependencies pinned to specific commits in `Cargo.toml`; Cargo fetches them automatically.

`serde_json` is a development dependency of `iku_structured_fields` only, for the HTTP working group's structured-field test vectors in `crates/iku_structured_fields/tests/vectors/` (BSD 3-Clause; source commit and license in that directory's `README.md`).

From the repository root:

```sh
cargo fmt --all --check
for package in crates/*/; do cargo clippy --locked -p "$(basename "$package")" --all-targets -- -D warnings; done
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-targets
cargo test --locked --workspace --all-targets --all-features
cargo test --locked --workspace --doc
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps
for package in iku_semantics iku_structured_fields iku_capsule; do cargo build --locked -p "$package" --target x86_64-unknown-none; done
```

The last command checks that the packages, and with them `iku_uri`, build without the standard library.

For development against local checkouts of SNAFU, nitori or sakuya, an ignored `.cargo/config.toml` may patch their Git sources to local paths under `[patch."https://github.com/opengensokyonet/snafu"]`, `[patch."https://github.com/opengensokyonet/nitori"]` and `[patch."https://github.com/opengensokyonet/sakuya"]`. Keep that configuration and the resulting lockfile changes out of commits; the committed lockfile records the public sources.

## License

Software and code examples are licensed under either [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your option. Explanatory documentation is licensed under [CC BY 4.0](LICENSE-CC-BY). Contributors retain copyright in their contributions. Project names and logos are not licensed as trademarks by these licenses.
