# Implementation Plan: The `PointRegion` and `LatLng` Codecs

**Branch**: `tb/014-frm/1` | **Date**: 2026-11-01 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/014-pointregion-latlng-codecs/spec.md`

## Summary

Add upstream's two missing standalone encodings, `S2PointRegion` (version byte, three doubles,
unit-length check on decoding) and `S2LatLng` (two doubles, any bytes), as `S2Encode`/`S2Decode`
implementations, and verify them as spec 003 verified the other fixed formats: new descriptors
in the codec framework, the reference description extended, round trips, canonical encoders and
agreement on every input up to the format's length plus one, C++ vectors and their diff tests,
twins, mutants and records. The unit-length check is handled by spec 003's R18 fallback with a
pure stub (research R1).

## Technical Context

**Language/Version**: Rust edition 2024; Kani 0.68.0, CBMC 6.11.0; MSRV 1.88.

**Primary Dependencies**: none new.

**Testing**: `make test`; `cd verify && cargo test`; `make verify-decoding P=codecs`;
`make verify-mutants`.

**Target Platform**: Linux x86_64; GitHub `ubuntu-latest`.

**Project Type**: a library feature with its verification.

**Performance Goals**: the `codecs` leg stays under 10 minutes (SC-004): 2 round trips, 2 encoder
checks and 45 agreement harnesses of a few seconds each (spec 003's `Point` and `Rect` ones).

**Constraints**: byte-for-byte parity; no behaviour change elsewhere; Kani runs one at a time.

**Scale/Scope**: 4 trait implementations; 4 catalogue entries, 49 harnesses; 4 C++ row kinds;
6 mutants.

## Constitution Check

| Principle / rule | Assessment | Status |
|------------------|-----------|--------|
| **I. Upstream Parity** | Both formats byte for byte as upstream, version 0 and invalid `LatLng`s accepted as upstream; C++ vectors check it | ✅ Pass |
| **II. Robustness & Memory Safety** | Decoders proven crash-free on every input up to length + 1 | ✅ Pass |
| **III. Test-Backed Changes** | Unit tests, twins, C++ diff tests, Kani proofs | ✅ Pass |
| **IV. Thin, Synchronized Bindings** | The bindings expose no fixed-format codec (`Point`, `Cap`, `Rect` neither); none added | ✅ Pass |
| **V. Measured Performance** | Codecs of fixed size; no hot path changes | ✅ Pass |
| **Workflow: verification records** | Records updated in the same change | ✅ Pass |

**Gate result: PASS.** **Post-design re-check:** still PASS.

## Project Structure

```text
specs/014-pointregion-latlng-codecs/
├── plan.md, research.md (R0–R6), data-model.md, quickstart.md
├── contracts/{verify-commands,harness-conventions}.md
└── checklists/requirements.md

core/src/s2/encoding.rs                     # the four impls and unit tests
core/tests/{cpp_coding_diff,property_tests}.rs; core/tests/data/cpp_coding_vectors.csv
tools/refgen-cpp/generate_coding.cc
verify/src/{codec,wire_spec,cd_fx}.rs; verify/tests/wire_spec_vs_cpp.rs
verify/decoding/{INVENTORY,PROPERTIES,EVALUATION}.md; verify/{PROPERTIES_BY_KIND.md,.html,MUTANTS.md}
verify/mutants/M-993 …; VERIFIED_TYPES.md, CORE_TRAITS.md
```

**Structure Decision**: the existing decoding suite and its `codecs` part.

## Complexity Tracking

No exceptions.
