---

description: "Task list for the formal verification of Cap and its trait laws (spec 008)"
---

# Tasks: Formal Verification of `Cap`, with the Trait Laws It Carries

**Input**: Design documents from `specs/008-formal-verify-cap/`:

- plan.md, spec.md (clarified 2026-10-24: option A; amended at planning with R0–R0c: the `cos`
  contract, FR-003a's lemma stubs, SC-004);
- research.md (R0–R15; R0 ran in three rounds on 2026-10-24);
- data-model.md; contracts/ (verify-commands, harness-conventions); quickstart.md;
- the probes in `probe/`:
  - the survey (`cpp/survey.cc` and `rust/src/main.rs`, with their outputs: K1–K6 on running
    C++ and the library);
  - the Kani probes `r0_harnesses.rs`, `r0b_harnesses.rs`, `r0c_harnesses.rs`, with
    `r0_results.tsv` and `r0bc_results.tsv`.

  **R0c's harnesses are the first drafts of most tasks below**, and its `Table<K>`, stubs and
  abstract cell are the model for `verify/src/stubs.rs`. Exceptions: R0's and R0b's harnesses
  whose failures were the checker's (`log2`, `cos`, `f64 %`), and R0b's `thread_local!` and
  `Vec` tables, which must not be copied.

The feature extends the `values` suite of 004–007 in `verify/`. 007's task list
(`specs/007-formal-verify-chord-angle-angle/tasks.md`) shows the patterns; its files are the
style reference:

- `verify/src/chord_spec.rs`, `ca_*.rs`; `verify/src/value_domain.rs`; `verify/src/laws.rs`;
- `verify/tests/values_spec_vs_cpp.rs`;
- `tools/refgen-cpp/generate_values.cc` and `core/tests/cpp_values_diff.rs` (for the new
  generator and diff test);
- the hand-written `Deserialize` of `Cell` (`core/src/s2/cell.rs:810–850`).

**Prerequisites**:

- **Kani 0.68.0**: `make verify-kani-check` passes.
- **For T006 only**: the C++ reference build in `/home/t/sources/s2geometry/build`
  (`v0.14.0-40-g37a4501`), installed with `cmake --install /home/t/sources/s2geometry/build
  --prefix <prefix>`; the generator runs with `LD_LIBRARY_PATH=<prefix>/lib`
  (probe/README.md).
- **Disk**: `df -h /` shows at least 20 GB free before any Kani run (67 GB on 2026-10-24;
  `verify/target` is 59 GB; memory "disk space and the Kani cache").

**Tests**: The spec asks for these ordinary tests, so they appear as tasks:

- the regression tests of FR-017 to FR-020, FR-022 and FR-023, each failing without its change
  (Constitution III, SC-007);
- the differential tests against the new C++ vectors (FR-024);
- the description's unit tests and its sanity tests against the vectors (FR-007);
- the native tests of the three function contracts (FR-003);
- the quickcheck generator and K5's example (FR-011).

The Kani harnesses are the verification deliverable, so they are implementation tasks.

**Organization**: Tasks are grouped by user story (spec.md):

| Story | Priority | Phase |
|-------|----------|-------|
| US4: parity restored (the library changes) | P2 | 3 (first: the proofs rest on it) |
| US1: proven crash freedom | P1 | 4 |
| US2: the generic trait laws for `Cap` | P1 | 5 |
| US3: proven meaning against `cap_spec` | P2 | 6 |
| US5: CI, records and evaluation | P3 | 7 |

The library changes come first, in Phase 3, because the statements rest on the fixed
behaviour (plan, Complexity Tracking): the level (P-CPBND-02), the complement (P-CPCMP-01),
the reduction (P-CPBND-05) and the cell tests' shape (P-CPREG-01, -02).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on unfinished tasks). For Kani
  **runs**, "parallel" applies only to writing the code; the runs go one after another.
- **[Story]**: the user story the task belongs to (US1–US5).
- Paths are relative to the repository root.

## Memory safety (read first)

- **Every** Kani run goes through a Makefile target or `verify/run-verify.sh`, which apply the
  memory guard: `make verify-values P=cap`, `make verify-harness H=…`,
  `make verify-slow F=values P=…`, `make verify-mutants M=…`.
  For a throwaway measurement, use R0's scratch crate (probe/README.md, "Scratch crate
  layout"; lost on reboot) with a copy of `run-verify.sh`, `VERIFY_HARNESS_TIMEOUT=12m
  VERIFY_MEM_PER_HARNESS=4G`, and `-Z stubbing`.
- **No run is queued** (2026-10-24). Check `pgrep -fa '[c]bmc'` before starting one. `verify/`
  must **compile at every save** while a run is going (codegen at the start, playback at the
  end).
- **Kani runs go one at a time.** Writing code, ordinary tests and `cargo kani --only-codegen
  -Z stubbing` can go on meanwhile.

## Upstream findings rule (FR-025, read first)

If a harness, a C++ vector or reading upstream shows **undefined or wrong behaviour in
upstream**, before marking the affected proof complete:

1. Append a task to Phase 8 with the next free task ID and the next free **U-015** (U-001 to
   U-014 are taken), with the upstream `file:line`, the inputs, the behaviour, and what the
   library does.
2. Add it to the spec-008 part of `verify/values/EVALUATION.md` and to `UPSTREAM.md`.
3. Cite it in the affected harness's doc comment.

Research R1 met none. Never change upstream code.

## Known-defect rule (FR-014, read first)

- A proof that fails on a **real library defect** gets the next free **D-009** (D-001 to
  D-008 are taken). K1–K7 are the survey's findings, handled by FR-017 to FR-023, and need no
  `D-NNN`.
- Record it under "Defects found" in the spec-008 part of `verify/values/EVALUATION.md`; list
  its harness in `verify/known-defects.txt` if it is written before the fix.
- The fix is a separate, tested change. **Ask the user before fixing** a defect not in the
  spec's survey or this plan.
- **A failure is replayed natively before it is believed** (research R0: three of R0's
  failures were CBMC's `log2`, `cos` and `%`). With stubs, Kani's playback warns that the
  stubs are not applied; replay the cap and point inputs on the real code, and for a stubbed
  law first look for a fact the stub lacks (memory "Kani lemma stubs").

## Facts these tasks rely on

1. **R0–R0c's results** (`probe/r0_results.tsv`, `probe/r0bc_results.tsv`; 4 GiB, 12 min):
   - **proven on the real code**: the equality laws 1.9 s; `contains` reflexive 13.5 s;
     FR-017's level 13.8 s; FR-020's reduction 2.9 s; `expanded` contains 336.9 s;
     L-CP-01 1.7 s, L-CP-02 0.9 s, L-CP-03 54.6 s, L-CP-04 94.1 s, L-CP-05 626.1 s, L-CP-07
     0.0 s; the edge test's crash over box values 385.0 s;
   - **proven with lemma stubs** (R0c): `add_point` 6.8 s, `add_cap` 17.0 s, complement valid
     4.7 s, `intersects` symmetric 27.0 s, the two cell laws 53.0 s and 51.1 s (edge test
     abstracted), 696.9 s (real edge test); R0b: `contains ⇒ intersects` 20.3 s,
     `from_center_angle` crash 1.7 s, the codec law 8.8 s;
   - **proven under contracts**: `rect_bound` (FR-020's draft) valid, 84.6 s (asin, atan2, cos);
   - **failed as expected**: the library's complement (K4) 3.3 s;
   - **timeouts**: `cell_union_bound`'s length, `union`'s crash (also with
     `interpolate_at_distance` stubbed), `lemma_norm2_neg`;
   - **checker artefacts** (replayed): CBMC's `log2` (r = 0.4866111182066655), `cos` (0 at
     1.1·10⁻³⁰⁸ and −0), Kani's `f64 %` in the library's `rem_euclid`.
2. **Stubbing** (research R0b, R6): `#[kani::stub]` resolves `f64::asin`, `f64::atan2`,
   `f64::cos`, `s2rst::s2::Point::chord_angle`, `s2rst::s2::Point::is_unit`,
   `s2rst::s2::Cell::vertex`, `s2rst::s2::Cell::edge_raw`, `s2rst::s2::Cell::contains_point`,
   `<s2rst::s1::ChordAngle as std::ops::Add>::add`, and private
   `s2rst::s2::cap::interpolate_at_distance`. Every `cargo kani` call of the crate then needs
   `-Z stubbing`. **`thread_local!` + `RefCell` in a stub crashes Kani 0.68.0's compiler**
   (`intrinsics.rs:243 … IntTy::I32`); a `Vec` table runs the cell laws out of memory. Use
   `static Mutex<Table<K>>` with fixed slots, element-wise key comparisons, no `==` or `<` on
   arrays or tuples in a stub.
3. **`Cap`** (`core/src/s2/cap.rs`, unchanged since the initial commit except 007's decoder
   work in `encoding.rs`):
   - `pub struct Cap { center: Point, radius: ChordAngle }`, private fields; derived `Clone,
     Copy, Debug` and serde pair (lines 84–89);
   - constructors 97–165 (`from_center_angle` debug-asserts `is_valid`, line 114); accessors
     171–200; `is_valid` 206 (`is_unit && length2 <= 4`); predicates 212–261;
   - `complement` 270–278 (`ChordAngle::STRAIGHT − self.radius`: K4);
   - `expanded` 282–288 (debug-asserts a non-negative distance);
   - `union` 291–322 (through `angle_radius`, `Vector::angle`, `interpolate_at_distance`,
     `from_center_angle`, plus an error margin);
   - `add_point` 326–336; `add_cap` 339–355 (margin `(2ε + 2.02ε) · d²`);
   - `rect_bound` 366–403 (`(x + π).rem_euclid(2π) − π`, lines 395–396: K5);
   - `cell_union_bound` 406–427 (`(d / r).log2().floor()`, line 418: K1);
   - `centroid` 434–440; `equal` 446–450; `approx_eq` 454, `approx_eq_with` 460–468 (centres
     at a fixed 10⁻¹⁵ through `Point::approx_eq`: K6);
   - private `interpolate_at_distance` 476–486; `PartialEq` 488; `Display` 494;
   - inline `mod tests` from line 505. New tests go to a sibling `cap_tests.rs`.
4. **`Region for Cap`** (`core/src/s2/region.rs:64–92`): `contains_cell` is
   `self.contains(cell.cap_bound())` (K2), `intersects_cell` `self.intersects(cell.cap_bound())`
   (K3); the other four delegate.
5. **Helpers in the library**: `MIN_WIDTH.max_level(val)` (`core/src/s2/metric.rs:93–105`,
   exact `ilogb`, debug-checks NaN); `Point::approx_eq_angle(other, eps)`
   (`core/src/s2/point.rs:199`, `Vector::angle <= eps`); `Cell::vertex`, `edge_raw`,
   `contains_point` (`core/src/s2/cell.rs:229, 250, 380`); 007's exact `remainder_2pi`
   (`core/src/s1/angle_tests.rs:59–79`, maps −π to π at the end).
6. **Upstream** (`/home/t/sources/s2geometry/src/s2/s2cap.cc`): `Complement` 65–71;
   `Contains(S2Cap)` 73; `Intersects` 78; `InteriorIntersects` 83; `AddPoint` 89–101;
   `AddCap` 103–113; `Expanded` 115–119; `Union` 121–141; `GetRectBound` 151–191;
   `GetCellUnionBound` 196–213; `Intersects(cell, vertices)` 225–264; `Contains(S2Cell)`
   266–280; `MayIntersect` 282–290; `operator==` 306–310; `ApproxEquals` 312–322;
   `s2metrics.h:186–200` (`GetLevelForMinValue`).
7. **Records and numbering.**
   - `verify/values/INVENTORY.md` has §1–8 and `PROPERTIES.md` §1–9 (§3 is the generic laws).
     This feature adds INVENTORY §9 and PROPERTIES §10, a `Cell` entry P-CCAP-01 in §1 and
     an update of P-PTDST-01 in §7 (line 220).
   - Mutants are numbered from `M-701`. The CI mutant matrix's `series` is
     `['0', …, '6']` (`.github/workflows/verify.yml:336`).
   - `make verify-values` maps parts through `VALUES_HARNESSES` (`Makefile:238`); its recipe
     (`:241–244`) runs `cd verify && cargo test`, the enumeration for `P` empty or `angle`,
     then `./run-verify.sh $(VALUES_HARNESSES)`.
   - `verify/run-verify.sh` calls `cargo kani` at lines 102 (codegen), 262 (main), 301
     (re-run) and 344 (playback).
   - `verify/known-timeouts.txt:21` lists `pt_dst::p_ptdst_01__symmetric P-PTDST-01`.
8. **The quickcheck twins** (`core/tests/trait_property_tests.rs`): generators `cap` and
   `small_cap` (lines 149–160) draw whole degrees; the `Region` property functions are at
   359–420 (`region_cap_bound_contains_point`, `region_rect_bound_contains_point`, …); the
   `Cap` instances at 673–705.
9. **C++ vectors**: `core/tests/data/cpp_values_vectors.csv` is 9.6 MB; `Cap`'s rows go to a
   new `cpp_cap_vectors.csv` from a new `generate_cap` target (research R9), alongside
   `generate_cellid`, `generate_cellunion`, `generate_coding`, `generate_values`
   (`tools/refgen-cpp/CMakeLists.txt:12–25`).
10. **Python and wasm** pin no exact coverings of caps (`python/tests/test_coverer.py`,
    `test_encoding.py`, `test_term_index.py` assert sizes and round trips only).

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: the modules, the runner's flag, the Makefile part and the record sections
(research R2; contracts).

- [X] T001 Add the new modules to `verify/src/lib.rs` and update `verify/Cargo.toml`'s comment (research R2; contracts/harness-conventions.md "Modules and names").
  - **New files**, each with the two SPDX lines, `// Written for this crate (not ported from upstream S2).` and a one-line `//!` naming its data-model section:
    - `verify/src/cap_spec.rs`, `verify/src/cap_spec_tests.rs` (data-model §1);
    - `verify/src/stubs.rs` (data-model §3);
    - `verify/src/cp_{crash,con,cls,mea,mem,rel,gro,cmp,bnd,eq,reg,lem}.rs`;
    - `verify/tests/libm_contracts.rs` (an empty test module for now).
  - **`verify/src/lib.rs`:** `pub mod cap_spec;` unconditional, next to `pub mod chord_spec;`,
    with its `#[cfg(test)] #[path = "cap_spec_tests.rs"]` link; `stubs` behind
    `#[cfg(kani)]`; the `cp_*` modules behind `#[cfg(all(kani, feature = "values"))]`;
    extend the crate doc's values paragraph (spec 008, `cap_spec`, `stubs`, `cp_`).
  - **`verify/Cargo.toml`:** the `values` comment names spec 008 and `cp_*` (`s2::Cap`). No
    dependency changes.
  - **Checks:** `cd verify && cargo build && cargo test`; `cargo build --no-default-features`;
    `cargo kani --only-codegen -Z stubbing`.
- [X] T002 [P] Pass `-Z stubbing` on every `cargo kani` call in `verify/run-verify.sh` (contracts/verify-commands.md; research R2; fact 7).
  - Lines 102, 262, 301 and 344: add `-Z stubbing` (one shell array, `stub_flag=(-Z stubbing)`,
    used in all four). Document it in the header comment ("the crate has `#[kani::stub]`
    attributes, which Kani requires the flag for; it changes nothing for harnesses without
    stubs").
  - **Check:** `make verify-values P=chord` still passes 28 of 28, unchanged times (±20%):
    the flag must not change other harnesses.
- [X] T003 [P] Extend `make verify-values` with `P=cap` in `Makefile` (fact 7).
  - `VALUES_HARNESSES` (line 238): `cap` → `$(call values_part,cp)`; without `P`, add
    `--harness cp_`.
  - The help text (line 241): add `s2::Cap (spec 008)` and `cap` to the `P=` list. Update the
    comment block above (lines 226–236).
  - **Check:** `make -n verify-values P=cap` lists the twelve `cp_*::` modules.
- [X] T004 [P] Add the record sections in `verify/values/` (data-model §4, §5, §8; fact 7).
  - `INVENTORY.md`: section 9 "`s2::Cap` (43 entries)", with 004's columns and no rows yet.
  - `PROPERTIES.md`: section 10 "`s2::Cap`", with 004's columns and no rows yet. In the
    opening paragraph: spec 008, and the two new statuses (data-model §8): **proven by
    composition** (lemma stubs, named) and **proven under a stated contract** (function
    contracts, named).
  - `EVALUATION.md`: a part "Spec 008: `Cap`", with 007's headings, "The canonical-value
    invariant" replaced by "Lemma stubs and function contracts", and "`LatLng` and `Cap`" by
    "What next: `LatLng`, and the other `Region` implementors".
  - **Check:** `cd verify && cargo test --test property_kinds` passes; if it checks statuses,
    teach it the two new ones.

**Checkpoint**: `make verify-values P=cap` runs zero harnesses and passes; `make
verify-values P=chord` is unchanged.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: the stubs and lemmas, the description, the domains, the new generic laws, the C++
vectors.

**⚠️ CRITICAL**: No user-story work starts until this phase is complete.

- [X] T005 Write `verify/src/stubs.rs`: the function contracts and the lemma stubs (data-model §3; research R5, R6; contracts/harness-conventions.md "Stubs"; fact 2).
  - **`Table<K>`**, exactly as `probe/r0c_harnesses.rs` (ten slots, element-wise keys, a fresh
    unrecorded value once full), plus `fn reset()` that clears every table. Tables in
    `static Mutex<Table<K>>`; never `thread_local!` or `Vec`.
  - **Function contracts:** `asin_contract`, `atan2_contract`, `cos_contract`, as
    `probe/r0b_harnesses.rs` states them (data-model §3's "Allows" column, verbatim).
  - **Lemma stubs:** `pure_chord`, `pure_chord_sym`, `pure_add`, `pure_add_sym`,
    `pure_is_unit`, `pure_approx_eq_angle`, from `probe/r0c_harnesses.rs`, renamed
    `memo_* → pure_*`; each with a doc comment naming its lemmas (data-model §3's "Rests on").
  - **Abstractions:** `draw_abstract_cell()`, `fixed_vertex`, `fixed_edge`,
    `any_contains_point`, `any_edge_result` (R0c), the last with the signature of FR-018's
    private `Cap::intersects_cell_edges` (T013).
  - Module doc: the soundness argument (research R6, "Soundness"), and the rule that a stub
    missing a fact can only fail a law.
  - **Check:** `cargo kani --only-codegen -Z stubbing` builds.
- [X] T006 [P] Add the C++ generator `tools/refgen-cpp/generate_cap.cc` and its CSV (research R9; fact 9; spec FR-024).
  - **`CMakeLists.txt`:** `add_executable(generate_cap generate_cap.cc)` and its
    `target_link_libraries`, after `generate_values`.
  - **Header comment**, as `generate_values.cc`'s: the oracle version, the columns of each
    section, "Deterministic (fixed seed)".
  - **Sections**, about 1,000 rows each except `CAP_SPECIAL`:
    - `# CAP_OPS`: random valid caps (unit centres from random lat/lng, radii from length²
      uniform in [0, 4] and from angles), a second cap and a point each. Results:
      `FromCenterHeight`, `FromCenterArea`, `S2Cap(c, S1Angle)` (length²); height, area,
      centroid (3 doubles); `Contains(p)`, `InteriorContains(p)`; `Contains(b)`,
      `Intersects(b)`, `InteriorIntersects(b)`; `Complement()` (centre, length²);
      `Expanded(S1Angle)` length²; `Union(b)` (centre, length²); `AddPoint(p)`, `AddCap(b)`
      (length²); `ApproxEquals(b)` and `ApproxEquals(b, 1e-10)`; `a == b`.
    - `# CAP_SPECIAL`: empty, full, single-point, hemisphere (length² 2), antipodal and
      identical centres, the radii next to each covering level's power of two (the probe's
      0.11785113019775795 and its neighbours for k = 1–29), K5's cap, K6's pair, centres at
      the poles and at longitude ±π.
    - `# CAP_BOUNDS`: `GetRectBound()` (four doubles), `GetCellUnionBound()` (the ids, `;`
      separated), `S2RegionCoverer` with `max_cells = 8` (ids), for the `CAP_OPS` and
      `CAP_SPECIAL` caps.
    - `# CAP_CELL`: `Contains(S2Cell)`, `MayIntersect(S2Cell)` for random caps against random
      cells at levels 0–10 within 2 radii of the centre, K2's 65-case grid and K3's cap.
  - Never call an operation outside upstream's domain (no non-unit point in `Contains`, no
    negative distance in `Expanded`).
  - Run it (prerequisites) to `core/tests/data/cpp_cap_vectors.csv`; record the oracle line
    as the first line. Add the target to `tools/refgen-cpp/README.md`'s table.
  - **Check:** `CAP_SPECIAL`'s rows reproduce `probe/cpp/survey_cpp.txt` (level 1, 38 of 65,
    `MayIntersect` 0, length² 3, the K5 rectangle, `ApproxEquals` 1).
- [X] T007 [P] Write `verify/src/cap_spec.rs` and `verify/src/cap_spec_tests.rs` (data-model §1; research R3).
  - Every item of data-model §1's two tables, reading only `center()` and `chord_radius()`.
    Never call `Cap::contains_point`, `contains`, `complement` or the other described
    operations. No `%`. Booleans compared with `==`.
  - Unit tests: `member` at the boundary (equal length²), for empty and full caps, and for
    `from_center_chord_angle(p, ZERO)` at `p`; each restatement on two hand-computed values
    (`upstream_complement_length2(1.0) == 3.0`; `upstream_level(0.11785113019775795) == 1`).
  - **Check:** `cd verify && cargo test cap_spec`.
- [X] T008 Extend `verify/tests/values_spec_vs_cpp.rs` with the `CAP_*` sections (research R9; depends on T006, T007).
  - Each restatement against its column, bit for bit; `member` against `Contains(p)`;
    `upstream_level` against `CAP_BOUNDS`' id levels. Print the row counts with
    `--nocapture`, as 007's sections do.
  - **Check:** `cd verify && cargo test --test values_spec_vs_cpp -- --nocapture`.
- [X] T009 [P] Write `verify/tests/libm_contracts.rs`: the contracts against the real functions (research R5; spec FR-003).
  - The three contracts restated as plain predicates (`asin_ok(x, r)`, `atan2_ok(y, x, r)`,
    `cos_ok(x, r)`), mirroring `stubs.rs` (a comment names the pair; a drift between them is a
    review finding).
  - Special values: ±0, ±1, ±(1 ± ε), ±∞, NaN, the smallest subnormal, `FRAC_PI_2` and its two
    neighbours, π; for `atan2` all sixteen combinations of {±0, ±∞} and the eight of {±0} ×
    {±1}.
  - 10⁶ random inputs per function from a fixed-seed generator (uniform bit patterns, and
    uniform values in the function's domain).
  - **Check:** `cd verify && cargo test --test libm_contracts --release`, under 10 s.
- [X] T010 [P] Extend `verify/src/value_domain.rs` with `any_box_cap`, `any_valid_cap`, `any_nonempty_cap`, `any_finite_nonneg_angle`, `any_lng_sum` (data-model §2; research R4).
  - `any_valid_cap` assumes `c.center().is_unit()`: a harness using it must stub
    `Point::is_unit` with `stubs::pure_is_unit` (say so in the doc comment, with R0's
    timeouts as the reason).
  - `ValidCell` is 004's `any_cell`.
  - **Check:** `cargo kani --only-codegen -Z stubbing`.
- [X] T011 [P] Add `laws::region::no_intersect_no_vertex` and `laws::region::cap_bound_contains_points` to `verify/src/laws.rs` (spec FR-010; data-model §4 P-CPREG-02, -03).
  - `no_intersect_no_vertex<R: Region>(r, c)`: if `!r.intersects_cell(c)`, no `c.vertex(k)` is
    contained. Doc: stated over the vertices the region's own test computed, so instances stub
    `Cell::vertex` purely (spec Edge Cases, "Two `sqrt` results").
  - `cap_bound_contains_points<R: Region>(r, p)`: if `r.contains_point(p)`, then
    `r.cap_bound().contains_point(p)`.
  - Add them to `all` only if every existing instance (`cl_cnt`'s P-CCNT-03) can take them;
    otherwise keep `all` and instantiate the new laws separately.
  - Name their quickcheck twins in the doc comments (T022).
  - **Check:** `cargo kani --only-codegen -Z stubbing`; `make verify-values P=cell` still
    passes (`cl_cnt`).
- [X] T012 Write the lemma harnesses in `verify/src/cp_lem.rs`, and measure L-CP-06 first (data-model §3 "Lemmas"; research R13).
  - `l_cp_01` … `l_cp_07`, each on the **real code** (no stub), each documented with its id,
    kind "law", statement, R0 time. L-CP-05 behind `#[cfg(feature = "slow")]` (626 s).
  - **L-CP-06** (`a + b` is `b + a` bit for bit, lengths in [0, 4]): run it first, alone,
    with `make verify-harness H=cp_lem::l_cp_06`. If it times out at 12 minutes, try it with
    `f64::sqrt` stubbed by a pure function (a `pure_sqrt` in `stubs.rs`, keyed by the input
    bits; the lemma is then "by composition", resting on `sqrt` being a function). Record the
    outcome; P-CPREL-03 depends on it.
  - L-CP-04 also covers `is_unit(−p) == is_unit(p)` by composition (research R0b); write the
    argument in `l_cp_04`'s doc comment.
  - **Check:** `make verify-values P='cp_lem::'` passes (per-change members); record the times.

**Checkpoint**: `cargo test` in `verify/` passes with the new tests; the lemmas are proven;
`stubs.rs` compiles; the C++ vectors exist.

---

## Phase 3: User Story 4 — Parity restored: the library changes (Priority: P2)

**Goal**: K1–K7 fixed to upstream's behaviour, each its own commit with a test that fails
without it (spec FR-017 to FR-023; research R8).

**Independent Test**: `cargo test -p s2rst --lib cap_tests` passes; each test fails with the
fix reverted, with the probe's value (quickstart §6); `cargo test -p s2rst --test
cpp_cap_diff` passes with no unexpected deviation rows.

- [X] T013 [US4] Create `core/src/s2/cap_tests.rs` and the FR-018 helper's shape in `core/src/s2/cap.rs` (fact 3, 4; research R7).
  - `cap_tests.rs`: two-line SPDX header and the "original test file" note; linked from
    `cap.rs` by `#[cfg(test)] #[path = "cap_tests.rs"] mod cap_tests;`.
  - The probe's cases as tests that **fail today** (record each failure in the commit message
    of its fix): `covering_level_matches_upstream` (K1), `contains_cell_matches_upstream` and
    `intersects_cell_matches_upstream` (K2, K3), `complement_radius_is_upstream`
    (K4), `rect_bound_contains_centre_at_tiny_longitude` (K5),
    `approx_eq_uses_tolerance_for_centres` (K6), `deserialise_rejects_invalid_caps` (K7, under
    `#[cfg(feature = "serde")]`).
  - Confirm K6 and K7 by these tests before fixing them (spec: "once a test confirms").
  - **Check:** the seven tests fail, with the values of quickstart §6.
- [X] T014 [US4] FR-017 (K1): the covering level through `MIN_WIDTH.max_level` in `core/src/s2/cap.rs:406–427` (research R8).
  - `let level = i32::from(MIN_WIDTH.max_level(radius).as_u8()) - 1;` replacing the `log2`
    computation and the `radius <= 0.0` branch (`max_level` returns 30 for it); keep the six
    faces below 0.
  - Test: the probe's radius gives level 1; for the 1,856 probe radii near the powers of two,
    the level equals `MIN_WIDTH.max_level(angle_radius) − 1` (the probe loop).
  - Commit message: K1, the failing value (level 2 for 0.11785113019775795), FR-017.
- [X] T015 [US4] FR-018 (K2, K3): upstream's cell tests in `core/src/s2/cap.rs` and `core/src/s2/region.rs:64–92` (research R7; fact 4, 6).
  - Run `core/benches/region_coverer.rs` and `core/benches/cap.rs` (gungraun) **before** the
    change and save the output (Constitution V).
  - `cap.rs`: private `fn intersects_cell_edges(self, cell: &Cell, vertices: &[Point; 4]) ->
    bool` (upstream `Intersects(cell, vertices)`, `s2cap.cc:225–264`, line for line, with the
    `RIGHT` hemisphere shortcut and the `Norm2()` factor); private `fn contains_cell_upstream`
    and `fn may_intersect_cell` computing the four vertices **once each**, as upstream's
    `Contains(S2Cell)` and `MayIntersect`.
  - `region.rs`: `Region for Cap`'s `contains_cell` and `intersects_cell` call them; update
    the comments (no longer "conservative").
  - Rustdoc: the parity with upstream and that coverings of caps now match C++.
  - Tests: the probe's 65-case grid (38 true) and K3's cap (false); a cell inside, outside and
    straddling a cap, at levels 0, 5, 20.
  - Run the benchmarks **after**; put both outputs in the commit message.
  - Commit message: K2, K3, the failing values, FR-018.
- [X] T016 [US4] FR-019 (K4): `complement` with upstream's radius in `core/src/s2/cap.rs:270–278`.
  - `Cap::from_center_chord_angle(-self.center, ChordAngle::from_length2(4.0 -
    self.radius.length2()))`.
  - Test: length² 1 → 3, 1.7 → 2.3 (C++'s values, `probe/cpp/survey_cpp.txt`); empty ↔ full.
  - Commit message: K4, 2.99999999999999956, FR-019.
- [X] T017 [US4] FR-020 (K5): an exact longitude reduction in `core/src/s2/cap.rs:395–398` (research R8).
  - Private `fn reduce_longitude(x: f64) -> f64` exactly as `probe/r0_harnesses.rs`'s
    `reduce_draft`, with a doc comment: the domain |x| ≤ 3π/2 (why it holds), the Sterbenz
    argument, no `%`.
  - Move 007's `remainder_2pi` and its helpers out of `core/src/s1/angle_tests.rs` into a
    shared test-only module (for example `core/src/test_util/remainder.rs` under
    `#[cfg(test)]`), with a variant that keeps −π (upstream's `remainder`, which
    `GetRectBound` uses); `angle_tests.rs` imports it. Its existing tests still pass.
  - Tests: K5's cap contains its centre; `reduce_longitude(x)` equals the exact remainder on
    10⁶ inputs uniform in [−3π/2, 3π/2] and on ±π, ±3π/2 and their neighbours.
  - Commit message: K5, the rectangle `[0, 0]`, FR-020.
- [X] T018 [US4] FR-022 (K6): `approx_eq_with` and `approx_eq` in `core/src/s2/cap.rs:454–468`.
  - `self.center.approx_eq_angle(other.center, Angle::from_radians(max_error))`;
    `approx_eq` is `approx_eq_with(other, 1e-14)`; rustdoc names upstream's `ApproxEquals`.
  - Test: the probe's pair (5.02·10⁻¹⁵ apart) is approximately equal; a pair 2·10⁻¹⁴ apart is
    not.
  - Commit message: K6, FR-022.
- [X] T019 [US4] FR-023 (K7): a validating `Deserialize` for `Cap` in `core/src/s2/cap.rs` (fact 3, 5).
  - Remove `serde::Deserialize` from the derive (line 85); a hand-written impl through a
    shadow struct `#[serde(rename = "Cap")] struct Fields { center: Point, radius: ChordAngle }`,
    as `Cell`'s (`cell.rs:810–850`): reject unless `is_valid()`, with "cap centre is not unit
    length" or "cap radius is above STRAIGHT" (the latter unreachable through 007's
    `ChordAngle` deserialiser, kept as defence).
  - Rustdoc: why (a derived impl accepts caps the binary decoder rejects).
  - Tests: a NaN centre and a centre of length 2 are rejected with their messages; empty,
    full and a random valid cap round-trip bit for bit through `serde_json`; the JSON text is
    unchanged.
  - Commit message: K7, FR-023.
- [X] T020 [P] [US4] Document the domains in `core/src/s2/cap.rs` (spec SC-010; research R1).
  - `contains_point` and `interior_contains_point`: a unit point (upstream's debug check);
    what a non-unit point gives.
  - `expanded`: `# Panics` in debug builds for a negative distance (upstream's check) and for
    +∞ (the chord-angle sum's check, 007 FR-019).
  - `union`: the documented deviation (the added error margin, `cap.rs:315–321`), with the
    reason (the union then contains both caps).
  - `from_center_angle`: `# Panics` in debug builds when the centre is not unit length.
  - `make doc` passes.
- [X] T021 [US4] Add `core/tests/cpp_cap_diff.rs` over `core/tests/data/cpp_cap_vectors.csv` (research R9; depends on T006, T014–T019).
  - Two-line SPDX header; parse as `cpp_values_diff.rs` does.
  - Every column bit for bit (ids exactly, coverings as sorted id lists); `Union` by
    containment of both caps, marked as the documented deviation.
  - **Check:** `cargo test -p s2rst --test cpp_cap_diff` passes; with T014–T019 reverted one at
    a time, the matching rows fail.
- [X] T022 [P] [US4] FR-011: the quickcheck twins in `core/tests/trait_property_tests.rs` (fact 8; research R14).
  - A generator `cap_f64(lat: f64, lng: f64, r: f64)` clamping finite inputs into range,
    beside `cap` (line 149); the `Region` cap properties (673–705) gain `_f64` instances.
  - K5's cap as a fixed `#[test]` of `region_rect_bound_contains_point` (fails before T017).
  - Twins of the new laws (T011): `region_no_intersect_no_vertex`,
    `region_cap_bound_contains_points`, instantiated for `Cap`, `Cell` and `Rect`.
  - **Check:** `cargo test -p s2rst --test trait_property_tests`.
- [X] T023 [US4] Run `make test` and the probe again (spec SC-007; SC-009).
  - `make test` passes (Python and wasm included).
  - Re-run `specs/008-formal-verify-cap/probe/rust` (`cargo run --release -q`): K1 0 of 1,856,
    K2 0 of 65, K3 false, K4 0 of 10⁶, K5 contains the centre, K6 true. Save the output as
    `probe/rust/survey_rust_after.txt` and add a column to `probe/README.md`.

- [X] T059 [US4] K8, found by T008's vectors: upstream's `GetCellUnionBound` does not treat the empty cap apart (its radius −1 rad gives level 29 and four vertex-neighbour cells), where the library returns no cells (`core/src/s2/cap.rs:406–409`). The library's answer is tighter and still a covering of the empty set. Document it as a deviation in `cell_union_bound`'s rustdoc (with the reason), mark the empty-cap row of `CAP_BOUNDS` as the deviation in `core/tests/cpp_cap_diff.rs`, and record K8 in the evaluation. **Changing the behaviour to upstream's needs the user's go-ahead** (known-defect rule); until then, document only. **Changed 2026-10-25:** the user chose to match upstream; `cell_union_bound` now returns the four cells (`8c91967`, test `empty_cap_covering_matches_upstream`), and `cpp_cap_diff` compares the empty cap's row bit for bit.

**Checkpoint**: the library matches upstream on every probe case; the gate passes.

---

## Phase 4: User Story 1 — Proven crash freedom (Priority: P1)

**Goal**: every in-scope entry proven free of panics, overflow and undefined behaviour over its
documented domain (spec FR-004, SC-001, SC-002).

**Independent Test**: `make verify-values P=cap` reports every crash harness `SUCCESSFUL`; the
weekly ones pass or end `KNOWN-TIMEOUT` with a recorded reason.

- [X] T024 [US1] Write the crash harnesses of the 27 plain entries in `verify/src/cp_crash.rs` (data-model §5; contracts/harness-conventions.md).
  - One harness per operation, or one per group of accessors and predicates, named
    `c_<method>`; a property harness of Phase 6 may discharge an entry when its domain covers
    the crash domain (002's rule), then the inventory row names it.
  - Domains: `BoxCap`, `BoxPoint`, `AnyF64` for heights and areas, `FiniteNonNegAngle` for
    `expanded`; `from_center_angle` over a unit centre with `pure_is_unit` (R0b 1.7 s).
  - Each with a `kani::cover!` showing its outcomes reachable (both results of each predicate).
  - **Check:** `make verify-values P='cp_crash::'`.
- [X] T025 [US1] Write the crash harnesses of the cell tests (research R7).
  - `c_contains_cell__abstract`, `c_intersects_cell__abstract`: over `AbstractCell` (stubs
    `fixed_vertex`, `fixed_edge`, `any_contains_point`), per change if under a minute.
  - `c_cell_tests__real` (weekly, `#[cfg(feature = "slow")]`): over `ValidCell` with
    `fixed_vertex` only (the vertices' normalisation is 006's), the edges real.
  - **Check:** run each alone; record times.
- [X] T026 [US1] Write the crash harnesses of the 8 contract entries (data-model §5; spec FR-003).
  - `angle_radius` (asin), `rect_bound` and `Region::rect_bound` (asin, atan2, cos: R0b 84.6 s),
    `approx_eq`, `approx_eq_with` (atan2), per change.
  - `cell_union_bound`, `Region::cell_union_bound` (asin), `union` (asin, atan2): weekly
    (`#[cfg(feature = "slow")]`), 60 minutes; if they time out, list them in
    `verify/known-timeouts.txt` as tool limits.
  - Each documents its contracts ("Under contract: `asin_contract` …").
- [X] T027 [US1] Run the crash suite and fill `verify/values/INVENTORY.md` §9 (data-model §5).
  - `make verify-values P='cp_crash::'`, then `make verify-slow F=values P='cp_crash::c_cell_tests__real cp_crash::c_cell_union_bound cp_crash::c_union'`.
  - 43 rows: domain, harness, status (proven / under contract / 003 / excluded), time; every
    exclusion with its reason.
  - Any panic found inside a documented domain: the known-defect rule.
- [X] T028 [US1] Record undocumented preconditions the proofs needed (spec FR-004; SC-010).
  - Each one documented in `cap.rs` (as T020) and listed in the evaluation.

**Checkpoint**: US1 independently verified (spec SC-001, SC-002).

---

## Phase 5: User Story 2 — The generic trait laws for `Cap` (Priority: P1)

**Goal**: `Cap` counts as a verified `Region`, `PartialEq` and codec implementor (spec FR-010,
SC-004).

**Independent Test**: each law instance reported by name; a mutant of `equal`, the cell tests,
`contains_point` or the codec fails its instance.

- [X] T029 [P] [US2] P-CPEQ-01: the equality laws in `verify/src/cp_eq.rs` (data-model §4).
  - `p_cpeq_01`: `laws::eq::all` over `BoxCap³` (R0 1.9 s); plus any two empty caps equal and
    any two full caps equal, whatever their centres.
- [X] T030 [P] [US2] P-CPREG-01: containment implies intersection in `verify/src/cp_reg.rs` (research R7).
  - `p_cpreg_01__abstract`: `laws::region::contains_implies_intersects(&cap, &face0())` with
    `draw_abstract_cell()` first; stubs `fixed_vertex`, `pure_chord`, and
    `any_edge_result` on `s2rst::s2::cap::Cap::intersects_cell_edges` (the private helper of
    T015). R0c 53 s.
  - `p_cpreg_01__real` (weekly): the edge test real, stubs `fixed_vertex`, `fixed_edge`,
    `any_contains_point`, `pure_chord`. R0c 697 s.
  - Doc: why an abstract cell proves it for every real cell (research R7, last bullet).
- [X] T031 [P] [US2] P-CPREG-02: no intersection, no contained vertex in `verify/src/cp_reg.rs`.
  - `p_cpreg_02`: `laws::region::no_intersect_no_vertex`, stubs as `p_cpreg_01__abstract`.
    R0c 51 s.
- [X] T032 [P] [US2] P-CPREG-03: the bounding cap contains contained points in `verify/src/cp_reg.rs`.
  - `p_cpreg_03`: `laws::region::cap_bound_contains_points` over `BoxCap × BoxPoint`, no stub
    (`cap_bound` is the identity; if it recomputes the chord twice and times out, add
    `pure_chord`).
- [X] T033 [US2] P-CPREG-04: the covering law, weekly attempt, in `verify/src/cp_reg.rs` (spec SC-004).
  - `p_cpreg_04` (`#[cfg(feature = "slow")]`): `laws::region::covering_covers::<Cap, 6>` over
    `ValidCap × ValidCell` with `asin_contract`, `fixed_vertex`, `pure_is_unit`. If it times
    out at 60 minutes, record "not proven (timeout)" and list it in `known-timeouts.txt`.
- [X] T034 [P] [US2] P-CPCOD-01: the codec law under `Cap`'s equality in `verify/src/cp_eq.rs` (data-model §4).
  - `p_cpcod_01`: encode, decode, `==`, over `ValidCap` with `pure_is_unit` (R0b 8.8 s). Doc:
    003's P-FX-03 proves the bytes; this is the `CORE_TRAITS.md` law.
- [X] T035 [US2] Run US2's harnesses and record them (spec FR-026).
  - `make verify-values P='cp_eq:: cp_reg::'`; the weekly members with `make verify-slow`.
  - `verify/values/PROPERTIES.md` §3 (generic laws): the two new laws, and the instances'
    ids. §10: P-CPEQ-01, P-CPREG-01 to -04, P-CPCOD-01 with statuses and stubs named.
  - `CORE_TRAITS.md`'s "Region" and "S2Decode" invariants name their proven instances (Kani
    and quickcheck).
  - `verify/decoding/PROPERTIES.md`: P-FX-03's row points to P-CPCOD-01.

**Checkpoint**: US2 independently verified (spec SC-004).

---

## Phase 6: User Story 3 — Proven meaning against `cap_spec` (Priority: P2)

**Goal**: the catalogue's other entries proven, and the compositions in `Cell` and `Point`
(spec FR-006, FR-012, SC-003).

**Independent Test**: every entry reported with its status; every proven one has a killed
mutant.

- [X] T036 [P] [US3] P-CPCON-01, -02 in `verify/src/cp_con.rs` (data-model §4).
  - Constructors against the restatements, bit for bit; `from_center_angle`'s clamp, with
    `pure_is_unit` for its validity check.
- [X] T037 [P] [US3] P-CPCLS-01 in `verify/src/cp_cls.rs`; P-CPMEA-01 in `verify/src/cp_mea.rs`.
  - Classifications against `cap_spec`; empty and full exclusive on valid caps; the constants
    valid (L-CP-07). Measures bit for bit against the restatements, compared as the library's
    expression with an identical restatement (007's lesson); the area never negative.
- [X] T038 [P] [US3] P-CPMEM-01 in `verify/src/cp_mem.rs`.
  - `contains_point == member`, `interior_contains_point == interior_member`, the
    implication, `from_point(p)` contains `p`. One chord computation each side: measure on the
    real code first; if it times out, the description's `chord2` and the library's
    `chord_angle` are the same expression, so use `pure_chord` and say so.
- [X] T039 [P] [US3] P-CPREL-01 to -03 in `verify/src/cp_rel.rs` (R0 13.5 s, R0b 20.3 s, R0c 27.0 s).
  - P-CPREL-01 on the real code; P-CPREL-02 with `pure_chord`, `pure_add`; P-CPREL-03 with
    `pure_chord_sym`, `pure_add_sym`, resting on L-CP-06 (T012's outcome decides its status).
- [X] T040 [P] [US3] P-CPGRO-01 to -03 in `verify/src/cp_gro.rs` (R0c 6.8 s, 17.0 s; R0 336.9 s).
  - P-CPGRO-01, -02 with the stubs of R0c; the "did not shrink" clauses.
  - P-CPGRO-03: first a per-change variant with `pure_add`; if it fails or times out, keep
    the real-code member weekly (`#[cfg(feature = "slow")]`).
- [X] T041 [P] [US3] P-CPCMP-01 to -03 in `verify/src/cp_cmp.rs`.
  - -01 bit for bit against `upstream_complement_length2` (after T016); -02 with
    `pure_is_unit` (R0c 4.7 s); -03: characterise with Kani where `4 − (4 − l)` differs from
    `l` and state it (exact for every l² ≥ 2; else within 2⁻⁵¹), split `__ge2`/`__lt2` if
    needed.
- [X] T042 [P] [US3] P-CPBND-01 to -05 in `verify/src/cp_bnd.rs`.
  - -01 the delegation and identity; -02 the level against `upstream_level` and the margin
    (R0 13.8 s), stated on the library's level function — if it is not separately callable,
    on `cell_union_bound`'s ids' level under `asin_contract` with `CellId::from_point` and
    `vertex_neighbors` cost measured, else on a restated level with the code shape compared in
    review; -03 weekly under `asin_contract`; -04 under the three contracts (R0b 84.6 s);
    -05 on `reduce_longitude` (R0 2.9 s) — private, so instantiate it through a public path
    or copy it with a drift guard as 003's restated codecs did, and say which.
- [X] T043 [P] [US3] P-CPEQ-02 in `verify/src/cp_eq.rs`.
  - `approx_eq_with` against upstream's formula with `pure_approx_eq_angle`; `approx_eq` is
    it at 10⁻¹⁴.
- [X] T044 [US3] The compositions in other types (spec FR-012; research R12; data-model §4.1).
  - **P-CCAP-01** (new, `Cell`): a harness in `verify/src/cl_cnt.rs` over `ValidCell` with
    `Cell::vertex` stubbed purely: `cell.cap_bound()` contains `cell.vertex(k)` for k = 0..3
    and the centre it used, by P-CPGRO-01. Add the row to `PROPERTIES.md` §1.
  - **P-PTDST-01** (`Point`): delete `pt_dst::p_ptdst_01__symmetric` and its line in
    `verify/known-timeouts.txt:21`; write the composition argument (L-CP-03, L-CP-04, equal
    sums, `min`, `from_length2`) into the entry at `PROPERTIES.md:220`, status "proven by
    composition" for the symmetry clause. Its mutants M-532, M-533: re-check they still have a
    killing harness, else retarget them to `cp_lem::l_cp_03`/`l_cp_04`.
- [X] T045 [US3] Run US3's per-change and weekly harnesses; fill `verify/values/PROPERTIES.md` §10 (data-model §4; spec SC-003).
  - `make verify-values P=cap`; `make verify-slow F=values P='<the cp_ weekly members>'`.
  - Every row: statement, kind, upstream source (fact 6), domain, harnesses, status (with
    stubs and contracts named), time, mutants.
  - SC-003: count the proven among the 25; if under 23, say which and why.
- [X] T046 [US3] Write the mutants `verify/mutants/M-701…` and run them (data-model §7; research R11).
  - One patch per candidate of data-model §7, 004's format (`Tier: slow` where the target is
    weekly; `Test:` where an ordinary test kills it, as for FR-020's reduction).
  - `make verify-mutants M='M-7*'` (`VERIFY_SLOW_HARNESS_TIMEOUT=60m` for the slow ones);
    record outcomes in `verify/MUTANTS.md`. A survivor: explain or add a harness.
  - The two stub mutants must make their laws fail (the facts are needed).

**Checkpoint**: US3 independently verified (spec SC-003, SC-005).

---

## Phase 7: User Story 5 — CI, records and evaluation (Priority: P3)

**Goal**: the proofs run in CI; the records and the evaluation are complete (spec FR-015,
FR-016, FR-026; SC-006, SC-011, SC-013).

**Independent Test**: a broken property fails the `values-cap` leg and names it; a maintainer
answers SC-013's questions from the evaluation alone.

- [X] T047 [US5] Extend `.github/workflows/verify.yml` (contracts/verify-commands.md; fact 7).
  - `verify` matrix (line 133): `values-cap`. Path filters: `core/src/s2/metric.rs`,
    `core/tests/trait_property_tests.rs`, `verify/src/stubs.rs` (`cap.rs`, `region.rs` are
    there).
  - `slow` job (lines 228–258): a `values-cap` leg with the weekly filters of T045; extend
    the leg list comment's totals.
  - `mutants` matrix (line 336): `'7'`.
  - **Check:** `actionlint` if installed; else a YAML parse.
- [X] T048 [P] [US5] Extend `verify/TRUSTED_BASE.md` (spec FR-016; research R5, R6).
  - A section for spec 008: the three function contracts, their tests (T009) and the proofs
    that rely on each; each lemma stub with its lemmas; the purity argument; the abstract
    cell; CBMC's loose `cos` and `log2` (R0); the Kani compiler crash with `thread_local!`.
  - Amend the earlier "Nothing is stubbed" statements: still true for 001–007's proofs.
- [X] T049 [P] [US5] `verify/PROPERTIES_BY_KIND.md` and its HTML render (spec SC-011).
  - Every new id once (26, the 7 lemmas, P-CCAP-01); P-PTDST-01's status; re-render with
    `pandoc -f markdown -t html`. **Check:** `cd verify && cargo test --test property_kinds`.
- [X] T050 [P] [US5] `VERIFIED_TYPES.md`: section 2.9 for `Cap` and the summary tables (spec FR-026).
  - §1: counts (harnesses, mutants), "Types with their own proof suite" gains `Cap`;
    "Traits with proofs" gains `Cap`'s `Region`, `PartialEq` and codec law.
  - §2.6: P-PTDST-01's symmetry now by composition. §2.8: remove `Cap`.
  - §3: the `Region`, `S2Encode`/`S2Decode` and standard-trait rows.
  - §6 step 4: the `Region` laws' state; a note on lemma stubs as the method for laws that
    recompute float values.
- [X] T051 [US5] Write the spec-008 part of `verify/values/EVALUATION.md` (spec FR-016, SC-013; research R15).
  - Effort, run times, what was reachable, limitations, defects (K1–K7, any D-009+).
  - "Lemma stubs and function contracts": what they cost (harness count, trust) and gained
    (the laws three earlier specs left as timeouts), and whether to use them for the other
    value types' open timeouts (004's P-IEXP-02/03, 005's P-LEXP-02, 006's P-PTCLS-03,
    P-PTORT-03, 007's `sin2`, `sin` and the quotients).
  - "What next": go or no-go for `LatLng`; which `Region` implementors the generic laws reach
    next (`Rect`, `CellUnion`, `Cell` through P-CCAP-01); ranked follow-ups (including T057,
    T058).
  - An Acceptance section: each SC with met / not met and the evidence.

**Checkpoint**: US5 independently verified.

---

## Phase 8: Polish & Cross-Cutting Concerns

- [X] T052 Final run, one at a time (quickstart §1–5).
  - `make test`; `cd verify && cargo test`; `make verify-values P=cap` and the whole
    `make verify-values` (the `-Z stubbing` flag across all parts); the weekly `values-cap`
    members; `make verify-mutants M='M-7*'`.
  - Record the run (date, commit, times) in the evaluation's "Final run".
- [X] T053 Check each SC against the evidence and update the spec's status line (spec SC-001 to SC-013).
- [X] T054 [P] `specs/008-formal-verify-cap/quickstart.md`: replace the expected values with the measured ones.
- [X] T055 [P] Update `CORE_TRAITS.md`'s "Generic Property-Test Harness" section: the Kani law instances beside the quickcheck twins, and the two new `Region` laws.
- [ ] T056 Live CI check (spec SC-006): needs a push and a draft PR. **The user said never push (2026-10-25)**, so it stays open; SC-006 is measured on the workstation only.
- [X] T057 Report Kani 0.68.0's compiler crash on a stub with `thread_local!` (`kani-compiler/src/intrinsics.rs:243`) to the Kani project, with a minimal reproduction from `probe/r0b_harnesses.rs`'s first version; **needs the user's approval** (as 007's T062). **Done 2026-10-25:** at the user's choice Kani bugs are recorded, not filed; this is KI-003 in `KANI_ISSUES.md`. The minimal reproduction (re-run that day) needs no stub: any harness reaching a `thread_local!` whose value has a destructor crashes the compiler.
- [X] T060 U-015 (found by T021's vectors, FR-025): upstream's `S2Cap::Union` (`s2cap.cc:121–141`) does not contain both caps on 337 of the 1,089 `CAP_OPS`/`CAP_SPECIAL` rows, by rounding (an ulp of the radius), though `s2cap.h` says it "encloses" them; the library's union, with its added margin (`core/src/s2/cap.rs`, the documented deviation), misses on 7 (K9). Recorded in `UPSTREAM.md` and the evaluation, cited in `cp_crash::c_union`'s doc comment, pinned by `cpp_cap_diff::union_containment_matches_the_record`. Reporting it upstream **needs the user's approval**. **Done 2026-10-25:** the user's answer is that upstream reports are kept in `UPSTREAM.md` at the repository root, where U-015 is recorded; its entry now names this task.
- [X] T061 K10 (found by T021's vectors): `RegionCoverer`'s covering of a cap (`max_cells = 8`) differs from C++'s on 38 of 836 `CAP_BOUNDS` rows, though the cap's cell tests and `cell_union_bound` match C++ on every row: the difference is in the coverer, outside spec 008's scope. Pinned in `cpp_cap_diff::cap_bounds_match_cpp`; rank it in the evaluation's follow-ups (a coverer spec), do not change it here.
- [X] T062 D-009 (found by `cp_mea::p_cpmea_01__area`, weekly): `Cap::area`'s `0.0_f64.max(h)` gives −0 for r² = −0 in debug builds, upstream's `std::max` +0. The user gave the go-ahead (2026-10-25). Fix with upstream's comparison and a test that fails before (`1b26156`); remove the `known-defects.txt` line; re-run `p_cpmea_01__area` weekly; regenerate `M-708` against the new code and add `M-741` (the fix reverted); update the records (P-CPMEA-01, the evaluation, `VERIFIED_TYPES.md`, the index by kind). **Done 2026-10-25:** `p_cpmea_01__area` proves in 1,195.3 s; `M-708` and `M-741` killed; `make verify-values P=cap` 45 of 45; `make test` passes.
- [X] T058 Follow-up, recorded in the evaluation only: the private `ieee_remainder` of `core/src/s1/interval.rs:589` computes `x − round(x / y) · y`, which is not exact, though its comment says "same as C `remainder()`" (seen while planning FR-020; it is spec 004's `expanded`). Rank it in the evaluation's follow-ups; do not change it here.

---

## Dependencies & Execution Order

### Phase dependencies

- **Phase 1** → **Phase 2** → **Phase 3 (US4)** → **Phases 4–6** (US1, US2, US3; any order,
  each needs Phase 3's code shape) → **Phase 7 (US5)** → **Phase 8**.
- T013 before T014–T019 (the failing tests first). T015 before T025, T030, T031 (the private
  helper they stub). T016 before T041. T017 before T042 (-04, -05). T014 before T042 (-02).
- T012 (L-CP-06) before T039's P-CPREL-03.
- T006, T007 before T008; T006 and T014–T019 before T021.

### Within each story

- Harness code [P] across files; Kani runs one at a time.
- Records (T027, T035, T045) after their runs.

### Parallel opportunities

- Phase 1: T002, T003, T004.
- Phase 2: T006, T007, T009, T010, T011 (T005 first for T010's stub references; T012 after
  T005).
- Phase 3: T020, T022 beside T014–T019 (different files); T014–T019 are one file each and
  sequential (one commit each).
- Phases 4–6: the `cp_*` modules (T029–T034, T036–T043) are separate files.
- Phase 7: T048, T049, T050.

---

## Parallel Example: User Story 3

```bash
# Write in parallel (different files):
Task: "P-CPCON-01, -02 in verify/src/cp_con.rs"
Task: "P-CPCLS-01 in verify/src/cp_cls.rs; P-CPMEA-01 in verify/src/cp_mea.rs"
Task: "P-CPMEM-01 in verify/src/cp_mem.rs"
Task: "P-CPREL-01 to -03 in verify/src/cp_rel.rs"
Task: "P-CPGRO-01 to -03 in verify/src/cp_gro.rs"
Task: "P-CPCMP-01 to -03 in verify/src/cp_cmp.rs"
# Then run once, alone:
make verify-values P=cap
```

---

## Implementation Strategy

### MVP first

1. Phases 1–2 (setup, stubs, lemmas, description, vectors).
2. Phase 3 (the fixes): the library matches upstream on every probe case — useful on its own.
3. Phase 4 (US1): crash freedom. **Stop and validate**: `make verify-values P=cap`.

### Incremental delivery

4. Phase 5 (US2): the trait laws — the user's explicit ask ("use the proofs we have generated
   for traits").
5. Phase 6 (US3): the catalogue and the compositions.
6. Phases 7–8: CI, records, evaluation, final run.

### Notes

- Every library change is its own commit with its failing value in the message.
- Commit `verify/` changes with their records in the same commit (constitution v1.2.0).
- Never push (the user, 2026-10-25): local commits only; T056 stays open. Upstream findings go in
  `UPSTREAM.md` and Kani bugs in `KANI_ISSUES.md`; neither is filed.
