# Implementation Plan: Formal Verification of `ChordAngle` and `Angle`

**Branch**: `004-specs-2/1` | **Date**: 2026-10-16 (R0 run the same day) |
**Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/007-formal-verify-chord-angle-angle/spec.md`

## Summary

The feature proves the chord angle `s1::ChordAngle` and the angle `s1::Angle` with Kani over
their whole documented domains, as the next members of 004's value-type suite:

- **The chord angle**:
  - 35 in-scope entries, 28 with crash proofs. The 7 through `asin` are tool limits (R0).
  - Its properties are stated against `chord_spec`: the canonical set {−1} ∪ [0, 4] ∪ {+∞},
    its order and its neighbours.
  - The maintainer's option B makes the canonical set the only values the type can hold, so
    the order and neighbour laws hold for every value.
  - Every law R0 probed proved in 0.2–15.4 s.
- **The angle**:
  - 30 in-scope entries, 29 with crash proofs; `tan` is a tool limit.
  - Its properties are stated against `angle_spec`: upstream's conversions with ties to even,
    and the exact remainder by 2π.
  - The E-form round trips and upstream's integer identities are **proven by enumeration** of
    every `i32` in release mode (about 70 s), since Kani times out on them.

Changes to `core/`, each its own commit with a test that fails without it:

- **FR-017 to FR-020**: canonical chord angles (option B, R5), including the `Cap` decoder's
  documented deviation; NaN to `STRAIGHT` as upstream; the arithmetic's preconditions with
  upstream's debug checks; a validating `Deserialize`.
- **FR-021**: `normalized` as the exact remainder via `%` (R6).
- **FR-022**: ties to even (R7).
- **FR-023**: documentation of the E-forms' domain and saturation.
- **FR-024**: two C++ vector sections (R9).

The planning probe R0 (29 harnesses, `probe/r0_harnesses.rs`, 2026-10-16) answered the
modelling questions and produced two findings, folded into the spec:

- **Kani's `%` on `f64` is wrong** (`1 % 2` gives 0). `normalized`'s values cannot be proven
  by Kani. They rest on the C++ vectors and a property test against an exact big-integer
  remainder (FR-003, SC-003 amended).
- **The library's `normalized` can return 4.0**, outside (−π, π], for
  −2.462748394469102·10¹⁶. K7 is worse than the survey found; the input becomes a regression
  test.

## Technical Context

**Language/Version**: Rust, edition 2024; Kani's pinned nightly for the proofs; `core/` stays on
MSRV 1.88 (`f64::round_ties_even` is stable since 1.77); the C++ generator builds against the
local s2geometry install, not in CI.

**Primary Dependencies**: Kani 0.68.0, CBMC 6.11.0. **No dependency changes**: the exact
remainder of the tests uses `num-bigint`, already a dependency of `core`.

**Storage**: N/A. Markdown, Rust and CSV rows.

**Testing**:
- `make verify-values P=chord|angle`;
- `cargo test` in `verify/`: `chord_spec_tests`, `angle_spec_tests`, `values_spec_vs_cpp`,
  `property_kinds`, and `angle_enumeration` (release, `--ignored`);
- `make verify-mutants M='M-6*'`;
- `core/src/s1/chord_angle_tests.rs`, `core/src/s1/angle_tests.rs` (new siblings),
  `core/src/s2/encoding_tests.rs`, `core/tests/cpp_values_diff.rs`;
- the gate `make test`, unchanged.

**Target Platform**: Linux x86_64, the workstation and GitHub `ubuntu-latest` (4 vCPUs, 16 GB).

**Project Type**: An extension of the verification harness for a library.

**Performance Goals**: each new CI leg under 10 minutes (SC-005). R0 suggests about 3 minutes
each, the enumeration included.

**Constraints**:
- no public API change and no `Cargo.toml` change;
- no stubbing;
- booleans compared only with `==`;
- no `f64` `%` in any harness or description;
- every run through `run-verify.sh`, one at a time.

**Scale/Scope**:

| Item | Count |
|------|-------|
| Inventory entries | 67: `ChordAngle` 36 (28 with crash proofs, 7 tool limits through `asin`, 1 excluded), `Angle` 31 (29 with crash proofs, 1 tool limit, 1 excluded) |
| Catalogue entries | 22: chord angle 14, angle 8 (data-model §4) |
| Kani harnesses | about 45 per change, none weekly (R12) |
| Enumeration tests | 1 file, 2 catalogue entries, about 70 s |
| Mutants | about 40 `M-6NN` |
| New C++ vector sections | 2 (R9) |
| Upstream records | U-014 (K5, K9, with K8's platform dependence) |

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle / rule | Assessment | Status |
|------------------|-----------|--------|
| **I. Upstream Parity** | Descriptions from upstream (R1, R3), checked against new C++ rows (R9). FR-018, FR-021 and FR-022 restore parity. Option B (FR-017) and the E-forms' saturation (FR-023) are deliberate deviations, documented in the rustdoc with their reasons and marked in the vectors. Nothing deviates silently | ✅ Pass |
| **II. Robustness & Memory Safety** | Crash freedom over documented domains; `# Panics` for the debug checks of the arithmetic and trigonometry; FR-020 makes untrusted serde input return an error; the `Cap` decoder still rejects NaN. No `unsafe` | ✅ Pass |
| **III. Test-Backed Changes** | Each fix's test fails without it; new tests in sibling `_tests.rs` files; the exact-remainder property is a `quickcheck` property | ✅ Pass |
| **IV. Thin, Synchronized Bindings** | No binding change. Python's `ChordAngle` and `Angle` inherit the fixes; their tests are checked against the new constructors | ✅ Pass |
| **V. Measured Performance** | `from_length2` gains one comparison, and `normalized` replaces a division, a rounding and a product with `%`. No hot path, so no benchmark is needed | ✅ Pass |
| **Tech constraints** | No new dependency; SPDX headers on new files; MSRV unchanged | ✅ Pass |
| **Workflow: gate, deep verification** | `make test` unchanged and Kani-free; the enumeration runs outside it, in the values leg; no new decoder | ✅ Pass |
| **Workflow: verification records (v1.2.0)** | Every harness change updates the inventory, catalogue, index (and HTML), `TRUSTED_BASE.md`, `MUTANTS.md` and `VERIFIED_TYPES.md` in the same commit, and 003's records for the `Cap` decoder (FR-026) | ✅ Pass |
| **Workflow: focused PRs** | FR-017 to FR-024 ship with the feature, each its own commit, as in 001–006 | ⚠️ Justified, see Complexity Tracking |

**Gate result: PASS**, with the exception recorded below.

**Post-design re-check (after Phase 1)**: still PASS.

- `core/` is touched only for FR-017 to FR-024, each with its test (data-model §6).
- The statements use `chord_spec` and `angle_spec`, with no restated library code and no
  stubs. The restatements of upstream's arithmetic are kept apart and checked against C++
  (data-model §1).
- The tool limits (`asin`, `tan`, `%`) are recorded with R0's measurements. No property is
  stated through them.

**Spec criteria at risk:**

- **SC-003.** The bar is 19 of the 21 counted entries. Seven are "not probed": P-CAVAL-01,
  P-CACLS-01, P-CAERR-02, P-CAARI-03, P-AGVAL-01, P-AGCNV-01, P-AGARI-01. All are single
  operations or bit-for-bit restatements, of the kind that proved in seconds in 004–006.
- **SC-005.** It is unmeasured on a hosted runner, like 004–006's legs (a push is needed). The
  enumeration's 70 s on the workstation may double there.
- **Option B's reach.** Python tests or other library code might rely on a negative chord
  angle other than −1. Nothing in `core/` builds one directly; the field is private and every
  construction goes through `from_length2` or the constants. The Python suite is checked at
  implementation.

## Project Structure

### Documentation (this feature)

```text
specs/007-formal-verify-chord-angle-angle/
├── plan.md              # This file
├── research.md          # Phase 0: R0–R14 (R0 run 2026-10-16)
├── data-model.md        # Phase 1: descriptions, domains, invariants, catalogue, inventories, changes, mutants
├── quickstart.md        # Phase 1: end-to-end validation guide
├── contracts/
│   ├── verify-commands.md        # P=chord|angle; CI legs values-chord, values-angle; mutants series 6
│   └── harness-conventions.md    # ca_*/ag_* modules, statements, M-6NN
├── probe/               # survey (cpp/, rust/), R0 harnesses, results, counterexamples, README
└── checklists/requirements.md
```

### Source Code (repository root)

```text
verify/src/{chord_spec,chord_spec_tests,angle_spec,angle_spec_tests}.rs     # NEW
verify/src/ca_{crash,val,cls,ord,nbr,err,ari}.rs                            # NEW
verify/src/ag_{crash,val,cnv,eform,ari}.rs                                  # NEW
verify/tests/angle_enumeration.rs                                           # NEW
verify/src/{lib,value_domain,wire_spec}.rs; verify/tests/values_spec_vs_cpp.rs
verify/values/{INVENTORY,PROPERTIES,EVALUATION}.md; verify/decoding/PROPERTIES.md
verify/{PROPERTIES_BY_KIND.md,PROPERTIES_BY_KIND.html,TRUSTED_BASE,MUTANTS}.md
verify/mutants/M-6NN-<slug>.patch
core/src/s1/chord_angle.rs, core/src/s1/chord_angle_tests.rs    # FR-017–FR-020, FR-023
core/src/s1/angle.rs, core/src/s1/angle_tests.rs                # FR-021–FR-023
core/src/s2/encoding.rs, core/src/s2/encoding_tests.rs          # FR-017 (Cap decoder)
core/tests/cpp_values_diff.rs, core/tests/data/cpp_values_vectors.csv
tools/refgen-cpp/generate_values.cc, tools/refgen-cpp/README.md
UPSTREAM.md (U-014), VERIFIED_TYPES.md, Makefile (P=chord|angle, the enumeration)
.github/workflows/verify.yml   # + values-chord, values-angle; mutants series 6
```

**Structure Decision**: the proofs join 004's `values` suite with `ca_` and `ag_` modules
(R2). Changes outside `verify/` are separate commits.

## Complexity Tracking

| Exception | Why it is needed | Simpler alternative rejected because |
|-----------|------------------|--------------------------------------|
| FR-017 to FR-024 ship in this feature's PR | The proofs' domains and statements rest on the fixed and documented behaviour, above all option B's canonical set | Separate PRs first would hold the proofs back; each change stays its own commit, as in 001–006 |
| A second proof method (enumeration) beside Kani | The E-form round trips time out in Kani even over 2²⁴ inputs (R0) | Hundreds of weekly Kani members over slices of the `i32` range, for what enumeration does in about 70 s |
