# Implementation Plan: Formal Verification of `Cell` and `s1::Interval`

**Branch**: `004-specs-2/1` | **Date**: 2026-10-12 (after the R0 probe and the R1 re-check of
upstream) | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/004-formal-verify-cell-interval/spec.md`

## Summary

The feature proves `Cell` and the circular interval `s1::Interval` with Kani over their whole
domains. Both are fixed-size values, so there is no size bound. The one exception is three
properties of `Cell`'s floating-point bounds, which are proven per level up to a measured
level (R8).

- **Crash freedom.** Every in-scope operation (21 of 53 for `Cell`, 30 of 31 for the
  interval) completes without a panic, overflow or a firing debug check, for every valid
  cell, every valid pair of `f64` bounds, every point in [−π, π], and every margin except
  NaN (K5, documented).
- **Meaning.** A catalogue of 40 properties, each against an independent reference
  description:
  - `cell_spec.rs` extends 001's model of the cell hierarchy with upstream's u-v projection;
  - `arc_spec.rs` describes an interval as the set of floats it denotes, and restates
    upstream's length arithmetic.

The planning probe (R0) measured what the spec left open:

- **The interval.**
  - Most of its properties prove whole in 1–254 s each, with every CBMC process under
    1 GB.
  - The FR-020 proofs take 217 s and 0.2 s.
  - `expanded`'s containment clause and half of the Hausdorff distance's range clause
    timed out at 12 minutes, and are split in implementation (R9).
- **`Cell`.**
  - Its integer half proves whole, as in the pilot (10 s and 125 s).
  - Its stored bounds are squares of symbolic floats. Properties on them prove only per
    level, up to levels 5–10 within 12 minutes (R8).
  - An exhaustive enumeration of all 2^30 + 1 boundary values covers the float lemma
    beyond that, and the C++ vectors test the rest.
- **Two new findings:**
  - **D-005**, a port defect: `directed_hausdorff_distance` trips a debug check on valid
    input, because it skips upstream's −π normalisation;
  - **K5**, an undocumented debug panic in `expanded` on a NaN margin (R0, R9).

**The containment proof** (user input to this plan: "Include a proof to contain the
correction to certain values only"). FR-020's seam fix is narrow: upstream's arithmetic,
with a fallback only where it gives a non-positive length for a non-empty interval. Two
entries hold it in place:

- **P-IMEA-01** proves that, for every valid interval except the one-ulp corner, `length` is
  bit for bit upstream's arithmetic, and that at the corner it is one ulp of π.
- **P-IMEA-03** proves that the corner is the only valid, non-empty interval upstream gets
  wrong.

A dedicated mutant that widens the fix must break P-IMEA-01 (research R5). R0 already proved
both against a restated copy of the fix.

**Trait properties are reused, not restated** (R15, after `/speckit-analyze`):

- the binary codec stays 003's generic `Codec` contract;
- the `Region` and equality laws are stated once, in a new `verify/src/laws.rs`, and
  instantiated for `Cell` and the interval;
- the generic quickcheck tests that `CORE_TRAITS.md` names, `core/tests/trait_property_tests.rs`,
  come onto this line from `tb/python-bindings/1`. They found D-006 on their first run here:
  `Rect::cap_bound` returned a cap that misses part of some rectangles that cross the
  antimeridian.

The feature changes `core/` in seven separately committed ways:

- **FR-017 and FR-019 (K1, K3, K5):** `# Panics` documentation (R7).
- **FR-018 (K2):** a hand-written `Deserialize` for `Cell`. It rejects invalid or
  inconsistent input and rebuilds the cell from its identifier, allowing one ulp in the
  stored bounds.
  - R6 found that an honest JSON round trip moves a bound by one ulp in 23.5% of cells,
    which led to a spec amendment.
- **FR-020 (K4):** the narrow seam fix in `length`, documented as a deviation (R5).
- **D-005 (FR-012):** `directed_hausdorff_distance` uses the normalising constructor, as
  upstream does. This fixes a debug-build panic, and release results are unchanged (R9).
- **FR-021:** a new C++ generator, `generate_values.cc`, with differential tests (R10).
- **The generic trait tests** (R15): `core/tests/trait_property_tests.rs`, with `Cell`'s
  missing `Region` instances.
- **D-006** (R15): `Rect::cap_bound` uses upstream's `lng.length() <= π`. A regression test
  covers it. It is already applied in the working tree and passes the core and Python suites.

## Technical Context

**Language/Version**:
- Rust, edition 2024. The proofs build with Kani's pinned nightly (`nightly-2026-08-21`).
- `core/` stays on MSRV 1.88.
- The C++ generator builds against the local s2geometry install and is not part of CI.

**Primary Dependencies**:
- Kani 0.68.0 and CBMC 6.11.0, unchanged.
- **No dependency changes** in `core/`, `python/`, `wasm/` or `verify/`. The FR-018 tests use
  `serde_json`, already a dev-dependency of `core/`.

**Storage**: N/A. The artifacts are Markdown, Rust and one CSV.

**Testing**:
- **Proofs:** `make verify-values [P=cell|interval]` (`run-verify.sh --harness cl_
  --harness iv_`).
- **Models:** `cargo test` in `verify/`: `cell_spec_tests`, `arc_spec_tests`,
  `values_spec_vs_cpp`, `property_kinds` and the exhaustive `uv_projection`, which covers
  all 2^30 + 1 boundary values in about 90 s in a debug build (R8).
- **Non-vacuity:** `make verify-mutants` (`M-3NN`).
- **Changes to `core/`:**
  - the regression tests in the new sibling files `core/src/s2/cell_tests.rs` (FR-018) and
    `core/src/s1/interval_tests.rs` (FR-020 and D-005);
  - the differential tests in `core/tests/cpp_values_diff.rs`;
  - the generic trait tests `core/tests/trait_property_tests.rs` (110 tests) and D-006's
    `core/src/s2/rect_tests.rs`.
- **Bindings:** one pytest in `python/tests/test_interval.py` for the corner's length.
- **Gate:** `make test`, definition unchanged.

**Target Platform**: Linux x86_64, both the workstation and GitHub `ubuntu-latest`. The
reference runner has 4 vCPUs and 16 GB.

**Project Type**: An extension of the verification harness for a library.

**Performance Goals** (R12):
- Two per-change legs, `values-cell` and `values-interval`, each under 10 minutes
  wall-clock on the reference runner (estimated at 4–6 and 7–9 minutes).
- The deeper `__l<L>` members and any split members run weekly (`F=values`, 60 minutes
  each).
- The other legs are unchanged.

**Constraints**:
- **`core/` changes** are limited to:
  - `s2/cell.rs`: the FR-017 rustdoc, the FR-018 `Deserialize`, and the wiring of
    `cell_tests.rs`;
  - `s1/interval.rs`: the FR-019 rustdoc, the FR-020 fix and its rustdoc, D-005's
    two-line fix, and the wiring of `interval_tests.rs`;
  - `s2/rect.rs`: D-006's one-line fix and the wiring of `rect_tests.rs`;
  - the new test files and the CSV.

  No change to `core/Cargo.toml` and no public API change (FR-011).
- **Proof rules** (001–003): no stubbing, unwinding assertions on, partial domains only
  through a stated split. Every Kani run goes through `run-verify.sh`. Verification jobs
  run one after another.
- **`verify/`:** `#![deny(unsafe_code)]` is unchanged, with no new `unsafe`.

**Scale/Scope**:

| Item | Count |
|------|-------|
| Inventory entries | 84: `Cell` 53 (21 in scope: 19 integer and 2 stored-bound readers; 32 excluded), interval 31 (30 in scope; `Display` excluded) |
| In-scope share | `Cell`: all 19 integer entries + the 2 readers (SC-002). Interval: 30 of 31, 97% (SC-002 needs 80%) |
| Catalogue entries | 40 (data-model §4): `Cell` 13, interval 27. Three are instances of the generic laws (§4.3). Two are model checks of the reference descriptions |
| Kani harnesses | about 55 per change (about 40 property harnesses, crash harnesses for the entries without one, the low `__l<L>` members), plus about 15–25 weekly members (R8, R9, R12) |
| Mutants | about 34 `M-3NN`, plus 2 against the reference descriptions |
| New C++ vector sections | 4 (R10) |
| Documentation additions | 4 `# Panics` for `Cell`, 7 for the interval (K3's six and K5's `expanded`), 1 deviation note, 1 `Deserialize` contract |
| Defects found by planning | D-005 (port defect, fixed under FR-012). K5 is an undocumented precondition (documented under FR-019) |
| Upstream records | U-007 (new), U-004 (one sentence), follow-up task to report both (R1) |

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle / rule | Assessment | Status |
|------------------|-----------|--------|
| **I. Upstream Parity** | Every property is stated against a description written from upstream (R3), checked against new C++ vectors (R10), and the library is checked against the same vectors. The one new deviation, FR-020, is documented in the rustdoc and `UPSTREAM.md`, and P-IMEA-01 proves it is confined to one input. Everywhere else `length` stays bit-identical to upstream, which is why the narrow fix was chosen over the stable form everywhere (22% of inverted lengths would change) | ✅ Pass |
| **II. Robustness & Memory Safety** | Crash freedom over whole domains, including every debug check on the path. That is how R0 found D-005, a debug panic on valid input, now fixed with a regression test. K1, K3 and K5 get `# Panics` (FR-017, FR-019). FR-018 closes a way to build an inconsistent `Cell` from untrusted input, and returns an error rather than panicking. No `unsafe` | ✅ Pass |
| **III. Test-Backed Changes** | `CORE_TRAITS.md`'s invariants become quickcheck tests on this line: the generic file is brought in, and `Cell` gets its missing instances (R15). The FR-018, FR-020, D-005 and D-006 regression tests fail before their fixes (seen first; D-006's already was). New unit tests go in sibling `cell_tests.rs` and `interval_tests.rs`. The existing quickcheck property that K4 contradicts holds unchanged after FR-020. Python gets a test for the visible FR-020 change | ✅ Pass |
| **IV. Thin, Synchronized Bindings** | No binding code changes. Python's `S1Interval.length` inherits FR-020 and a pytest covers it. No signature changes, so the stub is unchanged. wasm exposes neither type's affected API | ✅ Pass |
| **V. Measured Performance** | The FR-020 fix adds one branch on a path already taken only for inverted intervals with a non-positive wrapped length: no measurable change, and no performance claim is made. FR-018 runs only during deserialisation. No benchmarks are needed | ✅ Pass |
| **Tech: Toolchain/MSRV** | `core` MSRV 1.88 untouched. The fix uses only `f64` arithmetic | ✅ Pass |
| **Tech: Lints** | No `#[allow]` without a reason. `verify/` keeps its lints | ✅ Pass |
| **Tech: Allocation** | No collection is built by the changes. The FR-018 shadow struct is fixed-size | ✅ Pass |
| **Tech: Dependencies** | None added anywhere | ✅ Pass |
| **Tech: Licensing** | New files carry the two-line SPDX header and the "written for this crate" note. `generate_values.cc` follows `generate_coding.cc` | ✅ Pass |
| **Workflow: gate** | `make test` unchanged and needs no Kani. CI calls `make verify-values` | ✅ Pass |
| **Workflow: deep verification** | No new decoder, so no new fuzz target. The existing `decode_cell` target is unaffected: the binary codec does not use serde | ✅ Pass |
| **Workflow: focused PRs** | The FR-017 to FR-020 changes and D-005's fix ship with the feature, each as its own commit, as in 001–003 | ⚠️ Justified exception, see Complexity Tracking |

**Gate result: PASS**, with the exception recorded in Complexity Tracking.

**Post-design re-check (after Phase 1)**: still PASS.
- **`core/` changes.** The design touches `core/` only for FR-017 to FR-020, D-005's
  fix, and tests.
- **Verification boundaries.** Kani stays out of `make test`. CI uses Makefile targets. No
  stubs and no `unsafe` are added. No published manifest changes.
- **Restated code.** The only restatement is `upstream_length`, three lines, and it is the
  statement of P-IMEA-01 by design (R5). The trusted base names it (R13).
- **Proof domains (FR-004).** Whole domains everywhere, except `Cell`'s three bound
  properties. Those are proven per level up to a measured level and recorded as "proven
  within stated bounds", with exhaustive enumeration of the float lemma beyond it (R8).
  Nothing is narrowed silently.

**Spec criteria at risk** (none is a constitution issue):

- **SC-003, `Cell`'s bounds.** P-CBND-01, P-CBND-02 and P-CCHD-02 are proven only within
  levels 0–L, with L between 5 and 10 by R0 (R8). If "proven within stated bounds" counts as
  proven, as in 002, SC-003 holds. Otherwise 3 of the 40 entries fall short, which is 92%,
  still at least the 90% that SC-003 asks for.
- **SC-003, `expanded`'s containment clause and the Hausdorff range clause.** Both timed
  out whole and are split in implementation (R9). In the worst case, one or both clauses
  become **not proven (tool limit)**, and the entries' other clauses stay proven.
- **SC-005.** The legs are estimated at 4–6 and 7–9 minutes from contended workstation
  times. The CI runner may be slower. The fallback is to move members to the weekly tier,
  never the FR-020 proofs (R12).
- **SC-001, the Hausdorff distance.** Its crash proof holds only after D-005's fix. Until
  then `known-defects.txt` lists it (R9).

## Project Structure

### Documentation (this feature)

```text
specs/004-formal-verify-cell-interval/
├── plan.md              # This file
├── research.md          # Phase 0: decisions R0–R15 (R0 is the 2026-10-12 probe; R15 the trait reuse)
├── data-model.md        # Phase 1: reference descriptions, domains, invariants, catalogue, inventory, changes
├── quickstart.md        # Phase 1: end-to-end validation guide
├── contracts/
│   ├── verify-commands.md        # make verify-values [P=cell|interval]; two CI legs; F=values weekly
│   └── harness-conventions.md    # cl_*/iv_* modules, catalogue areas, generators, __l<L>, M-3NN
├── probe/
│   └── r0_harnesses.rs  # R0 Kani probe, two rounds (32 harnesses); draft for iv_mea, iv_exp, cl_bnd
├── checklists/
│   └── requirements.md  # from /speckit-specify, run 2
└── tasks.md             # Phase 2 (/speckit-tasks)
```

### Source Code (repository root)

```text
verify/
├── Cargo.toml                    # + feature `values` (default)
├── README.md                     # + values section, new target
├── TRUSTED_BASE.md               # + values section (R13)
├── MUTANTS.md                    # + M-3NN rows
├── known-defects.txt             # + D-005 while open
├── PROPERTIES_BY_KIND.md         # + the 40 entries under their kinds
├── values/                       # NEW per-suite records
│   ├── INVENTORY.md              # FR-002, FR-003 (53 + 31 rows)
│   ├── PROPERTIES.md             # FR-006 catalogue (P-CID … P-IEXP)
│   └── EVALUATION.md             # FR-016
├── mutants/M-3NN-<slug>.patch    # NEW, about 36
├── src/
│   ├── lib.rs                    # + pub mod cell_spec, arc_spec; cfg(all(kani, feature = "values")) mods
│   ├── cell_spec.rs              # NEW reference description of a cell (always compiled)
│   ├── cell_spec_tests.rs        # NEW
│   ├── arc_spec.rs               # NEW reference description of an arc (always compiled)
│   ├── arc_spec_tests.rs         # NEW
│   ├── value_domain.rs           # NEW generators
│   ├── laws.rs                   # NEW generic trait laws: laws::region, laws::eq (R15)
│   ├── cl_{crash,id,chd,cnt,bnd,eq}.rs   # NEW Cell harnesses
│   └── iv_{crash,val,mem,rel,set,mea,exp}.rs  # NEW interval harnesses
└── tests/
    ├── values_spec_vs_cpp.rs     # NEW reference descriptions vs cpp_values_vectors.csv
    ├── uv_projection.rs          # NEW exhaustive check of the projection at all 2^30 + 1 boundary values (R8)
    └── property_kinds.rs         # + values/PROPERTIES.md in CATALOGUES

core/src/s2/cell.rs                       # FR-017 rustdoc; FR-018 Deserialize; test wiring
core/src/s2/cell_tests.rs                 # NEW FR-018 regression tests
core/src/s1/interval.rs                   # FR-019 rustdoc; FR-020 fix + deviation note; D-005 fix; test wiring
core/src/s1/interval_tests.rs             # NEW FR-020 and D-005 regression tests
core/src/s2/rect.rs                       # D-006 fix (applied); test wiring
core/src/s2/rect_tests.rs                 # NEW D-006 regression tests (applied)
core/tests/trait_property_tests.rs        # NEW (from tb/python-bindings/1, applied) + Cell's Region instances
core/tests/cpp_values_diff.rs             # NEW differential tests
core/tests/data/cpp_values_vectors.csv    # NEW, generated locally
tools/refgen-cpp/generate_values.cc       # NEW generator
tools/refgen-cpp/CMakeLists.txt           # + add_executable(generate_values)
tools/refgen-cpp/README.md                # + section table
python/tests/test_interval.py             # + the corner's length
UPSTREAM.md                               # U-007; U-004 sentence
VERIFIED_TYPES.md                         # Cell and s1::Interval move to "Types with proofs"; generic laws
CORE_TRAITS.md                            # tracked, updated (R15)
Makefile                                  # + verify-values [P=cell|interval]; verify-slow F=values
.github/workflows/verify.yml              # + matrix legs values-cell, values-interval; slow leg values; + paths
```

**Structure Decision**:
- **The proofs extend `verify/`** behind the `values` feature, with crate-root `cl_*` and
  `iv_*` modules and per-suite records in `verify/values/` (R2). This follows 002 and 003.
- **Changes outside `verify/`** come only in these separate commits:
  - the FR-017 and FR-019 documentation;
  - FR-018 with its tests;
  - FR-020 with its tests, the rustdoc and U-007;
  - D-005's fix with its regression test;
  - the generic trait tests, with `CORE_TRAITS.md`;
  - D-006's fix with its regression test;
  - the vector generator and CSV with the differential tests;
  - the CI and Makefile wiring;
  - the `VERIFIED_TYPES.md` update.

## Complexity Tracking

| Exception | Why it is needed | Simpler alternative rejected because |
|-----------|------------------|--------------------------------------|
| The FR-017 to FR-020 changes, the D-005 and D-006 fixes, and the generic trait tests ship in this feature's PR, against "PRs SHOULD stay focused on one concern" | The spec makes the survey findings an exception to FR-012, and 003 fixed its D-003 the same way. The proofs are stated against the fixed behaviour: P-IMEA-01 and P-IMEA-02 hold only with FR-020, the Hausdorff crash proof only with D-005's fix, and P-CEQ-02's "equal cells have equal fields" needs FR-018 for deserialised cells | Separate PRs first would hold the proofs back, or force them to carry exceptions that the fixes then remove. Each change stays its own commit with its own tests, as in 001–003 |
| One restated upstream function (`upstream_length`) in the reference | P-IMEA-01's statement is "bit for bit upstream's arithmetic", so the arithmetic is the reference | A pointwise length model would need exact reals (R3). Comparing with the library's own pre-fix code would make the proof depend on git history |
