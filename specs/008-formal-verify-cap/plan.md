# Implementation Plan: Formal Verification of `Cap`, with the Trait Laws It Carries

**Branch**: `008-bounded-asin-atan2/1` | **Date**: 2026-10-24 (R0, R0b, R0c run the same day) |
**Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/008-formal-verify-cap/spec.md`

## Summary

The feature proves `s2::Cap` with Kani as the ninth member of 004's value-type suite, and
instantiates the project's generic trait laws for it:

- **43 inventory entries**: 31 crash proofs, 8 under the function contracts the maintainer
  chose (option A: `asin`, `atan2`, and, from R0, `cos`), the 2 codec entries by 003, and
  `Display` excluded.
- **26 catalogue entries** against `cap_spec` (a cap as the points within a chord distance of
  its centre), **7 lemmas** on the real code, and **2 compositions** in other types'
  catalogues: `Cell::cap_bound` contains the cell's vertices, and 006's chord-angle symmetry,
  a 60-minute timeout until now.
- **The trait laws**: 004's equality laws, the `Region` laws (one existing, two added to
  `laws.rs` from `CORE_TRAITS.md`; the covering law attempted weekly), and 003's codec law
  under `Cap`'s own equality.

The planning probe (research R0) showed that most of `Cap`'s laws, as first stated, time out:
each recomputes the float products the operation already computed (`add_point` stores
`d(c, p)` and `contains_point` computes it again). The plan's main technique is the **lemma
stub** (spec FR-003a, research R6): the recomputed library function is replaced by a pure
function returning any value its lemmas allow, each lemma proven on the real code. With it,
R0c proved every law that had timed out in 2–53 s.

Changes to `core/`, each its own commit with a test that fails without it:

- **FR-017** (K1): the covering level through `MIN_WIDTH.max_level`.
- **FR-018** (K2, K3): upstream's cell tests (vertices, then edges) for `Region for Cap`.
- **FR-019** (K4): `complement`'s radius `FromLength2(4 − r²)`.
- **FR-020** (K5): an exact longitude reduction in `rect_bound`, without `%`.
- **FR-022** (K6): `approx_eq_with`'s tolerance for the centres.
- **FR-023** (K7): a validating `Deserialize`.
- **FR-024**: four C++ vector sections, from the existing s2geometry build.

## Technical Context

**Language/Version**: Rust, edition 2024; Kani's pinned nightly for the proofs; `core/` stays
on MSRV 1.88. The C++ generator builds against the existing s2geometry release build, not in
CI.

**Primary Dependencies**: Kani 0.68.0 (with `-Z stubbing`), CBMC 6.11.0. **No dependency
changes**: the exact remainder reuses 007's `num-bigint` test helper.

**Storage**: N/A. Markdown, Rust and CSV rows.

**Testing**:
- `make verify-values P=cap`; `make verify-slow F=values P=…` (the `values-cap` leg);
- `cargo test` in `verify/`: `cap_spec_tests`, `values_spec_vs_cpp`, `libm_contracts`,
  `property_kinds`;
- `make verify-mutants M='M-7*'`;
- `core/src/s2/cap_tests.rs` (new sibling), `core/tests/cpp_cap_diff.rs` (new),
  `core/tests/trait_property_tests.rs`;
- the gate `make test`, unchanged.

**Target Platform**: Linux x86_64, the workstation and GitHub `ubuntu-latest` (4 vCPUs, 16 GB).

**Project Type**: An extension of the verification harness for a library.

**Performance Goals**: the `values-cap` CI leg under 10 minutes (SC-006); R0c's laws took under
3 minutes at 3 jobs.

**Constraints**:
- no public API change and no `Cargo.toml` change;
- stubs only as spec FR-003 and FR-003a allow, each named in its harness, the status following
  them (contracts/harness-conventions.md);
- stub tables in `static Mutex`, fixed capacity, element-wise keys (no `thread_local!`: it
  crashes Kani 0.68.0's compiler);
- booleans compared only with `==`; no `f64 %` anywhere in the proofs;
- every run through `run-verify.sh`, one at a time, memory-capped.

**Scale/Scope**:

| Item | Count |
|------|-------|
| Inventory entries | 43: 31 crash proofs, 8 under contract, 2 by 003, 1 excluded |
| Catalogue entries | 26 `Cap` (25 counted for SC-003), 7 lemmas, 2 in other catalogues |
| Kani harnesses | about 40 per change, about 8 weekly (research R10) |
| Native contract tests | 1 file, 3 contracts × (specials + 10⁶ random) |
| Mutants | about 40 `M-7NN` |
| New C++ vector sections | 4 (research R9) |
| Upstream records | none so far (research R1) |

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle / rule | Assessment | Status |
|------------------|-----------|--------|
| **I. Upstream Parity** | The description and restatements come from upstream (R1, R3), and new C++ rows from a running build check them (R9). FR-017 to FR-020 and FR-022 restore parity, observed against C++ in the survey; FR-018 changes the coverer's results for caps to C++'s. `union`'s error margin stays a documented deviation. Nothing deviates silently | ✅ Pass |
| **II. Robustness & Memory Safety** | Crash freedom over the documented domains; the preconditions of `contains_point`, `expanded` and the arithmetic documented; FR-023 turns invalid serde input into an error. No `unsafe` (the stub tables use `Mutex`, not `static mut`) | ✅ Pass |
| **III. Test-Backed Changes** | Each fix's test fails without it, in a sibling `cap_tests.rs`; the reduction's exactness is a property test against the exact remainder; the contracts have native tests | ✅ Pass |
| **IV. Thin, Synchronized Bindings** | No binding change. Python's and wasm's caps inherit the fixes; their tests pin no exact coverings (checked) | ✅ Pass |
| **V. Measured Performance** | FR-018 adds four vertex computations per cell test, as upstream; the coverer calls them per candidate cell. The existing `core/benches/region_coverer.rs` and `core/benches/cap.rs` (gungraun) are run before and after, and the change records both | ✅ Pass (measured in the change) |
| **Tech constraints** | No new dependency; SPDX headers on new files; MSRV unchanged | ✅ Pass |
| **Workflow: gate, deep verification** | `make test` unchanged and Kani-free; no new decoder or fuzz target | ✅ Pass |
| **Workflow: verification records (v1.2.0)** | Every harness change updates the inventory, catalogue, index (and HTML), `TRUSTED_BASE.md`, `MUTANTS.md`, `VERIFIED_TYPES.md` and `CORE_TRAITS.md` in the same commit, and the two other catalogues (R12) | ✅ Pass |
| **Workflow: focused PRs** | FR-017 to FR-024 ship with the feature, each its own commit, as in 001–007 | ⚠️ Justified, see Complexity Tracking |

**Gate result: PASS**, with the exceptions recorded below.

**Post-design re-check (after Phase 1)**: still PASS.

- `core/` is touched only for FR-017 to FR-024 and documentation, each with its test
  (data-model §6).
- The statements use `cap_spec`; restatements are kept apart and checked against C++.
- Every stub is a function contract (tested against the real function) or a lemma stub
  (lemmas proven on the real code); harnesses name them and their status follows.

**Spec criteria at risk:**

- **SC-003** (90% of 25): the bar is 23. At risk are P-CPBND-03 (`cell_union_bound`'s length,
  an R0 timeout), P-CPREG-04 (the covering law, not probed) and P-CPREL-03 (rests on L-CP-06,
  not probed). If all three fail, 22 of 25 (88%) miss the bar; L-CP-06 is measured first
  (research R13).
- **SC-004**: the covering law was moved out of the count at planning (R0); the rest proved in
  R0c.
- **SC-006**: unmeasured on a hosted runner, as 004–007's legs (a push is needed).
- **Lemma stubs (FR-003a)** were introduced at planning, beyond the maintainer's option A,
  which covered only `asin` and `atan2`. They are sound by construction (research R6), but the
  maintainer may prefer weekly timeouts instead. The spec records the amendment.

## Project Structure

### Documentation (this feature)

```text
specs/008-formal-verify-cap/
├── plan.md              # This file
├── research.md          # Phase 0: R0–R15 (R0 in three rounds, 2026-10-24)
├── data-model.md        # Phase 1: description, domains, stubs, catalogue, inventory, changes, mutants
├── quickstart.md        # Phase 1: end-to-end validation guide
├── contracts/
│   ├── verify-commands.md        # -Z stubbing; P=cap; CI legs values-cap; mutants series 7
│   └── harness-conventions.md    # cp_* modules, stubs, statuses, M-7NN
├── probe/               # survey (cpp/, rust/), R0/R0b/R0c harnesses and results, README
└── checklists/requirements.md
```

### Source Code (repository root)

```text
verify/src/{cap_spec,cap_spec_tests,stubs}.rs                                # NEW
verify/src/cp_{crash,con,cls,mea,mem,rel,gro,cmp,bnd,eq,reg,lem}.rs          # NEW
verify/tests/libm_contracts.rs                                               # NEW
verify/src/{lib,value_domain,laws}.rs; verify/tests/values_spec_vs_cpp.rs
verify/src/cl_*.rs (P-CCAP-01), verify/src/pt_dst.rs (P-PTDST-01)
verify/run-verify.sh (-Z stubbing), verify/known-timeouts.txt
verify/values/{INVENTORY,PROPERTIES,EVALUATION}.md; verify/decoding/PROPERTIES.md
verify/{PROPERTIES_BY_KIND.md,PROPERTIES_BY_KIND.html,TRUSTED_BASE,MUTANTS}.md
verify/mutants/M-7NN-<slug>.patch
core/src/s2/cap.rs, core/src/s2/cap_tests.rs     # FR-017–FR-020, FR-022, FR-023
core/src/s2/region.rs                            # FR-018
core/src/s1/angle_tests.rs → shared exact-remainder helper (FR-020's test)
core/tests/cpp_cap_diff.rs, core/tests/data/cpp_cap_vectors.csv   # NEW
core/tests/trait_property_tests.rs               # FR-011
tools/refgen-cpp/generate_cap.cc (NEW), CMakeLists.txt, README.md
VERIFIED_TYPES.md, CORE_TRAITS.md, Makefile (P=cap)
.github/workflows/verify.yml   # + values-cap; slow values-cap; mutants series 7
```

**Structure Decision**: the proofs join 004's `values` suite with `cp_` modules (research R2),
and the stubs live in one module, `stubs.rs`, so every replacement is in one reviewed place.
Changes outside `verify/` are separate commits.

## Complexity Tracking

| Exception | Why it is needed | Simpler alternative rejected because |
|-----------|------------------|--------------------------------------|
| FR-017 to FR-024 ship in this feature's PR | The proofs' statements rest on the fixed behaviour (the level, the complement, the reduction, the cell tests' shape) | Separate PRs first would hold the proofs back; each change stays its own commit, as in 001–007 |
| Function contracts and lemma stubs: the first proofs that do not run every library function | Without them, 8 entries are unreachable (`asin`, `atan2`) and most laws time out (R0) | Weekly 60-minute runs: R0's 12-minute timeouts and 006's 60-minute timeout on the same pattern say they would not finish; tool-limit rows would leave `Cap` mostly unproven |
| `-Z stubbing` on every `verify` build | Kani requires the flag for a crate with stub attributes | A second crate for the `Cap` proofs would duplicate the suite's domains, laws and runner |
