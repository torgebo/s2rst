---

description: "Task list for the formal verification of r2::Rect and Point (spec 006)"
---

# Tasks: Formal Verification of `r2::Rect` and `Point`

**Input**: Design documents from `specs/006-formal-verify-r2-rect-point/`:

- plan.md, spec.md;
- research.md (R0–R14; R0 ran on 2026-10-14);
- data-model.md;
- contracts/ (verify-commands, harness-conventions);
- quickstart.md;
- the Kani probe `probe/r0_harnesses.rs`, with its results `probe/r0_results.tsv` and
  `probe/r0_counterexamples.txt`. Its harnesses are the first drafts of several tasks below,
  except `r0_pt_from_coords_finite`, `r0_pt_ortho_odd` and `r0_pt_point_cross_antisymmetric`,
  whose statements R0 corrected.

The feature extends the `values` suite of 004 and 005 in `verify/`. 005's task list
(`specs/005-formal-verify-r1-interval-rect/tasks.md`) shows the patterns, and its files are
the style reference:

- `verify/src/real_spec.rs`, `rect_spec.rs`, `ri_*.rs` and `rl_*.rs`;
- `verify/src/value_domain.rs` and `laws.rs`;
- `tools/refgen-cpp/generate_values.cc`;
- `core/tests/cpp_values_diff.rs`;
- `verify/tests/values_spec_vs_cpp.rs`;
- the hand-written `Deserialize` of `s2::Rect` (`core/src/s2/rect.rs:161`).

**Prerequisites**:

- **Kani 0.68.0**: `make verify-kani-check` passes.
- **For T005 only**: the C++ reference install at `/tmp/s2-install`, built from
  `/home/t/sources/s2geometry` at `v0.14.0-40-g37a4501`. If it is gone, rebuild it with
  `cmake --install` from `/home/t/sources/s2geometry/build` (`tools/refgen-cpp/README.md`).
  The generator runs with `LD_LIBRARY_PATH=/tmp/s2-install/lib`.
- **For T021's Go check only**: the Go checkout at `/home/t/sources/geo` (`fe8a584`), used
  through a scratch module with `replace github.com/golang/geo => /home/t/sources/geo`.
- **Disk**: `df -h /` shows at least 20 GB free before any Kani run.

**Tests**: The spec asks for these ordinary tests, so they appear as tasks:

- the regression tests of FR-017's debug check, FR-019, FR-020 and FR-021, each failing
  without its change (Constitution III, SC-009);
- the tests pinning the documented behaviour of K10 and K11 (FR-022);
- the differential tests against the new C++ vectors (FR-023);
- the reference descriptions' unit tests and their sanity tests against the same vectors
  (FR-007).

The Kani harnesses are the verification deliverable itself, so they are implementation
tasks.

**Organization**: Tasks are grouped by user story (spec.md):

| Story | Priority | Phase |
|-------|----------|-------|
| US1: proven crash freedom | P1 | 4 |
| US2: proven meaning, a product of proven intervals and an ordered vector | P2 | 5 |
| US3: CI regression protection | P3 | 6 |
| US4: the evaluation of the value types as a whole | P3 | 7 |

The library changes (FR-017 to FR-024) come first, in Phase 3, because the proofs' domains
and statements rest on the documented and fixed behaviour (plan, Complexity Tracking).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on unfinished tasks). For
  Kani **runs**, "parallel" applies only to writing the code; the runs themselves go one
  after another (Memory safety, below).
- **[Story]**: the user story the task belongs to (US1–US4).
- Paths are relative to the repository root.

## Memory safety and the open runs (read first)

- **Every** Kani run goes through a Makefile target or `verify/run-verify.sh`, which apply
  the memory guard:
  - `make verify-values [P=uvrect|point]`;
  - `make verify-harness H=…`;
  - `make verify-mutants M=…`;
  - `make verify-slow F=values P=…`.

  For a throwaway measurement, use R0's set-up: a scratch crate with a copy of
  `run-verify.sh`, `VERIFY_JOBS=4 VERIFY_HARNESS_TIMEOUT=12m VERIFY_MEM_PER_HARNESS=4G`.
- **The queued runs of 004 and 005 finished** (2026-10-14): 004's P-IEXP-03 members timed
  out at 60 minutes (not proven, tool limit; they are now weekly), and 005's corrected
  `rl_exp::p_lexp_01__mneg` proved in 156 s. Before starting a run, check that no other is
  going with `pgrep -fa '[c]bmc'` (the bracket keeps `pgrep` from matching itself). Each
  run starts with a codegen of the whole crate and ends with a concrete playback, which
  compiles it again. So `verify/` must **compile at every save** while one is going.
- **This feature's Kani runs go one at a time**, never beside another Kani run. Writing
  code, the ordinary tests and `cargo kani --only-codegen` (no CBMC) can go on meanwhile.
- When timing a harness for a tier decision, run with `VERIFY_JOBS=4` or fewer.

## Upstream findings rule (FR-024, read first)

If a harness, a C++ vector or reading upstream code shows **undefined or wrong behaviour in
the upstream reference**, do these **before** marking the affected proof complete:

1. Append a task to Phase 9 with the next free task ID and the next free `U-NNN`:
   - U-012 (C++ `RobustCrossProd` overflowing for unit points, K6) and U-013 (Go
     `PointCross` breaking its documented sign laws, K11) are this feature's, with tasks
     T057 and T058;
   - the next free number is **U-014**.

   Give the upstream `file:line`, the inputs, the observed behaviour, and what the library
   does.
2. Add the same entry under "Upstream findings" in the spec-006 part of
   `verify/values/EVALUATION.md`, and to `UPSTREAM.md`.
3. Cite the `U-NNN` in the affected harness's doc comment.

Never change upstream code.

## Known-defect rule (FR-012, read first)

A proof that fails on a **real library defect** gets a `D-NNN`:

- D-005 to D-007 are taken (004); the next free one is **D-008**.
- K5 (`ensure_normalizable`) is the survey's defect, fixed by FR-020 (T018), and needs no
  `D-NNN`, as 004's survey defects did not.
- Record a new one under "Defects found" in the spec-006 part of
  `verify/values/EVALUATION.md`.
- If its harness is written before the fix lands, list the harness in
  `verify/known-defects.txt`, so that runs report `KNOWN-DEFECT`.
- The fix is a **separate, tested change**.
- **Ask the user before fixing** a defect that is not in the spec's survey or this plan.

**A failure is replayed before it is believed** (research R0, R14): run the concrete
playback's input under IEEE-754 in an ordinary test. CBMC reports a division by an
infinity as NaN, which IEEE-754 does not produce (fact 2).

## Facts these tasks rely on

1. **R0's results** (research R0; 4 jobs, 4 GiB, 12 minutes; `probe/r0_results.tsv`):
   - **the u-v rectangle**: validity 0.1 s; membership 0.3 s; containment 0.6 s; union
     2.2 s; intersection 6.0 s; adding a point 1.8 s; expansion crash 0.4 s; growth
     containment 28.2 s; centre-size 19.3 s; projection 1.2 s; approximate equality
     reflexive 3.1 s; `vertex` for every `i32` 0.2 s;
   - **the point**: the order 0.4–0.9 s; `is_normalizable`'s model 0.2 s; **`is_unit`'s
     model 644.5 s** (weekly); FR-020's draft scaling 113.8 s; arithmetic 150.5 s; `rotate`'s
     crash 134.1 s; `ortho` non-zero 358.0 s; the chord angle's range 6.2 s; zero distance to
     itself 3.2 s;
   - **failed as expected**: the library's `ensure_normalizable` at `(−4.17e-309)³` (K5);
     `distance`'s symmetry on the `atan2` check; infinite coordinates in `from_coords`;
   - **failed and reopened decisions**: facts 2, 3 and 9.
2. **Kani's NaN checks.** Kani reports "NaN on subtraction" (or addition, multiplication)
   when an operation on non-NaN operands produces NaN: `∞ − ∞`, `0 × ∞`. A harness of an
   arithmetic operation therefore assumes the operands where that cannot happen (FR-004);
   never filter NaN after the fact. **CBMC 6.11 also reports a division by ±∞ as "NaN on
   division"**, though IEEE-754 gives ±0: `normalize`'s `1.0 / n2.sqrt()` with an overflowing
   squared length (`(0, 0, −2.68e154)`), and the old `ensure_normalizable`'s `2 / −∞`. So
   `from_coords` and `normalize` assume `NormableTriple` (`point_spec::normable`), and the
   rest of the finite triples is a tool limit with ordinary tests (FR-004, amended).
3. **The zeros' signs.** `ortho(a)` and `ortho(−a)` can both have `+0` coordinates: a
   coordinate computed as `0 − 0` is `+0` for both signs of the input, and `normalize`
   returns `(+0, +0, +0)` when the squared length underflows (R0's counterexample
   `(−0, −7.6e-249, 0)`). The orthogonal directions' laws compare with `point_spec::same`
   (IEEE equality per coordinate). Every other "bit for bit" statement compares `to_bits`.
4. **Trigonometry** (research R0). `f64::atan2` is a foreign C call that Kani replaces with a
   failing check. `distance`, `stable_angle`, `approx_eq`, `approx_eq_angle` and
   `approx_eq_with` reach it through `r3::Vector::angle` (`core/src/r3/vector.rs:180–190`),
   which returns 0 early for equal vectors, so equal points never reach it. `sin` and `cos`
   lower to CBMC's built-in models, over-approximated in [−1, 1]: `rotate`
   (`core/src/s2/point.rs:1271–1277`, through `Angle::sin_cos`) gets a crash proof.
5. **`r2::Rect`** (`core/src/r2/rect.rs`):
   - `#[derive(Clone, Copy, Debug)]` and a derived serde pair (line 41); public fields `x`
     and `y`;
   - `new` has `debug_assert_eq!(x.is_empty(), y.is_empty())` (line 53); `from_points` has
     `debug_assert!(r.is_valid())` (line 63); `from_center_size` (lines 90–95) checks nothing
     (K2);
   - `is_valid` is `x.is_empty() == y.is_empty()`: a NaN bound passes it (K4);
   - `vertex(k)` reduces `k` with `rem_euclid(4)` (line 126); `project` calls
     `r1::Interval::project` per axis (lines 211–213);
   - `expanded` and `intersection` return `Rect::empty()` when either axis is empty;
     `approx_eq` is `approx_eq_with(1e-15)`; `PartialEq` compares both intervals;
   - an inline `mod tests` at line 285; new tests go to a sibling `rect_tests.rs`, linked
     as `s2::Rect`'s are (`core/src/s2/rect.rs:2432–2433`).
6. **`r1::Interval`, which `r2::Rect` calls** (`core/src/r1/interval.rs`):
   - `expanded` (lines 299–307) is `[lo − m, hi + m]`, or the empty interval unchanged. No
     rounding allowance, so a zero margin is neither growth nor shrinking, unlike
     `s1::Interval` (005's `p_lexp_01__mneg` lesson, research R14);
   - `project` (lines 283–286) is `p.clamp(lo, hi)`, which panics on an empty interval in
     every build (U-008);
   - equality treats every empty interval as equal (`r1interval.h:202`).
7. **`Point`** (`core/src/s2/point.rs`):
   - `#[derive(Clone, Copy, Debug, Default, PartialEq)]` and a derived serde pair (lines
     51–52); `Point(pub Vector)`;
   - `from_coords` (66–71): every signed zero triple gives `origin`; otherwise
     `Vector::normalize` (`core/src/r3/vector.rs:146–152`: zero for `n2 == 0`, else
     `self * (1.0 / n2.sqrt())`);
   - `is_unit` (117–119): `|n2 − 1| ≤ 5ε`, upstream's `IsUnitLength`
     (`s2pointutil.cc:32`), not `Vector::is_unit`'s `5e-14`;
   - `is_normalizable` (218–221): the largest absolute coordinate `≥ 2⁻²⁴²`, through
     `f64::max`;
   - `ensure_normalizable` (226–240): K5;
   - `ortho` and `ortho_impl` (248–265): the perturbation vector `(0.012, 0.0053, 0.00457)`
     with 1 at the axis before the largest absolute component
     (`Vector::largest_abs_component`, `vector.rs:195–204`; ties go to the later axis);
     `S2::Ortho` is `s2pointutil.cc:48`;
   - `ilogb` (269–279) handles subnormals;
   - `point_cross` (176–195): `(p + q) × (q − p)`, or `ortho(p)` when that is exactly
     the zero vector. Its doc lists Go's four properties (K11);
   - `cmp_point` (208) is `Vector::cmp` (`vector.rs:243`), and `PartialOrd` uses it;
   - inline `mod tests` (line 334) and `mod point_property_tests` after `rotate`; new tests
     go to a sibling `point_tests.rs`.
8. **The robust cross product's private scaling** (`core/src/s2/edge_crossings.rs`):
   - `is_normalizable` (252–256) and `ensure_normalizable` (261–276), which scales into
     [0.5, 1) through `p_max.log2().floor()` and `powi` (K6);
   - callers: `normalizable_from_exact` (line 283) and the symbolic branch (lines 345, 347);
   - tests in the inline `mod tests` (line 487) call them at lines 1059, 1079 and 1086;
   - upstream: `EnsureNormalizable` (`s2edge_crossings.cc:294`), called at lines 344, 346
     and 357.
9. **K11** (spec): `point_cross(p, q)` takes the `ortho` branch whenever `(p + q) × (q − p)`
   rounds to the zero vector, which includes distinct parallel points and underflowing
   ones. For `p = (1, 0, 0)` and `q = (1 + 2ε, 0, 0)`, both passing `is_unit`,
   `q.point_cross(p)` is `ortho(q)`, not `−p.point_cross(q)`. Go's `PointCross`
   (`/home/t/sources/geo` `s2/point.go:71–92`) does the same. C++'s `RobustCrossProd`
   (`s2edge_crossings.h:84–94`) perturbs parallel points symbolically and keeps the laws.
10. **Inventory counts** (FR-002, FR-003):
    - `r2::Rect`: 30 inherent operations, `Default`, `PartialEq` and `Display`, 33 entries
      (32 in scope);
    - `Point`: 20 inherent operations, `ortho` and `rotate`, 7 hand-written trait
      implementations, and the derived `PartialEq` and `Default`, 31 entries (24 in scope, 6
      through trigonometry, `Display` excluded).
11. **Records and numbering.** `verify/values/INVENTORY.md` has sections 1–4 and
    `PROPERTIES.md` sections 1–5; this feature adds INVENTORY §5–6 and PROPERTIES §6–7, and
    the order laws join PROPERTIES §3. `--harness ur_` and `--harness pt_` select only the new
    suites (research R2). Mutants are numbered from `M-501`.
12. **The C++ vectors.** `tools/refgen-cpp/generate_values.cc`'s `main()` (line 614) calls
    `RectSpecial()` last. New sections called after it continue the one `std::mt19937_64`
    sequence and leave every earlier line byte-identical.
    `core/tests/data/cpp_values_vectors.csv` is 7.0 MB and excluded from the published crate
    (`core/Cargo.toml:14`).
13. **The pinned upstream** (`/home/t/sources/s2geometry`):
    - `r2rect.cc:25` (`FromCenterSize`), `:58–59` (`Project`);
    - `util/math/vector.h:110` (`operator<`), `:191` (`Normalize`);
    - `s2pointutil.cc:32` (`IsUnitLength`), `:48` (`Ortho`), `:63–64` (`Rotate`'s checks);
    - `s2edge_crossings.cc:147` (`RobustCrossProd`), `:294` (`EnsureNormalizable`).

---

## Progress (2026-10-14, evening)

Done: Phases 1–3, the harnesses of US1 and US2, the CI wiring and its documentation.

- **Library changes** (Phase 3), each its own commit with a probe in debug and release builds:
  FR-017 `7072e65`, FR-018 `0ce304a`, FR-019 `d22cb11`, FR-020 `073d7b0`, FR-021 `36a4a9c`,
  FR-022 with `UPSTREAM.md` `a65c350`; the vectors and differential tests `94da57e`. `make test`
  passes.
- **The suite** (`f2b89de`): per change, `P=uvrect` 31 of 31 successful in 2 min 18 s and
  `P=point` 21 of 21 in 10 min 50 s on 4 jobs (its 516 s member since moved weekly).
- **Findings** (research R0, findings 4–7): the checker's `sqrt` is not a function, so
  P-PTORT-01, P-PTVAL-02's normalising clause and P-PTORT-03's `ortho` branch are tool limits;
  `from_coords` and `normalize` are proven per change for coordinates up to `2⁵¹¹` and weekly
  for the band; K12 (`normalize`'s zero signs follow Go) and the robust cross product's length
  past its double step are documented deviations; clause-heavy harnesses were split.

Done since: the weekly members (10 of 18 proven, 8 timed out at 60 minutes), the bounded
attempts on the near-unit box (T036: all 4 timed out), the mutant campaign (39 of 40 killed,
`M-539` without a verdict; `prop_from_coords_unit` catches it), the records (T026, T034,
T042–T044, T049–T051), the local halves of T046 and T047, the final run (T052; two spec 005
members moved weekly, and each part's harness filter anchored to its modules), the
quickstart (T053) and the success criteria (T054: 13 of 14 met, SC-001 except `from_coords`
and `normalize`).

T055's HIGH findings, resolved with the maintainer's decisions (2026-10-14, late evening):

- **Weekly CI for the value types:** eight `values-*` legs in the `slow` job run all 65 weekly
  members of specs 004–006, the 16 that time out included. `verify/known-timeouts.txt` lists
  those 16, which the runner reports as `KNOWN-TIMEOUT` and does not fail on.
- **The weekly mutant job:** 360 minutes, the most a hosted job may run, is less than the 226
  mutants take, so it runs one leg per spec (`make verify-mutants M='M-<N>*'`), with
  `VERIFY_SLOW_HARNESS_TIMEOUT=60m`. A mutant whose target times out unmutated too is
  `no-verdict` rather than `error`, and failed harnesses are no longer re-run for a
  counterexample, which had cost `M-540` an hour (its kill took 145 s, not the 59.9 minutes
  first recorded).
- **SC-003** was amended to leave out the two entries the checker's `sqrt` cannot state:
  26 of 28 (93%), met.
- **Spec 005's records** are complete, and `VERIFIED_TYPES.md` has sections for specs 004–006.

Open:

1. **T046, T047 on CI**, after a push.
2. **Push** (T056 committed `11a07e3`, `1da9428`, `49cbf07`, and the commits for T055): only
   with approval.
3. **T057–T059**: the user's decisions.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: the new modules, the Makefile parts, and the record sections (research R2;
contracts).

- [X] T001 Add the new modules to `verify/src/lib.rs` and update `verify/Cargo.toml`'s comment (research R2; contracts/harness-conventions.md "Modules and names").
  - **New files**, each starting with the two SPDX lines and
    `// Written for this crate (not ported from upstream S2).`, with a one-line `//!` naming
    its data-model section or property area:
    - `verify/src/uv_spec.rs` and `verify/src/uv_spec_tests.rs` (data-model §1.1);
    - `verify/src/point_spec.rs` and `verify/src/point_spec_tests.rs` (§1.2);
    - `verify/src/ur_{crash,val,mem,rel,set,mea,exp,eq}.rs`;
    - `verify/src/pt_{crash,val,cls,scl,ord,ari,ort,dst}.rs`.
  - **`verify/src/lib.rs`:**
    - `pub mod uv_spec;` and `pub mod point_spec;`, unconditional, next to
      `pub mod rect_spec;`;
    - `#[cfg(test)] #[path = "uv_spec_tests.rs"] mod uv_spec_tests;` in `uv_spec.rs`, and
      the same for `point_spec`;
    - the `ur_*` and `pt_*` modules behind `#[cfg(all(kani, feature = "values"))]`;
    - extend the crate doc's values paragraph: spec 006, `uv_spec`, `point_spec`, and the
      two prefixes.
  - **`verify/Cargo.toml`:** the `values` comment names spec 006, `--harness ur_`
    (`r2::Rect`) and `--harness pt_` (`Point`).
  - **Checks:** `cd verify && cargo build && cargo test`; `cargo build --no-default-features`;
    `cargo kani --only-codegen`. Codegen needs no cap.
- [X] T002 [P] Extend `make verify-values` with `P=uvrect` and `P=point` in `Makefile` (contracts/verify-commands.md).
  - `VALUES_HARNESSES` (line 228) maps `uvrect` → `--harness ur_` and `point` →
    `--harness pt_`, and nothing to all six parts.
  - `verify-values`'s help text: `[P=<cell|interval|real|rect|uvrect|point>]`, specs 004–006.
  - `F=values` in `SLOW_HARNESSES` already reuses `VALUES_HARNESSES`.
  - **Check:** `make -n verify-values P=point`; `make help | grep verify-values`.
- [X] T003 [P] Add the record sections in `verify/values/` (data-model §4, §5, §8; fact 11).
  - `INVENTORY.md`: section 5 "`r2::Rect` (33 entries)" and section 6 "`Point` (31
    entries)", with 004's columns and no rows yet.
  - `PROPERTIES.md`: section 6 "`r2::Rect`" and section 7 "`Point`", with 004's columns
    and no rows yet; the opening paragraph names spec 006.
  - `EVALUATION.md`: a part "Spec 006: `r2::Rect` and `Point`", with the headings of 005's
    part, "Composition (SC-010)" becoming "Composition (SC-014)", plus "Reach into the
    point's geometry" and "`ChordAngle` (FR-025)".
  - **Check:** `cd verify && cargo test --test property_kinds` passes.

**Checkpoint**: `make verify-values P=uvrect` and `P=point` run, with zero harnesses, and
pass. `cargo test` in `verify/` passes.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: the C++ reference vectors, the two reference descriptions with their tests, the
generators, and the order laws.

**⚠️ CRITICAL**: No user-story work starts until this phase is complete.

- [X] T004 Extend `tools/refgen-cpp/generate_values.cc` with four sections (research R9; fact 12).
  - **Header comment:** name spec 006, and document the new columns as the existing ones
    are documented.
  - **`# R2RECT_OPS`:** about 1,500 rows.
    - **Columns:** `a_x_lo,a_x_hi,a_y_lo,a_y_hi,p_x,p_y,m_x,m_y,max_error,b_x_lo,b_x_hi,b_y_lo,b_y_hi,results,note`.
    - **Rectangles:** half from `R2Rect::FromPointPair` over uniform `[−2, 2]`, small
      integers (ties) and wide magnitudes (`ldexp`); the rest built from two `R1Interval`s
      that are both empty or both not. Never call a constructor outside its `ABSL_DCHECK`
      domain.
    - **The other inputs:** a random point, a margin per axis (both signs, zero), an error
      bound, and a second rectangle.
    - **`results`**, `;`-separated:
      - 0 `is_valid`, 1 `is_empty`;
      - 2–9 `GetVertex(k)` x and y, k = 0..3;
      - 10–11 `GetCenter`, 12–13 `GetSize` (`-` for an empty rectangle);
      - 14 `Contains(p)`, 15 `InteriorContains(p)`;
      - 16–19 `AddPoint(p)`;
      - 20–21 `Project(p)` (`-` for an empty rectangle);
      - 22–25 `Expanded(m)`, 26–29 `Expanded(m_x)` (one margin);
      - 30 `Contains(b)`, 31 `InteriorContains(b)`, 32 `Intersects(b)`,
        33 `InteriorIntersects(b)`;
      - 34–37 `AddRect(b)`, 38–41 `Union(b)`, 42–45 `Intersection(b)`;
      - 46 `ApproxEquals(b, max_error)`, 47 `ApproxEquals(b)`, 48 `a == b`;
      - 49–52 `FromCenterSize(p, |m|)`.
  - **`# R2RECT_SPECIAL`:** the same columns.
    - **Inputs:**
      - `R2Rect::Empty()` and the empty rectangle's other representations;
      - sides `[0, 0]`, `[−0, 0]`, `[0, −0]`;
      - every pair of bounds per axis from `{−∞, −1, −0, 0, 1, ∞}` that keeps both axes
        empty or both non-empty;
      - margins `0`, `±1`, `±∞`;
      - every cell's u-v bound at levels 0–3 (`S2Cell::GetBoundUV`), against each other.
    - **`note`:** `deviation:infinite-corner` on a row where an operand bound is infinite and
      a result (10–13, 22–29, 46–47) is NaN or differs from the library's documented value
      (FR-018).
  - **`# S2POINT_OPS`:** about 1,500 rows.
    - **Columns:** `a_x,a_y,a_z,b_x,b_y,b_z,angle,results,note`.
    - **Points:** random unit points (`S2Testing`-style: three normals, normalised), half of
      the pairs nearly equal or nearly antipodal (`a ± small`, normalised); a random angle.
    - **`results`**, `;`-separated:
      - 0 `a < b`, 1 `b < a`, 2 `a == b`;
      - 3 `IsUnitLength(a)`, 4–6 `a.Normalize()`;
      - 7–9 `S2::Ortho(a)`;
      - 10–12 `S2::RobustCrossProd(a, b)`;
      - 13 `S1Angle(a, b).radians()`, 14 `S1ChordAngle(a, b).length2()`;
      - 15–17 `S2::Rotate(a, b, angle)`.
  - **`# S2POINT_SPECIAL`:** the same columns.
    - **Inputs:**
      - the zero vector and its signed variants; `(1, 0, 0)`, `(−0, 1, 0)` and the other
        axes; ties in the largest component (`(1, 1, 0)`, `(1, −1, 1)`);
      - subnormal, tiny (`2⁻²⁴²` and its neighbours) and huge (`1e154`, `1e200`) coordinates;
      - equal and antipodal pairs;
      - NaN and ±∞ in each position (results 0–6 only; `-` elsewhere);
      - for k in 300, 1023, 1024, 1074: `a = (2⁻ᵏ, 0, 1)` and `b = −a`, with
        `RobustCrossProd(b, a)` in results 10–12.
    - **`note`:** `deviation:robust-cross-overflow` on a row where C++'s robust cross
      product is not finite (FR-021, U-012).
    - Call `S2::Rotate` only for unit `a` and `b` (its `ABSL_DCHECK`s); `-` otherwise.
  - **`main()`:** call the four new functions **after** `RectSpecial()` (fact 12).
- [X] T005 Build and run the generator, append the sections to `core/tests/data/cpp_values_vectors.csv`, and document them (depends on T004).
  - **Build:**
    `cmake -B tools/refgen-cpp/build -DCMAKE_BUILD_TYPE=Release -DCMAKE_PREFIX_PATH=/tmp/s2-install`;
    `cmake --build tools/refgen-cpp/build --target generate_values`.
  - **Run:**
    `LD_LIBRARY_PATH=/tmp/s2-install/lib tools/refgen-cpp/build/generate_values > core/tests/data/cpp_values_vectors.csv`.
  - **Checks:**
    - run it twice and `cmp` the outputs;
    - every earlier line is unchanged
      (`git diff core/tests/data/cpp_values_vectors.csv | grep '^-[^-]'` is empty);
    - the CSV stays under 9 MB (it is not published, fact 12); cut the random rows if not.
  - **The U-012 rows:** check that `RobustCrossProd(−a, a)` for `a = (2⁻¹⁰⁷⁴, 0, 1)` gives
    `(NaN, ∞, NaN)` and for k = 300 gives `(−0, 1, 0)`. If C++ disagrees, stop and revisit K6
    with the user.
  - **`tools/refgen-cpp/README.md`:** add the four sections to the "Value types" table, with
    their columns and the Rust methods they check.
- [X] T006 [P] Write the u-v rectangle's description in `verify/src/uv_spec.rs` (data-model §1.1; research R3, R5).
  - **The functions:** `valid`, `empty`, `member`, `interior`, `contains`,
    `interior_contains`, `intersects`, `interior_intersects`, `vertex`, `finite`, built from
    `real_spec`, never from `r2::Rect`'s or `r1::Interval`'s methods.
  - **`valid(r)`** is `real_spec::empty(r.x) == real_spec::empty(r.y)` and no bound NaN
    (data-model §1.1). Also provide `emptiness_rule(r)`, the first half alone: P-QVAL-01
    compares `is_valid` with it, because a NaN bound passes `is_valid` (K4).
  - **`vertex(r, k)`** for `k mod 4` = 0, 1, 2, 3: `(x.lo, y.lo)`, `(x.hi, y.lo)`,
    `(x.hi, y.hi)`, `(x.lo, y.hi)`, upstream's counter-clockwise order (`r2rect.h`).
  - **Written from** `r2rect.h`. Each function's doc cites the upstream method.
  - **Booleans** are combined with `&&`, `||` and `==` only (004 R13).
- [X] T007 [P] Write the hand-made unit tests in `verify/src/uv_spec_tests.rs`.
  - Cover: empty with many representations; one axis empty and the other not (invalid); a
    NaN bound (invalid, though the emptiness rule holds); infinite bounds; `−0` against `0`;
    single-point sides; interior containment of a single point (false); containment of the
    empty rectangle in everything; the four vertices of `[0, 1] × [2, 3]` and `k = −1`,
    `i32::MIN`.
- [X] T008 [P] Write the point's description in `verify/src/point_spec.rs` (data-model §1.2; research R3; facts 2, 3, 7).
  - **The meaning:** `lex_less`, `order`, `same`, `max_abs`, `normalizable`, `normable`,
    `exponent`, `scaled_by`, `in_box`, `zero`, `ORIGIN`, exactly as data-model §1.2 defines
    them:
    - `lex_less(a, b)`: "C++'s `std::lexicographical_compare` over `(x, y, z)` with `<`:
      the first coordinate where `a_i < b_i` or `b_i < a_i` decides; a NaN pair decides
      nothing" (`util/math/vector.h:110`);
    - `same(a, b)`: "`a.x == b.x ∧ a.y == b.y ∧ a.z == b.z` (IEEE equality: signed zeros
      equal, NaN unequal)";
    - `normalizable(p)`: "`max_abs(p) ≥ 2⁻²⁴²`";
    - `normable(p)`: "every coordinate finite, and `x·x + y·y + z·z` (upstream's `Norm2`
      order) finite";
    - `scaled_by(p, k)`: "`p · 2^k` computed in two exact steps of at most `2^537` each";
    - `ORIGIN`: upstream's `S2::Origin()` coordinates as bit patterns.
  - **The restatements of upstream's arithmetic**, in a separate section of the module
    with its own doc (FR-007): `upstream_is_unit` (`fabs(Norm2() − 1) <= 5 * DBL_EPSILON`,
    products summed in upstream's order), `upstream_normalize`
    (`n = Norm(); if (n != 0) n = 1 / n; return p * n`), `upstream_scaled`
    (`ldexp(2, −1 − ilogb(max_abs(p))) * p` where it is finite).
  - **Never** call `Point`'s or `Vector`'s methods; read the fields.
  - **Booleans** are combined with `&&`, `||` and `==` only; orders are `Ordering` values.
- [X] T009 [P] Write the hand-made unit tests in `verify/src/point_spec_tests.rs`.
  - Cover:
    - the order: K8's NaN cycle `(1, 0, 0) < (NaN, 5, 0) < (0, 9, 0) < (1, 0, 0)`; signed
      zeros tie; the first differing coordinate decides;
    - `normalizable` at `2⁻²⁴²` and its predecessor; `normable` at `(0, 0, 1.3e154)` (true)
      and `(0, 0, 1.4e154)` (false);
    - `exponent` on normal and subnormal values, `2⁻¹⁰⁷⁴` included;
    - `scaled_by` for `(2⁻¹⁰⁷⁴, 0, 0)` by `2¹⁰⁷⁴` (exact) and for a normal vector;
    - `upstream_scaled` overflowing for `(2⁻¹⁰⁷⁴, 0, 1)`'s cross product (K6) and finite
      for `(2⁻³⁰⁰, 0, 0)`;
    - `same` on `(0, 0, 0)` against `(−0, −0, −0)` (true) and on NaN (false).
- [X] T010 Add the description checks for the new sections to `verify/tests/values_spec_vs_cpp.rs` (depends on T005, T006, T008; research R9).
  - **`uv_description_matches_cpp`**: for every `R2RECT_*` row, `is_valid` and `is_empty`
    (results 0, 1) equal `uv_spec`'s emptiness rule and `empty`; the memberships (14, 15),
    the four relations (30–33) and the vertices (2–9) equal `uv_spec`'s.
  - **`point_description_matches_cpp`**: for every `S2POINT_*` row:
    - `a < b` and `b < a` (results 0, 1) equal `point_spec::lex_less`, NaN rows included;
    - `IsUnitLength` (3) equals `upstream_is_unit`; `Normalize` (4–6) equals
      `upstream_normalize`, bit for bit;
    - the `deviation:robust-cross-overflow` rows: `upstream_scaled` is not finite exactly
      there.
  - Report the row counts per section, as the existing checks do.
  - **Check:** `cd verify && cargo test --test values_spec_vs_cpp`.
- [X] T011 Add the generators to `verify/src/value_domain.rs` (data-model §2; research R4; depends on T006, T008).
  - **The generators**, each built from `kani::any()` fields and the descriptions, never
    from the library's checks:
    - `any_uv_rect() -> r2::Rect`: both axes' bounds non-NaN, `uv_spec::valid`;
    - `any_finite_uv_rect()`: the same with finite bounds;
    - `any_uv_point() -> r2::Point`: two non-NaN coordinates;
    - `any_triple() -> Point`: three arbitrary `f64`, NaN and the infinities included;
    - `any_plain_triple()`: no NaN;
    - `any_finite_triple()`: finite;
    - `any_normable_triple()`: `point_spec::normable` (fact 2);
    - `any_box_point()`: `point_spec::in_box`;
    - `any_normalizable_box_point()`: `in_box ∧ normalizable`;
    - `any_finite_angle() -> Angle`: finite radians.
  - Each doc names its domain class (data-model §2) and why it is built that way.
  - **Check:** `cd verify && cargo kani --only-codegen`.
- [X] T012 [P] Add the generic order laws to `verify/src/laws.rs` (data-model §4, "Generic order laws"; contracts/harness-conventions.md "Statements").
  - A module `ord`, beside `eq`: `reflexive` (`Equal` with itself), `dual` (`cmp(b, a)` is
    `cmp(a, b)` reversed), `transitive` (on `Ordering` values: `a ≤ b ∧ b ≤ c ⇒ a ≤ c`, and
    the same for `<`), `ties_are_eq` (`cmp(a, b) == Equal` exactly when `a == b`), and
    `all`.
  - Generic over `T: PartialEq` and a comparison `Fn(&T, &T) -> Ordering`, so that
    `ChordAngle` can reuse them (FR-025).
  - Compare `Ordering` values, never booleans with `<` (Kani's bool ordering bug).
  - The module doc says which instance proves it (P-PTORD-02), as `eq`'s does.

**Checkpoint**: The descriptions pass their unit tests and the C++ rows. The generators and
the laws compile under Kani.

---

## Phase 3: Library Changes and Tests (FR-017 to FR-024), no story

**Purpose**: the documented domains and the fixed behaviour the proofs assume (research R6–R8).
Each change is its own commit (plan, Structure Decision).

**Confirm every documented behaviour with a concrete probe** in debug and release builds, in
a scratch crate, before committing; record the probe's output in the commit message (004
found a wrong panic claim this way).

- [X] T013 FR-017: document `r2::Rect`'s preconditions and check `from_center_size`'s in `core/src/r2/rect.rs`, with its test in a new `core/src/r2/rect_tests.rs` (K1, K2; research R7; fact 5).
  - **`# Panics`:**
    - `new` and `from_points`: in debug builds, when one interval is empty and the other is
      not; release builds return that invalid rectangle;
    - `project`: on the empty rectangle, in every build, through `r1::Interval::project`'s
      `f64::clamp` (fact 6). Upstream reaches `std::clamp`'s undefined behaviour in release
      builds (U-008).
  - **`from_center_size`:** document the precondition (both size components non-negative,
    not NaN), and add `debug_assert!(r.is_valid())` on the result, as upstream's checking
    constructor does (`r2rect.cc:25`). This changes debug builds only.
  - **`core/src/r2/rect_tests.rs`:** link it with
    `#[cfg(test)] #[path = "rect_tests.rs"] mod rect_tests;` at the end of `rect.rs`. A
    `#[cfg(debug_assertions)] #[should_panic]` test: size `(−1, 1)` at centre `(0, 0)`.
    Check that it **fails** before the check is added.
- [X] T014 FR-018: document the infinite and NaN corners on `r2::Rect` in `core/src/r2/rect.rs` (K4; research R7; depends on T005, T013).
  - An "Infinite bounds" paragraph on `center`, `size`, `expanded`, `expanded_by_margin` and
    `approx_eq_with`, each with its input and result (for example `size` of a rectangle with
    an `[∞, ∞]` side has a NaN component), and whether C++ computes the same: check against
    `R2RECT_SPECIAL`'s `deviation:infinite-corner` rows.
  - A paragraph on `Rect` itself: a NaN bound passes `is_valid` but contains nothing, not
    even the point it was built from; FR-019's deserialiser rejects it.
- [X] T015 FR-019: write the deserialisation regression tests in `core/src/r2/rect_tests.rs` (K3; research R6; depends on T013).
  - **Rejected, each error naming its rule:** `{"x": [1, 0], "y": [0, 1]}` (emptiness
    disagrees), the mirror, and a NaN bound (through a serde format that carries NaN, or a
    hand-built `serde_json::Value` if the format cannot).
  - **Accepted unchanged, bit for bit:** `Rect::empty()`, `[0, 1] × [2, 3]`, infinite
    bounds, `−0` bounds, single points.
  - **A round-trip test:** 10,000 rectangles from a seeded generator, each equal bit for bit
    after `serde_json` and back.
  - **Run** `cargo test -p s2rst --features serde r2::rect::rect_tests` and check that the
    rejection tests **fail** before T016.
- [X] T016 FR-019: implement the hand-written `Deserialize` for `r2::Rect` in `core/src/r2/rect.rs` (depends on T015; research R6).
  - Replace the derived `Deserialize` (line 41) with a hand-written one, keeping the derived
    `Serialize`, as `s2::Rect`'s (`core/src/s2/rect.rs:161`):
    - a private shadow struct with the same two fields, so the format is unchanged;
    - reject when "both empty or neither" fails, or a bound is NaN; the error names the rule
      ("empty", "NaN");
    - return the rectangle as read.
  - The doc comment on `Rect` says what deserialisation accepts and why.
  - **Check:** T015's tests pass, and `make test` passes.
- [X] T017 FR-020: write the scaling regression tests in a new `core/src/s2/point_tests.rs` (K5; research R8; data-model §6).
  - Link it with `#[cfg(test)] #[path = "point_tests.rs"] mod point_tests;` in `point.rs`.
  - **The cases** (data-model §6): `(x, 0, −x/2)` for x = 2⁻¹⁰²⁴ and 2⁻¹⁰²⁵ gives
    `(1, 0, −0.5)`, and `(2⁻¹⁰⁷⁴, 0, 0)` gives `(1, 0, 0)`, bit for bit; build the inputs
    with `f64::from_bits`, never `powi` (research R0's lesson: `powi` is inexact there).
  - **Unchanged:** a normalizable input is returned as is; the zero vector is returned as
    is; for largest components from `2⁻²⁴³` down to `2⁻¹⁰²²`, the result is
    `upstream_scaled`'s value, computed in the test with `ldexp`'s definition.
  - **Check** that the first three **fail** before T018:
    `cargo test -p s2rst --lib ensure_normalizable`.
- [X] T018 FR-020: fix `Point::ensure_normalizable` in `core/src/s2/point.rs` (K5; research R8; depends on T017).
  - `e = −ilogb(p_max)` with the existing `ilogb` (fact 7); the result is
    `self.0 * 2^(e/2) * 2^(e − e/2)`, each factor built from exponent bits and at most
    `2^537`, so normal, and each product exact. R0's `r0_pt_scale_draft` proved the draft.
  - The rustdoc states the domain (every non-zero, non-NaN finite vector), the postcondition
    (the largest component in [1, 2)), that the zero vector is returned unchanged, and that
    the result equals upstream's wherever upstream's is finite.
  - **Check:** T017 passes; `make test` passes.
- [X] T019 FR-021: write the shared-scaling regression tests in a new `core/src/s2/edge_crossings_tests.rs` (K6, U-012; research R8; depends on T018).
  - Link it with `#[cfg(test)] #[path = "edge_crossings_tests.rs"] mod edge_crossings_tests;`.
  - **Parity:** `robust_cross_prod(−b, b)` for `b = (2⁻³⁰⁰, 0, 1)` is `(−0, 1, 0)`, bit for
    bit, as C++ (it is `(−0, 0.5, 0)` before the change).
  - **The deviation:** for `b = (2⁻¹⁰⁷⁴, 0, 1)` it is finite, non-zero and normalizable,
    where C++ gives `(NaN, ∞, NaN)`.
  - **Check** that both **fail** before T020.
- [X] T020 FR-021: make `robust_cross_prod` use `Point`'s scaling in `core/src/s2/edge_crossings.rs` (K6, U-012; research R8; fact 8; depends on T019).
  - Delete the private `is_normalizable` and `ensure_normalizable` (lines 252–276); call
    `Point::is_normalizable` and `Point::ensure_normalizable` at lines 283, 345 and 347.
  - Move the inline tests that called the private copies (lines 1059, 1079, 1086) to the
    public functions, or delete them where `point_tests.rs` covers the same case.
  - `robust_cross_prod`'s rustdoc: a "Deviation from upstream" section for U-012, as
    `s1::Interval::expanded` has for U-011.
  - **Check:** T019 passes; `make test` passes.
- [X] T021 FR-022: document `Point`'s limits in `core/src/s2/point.rs`, and pin K10 and K11 in `core/src/s2/point_tests.rs` (K7, K8, K10, K11; research R7; facts 2, 3, 7, 9; depends on T018).
  - **`from_coords` and `normalize` (K7):** unit length only where the squared length is a
    normal number; the zero vector when it overflows (coordinates above about 1.34·10¹⁵⁴) or
    underflows, and a vector off unit length when it is subnormal
    (`from_coords(1e-160, 0, 0)` has x = 1.0000055664551362); infinite coordinates give NaN.
    Upstream's `Normalize` and Go's `PointFromCoords` compute the same.
  - **`cmp_point` and `PartialOrd` (K8):** a NaN coordinate counts as equal and the
    comparison moves on, so the order is not transitive with NaN, and `partial_cmp` can say
    `Equal` where `==` says unequal; C++'s `operator<` gives the same cycle.
  - **`is_normalizable`:** ignores a NaN coordinate (through `f64::max`); upstream's
    `std::max` chain depends on its position.
  - **The geometric operations:** the box or unit-length precondition each needs; `ortho`
    returns the zero vector for a non-normalizable input.
  - **`point_cross` (K10, K11):** replace the four properties copied from Go with the true
    ones:
    - the result is `ortho(p)` whenever `(p + q) × (q − p)` is exactly the zero vector:
      equal or antipodal points (K10: C++'s choice for equal points; Go returns the axis
      `Ortho`), distinct parallel points, and points so small that the products underflow
      (K11);
    - elsewhere `f(q, p) == −f(p, q)`, `f(−p, q) == −f(p, q)` and `f(p, −q) == −f(p, q)`
      as values (signed zeros may differ);
    - Go's `PointCross` documents the narrower exception and behaves the same (U-013).
  - **The pinning tests**, in `point_tests.rs`:
    - K10: `point_cross(p, p)` for `p = (1, 0, 0)` is `ortho(p)`, about
      `(0, −0.999986, 0.0053)`;
    - K11: for `p = (1, 0, 0)` and `q = (1 + 2ε, 0, 0)`, `q.point_cross(p)` is `ortho(q)`
      and not `−p.point_cross(q)`.
  - **Confirm K11 in Go** with a scratch module (Prerequisites) before committing, and
    record the output in the commit message.
- [X] T022 [P] FR-024: record U-012 and U-013 in `UPSTREAM.md` (K6, K11; research R0, R1).
  - **U-012**, in U-011's format: `s2edge_crossings.cc:294`, `EnsureNormalizable`'s
    `ldexp(2, −1 − ilogb(p_max))` overflowing below `2⁻¹⁰²³`; `RobustCrossProd(−b, b)` for
    `b = (2⁻ᵏ, 0, 1)` at k = 1024 and 1074 gives `(NaN, ∞, NaN)` (C++ Release, observed);
    the library's answer since FR-021. Mark it "candidate, not reported".
  - **U-013**: Go `s2/point.go:71–92`, `PointCross`'s documented properties (2)–(4) "unless
    p == op or p == −op", broken for distinct parallel points
    (`(1, 0, 0)`, `(1 + 2ε, 0, 0)`, both `IsUnit`); C++'s `RobustCrossProd` keeps them; the
    library keeps Go's behaviour and documents the true exception. Mark it "candidate, not
    reported".
  - Rows in the summary table, and K10 under "Upstream behaviours s2rst does not follow"
    (the library follows C++ for equal points, not Go).
- [X] T023 Extend `core/tests/cpp_values_diff.rs` to the new sections (depends on T005, T013–T021; research R9).
  - **`r2_rect_matches_cpp`:** every `R2RECT_*` row; bits for floats; `-` entries skipped.
    `deviation:infinite-corner` rows check the documented library value (T014), and the test
    says so in a comment.
  - **`s2_point_matches_cpp`:** every `S2POINT_*` row:
    - the order, equality, `is_unit`, `normalize` and `ortho`, bit for bit;
    - `robust_cross_prod` bit for bit, except `deviation:robust-cross-overflow` rows, where
      the result is finite, non-zero and normalizable (SC-010);
    - `chord_angle` bit for bit; `distance` (`S1Angle`) bit for bit in the default build
      (K9), except where a row differs only through the platform's `atan2`, then within
      2 ulps, with the reason in a comment;
    - `rotate` where C++ gave a value.
  - Report the counts, including the deviation rows.
  - **Check:** `cargo test -p s2rst --test cpp_values_diff`.

**Checkpoint**: `make test` passes. The documentation states every domain the proofs
assume, deserialisation rejects invalid rectangles, and the scaling is fixed and shared.

---

## Phase 4: User Story 1 — Proven crash freedom (Priority: P1) 🎯 MVP

**Goal**: Every in-scope operation of both types is proven free of panics, overflow,
undefined behaviour and firing debug checks over its documented domain (FR-004), with covers
showing the domains non-empty and the outcomes reachable (FR-009).

**Independent Test**: `make verify-values P=uvrect` and `P=point` report every crash harness
`SUCCESSFUL`, with all covers satisfied. `verify/values/INVENTORY.md` §5–6 names a harness
or a written reason for every row.

- [X] T024 [P] [US1] Write the u-v rectangle's crash harnesses in `verify/src/ur_crash.rs` (data-model §5; research R4; facts 5, 6).
  - **The 32 in-scope entries, grouped into harnesses** as 005's `rl_crash` (cheap
    accessors together). The domains:
    - `UvRect` (and `UvPoint`) for the constructors from intervals and points, `empty`,
      `lo`, `hi`, `is_empty`, `vertex_ij`, `vertices`, the memberships, the four relations,
      `add_point`, `add_rect`, `union`, `intersection`, `PartialEq`, `Default`;
    - four `AnyF64` bounds for `is_valid`, and every `i32` for `vertex`;
    - `FiniteUvRect` with finite margins for `center`, `size`, `expanded`,
      `expanded_by_margin`; a finite non-negative error bound for `approx_eq_with` and
      `approx_eq`;
    - `UvPoint` centre and a finite non-negative size for `from_center_size`, with T013's
      debug check proven never to fire;
    - a non-empty `UvRect` and a `UvPoint` for `project`;
    - `new` on two intervals that are both empty or both not, and `from_points` on corners
      whose two axes are both ordered (`lo ≤ hi`) or both reversed, proving their debug
      checks never fire there.
  - Each harness's doc starts with `C-r2::Rect::<op>:` and states its domain.
  - **Covers:** empty, single-point and infinite sides where in the domain; `expanded`
    returning the empty rectangle; `project` returning each bound and the point itself.
  - The probe's `r0_uv_expanded_crash_finite` and `r0_uv_from_center_size` are the drafts.
- [X] T025 [P] [US1] Write the point's crash harnesses in `verify/src/pt_crash.rs` (data-model §5; research R0, R4; facts 2, 4, 7).
  - **The 25 entries with crash proofs**: the 24 in scope and `rotate`. Each domain is
    maximal, as 005's T018: every triple except exactly those where an operation produces a
    new NaN (FR-004). The domains:
    - `AnyTriple` for `new`, `origin`, `vector`, `x`, `y`, `z`, the two `From`
      conversions, `cmp_point`, `PartialOrd`, `PartialEq`, `Default`, `is_normalizable` and
      `Neg`;
    - `AnyTriple` first for `is_unit` and `ensure_normalizable` (after T018), narrowed to
      `PlainTriple`, then `FiniteTriple`, only on a NaN-check failure that replays as a real
      NaN (fact 2); the data model's `FiniteTriple` is the floor;
    - `PlainTriple²` for `Add` and `Sub`, without a coordinate where they meet `∞ − ∞`
      (opposite infinities for `Add`, equal ones for `Sub`);
    - **`NormableTriple`** for `from_coords` and `normalize` (fact 2); the doc names the
      tool limit and the ordinary test of the rest (T021, K7);
    - `BoxPoint` for `ortho`, `reference_dir`, `point_cross`, `chord_angle`;
      `BoxPoint² × FiniteAngle` for `rotate`.
  - **The 5 entries through `atan2`** get no harness: their inventory rows say "tool limit
    (`atan2` unsupported, research R0)", with R0's 14.9 s failing run.
  - Each harness's doc starts with `C-s2::Point::<op>:` and states its domain.
  - **Covers:** a NaN and an infinite coordinate where in the domain; the zero triple
    (`from_coords` returning `origin`); `ensure_normalizable` scaling; `point_cross` taking
    its `ortho` branch.
  - The probe's `r0_pt_rotate_crash` is the draft.
- [X] T026 [US1] Run `make verify-values P=uvrect` and `P=point`, and fill in the crash columns of `verify/values/INVENTORY.md` §5–6 (depends on T024, T025, and on the open runs having finished).
  - **All 64 rows:** name, scope, reason, domain, crash result, crash harness.
  - **The 7 excluded or limited rows:** `Display` for both types (tool limit, as 004's), and
    the 5 `atan2` rows with R0's measurement and a pointer to T023's vector checks.
  - **`from_coords` and `normalize`:** the domain `NormableTriple`, and the tool-limit part
    with its test (fact 2).
  - Record each harness's time and peak memory from the run's `results.tsv`.

**Checkpoint**: US1 is complete and shippable. SC-001 and SC-002 hold.

---

## Phase 5: User Story 2 — Proven meaning, a product of proven intervals and an ordered vector (Priority: P2)

**Goal**: The 32 catalogue properties (data-model §4) are proven against the reference
descriptions, or recorded with a reason. Each proven one has a mutant that breaks it.

**Independent Test**:

- `make verify-values P=uvrect` and `P=point` report every property harness `SUCCESSFUL`;
  `make verify-slow F=values P=point` proves the weekly members.
- `make verify-mutants` reports every `M-5NN` killed.
- `cargo test --test property_kinds` passes.

### u-v rectangle properties

- [X] T027 [P] [US2] Write P-QVAL-01 to P-QVAL-04 in `verify/src/ur_val.rs` (data-model §4.1, §3).
  - **P-QVAL-01:** over four `AnyF64` bounds, NaN included: `is_valid` equals
    `uv_spec::emptiness_rule`, and so equals `uv_spec::valid` wherever no bound is NaN
    (data-model: "a NaN bound passes it (K4, documented by FR-018; FR-019's deserialiser
    rejects it)"). The probe's `r0_uv_valid_model` is the draft (it drew non-NaN bounds).
  - **P-QVAL-02:** every operation that returns a rectangle returns a `uv_spec::valid` one,
    over the domains of T024; split by operation as 005's `p_lval_02__*` if one is slow.
  - **P-QVAL-03:** each constructor against its documented rectangle, bit for bit; the
    empty rectangle is `[1, 0]²`; `Default` is empty; `is_empty` is `uv_spec::empty`.
  - **P-QVAL-04:** `lo`, `hi`, `vertex` for every `i32`, `vertex_ij` and `vertices` against
    `uv_spec::vertex`. The probe's `r0_uv_vertex` is the draft.
- [X] T028 [P] [US2] Write P-QMEM-01 and P-QMEM-02 in `verify/src/ur_mem.rs` against `uv_spec::member` and `interior` (the probe's `r0_uv_contains_point_model` is the draft).
- [X] T029 [P] [US2] Write P-QREL-01 and P-QREL-02 in `verify/src/ur_rel.rs` (data-model §4.1).
  - The probe's `r0_uv_contains_model` is the draft. Add interior containment, both
    intersections, and the symmetry of `intersects`.
- [X] T030 [P] [US2] Write P-QSET-01 to P-QSET-03 in `verify/src/ur_set.rs` (data-model §4.1).
  - **P-QSET-01:** the union contains both and is contained in every rectangle that
    contains both; `add_rect` equals `union`, bit for bit. Draft: `r0_uv_union_least`.
  - **P-QSET-02:** every point is in the intersection exactly when it is in both; the
    intersection is empty exactly when the two do not intersect. Draft:
    `r0_uv_intersection_common_points`.
  - **P-QSET-03:** `add_point` contains the point and the original, and is contained in
    every rectangle that contains both. Draft: `r0_uv_add_point`.
- [X] T031 [P] [US2] Write P-QMEA-01 in `verify/src/ur_mea.rs` (data-model §4.1).
  - Over `FiniteUvRect`: the centre of a non-empty rectangle is in it; `size` is the two
    lengths, bit for bit, non-negative for a non-empty rectangle.
- [X] T032 [P] [US2] Write P-QEXP-01 to P-QEXP-03 in `verify/src/ur_exp.rs` (data-model §4.1; fact 6).
  - **P-QEXP-01:** over `FiniteUvRect` and finite margins: valid; for margins with both
    components `≥ 0`, it contains the original; both `≤ 0`, it is contained in it (a zero
    margin is safe here: `r1::Interval::expanded` has no rounding allowance, fact 6); either
    axis empty gives the empty rectangle; `expanded_by_margin(m)` is `expanded((m, m))`, bit
    for bit. Draft: `r0_uv_expanded_contains` (28.2 s). Split into `__mpos` and `__mneg`
    (contracts "Split families") only if the first run puts it over 6 minutes.
  - **P-QEXP-02:** over a non-empty `UvRect` and a `UvPoint`: `project` gives the point when
    it is inside, otherwise the nearer bound per axis. Draft: `r0_uv_project_inside`.
  - **P-QEXP-03:** reflexive and symmetric over `FiniteUvRect²` and a finite non-negative
    error bound; `approx_eq` is `approx_eq_with(1e-15)`. Draft: `r0_uv_approx_eq_reflexive`.
- [X] T033 [P] [US2] Write P-QEQ-01 in `verify/src/ur_eq.rs` (data-model §4.1).
  - `laws::eq::all` over `any_uv_rect()³`, plus upstream's meaning: two rectangles are
    equal exactly when both intervals are, so every two empty rectangles are equal.
- [X] T034 [US2] Run `make verify-values P=uvrect`, and record times, memory and results in the spec-006 part of `verify/values/EVALUATION.md` ("Measurements") and the status column of `verify/values/PROPERTIES.md` §6 (depends on T024, T027–T033).
  - Move a member that takes over 6 minutes to the weekly tier, and record why.

### Point properties

- [X] T035 [P] [US2] Write P-PTVAL-01 to P-PTVAL-03 in `verify/src/pt_val.rs` (data-model §4.2).
  - **P-PTVAL-01:** over `AnyTriple`: `new`, the two conversions and the accessors keep the
    bits; `Default` is `(0, 0, 0)` with positive zeros; `origin` is `point_spec::ORIGIN`, bit
    for bit, and `upstream_is_unit`.
  - **P-PTVAL-02:** over `NormableTriple`: `from_coords` gives `origin` for every
    `point_spec::zero` triple and `normalize`'s result otherwise; `normalize` is
    `upstream_normalize`, bit for bit, and maps zero to zero. The doc says the finite triples
    whose squared length overflows are a tool limit (fact 2), tested by T021.
  - **P-PTVAL-03:** `laws::eq::all` over `any_plain_triple()³`.
- [X] T036 [P] [US2] Write P-PTCLS-01 to P-PTCLS-03 in `verify/src/pt_cls.rs` (data-model §4.2; research R12).
  - **P-PTCLS-01** (per change): over `PlainTriple`, `is_normalizable` is
    `point_spec::normalizable`. Draft: `r0_pt_is_normalizable_model`.
  - **P-PTCLS-02** (weekly, `#[cfg(feature = "slow")]`; 644.5 s in R0): over
    `FiniteTriple`, `is_unit` is `upstream_is_unit`. Draft: `r0_pt_is_unit_model`.
  - **P-PTCLS-03** (weekly): `from_coords` and `normalize` give a unit point
    (`upstream_is_unit`) for normable coordinates within the documented range (K7: the
    squared length a normal number); `ortho` and `reference_dir` give a unit point over
    `NormalizableBoxPoint`. If a member times out at 60 minutes, prove it within bounds (for
    example coordinates in `[2⁻⁵¹¹, 1]`) and record the gap.
- [X] T037 [P] [US2] Write P-PTSCL-01 and P-PTSCL-02 in `verify/src/pt_scl.rs` (data-model §4.2; research R8; depends on T018).
  - **P-PTSCL-01:** over `FiniteTriple`: `ensure_normalizable` returns a `normalizable`
    vector for every non-zero input, the input itself when it already is, and the zero
    vector for zero (SC-010).
  - **P-PTSCL-02:** over `FiniteTriple` not normalizable: the result is
    `point_spec::scaled_by(p, −exponent(max_abs(p)))`, bit for bit, its largest coordinate in
    [1, 2), and it equals `upstream_scaled(p)` wherever that is finite. Draft:
    `r0_pt_scale_draft` (113.8 s).
  - Split by `__sub` (largest coordinate below 2⁻¹⁰²²) and `__norm` only if the first run
    needs it.
- [X] T038 [P] [US2] Write P-PTORD-01 and P-PTORD-02 in `verify/src/pt_ord.rs` (data-model §4.2; depends on T012).
  - **P-PTORD-01:** over `AnyTriple²`, NaN included: `cmp_point` is `point_spec::order`, and
    `partial_cmp` is `Some` of it. Draft: `r0_pt_cmp_is_upstreams_order`.
  - **P-PTORD-02:** `laws::ord::all` with `cmp_point` over `any_plain_triple()³`. Drafts:
    `r0_pt_cmp_transitive`, `r0_pt_ties_are_equality`.
- [X] T039 [P] [US2] Write P-PTARI-01 in `verify/src/pt_ari.rs` (data-model §4.2).
  - Over `FiniteTriple²`: `Neg`, `Add` and `Sub` are the coordinate-wise IEEE-754
    operations, bit for bit; `−(−a)` is `a`; `a − b` is `a + (−b)`, bit for bit. Draft:
    `r0_pt_add_sub_neg` (150.5 s).
- [X] T040 [P] [US2] Write P-PTORT-01 to P-PTORT-03 in `verify/src/pt_ort.rs` (data-model §4.2; facts 3, 9).
  - **P-PTORT-01:** over `BoxPoint`: `point_spec::same(ortho(−a), −ortho(a))`, not bit
    for bit ("the zeros' signs may differ"); `reference_dir(a)` is `ortho(a)`, bit for bit.
  - **P-PTORT-02:** `ortho` is not the zero vector over `NormalizableBoxPoint`; `point_cross`
    is not the zero vector over `BoxPoint²` where `p` is normalizable. Draft:
    `r0_pt_ortho_nonzero` (358.0 s, the leg's longest; move it weekly if the first run puts
    it over 6 minutes).
  - **P-PTORT-03:** over `BoxPoint²`, wherever `(p + q) × (q − p)` is not the zero vector
    (computed in the description from the fields): `same(q.point_cross(p),
    −p.point_cross(q))`, `same((−p).point_cross(q), −p.point_cross(q))`,
    `same(p.point_cross(−q), −p.point_cross(q))`; and where it is the zero vector, the result
    is `ortho(p)`, bit for bit (K11). Cite U-013 in the doc.
- [X] T041 [P] [US2] Write P-PTDST-01 in `verify/src/pt_dst.rs`, and record P-PTDST-02 as not proven (data-model §4.2; fact 4).
  - **P-PTDST-01:** over `BoxPoint²` and `FiniteAngle`: `distance(p, p)` and
    `chord_angle(p, p)` are zero; `chord_angle` is symmetric, bit for bit, and its squared
    length is in [0, 4]; `approx_eq(p, p)`, `approx_eq_angle(p, p, e)` and
    `approx_eq_with(p, p, e)` hold (each reaches only `angle`'s early return). Drafts:
    `r0_pt_distance_self`, `r0_pt_chord_angle_range`.
  - **P-PTDST-02:** no harness. Its row says "not proven (tool limit: `atan2` unsupported,
    R0)", with R0's run, and points at T023's vector checks and a quickcheck property in
    `core/tests/property_tests.rs` (distance symmetric, default build), added here if none
    exists.
- [X] T042 [US2] Run `make verify-values P=point`, then `make verify-slow F=values P=point` for the weekly members, and record the results as in T034, in `verify/values/PROPERTIES.md` §7 (depends on T025, T035–T041).
  - Move a per-change member that takes over 6 minutes to the weekly tier, and record why.
  - A weekly member that times out at 60 minutes is "not proven (tool limit)" or proven
    within stated bounds, with the measurement.

### Non-vacuity, catalogue and index

- [X] T043 [US2] Write the `M-5NN` mutant patches in `verify/mutants/` and run them (data-model §7; research R10; contracts/harness-conventions.md "Mutant patches").
  - **The patches:** in 004's format (`Mutant`, `Targets`, `Expect-Fail`, `Defect`, optional
    `Tier: slow`), one defect each, from `M-501`, as data-model §7 lists, at least one per
    proven property (SC-004). Generate them with a script, as 004 did, so that the context
    lines match the formatted source.
  - **The FR-020 fix removed** must be killed by P-PTSCL-01.
  - **The three description mutants** patch `verify/src/uv_spec.rs` and
    `verify/src/point_spec.rs`.
  - **Run** `make verify-mutants M="$(cd verify/mutants && ls M-5*.patch | cut -c1-5 | tr '\n' ' ')"`,
    one campaign, after T034 and T042. Add a row per patch to `verify/MUTANTS.md`.
  - A mutant that survives means the harness is vacuous or too weak. Strengthen the harness
    until the mutant is killed; never delete the mutant. (`ortho` choosing the largest axis
    may survive P-PTORT-01 and -02, which hold for any axis: if so, add `ortho`'s exact value
    against a restatement of `S2::Ortho` to P-PTORT-01, checked against the C++ rows.)
- [X] T044 [US2] Complete `verify/values/PROPERTIES.md` §6–7 and §3, and `verify/PROPERTIES_BY_KIND.md`, and tag the harness docs (FR-010, SC-012).
  - **`PROPERTIES.md`:** all 32 rows, with 004's columns. Statuses come from T034 and T042;
    mutants from T043. §3 gains the order laws (`laws::ord`) and their instance.
  - **`PROPERTIES_BY_KIND.md`:** every entry under its kind, and the summary counts updated.
  - **Harness docs:** each property harness's doc starts with `P-ID (kind): <plain
    statement>`.
  - **Check:** `cd verify && cargo test --test property_kinds` passes.

**Checkpoint**: US2 is complete. SC-003, SC-004, SC-010 and SC-012 hold.

---

## Phase 6: User Story 3 — Regression protection for future changes (Priority: P3)

**Goal**: The proofs run in CI on relevant changes, and stay out of the default gate (FR-014,
SC-005).

**Independent Test**: a change that breaks a proven property fails the matching `verify` leg,
naming the property. The revert passes. An unrelated change does not start the legs.

- [X] T045 [US3] Wire the new parts into `.github/workflows/verify.yml` (contracts/verify-commands.md "CI").
  - **The matrix** (line 123): add `values-uvrect` and `values-point` to `suite`. The
    existing `values-*` case already maps them.
  - **Both `paths` lists:** add `core/src/s2/edge_crossings.rs` (FR-021 moved its scaling to
    `Point`); `core/src/r2/**`, `core/src/r3/**` and `core/src/s2/point.rs` are already
    listed.
  - **The header comment:** name spec 006 and the two new legs.
- [ ] T046 [US3] Measure both legs against SC-005, record them in `verify/values/EVALUATION.md`, and move members to the weekly tier until each leg is under 10 minutes on the reference runner (depends on T045; research R12).
  - Measure locally with `VERIFY_JOBS=4 make verify-values P=uvrect`, and the same for
    `P=point` (R0 estimated about 1 and 6 minutes). On CI, measure with
    `workflow_dispatch`, which needs the user's push approval.
  - Record the times in the spec-006 part of `verify/values/EVALUATION.md`.
- [ ] T047 [US3] Check US3's acceptance scenarios with a deliberate break in `core/src/s2/point.rs` (spec US3), reverted afterwards.
  - On a scratch change, make `cmp_point` skip the y coordinate.
  - `make verify-values P=point` fails, naming `pt_ord::p_ptord_01` with a concrete input.
    Revert it.
  - On CI (needs push approval), the `values-point` leg fails, and a change touching none of
    the paths does not start the leg.
- [X] T048 [P] [US3] Document the new parts in `verify/README.md` and `CONTRIBUTING.md`.
  - `verify/README.md`, the values section: `P=uvrect` and `P=point`, and spec 006.
  - `CONTRIBUTING.md`, "Formal verification": name `r2::Rect` and `Point`. Before changing
    `r2/rect.rs`, `s2/point.rs`, `r3/vector.rs` or `s2/edge_crossings.rs`'s scaling, run
    `make verify-values`.

**Checkpoint**: CI runs the new legs on relevant changes. `make test` is unchanged.

---

## Phase 7: User Story 4 — Evaluation of the value types as a whole (Priority: P3)

**Goal**: A maintainer who did not take part can answer SC-007's questions from the
evaluation alone (FR-016).

**Independent Test**: read the spec-006 part of `verify/values/EVALUATION.md` cold, and
answer "is `ChordAngle` next, and what must be decided first?", "type-level invariants
first?" and "what next?".

- [X] T049 [US4] Write the spec-006 part of `verify/values/EVALUATION.md` (research R13; FR-016, FR-025; SC-007, SC-014).
  - **Composition (SC-014):**
    - the share of `P-Q` entries whose statements are products of `P-R` statements;
    - the time saved: the reused `P-R` proofs' times against the `P-Q` proofs' times (005's
      method, research R5).
  - **Reach into the point's geometry:** what was proven over the box, what stopped at
    `atan2`, what needed `same` instead of bits, and which properties need certified error
    bounds instead of exhaustive checking.
  - **The findings:** K1–K11, U-012 and U-013, the CBMC division-by-infinity report
    (research R0), and any `D-NNN`.
  - **`ChordAngle` (FR-025):** a go or no-go, with the A/B/C validity question from the
    spec's "Deferred: `ChordAngle`" section that must be answered first.
  - **Type-level invariants:** both types have public fields; FR-019 closes
    deserialisation only.
  - **The assessment of 004–006 together**, and a ranked list of follow-ups, K11's
    alternative (T059) among them.
  - **Also:** effort, run times, limitations, the comparison with 001–005.
- [X] T050 [P] [US4] Extend `verify/TRUSTED_BASE.md` with a spec-006 section (research R11; FR-015).
  - **Reliance on 005:** `real_spec` and the generators reused; `r1::Interval`'s code,
    executed (not stubbed) inside every u-v rectangle proof; per `P-Q` entry, the `P-R`
    statements it composes (research R5).
  - **The checker's model:** IEEE-754 with `sqrt`; `sin` and `cos` over-approximated in
    [−1, 1]; `atan2` unsupported; the NaN checks, including the spurious division by ±∞
    (fact 2).
  - **The build:** the proofs cover the default build, whose products are unfused (K9).
  - **`point_spec`'s restatements** and their check against the C++ vectors; the box
    `[−1, 1]³` as the geometric domain.
- [X] T051 [P] [US4] Update `VERIFIED_TYPES.md`: move `r2::Rect` and `Point` under "Types with proofs", with the suite, the per-change and weekly parts, the tool limits, and links to the records.

**Checkpoint**: US4 is complete. SC-007 and SC-014 hold.

---

## Phase 8: Polish & Cross-Cutting Concerns

- [X] T052 Run the whole gate and the whole values suite once (`make test`, then `make verify-values` over all six parts), one after another, and record the run in the spec-006 part of `verify/values/EVALUATION.md` ("Final run").
- [X] T053 [P] Walk through `specs/006-formal-verify-r2-rect-point/quickstart.md` §1–§7, and correct the quickstart where reality differs (command names, counts, times).
- [X] T054 [P] Check the spec's success criteria SC-001 to SC-014 one by one against the records, and list the result per criterion in the spec-006 part of `verify/values/EVALUATION.md` ("Success criteria").
- [X] T055 [P] Re-run `/speckit-analyze` on `specs/006-formal-verify-r2-rect-point/`, and resolve any CRITICAL or HIGH finding before the PR.
- [X] T056 Commit the work in the order of "Commit plan" below. Push only with the user's approval.

---

## Phase 9: Follow-ups (FR-024) — recorded, not implemented in this feature

These tasks record upstream findings and decisions so that they are not lost. They need the
user's decision.

- [ ] T057 U-012: report C++ `RobustCrossProd`'s overflow for unit points upstream (K6; research R1).
  - Upstream: `s2edge_crossings.cc:294`, `EnsureNormalizable`'s
    `ldexp(2, −1 − ilogb(p_max)) * p`, called from the symbolic branch (lines 344, 346).
  - Input: `S2::RobustCrossProd(−b, b)` for `b = (2⁻¹⁰⁷⁴, 0, 1)`, a unit point: `(NaN, ∞,
    NaN)` in a Release build (T005's rows show it).
  - The library returns the correctly scaled vector since FR-021.
- [ ] T058 U-013: report Go `PointCross`'s documented sign laws upstream (K11).
  - Upstream: `s2/point.go:71–92` at `fe8a584`, properties (2)–(4) "unless p == op or
    p == −op".
  - Input: `p = (1, 0, 0)`, `op = (1 + 2ε, 0, 0)`, both `IsUnit`: `op.PointCross(p) ==
    p.PointCross(op)`, not its negation (observed).
  - Suggest either the true exception in the doc, or C++'s symbolic perturbation.
- [ ] T059 K11's alternative for `core/src/s2/point.rs`: decide whether `point_cross` should keep its sign laws for parallel points by perturbing them symbolically, as C++'s `RobustCrossProd` does (research R0 finding 3). A behaviour change with the cost of exact arithmetic, outside this feature; listed in T049's ranked follow-ups.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: no dependencies. T002 and T003 are [P] with T001.
- **Phase 2 (Foundational)**: needs Phase 1.
  - T005 needs T004. T010 needs T005, T006 and T008.
  - T006–T009 and T012 are [P].
  - T011 needs T006 and T008, because the generators use the descriptions.
- **Phase 3 (library changes)**: needs Phase 1. It can run alongside Phase 2.
  - `rect.rs` and `rect_tests.rs`: T013 → T014 (also needs T005) → T015 → T016.
  - `point.rs` and `point_tests.rs`: T017 → T018 → T021.
  - `edge_crossings.rs`: T019 (needs T018) → T020.
  - T022 is [P].
  - T023 needs T005 and T013–T021.
- **Phase 4 (US1)**: needs T011, and Phase 3's T013, T016, T018 and T020 (the documented
  domains and the fixed behaviour). T026 also needs the open runs of 004 and 005 to have
  finished.
- **Phase 5 (US2)**: needs Phases 2 and 3. The runs T034 and T042 follow T026. The mutant run
  T043 needs every property harness.
- **Phase 6 (US3)**: needs the harnesses it runs (US1, US2).
- **Phase 7 (US4)**: needs US1–US3's measurements.
- **Phase 8 (Polish)**: needs everything above.
- **Phase 9**: T057–T059 can be taken up at any time, with the user.

### User Story Dependencies

- **US1 (P1)**: independent after Phases 2–3. It is the MVP.
- **US2 (P2)**: needs US1's modules only for the shared generators. The property harnesses
  stand alone, and each one also discharges its operations' crash obligations.
- **US3 (P3)**: needs the suites it runs.
- **US4 (P3)**: needs everything it evaluates.

### Parallel Opportunities

- Phase 1: T002 and T003.
- Phase 2: T006, T007, T008, T009 and T012.
- Phase 3: the three file chains (`rect.rs`; `point.rs`; `edge_crossings.rs` after T018) and
  T022.
- US1: T024 and T025.
- US2: T027–T033 and T035–T041 (different modules). The Kani runs go one after another.
- US4: T050 and T051.
- Polish: T053–T055.

---

## Parallel Example: User Story 2

```text
# After Phase 3, write these side by side; run them one at a time, after the open runs:
Task: "T027 [US2] P-QVAL-01..04 in verify/src/ur_val.rs"
Task: "T030 [US2] P-QSET-01..03 in verify/src/ur_set.rs"
Task: "T032 [US2] P-QEXP-01..03 in verify/src/ur_exp.rs"
Task: "T037 [US2] P-PTSCL-01..02 in verify/src/pt_scl.rs"
Task: "T038 [US2] P-PTORD-01..02 in verify/src/pt_ord.rs"
Task: "T040 [US2] P-PTORT-01..03 in verify/src/pt_ort.rs"
```

---

## Implementation Strategy

### MVP first (User Story 1 only)

1. Phase 1, then Phases 2 and 3 (they can overlap).
2. Phase 4 (US1): every in-scope operation proven crash-free, with K1–K11 documented,
   FR-019 to FR-021 fixed, and the `atan2` entries recorded as tool limits.
3. **Stop and validate:** `make verify-values P=uvrect` and `P=point` pass, and the inventory
   is complete.

### Incremental delivery

1. Setup, foundation and library changes: the vectors, the descriptions, the documentation,
   the validating deserialiser, the fixed and shared scaling.
2. US1: crash freedom.
3. US2: meaning, with the mutants.
4. US3: CI.
5. US4: the evaluation.

Each step leaves `make test` and `make verify` green.

### Commit plan

Separate commits, in this order:

1. the spec 006 documents (`specs/006-formal-verify-r2-rect-point/`);
2. T013 (FR-017), then T014 (FR-018);
3. T015 with T016 (FR-019);
4. T017 with T018 (FR-020);
5. T019 with T020 (FR-021);
6. T021 (FR-022) and T022 (`UPSTREAM.md`);
7. the vectors (T004, T005, T023);
8. the `verify/` suite (Phases 1–2, US1, US2);
9. the CI wiring (T045, T048);
10. the records (T026, T034, T042–T044, T049–T051).

Push only with the user's approval.
