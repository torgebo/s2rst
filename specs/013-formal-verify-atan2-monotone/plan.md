# Implementation Plan: A Monotone `atan2`, and the Bounding Laws It Opens

**Branch**: `tb/013-frm/1` | **Date**: 2026-11-01 (R0 run the same day) | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/013-formal-verify-atan2-monotone/spec.md`

## Summary

- **An order contract for `atan2`** (`stubs::atan2_order`): the range contract, one value per
  argument pair, the axes exactly, and results ordered by *dominance* within a closed quadrant,
  against earlier calls and six exact anchors (research R1). Tested on ten million pairs per rule
  (K1 found no counterexample in 28 million). With it, `sqrt`'s order (IEEE-754) and a trusted
  lemma, the library's sum of squares is monotone (IEEE-754's monotone rounding; the checker
  times out on it, R2).
- **`Cell`'s bounding-rectangle law P-CREG-02 proven** for the cell's points as directions, over
  every valid cell: level 0 (104 s) and latitude (261 s) per change, longitude per face weekly
  (about 6 minutes each), on the quadrant premise proven by exhaustion (R3). `CellUnion`'s
  P-UREG-05 follows by composition for its cells' points (R8).
- **Recorded, not proven:** the normalised point (an accuracy contract would be needed, R5);
  `Rect`'s P-LREG-03 and so P-ADREG-04 (`sin_cos` must be related to `atan2`, R6); P-PTDST-02
  (still times out with pure `atan2` and `sqrt`, R7).

## Technical Context

**Language/Version**: Rust edition 2024; Kani 0.68.0 (`-Z stubbing`), CBMC 6.11.0; MSRV 1.88.

**Primary Dependencies**: none new.

**Testing**: `make verify-values P=cellrect`; `make verify-slow F=values P=…`; `cargo test` (and
`--release -- --ignored`) in `verify/`; the twin in `core/tests/trait_property_tests.rs`;
`make test`.

**Target Platform**: Linux x86_64 (glibc's `atan2`); GitHub `ubuntu-latest`.

**Project Type**: an extension of the verification harness.

**Performance Goals**: `values-cellrect` under 10 minutes (SC-006): two harnesses, 104 s and
261 s of solver time in R0.

**Constraints**: no library change; every Kani run through `run-verify.sh`, one at a time; the
order contracts' logs fit ten calls (a cell bound and a point take six `atan2` calls).

**Scale/Scope**: 1 law proven (8 harnesses), 2 lemma rows, 4 rows re-stated; 4 contract stubs; 5
mutants `M-988` …; 5 native tests.

## Constitution Check

| Principle / rule | Assessment | Status |
|------------------|-----------|--------|
| **I. Upstream Parity** | No behaviour change; the law is about upstream's own bound, which the C++ rows already check bit for bit | ✅ Pass |
| **II. Robustness & Memory Safety** | No new operation; crash freedom unchanged | ✅ Pass |
| **III. Test-Backed Changes** | Every contract and lemma has a native test; the law has a twin | ✅ Pass |
| **IV. Thin, Synchronized Bindings** | No binding change | ✅ Pass |
| **V. Measured Performance** | No code change | ✅ Pass |
| **Workflow: verification records** | Every harness change updates the records in the same commit | ✅ Pass |
| **Trusted base** | Two new assumptions: the platform's `atan2` order (tested), and IEEE-754's monotone rounding where the checker times out (tested); both in `TRUSTED_BASE.md` | ✅ Pass (recorded) |

**Gate result: PASS.** **Post-design re-check:** still PASS.

**Spec criteria at risk:** SC-003 is met by recording (P-LREG-03, P-ADREG-04 not proven, R6);
SC-006 is unmeasured on a hosted runner (a push is needed).

## Project Structure

```text
specs/013-formal-verify-atan2-monotone/
├── plan.md, research.md (R0–R9), data-model.md, quickstart.md
├── contracts/{verify-commands,harness-conventions}.md
├── probe/   # survey (survey/, mono/), r0_harnesses.rs, README
└── checklists/requirements.md

verify/src/stubs.rs                  # the order contracts, the lemma, the quadrant bound
verify/src/cr_reg.rs                 # NEW: P-CREG-02
verify/src/{lib.rs, region_spec_tests.rs}; verify/Cargo.toml (comment); Makefile (P=cellrect)
verify/tests/{libm_contracts,uv_projection}.rs
verify/values/{PROPERTIES,EVALUATION}.md; verify/cell_union/PROPERTIES.md
verify/{PROPERTIES_BY_KIND.md,.html,TRUSTED_BASE,MUTANTS}.md; verify/mutants/M-988 …
core/tests/trait_property_tests.rs   # the twin
VERIFIED_TYPES.md, CORE_TRAITS.md, .github/workflows/verify.yml
```

**Structure Decision**: one new value-suite part, `P=cellrect`, its own CI leg and weekly leg.

## Complexity Tracking

| Exception | Why | Simpler alternative rejected because |
|-----------|-----|--------------------------------------|
| A relational contract (results ordered across calls) | A bound from corner values holds a point only if `atan2` orders them; no single-call contract says that | The range contract gives a counterexample (spec 012 R0) |
| A trusted lemma (monotone sum of squares) | The checker times out on two symbolic squares (R0); the fact is IEEE-754's | Without it latitude times out per face |
