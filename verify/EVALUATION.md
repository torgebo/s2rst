# Pilot Evaluation: Formal Verification of `CellId` (FR-014)

Design and decisions: `specs/001-formal-verify-base-type/`. How to run the proofs:
[`README.md`](README.md).

## Summary

The pilot applied Kani (bounded model checking with CBMC) to `s2rst::s2::CellId`, the
64-bit cell identifier at the base of the S2 port.

- **Crash-freedom:** every one of the 48 in-scope public operations is proven free of
  panics, overflow and undefined behaviour on its documented domain
  ([`INVENTORY.md`](INVENTORY.md)): 45 over the whole domain, the 3 text parsers up to an
  input-size bound. 16 operations are out of scope: 12 floating-point conversions by
  design, 4 because the tool could not finish.
- **Correctness:** 39 of the 43 catalogue properties whose operations are all in scope are
  proven against an independent model of the encoding, and 3 more within the parsers' input
  bound ([`PROPERTIES.md`](PROPERTIES.md)). Every proven property has a mutant that breaks
  it ([`MUTANTS.md`](MUTANTS.md)).
- **Cost:** the per-change run takes 12.5 minutes on a 4-vCPU runner. The neighbour
  operations cross cube faces through `f64` geometry and need 20.6 CPU-hours, so they run
  in a weekly tier.
- **Findings:** one new defect, D-001: `from_face_ij` panics in debug builds outside its
  documented input range, where C++ wraps (follow-up task T056). Two parity defects found in
  review were fixed with C++ differential tests (FR-015, FR-016). The proofs led to 22
  `# Panics` sections, and exposed undefined behaviour in the C++ reference twice (U-001,
  U-002; follow-up tasks T054, T055).

**Recommendation:** continue, with Kani, for integer and byte-level code; stay at this
assurance level; next, the untrusted-input decoders, then `CellUnion` normalisation
("Recommendations" below).

## Effort log

| Date | Phase | Person-hours | Notes |
|------|-------|--------------|-------|
| 2026-09-29 – 2026-10-01 | Specification, clarification, planning (incl. the memory-capped Kani probe) | — | Not separately timed |
| 2026-10-01 | Phase 1 setup (`verify/` crate, `run-verify.sh`, Makefile) | — | |
| 2026-10-01 | Phase 2 foundational (model, generators, inventory) | — | |
| 2026-10-01 | Phase 3: FR-015, FR-016, FR-017 fixes and tests, with C++ vectors | — | Three commits |
| 2026-10-01 – 2026-10-02 | Phase 4–5 (US1, US2): crash proofs, `# Panics` docs, property harnesses for 42 catalogue entries, 38 mutants, budget work | — | Most time went to the neighbour operations and to runtime budgets |
| 2026-10-02 | Phase 6–8 (US3, US4, polish): CI workflow, measurements, documentation | — | |

**Who did the work:** an AI coding agent wrote the harnesses, scripts and documents; the
maintainer made the scoping decisions (15 recorded clarifications, including the slow tier,
and committing over the gate drift). Person-hours were not timed, so the table cannot give person-days per
phase. The elapsed time from specification to a complete pilot was about four days, two of
them implementation. **Machine time** is the larger cost: about 1 CPU-hour per full
per-change run during development, and the neighbour proofs in the slow tier, 20.6
CPU-hours per complete run.

## Exclusions and tool limits (FR-002)

Measured in the 2026-10-01 probe, with each harness alone in a memory-capped scope; full
tables in research R6 (text) and R12 (neighbours).

| Operation | Reason | Measurement |
|-----------|--------|-------------|
| 12 floating-point conversions (`from_point`, `from_lat_lng`, `to_point_raw`, `to_point`, `to_lat_lng`, `center_st`, `center_uv`, `size_st`, `bound_st`, `bound_uv`, `From<&Point>`, `From<&LatLng>`) | `float-geometry` (FR-002 default) | — |
| `to_token` | `tool-limit`: `format!("{:016x}")` | over 5 min (CaDiCaL), over 7 min (Z3) |
| `to_debug_string` | `tool-limit`: `format!` and `String` growth | OOM above 6 GB after 199 s (invalid branch); over 5 min (valid branch, even at a fixed level 5) |
| `impl Display` | `tool-limit`: `core::fmt` | over 5 min at a fixed level 5 |
| `all_neighbors` | `float-geometry: crash-freedom intractable` (FR-002 neighbour clause) | OOM above 4 GB at gaps 0–2 (96–147 s); above 9.5 GB at gap 0 in 105 s, during SSA conversion (1.04 M steps, 29 k VCCs) |

What worked instead: the three text parsers are proven within an input-size bound over a
byte-superset domain (0.2–14 s per length, ≤ 0.5 GB); `edge_neighbors` (411 s, 2.5 GB) and
`vertex_neighbors` (638 s, 1.9 GB) are tractable through the floating-point wrap path.

## Upstream findings (FR-018, SC-012)

| Id | Upstream | Inputs | Observed upstream | Rust | Follow-up |
|----|----------|--------|-------------------|------|-----------|
| U-001 | C++ `S2CellId::level()`, `s2cell_id.h:599–602`: `ABSL_ASSUME(id_ != 0); return kMaxLevel - (countr_zero(id_) >> 1);` | `0`, `1<<62`, `1<<63` | undefined behaviour for `0`; −1 for the other two | `u8` underflow: panics (debug and release) | tasks.md T054 |
| U-002 | C++ `S2CellId::GetCommonAncestorLevel`, `s2cell_id.cc:193–207`: `ABSL_ASSUME(bits != 0)`; Go `CommonAncestorLevel`, `geo/s2/cellid.go:741–747` | (`none`, `none`) | C++: undefined behaviour; Go: `(30, true)` | `0u64.ilog2()` panics | tasks.md T055 |

## `# Panics` documentation added (SC-011)

Commit `8a06089` (`docs(s2): document when CellId operations panic`), documentation only:
22 sections in `core/src/s2/cell_id.rs`.

| Operation(s) | Documented condition | Source |
|--------------|----------------------|--------|
| `from_face_pos_level`, `parent_at_level`, `child_begin_at_level`, `child_end_at_level`, `begin`, `end`, `size_st`, `all_neighbors`, `lsb_for_level`, `size_ij`, `ij_level_to_bound_uv` | `level` converts from a raw integer greater than 30 | inspection of the 13 `impl Into<Level>` functions (with `child_position` and `vertex_neighbors` below) |
| `level` | the id is 0 or has 62 or more trailing zero bits (no level marker); C++ assumes a non-zero id and returns −1 for the others | crash harness assumption; U-001 |
| `child_position` | raw level above 30; in debug builds, an invalid id or `l ∉ 1..=level` | crash harness assumption |
| `parent` | in debug builds, a face cell (the old text said "Panics if `self.is_face()`" without the debug-only qualifier) | crash harness assumption |
| `children`, `child_begin`, `child_end` | in debug builds, an invalid id or a leaf | crash harness assumption |
| `next_wrap`, `prev_wrap` | in debug builds, an invalid id | crash harness assumption |
| `vertex_neighbors` | `level` is 30 or a raw integer above 30; it must be below `self.level()`, as upstream requires | crash harness assumption (upstream `DCHECK_LT`) |
| `common_ancestor_level` | both ids are `none`; C++ is undefined behaviour there and Go returns level 30 | crash harness assumption; U-002 |
| `from_face_ij` | in debug builds, `i` or `j` outside `[0, 2^30)` (the `n * 2` overflow) | the `c_from_face_ij` counterexample |

**Gate drift (not caused by this feature):** `make test` fails on this branch for two
reasons that predate it. Clippy 1.98.1 adds lints that fire 21 times: in 13 files this
feature does not touch, and at two `cell_id.rs` lines unchanged since the root commit `2c2ca67`
(`get_center_si_ti`'s late-initialised `delta`, and a manual midpoint in
`from_face_ij_wrap`). `cargo-deny` reports RUSTSEC-2026-0176/0177 against pyo3 0.24.2
(bans, licenses and sources pass). Every other stage passes: `check`, `fmt-check`, `test-core` (3,575 tests), `doc`, the
Python suite (902 tests) and the WASM suite (274 tests), re-run on 2026-10-02. On `master`
(a throwaway worktree, same toolchain), clippy reports the same 21 errors in the same files,
and the feature leaves `Cargo.lock` unchanged, so the gate fails identically before and
after the feature. The maintainer chose to commit with the drift noted in each commit
message. For the `# Panics` commit itself, `make doc` and `cargo fmt --check` pass, and its
diff is rustdoc lines only. Each documented panic is also recorded in the `panics_doc`
column of [`INVENTORY.md`](INVENTORY.md) §2.

## Defects found

Both known defects were found while the plan was reviewed, before any proof ran (spec
clarifications, session 2026-10-01). Each was fixed in its own commit, with a regression
test that fails without the fix and a differential test against C++ (Constitution I, III):

| Id | Defect | Fix | Regression test | Differential test (C++ vectors) | Would the proofs catch it again? |
|----|--------|-----|-----------------|----------------------------------|----------------------------------|
| FR-015 | `CellId::from_token` accepted a leading `+` (`"+1"` decoded to the valid cell `0/00`), because `u64::from_str_radix` allows a sign; C++ and Go reject it | `cf03cd9` | `from_token_rejects_non_hex_characters` (`core/src/s2/cell_id_tests.rs`) | `from_token_matches_cpp` (`CELLID_FROM_TOKEN`, 1,519 rows) | Yes: mutant M-050 reverses the fix, and `enc::p_enc_06__b2` fails on `"+1"` |
| FR-016 | The debug string (and `Display`) of an invalid id was unpadded hex (`Invalid: 2`); C++ `ToString` pads to 16 digits | `8e71824` | `invalid_debug_string_is_zero_padded` | `to_debug_string_matches_cpp` (`CELLID_TO_STRING`, 1,539 rows) | No: the formatters are a tool limit (R13). The differential test guards it |

The text parsers also gained robustness tests and a fuzz target (FR-017, `8925e6f`), and
`from_debug_string_matches_cpp` (`CELLID_FROM_DEBUG_STRING`, 1,509 rows).

**D-001 (found by the proofs; fix deferred under FR-010, task T056):** `s2::from_face_ij`
panics in debug builds when `i` or `j` is outside `[0, 2^30)`: the top 4-bit chunk sets bits
60–63 of `n`, and `n * 2` overflows. C++ `FromFaceIJ` computes in `uint64_t` and wraps
silently, and Rust release builds wrap the same way. The range is upstream's documented
precondition (`s2coords.h:44–45`), so the crash proof `crash::c_from_face_ij` covers exactly
that range ([`INVENTORY.md`](INVENTORY.md) row 57). The `crash::c_from_face_ij` counterexample
before the domain was narrowed was `i = −2147483641`. Under FR-003, a panic that differs from
upstream is a defect; it is documented by the `# Panics` commit and left to its own change.

## Measurements

Kani 0.68.0, CBMC 6.11.0, CaDiCaL, 4 GiB ceiling per harness. CPU time is the sum of Kani's
per-harness `Verification Time`s. Workstation: Intel i7-8665U laptop CPU (4 cores, 8
threads), 30 GiB.

### The per-change run (`make verify`, SC-005)

Measured with nothing else running, pinned to 2 cores × 2 threads
(`taskset -c 0,1,4,5 make verify`) to match a 4-vCPU / 16 GB runner; `run-verify.sh` picked
4 jobs, as it would there. The time covers the whole target: the model's sanity tests, the
build and every harness.

| Run | Harnesses | Result | Wall clock | CPU time |
|-----|-----------|--------|------------|----------|
| With `crash::c_edge_neighbors__f*` | 202 | 202 SUCCESSFUL, all covers satisfied | 1,638 s (27.3 min) | 55.1 min |
| Final: that family in the weekly slow tier (T046, R15) | 196 | 196 SUCCESSFUL, all covers satisfied | **751 s (12.5 min)** | **33.4 min** |
| Re-run on the final tree (after the playback change) | 196 | 196 SUCCESSFUL, all covers satisfied | 749 s (12.5 min) | — |

The final run by family:

| Family | Harnesses | CPU time | Slowest |
|--------|-----------|----------|---------|
| Crash harnesses (`crash::c_*`) | 43 | 148 s | `c_maximum_tile` 122 s |
| Parser families (`enc::p_enc_06/09/10__b*`) | 86 | 945 s | `p_enc_06__b16` 26 s |
| `val` | 10 | 10 s | `p_val_01` 3 s |
| `hier` | 8 | 35 s | `p_hier_07` 22 s |
| `range` | 6 | 294 s | `p_range_06` 279 s |
| `trav` (with the `p_trav_05__l*` family of 31) | 37 | 241 s | `p_trav_07` 92 s |
| `anc` | 2 | 143 s | `p_anc_01` 140 s |
| `enc::p_enc_01…04` | 4 | 186 s | `p_enc_03` 63 s |

Every harness is under the 5-minute soft limit; `range::p_range_06` (279 s) is the closest.
None hit the 4 GiB ceiling (peak memory per harness was not recorded).

### The weekly slow tier (`make verify-slow`)

One complete run on the workstation, 4 jobs on 4 cores / 8 threads, 30-minute limit per
harness, nothing else active: **91 harnesses, 91 SUCCESSFUL with all covers satisfied, in
5.4 hours (19,528 s) wall clock, 20.6 CPU-hours**.

| Family | Harnesses | CPU time | Slowest |
|--------|-----------|----------|---------|
| `crash::c_edge_neighbors__f*` (C-edge_neighbors) | 6 | 30 min | 394 s |
| `crash::c_vertex_neighbors` (C-vertex_neighbors) | 1 | 14 min | 857 s |
| `nbr::p_nbr_01__f*_k*` (P-NBR-01, one neighbour's conjuncts) | 24 | 224 min | 668 s |
| `nbr::p_nbr_01__f*_p*` (P-NBR-01, one pair's distinctness) | 36 | 399 min | 894 s |
| `nbr::p_nbr_03__f*_q*` (P-NBR-03, face × quadrant) | 24 | 570 min | 1,751 s |

The P-NBR-03 cases took 15–29 minutes each with four jobs competing, close to the
30-minute limit. CI therefore runs the slow tier as a seven-leg matrix (one leg per face,
one for `vertex`) with `VERIFY_JOBS=2`, one CBMC per physical core.

On 2026-10-08 a plain `make verify-slow` on the same workstation sized itself to 8 jobs from
memory (4 cores / 8 threads). 69 of the 91 harnesses passed; 22 of the 24 P-NBR-03 cases
timed out at 30 minutes. Re-run one at a time with a 60-minute limit, all 22 passed with
covers 2/2 in 708–1,103 s each (5.6 hours in all). `make verify-slow` has since defaulted to
one harness at a time with 60 minutes each; CI keeps its own `VERIFY_JOBS` and the
30-minute budget. The first full run that way, on 2026-10-09: **91 harnesses, 91 SUCCESSFUL
with all covers satisfied, in 12.9 hours wall clock (12.8 hours of harness time)**. The
slowest were `crash::c_vertex_neighbors` 470 s, `nbr::p_nbr_01__f0_p23` 545 s and
`nbr::p_nbr_03__f2_q3` 1,082 s, all under half the 60-minute limit.

### Development measurements

Taken while the design settled, often with other runs active (see "Notes from
implementation": these times are upper bounds).

| Group | Harnesses | Result | CPU time | Slowest |
|-------|-----------|--------|----------|---------|
| Integer crash harnesses (`crash::c_*`) | 43 | 43 SUCCESSFUL, all covers satisfied | 77 s | `c_maximum_tile` 65.5 s |
| Parser families (`enc::p_enc_06/09/10__b*`) | 86 | 86 SUCCESSFUL, all covers satisfied | 557 s | `p_enc_06__b16` 14.8 s |
| `crash::c_edge_neighbors__f0` (crash only, face 0) | 1 | SUCCESSFUL | 221 s | — |
| P-NBR-01 validity-and-level conjunct, face 0 (measurement) | 1 | SUCCESSFUL | 248 s | — |
| `nbr::p_nbr_01` (whole domain) | 1 | stopped after more than 20 min | — | — |
| `nbr::p_nbr_01__f0` (all four conjuncts, face 0) | 1 | TIMEOUT at 8 min | — | — |
| `nbr::p_nbr_03__q0…q3` (first form, loops over the `Vec`) | 4 | OOM at the 4 GiB ceiling (~20 s) | — | — |
| `nbr::p_nbr_03__q0` (fixed-index form) | 1 | TIMEOUT at 8 min | — | — |
| `crash::c_vertex_neighbors__q0` (crash only, one quadrant) | 1 | TIMEOUT at 8 min | — | — |
| `crash::c_vertex_neighbors` (unsplit, weekly slow tier, 30-min limit) | 1 | SUCCESSFUL, covers 2/2 | 776 s | — |
| `nbr::p_nbr_02__f*` (P-NBR-02 per face, slow tier) | 6 | out of memory at the 4 GiB ceiling (terse `FAILED, 0 of 422 failed`) | 24–30 s each | — |
| `nbr::p_nbr_02__f0_k0` (per face × neighbour index) | 1 | OOM: "Solver ran out of memory during propositional reduction" | 34 s | — |
| `nbr::p_nbr_03__q0…q3` (P-NBR-03 per quadrant, slow tier, heavy contention) | 4 | TIMEOUT at 30 min | — | — |
| `nbr::p_nbr_03__f0_q0` (per face × quadrant, 3 processes on 4 cores) | 1 | SUCCESSFUL, covers 2/2 | 1,164 s | — |
| `nbr::p_nbr_01__f0…f5` (P-NBR-01 per face, all conjuncts, slow tier, contention) | 6 | `f1` SUCCESSFUL in 1,266 s, `f4` in 1,612 s; `f0`, `f2`, `f3`, `f5` TIMEOUT at 30 min | — | — |
| `nbr::p_nbr_01__f0_k0` (one neighbour's conjuncts) and `__f0_p01` (one pair's distinctness) | 2 | SUCCESSFUL | 514 s and 602 s | — |

**Why the neighbour proofs moved to the slow tier:** `vertex_neighbors` did not fit the
8-minute workstation proxy in any split tried (unsplit 10.6 min and face 0 9.6 min in the
probe, quadrant 0 over 8 min), because every case keeps its floating-point wrap paths. The
maintainer chose a weekly tier over excluding it. The `edge_neighbors` crash family fitted
per face, but cost 22 CPU-minutes and pushed the per-change run to 27 minutes, so it moved
too.

## Limitations

- **Scope (SC-002):** 48 of the 64 public operations (75.0%) are in scope. Twelve
  floating-point conversions are excluded by design (FR-002). Four are tool limits:
  `to_token`, `to_debug_string` and `Display`, whose `core::fmt` code CBMC could not finish
  (over 5 minutes, or out of memory above 6 GB, research R6), and `all_neighbors`, out of
  memory above 4 GB even for crash-freedom (research R12). Their behaviour stays covered by
  the existing unit, property and C++ differential tests.
- **Input-size bounds:** the three text parsers are proven for inputs up to 17 or 33 bytes.
  Longer inputs are rejected by a length check before any loop, by reading the code; tests
  and fuzzing exercise it, but it is not proven.
- **The text domain** is a byte superset of `&str`, built with the one `unsafe` helper in
  `verify/` (`TRUSTED_BASE.md`).
- **Neighbours cost hours:** every neighbour proof crosses a cube-face edge through `f64`
  geometry, which CBMC models bit for bit. The `edge_neighbors` and `vertex_neighbors` crash
  proofs and P-NBR-01 and P-NBR-03 need 20.6 CPU-hours per run, so they run weekly, not
  per change. P-NBR-02 (symmetry) runs out of memory at 4 GiB in every split tried.
- **Kani issues met** (Kani 0.68.0):
  - an unsatisfied `kani::cover!` does not fail a run (Kani exits 0); `run-verify.sh`
    enforces covers itself;
  - terse output gives no counterexample values; `run-verify.sh` re-runs failures with
    concrete playback;
  - a harness killed by the memory ceiling reports `FAILED` with "0 of N failed" in terse
    output; `run-verify.sh` re-runs it alone to tell `OOM` apart;
  - the concrete-playback re-run of a failed neighbour harness ran out of memory in Kani's
    own driver at the 4 GiB ceiling ("memory allocation of 8388352 bytes failed", then a
    Kani panic) and printed no test. It now runs alone under `VERIFY_PLAYBACK_MEM`
    (12 GiB by default here); for mutant M-060 it then printed the failing check
    (`n[0] != n[2]`) and the inputs (face 0, level 0);
  - `--cbmc-args --stop-on-fail` crashes Kani's output parser (research R6);
  - CBMC's SMT2 backend aborted with an internal error on a text harness (`map::at`, research
    R6); CaDiCaL was never beaten;
  - memory blows up on symbolic-length strings and on `core::fmt` (research R6);
  - a symbolic divide or multiply by `4^level` is costly; shifts and per-level splits
    turned 8-minute harnesses into seconds ("Notes from implementation").

## Recommendations

### Continue? Yes, with Kani, for integer-heavy code

Kani proved crash-freedom for all 48 in-scope `CellId` operations, 39 functional properties
against an independent model of the encoding, and 3 more within an input-size bound. The
per-change run takes 12.5 minutes on a 4-vCPU machine; the neighbour proofs run weekly. The work produced:

- one new defect, D-001: `from_face_ij` panics in debug builds outside `[0, 2^30)`, where
  C++ wraps (follow-up task T056);
- 22 `# Panics` sections;
- two cases of undefined behaviour in the C++ reference (U-001, U-002), each now a
  follow-up task;
- two parity fixes with C++ differential tests (FR-015, FR-016), which the proofs and
  mutants now guard;
- a reusable harness: model, generators, memory-guarded runner, mutant campaign and CI.

The cost line is floating-point geometry. Integer code verifies in seconds to minutes;
anything that goes through `f64` face transforms takes tens of minutes to hours, or runs
out of memory. Continue for integer and byte-level code; do not extend to the geometric
types without a different tool.

### Assurance level: stay with Kani for now

Kani is bounded model checking, but with unwinding assertions on, the bounds here are
complete: every proof except the three parsers covers its whole domain. A deductive
verifier (Verus or Creusot; [`research/rust_formalmethods.md`](research/rust_formalmethods.md) §2) would mainly add loop
invariants to remove the parsers' input-size bound, which tests and fuzzing already cover.
A kernel-checked route (Charon + Aeneas to Lean 4, or hax; §1, recommendation 1) would turn
the Hilbert-curve model into a foundational specification. That is the right step only if
the project wants a published-grade guarantee for the cell encoding, and it costs much more
manual proof. Not recommended as the next step.

Rewriting `to_token` and `to_debug_string` with a fixed-size byte buffer instead of
`core::fmt` would bring three operations and five properties (P-ENC-05, -07, -08, -11, -12)
within Kani's reach (research R6). It is a small, behaviour-preserving change, but it bends
the code to the tool, and the C++ differential tests already guard these functions. Low
priority.

### Which type next (ranked)

1. **The untrusted-input decoders** (Constitution II): their panics are a security
   concern, and the byte-superset domain with input-size bounds worked well for the text
   parsers here. Kani would complement the 19 nightly fuzz targets with bounded proofs.
2. **`CellUnion` normalisation:** integer logic built directly on the proven `CellId`
   operations (`contains`, `parent`, `range_min`/`range_max`), with properties that are easy
   to state (sorted, disjoint, same coverage) for small bounded unions.
3. **`r1::Interval`:** a small value type with mostly comparisons and additions; tractable
   if properties avoid division-heavy paths.
4. **`s1::ChordAngle`:** `f64` throughout, with square roots and trigonometry; expect the
   same cost wall as the neighbour operations.

## Notes from implementation

- **Planning-doc correction:** the spec, research R13 and the tasks used `5` as an example
  of an invalid id. `5 = 0b101` is a valid leaf (`0/0…02`): its lowest set bit is the leaf
  marker. The reference model's sanity test caught it; the examples now use `2`.
- **`VERIFY_JOBS` default corrected:** the planned `⌊(MemTotal − 2 GiB) / 4 GiB⌋` gave 3
  jobs on a "16 GB" runner (not the intended 4) and allowed a 28 GiB worst case on a 30 GiB
  workstation. `run-verify.sh` uses `⌊MemAvailable / (7/8 of the ceiling)⌋`, 3.5 GiB per
  job at the default 4 GiB, and caps its cgroup backstop below `MemAvailable` (research
  R15).
- **Kani 0.68.0 terse output:** a harness killed by the memory ceiling prints `FAILED` with
  `0 of N failed`, without the out-of-memory message, so `run-verify.sh` re-runs such a
  harness alone in regular output to classify it as `OOM` or `ERROR`.
- **Cheaper reference model:** the model divided and multiplied by `pow4(k)`, which CBMC
  encodes as a full 64-bit divider or multiplier when `k` is symbolic. `ancestor`,
  `leaf_index`, `position` and `from_position` now use the equivalent shifts and masks
  (`pow4(k) = 1 << 2k`); the model still passes its 9 sanity tests against the C++ vectors.
  `anc::p_anc_01` went from over 8 minutes to 213 s.
- **`range::p_range_06` is compositional:** it checks `maximum_tile`'s result through
  `is_valid`, `range_min`, `range_max` and `parent`, which P-VAL-03, P-RANGE-01 and
  P-HIER-03 tie to the model, instead of through the model's 30-level `decompose` search
  (over 8 minutes). It took 442 s with moderate contention and 279 s in the clean run. A
  split by level brought each member to 13–36 s but nearly doubled the total CPU time, so it
  stays unsplit.
- **`trav::p_trav_05` is split by level** (`p_trav_05__l0…l30`): with the level concrete,
  the `rem_euclid` by `6·4^level` and the implementation's wrap modulus are constants. The
  unsplit harness took 465 s; each level takes 2–4 s.
- **Contention skews times:** the 2026-10-01 runs overlapped (per-change run, slow tier,
  mutant campaign and measurements; up to seven CBMC processes on a 4-core/8-thread
  laptop CPU). Times from those runs are upper bounds. The SC-005 figures come from a
  separate run with nothing else active.

## Acceptance (SC-001 … SC-012)

Checked 2026-10-02 against the final runs.

| Criterion | Result | Evidence |
|-----------|--------|----------|
| SC-001 crash-freedom | **Met.** 48/48 in-scope operations proven; 45 (94%) over the whole documented domain, the 3 parsers within an input-size bound with the gap documented | `INVENTORY.md` §1; `TRUSTED_BASE.md` "Input-size bounds" |
| SC-002 scope ≥ 75% | **Met, at the threshold.** 48/64 = 75.0%; every excluded row has a reason | `INVENTORY.md` §1; "Exclusions and tool limits" |
| SC-003 catalogue | **Met, with no slack.** Every in-scope operation has a `P-*`; every FR-005 area plus `NBR` is covered; 39/43 = 90.7% of the in-scope entries are `proven` (the other 4: the 3 parser properties, proven within bounds, and P-NBR-02, a tool limit) | `PROPERTIES.md` |
| SC-004 non-vacuity | **Met.** 38 mutants, 38 killed (`make verify-mutants`, exit 0); every proven or within-bounds property has at least one; every crash harness's covers are satisfied in both tiers | `MUTANTS.md`; "Measurements" |
| SC-005 run time | **Met on the reference machine; CI not yet measured.** Per-change run 12.5 min on a 4-vCPU emulation of the reference runner (33 CPU-min). The slow tier needs 20.6 CPU-hours; split into seven CI legs with 2 jobs each, the largest leg is about 3.5 CPU-hours, an estimated 2 hours against the 6-hour job limit. Neither has run on a hosted runner (T046's live check needs a push) | "Measurements" |
| SC-006 no API or dependency change, gate unchanged | **Met, except that the gate was already red.** No public signature or manifest changed; `make test` does not invoke Kani. The gate fails identically on `master` and on this branch (clippy 1.98.1 lints, two pyo3 advisories); every other stage passes | "`# Panics` documentation added", gate drift note |
| SC-007 self-contained evaluation | **Met** (T048 read-through): "Summary" and "Recommendations" answer continue/approach/next type | This document |
| SC-008 token parity | **Met.** `from_token("+1")` is invalid; the regression test fails without the fix (re-checked 2026-10-02); 1,519 C++ token vectors match | "Defects found" |
| SC-009 debug-string parity | **Met.** `Invalid: ` plus 16 digits; the regression test fails without the fix (re-checked); 1,539 C++ vectors match | "Defects found" |
| SC-010 untrusted-input coverage | **Met.** Robustness tests pass; `parse_cell_id_text` is in the nightly fuzz matrix (60 s smoke run: 11.9 M executions, no crash); the plan records the return-type exception | `core/tests/decoder_robustness.rs`; plan "Complexity Tracking" |
| SC-011 `# Panics` documented | **Met.** 22 sections in one documentation-only commit (`8a06089`) | "`# Panics` documentation added" |
| SC-012 upstream findings | **Met.** U-001 and U-002 each have a follow-up task (T054, T055), created before the affected proofs were complete, and an entry here | "Upstream findings" |
