# Fuzzing the s2rst decoders

Coverage-guided, AddressSanitizer-instrumented fuzzing of the byte decoders —
the deep, nightly complement to the bounded `cargo test` suite in
[`core/tests/decoder_robustness.rs`](../core/tests/decoder_robustness.rs).

The decoders accept untrusted input (`&mut dyn Read`) and return `io::Result`,
so the invariant each target checks is simply: **never panic, abort, or hang on
arbitrary bytes.**

## Requirements

```
rustup toolchain install nightly
cargo install cargo-fuzz
```

This is a standalone workspace (see `[workspace]` in `Cargo.toml`) so it stays
out of the main `cargo build` / CI matrix; it is built and run only on demand.

## Targets

| Target | Decoder |
|--------|---------|
| `decode_uint_vector_u32`  | `encoded_uint_vector::decode_uint_vector_u32`     |
| `decode_uint_vector_u64`  | `encoded_uint_vector::decode_uint_vector_u64`     |
| `decode_string_vector`    | `encoded_string_vector::decode_string_vector`     |
| `decode_s2cell_id_vector` | `encoded_s2cell_id_vector::decode_s2cell_id_vector` |
| `decode_s2point_vector`   | `encoded_s2point_vector::decode_s2point_vector`   |

## Running

```sh
# Fuzz one target (Ctrl-C to stop; a crash is written to fuzz/artifacts/).
cargo +nightly fuzz run decode_s2point_vector

# Time-boxed run, e.g. for a nightly CI lane:
cargo +nightly fuzz run decode_s2point_vector -- -max_total_time=300 -rss_limit_mb=2048

# List all targets:
cargo +nightly fuzz list
```

A reproducer is saved under `fuzz/artifacts/<target>/`. Re-run it with
`cargo +nightly fuzz run <target> fuzz/artifacts/<target>/<crash-file>`, then add
the minimized input as a regression case to `decoder_robustness.rs`.
