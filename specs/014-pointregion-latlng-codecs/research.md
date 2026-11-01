# Research: The `PointRegion` and `LatLng` Codecs

Spec: [spec.md](spec.md). Upstream: `s2point_region.cc:55–78`, `s2latlng.cc:38–54`,
`s2pointutil.cc:32–40` (v0.14.0-40).

## R0: Risk

**Decision:** no planning probe. Both codecs are spec 003's fixed formats again (`Point`'s three
doubles, `Rect`'s version byte), and the framework (`codec::Codec`, `cd_fx`'s `round_trip`,
`check_encoder` and `agreement!`) takes them as new descriptors. The one cost risk is
`PointRegion`'s unit-length check: a float predicate in the decoder, like `Cap::is_valid`, whose
agreement harnesses at 32–33 bytes went weekly in spec 003 (R18). R1 removes it.

## R1: `PointRegion`'s unit-length check in the proofs

**Decision:** the descriptor `PointRegionC` states acceptance relative to the library's own
`Point::is_unit` (spec 003's R18 fallback for `Cap`), and its harnesses stub `Point::is_unit`
with spec 008's `stubs::pure_is_unit` (one answer per point, L-CP-04 and -07). The decoder's
logic (length, version, fields, the check's place and polarity, consumption) is then proven
exactly; `is_unit`'s formula is upstream's `IsUnitLength` (`|x² + y² + z² − 1| ≤ 5ε`), which the
C++ rows check on points at `5ε`'s edge and spec 006's catalogue already records.

**Alternatives considered:** restating the predicate in `wire_spec` and asserting it against the
library's: the float products would be computed twice (spec 003's T048 timeouts).

## R2: The reference description

**Decision:** `wire_spec` gains `ref_point_region` (25 bytes; version ≤ 1; three doubles;
`point_unit`, upstream's formula restated) and `enc_point_region` (version 1), `ref_latlng` and
`enc_latlng` (16 bytes, any bits). `wire_spec_vs_cpp` checks them on the C++ rows, so the
restated `point_unit` is checked against upstream's `IsUnitLength` there.

## R3: Harnesses and ids

- `cd_fx::p_fx_11` (`PointRegion` round trip, every unit point, `pure_is_unit`), `p_fx_11_enc`
  (the canonical bytes), `p_fx_12` (`LatLng` round trip, every pair of bit patterns),
  `p_fx_12_enc`; `p_fx_13__b0 … __b26` (`PointRegion` agreement), `p_fx_14__b0 … __b17`
  (`LatLng` agreement). Covers: rejected for a short input; at 25 and 26 bytes, accepted, and
  rejected for the version and for the unit check.
- Catalogue: P-FX-11 … P-FX-14 in `verify/decoding/PROPERTIES.md`; inventory rows in
  `verify/decoding/INVENTORY.md`.
- Part: the existing `make verify-decoding P=codecs` (the `cd_fx` module), no new part.

## R4: C++ vectors

**Decision:** `generate_coding.cc` appends `POINT_REGION_ENCODE`/`_DECODE` and
`LATLNG_ENCODE`/`_DECODE` rows after every existing section, so the existing rows (one seeded
generator) are unchanged: random unit points, points at `|norm² − 1|` just inside and outside
`5ε`, zero, NaN and infinite coordinates, versions 0, 1, 2 and 255, every truncation length, a
trailing byte; for `LatLng` random and special pairs (NaN, ±∞, ±0, out-of-range). The diff test
`cpp_coding_diff.rs` and `wire_spec_vs_cpp.rs` handle the four kinds.

## R5: Mutants

`M-993` onwards: the version check off by one (`> 0`), the unit check dropped, the unit check
inverted, the encoder's version 0, `LatLng`'s fields swapped on encoding, `LatLng`'s decoder
reading the longitude twice.

## R6: Records and CI

The decoding suite's inventory, catalogue (P-FX-11 … 14) and evaluation (a spec-014 part); the
index by kind; `MUTANTS.md`; `VERIFIED_TYPES.md` (§2.15 and the `S2Encode`/`S2Decode`
implementors); `CORE_TRAITS.md` (the codec law's implementors). CI's decoding `codecs` leg runs
`cd_fx` already; `core/src/s2/encoding.rs` is in the path filters.

## R7: Measured in implementation (2026-11-01)

- The 51 new harnesses: 136 s of solver time in all, the longest about 13 s. `pure_is_unit`'s
  table needs an unwind of 11 in the harnesses that reach the unit check (`p_fx_11`,
  `p_fx_13__b25`, `__b26`); the first run failed on the unwinding assertion at 5 and 9.
- The `codecs` leg rose from 8.2 min (142 harnesses, 3 jobs, spec 003) to 11.0 min (197, 4
  jobs): over SC-004's budget. It is split into `P=codecs` (101 harnesses, 400 s) and `P=fixed`
  (96, 385 s), with a `decoding-fixed` CI leg in both the per-change and the weekly matrix.
- The C++ rows: 240 new, all matched; the 4,715 earlier rows unchanged.
- Mutants `M-993` … `M-998`: all killed.
