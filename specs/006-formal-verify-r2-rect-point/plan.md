# Implementation Plan: Formal Verification of `r2::Rect` and `Point`

**Branch**: `004-specs-2/1` | **Date**: 2026-10-13; R0 run and decisions amended 2026-10-14 |
**Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/006-formal-verify-r2-rect-point/spec.md`

## Summary

The feature proves the u-v rectangle `r2::Rect` and the point `s2::Point` with Kani over their
whole documented domains, as the next members of 004's value-type suite:

- **The u-v rectangle**: 32 in-scope entries, stated against `uv_spec`, the product of two of
  005's `real_spec`s (R3, R5).
- **The point**: 24 in-scope entries plus `rotate` (`sin` and `cos` are modelled; `atan2` is
  not, so 5 entries are tool limits, R0), stated against
  `point_spec`: C++'s lexicographic order, the classifications, the power-of-two scaling, and
  the orthogonal directions over the box `[−1, 1]³`, whose laws hold as values (R3, R4).

Changes to `core/`, each its own commit with a test that fails without it:

- **FR-017 to FR-019**: `r2::Rect` documentation, a debug check in `from_center_size`, and a
  validating `Deserialize` (R6, R7).
- **FR-020**: the `ensure_normalizable` defect (K5).
- **FR-021**: one scaling shared with the robust cross product, which restores parity and
  deviates where upstream overflows (K6, U-012) (R8).
- **FR-022**: the point's documentation (K7, K8, K10, K11).
- **FR-023**: four C++ vector sections (R9).

The planning probe R0 (29 harnesses, `probe/r0_harnesses.rs`) ran on 2026-10-14: 23
succeeded, three failed as expected, and three reopened decisions, now amended (research
R0):

- CBMC reports `1 / ∞` as NaN, so `from_coords` and `normalize` range over the triples with
  a finite squared length, and the rest is a tool limit (FR-004);
- the orthogonal directions' laws hold as values, not bit for bit;
- `point_cross`'s sign laws fail for distinct parallel and underflowing points, in Go too
  (K11, U-013), so the documentation and P-PTORT-03 state the true exception.

Planning also amended the spec earlier: the point's crash domain excludes arithmetic that
produces NaN (FR-004), and K10 was added.

## Technical Context

**Language/Version**: Rust, edition 2024; Kani's pinned nightly for the proofs; `core/` stays on
MSRV 1.88; the C++ generator builds against the local s2geometry install, not in CI.

**Primary Dependencies**: Kani 0.68.0, CBMC 6.11.0. **No dependency changes.**

**Storage**: N/A. Markdown, Rust and CSV rows.

**Testing**: `make verify-values P=uvrect|point`; `cargo test` in `verify/` (`uv_spec_tests`,
`point_spec_tests`, `values_spec_vs_cpp`, `property_kinds`); `make verify-mutants` (`M-5NN`);
`core/src/r2/rect_tests.rs`, `core/src/s2/point_tests.rs`, `core/src/s2/edge_crossings_tests.rs`
(new siblings); `core/tests/cpp_values_diff.rs`; the gate `make test`, unchanged.

**Target Platform**: Linux x86_64, the workstation and GitHub `ubuntu-latest` (4 vCPUs, 16 GB).

**Project Type**: An extension of the verification harness for a library.

**Performance Goals**: each new CI leg under 10 minutes (SC-005); members over 6 minutes weekly
(R12).

**Constraints**: no public API change and no `Cargo.toml` change; no stubbing; booleans compared
only with `==`; every run through `run-verify.sh`, one at a time.

**Scale/Scope**:

| Item | Count |
|------|-------|
| Inventory entries | 64: `r2::Rect` 33 (32 in scope), `Point` 31 (25 with crash proofs, 5 tool limits through `atan2`, 1 excluded) |
| Catalogue entries | 32: u-v rectangle 16, point 16, plus the generic order laws (data-model §4) |
| Kani harnesses | about 35 per change, plus the weekly unit-length members (R12) |
| Mutants | about 35 `M-5NN` |
| New C++ vector sections | 4 (R9) |
| Upstream records | U-012 (K6), U-013 (K11); K10 as a documented deviation from Go |

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle / rule | Assessment | Status |
|------------------|-----------|--------|
| **I. Upstream Parity** | Descriptions from upstream (R1, R3), checked against new C++ rows (R9). FR-021 restores parity for the robust cross product and documents the U-012 deviation; K10 is documented against Go, and K11 keeps Go's behaviour with its true exception documented. Nothing deviates silently | ✅ Pass |
| **II. Robustness & Memory Safety** | Crash freedom over documented domains; `# Panics` for `new`, `from_points`, `project`; FR-019 makes untrusted serde input return an error. No `unsafe` | ✅ Pass |
| **III. Test-Backed Changes** | Each fix's test fails without it; new tests in sibling `_tests.rs` files | ✅ Pass |
| **IV. Thin, Synchronized Bindings** | No binding change; Python's point inherits the fixes | ✅ Pass |
| **V. Measured Performance** | The scaling's second multiplication runs only for non-normalizable vectors; no benchmark needed | ✅ Pass |
| **Tech constraints** | No new dependency; SPDX headers on new files | ✅ Pass |
| **Workflow: gate, deep verification** | `make test` unchanged and Kani-free; no new decoder | ✅ Pass |
| **Workflow: focused PRs** | FR-017 to FR-023 ship with the feature, each its own commit, as in 001–005 | ⚠️ Justified, see Complexity Tracking |

**Gate result: PASS**, with the exception recorded below.

**Post-design re-check (after Phase 1)**: still PASS.

- `core/` is touched only for FR-017 to FR-023, each with its test (data-model §6).
- The statements reuse `real_spec`, with no restated library code and no stubs. The point's
  restatements of upstream's arithmetic are kept apart and checked against C++ (data-model
  §1.2).
- The domains are narrowed exactly as FR-004 and FR-022 document: finite triples for the
  arithmetic, and the box for the geometry (R4).

**Spec criteria at risk:**

- **SC-002, trigonometry.** `atan2` is unsupported (R0, by codegen): 5 entries are tool-limit
  exclusions, and P-PTDST-02 is "not proven (tool limit)". SC-002 allows it, with the
  measurement.
- **SC-003, unit length.** P-PTCLS-02 took 644.5 s in R0 and is weekly. P-PTCLS-03 was not
  probed (the pilot took 1802 s); it is weekly, or proven within bounds.
- **SC-005, the point's leg.** About 6 minutes on 4 jobs in R0, set by `ortho`'s 358 s;
  P-PTORT-02 moves weekly if the first run puts it over 6 minutes.
- **Interaction with 004 and 005.** R0 and every later run wait for their queue, one job at a
  time.

## Project Structure

### Documentation (this feature)

```text
specs/006-formal-verify-r2-rect-point/
├── plan.md              # This file
├── research.md          # Phase 0: R0–R14 (R0 run 2026-10-14)
├── data-model.md        # Phase 1: descriptions, domains, invariants, catalogue, inventories, changes, mutants
├── quickstart.md        # Phase 1: end-to-end validation guide
├── contracts/
│   ├── verify-commands.md        # P=uvrect|point; CI legs values-uvrect, values-point
│   └── harness-conventions.md    # ur_*/pt_* modules, laws::ord, statements, M-5NN
├── probe/               # r0_harnesses.rs, r0_results.tsv, r0_counterexamples.txt
└── checklists/requirements.md
```

### Source Code (repository root)

```text
verify/src/{uv_spec,uv_spec_tests,point_spec,point_spec_tests}.rs   # NEW
verify/src/ur_{crash,val,mem,rel,set,mea,exp,eq}.rs                  # NEW
verify/src/pt_{crash,val,cls,scl,ord,ari,ort,dst}.rs                 # NEW
verify/src/{lib,value_domain,laws}.rs; verify/tests/values_spec_vs_cpp.rs
verify/values/{INVENTORY,PROPERTIES,EVALUATION}.md; verify/{PROPERTIES_BY_KIND,TRUSTED_BASE,MUTANTS}.md
verify/mutants/M-5NN-<slug>.patch
core/src/r2/rect.rs, core/src/r2/rect_tests.rs                      # FR-017–FR-019
core/src/s2/point.rs, core/src/s2/point_tests.rs                    # FR-020, FR-022
core/src/s2/edge_crossings.rs, core/src/s2/edge_crossings_tests.rs  # FR-021
core/tests/cpp_values_diff.rs, core/tests/data/cpp_values_vectors.csv
tools/refgen-cpp/generate_values.cc, tools/refgen-cpp/README.md
UPSTREAM.md (U-012, U-013), VERIFIED_TYPES.md, Makefile (P=uvrect|point)
.github/workflows/verify.yml   # + values-uvrect, values-point; + edge_crossings.rs path
```

**Structure Decision**: the proofs join 004's `values` suite with `ur_` and `pt_` modules
(R2); changes outside `verify/` are separate commits.

## Complexity Tracking

| Exception | Why it is needed | Simpler alternative rejected because |
|-----------|------------------|--------------------------------------|
| FR-017 to FR-023 ship in this feature's PR | The proofs' domains and statements rest on the documented and fixed behaviour | Separate PRs first would hold the proofs back; each change stays its own commit, as in 001–005 |
