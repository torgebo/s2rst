---

description: "Task list for the formal verification of Cell and s1::Interval (spec 004)"
---

# Tasks: Formal Verification of `Cell` and `s1::Interval`

**Input**: Design documents from `specs/004-formal-verify-cell-interval/`:

- plan.md, spec.md;
- research.md (R0–R15);
- data-model.md;
- contracts/ (verify-commands, harness-conventions);
- quickstart.md;
- the Kani probe `probe/r0_harnesses.rs`, whose harnesses are the first drafts of several
  tasks below.

The feature extends the `verify/` crate of 001–003. Their task lists
(`specs/00{1,2,3}-*/tasks.md`) show the established patterns, and the existing files in
`verify/` are the style reference.

**Prerequisites**:

- **Kani 0.68.0**: `make verify-kani-check` passes.
- **For T005 only**: a C++ toolchain (`cmake`, `g++`), the pinned upstream checkout at
  `/home/t/sources/s2geometry` (`v0.14.0-40-g37a4501`), and its install at `/tmp/s2-install`.
  **`/tmp/s2-install` no longer exists (checked 2026-10-12)**, so T005 rebuilds it
  (`tools/refgen-cpp/README.md`, "Build the C++ reference").
- **Disk**: `df -h /` shows at least 20 GB free before any Kani run. `verify/target` is
  already 40 GB (memory note "Disk space and the Kani cache").
- **The working tree** already holds three uncommitted changes from planning (research
  R15). T013 commits them:
  - `core/tests/trait_property_tests.rs`, new;
  - `core/src/s2/rect.rs` (D-006's fix);
  - `core/src/s2/rect_tests.rs`, new.

  Do not revert them.

**Tests**: The spec asks for these ordinary tests, so they appear as tasks:

- the regression tests of FR-018, FR-020, D-005 and D-006, each failing without its fix
  (Constitution III);
- the differential tests against the new C++ vectors (FR-021);
- the reference descriptions' sanity tests against the same vectors (FR-007);
- the exhaustive projection check over every boundary value (R8);
- the generic trait property tests and `Cell`'s new instances (R15, Constitution III);
- one Python test for FR-020's visible change.

The Kani harnesses are the verification deliverable itself, so they are implementation
tasks.

**Organization**: Tasks are grouped by user story (spec.md):

| Story | Priority | Phase |
|-------|----------|-------|
| US1: proven crash freedom | P1 | 4 |
| US2: proven meaning | P2 | 5 |
| US3: CI regression protection | P3 | 6 |
| US4: the evaluation | P3 | 7 |

The library changes (FR-017 to FR-020, D-005, the generic tests with D-006) come first, in
Phase 3, because the proofs are stated against the fixed behaviour (plan, Complexity
Tracking).

## Progress (2026-10-13)

Paused on 2026-10-12 at the user's request to specify spec 005, and resumed on 2026-10-13 for
the runs. Done: Phases 1–4; in US2 every harness, the catalogue and the by-kind index; every
per-change harness passes (`make verify-values`); and since 2026-10-13:

- **The weekly members (T032; T039's weekly half).** From a frozen worktree at `e60a044`,
  60 minutes each: every bound-family level is proven (P-CBND-01 and -02 to 12, P-CCHD-02
  to 6, P-CEQ-02 to 10, the longest 2625 s), and P-IMEA-06's range clause. P-IEXP-02's
  symmetry and default members time out: not proven (tool limit). `p_iexp_01_shrink__xinv`
  found **U-011**, fixed by FR-023 (T055); the four containment members are re-run after the
  fix (T056).
- **The mutant campaign (T040):** all 42 killed.
- **The re-runs on the fixed library (T056, 2026-10-13):** per change, 41 of 43; the two
  P-IEXP-03 members timed out at 12 minutes. The four `p_iexp_01_{grow,shrink}__*` members
  proved in 21–63 s each, so P-IEXP-01 is `proven` over its whole domain. `M-343` was
  killed; `M-344` and `M-345` are errors, because their targets are the P-IEXP-03 members.
- **P-IEXP-03 at the weekly limit (T056, 2026-10-14):** both members timed out at 60 minutes,
  so P-IEXP-03 is not proven (tool limit), and P-IEXP-01's proof stands on its own. The two
  members moved to the weekly tier, with `Tier: slow` on `M-344` and `M-345`, which stay
  without a verdict.

Open, in order:

1. **T042's remaining half: the weekly `values` leg** in `.github/workflows/verify.yml`. The
   CI weekly tier allows 30 minutes per harness, but two members took longer on the
   workstation (P-CCHD-02 at level 6, 33 minutes; P-IMEA-06's `__ynoninv_xnoninv`, 44
   minutes), a hosted runner is slower, and the P-IEXP-02 and P-IEXP-03 members never finish.
   Decide the leg's time limit, and whether the members that only time out stay in it.
2. **T043, T044: CI.** Measure both legs on the runner. That needs the user's push approval.
3. **T046, T048: the evaluation and `VERIFIED_TYPES.md`.** The interim notes are in
   `verify/values/EVALUATION.md`.
4. **T049–T052: polish; T053, T057: follow-ups** (upstream reports, the user's decision).

Findings during implementation (recorded in `verify/values/EVALUATION.md`):

- a **Kani 0.68.0 soundness bug**: `<` and `<=` on symbolic `bool`s are wrong;
- the interval relations are those of **real arcs**, not float sets (T010);
- P-IMEA-06's distance **rounds to 0 across a one-ulp gap**;
- an id whose marker bit is at an odd position gives `Cell::from_cell_id` **unspecified
  fields rather than a panic** (now documented).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on unfinished tasks). For
  Kani **runs**, "parallel" applies only to writing the code; the runs themselves go one
  after another (Memory safety, below).
- **[Story]**: the user story the task belongs to (US1–US4).
- Paths are relative to the repository root.

## Memory safety while implementing (read first)

R0 saw harnesses reach 0.9 GB and the 12-minute limit. Uncapped CBMC can take down the
session.

- **Every** Kani run goes through a Makefile target, which applies the memory guard of
  `verify/run-verify.sh`:
  - `make verify-values [P=cell|interval]`;
  - `make verify-harness H=…`;
  - `make verify-mutants M=…`;
  - `make verify-slow F=values`;
  - `make verify`.
- For a throwaway measurement outside the Makefile, use R0's set-up: a scratch crate with a
  copy of `run-verify.sh` and an empty `known-defects.txt`, run with
  `VERIFY_JOBS=4 VERIFY_HARNESS_TIMEOUT=12m VERIFY_MEM_PER_HARNESS=4G`. Or cap by hand:
  `timeout -k 10 900 systemd-run --user --scope --unit=vt-probe-<n> -p MemoryMax=4G -p MemorySwapMax=0 cargo kani --exact --harness <h>`.
  Run `systemctl --user reset-failed 'vt-probe-*'` before reusing unit names.
- Run separate verification jobs **one after another**: full runs, mutant campaigns and
  family measurements.
- When timing a harness for a tier decision, run it with `VERIFY_JOBS=4` or fewer. R0's
  round 2 ran 5 jobs, and its times are inflated.

## Upstream findings rule (FR-022, read first)

If a harness, a C++ vector, or reading upstream code shows **undefined or wrong behaviour in
the upstream reference**, do these **before** marking the affected proof complete:

1. Append a task to Phase 9 with the next free task ID and the next free `U-NNN`. U-007 is
   the K4 corner (T020); the next free one is U-008. Give the upstream `file:line`, the
   inputs, the observed behaviour, and what the library does.
2. Add the same entry under "Upstream findings" in `verify/values/EVALUATION.md`, and to
   `UPSTREAM.md`.
3. Cite the `U-NNN` in the affected harness's doc comment.

Never change upstream code.

## Known-defect rule (FR-012, read first)

A proof that fails on a **real library defect** gets a `D-NNN`:

- D-005 (`directed_hausdorff_distance`, R9) and D-006 (`Rect::cap_bound`, R15) are taken;
  the next free one is **D-007**.
- Record it under "Defects found" in `verify/values/EVALUATION.md`.
- If its harness is written before the fix lands, list the harness in
  `verify/known-defects.txt`, so that runs report `KNOWN-DEFECT`.
- The fix is a **separate, tested change**.
- **Ask the user before fixing** a defect that is not in the spec's survey or this plan,
  as 002 did for D-002. D-005's fix is in the plan (R9). D-006's fix was applied during
  planning, after the user asked for the generic tests to be brought in.

## Facts these tasks rely on

1. **Products of symbolic floats do not scale** (R0). Comparisons and additions prove
   whole in seconds to minutes. Anything asserted on `Cell`'s u-v bounds squares a
   symbolic `s`:
   - P-CBND-01 (L=3: 9.5 s; L=10: 575 s; L=20: ⏱);
   - P-CBND-02 (L=3: 9 s; L=10: 98 s; L=15: ⏱);
   - P-CCHD-02 (L=3: 109 s; L=5: 638 s; L=7: ⏱).

   They are proved per level, as `__l<L>` families (R8). A whole-domain harness asserting on
   the bounds did not finish in 2 hours in the pilot.
2. **Bit equality.** A property that says two floats are equal compares `to_bits()`
   (harness-conventions). `PartialEq` on `s1::Interval` is float `==`: `[0,0] == [−0,−0]`.
3. **Kani builds with debug assertions on.** Every `debug_assert!` on a harness's path is
   an obligation. That is how R0 found D-005 and K5.
4. **Intervals are built as struct literals** (`Interval { lo, hi }`, public fields), and
   assumed valid through `arc_spec::valid`, never the library's `is_valid` (R4). Cells come
   from 001's `domain::any_valid()` / `any_valid_at(L)`; `CellId`'s tuple field is public.
5. **`--harness cl_` and `--harness iv_` select only the new suites.** No existing harness
   path or module contains either (checked 2026-10-12, 172 harnesses).
6. **P-IMEA-01's shape** (R0):
   - the library's `length` against `arc_spec::upstream_length`, two pipelines, took 217 s;
   - a three-way version, which also ran a restated fix, took 525 s;
   - splitting by inversion took longer in total (318 s + 121 s).

   Keep one harness.
7. **D-005's input**: `[1, 2].directed_hausdorff_distance([0, 1e−17])` panics in a debug
   build with "assertion failed: is_valid_pair(lo, hi)", in `new_checked`. A release build
   returns 2, which is correct (R9).
8. **K5**: `expanded(NaN)` trips `new`'s debug check. Every other margin, the infinities
   included, is crash-free and gives a valid interval (R0, 91 s).
9. **`serde_json` without `float_roundtrip`** parses floats to within one ulp. The derived
   `Cell` round trip moved a bound in 375,836 of 1,598,279 cells (R6). The tolerance in
   FR-018 is exactly one ulp.
10. **The corner** is `Interval { lo: PI, hi: f64::from_bits((-PI).to_bits() - 1) }`. Its
    true length is `2^−51 = f64::from_bits(0x3CC0_0000_0000_0000)`. Upstream's arithmetic
    gives −1. It is reachable as `Interval::from_point_pair(PI, next_up(−PI))`.
11. **The exhaustive projection check** (R8): `k ∈ [0, 2^30]`, the library's
    `coords::st_to_uv(coords::ij_to_st_min(k))` against the restated formula, and
    `si_ti_to_st(2k) == ij_to_st_min(k)`. It took 4.3 s in a release build and 88 s in a
    debug build (contended), with 0 differences.
12. **The pinned upstream** is `/home/t/sources/s2geometry` (`v0.14.0-40`). It has the
    `GetLength` corner (`s1interval.cc:41–47`), the normalising constructor that D-005's
    fix copies (`s1interval.cc:155–158`), and `GetCapBound` (`s2latlng_rect.cc:323`, D-006).

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: the `values` feature with its empty modules, the Makefile entry points, and the
records directory (research R2; contracts/verify-commands.md,
contracts/harness-conventions.md).

- [X] T001 Add the `values` feature and the empty modules to `verify/Cargo.toml` and `verify/src/lib.rs` (research R2).
  - **`verify/Cargo.toml`:**
    - `default = ["cellunion", "decoding", "values"]` and `values = []`;
    - a comment: the value-type proofs (spec 004), selected by `--harness cl_` (`Cell`) and
      `--harness iv_` (`s1::Interval`);
    - update the header comment to name spec 004.
  - **New files**, each starting with the two SPDX lines and
    `// Written for this crate (not ported from upstream S2).`, with a one-line `//!` naming
    its data-model section or property area:
    - `verify/src/cell_spec.rs` and `verify/src/cell_spec_tests.rs` (data-model §1.1);
    - `verify/src/arc_spec.rs` and `verify/src/arc_spec_tests.rs` (data-model §1.2);
    - `verify/src/value_domain.rs` (§2);
    - `verify/src/laws.rs` (§4.3);
    - `verify/src/cl_{crash,id,chd,cnt,bnd,eq}.rs` and
      `verify/src/iv_{crash,val,mem,rel,set,mea,exp}.rs`.
  - **`verify/src/lib.rs`:**
    - `pub mod cell_spec;` and `pub mod arc_spec;`, unconditional, next to
      `pub mod wire_spec;`;
    - `#[cfg(test)] #[path = "cell_spec_tests.rs"] mod cell_spec_tests;` in `cell_spec.rs`,
      and the same for `arc_spec`;
    - `laws` behind `#[cfg(kani)]` only (harness-conventions: every suite may use it);
    - `value_domain` and the `cl_*` and `iv_*` modules behind
      `#[cfg(all(kani, feature = "values"))]`;
    - extend the crate doc to name spec 004 and link `values/INVENTORY.md` and
      `values/PROPERTIES.md`.
  - **Checks:** `cd verify && cargo build && cargo test`;
    `cargo build --no-default-features`; `cargo kani --only-codegen`. Codegen needs no cap.
- [X] T002 [P] Add `make verify-values [P=cell|interval]` and the `F=values` part of `make verify-slow` to `Makefile` (contracts/verify-commands.md).
  - `VALUES_HARNESSES = $(if $(filter cell,$(P)),--harness cl_,$(if $(filter interval,$(P)),--harness iv_,--harness cl_ --harness iv_))`,
    in the style of `DECODING_HARNESSES`.
  - `verify-values: verify-kani-check`, with a `##` help text naming spec 004 and `[P=cell|interval]`.
    Its body runs `cd verify && cargo test`, then
    `cd verify && ./run-verify.sh $(VALUES_HARNESSES)`.
  - In `SLOW_HARNESSES`, add `F=values` → `$(VALUES_HARNESSES)`. Add `values` to
    `SLOW_ENV`'s filter, so it gets an 8G ceiling and one job, like `union`. Extend the
    comment above `SLOW_HARNESSES`, and `verify-slow`'s help text `[F=…|values]`.
  - Update the section header comment (`# ===== Formal verification …`) to name spec 004.
  - **Check:** `make help | grep verify-values`; `make -n verify-values P=cell`.
- [X] T003 [P] Create the records skeleton in `verify/values/` and register the catalogue in `verify/tests/property_kinds.rs` (research R2; data-model §4, §5, §8).
  - `verify/values/INVENTORY.md`: header and column legend copied from
    `verify/decoding/INVENTORY.md`, with two empty tables, "`Cell` (53 entries)" and
    "`s1::Interval` (31 entries)".
  - `verify/values/PROPERTIES.md`: header copied from `verify/decoding/PROPERTIES.md`, with
    sections 4.1 `Cell`, 4.2 the interval and 4.3 the generic laws, and no rows yet.
  - `verify/values/EVALUATION.md`: the section headings of data-model §8 and FR-016.
  - `verify/tests/property_kinds.rs`: `CATALOGUES` becomes a `[&str; 4]` that includes
    `"values/PROPERTIES.md"`.
  - **Check:** `cd verify && cargo test --test property_kinds` passes, since there are no
    rows yet.

**Checkpoint**: `make verify-values` runs, with zero harnesses, and passes. `cargo test` in
`verify/` passes.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: the C++ reference vectors, the two reference descriptions and their sanity
tests, the exhaustive projection check, the generators and the generic laws.

**⚠️ CRITICAL**: No user-story work starts until this phase is complete.

- [X] T004 Write the generator `tools/refgen-cpp/generate_values.cc` and wire it in `tools/refgen-cpp/CMakeLists.txt` (research R10).
  - **Style:** follow `generate_coding.cc`: SPDX header, a column comment, a fixed seed, and
    floats as 16-hex-digit bit patterns.
  - **`# S1INTERVAL_OPS`:** about 5,000 rows. Random valid intervals: include struct
    literals made with `S1Interval::FromPointPair` and the two-argument constructor, with
    inverted, single-point, empty and full ones. Add a random point in [−π, π], a margin and
    an error bound.
    - Per row, record every in-scope operation's result: `is_valid`, `is_empty`, `is_full`,
      `is_inverted`, `GetCenter`, `GetLength`, `Complement` (bounds), `GetComplementCenter`,
      `Contains(p)`, `InteriorContains(p)`, `Project(p)` (only for non-empty), `AddPoint(p)`,
      `Expanded(margin)`.
    - For a second random interval `y`: `Contains(y)`, `InteriorContains(y)`,
      `Intersects(y)`, `InteriorIntersects(y)`, `Union(y)`, `Intersection(y)`,
      `GetDirectedHausdorffDistance(y)`, `ApproxEquals(y, max_error)`.
    - Never call a function outside its `ABSL_DCHECK` domain (K3).
  - **`# S1INTERVAL_SPECIAL`:** the same columns for these inputs:
    - upstream's test fixtures (`s1interval_test.cc`: the quadrants and their unions,
      `pi`, `mipi`, `zero`);
    - both forms of the seam point;
    - the K4 corner `S1Interval(M_PI, nextafter(-M_PI, 0))` and its neighbours;
    - margins 0, ±π, ±2π, 3.2 and the smallest subnormal;
    - `y = [0, 1e-17]` against `x = [1, 2]` (D-005).
    - Mark every row whose expected value depends on the corner's length with note
      `deviation:seam-length` (FR-020): the corner's `GetLength`, `Expanded` and
      `ApproxEquals`.
  - **`# CELL_FIELDS`:** about 5,000 rows. A random valid id at a random level 0–30, with:
    - `face`, `level`, `orientation`;
    - the four `GetBoundUV` bounds as bit patterns;
    - `GetUVCoordOfEdge(k)` for k = 0..3;
    - `GetSizeIJ`, `is_leaf`.
  - **`# CELL_CHILDREN`:** about 1,000 rows. A random non-leaf id, and for each child `k`
    from `Subdivide`: its id, its orientation, and the four bound bit patterns.
  - **`CMakeLists.txt`:** `add_executable(generate_values generate_values.cc)` with
    `target_link_libraries(generate_values PRIVATE s2::s2)`.
- [X] T005 Rebuild the C++ reference install, build and run the generator, and commit `core/tests/data/cpp_values_vectors.csv` (depends on T004).
  - **Rebuild** `/tmp/s2-install` per `tools/refgen-cpp/README.md` ("Build the C++
    reference"), from `/home/t/sources/s2geometry` at the pinned commit. Check
    `git -C /home/t/sources/s2geometry describe --tags`.
  - **Build and run:**
    `cmake -B tools/refgen-cpp/build -DCMAKE_BUILD_TYPE=Release -DCMAKE_PREFIX_PATH=/tmp/s2-install`;
    `cmake --build tools/refgen-cpp/build --target generate_values`;
    `tools/refgen-cpp/build/generate_values > core/tests/data/cpp_values_vectors.csv`.
  - Run it twice and `cmp` the outputs: they must be deterministic.
  - **`tools/refgen-cpp/README.md`:** add a section "Value types (`generate_values`)" with a
    table of the four sections, their columns, and the Rust methods they check.
  - **Check:** the CSV is under 5 MB.
- [X] T006 [P] Write the reference description of a cell in `verify/src/cell_spec.rs` (data-model §1.1; research R3).
  - `CellModel { face, level, orientation, i_lo, j_lo, size }` from `spec::Decomposed`, via
    `spec::face_ij_orientation` and `spec::size_ij`.
  - `child_model(d, k)`, using `spec::child` and `spec::POS_TO_ORIENTATION`.
  - `st_to_uv(s)`, written from `s2coords.h:324–327` in upstream's operation order. Cite the
    lines. Do not call `s2rst::s2::coords`.
  - `ij_to_st_min(k)` and `uv_bounds(m) -> [[f64; 2]; 2]`.
  - `contains(a, b)` and `intersects(a, b)` over `Decomposed`, by delegating to
    `spec::contains`.
  - `consistent(c: &Cell, d: Decomposed) -> bool`, comparing all fields and the bounds bit
    for bit.
- [X] T007 [P] Write the hand-made unit tests in `verify/src/cell_spec_tests.rs` (Constitution III).
  - The six face cells: bounds `[−1, 1]²`, the orientation of each face.
  - A level-1 cell's children and orientations, against upstream's `s2cell_test.cc`
    expectations.
  - `st_to_uv(0) = −1`, `st_to_uv(0.5) = 0`, `st_to_uv(1) = 1`.
- [X] T008 [P] Write the reference description of an arc in `verify/src/arc_spec.rs` (data-model §1.2; research R3, R5).
  - The functions `in_range`, `canon`, `valid`, `empty`, `full`, `inverted`, `member`,
    `interior`, `key`, `contains`, `interior_contains`, `intersects`, `interior_intersects`
    (the real-arc relations; `witnesses`/`succ`/`pred` in the first draft, see T010), `upstream_length`,
    `ulps_apart`, `CORNER` and `ULP_PI`, exactly as data-model §1.2 defines them.
  - `upstream_length`'s doc cites `s1interval.cc:41–47` and says it is a restatement on
    purpose: P-IMEA-01's statement is "upstream's arithmetic" (plan, Complexity Tracking).
  - `DEV_SEAM_LENGTH`: a named predicate `is_corner(lo, hi)`, kept apart from the upstream
    meaning, as `wire_spec`'s deviations are.
  - Never call `s2rst::s1::Interval`'s methods. The type may be used for its fields only.
- [X] T009 [P] Write the hand-made unit tests in `verify/src/arc_spec_tests.rs`.
  - Upstream's `s1interval_test.cc` fixtures: membership of `0`, `π`, `−π` and the quadrant
    bounds in each fixture interval.
  - The `IntervalOps` table's containment and intersection cases, through `member` and
    real-arc relations, including `[next_up(−π), π]` not containing the full circle.
  - `upstream_length(CORNER) == −1.0`.
  - `valid` for the 8 special bit patterns: ±0, ±π, NaN, ±∞, and `next_up(−π)`.
- [X] T010 Write `verify/tests/values_spec_vs_cpp.rs`: both descriptions against every row of `core/tests/data/cpp_values_vectors.csv` (depends on T005, T006, T008; research R10).
  - `S1INTERVAL_*`: `member`, `interior` and `upstream_length` against `Contains`,
    `InteriorContains` and `GetLength`. The rows marked `deviation:seam-length` are checked
    against upstream's value too, because the description is upstream's.
  - `CELL_FIELDS` and `CELL_CHILDREN`: `CellModel` and `uv_bounds` against the C++ fields
    and bounds, bit for bit.
  - Parse the file the way `verify/tests/wire_spec_vs_cpp.rs` does.
  - **Check:** `cd verify && cargo test --test values_spec_vs_cpp`.
- [X] T011 [P] Write the exhaustive projection check `verify/tests/uv_projection.rs` (research R8; fact 11).
  - For every `k` in `0..=1 << 30`:
    - `coords::st_to_uv(coords::ij_to_st_min(k)).to_bits() == cell_spec::st_to_uv(cell_spec::ij_to_st_min(k)).to_bits()`;
    - `coords::si_ti_to_st(2 * k as u32).to_bits() == coords::ij_to_st_min(k).to_bits()`.
  - Count and report the differences, and assert there are none.
  - Measure it in a debug build. If it takes over 120 s, give the test binary
    `[profile.test] opt-level = 3` for `s2rst` only, or put the test behind `#[ignore]` with
    a `make verify-values` step that runs it with `--release --ignored`. Record the choice
    and the time in `verify/values/EVALUATION.md`.
- [X] T012 Write the generators in `verify/src/value_domain.rs` and the generic laws in `verify/src/laws.rs` (contracts/harness-conventions.md; data-model §2, §4.3).
  - **`value_domain.rs`:** `any_cell()`, `any_cell_at(L)`, `any_interval()`, `any_point()`,
    `any_f64()` and `any_edge()`, as the generator table defines them. `any_cell*` return
    the `Decomposed` too.
  - **`laws.rs`:**
    - **`laws::region`:**
      - `contains_implies_intersects<R: Region>(r: &R, c: &Cell)`;
      - `covering_covers<R: Region>(r: &R, c: &Cell)`: if `r.contains_cell(c)`, some id in
        `r.cell_union_bound()` contains `c.id()`. Iterate over a concrete maximum: for
        `Cell`, the bound has exactly one id (fact from 002: never compare collections
        with `==`);
      - `all<R: Region>(r, c)`, calling both.
    - **`laws::eq`:** `reflexive`, `symmetric`, `transitive` and `all<T: PartialEq>(a, b, c)`.
    - Each function's doc says which catalogue entries instantiate it.
  - **Check:** `cd verify && cargo kani --only-codegen`.

**Checkpoint**: `cd verify && cargo test` passes, with both descriptions' sanity tests and
the exhaustive projection check. The CSV is committed.

---

## Phase 3: Library Changes and Tests (FR-017 to FR-020, D-005, the generic tests with D-006), no story

**Purpose**: put `core/` into the state the proofs are stated against. Each change is its
own commit, with its tests. Tests come first and must fail before the fix (Constitution III).

- [X] T013 Commit the generic trait property tests and D-006's fix, already in the working tree, as two separate commits (research R15).
  - **First commit:**
    - `core/tests/trait_property_tests.rs`, which has the SPDX header for original tests
      and came from `tb/python-bindings/1` at `f9805f8`;
    - `CORE_TRAITS.md`, tracked for the first time. Update its "Generic Property-Test
      Harness" section to name the file's actual location, and list the Kani twin
      `verify/src/laws.rs`. Bring its `Region` invariants in line with the current
      supertraits, as `CORE_TRAITS_PROPTEST.md`'s note asks ("Super traits etc changed").
      Leave `CORE_TRAITS_PROPTEST.md` untracked: it is a working note.
    - **Before committing**, revert `core/src/s2/rect.rs` to `HEAD` temporarily and show
      that `region_union_cap_contains` fails. Then restore the fix.
  - **Second commit:** `core/src/s2/rect.rs`, D-006's fix and the `rect_tests` wiring, with
    `core/src/s2/rect_tests.rs`.
    - The message names D-006, the failing input, the upstream line
      (`s2latlng_rect.cc:323`) and the Go origin of the bug (`math.Remainder` ported as
      `rem_euclid`).
  - **Checks:** `cargo test -p s2rst --all-features --test trait_property_tests`;
    `cargo test -p s2rst --lib s2::rect`; `make clippy`; `make fmt-check`.
- [X] T014 [P] Add `Cell`'s missing instances of the integer `Region` laws to `core/tests/trait_property_tests.rs` (research R15; data-model §4.3).
  - In the generic `prop` module, add `region_cell_union_bound_covers<R: Region>(r, c)`: if
    `r.contains_cell(c)`, some id in `r.cell_union_bound()` contains `c.id()`.
  - Add the `#[quickcheck]` wire-ups `region_cell_contains_implies_intersects`,
    `region_cell_union_bound_covers` and `region_cell_deterministic`. Use the existing
    `generators::cell` for both the region and the cell argument; vary the levels so that
    containment occurs.
  - **Check:** the three pass with `QUICKCHECK_TESTS=5000`.
  - Commit with T013's first commit, or as its own commit right after it.
- [X] T015 [P] Document K1 in `core/src/s2/cell.rs`, FR-017 (research R7).
  - Add a `# Panics` section to `Cell::from_cell_id`, `Cell::from_face`,
    `Cell::from_face_pos_level` and `impl From<CellId> for Cell`.
  - The identifier must be a valid cell. The null identifier, an identifier with face 6 or
    7, or one without a level marker panics. The message differs between debug and release
    builds.
  - For `from_face` and `from_face_pos_level`, say that they panic through `CellId`'s
    constructors, which document it.
  - **Check:** `make doc`.
- [X] T016 [P] Document K3 and K5 in `core/src/s1/interval.rs`, FR-019 (research R7).
  - Add a `# Panics` section to `new`, `from_point_pair`, `contains`, `interior_contains`,
    `add_point` and `project`:
    - in debug builds, each panics when a point lies outside [−π, π] (and `project` also on
      an empty interval);
    - in release builds, `new` returns an interval that is not valid, and the others return
      an unspecified result.
  - `expanded` (K5): in debug builds a NaN margin panics, through `new`'s check, and in
    release builds it returns an interval that is not valid. Every other margin, including
    the infinities, is accepted.
  - Use the existing `# Panics` wording of `bound` as the style.
  - **Check:** `make doc`.
- [X] T017 Write the FR-020 and D-005 regression tests in the new sibling file `core/src/s1/interval_tests.rs`, and wire it in `core/src/s1/interval.rs` with `#[cfg(test)] #[path = "interval_tests.rs"] mod interval_tests;` (research R5, R9; Constitution III).
  - The header follows `core/src/s2/cell_id_tests.rs`.
  - **FR-020** (fact 10): the corner from `Interval::from_point_pair(PI, next_up(−PI))`:
    - `length() == f64::from_bits(0x3CC0_0000_0000_0000)`;
    - `expanded(3.2).is_full()`;
    - `!Interval::empty().approx_eq_with(corner, 0.0)`;
    - `Rect::new(r1::Interval::new(0.0, 0.1), corner).area() > 0.0`.
  - **D-005** (fact 7): `Interval::new(1.0, 2.0).directed_hausdorff_distance(Interval::new(0.0, 1e-17)) == 2.0`.
    It must not panic in the debug test build.
  - **Run** `cargo test -p s2rst --lib s1::interval::interval_tests` and confirm the FR-020
    tests and the D-005 test fail. Record the failure output for the commit messages.
- [X] T018 Fix D-005 in `core/src/s1/interval.rs` (depends on T017; research R9).
  - In `directed_hausdorff_distance`, build both helper arcs with `Interval::new(…)`
    instead of `Interval::new_checked(…)`, as upstream's `S1Interval(lo, hi)` does
    (`s1interval.cc:155–158`). Add a comment citing it.
  - **Checks:** T017's D-005 test passes. `cargo test -p s2rst --all-features` passes.
  - Commit on its own. The message names D-005, the input, and "release results unchanged"
    (R9's argument).
  - Add D-005 under "Defects found" in `verify/values/EVALUATION.md`.
- [X] T019 Fix FR-020 in `core/src/s1/interval.rs` (depends on T017; research R5).
  - In `length`: keep upstream's three lines. Replace the final `else { -1.0 }` with this:
    if `self.is_empty()`, return `-1.0`; otherwise return `(self.hi + PI) - (self.lo - PI)`.
  - Extend the rustdoc with a "Deviation from upstream" paragraph:
    - the one-ulp interval across the seam, `[π, next_up(−π)]`, has length `2^−51`, where
      upstream's `GetLength` returns −1;
    - every other interval's length is bit for bit upstream's, proved by P-IMEA-01;
    - the reason.
  - **Python:** add `test_s1interval_seam_corner_length` to `python/tests/test_interval.py`.
    `S1Interval.from_point_pair(math.pi, math.nextafter(-math.pi, 0.0)).length()` equals
    `2.0 ** -51`.
  - **Checks:**
    - T017's FR-020 tests pass;
    - the existing `prop_length_non_negative_for_non_empty` still passes, unchanged;
    - `cargo test -p s2rst --all-features`;
    - `make -C python test`.
  - Commit on its own.
- [X] T020 [P] Record FR-020's upstream side in `UPSTREAM.md` (research R1).
  - Move the `S1Interval::GetLength` row out of "Other upstream behaviours that s2rst
    inherits" into a new numbered section, `## U-007: S1Interval::GetLength at the seam`.
    Include:
    - the upstream lines (`s1interval.cc:41–47`) and the callers that inherit the wrong
      value (`:213`, `:218`, `:268`, `:286–295`, `s2latlng_rect.cc:94–99`);
    - the input and the observed −1;
    - "s2rst: fixed as a documented deviation (spec 004 FR-020)";
    - "Follow-up: T053".
  - Add U-007 to the summary table at the top.
  - In U-004, add one sentence: the constructor `S2Cell(S2CellId)` itself (`s2cell.cc:64–71`)
    accepts the same identifiers, and s2rst's `Cell::from_cell_id` panics on them, as
    documented by FR-017.
- [X] T021 Write the FR-018 regression tests in the new sibling file `core/src/s2/cell_tests.rs`, and wire it in `core/src/s2/cell.rs` (research R6; Constitution III).
  - The tests, all under `#[cfg(feature = "serde")]`:
    - the K2 edit is rejected: the JSON of a level-5 cell with `"level":5` replaced by
      `"level":12`;
    - an invalid id is rejected: id 0, and an id with face 6;
    - a face or orientation that differs from the identifier's is rejected;
    - a u-v bound 2 ulps from the identifier's is rejected, and one 1 ulp away is
      accepted and comes back exact;
    - the round trip: every cell to level 7 on all faces, plus 2,000,000 pseudo-random
      deeper cells (xorshift, fixed seed). `serde_json::to_string`, then `from_str`, gives
      the original cell's fields and bounds bit for bit (SC-009). If this takes over 10 s in
      a debug build, keep levels ≤ 5 and 100,000 random cells in the default test, and put
      the full set in an `#[ignore]` test that the quickstart runs with
      `--release --ignored`.
  - **Run** them and confirm all fail except "1 ulp away is accepted" (the derived
    `Deserialize` accepts everything).
- [X] T022 Implement FR-018's `Deserialize` for `Cell` in `core/src/s2/cell.rs` (depends on T021; research R6).
  - Keep `#[cfg_attr(feature = "serde", derive(serde::Serialize))]`. Remove the derived
    `Deserialize`.
  - Add `#[cfg(feature = "serde")] impl<'de> serde::Deserialize<'de> for Cell`, in the
    style of `CellUnion`'s (`core/src/s2/cell_union.rs:638`):
    - a private shadow struct `#[derive(serde::Deserialize)] #[serde(rename = "Cell")]`
      with the same five fields and types, so the format is unchanged;
    - reject `!id.is_valid()` with "cell id is not a valid cell";
    - build `c = Cell::from_cell_id(id)`;
    - reject a face, level or orientation that differs from `c`'s, naming the field;
    - reject any of the four u-v bounds whose `to_bits()` differs from `c`'s by more than 1
      (adjacent floats of the same sign; treat different signs as more than 1 unless both
      are zeros);
    - return `c`.
  - The rustdoc on the impl states the rule, the one-ulp reason (text formats that parse
    floats to within one ulp), and that the rule matches the binary decoder's
    (`DEV_VALID_ID`, spec 003).
  - **Checks:** T021's tests pass; `cargo test -p s2rst --all-features`; `make clippy`;
    `make doc`.
  - Commit on its own.
- [X] T023 Write the differential tests `core/tests/cpp_values_diff.rs` against every row of `core/tests/data/cpp_values_vectors.csv` (depends on T005, T018, T019; research R10).
  - **Intervals:** every column of `S1INTERVAL_*`, bit for bit for floats. Rows noted
    `deviation:seam-length` are checked against the documented values: the length `2^−51`,
    `Expanded` and `ApproxEquals` as computed with that length.
  - **Cells:** every column of `CELL_FIELDS` and `CELL_CHILDREN`, bit for bit.
  - **Header:** the original-test SPDX header, plus a doc naming spec 004 FR-021.
  - **Check:** `cargo test -p s2rst --all-features --test cpp_values_diff`.
  - A row that fails for a reason other than a documented deviation is a defect (known-defect
    rule) or an upstream finding (upstream findings rule). Stop and follow the rule.

**Checkpoint**: `make test` passes. The corner's length is one ulp, D-005 and D-006 are
fixed, `Cell` deserialises only consistent cells, and the generic trait tests run on this
line.

---

## Phase 4: User Story 1 — Proven crash freedom (Priority: P1) 🎯 MVP

**Goal**: Every in-scope operation of both types is proven free of panics, overflow,
undefined behaviour and firing debug checks over its documented domain (FR-004), with covers
showing the domains non-empty and the outcomes reachable (FR-009).

**Independent Test**: `make verify-values` reports every crash harness `SUCCESSFUL` with all
covers satisfied. `verify/values/INVENTORY.md` names a harness for every in-scope row.

- [X] T024 [P] [US1] Write the `Cell` crash harnesses in `verify/src/cl_crash.rs` (data-model §5.1; contracts/harness-conventions.md).
  - One `cl_crash::c_<op>` for each in-scope entry that no property harness covers.
  - **The entries:** `from_cell_id`, `from_face` (faces 0–5), `from_face_pos_level` (valid
    positions and levels), `From<CellId>`, `Default`, `face`, `level`, `id`,
    `orientation`, `is_leaf`, `size_ij`, `children`, `contains_cell`, `intersects_cell`,
    `cell_union_bound`, the three `Region` methods, `PartialEq`, `bound_uv`,
    `uv_coord_of_edge`.
  - Group cheap accessors into one harness per group where 001 did. Each harness's doc
    starts with `C-Cell::<op>:` and states the domain.
  - **Covers:** a leaf cell; a cell with children; each face; each orientation.
  - Every harness has `#[kani::unwind(9)]`.
- [X] T025 [P] [US1] Write the interval crash harnesses in `verify/src/iv_crash.rs` (data-model §5.2; research R9).
  - One `iv_crash::c_<op>` for each of the 30 in-scope entries, over the FR-004 domains:
    - `ValidInterval`;
    - `Point` for the debug-checked point operations;
    - non-empty for `project`;
    - `i ≤ 1` for `bound`;
    - every non-NaN margin for `expanded` (K5, documented by T016);
    - every `f64` for `approx_eq_with`'s error bound.
  - `c_directed_hausdorff_distance` passes only after T018. If you run it earlier, list it
    as `iv_crash::c_directed_hausdorff_distance D-005` in `verify/known-defects.txt`, and
    remove the line once T018 lands.
  - **Covers:** inverted and non-inverted, empty, full and single-point intervals; the
    corner; `y = [0, 1e−17]` reaching the helper arcs (D-005's path).
  - The probe's `r0_approx_eq_any_error`, `r1_expanded_non_nan` and `r1_dhd_crash` are the
    drafts.
- [X] T026 [US1] Run `make verify-values` and fill in the crash columns of `verify/values/INVENTORY.md` (depends on T024, T025).
  - **All 84 rows:** name, scope, reason, domain, crash result, crash harness.
  - **The 32 excluded `Cell` rows**, with the reasons in data-model §5.1. The float-geometry
    row points at `CORE_TRAITS.md` and the `prop_cell_*` and `region_cell_*` tests (analysis
    C3).
  - **`Display`**, for both types: a tool limit, as 001's formatter.
  - **The binary codec of `Cell`:** a pointer row to `verify/decoding/INVENTORY.md`.
  - Record each harness's time and peak memory from `verify/target/run-verify/results.tsv`.

**Checkpoint**: US1 is complete and shippable. SC-001 and the crash half of SC-002 hold.

---

## Phase 5: User Story 2 — Proven meaning of a cell and of a circular interval (Priority: P2)

**Goal**: The 40 catalogue properties (data-model §4) are proven against the reference
descriptions, or recorded with a reason. Each proven one has a mutant that breaks it.

**Independent Test**: `make verify-values` reports every property harness `SUCCESSFUL`.
`make verify-mutants` reports every `M-3NN` killed. `cargo test --test property_kinds`
passes.

### `Cell` properties

- [X] T027 [P] [US2] Write P-CID-01, P-CID-02 and P-CID-03 in `verify/src/cl_id.rs` against `cell_spec::CellModel` (data-model §4.1).
  - The pilot's `cell_reports_its_ids_face_level_and_leafness` is the draft. Assert the
    orientation against the model, not `< 4`.
  - P-CID-03 covers the face constructor, the face-position-level constructor, `From` and
    `Default`.
- [X] T028 [P] [US2] Write P-CCHD-01 in `verify/src/cl_chd.rs` against `cell_spec::child_model` (data-model §4.1).
  - The pilot's `children_agree_with_cell_id_children` is the draft. Assert all four
    children's ids, faces, levels and orientations against the model.
- [X] T029 [P] [US2] Write P-CCNT-01, P-CCNT-02 and P-CCNT-03 in `verify/src/cl_cnt.rs` (data-model §4.1, §4.3).
  - P-CCNT-01: `contains_cell` and `intersects_cell` against `cell_spec::contains` and
    `intersects`.
  - P-CCNT-02: the `Region` methods against the inherent ones; `cell_union_bound` is exactly
    `[self.id()]`.
  - P-CCNT-03: `laws::region::all(&a, &b)` for two `any_cell()` values. Its doc says it is
    the generic law's `Cell` instance.
- [X] T030 [P] [US2] Write P-CEQ-01 and P-CEQ-02 in `verify/src/cl_eq.rs` (data-model §4.1, §4.3).
  - P-CEQ-01: `laws::eq::all(&a, &b, &c)`, plus `(a == b) == (a.id() == b.id())`.
  - P-CEQ-02: two cells built from the **same** symbolic id (one value, used twice) have
    identical fields and bound bits.
- [X] T031 [US2] Write P-CBND-03 in `verify/src/cl_bnd.rs`, and the `__l<L>` families of P-CBND-01, P-CBND-02 and P-CCHD-02 (research R8; fact 1; harness-conventions "Split families").
  - **P-CBND-03:** whole domain (R0: 26 s); the probe's `r0_cell_uv_edge` is the draft.
  - **The families:** one `macro_rules!` per property, generating `__l00 … __lL` with
    `any_cell_at(L)`. The probe's `bound_model_at`, `uv_ordered_at` and `children_uv_at` are
    the drafts. P-CBND-01 compares with `cell_spec::uv_bounds`, not with the library's
    `coords`.
  - **Measure** each member with `make verify-harness H=…`, one at a time, under 4 jobs or
    fewer:
    - members under 2 minutes are per change;
    - members up to the 60-minute weekly limit carry `#[cfg(feature = "slow")]`;
    - stop at the first level that times out at 60 minutes, and do not write that member or
      any above it.
  - **Record** each property's reach in `verify/values/PROPERTIES.md` as **proven within
    stated bounds (levels 0–L)**, and the levels above as **not proven (tool limit)**, with
    the measurements. Note the exhaustive check (T011) and the vectors (T023) as the
    evidence above L.
- [ ] T032 [US2] Run `make verify-values P=cell`, then `make verify-slow F=values` for the weekly `__l<L>` members, and record times, memory and results in `verify/values/EVALUATION.md` ("Measurements") and the status column of `verify/values/PROPERTIES.md` (depends on T027–T031).

### Interval properties

- [X] T033 [P] [US2] Write P-IVAL-01 to P-IVAL-05 in `verify/src/iv_val.rs` (data-model §4.2, §4.3).
  - P-IVAL-01 runs over `AnyF64²`: `Interval { lo, hi }.is_valid() == arc_spec::valid(lo, hi)`.
  - P-IVAL-02 includes "the default is empty" (analysis I2). The pilot's
    `constructors_produce_valid_intervals` is the draft, with its tolerance for the arc
    through two points.
  - P-IVAL-05: `laws::eq::all` over three `any_interval()` values. Its doc states the
    signed-zero point.
- [X] T034 [P] [US2] Write P-IMEM-01 and P-IMEM-02 in `verify/src/iv_mem.rs` against `arc_spec::member` and `interior`.
- [X] T035 [P] [US2] Write P-IREL-01 to P-IREL-06 in `verify/src/iv_rel.rs` (data-model §4.2).
  - The soundness entries (01, 03, 04, 06) draw a symbolic point.
  - The exactness entries (02, 03, 04, 06) compare with `arc_spec`'s real-arc relations
    (data-model §1.2, as amended in T010).
  - P-IREL-05 is a model check that calls no library code: `member(x, p) ⇔ contains(x, [p, p])`,
    and `contains(x, y) ∧ member(y, p) ⇒ member(x, p)`, and the same for the interior.
  - The pilot's relation laws are drafts for the consistency parts.
- [X] T036 [P] [US2] Write P-ISET-01 to P-ISET-06 in `verify/src/iv_set.rs` (data-model §4.2).
  - Use the pilot's complement, add-point, union, intersection and projection harnesses as
    drafts, restated against `arc_spec`.
  - P-ISET-05 keeps the pilot's one-ulp tolerance for the arc through two points, and states
    it in ulps with the reason (FR-008).
- [X] T037 [US2] Write P-IMEA-01 to P-IMEA-06 in `verify/src/iv_mea.rs`, including the FR-020 containment proof (depends on T019; research R5, R9; facts 6, 10).
  - **P-IMEA-01 (exact output)**, one harness, two pipelines, with doc
    `P-IMEA-01 (exact output): the seam fix changes the length of exactly one valid
    interval, …`:
    - if `arc_spec::is_corner(x)`, `x.length() == ULP_PI`;
    - else `x.length().to_bits() == arc_spec::upstream_length(x).to_bits()`.

    The probe's `r1_len_2way` is the draft (217 s).
  - **P-IMEA-02 (law):** the length is non-negative for every non-empty interval, positive
    unless it is a single point, −1 for the empty interval and 2π for the full one.
  - **P-IMEA-03 (model check):** `r0_len_unique` (0.2 s), calling only `arc_spec`.
  - **P-IMEA-04 and P-IMEA-05:** from `r0_center` and `r0_complement_center` (30 s each).
  - **P-IMEA-06:** the 0 and π clauses whole; the range clause as `__yinv` and `__ynoninv`.
    If `__ynoninv` is over 12 minutes, split it further by `x`'s inversion
    (`__ynoninv_xinv`, `__ynoninv_xnoninv`). A member over 12 minutes but under 60 goes to
    the weekly tier. A member over 60 minutes leaves the range clause **not proven (tool
    limit)**, with the measurements.
- [X] T038 [US2] Write P-IEXP-01 and P-IEXP-02 in `verify/src/iv_exp.rs` (research R9).
  - **P-IEXP-01:**
    - the validity clause over every non-NaN margin (`r1_expanded_non_nan`, 91 s);
    - the containment clause, split by the margin's sign and by `x`'s inversion
      (`__mpos_xinv`, `__mpos_xnoninv`, `__mneg_xinv`, `__mneg_xnoninv`).
    - Tier its members as in T031. If no member fits within 60 minutes, record the
      containment clause as **not proven (tool limit)**.
    - With T019 in place, the corner must no longer be a counterexample (R0 found it as
      one).
  - **P-IEXP-02:** reflexivity (`r0_approx_eq_reflexive`, 254 s), symmetry, and
    `approx_eq == approx_eq_with(·, 1e-15)`.
- [ ] T039 [US2] Run `make verify-values P=interval` and the weekly members, and record times, memory and results in `verify/values/EVALUATION.md` ("Measurements") and the status column of `verify/values/PROPERTIES.md` (depends on T033–T038).

### Non-vacuity, catalogue and index

- [X] T040 [US2] Write the `M-3NN` mutant patches in `verify/mutants/` and run them (data-model §7; research R11; contracts/harness-conventions.md "Mutant patches").
  - **The patches:** about 34 in 001's format (`Mutant`, `Targets`, `Expect-Fail`,
    `Defect`), one defect each, numbered from `M-301`. They include the FR-020 pair:
    - `M-3NN-seam-fix-widened`: the stable form for every inverted interval. It must fail
      `iv_mea::p_imea_01`;
    - `M-3NN-seam-fix-removed`: it must fail `p_imea_01` and `p_imea_02`.
  - Add `Tier: slow` to mutants whose `Expect-Fail` harnesses are weekly members.
  - **The model-check mutants** patch `verify/src/arc_spec.rs`, not `core/`:
    - `member` ignoring the seam: P-IMEM-01 fails;
    - `contains` without its `ccw_le(x.lo, y.hi, x.hi)` clause: P-IREL-05 fails.

    Check that `run-mutants.sh` accepts patches against `verify/`. If not, extend it to
    apply them in the throwaway worktree too.
  - **Run** `make verify-mutants M=M-3NN` for each, one at a time. Add a row per patch to
    `verify/MUTANTS.md`.
  - A mutant that survives means the harness is vacuous or too weak. Strengthen the harness
    until the mutant is killed; never delete the mutant.
  - **Result (2026-10-13):** all 42 killed (`run-mutants: 42 killed, 0 survived, 0 error`,
    18:06–19:05, from the frozen worktree at `e60a044`), `M-336` and `M-337` with the 60-minute
    weekly limit. Recorded in `verify/MUTANTS.md`.
- [X] T041 [US2] Complete `verify/values/PROPERTIES.md` and `verify/PROPERTIES_BY_KIND.md`, and tag the harness docs (FR-010, SC-012).
  - **`PROPERTIES.md`:** all 40 rows, with 001's columns: id, kind, plain statement, exact
    statement, upstream source, domain, harness, status, mutants.
    - Statuses come from T032 and T039.
    - Section 4.3 lists the generic laws and their instances, and names the quickcheck
      twins in `core/tests/trait_property_tests.rs`.
  - **`PROPERTIES_BY_KIND.md`:** every entry under its kind, and the summary counts updated.
  - **Harness docs:** each property harness's doc starts with `P-ID (kind): <plain
    statement>`.
  - **Check:** `cd verify && cargo test --test property_kinds` passes.

### Found by the weekly proofs (FR-023, amendment 2026-10-13)

The weekly member `iv_exp::p_iexp_01_shrink__xinv` failed with a counterexample: upstream's
`Expanded` can cross its bounds when shrinking to almost nothing (U-011, D-007). The user chose
the fix in both directions.

- [X] T055 [US2] Fix `s1::Interval::expanded` (FR-023) in `core/src/s1/interval.rs`.
  - After upstream's arithmetic: `margin >= 0.0 && !result.contains_interval(self)` gives
    `Interval::full()`, `margin < 0.0 && !self.contains_interval(result)` gives
    `Interval::empty()`. A NaN margin passes neither test, so its documented release
    behaviour is unchanged.
  - Rustdoc: a containment sentence and a `# Deviation from upstream` section with the
    counterexample.
  - Regression tests failing without the fix: `test_expanded_crossing_bounds_give_empty`
    (Kani's counterexample and three from a search) and
    `test_expanded_keeps_containment_near_empty_and_full` in `core/src/s1/interval.rs`;
    `expanded_shrinking_longitudes_to_almost_nothing_is_empty` in
    `core/src/s2/rect_tests.rs`.
  - C++ vectors: the four inputs as the last `S1INTERVAL_SPECIAL` rows, noted
    `deviation:expanded-crossing` by C++'s own containment test (corner rows excluded, whose
    expansion is FR-020's). The earlier rows are unchanged. `core/tests/cpp_values_diff.rs`
    checks the library's empty result there.
  - `UPSTREAM.md` U-011, with C++ and Go observed on the four inputs.
- [X] T056 [US2] Prove the fix narrow: P-IEXP-03 in `verify/src/iv_exp.rs` against
  `arc_spec::upstream_expanded` (data-model §4.2).
  - `arc_spec::upstream_expanded` restates `s1interval.cc:208–224` from the fields only
    (FR-007), with the library's documented length (FR-020) and remainder. Check:
    `verify/tests/values_spec_vs_cpp.rs` matches C++'s `Expanded` bit for bit on every row
    off the corner, and flags exactly the four crossing rows.
  - Members `p_iexp_03__mneg` and `p_iexp_03__mpos`. Measure each; per change if under 12
    minutes, else weekly.
  - Mutants `M-343` (shrink guard removed, `Tier: slow`: Kani must find a crossing), `M-344`
    (interior containment), `M-345` (grow guard reversed). Run them after 004's campaign.
  - Re-run the four `p_iexp_01_{grow,shrink}__*` members after the fix. Where they still
    time out, record P-IEXP-01's containment clause as following from P-IEXP-03 and
    P-IREL-02.
  - Records: `verify/values/PROPERTIES.md` (P-IEXP-03; P-IEXP-01's status),
    `verify/PROPERTIES_BY_KIND.md`, `verify/values/EVALUATION.md` (D-007).

**Checkpoint**: US2 is complete. SC-003 and SC-004 hold, and the FR-020 containment proof
runs per change.

---

## Phase 6: User Story 3 — Regression protection for future changes (Priority: P3)

**Goal**: The values proofs run in CI on relevant changes, and stay out of the default gate
(FR-014, SC-005).

**Independent Test**: a change that breaks a proven property fails the matching `verify`
leg, naming the property. The revert passes. An unrelated change does not start the legs.

- [X] T042 [US3] Wire the values suite into `.github/workflows/verify.yml` (contracts/verify-commands.md "CI").
  - **The matrix:** add `values-cell` and `values-interval` to `suite`. The run step maps
    `values-*` to `make verify-values P="${SUITE#values-}"`, in the same `case` as
    `decoding-*`.
  - **Both `paths` lists:** add `core/src/s2/region.rs`, `core/src/r2/**`,
    `core/tests/data/cpp_values_vectors.csv` and `core/tests/trait_property_tests.rs`.
  - **The `slow` job's matrix:** add a `values` leg running `make verify-slow F=values`.
  - Update the header comment to name spec 004 and both new legs.
- [ ] T043 [US3] Measure both legs against SC-005, and move members to the weekly tier until each leg is under 10 minutes on the reference runner (depends on T042; research R12).
  - Measure locally with `VERIFY_JOBS=4 make verify-values P=cell`, and the same for
    `P=interval`. Then measure on CI with `workflow_dispatch`, which needs the user's push
    approval.
  - Never move P-IMEA-01, P-IMEA-02 or P-IMEA-03 out of the per-change tier (R12).
  - Record the times in `verify/values/EVALUATION.md`.
- [ ] T044 [US3] Check US3's acceptance scenarios with a deliberate break (quickstart §6).
  - Change `Cell::uv_coord_of_edge`'s `Right` arm to return `x.lo` on a scratch branch.
  - `make verify-values P=cell` fails, naming `cl_bnd::p_cbnd_03` with a concrete cell.
    Revert it.
  - On CI (needs push approval), the `values-cell` leg fails, and a change touching none of
    the paths does not start the leg.
- [X] T045 [P] [US3] Document the suite in `verify/README.md` and `CONTRIBUTING.md`.
  - `verify/README.md`: a "Values (spec 004)" section with `make verify-values`, the two
    parts, `F=values`, and the records directory.
  - `CONTRIBUTING.md`, "Formal verification": name `Cell` and `s1::Interval`. Before
    changing `cell.rs`, `region.rs` or `s1/interval.rs`, run `make verify-values`.

**Checkpoint**: CI runs the values legs on relevant changes. `make test` is unchanged.

---

## Phase 7: User Story 4 — Evaluation of floating-point value types (Priority: P3)

**Goal**: a self-contained evaluation that answers FR-016's questions (SC-007).

**Independent Test**: a maintainer who did not do the work can answer, from
`verify/values/EVALUATION.md` alone:

- continue with the value types, or stop?
- with which approach?
- are type-level invariants needed first?
- what comes next?

- [ ] T046 [US4] Write `verify/values/EVALUATION.md` (data-model §8; research R14).
  - **Effort and dates.**
  - **Measurements:** per harness, from T026, T032, T039 and T043.
  - **"Floating-point cost":** time against operation mix, comparisons and additions versus
    products, with R0's level curve.
  - **Findings:**
    - the survey's K1–K4;
    - the planning probe's K5, D-005 and the one-ulp JSON drift;
    - D-006 from the generic tests;
    - anything found during implementation.
  - **The comparison with 001–003.**
  - **The FR-016 questions:**
    - go or no-go for `r1::Interval`, the rectangle, `ChordAngle` and `Point`;
    - type-level invariants first or not (the struct-literal domain, which reached the
      corner);
    - the ranked follow-ups.
- [X] T047 [P] [US4] Extend `verify/TRUSTED_BASE.md` with a values section (research R13).
  - CBMC's IEEE-754 model.
  - The float functions Kani models on proof paths, each with the harnesses that reach it:
    `f64::midpoint`, `f64::round`, `abs`, `to_bits` and `from_bits`.
  - The reliance on 001's results and generators, and on 003's codec.
  - `upstream_length` as the one restatement.
  - The `__l<L>` splits and the levels left unproved, with the exhaustive check and the
    vectors as test evidence above L.
- [ ] T048 [P] [US4] Update `VERIFIED_TYPES.md`.
  - Move `Cell` and `s1::Interval` to "Types with proofs" (new §2.5), with their counts and
    tiers.
  - In §3, list the generic laws (`verify/src/laws.rs`) next to 003's `Codec`, with their
    instances, and the quickcheck twins.
  - Step 3's note on the length corner: it is now a documented deviation (FR-020), not an
    exception.
  - Add D-005 and D-006 to the findings.

**Checkpoint**: the evaluation answers SC-007's four questions.

---

## Phase 8: Polish & Cross-Cutting Concerns

- [ ] T049 Run the whole gate and the whole verification once (`Makefile` targets `test`, `verify`, `verify-mutants`, one after another), and record the run in `verify/values/EVALUATION.md` ("Final run").
  - Fix anything that fails.
  - Confirm `cargo test --test property_kinds`, `values_spec_vs_cpp` and `uv_projection`
    pass inside `make verify`.
- [ ] T050 [P] Walk through `specs/004-formal-verify-cell-interval/quickstart.md` §1–§7, and correct the quickstart where reality differs (command names, counts, times).
- [ ] T051 [P] Check the spec's success criteria SC-001 to SC-013 one by one against the records, and list the result per criterion in `verify/values/EVALUATION.md` ("Success criteria").
- [ ] T052 [P] Re-run `/speckit-analyze` on `specs/004-formal-verify-cell-interval/`, and resolve any CRITICAL or HIGH finding before the PR.

---

## Phase 9: Follow-ups (FR-022, FR-012) — recorded, not implemented in this feature

**Purpose**: Each task here records upstream behaviour to report, or work that belongs in a
separate change. Resolve each in a **separate change** after this feature.
`/speckit-implement` must not execute these tasks; it only appends new ones by the rules
above. The next free task ID is **T058**.

- [ ] T053 U-007: report `S1Interval::GetLength`'s seam corner upstream (research R1).
  - The report includes the input, the −1, the wrong `Expanded`, `ApproxEquals` and
    `S2LatLngRect::Area` results, and the narrow fix with its proof (P-IMEA-01, P-IMEA-03).
  - Send it together with the open U-004, U-005 and U-006 reports (memory note
    "s2rst-verify-next-type").
  - **Decision needed:** whether and when to report.
- [ ] T057 U-011: report `S1Interval::Expanded`'s crossed bounds upstream, to C++ and Go
  (FR-022).
  - The report includes Kani's counterexample
    (`[-0.4669189453124994, -2.7499821821795862]` shrunk by `-2.0000610351562496` gives
    `[1.5331420898437502, 1.53314208984375]`, length 2π), the three found by search, the
    cause (the 2·ε allowance against the rounding of `hi + margin` and the float `2π`), and
    the containment check with its proof (P-IEXP-03).
  - Send it together with the other open reports (T053; 005 T050–T052).
  - **Decision needed:** whether and when to report.
- [ ] T054 Instantiate `laws::region` for `CellUnion` within 002's size bounds, as `cu_mem::p_umem_06__n*`, and note in `verify/cell_union/PROPERTIES.md` that P-UMEM-05 implies it through the model (research R15).
  - It re-runs 002's proven, size-bounded members, which is why it is a follow-up.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: no dependencies. T002 and T003 are [P] with T001.
- **Phase 2 (Foundational)**: needs Phase 1.
  - T005 needs T004. T010 needs T005, T006 and T008.
  - T006–T009 and T011 are [P].
  - T012 needs T006 and T008, because the generators use the descriptions.
- **Phase 3 (library changes)**: needs Phase 1. It can run alongside Phase 2.
  - T013 first.
  - T014–T016 and T020 are [P].
  - T017 → T018 and T019. T021 → T022.
  - T023 needs T005, T018 and T019.
- **Phase 4 (US1)**: needs Phase 2's T012, and Phase 3's T016, T018 and T019 (the
  documented domains and fixed behaviour).
- **Phase 5 (US2)**: needs Phase 2 and Phase 3. T037 needs T019. The mutant run T040 needs
  every property harness.
- **Phase 6 (US3)**: needs the harnesses it runs (US1, US2).
- **Phase 7 (US4)**: needs US1–US3's measurements.
- **Phase 8 (Polish)**: needs everything above.

### User Story Dependencies

- **US1 (P1)**: independent after Phases 2–3. It is the MVP.
- **US2 (P2)**: needs US1's modules only for the shared generators. The property harnesses
  stand alone, and each one also discharges its operations' crash obligations.
- **US3 (P3)**: needs the suites it runs.
- **US4 (P3)**: needs everything it evaluates.

### Parallel Opportunities

- Phase 1: T002 and T003.
- Phase 2: T006, T007, T008, T009 and T011.
- Phase 3: T014, T015, T016 and T020. Then T017/T021 are in different files, and so are
  T018/T019 and T022 (but T018 and T019 edit one file: commit one, then the other).
- US1: T024 and T025.
- US2: T027–T030 and T033–T036 (different modules). The Kani runs go one after another.
- US4: T047 and T048.
- Polish: T050–T052.

---

## Parallel Example: User Story 2

```text
# After Phase 3, write these side by side; run them one at a time:
Task: "T027 [US2] P-CID-01..03 in verify/src/cl_id.rs"
Task: "T028 [US2] P-CCHD-01 in verify/src/cl_chd.rs"
Task: "T029 [US2] P-CCNT-01..03 in verify/src/cl_cnt.rs"
Task: "T033 [US2] P-IVAL-01..05 in verify/src/iv_val.rs"
Task: "T034 [US2] P-IMEM-01..02 in verify/src/iv_mem.rs"
Task: "T035 [US2] P-IREL-01..06 in verify/src/iv_rel.rs"
Task: "T036 [US2] P-ISET-01..06 in verify/src/iv_set.rs"
```

---

## Implementation Strategy

### MVP first (User Story 1 only)

1. Phase 1, then Phases 2 and 3 (they can overlap).
2. Phase 4 (US1): every in-scope operation proven crash-free, with D-005 fixed and K1, K3
   and K5 documented.
3. **Stop and validate:** `make verify-values` passes, and the inventory is complete.

### Incremental delivery

1. Setup, foundation and library changes: the corner fixed and documented, D-005 and D-006
   fixed, `Cell` deserialising only consistent cells, the generic trait tests on this line.
2. US1: crash freedom.
3. US2: meaning, with the containment proof per change and the mutants.
4. US3: CI.
5. US4: the evaluation.

Each step leaves `make test` and `make verify` green.

### Commit plan

Separate commits, in this order:

1. T013 (generic tests with `CORE_TRAITS.md`; then D-006), and T014;
2. T015 and T016 (documentation);
3. T017 with T018 (D-005);
4. T019 (FR-020, with the Python test) with T020 (U-007);
5. T021 with T022 (FR-018);
6. the vectors (T004, T005, T023);
7. the `verify/` suite;
8. the CI wiring;
9. the records.

Push only with the user's approval.
