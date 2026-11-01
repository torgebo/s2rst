# Implementation Plan: `Cell`'s Bounding Laws, and the Region Adaptors

**Branch**: `tb/012-frm/1` | **Date**: 2026-10-30 (R0 run the same day) | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/012-formal-verify-cell-bounds-adaptors/spec.md`

## Summary

- **The region adaptors** join the value suite (`ad_*`, part `P=adaptors`). Their members are
  *abstract regions* (`verify/src/any_region.rs`): any answers, one per input, so a statement holds
  for every implementor. Proven: crash freedom (the tests for up to 3 members per change, the
  bounds weekly), the meaning (`any`/`all` folds, the rectangle bound's containment, the empty
  adaptors), and the preservation of three `Region` laws: containment implies intersection, the
  vertex law and the bounding rectangle, for up to 3 members (2 for the vertex law) and for any
  number by a written argument. The bounding-cap and covering laws are recorded with reasons (they
  need `Rect`'s and `Cap`'s, not proven).
- **`Cell`'s bounding laws** for its own points: R0 found both out of reach (the cap's timeouts,
  the rectangle's need of `atan2`'s monotonicity); they are catalogue rows with reasons and the
  probe's 240,000 own points as evidence. The covering law's closed form holds by composition.
- **U-017** (the bounds miss `contains_point`'s margin, upstream too): documented as the maintainer
  chose, with a fixed test, `UPSTREAM.md` and `CORE_TRAITS.md`. **K2**: `region(i)`'s panic
  documented.
- **C++ vectors** for both (FR-018).

## Technical Context

**Language/Version**: Rust edition 2024; Kani 0.68.0 (`-Z stubbing`), CBMC 6.11.0; MSRV 1.88.

**Primary Dependencies**: none new.

**Testing**: `make verify-values P=adaptors`; `make verify-slow F=values P=…`; `make
verify-release`; `cargo test` in `verify/`; `core/tests/cpp_cell_bounds_diff.rs` (new),
`cell_tests`, the adaptors' tests, `trait_property_tests`; `make test`.

**Target Platform**: Linux x86_64; GitHub `ubuntu-latest`.

**Project Type**: an extension of the verification harness.

**Performance Goals**: the `values-adaptors` leg under 10 minutes (SC-006). R0's per-change set:
the tests and containment at `n ≤ 3` (12–23 s each), the rectangle law at 1 (21.5 s), the vertex
law at 1 (80.5 s), the bounds of the empty adaptors; the rest weekly (the bounds at 1 and 2
members, 260–308 s; the rectangle law at 2 and 3; the vertex law at 2).

**Constraints**: no API or behaviour change; concrete member counts whose table keys fit ten
slots; every run through `run-verify.sh`, one at a time.

**Scale/Scope**: inventory 16 adaptor entries (14 Kani, `Debug` excluded twice); catalogue 10
adaptor entries (8 counted) and 4 `Cell` rows; about 30 per-change harnesses, 10 weekly; about 12
mutants `M-974` …; 3 new C++ vector sections; 1 upstream record (U-017).

## Constitution Check

| Principle / rule | Assessment | Status |
|------------------|-----------|--------|
| **I. Upstream Parity** | The adaptors match upstream line for line (the probe); U-017 is upstream's own and kept, at the maintainer's choice; the C++ vectors check both | ✅ Pass |
| **II. Robustness & Memory Safety** | Crash freedom for the adaptors over any members; `region(i)`'s panic documented and proven exact | ✅ Pass |
| **III. Test-Backed Changes** | Documentation changes only, each with a test (FR-016, FR-017) | ✅ Pass |
| **IV. Thin, Synchronized Bindings** | No binding change | ✅ Pass |
| **V. Measured Performance** | No code change | ✅ Pass |
| **Workflow: verification records** | Every harness change updates the records in the same commit | ✅ Pass |
| **Workflow: focused PRs** | The documentation and vectors ship with the feature, each its own commit | ⚠️ Justified, as in 001–011 |

**Gate result: PASS.** **Post-design re-check:** still PASS.

**Spec criteria at risk:** SC-004's `Cell` half is "proven or recorded with a reason", and R0 says
recorded; SC-006 is unmeasured on a hosted runner (a push is needed).

## Project Structure

```text
specs/012-formal-verify-cell-bounds-adaptors/
├── plan.md, research.md (R0–R7), data-model.md, quickstart.md
├── contracts/{verify-commands,harness-conventions}.md
├── probe/   # survey (cpp/, rust/), R0 harnesses and results, README
└── checklists/requirements.md

verify/src/any_region.rs                 # NEW (the abstract region)
verify/src/ad_{crash,mean,reg}.rs        # NEW
verify/src/{lib,stubs}.rs; verify/src/region_spec_tests.rs (the adaptors' drift guard)
verify/values/{INVENTORY,PROPERTIES,EVALUATION}.md; verify/cell_union/PROPERTIES.md
verify/{PROPERTIES_BY_KIND.md,.html,TRUSTED_BASE,MUTANTS,COVERAGE}.md; verify/mutants/M-97N…
core/src/s2/cell.rs, cell_tests.rs                     # FR-016 (documentation, a test)
core/src/s2/region_intersection.rs (and its tests)     # FR-017
core/tests/cpp_cell_bounds_diff.rs, core/tests/data/cpp_cell_bounds_vectors.csv   # NEW
core/tests/trait_property_tests.rs                     # FR-012
tools/refgen-cpp/generate_cell_bounds.cc (NEW)
UPSTREAM.md (U-017), VERIFIED_TYPES.md, CORE_TRAITS.md, Makefile (P=adaptors), .github/workflows/verify.yml
```

**Structure Decision**: one new value-suite part, `P=adaptors`, its own CI leg; `Cell`'s laws are
catalogue rows only.

## Complexity Tracking

| Exception | Why | Simpler alternative rejected because |
|-----------|-----|--------------------------------------|
| Abstract member regions instead of concrete ones | A preservation law must hold for every implementor, and concrete members would each bring their own cost (a real `Cell` member alone runs a projection) | Concrete members prove the law for those types only |
