# Contract: Harness and Artifact Conventions (changes for spec 014)

003's to 013's contracts still apply.

- **Descriptors** `PointRegionC`, `LatLngC` in `codec.rs`; `PointRegionC` states acceptance
  relative to `Point::is_unit` (R18's fallback), and its harnesses stub it with `pure_is_unit`.
- **Harness ids** `cd_fx::p_fx_11`, `_enc`, `p_fx_12`, `_enc`, `p_fx_13__b<n>`, `p_fx_14__b<n>`.
- **Mutants** `M-993` onwards, series 9.
