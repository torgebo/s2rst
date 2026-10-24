# Implementation Plan: Formal Verification of `r1::Interval` and `s2::Rect`

**Branch**: `004-specs-2/1` | **Date**: 2026-10-12 (after the R0 probe and the R1 re-check of
upstream) | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/005-formal-verify-r1-interval-rect/spec.md`

## Summary

The feature proves the real interval `r1::Interval` and the latitude–longitude rectangle
`s2::Rect` with Kani over their whole domains, as the next members of 004's value-type suite:

- **Crash freedom:** 26 real-interval and 34 rectangle operations.
- **Meaning:** a 31-entry catalogue against two new descriptions:
  - `real_spec`, the closed set of numbers between the bounds;
  - `rect_spec`, the product of `real_spec` and 004's `arc_spec`.

The planning probe (R0) measured the open questions:

- **The rectangle composes cheaply through descriptions.** Every product-stated property proves
  in 0.1–62 s, approximate equality aside (256 s, the circular interval's `ieee_remainder` as in
  004). Stating it component-wise times out at 12 minutes. So the rectangle reuses 004's
  descriptions, laws and generators, and lets Kani re-explore the interval code it calls.
- **The real interval's laws prove in 0.2–21 s.** Its arithmetic meets `∞ − ∞`, which Kani
  reports as a failed NaN check. So the arithmetic operations are proven for finite bounds, and
  their infinite corners are documented. The spec was amended. The probe also found that
  approximate equality is not reflexive for an interval with an infinite bound.
- **Expansion containment** times out per change, as in 004. It goes to the weekly tier.

R1 re-read upstream:

- K1 is inherited;
- K2 follows Go and is documented as a deviation from C++;
- K3 matches upstream's debug-only checks;
- three new upstream findings:
  - U-008 (candidate): C++ `R1Interval::Project` on an empty interval reaches undefined
    behaviour through `std::clamp` in release builds, where the library panics;
  - U-009: C++ `R1Interval::GetDirectedHausdorffDistance` returns 0 for `[0, ∞]` against
    `[5, ∞]`, where the true distance, and the library's answer, is 5;
  - U-010 (found by the vectors during implementation): C++ `R1Interval::GetCenter`
    overflows for finite bounds beyond `DBL_MAX / 2`, where the library's `f64::midpoint`
    does not.

The feature changes `core/` in three separately committed ways:

- **FR-017 to FR-019:** documentation: the infinite corners, the Go-style deviation, and the
  panics (R7).
- **FR-020:** a hand-written `Deserialize` for `s2::Rect` that rejects invalid rectangles, as
  its binary decoder does. A million honest JSON round trips stayed valid, so it needs no
  tolerance (R6).
- **FR-021:** four new sections in 004's C++ generator, with differential tests (R9).

## Technical Context

**Language/Version**: Rust, edition 2024. Kani's pinned nightly for the proofs; `core/` stays on
MSRV 1.88; the C++ generator builds against the local s2geometry install, not in CI.

**Primary Dependencies**: Kani 0.68.0, CBMC 6.11.0. **No dependency changes.**

**Storage**: N/A. The artifacts are Markdown, Rust and CSV rows.

**Testing**:
- **Proofs:** `make verify-values P=real|rect`.
- **Models:** `cargo test` in `verify/` (`real_spec_tests`, `rect_spec_tests`,
  `values_spec_vs_cpp`, `property_kinds`).
- **Non-vacuity:** `make verify-mutants` (`M-4NN`).
- **Changes to `core/`:**
  - the FR-020 tests in `core/src/s2/rect_tests.rs`;
  - the extended `core/tests/cpp_values_diff.rs`.
- **Gate:** `make test`, definition unchanged.

**Target Platform**: Linux x86_64, both the workstation and GitHub `ubuntu-latest` (4 vCPUs,
16 GB).

**Project Type**: An extension of the verification harness for a library.

**Performance Goals** (R8):
- `values-real` about 2 minutes, `values-rect` about 6–8 minutes, per change.
- The expansion containment members run weekly.

**Constraints**:
- **`core/` changes:**
  - `r1/interval.rs`: rustdoc only;
  - `s2/rect.rs`: rustdoc and the FR-020 `Deserialize`;
  - `s2/rect_tests.rs`: the FR-020 tests;
  - the CSV and the differential test.

  No public API change and no `Cargo.toml` change.
- **Proof rules (001–004):**
  - no stubbing; unwinding assertions on;
  - every run through `run-verify.sh`, one job at a time, never alongside 004's open runs;
  - booleans compared only with `==`.

**Scale/Scope**:

| Item | Count |
|------|-------|
| Inventory entries | 80: `r1::Interval` 27 (26 in scope), `s2::Rect` 53 (34 in scope, 19 excluded) |
| Catalogue entries | 31: real interval 15, rectangle 16 |
| Kani harnesses | about 45 per change, plus the weekly expansion members |
| Mutants | about 30 `M-4NN`, plus 2 against the descriptions |
| New C++ vector sections | 4 (R9) |
| Documentation additions | the infinite corners of 6 `r1` operations; `# Panics` on 3 `Rect` operations and `r1::project`; 2 deviation notes |
| Upstream records | U-008, U-009, U-010; the K2 deviation |

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle / rule | Assessment | Status |
|------------------|-----------|--------|
| **I. Upstream Parity** | Properties against descriptions written from upstream (R3), checked against new C++ rows (R9). K2 becomes a documented deviation that follows Go. A Hausdorff-distance difference at infinite bounds, if the vectors show one, is documented too. Nothing deviates silently | ✅ Pass |
| **II. Robustness & Memory Safety** | Crash freedom over whole documented domains. K3 and `r1::project` get `# Panics`. FR-020 makes untrusted serde input return an error, consistent with the binary decoder. No `unsafe` | ✅ Pass |
| **III. Test-Backed Changes** | FR-020's tests fail before the fix. New tests go in the sibling `rect_tests.rs`. The descriptions get unit tests and C++ sanity tests | ✅ Pass |
| **IV. Thin, Synchronized Bindings** | No binding change. Python's rectangle inherits the documentation, and exposes no serde | ✅ Pass |
| **V. Measured Performance** | No performance change. The FR-020 check runs only in deserialisation | ✅ Pass |
| **Tech constraints** (toolchain, lints, allocation, dependencies, licensing) | Unchanged toolchain. No new dependency. New files carry SPDX headers. No collections built | ✅ Pass |
| **Workflow: gate, deep verification** | `make test` unchanged and Kani-free. No new decoder, so no new fuzz target | ✅ Pass |
| **Workflow: focused PRs** | The FR-017 to FR-020 changes ship with the feature, each as its own commit, as in 001–004 | ⚠️ Justified exception, see Complexity Tracking |

**Gate result: PASS**, with the exception recorded in Complexity Tracking.

**Post-design re-check (after Phase 1)**: still PASS. `core/` is touched only for FR-017 to
FR-020 and tests. The rectangle's statements reuse 004's description, with no restated code
and no stubs. The real interval's arithmetic domains are narrowed exactly as documented
(FR-004, FR-017).

**Spec criteria at risk:**

- **SC-003, expansion.** The rectangle's expansion containment is weekly. If it times out at 60
  minutes, that clause is "not proven (tool limit)", as in 004.
- **SC-011, infinite corners.** The Hausdorff distance at infinite bounds may differ from C++
  (R1). Such rows are marked and documented, not failures.
- **Interaction with 004.** 004's weekly members and mutant campaign are still open, and both
  features' verification runs share the workstation, one at a time.

## Project Structure

### Documentation (this feature)

```text
specs/005-formal-verify-r1-interval-rect/
├── plan.md              # This file
├── research.md          # Phase 0: decisions R0–R13 (R0 is the 2026-10-12 probe)
├── data-model.md        # Phase 1: descriptions, domains, invariants, catalogue, inventories, changes
├── quickstart.md        # Phase 1: end-to-end validation guide
├── contracts/
│   ├── verify-commands.md        # P=real|rect; CI legs values-real, values-rect
│   └── harness-conventions.md    # ri_*/rl_* modules, catalogue areas, product statements, M-4NN
├── probe/
│   └── r0_harnesses.rs  # R0 Kani probe (20 harnesses); drafts for ri_* and rl_*
├── checklists/
│   └── requirements.md  # from /speckit-specify, amended at planning
└── tasks.md             # Phase 2 (/speckit-tasks)
```

### Source Code (repository root)

```text
verify/
├── src/
│   ├── lib.rs                    # + pub mod real_spec, rect_spec; ri_*, rl_* behind `values`
│   ├── real_spec.rs, real_spec_tests.rs     # NEW
│   ├── rect_spec.rs, rect_spec_tests.rs     # NEW
│   ├── value_domain.rs           # + the R4 generators
│   ├── ri_{crash,val,mem,rel,set,mea,exp}.rs  # NEW
│   └── rl_{crash,val,mem,rel,set,bnd,exp,eq}.rs  # NEW
├── tests/values_spec_vs_cpp.rs   # + the new sections
├── values/{INVENTORY,PROPERTIES,EVALUATION}.md  # + INVENTORY §3–4, PROPERTIES §4–5; spec-005 evaluation part
├── PROPERTIES_BY_KIND.md, TRUSTED_BASE.md, MUTANTS.md, README.md
└── mutants/M-4NN-<slug>.patch    # NEW, about 32

core/src/r1/interval.rs                   # FR-017, FR-019 rustdoc
core/src/s2/rect.rs                       # FR-018, FR-019 rustdoc; FR-020 Deserialize
core/src/s2/rect_tests.rs                 # + FR-020 tests
core/tests/cpp_values_diff.rs             # + R1INTERVAL_*, RECT_* rows
core/tests/data/cpp_values_vectors.csv    # + 4 sections (appended)
tools/refgen-cpp/generate_values.cc       # + 4 sections
tools/refgen-cpp/README.md                # + section rows
UPSTREAM.md                               # U-008, U-009, U-010; the K2 deviation
VERIFIED_TYPES.md                         # the two types under "Types with proofs"
Makefile                                  # VALUES_HARNESSES: P=real|rect
.github/workflows/verify.yml              # + values-real, values-rect; + latlng.rs path
```

**Structure Decision**:

- The proofs join 004's `values` suite, with new `ri_` and `rl_` modules and sections in
  004's records (R2).
- Changes outside `verify/` come in separate commits:
  - the documentation;
  - FR-020 with its tests;
  - the vectors and differential tests;
  - the CI and Makefile wiring;
  - the overview.

## Complexity Tracking

| Exception | Why it is needed | Simpler alternative rejected because |
|-----------|------------------|--------------------------------------|
| The FR-017 to FR-020 changes ship in this feature's PR, against "PRs SHOULD stay focused on one concern" | The spec makes K1–K4 an exception to FR-012. The proofs' domains and statements rest on the documented behaviour, and the rectangle's validity on FR-020 for deserialised values | Separate PRs first would hold the proofs back. Each change stays its own commit with its own tests, as in 001–004 |
