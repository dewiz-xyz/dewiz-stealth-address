# dewiz-stealth-address

ERC-5564 stealth address implementation for EVM-compatible blockchains (Scheme ID 1: secp256k1 + view tags).

## Build & Test

```bash
cargo build          # compile
cargo test           # run all tests
cargo run            # run CLI demo (full Alice→Bob flow)
cargo clippy         # lint — must pass with zero warnings
```

Use always the system variables `RUSTFLAGS="-C target-cpu=native -C link-arg=-s"` before `cargo test`, `cargo run`, and `cargo build`.

## Project Structure

- `src/lib.rs` — crate root, re-exports `stealth` module
- `src/stealth.rs` — core protocol: types, public API, internal helpers, tests
- `src/main.rs` — CLI demo exercising the full stealth address flow
- `Cargo.toml` — dependencies: k256, sha3, rand_core, hex

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| k256 | secp256k1 curve arithmetic (ProjectivePoint, Scalar, NonZeroScalar) |
| sha3 | keccak256 hashing |
| rand_core | OsRng for cryptographic random scalar generation |
| hex | hex encoding/decoding |

## Protocol Encoding Rules (Critical)

- Shared secret hash: `keccak256(0x04 || Sx || Sy)` — SEC1 uncompressed, all 65 bytes
- Ethereum address: `keccak256(Px || Py)[12:32]` — raw 64 bytes, skip 0x04 tag, take last 20
- Meta-address format: `st:eth:0x{compress(K)}{compress(V)}` — 33+33 bytes hex-encoded
- Scalar from hash: `Reduce<U256>::reduce_bytes` for mod n reduction

## Conventions

- All public API lives in `src/stealth.rs`
- No `unwrap()` in library code — return `Result` or `Option`
- `cargo clippy` must pass with zero warnings before any commit
- Tests go in `#[cfg(test)] mod tests` inside `stealth.rs`

## Communication Style

Adopt the persona defined in `.Codex/skills/rust-senior-blockchain-dev.md`.
