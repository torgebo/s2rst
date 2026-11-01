# Data Model: The `PointRegion` and `LatLng` Codecs

## 1. The formats (upstream)

| Type | Bytes | Layout | Decoder rejects |
|------|-------|--------|-----------------|
| `PointRegion` | 25 | `u8` version (1), `f64` x, y, z (little-endian) | fewer than 25 bytes (`UnexpectedEof`); version > 1, a point that is not `is_unit` (`InvalidData`) |
| `LatLng` | 16 | `f64` latitude, longitude in radians | fewer than 16 bytes (`UnexpectedEof`) |

Trailing bytes are not read.

## 2. Library additions (`core/src/s2/encoding.rs`)

`impl S2Encode for PointRegion`, `impl S2Decode for PointRegion`, `impl S2Encode for LatLng`,
`impl S2Decode for LatLng`, each documented with its format and upstream function. Unit tests
in the module's tests.

## 3. Verification additions

| Item | Where |
|------|-------|
| `PointRegionC`, `LatLngC` descriptors | `verify/src/codec.rs` |
| `ref_point_region`, `enc_point_region`, `point_unit`, `ref_latlng`, `enc_latlng` | `verify/src/wire_spec.rs` |
| P-FX-11 … P-FX-14 harnesses | `verify/src/cd_fx.rs` |
| C++ rows (4 kinds) | `tools/refgen-cpp/generate_coding.cc`, `core/tests/data/cpp_coding_vectors.csv` |
| diff tests | `core/tests/cpp_coding_diff.rs`, `verify/tests/wire_spec_vs_cpp.rs` |
| twins | `core/tests/property_tests.rs` (round trips; decoders on random bytes) |

## 4. Catalogue

| ID | Kind | Statement | Harness |
|----|------|-----------|---------|
| P-FX-11 | round trip | every unit-length `PointRegion` round-trips, bit for bit, leaving a following byte unread; the encoder writes the canonical 25 bytes | `p_fx_11`, `p_fx_11_enc` |
| P-FX-12 | round trip | every `LatLng` (any bits) round-trips, bit for bit; the encoder writes the canonical 16 bytes | `p_fx_12`, `p_fx_12_enc` |
| P-FX-13 | exact input | `PointRegion` agreement on every input up to 26 bytes (acceptance relative to `Point::is_unit`, R1) | `p_fx_13__b0 … __b26` |
| P-FX-14 | exact input | `LatLng` agreement on every input up to 17 bytes | `p_fx_14__b0 … __b17` |

## 5. Mutants (`M-993` …)

| Mutant | Patch | Killed by |
|--------|-------|-----------|
| M-993 | `PointRegion` decode: version check `> 0` | `p_fx_13__b25` |
| M-994 | `PointRegion` decode: unit check dropped | `p_fx_13__b25` |
| M-995 | `PointRegion` decode: unit check inverted | `p_fx_11` |
| M-996 | `PointRegion` encode: version 0 | `p_fx_11_enc` |
| M-997 | `LatLng` encode: fields swapped | `p_fx_12_enc` |
| M-998 | `LatLng` decode: the longitude read into both fields | `p_fx_12` |
