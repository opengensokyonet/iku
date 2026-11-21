# iku

The parts of HTTP that every HTTP version shares, for Open Gensokyo Network, administratively supported by Starspun Works: the semantics of RFC 9110, and later structured fields (RFC 9651) and the Capsule Protocol (RFC 9297), each in a package of its own. The HTTP/3, HTTP/2 and HTTP/1.1 implementations of Open Gensokyo Network build on it.

The repository is at an early stage. The APIs are experimental and are not stable production APIs.

## Layout

Each package has its own documentation in its `src/lib.rs`; `cargo doc --workspace --no-deps --open` builds it.

- `crates/iku_semantics`: the value types of HTTP semantics (RFC 9110) for the head of a message, checked when they are constructed: tokens, field names, field values and field sections, methods, status codes, the parts of a target URI (RFC 3986), request targets, the protocol of an extended CONNECT (RFC 8441, RFC 9220), request and response heads, and the `Content-Length`, `Host`, `TE` and `Connection` fields. It does not use the standard library and needs `alloc`.

## Development

Install [rustup](https://rustup.rs/). The repository pins `nightly-2026-11-03` with rustfmt, Clippy and the `x86_64-unknown-none` target in `rust-toolchain.toml`, the toolchain that the other Open Gensokyo Network repositories use; run commands from the repository root so rustup selects this toolchain. No nightly language feature is used. Packages remain unpublished (`publish = false`).

`snafu` is a Git dependency pinned to a specific commit in `Cargo.toml`; Cargo fetches it automatically.

From the repository root:

```sh
cargo fmt --all --check
for package in crates/*/; do cargo clippy --locked -p "$(basename "$package")" --all-targets -- -D warnings; done
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-targets
cargo test --locked --workspace --all-targets --all-features
cargo test --locked --workspace --doc
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps
cargo build --locked -p iku_semantics --target x86_64-unknown-none
```

The last command checks that `iku_semantics` builds without the standard library.

For development against a local checkout of SNAFU, an ignored `.cargo/config.toml` may patch its Git source to local paths under `[patch."https://github.com/opengensokyonet/snafu"]`. Keep that configuration and the resulting lockfile changes out of commits; the committed lockfile records the public sources.

## License

Software and code examples are licensed under either [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your option. Explanatory documentation is licensed under [CC BY 4.0](LICENSE-CC-BY). Contributors retain copyright in their contributions. Project names and logos are not licensed as trademarks by these licenses.
