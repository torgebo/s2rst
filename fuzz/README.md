# Fuzzing the s2rst decoders

Coverage-guided, AddressSanitizer-instrumented fuzzing of the byte decoders —
the deep, nightly complement to the bounded `cargo test` suite in
[`core/tests/decoder_robustness.rs`](../core/tests/decoder_robustness.rs).

The decoders accept untrusted input (`&mut dyn Read`) and return `io::Result`,
so the invariant each target checks is simply: **never panic, abort, or hang on
arbitrary bytes.** A few targets go further — they *operate* on the decoded
value (run a query, walk the tree) or assert encode/decode **round-trip
idempotence** — because a decoder that returns `Ok` for structurally-valid but
semantically-junk data can still strand a panic or a silent value corruption in
the code that consumes it.

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
| `decode_s2shape_index`    | `EncodedS2ShapeIndex::init`, then iterate edges + `ContainsPointQuery` over the decoded index |
| `decode_polygon`          | `<Polygon as encoding::S2Decode>::decode`         |
| `decode_points_compressed`| `point_compression::decode_points_compressed`     |
| `decode_density_tree`     | `density_tree::S2DensityTree::{init, decode}`     |
| `density_tree_ops`        | `S2DensityTree::{normalize, leaves, get_partitioning, get_normal_cell_weight, dilate}` over a decoded tree |
| `decode_shape_index_eager`| `shape_index::ShapeIndex::decode_from_reader` (decode only) |
| `decode_loop`             | `<Loop as encoding::S2Decode>::decode`            |
| `decode_lax_polygon`      | `<LaxPolygon as encoding::S2Decode>::decode`      |
| `decode_lax_polyline`     | `<LaxPolyline as encoding::S2Decode>::decode`     |
| `decode_polyline`         | `<Polyline as encoding::S2Decode>::decode`        |
| `decode_cell_union`       | `<CellUnion as encoding::S2Decode>::decode` (+ round-trip) |
| `decode_cell`             | `<Cell as encoding::S2Decode>::decode` (+ round-trip) |
| `decode_cap`              | `<Cap as encoding::S2Decode>::decode` (+ round-trip) |

`decode_uint_vector_u32` and `decode_uint_vector_u64` also assert round-trip
idempotence on the decoded values.

## Seeds and dictionary

`fuzz/corpus/` and `fuzz/artifacts/` are git-ignored, so committed bootstrap
inputs live in **`fuzz/seeds/<target>/`** (a handful of small, size-diverse,
real inputs per target) and the shared libFuzzer **`fuzz/fuzz.dict`** (magic
strings, encoding-version bytes, common little-endian counts).

Both matter for more than speed. Some targets sit behind a coverage wall a blind
mutator never clears on its own:

- **`decode_density_tree` / `density_tree_ops`** require the exact 14-byte magic
  prefix `"S2DensityTree0"` before any decoder code runs — without a seed (or
  the dict entry) carrying it, fuzzing these is a no-op. (See
  [`../fuzz_decode_density_tree.md`](../fuzz_decode_density_tree.md), Finding D.)

Pass the seeds (and dict) explicitly so libFuzzer merges them in:

```sh
cargo +nightly fuzz run <target> fuzz/corpus/<target> fuzz/seeds/<target> \
    -- -dict=fuzz.dict
```

## Running

```sh
# Fuzz one target (Ctrl-C to stop; a crash is written to fuzz/artifacts/).
# The default corpus dir is fuzz/corpus/<target>; add the seeds dir + dict too.
cargo +nightly fuzz run decode_s2point_vector \
    fuzz/corpus/decode_s2point_vector fuzz/seeds/decode_s2point_vector -- -dict=fuzz.dict

# Time-boxed run, as the nightly CI lane does (see .github/workflows/fuzz.yml):
cargo +nightly fuzz run decode_s2point_vector \
    fuzz/corpus/decode_s2point_vector fuzz/seeds/decode_s2point_vector \
    -- -dict=fuzz.dict -max_total_time=300 -rss_limit_mb=4096

# List all targets:
cargo +nightly fuzz list
```

The nightly [`fuzz` workflow](../.github/workflows/fuzz.yml) runs every target on
a matrix, caching each target's corpus between runs (keyed per target) so
coverage compounds instead of restarting from the seeds each night.

A reproducer is saved under `fuzz/artifacts/<target>/`. Re-run it with
`cargo +nightly fuzz run <target> fuzz/artifacts/<target>/<crash-file>`, then add
the minimized input as a regression case to `decoder_robustness.rs`.
