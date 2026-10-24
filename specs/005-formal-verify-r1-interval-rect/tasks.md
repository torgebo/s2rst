---

description: "Task list for the formal verification of r1::Interval and s2::Rect (spec 005)"
---

# Tasks: Formal Verification of `r1::Interval` and `s2::Rect`

**Input**: Design documents from `specs/005-formal-verify-r1-interval-rect/`:

- plan.md, spec.md;
- research.md (R0–R13);
- data-model.md;
- contracts/ (verify-commands, harness-conventions);
- quickstart.md;
- the Kani probe `probe/r0_harnesses.rs`, whose harnesses are the first drafts of several
  tasks below.

The feature extends 004's `values` suite in `verify/`. 004's task list
(`specs/004-formal-verify-cell-interval/tasks.md`) shows the patterns, and its files are the
style reference:

- `verify/src/arc_spec.rs`, `iv_*.rs` and `cl_*.rs`;
- `verify/src/value_domain.rs` and `laws.rs`;
- `tools/refgen-cpp/generate_values.cc`;
- `core/tests/cpp_values_diff.rs`;
- `verify/tests/values_spec_vs_cpp.rs`.

**Prerequisites**:

- **Kani 0.68.0**: `make verify-kani-check` passes.
- **For T005 only**: the C++ reference install at `/tmp/s2-install` (present on 2026-10-13),
  built from `/home/t/sources/s2geometry` at `v0.14.0-40-g37a4501`. If it is gone, rebuild it
  with `cmake --install` from `/home/t/sources/s2geometry/build`
  (`tools/refgen-cpp/README.md`). The generator runs with
  `LD_LIBRARY_PATH=/tmp/s2-install/lib`.
- **Disk**: `df -h /` shows at least 20 GB free before any Kani run (95 GB on 2026-10-13).

**Tests**: The spec asks for these ordinary tests, so they appear as tasks:

- FR-020's regression tests, failing without the fix (Constitution III);
- the differential tests against the new C++ vectors (FR-021);
- the reference descriptions' unit tests and their sanity tests against the same vectors
  (FR-007).

The Kani harnesses are the verification deliverable itself, so they are implementation
tasks.

**Organization**: Tasks are grouped by user story (spec.md):

| Story | Priority | Phase |
|-------|----------|-------|
| US1: proven crash freedom | P1 | 4 |
| US2: proven meaning, the rectangle built on its intervals | P2 | 5 |
| US3: CI regression protection | P3 | 6 |
| US4: the evaluation of composite value types | P3 | 7 |

The library changes (FR-017 to FR-020) come first, in Phase 3, because the proofs' domains
rest on the documented behaviour (plan, Complexity Tracking).

## Progress (2026-10-13)

**Done** (marked `[X]`): Phases 1–3 (committed: the documents, the library changes, the vectors);
every harness written and compiling under Kani (64: 62 per change, 2 weekly); the 43 mutants;
the CI legs; the README, CONTRIBUTING and trusted-base sections; the inventory and catalogue
rows, generated with their statuses pending.

**First per-change run** (2026-10-13, after 004's weekly members; 4 jobs, 4 GiB, 12
minutes): 59 of 62 succeeded, every cover reached, no counterexample; 12 and 13 minutes for
the two parts. Two timed out:
- `ri_exp::p_rexp_03`: split by clause into `p_rexp_03_reflexive`, `_symmetric` and
  `_default`; `M-416` and `M-417` name the clause they break;
- `rl_exp::p_lexp_02_symmetric`: moved to the weekly tier (T035's rule). The longitudes'
  symmetry is spec 004's P-IEXP-02 clause, whose members time out at 60 minutes.

Now 66 harnesses: 63 per change, 3 weekly.

**Spec 004's weekly run found U-011** (004 FR-023): `s1::Interval::expanded` could cross its
bounds when shrinking, and `Rect::expanded` inherits the fix through its longitudes. The
weekly `p_lexp_01__mneg` checks the rectangle's side; `rect_tests.rs` has the regression
test.

**The runs on the fixed library** (2026-10-13, one at a time):
- **Per change** (4 jobs, 4 GiB, 12 minutes): `P=real` 26 of 26 and `P=rect` 37 of 37
  succeeded, in 8 and 13 minutes.
- **Mutants:** 42 of the 43 killed. `M-414` was an error: its target `ri_exp::p_rexp_01`
  timed out at 12 minutes on the mutant (it proves on the library). At 60 minutes
  (2026-10-14) it was killed in 968 s, so all 43 are killed; its patch now carries
  `Tier: slow`, as 004's `M-343`.
- **Weekly** (60 minutes, 1 job): `p_lexp_01__mpos` proved in 130 s. `p_lexp_02_symmetric`
  timed out: not proven (tool limit), as 004's P-IEXP-02 members.
  `p_lexp_01__mneg` failed, on its domain, not the library: with a zero longitude margin
  (either sign) upstream's `S1Interval::Expanded` takes the growth branch, which makes an
  arc within 2·ε of full the full circle (C++ and Go give the same). The member now assumes
  a negative longitude margin, as 004's P-IEXP-01 does; the catalogue says so. Its re-run
  (2026-10-14) proved in 156 s, so both containment members prove. They stay weekly: the
  per-change `P=rect` leg is already over SC-005's 10 minutes locally (T039).

**Done since** (2026-10-14, late evening, with spec 006's records): the records of these runs
(T020, T027, T035, T037: 30 of 31 entries proven, P-LEXP-02 `not-proven (tool limit)`), the
evaluation (T042, with the composition for SC-010), `VERIFIED_TYPES.md` (T044), the quickstart
(T046) and the success criteria (T047: all 13 met, SC-005 on the workstation only). Spec 006's
final run had moved `ri_exp::p_rexp_03_symmetric` and `rl_bnd::p_lbnd_01` weekly (`1da9428`);
the local halves of T039 (`P=real` 2 min 20 s, `P=rect` 5 min 37 s) and T040 are done.

The final run (T045, 2026-10-14, 22:57–23:30): `make test` passed, and `make verify-values`
over all six parts proved 185 of 185 harnesses in 31 min 21 s.

Open: T039 and T040 on CI (need a push), T048, T049 (push only with approval), T050–T052 (the
upstream reports, the user's decision).

**Found while implementing**:
- U-010: upstream `R1Interval::GetCenter` overflows for finite bounds beyond `DBL_MAX / 2`;
  the library's `f64::midpoint` does not (the new vectors, T005).
- U-009 confirmed by the vectors.
- The sign of a zero bound that `f64::min` or `max` chooses may differ from upstream's
  `std::min` or `max` (same value): documented on `r1::Interval`, and compared by value in the
  differential test.
- The centre of the empty `[∞, −∞]` is NaN too (T018; a documentation fix).
- P-LEQ-01 gained upstream's meaning of equality: the laws alone let a mutant survive.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on unfinished tasks). For
  Kani **runs**, "parallel" applies only to writing the code; the runs themselves go one
  after another (Memory safety, below).
- **[Story]**: the user story the task belongs to (US1–US4).
- Paths are relative to the repository root.

## Memory safety and 004's open runs (read first)

- **Every** Kani run goes through a Makefile target or `verify/run-verify.sh`, which apply
  the memory guard:
  - `make verify-values [P=real|rect]`;
  - `make verify-harness H=…`;
  - `make verify-mutants M=…`;
  - `make verify-slow F=values`.

  For a throwaway measurement, use R0's set-up: a scratch crate with a copy of
  `run-verify.sh`, `VERIFY_JOBS=4 VERIFY_HARNESS_TIMEOUT=12m VERIFY_MEM_PER_HARNESS=4G`.
- **004's weekly members and mutant campaign run first, from a frozen worktree.**
  - The runs started on 2026-10-13 from the main tree (wave A: `cl_bnd` levels 4–8).
  - Every later 004 run goes from a detached worktree at `e60a044`:
    `git worktree add --detach verify/target/frozen-004 e60a044`.
  - The mutant runner copies the caller's `verify/` into each mutant's worktree, and every
    run starts with a codegen of the whole crate. So in the main tree, `verify/` must
    **compile at every save** while a 004 run started from it is still going.
- **This feature's Kani runs start only when 004's have finished.** Writing code, the
  ordinary tests and `cargo kani --only-codegen` (no CBMC) can go on meanwhile.
- When timing a harness for a tier decision, run with `VERIFY_JOBS=4` or fewer.

## Upstream findings rule (FR-022, read first)

If a harness, a C++ vector or reading upstream code shows **undefined or wrong behaviour in
the upstream reference**, do these **before** marking the affected proof complete:

1. Append a task to Phase 9 with the next free task ID and the next free `U-NNN`:
   - U-008 (`R1Interval::Project`), U-009 (`GetDirectedHausdorffDistance` at infinite
     bounds) and U-010 (`GetCenter`'s overflow, found by T005) are this feature's
     (research R1), with tasks T050–T052;
   - the next free number is **U-011**.

   Give the upstream `file:line`, the inputs, the observed behaviour, and what the library
   does.
2. Add the same entry under "Upstream findings" in the spec-005 part of
   `verify/values/EVALUATION.md`, and to `UPSTREAM.md`.
3. Cite the `U-NNN` in the affected harness's doc comment.

Never change upstream code.

## Known-defect rule (FR-012, read first)

A proof that fails on a **real library defect** gets a `D-NNN`:

- D-005 and D-006 are taken (004); the next free one is **D-007**.
- Record it under "Defects found" in the spec-005 part of `verify/values/EVALUATION.md`.
- If its harness is written before the fix lands, list the harness in
  `verify/known-defects.txt`, so that runs report `KNOWN-DEFECT`.
- The fix is a **separate, tested change**.
- **Ask the user before fixing** a defect that is not in the spec's survey or this plan.

## Facts these tasks rely on

1. **R0's results** (research R0; 4 jobs, 4 GiB, 12 minutes):
   - **Proved:**
     - real interval: containment 0.2 s; least union 2.1 s; finite expansion 3.4 s;
       projection 0.7 s; centre 21 s;
     - rectangle: validity check 0.1 s; latitude–longitude membership 0.7 s; containment
       2.5 s; union 48 s; intersection 28 s; adding a point 16 s; polar closure 0.5 s;
       expansion validity 62 s; centre-size 33 s; approximate equality reflexive 256 s.
   - **Timed out at 12 minutes:** the rectangle's union stated component-wise; expansion
     containment.
   - **Failed:** the real interval's expansion over infinite bounds, its directed Hausdorff
     distance and its approximate equality, all from Kani's NaN checks (fact 2).
2. **Kani's NaN checks.** Kani reports "NaN on subtraction" (or addition, multiplication) when
   an operation on non-NaN operands produces NaN: `∞ − ∞`, `0 × ∞`. A harness of an
   arithmetic operation therefore assumes the operands where that cannot happen. The
   documented domain (FR-004) is exactly that: never filter NaN after the fact.
3. **The real interval's infinite corners** (FR-017), read from `core/src/r1/interval.rs`:
   - `length`: `hi − lo`, NaN for `[∞, ∞]` and `[−∞, −∞]`;
   - `center`: `f64::midpoint(lo, hi)`, which halves each bound and adds them when either
     is above `f64::MAX / 2`, so NaN for `[−∞, ∞]` and for the empty `[∞, −∞]` (found by
     T018; commit `a26cccf`). Upstream's `0.5 * (lo + hi)` overflows instead for finite
     bounds beyond `DBL_MAX / 2` (U-010);
   - `expanded`: `lo − m`, `hi + m`, a NaN bound for `[−∞, x].expanded(−∞)` and its mirror;
   - `approx_eq_with`: `|y.lo − lo| ≤ e`, false for an interval with an infinite bound, even
     against itself;
   - `directed_hausdorff_distance`: `0.max((hi − y.hi).max(y.lo − lo))`. At `[0, ∞]` against
     `[5, ∞]` the library gives 5, the true distance, and upstream gives 0 (U-009).
4. **`project`** is `debug_assert!(!self.is_empty()); p.clamp(lo, hi)`. `f64::clamp` panics
   when `min > max` or either bound is NaN, **in every build**. A NaN `p` returns NaN without
   panicking. Upstream reaches `std::clamp`'s undefined behaviour on an empty interval in
   release builds (U-008).
5. **`LatLng::is_valid`** is `|lat| ≤ π/2 ∧ |lng| ≤ π` (`core/src/s2/latlng.rs:165`). NaN is
   invalid.
6. **The rectangle's operations** (`core/src/s2/rect.rs`):
   - `contains_lat_lng` and `add_point` return `false` and `self` for an invalid
     latitude–longitude (K2);
   - `interior_contains_lat_lng` does not screen, and passes its longitude to
     `s1::Interval::interior_contains`, which has a debug check `|p| ≤ π` (K3);
   - `from_lat_lng` and `from_point_pair` build their longitude with `s1::Interval`'s
     debug-checked constructors (K3);
   - `expanded(margin)` returns the empty rectangle when either expanded component is empty,
     and otherwise clips the latitudes to `[−π/2, π/2]`. A NaN longitude margin trips
     `s1::Interval::expanded`'s debug check (004's K5). That makes `from_center_size`
     (`from_lat_lng(center).expanded(size / 2)`) debug-checked too;
   - `approx_eq` is `lat.approx_eq_with(1e-15) ∧ lng.approx_eq_with(1e-15)`;
   - `PartialEq` is component-wise; the real interval's equality treats every empty interval
     as equal (upstream `r1interval.h:202`), and the circular interval's is float `==` on
     both bounds.
7. **serde round trips** (research R6): 1,001,444 valid rectangles through `serde_json`'s
   default parsing. 391,960 came back with a bound one ulp off, and none came back invalid.
   `s2::Rect` derives `Serialize` and `Deserialize` (`rect.rs:127`) over `lat: r1::Interval`
   and `lng: s1::Interval`.
8. **Inventory counts**:
   - `r1::Interval`: 23 inherent operations, plus `Default`, `PartialEq`,
     `From<(f64, f64)>` and `Display`;
   - `s2::Rect`: 44 inherent operations, plus `PartialEq`, `Default` and `Display`, and the
     6 `Region` methods in `core/src/s2/region.rs:96`.

   `RectVertex::{next, prev, iter}` belong to `RectVertex` and are not inventoried.
9. **The records' sections.** `verify/values/INVENTORY.md` has sections 1 and 2;
   `verify/values/PROPERTIES.md` has 1, 2 and 3 (the generic laws). This feature adds
   INVENTORY §3–4 and PROPERTIES §4–5.
10. **`--harness ri_` and `--harness rl_` select only the new suites** (research R2). Mutants
    are numbered from `M-401`. `run-mutants.sh` applies patches against `verify/` (004's
    `M-340` to `M-342`).
11. **Appending to the C++ generator keeps the CSV stable.** The generator draws everything
    from one `std::mt19937_64`. New sections called after `CellChildren()` in `main()`
    continue the sequence and leave every earlier line byte-identical.
12. **The pinned upstream** (`/home/t/sources/s2geometry`):
    - `r1interval.h`: `GetLength`, `Expanded` and `ApproxEquals` at lines 215–220;
      `operator==` at line 202; `Project` at lines 173–176;
    - `s2latlng_rect.cc`: `AddPoint`, and `Contains(S2LatLng)` at lines 156–158 and 214–220.

    Go: `golang/geo s2/rect.go` lines 139–145 and 284–289.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: the new modules, the Makefile parts, and the record sections (research R2;
contracts).

- [X] T001 Add the new modules to `verify/src/lib.rs` and update `verify/Cargo.toml`'s comment (research R2; contracts/harness-conventions.md "Modules and names").
  - **New files**, each starting with the two SPDX lines and
    `// Written for this crate (not ported from upstream S2).`, with a one-line `//!` naming
    its data-model section or property area:
    - `verify/src/real_spec.rs` and `verify/src/real_spec_tests.rs` (data-model §1.1);
    - `verify/src/rect_spec.rs` and `verify/src/rect_spec_tests.rs` (§1.2);
    - `verify/src/ri_{crash,val,mem,rel,set,mea,exp}.rs`;
    - `verify/src/rl_{crash,val,mem,rel,set,bnd,exp,eq}.rs`.
  - **`verify/src/lib.rs`:**
    - `pub mod real_spec;` and `pub mod rect_spec;`, unconditional, next to
      `pub mod arc_spec;`;
    - `#[cfg(test)] #[path = "real_spec_tests.rs"] mod real_spec_tests;` in `real_spec.rs`,
      and the same for `rect_spec`;
    - the `ri_*` and `rl_*` modules behind `#[cfg(all(kani, feature = "values"))]`;
    - extend the crate doc's values paragraph to name spec 005 and the two prefixes.
  - **`verify/Cargo.toml`:** the `values` comment names spec 005 and `--harness ri_`
    (`r1::Interval`) and `--harness rl_` (`s2::Rect`).
  - **Checks:** `cd verify && cargo build && cargo test`; `cargo build --no-default-features`;
    `cargo kani --only-codegen`. Codegen needs no cap.
- [X] T002 [P] Extend `make verify-values` with `P=real` and `P=rect` in `Makefile` (contracts/verify-commands.md).
  - `VALUES_HARNESSES` maps `cell` → `--harness cl_`, `interval` → `--harness iv_`, `real` →
    `--harness ri_`, `rect` → `--harness rl_`, and nothing to all four.
  - `verify-values`'s help text: `[P=<cell|interval|real|rect>]`, specs 004 and 005.
  - `F=values` in `SLOW_HARNESSES` already reuses `VALUES_HARNESSES`.
  - **Check:** `make -n verify-values P=rect`; `make help | grep verify-values`.
- [X] T003 [P] Add the record sections in `verify/values/` (data-model §4, §5, §8; fact 9).
  - `INVENTORY.md`: section 3 "`r1::Interval` (27 entries)" and section 4 "`s2::Rect`
    (53 entries)", with 004's columns and no rows yet.
  - `PROPERTIES.md`: section 4 "`r1::Interval`" and section 5 "`s2::Rect`", with 004's
    columns and no rows yet.
  - `EVALUATION.md`: a part "Spec 005: `r1::Interval` and `s2::Rect`", with the headings of
    004's parts plus "Composition (SC-010)".
  - **Check:** `cd verify && cargo test --test property_kinds` passes.

**Checkpoint**: `make verify-values P=real` and `P=rect` run, with zero harnesses, and pass.
`cargo test` in `verify/` passes.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: the C++ reference vectors, the two reference descriptions with their tests, and
the generators.

**⚠️ CRITICAL**: No user-story work starts until this phase is complete.

- [X] T004 Extend `tools/refgen-cpp/generate_values.cc` with four sections (research R9; fact 11).
  - **Header comment:** name spec 005, and document the new columns as the existing ones are
    documented.
  - **`# R1INTERVAL_OPS`:** about 5,000 rows.
    - **Columns:** `x_lo,x_hi,p,margin,max_error,y_lo,y_hi,results,note`.
    - **Random intervals:** half from `R1Interval::FromPointPair` over a mix of uniform
      `[−10, 10]`, small integers (ties) and wide magnitudes (`ldexp`). The rest are struct
      pairs with `lo > hi` (empty) or `lo == hi`.
    - **The other inputs:** a random point, margin (both signs, zero) and error bound.
    - **`results`**, `;`-separated:
      - 0 `is_empty`, 1 `GetCenter`, 2 `GetLength`, 3 `Contains(p)`, 4 `InteriorContains(p)`;
      - 5 `Project(p)` (`-` for an empty `x`);
      - 6–7 `AddPoint(p)`, 8–9 `Expanded(margin)`;
      - 10 `Contains(y)`, 11 `InteriorContains(y)`, 12 `Intersects(y)`,
        13 `InteriorIntersects(y)`;
      - 14–15 `AddInterval(y)`, 16–17 `Union(y)`, 18–19 `Intersection(y)`;
      - 20 `GetDirectedHausdorffDistance(y)`, 21 `ApproxEquals(y, max_error)`, 22 `x == y`.
  - **`# R1INTERVAL_SPECIAL`:** the same columns.
    - **Inputs:**
      - `R1Interval::Empty()`, `[0, 0]`, `[−0, 0]` and `[0, −0]`;
      - upstream's `r1interval_test.cc` fixtures;
      - every pair of bounds from `{−∞, −1, −0, 0, 1, ∞}`;
      - margins `0`, `±1`, `±∞`;
      - the Hausdorff pairs `[0, ∞]` against `[5, ∞]` and `[−5, ∞]`.
    - **`note`:** `deviation:infinite-corner` on a row where an operand bound is infinite
      and an arithmetic result (1, 2, 8, 9, 20, 21) is NaN, or differs from the library's
      expected value (fact 3).
  - **`# RECT_OPS`:** about 5,000 rows.
    - **Columns:**
      `a_lat_lo,a_lat_hi,a_lng_lo,a_lng_hi,ll_lat,ll_lng,m_lat,m_lng,b_lat_lo,b_lat_hi,b_lng_lo,b_lng_hi,results,note`.
    - **Rectangles:** random valid ones from `S2LatLngRect::FromPointPair` over random valid
      latitude–longitudes, plus `Empty()`, `Full()`, single points, and full-longitude
      bands. Never call a constructor outside its `ABSL_DCHECK` domain.
    - **`ll`:** valid in 90% of rows. Otherwise a latitude beyond ±π/2 or a longitude beyond
      ±π.
    - **The margin:** both signs, and zero.
    - **`results`**, `;`-separated:
      - 0 `is_valid`, 1 `is_empty`, 2 `is_full`, 3 `is_point`, 4 `is_inverted`;
      - 5–12 `GetVertex(k)` latitude and longitude, k = 0..3;
      - 13–14 `GetCenter`, 15–16 `GetSize`;
      - 17 `Contains(ll)`, 18 `InteriorContains(ll)`, 19–22 `AddPoint(ll)`, each `-` for
        an invalid `ll` (fact 6: C++ only logs it);
      - 23–26 `Expanded(m)`, 27–30 `PolarClosure()`;
      - 31 `Contains(b)`, 32 `InteriorContains(b)`, 33 `Intersects(b)`,
        34 `InteriorIntersects(b)`;
      - 35–38 `Union(b)`, 39–42 `Intersection(b)`;
      - 43 `ApproxEquals(b)`, 44 `a == b`;
      - 45–48 `FromCenterSize(ll, |m|)` (`-` for an invalid `ll`).
  - **`# RECT_SPECIAL`:** the same columns.
    - **Inputs:**
      - a rectangle at each pole: a band touching it, and a point on it;
      - rectangles across the antimeridian (inverted longitudes);
      - `[−π/2, π/2] × [π, π]` and `[−π/2, π/2] × [−π, −π]`;
      - `Empty()`, `Full()`;
      - the seam corner `[π, nextafter(−π, 0)]` as a longitude range;
      - `ll` at the poles and on ±π.
    - **`note`:** `deviation:seam-length` where the longitude is the seam corner and a
      result uses its length (004's rule): `Expanded`, `Intersection`, `ApproxEquals`.
  - **`main()`:** call the four new functions **after** `CellChildren()` (fact 11).
- [X] T005 Build and run the generator, append the sections to `core/tests/data/cpp_values_vectors.csv`, and document them (depends on T004).
  - **Build:**
    `cmake -B tools/refgen-cpp/build -DCMAKE_BUILD_TYPE=Release -DCMAKE_PREFIX_PATH=/tmp/s2-install`;
    `cmake --build tools/refgen-cpp/build --target generate_values`.
  - **Run:**
    `LD_LIBRARY_PATH=/tmp/s2-install/lib tools/refgen-cpp/build/generate_values > core/tests/data/cpp_values_vectors.csv`.
  - **Checks:**
    - run it twice and `cmp` the outputs;
    - `git diff --stat` shows only added lines, and every line of the committed file is
      unchanged (`git diff core/tests/data/cpp_values_vectors.csv | grep '^-[^-]'` is empty);
    - the CSV stays under 8 MB. (Measured 2026-10-13: 7.0 MB, with the random rows cut to
      3,000 real-interval and 2,000 rectangle rows; 5,000 of each made 10.3 MB.)
  - **The U-009 rows:** check whether `[0, ∞]` against `[5, ∞]` gives 0 in C++. If it does,
    U-009 is confirmed, and T051 stays. If not, delete T051 and drop U-009 from
    `research.md` R1.
  - **`tools/refgen-cpp/README.md`:** add the four sections to the "Value types" table, with
    their columns and the Rust methods they check.
- [X] T006 [P] Write the real-set description in `verify/src/real_spec.rs` (data-model §1.1; research R3).
  - The functions are `empty`, `member`, `interior`, `contains`, `interior_contains`,
    `intersects`, `interior_intersects` and `finite`. They take `(lo, hi)` pairs of `f64`, or
    `r1::Interval`, as `arc_spec` does, and follow data-model §1.1 exactly.
  - **Written from** `r1interval.h`, not from the library. Each function's doc cites the
    upstream method it describes.
  - **Booleans** are combined with `&&`, `||` and `==` only (004 R13).
- [X] T007 [P] Write the hand-made unit tests in `verify/src/real_spec_tests.rs`.
  - Cover: empty with many representations; infinite bounds; `−0` against `0`;
    single-point intervals; interior intersection when `x` is a single point (false);
    containment of the empty interval in everything.
- [X] T008 [P] Write the rectangle description in `verify/src/rect_spec.rs` (data-model §1.2; research R3, R5).
  - **The functions:** `valid`, `ll_valid`, `member`, `interior_member`, `contains`,
    `interior_contains`, `intersects`, `interior_intersects`, `touches_pole`, built from
    `real_spec` and `arc_spec`, never from the library's methods.
  - **`ll_valid(lat, lng)`** is `lat.abs() <= FRAC_PI_2 && lng.abs() <= PI`. That is false
    for NaN.
  - **The interior relations:** upstream's headers define them component-wise
    (`s2latlng_rect.h`); cite the lines.
- [X] T009 [P] Write the hand-made unit tests in `verify/src/rect_spec_tests.rs`.
  - Cover:
    - validity: the four rules separately, an empty latitude with a non-empty longitude,
      and `[−π/2, π/2] × empty` (invalid);
    - membership at the poles and on ±π;
    - containment of the empty rectangle;
    - a rectangle across the antimeridian;
    - `touches_pole`.
- [X] T010 Add the description checks for the new sections to `verify/tests/values_spec_vs_cpp.rs` (depends on T005, T006, T008; research R9).
  - **`real_description_matches_cpp`**: for every `R1INTERVAL_*` row, the relations and
    memberships (results 0, 3, 4, 10–13) equal `real_spec`'s.
  - **`rect_description_matches_cpp`**: for every `RECT_*` row:
    - `is_valid`, `is_empty` (results 0 and 1) and the membership of a valid `ll` (17, 18)
      equal `rect_spec`'s;
    - so do the four relations (31–34).
  - **Check:** `cd verify && cargo test --test values_spec_vs_cpp`.
- [X] T011 Add the generators to `verify/src/value_domain.rs` (data-model §2; research R4; depends on T006, T008).
  - **The generators:**
    - `any_real() -> r1::Interval`: both bounds non-NaN;
    - `any_finite_real()`: both bounds finite;
    - `any_number() -> f64`: non-NaN;
    - `any_rect() -> Rect`: a struct literal from `any_f64` bounds with
      `rect_spec::valid`, never `Rect::is_valid`;
    - `any_lat_lng() -> LatLng`: non-NaN components, valid or not;
    - `any_valid_lat_lng()`: with `rect_spec::ll_valid`.
  - Each doc names its domain class (data-model §2) and why it is built that way.
  - **Check:** `cd verify && cargo kani --only-codegen`.

**Checkpoint**: The descriptions pass their unit tests and the C++ rows. The generators
compile under Kani.

---

## Phase 3: Library Changes and Tests (FR-017 to FR-020), no story

**Purpose**: the documented domains the proofs assume, and the FR-020 fix (research R6, R7).
Each item is its own commit (plan, Structure Decision).

- [X] T012 [P] Document the real interval's infinite corners and `project`'s panic in `core/src/r1/interval.rs` (FR-017, FR-019; research R7; facts 3, 4).
  - **Infinite corners:** an "Infinite bounds" paragraph on `length`, `center`, `expanded`,
    `approx_eq_with`, `approx_eq` and `directed_hausdorff_distance`. Each gives:
    - the NaN or unexpected result, with the input from fact 3;
    - that upstream computes the same, or, for the Hausdorff distance, how upstream
      differs (U-009).
  - **`center`'s deviation** (U-010): a "Deviation from upstream" paragraph. Upstream's
    `0.5 * (lo + hi)` overflows to ±∞ for finite bounds beyond `f64::MAX / 2`, such as
    `[f64::MAX, f64::MAX]`. `f64::midpoint` halves first and returns the true centre.
  - **`project`:** a `# Panics` section. It panics on an empty interval, in every build,
    through `f64::clamp`; upstream's release builds reach undefined behaviour instead
    (U-008).
  - **Confirm each statement with a concrete probe** in debug and release builds, in a
    scratch crate, before committing. 004 found a wrong panic claim this way. Record the
    probe's output in the commit message.
- [X] T013 [P] Document K2's deviation and K3's debug-only panics in `core/src/s2/rect.rs` (FR-018, FR-019; research R7; fact 6).
  - **`add_point` and `contains_lat_lng`:** a "Deviation from upstream" paragraph. An
    out-of-range or NaN latitude–longitude is ignored or not contained. C++ only logs it
    and uses it; Go returns early, and so does the library, so that an invalid point never
    silently widens a bound.
  - **`from_lat_lng`, `from_point_pair` and `interior_contains_lat_lng`:** `# Panics` in
    debug builds on a longitude outside [−π, π], through `s1::Interval`'s checks. Release
    builds return an unspecified result.
  - **`expanded` and `from_center_size`:** `# Panics` in debug builds on a NaN margin or
    size, through `s1::Interval::expanded` (004's K5).
  - **Confirm each statement with a concrete probe**, as in T012.
- [X] T014 [P] Record the K2 deviation and U-008 in `UPSTREAM.md` (FR-018, FR-022; research R1).
  - A row in the deviations table for K2, with C++'s and Go's lines (fact 12).
  - A `U-008` section, in U-007's format: `r1interval.h:173–176`, an empty interval,
    `std::clamp`'s undefined behaviour in release builds, and the library's panic. Mark it
    "candidate, not reported".
  - **U-009 and U-010 sections** in the same format (confirmed by T005's rows: `[0, ∞]`
    against `[5, ∞]` gives 0; `GetCenter` of `[DBL_MAX, DBL_MAX]` gives ∞), and rows in the
    deviations table for the library's two corrected answers.
- [X] T015 Write the FR-020 regression tests in `core/src/s2/rect_tests.rs` (research R6; Constitution III).
  - **Rejected:**
    - a latitude beyond π/2;
    - a NaN bound;
    - an invalid longitude arc: `[−π, 0]` (upstream's arcs never start at −π unless they
      end at π) and `[4, 0]` (out of range);
    - an empty latitude with a full longitude;
    - a non-empty latitude with an empty longitude.
  - **The error messages:** each rejection says which rule failed.
  - **Accepted unchanged:**
    - the boundary values ±π/2, ±π, the floats just inside them, ±0;
    - `Rect::empty()`, `Rect::full()`;
    - a rectangle across the antimeridian;
    - the seam corner as a longitude.
  - **A round-trip test:** 10,000 rectangles from a seeded generator, each valid after
    `serde_json` and back.
  - **Run** `cargo test -p s2rst --features serde s2::rect::rect_tests` and check that the
    rejection tests **fail** before T016.
- [X] T016 Implement FR-020's `Deserialize` for `s2::Rect` in `core/src/s2/rect.rs` (depends on T015; research R6; fact 7).
  - Replace the derived `Deserialize` with a hand-written one, keeping the derived
    `Serialize`, as 004 did for `Cell`:
    - a private shadow struct with the same two fields, so the format is unchanged;
    - the error says which rule failed: a latitude out of range, an invalid longitude arc,
      or emptiness that disagrees;
    - it returns the rectangle as read.
  - The doc comment on `Rect` says what deserialisation accepts, and why it matches the
    binary decoder (003).
  - **Check:** T015's tests pass, and `make test` passes.
- [X] T017 Extend `core/tests/cpp_values_diff.rs` to the new sections (depends on T005, T012, T013, T016; research R9).
  - **`r1_interval_matches_cpp`:** every `R1INTERVAL_*` row; bits for floats.
    - `deviation:infinite-corner` rows check the documented library value (fact 3) instead
      of C++'s, and the test says so in a comment.
  - **`rect_matches_cpp`:** every `RECT_*` row.
    - `-` entries are skipped; `deviation:seam-length` rows are handled as in
      `s1_interval_matches_cpp`.
  - **Check:** `cargo test -p s2rst --test cpp_values_diff`.

**Checkpoint**: `make test` passes. The documentation states every domain the proofs
assume, and deserialisation rejects invalid rectangles.

---

## Phase 4: User Story 1 — Proven crash freedom (Priority: P1) 🎯 MVP

**Goal**: Every in-scope operation of both types is proven free of panics, overflow,
undefined behaviour and firing debug checks over its documented domain (FR-004), with covers
showing the domains non-empty and the outcomes reachable (FR-009).

**Independent Test**: `make verify-values P=real` and `P=rect` report every crash harness
`SUCCESSFUL`, with all covers satisfied. `verify/values/INVENTORY.md` §3–4 names a harness for
every in-scope row.

- [X] T018 [P] [US1] Write the real-interval crash harnesses in `verify/src/ri_crash.rs` (data-model §5; research R4; facts 2–4).
  - **The 26 in-scope entries, grouped into harnesses as 004's `iv_crash`** (done: 9
    harnesses). The domains:
    - `Real` (and `Number` points) for the constructors, `is_empty`, the memberships, the
      four relations, `add_point`, `add_interval`, `union`, `intersection`, `bound`,
      `PartialEq`, `Default` and `From<(f64, f64)>`;
    - for `length`, `center`, `expanded`, `directed_hausdorff_distance`, `approx_eq_with`
      and `approx_eq`, the same except exactly the inputs on which `∞ − ∞` arises, their
      documented corners (as implemented: maximal, not `FiniteReal`);
    - for `project`, a non-empty `Real` and every `f64` point, NaN included (fact 4).
  - Each harness's doc starts with `C-r1::Interval::<op>:` and states its domain.
  - **Covers:** empty, single-point and infinite intervals, where in the domain; `project`
    returning each bound and the point itself.
  - The probe's `r0_r1_project_crash` is the draft.
- [X] T019 [P] [US1] Write the rectangle crash harnesses in `verify/src/rl_crash.rs` (data-model §5; research R4; fact 6).
  - **The 34 in-scope entries, grouped into harnesses** (done: 12 harnesses), the cheap
    accessors together, as 004 did. The domains:
    - `ValidRect`;
    - every latitude–longitude, NaN included, for `add_point` and `contains_lat_lng`;
    - `ValidLatLng` for `from_lat_lng`, `from_point_pair` and `interior_contains_lat_lng`;
    - non-NaN margins for `expanded`, and for `from_center_size`'s size;
    - four `AnyF64` bounds for `is_valid`.
  - Each harness's doc starts with `C-s2::Rect::<op>:` and states its domain.
  - **Covers:**
    - a rectangle at a pole, across the antimeridian, empty and full;
    - an out-of-range point that `add_point` ignores;
    - `expanded` returning the empty rectangle, and clipping at a pole.
  - The probe's `r0_rect_expanded_crash` and `r0_rect_from_center_size` are the drafts.
- [X] T020 [US1] Run `make verify-values P=real` and `P=rect`, and fill in the crash columns of `verify/values/INVENTORY.md` §3–4 (depends on T018, T019, and on 004's runs having finished).
  - **All 80 rows:** name, scope, reason, domain, crash result, crash harness.
  - **The 19 excluded rectangle rows**, with the reasons of data-model §5. The
    float-geometry rows point at the `region_rect_*` tests in
    `core/tests/trait_property_tests.rs` and at D-006's `rect_tests.rs`.
  - **`Display`**, for both types: a tool limit, as 004's.
  - **The binary codec of `s2::Rect`:** a pointer row to `verify/decoding/INVENTORY.md`.
  - Record each harness's time and peak memory from `verify/target/run-verify/results.tsv`.

**Checkpoint**: US1 is complete and shippable. SC-001 and SC-002 hold.

---

## Phase 5: User Story 2 — Proven meaning, the rectangle built on its intervals (Priority: P2)

**Goal**: The 31 catalogue properties (data-model §4) are proven against the reference
descriptions, or recorded with a reason. Each proven one has a mutant that breaks it.

**Independent Test**:

- `make verify-values P=real` and `P=rect` report every property harness `SUCCESSFUL`.
- `make verify-mutants` reports every `M-4NN` killed.
- `cargo test --test property_kinds` passes.

### Real-interval properties

- [X] T021 [P] [US2] Write P-RIV-01, P-RIV-02 and P-RIV-03 in `verify/src/ri_val.rs` (data-model §4.1).
  - **P-RIV-01:** each constructor against its documented interval; `is_empty` against
    `real_spec::empty`; `Default` is `[1, 0]`, bit for bit.
  - **P-RIV-02:** `laws::eq::all` over `any_real()³`, plus "every two empty intervals are
    equal".
  - **P-RIV-03:** `bound`.
- [X] T022 [P] [US2] Write P-RMEM-01 and P-RMEM-02 in `verify/src/ri_mem.rs` against `real_spec::member` and `interior`.
- [X] T023 [P] [US2] Write P-RREL-01 and P-RREL-02 in `verify/src/ri_rel.rs` (data-model §4.1).
  - The probe's `r0_r1_contains_model` is the draft. Add interior containment, both
    intersections, and the symmetry of `intersects`.
- [X] T024 [P] [US2] Write P-RSET-01 to P-RSET-03 in `verify/src/ri_set.rs` (data-model §4.1).
  - **P-RSET-01:** the probe's `r0_r1_union_least` is the draft.
  - **P-RSET-02:** membership of every number in the intersection is membership of both.
  - **P-RSET-03:** `add_point` contains the point and the original, and is contained in
    every interval that contains both. `add_interval` equals `union`, bit for bit.
- [X] T025 [P] [US2] Write P-RMEA-01 and P-RMEA-02 in `verify/src/ri_mea.rs` (data-model §4.1; fact 3).
  - **P-RMEA-01:** over `FiniteReal`. The probe's `r0_r1_center_inside` is the draft; the
    length is `hi − lo`, bit for bit, and non-negative for a non-empty interval.
  - **P-RMEA-02:** over `FiniteReal²`: non-negative; 0 when `y` contains `x`; `∞` when
    only `y` is empty; 0 when `x` is empty.
  - Cite U-009 in P-RMEA-02's doc: the infinite corner is documented, not proven.
- [X] T026 [P] [US2] Write P-REXP-01 to P-REXP-03 in `verify/src/ri_exp.rs` (data-model §4.1).
  - **P-REXP-01:** the probe's `r0_r1_expanded_finite_contains` is the draft. Add the
    shrinking half: by a finite non-positive margin, the result is contained in the
    original, and the empty interval stays empty.
  - **P-REXP-02:** over a non-empty `Real` and a non-NaN point: inside, the point itself;
    otherwise the nearer bound.
  - **P-REXP-03:** reflexive and symmetric over `FiniteReal²` and a finite non-negative
    error bound; `approx_eq` is `approx_eq_with(1e-15)`.
- [X] T027 [US2] Run `make verify-values P=real`, and record times, memory and results in the spec-005 part of `verify/values/EVALUATION.md` ("Measurements") and the status column of `verify/values/PROPERTIES.md` §4 (depends on T021–T026).

### Rectangle properties

- [X] T028 [P] [US2] Write P-LVAL-01 to P-LVAL-03 in `verify/src/rl_val.rs` (data-model §4.2, §3).
  - **P-LVAL-01:** the probe's `r0_rect_valid_model` is the draft.
  - **P-LVAL-02:** one harness per operation that returns a rectangle, named
    `p_lval_02__<op>`, each asserting `rect_spec::valid` on the result. These are the
    13 operations of data-model §4.2.
    - The probe's `r0_rect_add_point`, `r0_rect_polar_closure`, `r0_rect_expanded_crash`
      and `r0_rect_from_center_size` are drafts.
    - Fold the validity assertion into T019's crash harnesses only where a harness would
      otherwise be duplicated, and say so in both docs.
  - **P-LVAL-03:** the classifications, and the empty, full and default rectangles.
- [X] T029 [P] [US2] Write P-LMEM-01 and P-LMEM-02 in `verify/src/rl_mem.rs` (data-model §4.2).
  - **P-LMEM-01:** the probe's `r0_rect_contains_latlng` is the draft. Its domain is
    `AnyLatLng`, so the K2 screen is part of the statement.
  - **P-LMEM-02:** over `ValidLatLng`.
- [X] T030 [P] [US2] Write P-LREL-01 and P-LREL-02 in `verify/src/rl_rel.rs` (data-model §4.2).
  - The probe's `r0_rect_contains_model` is the draft. State every relation against
    `rect_spec`, never against the components' operations (contracts "Statements").
- [X] T031 [P] [US2] Write P-LSET-01 to P-LSET-04 in `verify/src/rl_set.rs` (data-model §4.2).
  - **P-LSET-01:** the probe's `r0_rect_union_direct` is the draft. Do **not** use
    `r0_rect_union_componentwise`: it timed out (R0).
  - **P-LSET-02:** the probe's `r0_rect_intersection_direct` is the draft. Add the clause
    "empty exactly when the two do not intersect".
  - **P-LSET-03:** the probe's `r0_rect_add_point` is the draft. Add
    `from_lat_lng(ll)` containing `ll`, and `from_point_pair(a, b)` containing both.
  - **P-LSET-04:** the probe's `r0_rect_polar_closure` is the draft. Add the full longitudes
    when a pole is touched, and the identity otherwise.
- [X] T032 [P] [US2] Write P-LBND-01 in `verify/src/rl_bnd.rs` (data-model §4.2).
  - `lo`, `hi`, the four vertices in upstream's order (`s2latlng_rect.h`: lower left, lower
    right, upper right, upper left), `center` and `size`, against the components' bounds,
    `center` and `length`. Compare the bits.
- [X] T033 [P] [US2] Write P-LEXP-01 and P-LEXP-02 in `verify/src/rl_exp.rs` (data-model §4.2; research R8).
  - **P-LEXP-01, per change:** validity for every non-NaN margin; construction from a
    centre and size is valid and contains the centre.
  - **P-LEXP-01, weekly:** `p_lexp_01__mpos` (both margins finite and ≥ 0, so the result
    contains the original) and `p_lexp_01__mneg` (latitude margin ≤ 0 and longitude margin
    < 0, so the result is contained in it; a zero longitude margin grows, as in 004's
    P-IEXP-01), behind `#[cfg(feature = "slow")]`.
  - **P-LEXP-02:** reflexive per change (the probe's `r0_rect_approx_eq_reflexive`, 256 s).
    Symmetric goes per change if it finishes within 6 minutes, and weekly otherwise.
- [X] T034 [P] [US2] Write P-LEQ-01 and P-LEQ-02 in `verify/src/rl_eq.rs` (data-model §4.2).
  - **P-LEQ-01:** `laws::eq::all` over `any_rect()³`.
  - **P-LEQ-02:** `rect_bound` and `Region::rect_bound` return the rectangle, bit for bit.
  - The doc of `rl_eq` says why 004's `laws::region` is not instantiated here
    (data-model §4.2, the note after the table).
- [X] T035 [US2] Run `make verify-values P=rect`, then `make verify-slow F=values P=rect` for the weekly members, and record the results as in T027, in `verify/values/PROPERTIES.md` §5 (depends on T028–T034).
  - Move a per-change member that takes over 6 minutes to the weekly tier, and record why.
  - A weekly member that times out at 60 minutes is "not proven (tool limit)", with the
    measurement.

### Non-vacuity, catalogue and index

- [X] T036 [US2] Write the `M-4NN` mutant patches in `verify/mutants/` and run them (data-model §7; research R10; contracts/harness-conventions.md "Mutant patches").
  - **The patches:** 43 in 004's format (`Mutant`, `Targets`, `Expect-Fail`, `Defect`,
    optional `Tier: slow`), one defect each, `M-401` … `M-443`, as data-model §7 lists (done
    2026-10-13; each applies to HEAD). Generate them with a script, as 004 did, so that the context lines match
    the formatted source.
  - **The two model mutants** patch `verify/src/real_spec.rs` and `verify/src/rect_spec.rs`.
  - **Run** `make verify-mutants M="M-401 … M-4NN"`, one campaign, after T035. Add a row
    per patch to `verify/MUTANTS.md`.
  - A mutant that survives means the harness is vacuous or too weak. Strengthen the harness
    until the mutant is killed; never delete the mutant.
- [X] T037 [US2] Complete `verify/values/PROPERTIES.md` §4–5 and `verify/PROPERTIES_BY_KIND.md`, and tag the harness docs (FR-010, SC-012).
  - **`PROPERTIES.md`:** all 31 rows, with 004's columns. Statuses come from T027 and
    T035; mutants from T036.
  - **§5's note on the generic region laws** (data-model §4.2).
  - **`PROPERTIES_BY_KIND.md`:** every entry under its kind, and the summary counts updated.
  - **Harness docs:** each property harness's doc starts with `P-ID (kind): <plain
    statement>`.
  - **Check:** `cd verify && cargo test --test property_kinds` passes.

**Checkpoint**: US2 is complete. SC-003, SC-004 and SC-012 hold.

---

## Phase 6: User Story 3 — Regression protection for future changes (Priority: P3)

**Goal**: The proofs run in CI on relevant changes, and stay out of the default gate (FR-014,
SC-005).

**Independent Test**: a change that breaks a proven property fails the matching `verify` leg,
naming the property. The revert passes. An unrelated change does not start the legs.

- [X] T038 [US3] Wire the new parts into `.github/workflows/verify.yml` (contracts/verify-commands.md "CI").
  - **The matrix:** add `values-real` and `values-rect` to `suite`. The existing `values-*`
    case already maps them.
  - **Both `paths` lists:** add `core/src/s2/latlng.rs`.
  - **The header comment:** name spec 005 and the two new legs.
- [ ] T039 [US3] Measure both legs against SC-005, and move members to the weekly tier until each leg is under 10 minutes on the reference runner (depends on T038; research R8).
  - Measure locally with `VERIFY_JOBS=4 make verify-values P=real`, and the same for
    `P=rect`. On CI, measure with `workflow_dispatch`, which needs the user's push approval.
  - Record the times in the spec-005 part of `verify/values/EVALUATION.md`.
- [ ] T040 [US3] Check US3's acceptance scenarios with a deliberate break (quickstart §6).
  - On a scratch change, remove `contains_lat_lng`'s validity screen.
  - `make verify-values P=rect` fails, naming `rl_mem::p_lmem_01` with a concrete input.
    Revert it.
  - On CI (needs push approval), the `values-rect` leg fails, and a change touching none of
    the paths does not start the leg.
- [X] T041 [P] [US3] Document the new parts in `verify/README.md` and `CONTRIBUTING.md`.
  - `verify/README.md`, the values section: `P=real` and `P=rect`, and spec 005.
  - `CONTRIBUTING.md`, "Formal verification": name `r1::Interval` and `s2::Rect`. Before
    changing `r1/interval.rs`, `s2/rect.rs` or `s2/latlng.rs`, run `make verify-values`.

**Checkpoint**: CI runs the new legs on relevant changes. `make test` is unchanged.

---

## Phase 7: User Story 4 — Evaluation of composite value types (Priority: P3)

**Goal**: A maintainer who did not take part can answer SC-007's four questions from the
evaluation alone (FR-016).

**Independent Test**: read the spec-005 part of `verify/values/EVALUATION.md` cold, and
answer "continue with the value types?", "with which approach?", "type-level invariants
first?" and "what next?".

- [X] T042 [US4] Write the spec-005 part of `verify/values/EVALUATION.md` (research R12; FR-016; SC-007, SC-010).
  - **Composition (SC-010):**
    - the share of rectangle properties whose statements use `arc_spec`;
    - the rectangle's harness times against 004's circular-interval times;
    - R0's component-wise timeout as the counterpoint.
  - **The findings:** K1–K4, U-008 and U-009, and any `D-NNN`.
  - **Go or no-go**, ranked, for `r2::Rect`, `ChordAngle` and `Point`, with reasons.
  - **Type-level invariants:** the rectangle's public fields let any caller build an invalid
    rectangle, and FR-020 only closes deserialisation.
  - **Also:** effort, run times, limitations, the comparison with 001–004, and a ranked list
    of follow-ups.
- [X] T043 [P] [US4] Extend `verify/TRUSTED_BASE.md`'s values section for spec 005 (research R11; FR-015).
  - **Reliance on 004:**
    - the reuse of `arc_spec`, `laws::eq` and the generators;
    - the circular interval's code, executed (not stubbed) inside every rectangle proof.
  - **The checker's model:** Kani's NaN checks as part of the crash definition, and
    `f64::min`, `max`, `clamp` and `midpoint` as Kani models them.
  - **Domain splits:** the weekly `__mpos`/`__mneg` members.
- [X] T044 [P] [US4] Update `VERIFIED_TYPES.md`: move `r1::Interval` and `s2::Rect` under "Types with proofs", with the suite, the per-change and weekly parts, and links to the records.

**Checkpoint**: US4 is complete. SC-007 and SC-010 hold.

---

## Phase 8: Polish & Cross-Cutting Concerns

- [X] T045 Run the whole gate and the whole values suite once (`make test`, then `make verify-values` over all four parts), one after another, and record the run in the spec-005 part of `verify/values/EVALUATION.md` ("Final run").
- [X] T046 [P] Walk through `specs/005-formal-verify-r1-interval-rect/quickstart.md` §1–§6, and correct the quickstart where reality differs (command names, counts, times).
- [X] T047 [P] Check the spec's success criteria SC-001 to SC-013 one by one against the records, and list the result per criterion in the spec-005 part of `verify/values/EVALUATION.md` ("Success criteria").
- [ ] T048 [P] Re-run `/speckit-analyze` on `specs/005-formal-verify-r1-interval-rect/`, and resolve any CRITICAL or HIGH finding before the PR.
- [ ] T049 Commit the work in the order of "Commit plan" below. Push only with the user's approval.

---

## Phase 9: Follow-ups (FR-022) — recorded, not implemented in this feature

These tasks record upstream findings so that they are not lost. They need the user's
decision on reporting upstream.

- [ ] T050 U-008: report `R1Interval::Project`'s undefined behaviour on an empty interval upstream (research R1; fact 4).
  - Upstream: `r1interval.h:173–176`, `std::clamp(p, lo(), hi())` behind
    `ABSL_DCHECK(!is_empty())`.
  - Input: `R1Interval::Empty().Project(0.5)` in a release build.
  - The library panics in every build (T012 documents it).
- [ ] T051 U-009: report `R1Interval::GetDirectedHausdorffDistance` at infinite bounds upstream (research R1; fact 3).
  - Upstream: `r1interval.h:148`, `std::max(0.0, std::max(hi() − y.hi(), y.lo() − lo()))`.
  - Inputs (confirmed by T005's rows): `[0, ∞]` against `[5, ∞]`, where C++ gives 0 and the
    true distance, the library's answer, is 5; `[−∞, ∞]` against `[−1, ∞]`, where C++ gives
    0 and the true distance is ∞. 27 rows of `R1INTERVAL_SPECIAL` show it.
- [ ] T052 U-010: report `R1Interval::GetCenter`'s overflow upstream (found by T005).
  - Upstream: `r1interval.h:97`, `0.5 * (lo() + hi())`.
  - Input: `[DBL_MAX, DBL_MAX]` gives ∞, where the true centre, the library's answer through
    `f64::midpoint`, is `DBL_MAX`. 150 rows of `R1INTERVAL_SPECIAL` show it (bounds beyond
    `DBL_MAX / 2`).

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: no dependencies. T002 and T003 are [P] with T001.
- **Phase 2 (Foundational)**: needs Phase 1.
  - T005 needs T004. T010 needs T005, T006 and T008.
  - T006–T009 are [P].
  - T011 needs T006 and T008, because the generators use the descriptions.
- **Phase 3 (library changes)**: needs Phase 1. It can run alongside Phase 2.
  - T012, T013 and T014 are [P].
  - T015 → T016.
  - T017 needs T005, T012, T013 and T016.
- **Phase 4 (US1)**: needs T011, and Phase 3's T012, T013 and T016 (the documented domains and
  the fixed behaviour). T020 also needs 004's runs to have finished.
- **Phase 5 (US2)**: needs Phases 2 and 3. The runs T027 and T035 follow T020. The mutant run
  T036 needs every property harness.
- **Phase 6 (US3)**: needs the harnesses it runs (US1, US2).
- **Phase 7 (US4)**: needs US1–US3's measurements.
- **Phase 8 (Polish)**: needs everything above.
- **Phase 9**: T050–T052 can be reported at any time.

### User Story Dependencies

- **US1 (P1)**: independent after Phases 2–3. It is the MVP.
- **US2 (P2)**: needs US1's modules only for the shared generators. The property harnesses
  stand alone, and each one also discharges its operations' crash obligations.
- **US3 (P3)**: needs the suites it runs.
- **US4 (P3)**: needs everything it evaluates.

### Parallel Opportunities

- Phase 1: T002 and T003.
- Phase 2: T006, T007, T008 and T009.
- Phase 3: T012, T013 and T014. T015 and T016 edit `rect.rs` after T013: commit T013 first.
- US1: T018 and T019.
- US2: T021–T026 and T028–T034 (different modules). The Kani runs go one after another.
- US4: T043 and T044.
- Polish: T046–T048.

---

## Parallel Example: User Story 2

```text
# After Phase 3, write these side by side; run them one at a time, after 004's runs:
Task: "T021 [US2] P-RIV-01..03 in verify/src/ri_val.rs"
Task: "T023 [US2] P-RREL-01..02 in verify/src/ri_rel.rs"
Task: "T024 [US2] P-RSET-01..03 in verify/src/ri_set.rs"
Task: "T029 [US2] P-LMEM-01..02 in verify/src/rl_mem.rs"
Task: "T030 [US2] P-LREL-01..02 in verify/src/rl_rel.rs"
Task: "T031 [US2] P-LSET-01..04 in verify/src/rl_set.rs"
```

---

## Implementation Strategy

### MVP first (User Story 1 only)

1. Phase 1, then Phases 2 and 3 (they can overlap).
2. Phase 4 (US1): every in-scope operation proven crash-free, with K1–K3 documented and
   FR-020 fixed.
3. **Stop and validate:** `make verify-values P=real` and `P=rect` pass, and the inventory is
   complete.

### Incremental delivery

1. Setup, foundation and library changes: the vectors, the descriptions, the documentation,
   deserialisation that rejects invalid rectangles.
2. US1: crash freedom.
3. US2: meaning, with the mutants.
4. US3: CI.
5. US4: the evaluation.

Each step leaves `make test` and `make verify` green.

### Commit plan

Separate commits, in this order:

1. the spec 005 documents (`specs/005-formal-verify-r1-interval-rect/`);
2. T012 (the real interval's documentation);
3. T013 and T014 (the rectangle's documentation, `UPSTREAM.md`);
4. T015 with T016 (FR-020);
5. the vectors (T004, T005, T017);
6. the `verify/` suite (Phases 1–2, US1, US2);
7. the CI wiring (T038, T041);
8. the records (T020, T027, T035–T037, T042–T044).

Push only with the user's approval.
