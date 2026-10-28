# Evaluation: Formal Verification of `Cell` and `s1::Interval` (spec 004, FR-016)

## Summary

**Interim, 2026-10-12** (spec 004 implementation paused at a checkpoint; T046 completes this
file). Every in-scope operation of `Cell` (21) and `s1::Interval` (30) is proven crash-free over
its whole domain. 31 of the 40 catalogue properties are proven over their whole domain; the
other 9 are proven within stated bounds or have members waiting for the weekly tier. The FR-020
containment proof (P-IMEA-01) is proven against the library: the seam fix changes the length of
exactly one valid interval (213 s). The mutant campaign has not run yet.

## Effort log

## Exclusions and tool limits (FR-002, FR-003)

## Bounds reached

## Floating-point cost

## Upstream findings (FR-022)

- **U-007**: `S1Interval::GetLength` gives −1 for the one-ulp interval across the seam (K4),
  and `Expanded`, `ApproxEquals`, `Intersection` and `S2LatLngRect::Area` inherit it. Fixed in
  the library as a documented deviation (FR-020); follow-up task T053 to report it.
- **U-011** (2026-10-13): `S1Interval::Expanded` can cross its bounds when shrinking an
  inverted arc to almost nothing, and return almost the full circle; Go's `Interval.Expanded`
  is the same code. Found by the weekly member `p_iexp_01_shrink__xinv` in 778 s. Fixed in the
  library as a documented deviation (FR-023, D-007); follow-up task T057 to report it.
- U-004 gained a note: `S2Cell(S2CellId)` itself accepts invalid ids.

## `# Panics` documentation added (SC-008)

- `Cell::from_cell_id`, `from_face`, `from_face_pos_level`, `impl From<CellId>` (K1). Probing
  for the text found that an id whose lowest set bit is at an odd position does not panic but
  gives a cell with unspecified fields; the documentation says so.
- `s1::Interval::new`, `from_point_pair`, `contains`, `interior_contains`, `add_point`,
  `project` (K3) and `expanded` (K5, a NaN margin).

## Defects found

- **D-005** (planning probe, Kani crash proof): `directed_hausdorff_distance` built its helper
  arcs with `new_checked`, which keeps a −π bound; for a short `y` near 0 the complement's
  centre rounds to −π and the debug check fired on valid input. Fixed (`7977a32`), release
  results unchanged.
- **D-006** (the generic trait tests, brought in at analysis): `Rect::cap_bound` ported Go's
  `math.Remainder(...) >= 0` as `rem_euclid`, so every rectangle across the antimeridian took
  the centre cap; a 248° one got a cap that misses part of it. Fixed (`f7d78d1`).
- **FR-018 (K2)** and **FR-020 (K4)**, the survey's two behaviour defects, fixed (`ca18237`,
  `157eb81`).
- **D-007** (the weekly member `iv_exp::p_iexp_01_shrink__xinv`, 2026-10-13): `expanded`
  shrank `[−0.4669189453124994, −2.7499821821795862]` by `−2.0000610351562496` to
  `[1.5331420898437502, 1.53314208984375]`, an inverted interval of length 2π, where the true
  result is an arc of 5.2e-16. Upstream's 2·ε allowance is smaller than the rounding of
  `hi + margin` and of the float `2π`, so the bounds cross (U-011; C++ and Go give the same).
  A search of the margins just past the emptiness test found 182 more in 15 million, all on
  inverted arcs, and none growing in 15 million. `Rect::expanded` and `expanded_by_distance`
  inherit it through the longitudes. Fixed by FR-023: upstream's result is kept when it keeps
  containment, and is otherwise empty (shrinking) or full (growing); P-IEXP-03 states the fix
  narrow, but its two members time out at 60 minutes (2026-10-14), so it is not proven (tool
  limit). P-IEXP-01's containment members prove on the fixed library in 21–63 s each.

## Measurements

## Comparison with 001–003

## Limitations

## Recommendations (FR-016)

## Notes from implementation

- **A Kani soundness bug.** Kani 0.68.0 / CBMC 6.11.0 mis-models `<` and `<=` on symbolic
  `bool`s, and so any tuple comparison with a `bool` component: `assert_eq!(a <= b, !a || b)`
  fails, while `bool::partial_cmp`, `==` and concrete constants are fine. The first draft of
  `arc_spec` ordered points by a `(bool, f64)` key, and 8 property harnesses reported failures
  whose counterexamples passed when replayed concretely. `arc_spec::ccw_le`/`ccw_lt` now use
  float comparisons and boolean equality only. Nothing in the 001–003 harnesses or models
  orders booleans (checked). Recorded as KI-001 in `KANI_ISSUES.md` (2026-10-25).
- **Real arcs, not float sets.** `[next_up(−π), π]` holds every float, but C++ says it does not
  contain the full circle: the reals between −π and `next_up(−π)` are outside it. The first
  draft characterised containment by float witness points, and the C++ vectors refuted it
  (T010). The relations are now those of real arcs, by the cyclic order of the bounds.
- **A sub-ulp gap.** P-IMEA-06's first statement ("the distance is 0 exactly when `y`
  contains `x`") is false in floats: across a one-ulp gap of an inverted `y`, every point of `x`
  is within half an ulp of `y` and the distance rounds to 0. The statement now has that
  direction only, with a cover for the case.
- **What is expensive.** Comparisons and additions prove in seconds; `ieee_remainder` (a
  division and a `round`) in `expanded` and `approx_eq_with` makes their two-interval
  properties time out at 12 minutes, even split four ways; squaring a symbolic float limits the
  `Cell` bound properties to low levels (levels 0–3 per change).
- **The exhaustive projection check** covers all `2^30 + 1` boundary values in 37 s in a debug
  build.

## Acceptance (SC-001 … SC-013)

---

# Spec 005: `r1::Interval` and `s2::Rect`

## Summary (spec 005)

**2026-10-14** (T042, written after spec 006's implementation; the records of 2026-10-13 and 14
are its sources). Every in-scope operation of `r1::Interval` (26) and `s2::Rect` (34) is
proven crash-free over its whole documented domain, whose arithmetic operations leave out
exactly the infinite corners that FR-017 documents. The rectangle's 18 floating-point geometry
operations and both `Display`s are excluded, with reasons. 30 of the 31 catalogue entries are
proven over their whole domain, 3 of them with weekly members; P-LEXP-02's symmetry times out
at 60 minutes (a tool limit), as spec 004's P-IEXP-02 does for the longitudes. The
rectangle's statements are products of the real interval's description (`real_spec`) and spec
004's `arc_spec`. The library gained the documentation of FR-017 to FR-019 and one fix,
FR-020: deserialisation rejects an invalid rectangle. There are three upstream findings, U-008
(undefined behaviour in `R1Interval::Project`) and U-009 and U-010 (wrong results at infinite
or huge bounds), and no `D-NNN`. The mutant campaign killed all 43 mutants.

## Composition (SC-010) (spec 005)

- **Share.** 12 of the 16 rectangle entries (75%) are stated through `arc_spec`, spec 004's
  description of an arc, by way of `rect_spec`. They are validity (P-LVAL-01 to -03),
  membership (P-LMEM-01, -02), the relations (P-LREL-01, -02), the set operations (P-LSET-01
  to -04) and expansion (P-LEXP-01). Two more rest on spec 004's other results. P-LEQ-01
  instantiates its generic `laws::eq`. P-LBND-01 states the centre and size through
  `s1::Interval::center` and `length`, whose meaning P-IMEA-01, -02 and -04 prove. So 14 of 16
  (88%) rely on spec 004. P-LEXP-02 and P-LEQ-02 stand alone.
- **Time.** Spec 004's proofs that tie `arc_spec` to `s1::Interval` (P-IVAL-01 to -05,
  P-IMEM-01 and -02, P-IREL-01 to -06) take 36 s per change (2026-10-13). The 12 composed
  rectangle proofs take 458 s per change and 286 s weekly. As in spec 006, composition saves
  the description and its validation (`arc_spec`, 281 lines, checked against the C++
  vectors), not proof time: every rectangle proof executes `s1::Interval`'s code (no stubs).
- **Against spec 004's times.** A product statement costs about its longitude factor,
  because the latitude factor is cheap:
  - union: 33.7 s, against 30.0 s for the interval's (P-ISET-02);
  - intersection: 55.4 s, against 60.7 s (P-ISET-03);
  - adding a point: 29.4 s, against 16.1 s (P-ISET-04);
  - intersection relations: 24.4 s, against 18.3 s (P-IREL-04 and -06);
  - containment relations: 6.1 s, against 5.0 s (P-IREL-02 and -03);
  - membership: 1.1 s and 2.4 s, against 0.5 s and 1.9 s.
- **The counterpoint (R0).** Stated component-wise, through `lat.union` and `lng.union` and
  their proven properties, the rectangle's union timed out at 12 minutes. Stated directly
  against the product description, it proved in 48 s (33.7 s now). Citing the components'
  proven operations would not have made the proofs cheaper. Stating them against a composed
  description did.

## Effort log (spec 005)

- **2026-10-12**: the survey (K1–K4, with concrete probes); the specification and planning;
  the planning probe (R0, 20 harnesses) and the serde evidence (R6, 1,001,444 rectangles).
- **2026-10-12, 23:22, to 2026-10-13, 06:57**: the documents (`9cc4b5e`), the library changes
  FR-017 to FR-020 with their probes and tests (`894245a`, `190802e`, `37fa0a2`, `a26cccf`),
  the C++ vectors (`2a1d093`), the descriptions and the 64 harnesses, the 43 mutants and the
  CI legs (`cb71623`, `ff49f4c`).
- **2026-10-13, 10:19–10:44**: the first per-change run; its three timeouts were split or
  moved weekly.
- **2026-10-13, 18:34–23:05**: spec 004's U-011 fix, then the re-runs on the fixed library
  (19:24–19:45) and the mutant campaign (21:56–23:05, after spec 004's `M-343` to `M-345`).
- **2026-10-13, 23:05, to 2026-10-14, 03:14**: the weekly members, `p_lexp_01__mneg`'s
  corrected domain, and `M-414` at the weekly limit.
- **2026-10-14, 20:13–20:24**: during spec 006's final run, two members moved weekly and both
  parts were re-run (`1da9428`).
- **2026-10-14, evening**: these records, the deliberate break (T040) and the final run.

## Exclusions and tool limits (FR-002, FR-003) (spec 005)

- **Both types' `Display`**: formatting, as 001's and 004's.
- **The rectangle's 18 floating-point geometry operations** (FR-003): `area`,
  `contains_point`, `interior_contains_point`, `expanded_by_distance`,
  `expand_for_subregions`, `cap_bound`, `cell_union_bound`, `centroid`,
  `boundary_intersects`, the four distances, and the region interface's `cap_bound`,
  `cell_union_bound`, `contains_cell`, `intersects_cell` and `contains_point`. They are tested
  by the `region_rect_*` tests of `core/tests/trait_property_tests.rs`, and `cap_bound` by
  D-006's `rect_tests.rs`. Spec 004's `laws::region` go through `contains_cell` and
  `intersects_cell`, so they are not instantiated for the rectangle.
- **The infinite corners** (FR-017) are outside the arithmetic operations' domains, not tool
  limits: `∞ − ∞` gives NaN, which Kani reports as a failed check. Each is documented, and
  the C++ rows marked `deviation:infinite-corner` check them.
- **P-LEXP-02's symmetry** times out at 60 minutes (a tool limit).
- **The binary codec** of `s2::Rect` is spec 003's.

## Floating-point cost (spec 005)

- **Seconds**: every real-interval statement except the centre and the approximate
  equality's default form; the rectangle's validity, membership, containment, polar closure
  and equality.
- **Tens of seconds to minutes**:
  - the real interval's centre (82.8 s, `f64::midpoint`'s branches) and its approximate
    equality's default form (79.9 s);
  - the rectangle's union, intersection and intersection relations (24–55 s);
  - the rectangle's expansion validity (122.7 s) and the constructor from a centre and size
    (118.3 s);
  - the rectangle's approximate equality, reflexive (240.5 s).
- **Weekly**:
  - the real interval's approximate equality, symmetric (564.5 s);
  - the rectangle's bounds bit for bit (673.1 s): both intervals' centres and lengths at once;
  - the rectangle's expansion containment (130 s and 156 s).
- **Over an hour**: the rectangle's approximate equality, symmetric. It meets the longitude's
  `ieee_remainder`, as spec 004's P-IEXP-02 does.
- **The pattern**, as in spec 004: comparisons and selections take seconds. A difference
  compared with another difference (the approximate equalities), or a remainder (the
  longitude's expansion and approximate equality), takes minutes to hours.

## Upstream findings (FR-022) (spec 005)

- **U-008** (fact 4): C++ `R1Interval::Project` on an empty interval reaches `std::clamp` with
  `hi < lo` in release builds, which is undefined behaviour (`r1interval.h:173–176`). The
  library's `project` panics in every build, documented (FR-019). Follow-up T050.
- **U-009** (fact 3): C++ `R1Interval::GetDirectedHausdorffDistance` at infinite bounds gets
  the NaN of `∞ − ∞` and returns 0: for example, from `[0, ∞]` to `[5, ∞]`. The library's
  `f64::max` drops the NaN and returns the true distance, a documented deviation; the C++
  vectors confirmed it. Follow-up T051.
- **U-010** (found by the vectors, T005): C++ `R1Interval::GetCenter` is `0.5 * (lo() +
  hi())`, which overflows for finite bounds beyond `DBL_MAX / 2`: the centre of
  `[DBL_MAX, DBL_MAX]` is ∞. The library's `f64::midpoint` gives the true centre, a documented
  deviation. The C++ rows are marked `deviation:center-overflow`. Follow-up T052.
- All three are recorded in `UPSTREAM.md` and have not been reported. With them, K2's
  deliberate deviation, which follows Go, is in `UPSTREAM.md`'s table of upstream behaviours
  the library does not follow.

## `# Panics` and infinite-corner documentation added (SC-008) (spec 005)

- **`r1::Interval`** (FR-017, FR-019; `894245a`, `a26cccf`):
  - the infinite corners of `length`, `center`, `expanded`, `approx_eq_with`, `approx_eq` and
    `directed_hausdorff_distance`. The centre is NaN for `[−∞, ∞]` and for the empty
    `[∞, −∞]`, and approximate equality with an infinite bound is false even against itself;
  - the two deviations (U-009, U-010), and the sign of a zero bound that `min` or `max`
    chooses;
  - `project`'s panic on an empty interval in every build (U-008).
- **`s2::Rect`** (FR-018, FR-019; `190802e`):
  - `add_point` and `contains_lat_lng` ignore an invalid latitude–longitude, a deviation from
    C++ that follows Go (K2);
  - the debug-only checks of `from_lat_lng`, `from_point_pair`, `interior_contains_lat_lng`,
    `expanded` and `from_center_size`, what release builds return, and the latitude that no
    build checks (K3).
- Every statement was probed in debug and release builds before its commit.

## Defects found (spec 005)

- **K4, fixed by FR-020** (`37fa0a2`): the derived `Deserialize` accepted any four bounds,
  while the binary decoder rejects an invalid rectangle (spec 003). A hand-written
  implementation reads the same fields and rejects a latitude outside [−π/2, π/2], an invalid
  longitude interval, or latitudes and longitudes that disagree on emptiness. The three
  rejection tests in `rect_tests.rs` fail without it. No tolerance is needed: none of R6's
  1,001,444 valid rectangles came back invalid through `serde_json`'s default parsing.
- **No harness found a defect outside the survey**: there is no `D-NNN` for spec 005. Spec
  004's D-007 (U-011) reaches the rectangle through `Rect::expanded`'s longitudes. Its fix
  (spec 004's FR-023) covers the rectangle, and the weekly `p_lexp_01__mneg` checks it.

## Measurements (spec 005)

On the workstation (8 threads, 30 GB), one Kani run at a time:

| Run | Harnesses | Result | Wall time |
|-----|-----------|--------|-----------|
| first per change, `P=real` then `P=rect` (2026-10-13; 4 jobs, 12 min each) | 62 | 59 successful; `ri_exp::p_rexp_03`, `rl_exp::p_lexp_02_symmetric` and `rl_bnd::p_lbnd_01` timed out | 12 min, 13 min |
| per change on the fixed library (2026-10-13, after the split) | 26, 37 | all successful | 8 min, 13 min |
| per change, `P=real` and `P=rect` alone (2026-10-14, after the module-name fix, 2 members weekly) | 25, 36 | all successful; 21 crash harnesses, every cover satisfied | 2 min 20 s, 5 min 37 s |
| weekly members (60 min each, 8 GiB) | 5 | 4 proven (130–673 s), `p_lexp_02_symmetric` timed out | 1 h 25 min of harness time |
| mutant campaign `M-401` … `M-443` (12 min; `M-414` again at 60 min) | 43 mutants | 43 killed | 69 min, and 32 min for `M-414` |

- **The longest per-change members**: `rl_exp::p_lexp_02_reflexive` 240.5 s,
  `rl_val::p_lval_02__expanded` 122.7 s, `rl_exp::p_lexp_01_center_size` 118.3 s,
  `rl_crash::c_expanded` 87.8 s.
- **Memory**: the runner records no peak memory. Every harness stayed under its ceiling (4 GiB
  per change, 8 GiB weekly).
- **Per harness**: the times are in the catalogue's Bound column and the inventory.
- **SC-005**: both legs are under 10 minutes on the workstation. A hosted runner has 4
  vCPUs, half the workstation's threads, so `P=rect` may come close; the CI measurement needs
  a push (T039).

## Comparison with 001–004 (spec 005)

- **001–003** proved integer and byte-level code. **004** proved the first value types with
  floating-point bounds, whose operations compare and select. **005**'s real interval is the
  simplest float type left, and it proved as easily as 004's types: every operation and every
  entry, in seconds, except the approximate equality.
- **The rectangle is the first composite type.** Its statements were composed from two
  descriptions. They cost about their longitude factor (see Composition), and the
  description was reused rather than restated. The approach carried over unchanged to spec
  006's `r2::Rect`, whose 14 of 16 entries rest on this spec's proofs.
- **What stayed hard** is what was hard in spec 004: the approximate equalities, and an
  expansion that meets the longitude's remainder.
- **Mutants**: 43, all killed (004: 42 killed, and `M-343` to `M-345` added for its U-011
  fix).

## Limitations (spec 005)

- **The infinite corners** are documented and checked against C++, not proven: the arithmetic
  operations' domains leave them out (FR-017).
- **P-RMEA-02 holds over finite intervals.** At infinite bounds the library deliberately
  differs from upstream (U-009), and that corner is documented, not proven.
- **The rectangle's floating-point geometry** has no proofs: area, distances, `cap_bound` and
  the region interface's cell methods rely on property tests and D-006's tests.
- **`s2::Rect` has public fields.** Any caller can build a rectangle that `is_valid` rejects.
  FR-020 closes deserialisation only; the binary decoder already rejected one (spec 003).
- **SC-005** is measured on the workstation only.

## Recommendations (FR-016) (spec 005)

The answers to SC-007's four questions. They were written after spec 006, which followed the
first two:

1. **Continue with the value types?** Yes. The real interval and the rectangle proved 30 of
   31 entries and every crash obligation in about 3 hours of machine time, mutants included,
   and the rectangle
   showed that composite types can reuse their components' descriptions.
2. **With which approach?** Kani against independent reference descriptions, a composite
   type's description built from its components'. Statements that compare two float
   differences go to the weekly tier, at 60 minutes. Stating a property component-wise,
   through the components' operations, is slower (R0).
3. **Type-level invariants first?** No. The proofs take validity as a precondition and prove
   that every operation keeps it (P-LVAL-02). Deserialisation and the binary decoder reject
   invalid input. Private fields with checked constructors would make the rule a property of
   the type, but that is a breaking API change, and nothing in the proofs needs it. It is for
   the maintainer to decide after the value types.
4. **What next?** Ranked:
   1. **`r2::Rect`**: two real intervals, so `real_spec` and every proof here carry over.
      Spec 006 took it and proved all 16 entries, 14 through this spec's proofs.
   2. **`Point`**: its order, classifications and exact arithmetic look within reach; its
      geometry needs research. Spec 006 proved 10 of its 16 entries.
   3. **`ChordAngle`**: not until its validity question is answered (it accepts every
      negative value). The maintainer deferred it on 2026-10-13.
   4. **Report U-008 to U-010 upstream** (T050–T052), with the maintainer's approval.
   5. **Measure SC-005 on CI** (T039, T040), which needs a push.

## Notes from implementation (spec 005)

- **Splits after the first run.** `ri_exp::p_rexp_03` was split by clause into
  `_reflexive`, `_symmetric` and `_default`, and `M-416` and `M-417` name the clause they
  break. `rl_exp::p_lexp_02_symmetric` moved weekly. `rl_bnd::p_lbnd_01` proved in 693.6 s in
  the next run.
- **`p_lexp_01__mneg`'s domain.** With a zero longitude margin of either sign, upstream's
  `S1Interval::Expanded` takes its growth branch, so an arc within 2·ε of full becomes the
  full circle (C++ and Go agree). The member failed on that, not on the library. It now
  assumes a negative longitude margin, as spec 004's P-IEXP-01 does, and proves in 156 s.
- **Zero signs.** `f64::min` and `max` may choose a zero bound of the other sign than
  upstream's `std::min` and `max`, with the same value. It is documented, and the
  differential test compares those bounds by value.
- **The empty `[∞, −∞]` has a NaN centre** too, as upstream's. The crash harness found it, and
  `a26cccf` documents it.
- **P-LEQ-01** gained upstream's meaning of equality: with the generic laws alone, a mutant
  survived.
- **The part selection.** Research R2 checked that the new prefixes `ri_` and `rl_` match no
  existing harness, but not the reverse. Kani matches `--harness` anywhere in a name, so from
  this spec on `P=interval`'s `iv_` also ran the new `ri_val::p_riv_*`. Spec 006's final run
  found it, and the Makefile now selects by module name (`11a07e3`).
- **Two members over the 6-minute rule.** `ri_exp::p_rexp_03_symmetric` (474–565 s) and
  `rl_bnd::p_lbnd_01` (693.6 s) stayed per change until spec 006's final run. With all six
  parts sharing the CPUs, both timed out at 12 minutes, so they moved weekly (`1da9428`).

## Final run (spec 005)

2026-10-14 (T045), at `49cbf07` with the runner changes of spec 006's T055 and these records
uncommitted, one run at a time on the workstation:

- **`make test`** passed (22:57–22:58, builds cached): the Rust suites, the C++ comparisons
  (`r1_interval_matches_cpp` and `rect_matches_cpp` among them), 913 Python tests, mypy,
  stubtest, the wasm suite and `cargo-deny`.
- **`make verify-values`, all six parts** (`VERIFY_JOBS=4`, 12 minutes each): 185 of 185
  harnesses successful in 31 min 21 s, the crate's tests first (`property_kinds` and
  `values_spec_vs_cpp` among them). All 61 of spec 005's per-change harnesses succeeded
  (`P=real` 25, `P=rect` 36), and all 58 crash harnesses of the six parts satisfied their
  covers. It is also the first full run with the module-name selection and the two members
  moved weekly, and the first under the new runner, which reported no `KNOWN-TIMEOUT` (no
  listed member runs per change).
- **The weekly members** were not re-run: their results stand from 2026-10-13/14
  (Measurements).
- **The deliberate break (T040, local).** With `contains_lat_lng`'s validity screen removed, `make
  verify-values P=rect` failed `rl_mem::p_lmem_01` in 0.9 s and
  `rl_crash::c_screened_point_operations` in 5.5 s, each with a concrete-playback test: a NaN
  longitude reaches `s1::Interval::contains`, whose debug check fires. The other 34
  succeeded. Reverted. The CI half needs a push (T039, T040).

## Success criteria (SC-001 … SC-013) (spec 005)

Checked one by one against the records above (T047, 2026-10-14). All 13 are met, SC-005 on
the workstation only.

| Criterion | Result | Evidence |
|-----------|--------|----------|
| SC-001 crash freedom, whole domain | met | All 26 in-scope operations of `r1::Interval` and all 34 of `s2::Rect` are proven crash-free over their whole documented domain, with no size bound (`INVENTORY.md` §3–4). The arithmetic operations' domains leave out exactly the infinite corners that FR-017 documents. |
| SC-002 crash proofs | met | 26 of 26 real-interval entries and 34 of 34 rectangle entries (target: at least 30). Each of the 20 exclusions (18 geometry, 2 `Display`) has a reason. |
| SC-003 at least 90% proven | met | Every FR-006 area has entries in both catalogues. 30 of 31 entries are proven (97%), all of their operations in scope. Not proven: P-LEXP-02's symmetry (a 60-minute timeout). |
| SC-004 non-vacuity | met | Every entry has at least one killed mutant (43 of 43 killed, `M-414` at the weekly limit). All 21 crash harnesses satisfy every cover. |
| SC-005 per change under 10 minutes | met on the workstation; CI not measured | With `VERIFY_JOBS=4`: `P=real` 2 min 20 s, `P=rect` 5 min 37 s. The CI measurement needs a push (T039). |
| SC-006 API unchanged, gate passes | met | The changes are FR-017 to FR-019's documentation and FR-020's deserialiser. `make test` passed before them (spec 005's commits) and in the final run. |
| SC-007 the maintainer's questions | met | "Recommendations (FR-016)" answers all four. |
| SC-008 panics documented | met | `project` (every build), `from_lat_lng`, `from_point_pair`, `interior_contains_lat_lng`, `expanded` and `from_center_size` (debug builds); see "`# Panics` and infinite-corner documentation added". |
| SC-009 deserialisation | met | `37fa0a2`: the three rejection tests fail without the fix and pass with it; the boundary values, the empty and full rectangles, the antimeridian and the seam corner deserialise unchanged, and 10,000 random rectangles stay valid through JSON text. |
| SC-010 composition | met | "Composition (SC-010)": 12 of 16 rectangle entries (75%) are stated through spec 004's `arc_spec`, 14 (88%) rely on spec 004. No proof time is saved; the description and its validation are. |
| SC-011 C++ vectors | met | `r1_interval_matches_cpp` and `rect_matches_cpp` pass on every row, with the documented deviations (U-009, U-010, the zero signs, spec 004's seam corner), in the final run's `make test`. |
| SC-012 index by kind | met | `property_kinds` passes; every `P-R` and `P-L` id appears once in `PROPERTIES_BY_KIND.md`. |
| SC-013 upstream findings | met | U-008, U-009 and U-010, with follow-ups T050–T052, are in "Upstream findings" and `UPSTREAM.md`. |

---

# Spec 006: `r2::Rect` and `s2::Point`

## Summary (spec 006)

**2026-10-14** (T049). Every in-scope operation of the u-v rectangle (32) is proven crash-free
over its whole documented domain; of the point's 25 with crash proofs, 23 are proven over their
whole domain and `from_coords` and `normalize` within bounds (coordinates up to `2⁵¹¹`); 5
operations reach `atan2` and are tool limits. Of the 32 catalogue entries, all 16 of the u-v
rectangle are proven (4 with weekly members) and 10 of the point's 16; the other 6 are not
proven: three because the checker's `sqrt` and `atan2` cannot express them (P-PTVAL-02's
normalising clause, P-PTORT-01, P-PTDST-02), three because members timed out at 60 minutes
(P-PTCLS-03, P-PTORT-03, P-PTDST-01's symmetry). Each not-proven entry is checked by the C++
vectors or property tests. The library gained six fixes and documentation changes (FR-017 to
FR-022), two upstream findings (U-012, U-013) and two documented deviations from C++ that follow
Go (K10, K12) or have no counterpart (the robust cross product's length). The mutant campaign
killed 39 of its 40 mutants; the 40th, `M-539`, gives no verdict, because its target,
P-PTCLS-03, does not finish.

## Composition (SC-014) (spec 006)

- **Share.** 14 of the 16 u-v rectangle entries (88%) are stated as products of `real_spec`
  statements and cite spec 005's proofs: P-QVAL-01 and -02 (P-RIV-01, P-RSET-*, P-REXP-01),
  P-QMEM-01 and -02 (P-RMEM-01, -02), P-QREL-01 and -02 (P-RREL-01, -02), P-QSET-01 to -03
  (P-RSET-01 to -03), P-QMEA-01 (P-RMEA-01), P-QEXP-01 to -03 (P-REXP-01 to -03, the first and
  last through `r1::Interval`'s own `expanded` and `approx_eq_with`), and P-QEQ-01 (P-RIV-02).
  The other two state bounds directly: the constructors (P-QVAL-03) and the vertices
  (P-QVAL-04).
- **Time.** The cited spec-005 proofs took 683 s (2026-10-13, per change, P-REXP-03's 565 s
  among them). The 14 composed u-v proofs took 4,347 s: 214 s per change and 4,133 s weekly
  (the approximate equality's symmetry and product, 3,098 s; the expansion's result, 916 s).
  Composition saves the description and its validation, not proof time: every u-v proof still
  executes `r1::Interval`'s code (no stubs), so a product statement costs at least its factors.
  Restating the intervals would have added a description and its 683 s of proofs without making
  the rectangle's proofs cheaper.

## Reach into the point's geometry (spec 006)

How far exhaustive checking reached into floating-point geometry, over the box `[−1, 1]³`
(which holds every unit point):

- **Proven over the box**: crash freedom of `point_cross` (224 s per change), `chord_angle`,
  `ortho` and `reference_dir` (260 s, weekly) and `rotate` (346 s, weekly; `sin` and `cos` are
  CBMC's models); `point_cross` and `ortho` never zero for a normalizable point (332 s and 62 s,
  weekly); the chord angle's range and its value at zero.
- **Timed out over the box at 60 minutes**: the three sign laws of `point_cross` off its `ortho`
  branch, the chord angle's symmetry bit for bit, and unit length after `normalize` and
  `ortho`. Each asserts on a product of symbolic floats compared with another (or on a square
  root's result), where SAT time grows without bound.
- **Proven exactly, off the box**: the order for every triple (NaN included), the
  classifications, the power-of-two scaling bit for bit (P-PTSCL-02 in 210 s, its upstream
  agreement where upstream is finite), and the arithmetic bit for bit.
- **Stopped at `atan2`** (unsupported): `distance`, `stable_angle` and the three approximate
  equalities, except for equal points, where `r3::Vector::angle` returns before it.
- **Stopped at `sqrt`** (modelled without fixing its result): every statement that compares
  two normalisations, so `ortho`'s oddness and exact value, `reference_dir == ortho`,
  `from_coords == normalize` and `normalize` against upstream's formula. Single results are
  proven (non-zero, unit length weekly).
- **Needed `same` instead of bits**: the orthogonal directions' laws, whose zeros' signs differ
  (research R0, finding 2).
- **Needs certified error bounds instead**: any statement about a value after `atan2`, `sqrt`
  or `sin` and `cos` (angles, unit length to a tolerance, rotation) is out of reach of
  exhaustive bit-level checking at this cost; such properties need an error analysis (as
  upstream's `IsUnitLength` comment does) or a deductive tool with real-number semantics, not
  more SAT time.

## Effort log (spec 006)

- **2026-10-13**: specification, planning, design documents and the probe's harnesses
  (`8016b3a` and earlier).
- **2026-10-14, 00:12–00:28**: the planning probe (R0), 29 harnesses.
- **2026-10-14, 07:00–11:00**: the C++ vectors and descriptions (Phases 1–2), the six library
  changes with their probes and tests (Phase 3), the harnesses, three rounds of splitting after
  the first runs, the `sqrt` and band experiments.
- **2026-10-14, 11:00–14:20**: the weekly run.
- **2026-10-14, 14:20–17:14**: the mutant campaign (T043).
- **2026-10-14, 17:14–19:14**: the bounded attempts (T036): four near-unit statements, all
  timed out.
- **2026-10-14, from 19:15**: the deliberate break (T047), the final run (T052) and the
  closing checks.

## Exclusions and tool limits (FR-002, FR-003) (spec 006)

- **Both types' `Display`**: formatting, as 001's and 004's.
- **The five operations through `atan2`** (`distance`, `stable_angle`, `approx_eq`,
  `approx_eq_angle`, `approx_eq_with`): Kani replaces the foreign C call with a failing check
  (R0, 14.9 s). The C++ vectors check `distance` bit for bit on every row; a property test
  checks the symmetry of `distance` and `stable_angle` bit for bit (P-PTDST-02).
- **`from_coords` and `normalize` for triples whose squared length overflows**: CBMC reports
  `1 / ∞` as NaN (R0, finding 1). `point_tests.rs` and the C++ rows check their zero result.
- **Comparisons of two normalisations**: the checker's `sqrt` does not fix its result (finding
  4). P-PTORT-01 and P-PTVAL-02's normalising clause are not proven; P-PTORT-03's `ortho` branch
  likewise. The C++ vectors check `normalize` and `ortho` bit for bit, and `point_tests.rs` pins
  the branch (K10, K11).

## Floating-point cost (spec 006)

- **Cheap** (seconds): comparisons and selections of bounds, the order, the classifications,
  the arithmetic's negation, the product statements of the u-v rectangle per axis.
- **Minutes**: sums and differences checked bit for bit (subtraction 328 s), the exact scaling
  (210 s), one cross product (`point_cross`, 224 s), one normalisation of a bounded triple
  (61 s), `rotate`'s crash proof with modelled `sin` and `cos` (346 s).
- **Over an hour**: comparing two cross products or two squared lengths as values (P-PTORT-03,
  the chord angle's symmetry), unit length after a normalisation (P-PTCLS-03), the squared
  length's band near overflow.
- **Out of reach**: anything after `atan2`, and comparing two results of `sqrt`.

## Upstream findings (FR-024) (spec 006)

- **U-012** (K6): C++ `S2::RobustCrossProd(−b, b)` is `(NaN, ∞, NaN)` for the unit point
  `b = (2⁻¹⁰⁷⁴, 0, 1)`, and for every `b = (2⁻ᵏ, 0, 1)` with k ≥ 1024: `EnsureNormalizable`'s
  factor `ldexp(2, −1 − ilogb(p_max))` overflows below `2⁻¹⁰²³` (`s2edge_crossings.cc:294`).
  Observed on the `S2POINT_SPECIAL` rows. The library's private copy did the same until FR-021;
  it now shares `Point::ensure_normalizable` (FR-020) and returns `(−0, 1, 0)`. Follow-up T057.
- **U-013** (K11): Go's `PointCross` documents its sign laws "unless p == op or p == −op", but
  they fail for every pair of distinct parallel points, and for points so small that the
  products underflow: `(1, 0, 0)` and `(1 + 2ε, 0, 0)`, both `IsUnit`, give the same result
  both ways (observed in Go at `fe8a584`). The library keeps Go's behaviour and documents the
  true exception; P-PTORT-03 states it, but its members time out, so `point_tests.rs` pins it
  (K10, K11). Follow-up T058; the alternative, C++'s symbolic
  perturbation, is T059.
- Both are recorded in `UPSTREAM.md`, candidates, not reported.

## `# Panics` and limit documentation added (SC-008) (spec 006)

- **`r2::Rect`** (FR-017, FR-018): `# Panics` on `new` and `from_points` (debug builds) and
  `project` (every build, U-008); `from_center_size`'s precondition, now debug-checked as
  upstream's constructor; the infinite and NaN corners of `center`, `size`, `expanded`,
  `expanded_by_margin`, `approx_eq_with` and `from_center_size`; a NaN bound passes
  `is_valid` but contains nothing, and deserialisation rejects it (FR-019).
- **`Point`** (FR-020 to FR-022): `ensure_normalizable`'s domain and postcondition;
  `from_coords` and `normalize` off the sphere (K7) and their zero signs (K12); the order with
  NaN (K8); `is_normalizable` with NaN; `ortho`, `reference_dir`, `chord_angle` and `rotate`'s
  domains; `point_cross`'s true laws (K10, K11); `robust_cross_prod`'s "Deviation from
  upstream" (U-012, and the length past its double-precision step).
- Each documented behaviour was probed in debug and release builds before its commit; the
  probes' output is in the commit messages (`7072e65`, `0ce304a`, `d22cb11`, `073d7b0`,
  `36a4a9c`, `a65c350`).

## Defects found (spec 006)

- **K5** (the survey's): `Point::ensure_normalizable` returned infinities, NaN or zeros for a
  largest coordinate below `2⁻¹⁰²³`. Fixed by FR-020 (`073d7b0`); P-PTSCL-01 and -02 prove the
  fix, and `M-525` (the fix removed) must fail them. No `D-NNN`, as for 004's survey defects.
- **K6**: the robust cross product's private scaling went to [0.5, 1), half of upstream's, and
  overflowed one binade later than C++. Fixed by FR-021 (`36a4a9c`).
- No harness found a defect outside the survey: no `D-NNN` for spec 006.

## Measurements (spec 006)

On the workstation (8 threads, 30 GB), 2026-10-14, one Kani run at a time:

| Run | Harnesses | Result | Wall time |
|-----|-----------|--------|-----------|
| `make verify-values P=uvrect` (4 jobs, 12 min each) | 31 (10 crash, 21 property members) | 31 successful | 2 min 18 s |
| `make verify-values P=point` (4 jobs) | 21 (9 crash, 12 property members) | 21 successful | 10 min 50 s; 6 min without the 516 s member, since moved weekly |
| weekly members (3 jobs, 60 min each, 8 GB) | 18 | 10 successful (longest 1,751 s), 8 timed out | 3 h 23 min |
| mutant campaign `M-501` … `M-540` (weekly-tier mutants 60 min) | 40 mutants | 39 killed, 0 survived, 1 without a verdict (`M-539`: its target timed out) | 2 h 53 min, of which 1 h 0 min was `M-540`'s counterexample re-run |
| bounded attempts (T036; 3 jobs, 60 min each, 8 GB) | 4 | 4 timed out | 2 h 00 min |

The longest per-change members: `pt_ari::p_ptari_01__sub` 328 s, `pt_crash::c_point_cross`
224 s, `pt_scl::p_ptscl_02` 210 s; on the u-v side `ur_mea::p_qmea_01__size` 84 s. The runner
records no peak memory; every harness stayed under its ceiling (4 GiB per change, 8 GiB
weekly). Per-harness times are in the catalogue's Bound column and the inventory.

`M-540` (`point_cross` drops its x sign) was killed by `pt_ort::p_ptort_03__swap`, whose
unmutated run times out, in 145 s (324 s in a re-run). The runner's concrete-playback re-run of
the failure then took the full 60 minutes and produced no test; `run-mutants.sh` now skips that
re-run (2026-10-14), since a kill needs only the FAILED status. (An earlier version of this
section gave the 60 minutes as the kill's time.) `M-539` (`normalize` divides by the squared
length) targets P-PTCLS-03, which does not finish, so it has no verdict, as spec 004's `M-344`
and `M-345` have none for P-IEXP-03; the runner now reports these as `no-verdict`, not
`error`. The quickcheck property `prop_from_coords_unit` fails on `M-539` at once, on
`(0, 0, 2)` (checked 2026-10-14).

## `ChordAngle` (FR-025) (spec 006)

*Resolved 2026-10-16: the maintainer answered (B); spec 007 took the type, with `Angle`
(its part below).*

**Go, once the validity question is answered.** The generic order laws (`laws::ord`) are
ready for it, and the pilot proved its bit-level core in 0.1–1.5 s per harness. What spec 006
learned applies: its arithmetic (`sin`, `cos`, `tan`) meets the same modelled functions, and its
constructors from points meet `sqrt` through `from_angle`.

The decision it waits on, from the spec's "Deferred: `ChordAngle`" section: which values are
valid.

- (A, recommended) keep upstream's rule (every negative value is valid), prove on the
  canonical values, and pin the others with tests;
- (B) map every negative to `NEGATIVE` in the constructors, a documented deviation that changes
  003's `Cap` round trip;
- (C) make only `NEGATIVE` valid among the negatives, so the `Cap` decoder rejects bytes C++
  accepts.

The survey's other findings wait with it: `from_length2(NaN)` returns NaN where upstream
returns the straight angle (a silent deviation), the special values in the arithmetic, and
`e5`/`e6`/`e7` of `INFINITY` (a candidate upstream finding).

## Comparison with 001–005 (spec 006)

- **001–003** proved integer and byte-level code: crash freedom and exact contracts, with size
  bounds for collections. **004 and 005** proved the first value types with floating-point
  bounds, whose operations compare and select; their arithmetic stopped at `∞ − ∞`.
- **006** is the first to reach floating-point geometry. The u-v rectangle behaved like 005's
  types (everything proven, the costly members weekly). The point split three ways: its integer
  and bit-level side (order, scaling, arithmetic) proved as easily as 001's; its geometry proved
  crash freedom and single-result facts over the box; and its laws comparing two geometric
  results, unit length and angles did not finish or could not be expressed (`sqrt`, `atan2`).
- **Tool findings**, each now in memory and `TRUSTED_BASE.md`: 004's bool ordering bug, 006's
  spurious NaN for `x / ∞` and nondeterministic `sqrt`.

## Limitations (spec 006)

- The proofs cover the default build; with hardware FMA the dot and cross products fuse, and
  `distance` is not symmetric (K9). Nothing in the suite checks an FMA build. Since 2026-10-26
  the verify crate refuses to build with the `fma` target feature (`compile_error!` in
  `src/lib.rs`; code review of v0.4.0..74b9bc1, finding 8), so its unfused restatements are
  never paired with a fused library.
- The point's geometry is proven over the box, a superset of the unit sphere, so its
  statements are stronger than needed where they hold and do not distinguish unit inputs.
- `sin` and `cos` are over-approximated, `sqrt` is under-determined and `atan2` is absent:
  every proven statement after them holds for every value the model allows, which is assumed to
  include the true one (`TRUSTED_BASE.md`).
- `r2::Rect` and `Point` have public fields: the type cannot keep its invariant, and FR-019
  closes deserialisation only.

## Recommendations (FR-016) (spec 006)

Ranked:

1. **Answer `ChordAngle`'s validity question and take it next** (FR-025): the order laws are
   ready, and it is the distance type of the query layer.
2. **Report U-012 and U-013 upstream** (T057, T058), and U-007 to U-011 from specs 004 and 005;
   each needs the maintainer's approval.
3. **Decide K11's alternative** (T059): keep Go's `point_cross` with its true exception
   documented, or perturb parallel points symbolically as C++'s `RobustCrossProd` does (exact
   arithmetic, a behaviour change).
4. **Check an FMA build** in CI (K9): the property tests and the C++ comparison in a build with
   `target-feature=+fma`, so the documented asymmetry stays the only difference.
5. **Certified error bounds for the geometry**: the next properties of the point (unit length
   after `rotate`, the angle's accuracy) need an error analysis or a real-number tool, not
   more SAT time.
6. **Type-level invariants**: private fields with checked constructors for `r2::Rect` (and
   `s2::Rect`) would make the validity rule a type property; a breaking API change, for the
   maintainer.

## Notes from implementation (spec 006)

- **K12, a zero's sign in `normalize`.** C++'s `Normalize` multiplies by zero where the norm
  is zero, so each zero keeps its coordinate's sign; the library returns `(+0, +0, +0)`, as
  Go. Found writing `point_spec::upstream_normalize`; documented (FR-022), and P-PTVAL-02 and
  the C++ comparison state it.
- **The robust cross product's length.** C++ answers nearly parallel points from a `long
  double` step with twice the cross product; the library has no such step and its exact step
  normalises. On the C++ vectors' 48 such rows the directions agree within 2·10⁻¹⁶ rad. Found
  by the new vectors; documented in `robust_cross_prod`'s deviation section and `UPSTREAM.md`.
- **Upstream's dot product starts its sum at `+0`.** Where every product is `−0`, upstream's
  `DotProd` is `+0` and the library's `−0`. Only `atan2(0, ±0)` sees the difference: the angle
  between `(2⁻¹⁰⁷⁴, 0, 0)` and its antipode is 0 in C++ and π in the library. No unit point
  reaches it; `cpp_values_diff.rs` exempts the angle where both its arguments vanish.
- **`add_rect` and `union` of two empty rectangles** return different representations of the
  empty set, in C++ as in the library (`Union` returns the other operand, `AddRect` keeps its
  own). Kani found it against P-QSET-01's first statement, "bit for bit"; the statement now
  says "unless both are empty".
- **CBMC and the squared length.** Over all of `NormableTriple`, `from_coords` and
  `normalize` time out at 12 minutes, in every formulation tried; over the coordinates up to
  `2⁵¹¹` they finish in seconds. The band `(2⁵¹¹, 2⁵¹²)` is a weekly member, and the C++
  vectors' rows at `1e154` test it.
- **CBMC reports `x / ±∞` as NaN** (research R0): its NaN check treats a division by an
  infinity as producing NaN, which IEEE-754 does not. It set the `NormableTriple` domain.

## Final run (spec 006)

2026-10-14 (T052), at `bf866bf` with this section's records uncommitted, one run at a time on
the workstation:

- **`make test`** passed (19:22–19:24, builds cached): the Rust suites, the seven C++
  comparisons (`r2_rect_matches_cpp` and `s2_point_matches_cpp` among them), 913 Python tests,
  mypy, stubtest, the wasm suite and `cargo-deny`.
- **`make verify-values`, all six parts** (`VERIFY_JOBS=4`, 12 minutes each): 185 of 187
  harnesses successful in 29 min 49 s, the crate's tests first (`property_kinds` among them).
  All 51 of spec 006's per-change harnesses succeeded, and all 58 crash harnesses satisfied
  their covers.
- **The two timeouts were spec 005's.** `ri_exp::p_rexp_03_symmetric` (474–565 s before) and
  `rl_bnd::p_lbnd_01` (693.6 s before; it had timed out once already) were per-change members
  over the 6-minute rule, and with all six parts sharing the CPUs they passed 12 minutes. Both
  moved weekly (`M-416`, `M-431` and `M-432` now carry `Tier: slow`). With the weekly settings
  (60 minutes, 8 GiB, 2 jobs) both prove, in 564.5 s and 673.1 s; the parts re-run per change
  without them: `P=real` 25 of 25 in 2 min 20 s, `P=rect` 36 of 36 in 5 min 37 s.
- **The parts selected each other's harnesses.** Kani matches `--harness` anywhere in a name:
  `P=real`'s `ri_` also ran `pt_ari::p_ptari_01__*` (the 328 s subtraction among them),
  `P=cell`'s `cl_` ran `pt_scl::*`, and `P=interval`'s `iv_` had run `ri_val::*` since spec 005.
  The Makefile now passes each part's modules by name (`--harness ri_exp::`). `P=uvrect` and
  `P=point` were not affected, so their measurements stand.
- **The deliberate break (T047, local).** With `cmp_point` skipping y, `make verify-values
  P=point` failed `pt_ord::p_ptord_01` in 1.7 s, with a concrete-playback test (a NaN x
  coordinate, so y decides), and `p_ptord_02` in 3.2 s ("the order's ties are not equality");
  the other 18 succeeded. Reverted. The CI half needs a push (T046, T047).

## Success criteria (SC-001 … SC-014) (spec 006)

Checked one by one against the records above (T054, 2026-10-14). 13 of 14 are met, SC-005 on
the workstation only, and SC-003 as amended; SC-001 is met except for two operations.

| Criterion | Result | Evidence |
|-----------|--------|----------|
| SC-001 crash freedom, whole domain | met except `from_coords` and `normalize` | All 32 in-scope operations of `r2::Rect` and 23 of the point's 25 with proofs are proven crash-free over their whole documented domain (`INVENTORY.md` §5–6). `from_coords` and `normalize` are proven for coordinates up to `2⁵¹¹`; the band `(2⁵¹¹, 2⁵¹²)` timed out at 60 minutes, and the finite triples whose squared length overflows are a tool limit (`x / ∞`). The five operations through `atan2` are tool limits, as SC-002 allows. |
| SC-002 every entry has a crash proof | met | 32 of 32 `r2::Rect` entries and 24 of 24 point entries (`from_coords` and `normalize` within bounds); of the 6 trigonometric ones, `rotate` has a weekly proof (346 s) and 5 are tool limits with R0's measurement (a failing check in 14.9 s); both `Display` exclusions have reasons. |
| SC-003 at least 90% proven | met, as amended | Every FR-006 area has entries in both catalogues. 26 of the 28 counted entries are proven (93%). The criterion as first written missed: 26 of the 30 entries whose operations are all in scope (87%), 26 of all 32 (81%). The maintainer amended it on 2026-10-14 to leave out the entries that compare two normalisations, which the checker's `sqrt` cannot state: P-PTVAL-02 and P-PTORT-01. Not proven among the 28: P-PTCLS-03 and P-PTORT-03 (60-minute timeouts, also on the box points whose largest coordinate is at least ½). P-PTDST-01 and -02 involve the `atan2` operations. The `r2::Rect` catalogue alone: 16 of 16. |
| SC-004 non-vacuity | met | Every proven entry has at least one killed mutant (39 of `M-501` … `M-540` killed; `M-539`'s target is not proven). Every crash harness that finished has all its covers satisfied: the final run's 58 (18 of them spec 006's) and the weekly `c_ortho` and `c_rotate`. |
| SC-005 per change under 10 minutes | met on the workstation; CI not measured | With `VERIFY_JOBS=4`: `P=uvrect` 2 min 18 s, `P=point` about 6 minutes (T046's local half). The CI measurement needs a push (T046). |
| SC-006 API unchanged, gate passes | met | The public changes are FR-017 to FR-022's (documentation, `from_center_size`'s debug check, the deserialiser, the scaling); `make test` passed after Phase 3 and in the final run. |
| SC-007 the maintainer's questions | met | "`ChordAngle` (FR-025)" (go, after the A/B/C validity question), "Limitations" and "Recommendations" (type-level invariants sixth; what next, ranked). |
| SC-008 panics documented | met | "`# Panics` and limit documentation added (SC-008)": `new`, `from_points`, `project`, and `from_center_size`'s debug check. |
| SC-009 deserialisation | met | FR-019's two rejection tests failed before `d22cb11` and pass after (its commit message); 10,000 rectangles round-trip bit for bit. |
| SC-010 scaling and robust cross product | met | P-PTSCL-01 proven over `FiniteTriple` (18.4 s); K6's antipodal rows match C++ with `deviation:robust-cross-overflow`. |
| SC-011 C++ vectors | met | `r2_rect_matches_cpp` and `s2_point_matches_cpp` pass on every row, with the documented deviations (K10, K12, the robust cross product's length, the dot product's zero sign). |
| SC-012 index by kind | met | `property_kinds` passes in the final run; every `P-Q` and `P-PT` id appears once in `PROPERTIES_BY_KIND.md`. |
| SC-013 upstream findings | met | U-012 (T057) and U-013 (T058, with T059) in "Upstream findings" and `UPSTREAM.md`. |
| SC-014 composition | met | "Composition (SC-014)": 14 of 16 `r2::Rect` entries (88%) rest on spec 005's proofs, 683 s of reused proofs against 4,347 s for the composed ones. |

## After the analysis (T055) (spec 006)

The analysis after implementation found four HIGH items; the maintainer decided them on
2026-10-14:

- **No weekly CI run of the value types.** Specs 004–006's 65 weekly members ran only on the
  workstation. The weekly `slow` job now has eight `values-*` legs, at 60 minutes and 8 GiB
  per harness, one harness at a time. Four legs hold the members that prove. The four
  `values-limits-*` legs hold the 16 that time out, four to a leg, so each leg fits a hosted
  job's 6 hours. `verify/known-timeouts.txt` lists those 16. `run-verify.sh` reports them as
  `KNOWN-TIMEOUT`, which does not fail the run. It fails the run if one fails, runs out of
  memory or starts to prove.
- **The weekly mutant job could not finish.** Its 270 minutes were sized for specs 001–003,
  and the 226 mutants take about 9 hours on the workstation. A hosted job may run at most 6
  hours, so the job now runs one leg per spec (`make verify-mutants M='M-<N>*'`), 360 minutes
  each, with `VERIFY_SLOW_HARNESS_TIMEOUT=60m`. `run-mutants.sh` gives a mutant whose target
  times out unmutated too (`M-344`, `M-345`, `M-539`) the outcome `no-verdict`, which does
  not fail the campaign. The three were reclassified from `error` by runs at a 1-minute
  limit (their 60-minute runs had timed out); `M-540` was re-run with the new runner and
  killed in 324 s. The runner also no longer re-runs a failed harness for a counterexample.
  That re-run had taken `M-540` a full hour after its kill in 145 s.
- **SC-003** was amended to leave out the two entries the checker's `sqrt` cannot state; it
  is met at 26 of 28.
- **Spec 005's records** were completed (its part above), and `VERIFIED_TYPES.md` gained
  sections for specs 004 and 005.

# Spec 007: `s1::ChordAngle` and `s1::Angle`

## Summary (spec 007)

Spec 007 verified the chord angle, the query layer's distance type, and the angle it converts to
and from, after the maintainer answered the chord angle's validity question with option B
(2026-10-16): every negative value becomes `NEGATIVE`, so the type holds only −1, the lengths in
[0, 4] and +∞.

- **Crash freedom:** every in-scope operation, `ChordAngle` 28 entries and `Angle` 29, over its
  whole documented domain, per change (39.8 s and 2.9 s of harness time). 8 entries are tool
  limits: `asin` (`to_angle` and the 6 operations that read it) and `Angle::tan`.
- **Meaning:** 13 of `ChordAngle`'s 14 catalogue entries proven per change: the canonical-value
  invariant for every constructor and every operation, the classifications, the total order,
  the neighbours, `plus_error`, the error bounds and the arithmetic. Of `Angle`'s 8, the
  representation, the order and upstream's degree–radian identities are proven; the E-form
  round trips are `proven-by-enumeration` over every `i32`; `normalized`'s value is a tool
  limit (Kani's `%`). The members that compare two float products or quotients bit for bit
  time out at 12 minutes and are weekly: 6 of the 10 prove in 5 to 16 minutes (the degree
  conversions, the E-forms over every angle, the scalar products), so P-AGCNV-01 and P-AGEF-01
  are proven; `ChordAngle`'s `sin2` and `sin` and the angle's two quotients time out at 60
  minutes, so P-CAARI-03 and P-AGARI-01 are tool limits (the C++ vectors check them).
- **Library changes:** option B (FR-017), NaN parity (FR-018), the special operands' debug
  checks and documentation (FR-019), a validating `Deserialize` (FR-020), an exact `normalized`
  (FR-021; it returned 4.0 for −2.462748394469102·10¹⁶), E-form ties to even (FR-022), the
  E-forms' documented saturation (FR-023), and D-008 (`max_angle_error`, found by Kani).
- **Findings:** U-014 (C++'s E-forms of `Infinity()` and of out-of-range angles), and Kani's
  wrong `%` on `f64` (research R0, Finding 1), which affects any proof that uses it.

## The canonical-value invariant (option B) (spec 007)

- **What it made provable.** With one value per meaning, the order is a total order on every
  value the type can hold (P-CAORD-01), `successor` and `predecessor` are the adjacent values and
  undo each other (P-CANBR-01 to -03), and every way to build a chord angle lands in the set
  (P-CAVAL-02). Before, `predecessor(−5)` returned −1, a larger value; the laws held only on
  the canonical values, and a proof would have had to assume them.
- **What it cost.** Four lines in `from_length2` and `is_infinity`, a NaN check moved into the
  `Cap` decoder (so 003's D-004 rejection stays), a hand-written `Deserialize`, and one
  documented deviation: a `Cap` that C++ encoded with a negative radius other than −1 decodes
  as −1, so its bytes do not round-trip. The C++ coding vectors had one such row (radius
  −0.5), now noted `deviation:negative-canonical`; 003's model `lib_cap` and P-FX-08 name it,
  and 003's codecs part re-ran clean (142/142, 2026-10-18).
- **Type-level invariants.** `ChordAngle`'s field was already private, so option B made the
  validity rule a property of the type at no API cost. The other value types have public
  fields (`r1::Interval`, the rectangles, `Point`) or no validity rule (`Angle`); for them the
  same step is a breaking change. Recommendation: keep the proofs' precondition approach for
  them, and take type-level invariants only together with an API revision (see
  Recommendations).

## Reach into the trigonometric conversions (spec 007)

- **`sin`** is modelled (over-approximated in [−1, 1]): `from_angle`'s range and special values
  are proven, not its exact value (a NaN angle's 4 rests on the C++ vectors).
- **`sqrt`** is modelled without fixing its result: `Add`, `Sub` and `sin` get one-result
  properties; the range of `sin` (through `sin2`'s product) times out at 60 minutes.
- **`asin` and `tan`** are unsupported C library calls: 8 tool-limit entries.
- **`%` on `f64`** is modelled wrongly: `1 % 2` is 0 under Kani. `normalized` has a crash proof
  that does not depend on the value, and its value rests on the C++ vectors (bit for bit, no
  tolerance) and an exact big-integer property test.
- **A second method: enumeration.** Where the domain is one `i32` or `u32`, a native test that
  checks every value is a proof of the compiled code: the E-form round trips, the constructors
  and upstream's integer identities take 19.5 s, where Kani timed out at 12 minutes even over
  2²⁴ inputs.

## Effort log (spec 007)

- 2026-10-16: survey (C++ and Rust probes), spec, the maintainer's option B and ties to even,
  plan with the R0 probe (29 harnesses; Kani's `%` found), tasks.
- 2026-10-16/17: descriptions, C++ vector sections, library changes (FR-017 to FR-023) and
  their tests, the harnesses, the enumeration.
- 2026-10-17/18: per-change runs (chord 28, angle 15 harnesses, then splits), D-008 found and
  fixed, records, six commits (`6fcb06e` … `58fc91f`).

## Exclusions and tool limits (FR-002, FR-003) (spec 007)

| Entry | Reason | Evidence instead |
|-------|--------|------------------|
| `ChordAngle::to_angle`, `radians`, `degrees`, `e5`, `e6`, `e7`, `From<ChordAngle> for Angle` | `f64::asin` is a foreign C call (R0: 0.8 s, a failing check) | `CHORD_ANGLE` results 15–18, bit for bit |
| `Angle::tan` | `f64::tan` is a foreign C call | `ANGLE` result 10, bit for bit |
| both `Display` | `core::fmt` | — |
| `Angle::normalized`'s value | Kani's `%` on `f64` is wrong | `ANGLE` result 5 (no tolerance); `prop_normalized_is_the_exact_remainder` |
| the angle's quotients by an infinity | CBMC reports `x / ±∞` as NaN | `ANGLE` rows with divisor +∞ |

## Upstream findings (FR-025) (spec 007)

- **U-014** (`UPSTREAM.md`): `S1ChordAngle::e5`/`e6`/`e7` of `Infinity()`, a valid value, and
  `S1Angle`'s E-forms out of the `int32` range return −2147483648 in release builds and fail a
  debug check; off x86 the cast is undefined behaviour, and the tie rounding depends on the
  processor. Follow-up T061; not reported (needs the maintainer's approval).

## `# Panics` and domain documentation added (SC-009) (spec 007)

- `ChordAngle::sin`, `sin2`, `cos`, `tan`, `Add`, `Sub`: `# Panics` in debug builds for a
  special operand, with the release results (probed: `NEGATIVE.sin2()` is −1.25, `INFINITY.cos()`
  is −∞, `NEGATIVE + RIGHT` is `STRAIGHT`).
- `ChordAngle`: the canonical values and option B's deviation; `from_length2`'s four cases.
- `Angle::e5`–`e7`: the domain, ties to even, the saturation; the unsigned constructors'
  wrapping; `normalized`'s domain.
- The `Cap` decoder: option B's deviation, and why NaN is checked first.

## Defects found (spec 007)

- **K7** (survey, fixed by FR-021): `normalized` was inexact and could leave (−π, π]; R0's
  Kani run found −2.462748394469102·10¹⁶ ↦ 4.0.
- **K3** (survey, fixed by FR-018): NaN gave NaN, an invalid chord angle.
- **D-008** (found by Kani, `p_caerr_02__angle`, 2026-10-17; fixed 2026-10-18 at the
  maintainer's choice): `max_angle_error` was `ε · length²`, older upstream's; v0.14's is
  `1.5 · ε · length²`. `Cap` adds it to its error margins (`cap.rs:320`, `:1070`), which were
  too small by `0.5 · ε · length²`.
- **US4's break check** (T052, 2026-10-19): with `successor` stepping `STRAIGHT` by one ulp
  (`>=` made `>` in `chord_angle.rs:283`), `make verify-values P=chord` failed in 100 s wall
  time with 4 of 28 harnesses: `ca_nbr::p_canbr_01` (concrete input `length2` = 4.0),
  `p_canbr_03`, `ca_val::p_caval_02__nbr` and `ca_crash::c_neighbours` (an unsatisfied
  cover). Reverted. The CI half (the leg fails; an unrelated change does not start it) needs a
  push and has not been checked.

## Measurements (spec 007)

Per change, `make verify-values P=chord` and `P=angle` with 5 jobs on the 8-thread workstation
(2026-10-17/18; summed from the runs, the splits re-run alone):

| Part | Harnesses | Harness time | Longest |
|------|-----------|--------------|---------|
| `chord` | 28 (9 crash, 19 property members) | 159.5 s | `p_caval_02__ari` 27.9 s |
| `angle` | 14 (9 crash, 5 property members) and the enumeration (7 tests, 19.5 s) | 66.5 s | `p_agari_01__sub` 31.1 s |

Weekly, one at a time at 60 minutes and 8 GiB (2026-10-18):

| Member | Result | Time |
|--------|--------|------|
| `ag_cnv::p_agcnv_01__from_degrees` | SUCCESSFUL | 944.2 s |
| `ag_cnv::p_agcnv_01__degrees` | SUCCESSFUL | 872.2 s |
| `ag_eform::p_agef_01__e5` | SUCCESSFUL | 523.9 s |
| `ag_eform::p_agef_01__e6` | SUCCESSFUL | 616.6 s |
| `ag_eform::p_agef_01__e7` | SUCCESSFUL | 373.4 s |
| `ag_ari::p_agari_01__mul` | SUCCESSFUL | 303.9 s |
| `ca_ari::p_caari_03__sin2`, `__sin` | TIMEOUT | 60 min |
| `ag_ari::p_agari_01__div`, `__ratio` | TIMEOUT | 60 min |

The four timeouts are in `verify/known-timeouts.txt`; CI runs the six in the `slow` job's
`values-chord-angle` leg and the four in `values-limits-e`.

003's weekly `Cap` agreement members (`cd_fx::p_fx_08__b32`, `__b33`), re-run because option B
reaches them, failed (673.0 s, 813.6 s) with a NaN radius: `CapC`'s reference decoder built the
cap through `from_length2` and so inherited D-004's rejection from `from_length2(NaN)` being
NaN; since FR-018 that gives `STRAIGHT`, upstream's full cap, which the library still rejects.
The reference decoder now states D-004 itself, as `wire_spec::lib_cap` does, and both members
are proven (711.9 s, 743.0 s, 2026-10-19). A harness gap, not a library defect.

## Non-vacuity (spec 007)

`make verify-mutants M='M-6*'` with `VERIFY_SLOW_HARNESS_TIMEOUT=60m` (2026-10-19): all 39 of
`M-601` … `M-648` killed (`verify/MUTANTS.md`).

- **By Kani:** 33, the 7 of the slow tier included. `M-615` (`sin2`) and `M-643` (the ratio)
  are killed by members that time out when the property holds: a counterexample is found long
  before a proof would finish.
- **By ordinary tests** (`Test:` headers, run in the mutant's worktree): the enumeration kills
  `M-636`, `M-637`, `M-644` and `M-648`; `normalized`'s tests kill `M-646` and `M-647`.
- **One survivor, fixed in the records.** `M-644` (`from_e7`'s scale two ulps off, first
  described as one) was aimed at the E7 round trip and survived it: the rounding in `e7`
  absorbs the error, so the round trip is right to hold. It is now aimed at
  `p_agcnv_01_e_forms`, which compares the product bit for bit and kills it; `M-648` (`e7`
  truncates) was added so P-AGEF-02 keeps a mutant of its own.

## `LatLng` and `Cap` (spec 007)

- **`LatLng`: go, in part.** It is two `Angle`s with a validity range, so `angle_spec` and this
  suite carry over to its construction, validity, normalisation (`normalized` clamps and
  reduces, the latter through `%`, so the same tool limit) and E-form conversions. Its
  conversions to and from `Point` go through `atan2`, unsupported: tool limits, as spec 006's
  distance.
- **`Cap`: go, after `LatLng`.** It is a `Point` and a `ChordAngle`, both now verified, and
  003 proved its codec. Containment is a chord-angle comparison (cheap, now that the order is
  total); union and the bounds assert on products of coordinates (slow, as spec 006's
  geometry).

## Comparison with 004–006 (spec 007)

The value types of 004–006 took validity as a precondition; `ChordAngle` is the first whose
validity the type enforces, and its laws are the cheapest of the value types (all per change, at
most 28 s). `Angle` is the first type where the checker's arithmetic model, not its capacity,
set the limits: `%` is wrong, `tan` and `asin` are missing, and comparing two separately
computed products bit for bit times out. Native enumeration covered the integer domains.

## Limitations (spec 007)

- `normalized`'s value and the `asin`/`tan` entries have no Kani proof; their evidence is the
  C++ vectors and tests.
- `ChordAngle`'s `sin2` and `sin` and the angle's two quotients time out at 60 minutes; their
  evidence is the C++ vectors (bit for bit on every row).
- The enumeration proves the compiled code on one x86-64 machine; it is not a model-level proof.

## Recommendations (FR-016) (spec 007)

Ranked:

1. **Report U-014 upstream** (T061), with the maintainer's approval. Kani's `%` mis-model is
   recorded as KI-002 in `KANI_ISSUES.md` (T062, 2026-10-25), not filed with Kani.
2. **`LatLng` next**, reusing `angle_spec` and this suite; then `Cap`.
3. **Use enumeration** wherever a property's domain is one 32-bit integer, rather than slicing
   it into weekly Kani members.
4. **Type-level invariants** for the other value types only with an API revision; option B shows
   the gain when the field is already private.

## Final run (spec 007)

2026-10-19 (T057), at `58fc91f` with this part's records, the `CapC` fix and `M-644`/`M-648`
uncommitted, one run at a time on the workstation:

- **`make test`** passed (01:27–01:29, builds cached): the Rust suites with the new
  `chord_angle_tests`, `angle_tests` and the extended `cpp_values_diff`, 914 Python tests,
  mypy, stubtest, the wasm suite and `cargo-deny`.
- **`make verify-values`, all eight parts** (`VERIFY_JOBS=4`, 12 minutes each): 227 of 227
  harnesses successful in 24 min 40 s, the crate's tests first (`property_kinds`,
  `values_spec_vs_cpp` and `wire_spec_vs_cpp` among them), then the enumeration (7 tests,
  23.6 s). All 42 of spec 007's per-change harnesses succeeded, the longest `ca_err::p_caerr_01`
  at 30.5 s, and all 76 crash harnesses of the suite satisfied their covers.
- **The weekly members and 003's `Cap` members** ran before, alone (see "Measurements").

## Success criteria (SC-001 … SC-013) (spec 007)

Checked one by one against the records above (T059, 2026-10-19). All 13 are met, SC-005 on the
workstation only.

| Criterion | Result | Evidence |
|-----------|--------|----------|
| SC-001 crash freedom, whole domain | met | `ChordAngle`'s 28 and `Angle`'s 29 entries with crash proofs are proven over their whole documented domain, with no size bound (`INVENTORY.md` §7–8). |
| SC-002 every in-scope entry | met | `Angle`: 29 of 30 proven, `tan` a tool limit; `ChordAngle`: 28 of 35, the 7 through `asin` tool limits, each with its 0.8 s failing check (research R0). Both `Display`s are excluded with a reason ("Exclusions and tool limits"). |
| SC-003 at least 90% of the counted entries | met (90.5%) | 22 entries; P-AGNRM-01 (`normalized`'s value) is not counted (FR-003). Of the 21 counted, 19 are proven (one by enumeration); P-CAARI-03 (`sin2`, `sin`) and P-AGARI-01 (the quotients) time out at 60 minutes and count against. |
| SC-004 non-vacuity | met | All 39 mutants killed ("Non-vacuity"); every proven entry has one except P-AGVAL-02, whose equality and order are derived (no single edit breaks them): its evidence is the cover "±0 are equal". All 18 crash harnesses of the two types satisfy their covers. |
| SC-005 per change under 10 minutes on CI | met on the workstation | `P=chord` 100 s wall time with 5 jobs (T052 run); `P=angle` 66.5 s of harness time plus the 19.5 s enumeration. Not measured on the hosted runner: that needs a push. |
| SC-006 API and behaviour unchanged otherwise | met | The commits change only FR-017 to FR-023 and D-008 (the maintainer's choice); the gate passed before the commits (2026-10-18) and after (this run). |
| SC-007 canonical values only; regression tests | met | Proven for every constructor and operation (P-CAVAL-02, P-CAVAL-03); `Deserialize` and the `Cap` decoder tested (`chord_angle_tests`, `encoding_tests`). Each FR-017 to FR-022 test failed before its fix and passes after; the commit messages record the failing values (`dd3c15d`, `2202d81`). |
| SC-008 `normalized` equals upstream | met | Every `ANGLE` row, no tolerance (`cpp_values_diff`), and the probe's large angles (`normalized_matches_cpp_for_large_angles`). |
| SC-009 panics documented | met | "`# Panics` and domain documentation added". |
| SC-010 differential tests | met | `cpp_values_diff` passes on every `CHORD_ANGLE` and `ANGLE` row; the deviation rows are exactly FR-024's (`deviation:negative-canonical`, `deviation:eform-saturate`). |
| SC-011 index by kind | met | Every new id appears once in `PROPERTIES_BY_KIND.md`; `property_kinds` passes; the HTML is re-rendered. |
| SC-012 upstream findings | met | U-014 (T061) and Kani's `%` (T062), both under "Upstream findings"; reporting waits on the maintainer. Kani's `%` is recorded as KI-002 in `KANI_ISSUES.md` (2026-10-25). |
| SC-013 answerable from the evaluation | met | "The canonical-value invariant" answers whether it paid off and whether the other types should follow; "Recommendations" ranks what next. |

# Spec 008: `s2::Cap`

## Summary (spec 008)

Spec 008 verified `s2::Cap`, the first composite type of the suite and the first `Region`
implementor, after a survey against running C++ that found seven differences (K1–K7) and fixed
them. It is also the first part with function contracts (`asin`, `atan2`, `cos`) and lemma
stubs.

- **Crash freedom:** 42 in-scope entries. 31 have crash proofs on the real code or by
  composition, 5 under the function contracts per change, and the two codec entries are 003's;
  `Deserialize` is tested. 3 contract entries are tool limits: `union` (out of memory at 8 GiB,
  timeout at 16 GiB) and both `cell_union_bound`s (timeout at 60 minutes). Per change: 14 crash
  harnesses, 285.8 s of harness time, every cover satisfied.
- **Meaning:** 22 of the 25 counted catalogue entries proven (88%), and P-CCAP-01 for
  `Cell::cap_bound`. Every generic law SC-004 names is proven: three equality laws, three
  `Region` laws (containment implies intersection, the vertex law, the bound contains the
  points) and the codec round trip. The covering law (P-CPREG-04) is not proven, as planned:
  the level needs `asin`'s value. Not proven: P-CPCON-01 (`from_center_area`'s product times
  out), P-CPMEA-01 (the centroid's products time out; the area's proves since D-009's fix),
  P-CPBND-03 (times out).
- **Lemma stubs** made the laws that timed out in R0's first round prove in 7.5 s to 207.2 s,
  and closed spec 006's open P-PTDST-01 symmetry by composition.
- **Library changes:** K1–K7 (FR-017 to FR-023), fixed in six commits, each with a test that
  failed before.
  **D-009**, found by Kani: the area of a cap with r² = −0 was −0 in debug builds; fixed with
  the maintainer's go-ahead (`1b26156`), and the area's proof now passes. **K8**: the empty cap's
  covering is now upstream's four cells, at the maintainer's choice (`8c91967`).
- **Findings:** U-015 (upstream's `Union` does not enclose both caps on 337 of 1,089 rows) and
  Kani's compiler crash on a `thread_local!` stub. Neither is reported yet: both need the
  maintainer's approval. U-015 is recorded in `UPSTREAM.md`, the place for upstream reports
  (T060). K9 is a documented deviation; K10 is in the coverer.
- **Non-vacuity:** all 41 mutants `M-701` … `M-741` are killed.

## Lemma stubs and function contracts (spec 008)

Spec 008 is the first part of the suite whose proofs replace library code (FR-003, FR-003a;
`verify/TRUSTED_BASE.md`, "`s2::Cap` (spec 008)").

- **Function contracts** stand in for `f64::asin`, `atan2` and `cos`, which CBMC lacks (`asin`,
  `atan2`) or models loosely (`cos` returned 0 near 0, research R0). Each returns any value in
  the function's range with the right sign, and the special values exactly. Their tests
  (`verify/tests/libm_contracts.rs`) check them against the real functions on the special
  values and 10⁶ random inputs each, in 0.07 s. They buy crash proofs, not values: 8 entries
  are "proven under contract", and nothing that needs `asin`'s value (P-CPREG-04) is proven.
- **Lemma stubs** replace a library function that a law computes twice on the same input
  (`Point::chord_angle`, `ChordAngle`'s `+`, `Point::is_unit`, `approx_eq_angle`,
  `ChordAngle::from_angle`, `Vector::normalize`) by a pure function of the input's bits: the
  first call draws a value allowed by the stub's lemmas, the second returns it. The lemmas
  (L-CP-01 to -08) are proven on the real code. A stub can make a law fail that holds, if it
  lacks a fact, but cannot make one pass that fails, because each stub only allows values the
  real function can return (research R6, "Soundness"). The mutants `M-730` and `M-731` remove a
  stub's fact and must make their law fail.
- **What they cost:** `stubs.rs` (350 lines, `static Mutex` tables of ten slots), 9 lemma
  harnesses, a `-Z stubbing` flag on every Kani call, unwinding bounds of at least 11 where a
  table is searched (P-CCAP-01 first failed on an unwinding assertion at 9), and a trust
  argument per stub in the trusted base. Two Kani pitfalls: a `thread_local!` table crashes Kani
  0.68.0's compiler (KI-003 in `KANI_ISSUES.md`), and a `Vec` table runs the cell laws out of memory.
- **What they gained:** every law that timed out in R0's first round proves per change, in
  7.5 s to 207.2 s: `add_point` and `add_cap` contain what they added, `intersects` is
  symmetric, containment implies intersection (for caps and for cells), no vertex of a
  non-intersected cell is contained, the complement of a valid cap is valid, and the codec round
  trip under `Cap`'s equality. Spec 006's open P-PTDST-01 symmetry is now proven by
  composition from L-CP-03 and L-CP-04, and its 60-minute weekly harness is gone.
- **For the other types' open timeouts:** the method helps where a law recomputes a value the
  operation already computed (symmetry, "contains what it added"). It does not help where a
  statement compares the library's expression with an identical restatement bit for bit (007's
  `sin2`, `sin` and the quotients, the products of P-CPMEA-01): stubbing the expression would
  reduce the proof to "the code calls the same function", a code review. Candidates: 006's
  P-PTORT-03 (`point_cross`'s sign laws recompute the cross product) and 004's P-IEXP-02/03 and
  005's P-LEXP-02 (symmetric `expanded`, which recomputes the bounds); not 006's P-PTCLS-03
  (unit length needs `sqrt`'s value).

## Effort log (spec 008)

- 2026-10-24 (morning): the survey against running C++ (K1–K6) and the library, the spec and
  its clarification (option A: contracts for `asin` and `atan2`).
- 2026-10-24: the plan with three Kani probe rounds (R0, R0b, R0c: 51 harnesses; lemma stubs
  found; CBMC's `cos` and `log2` and Kani's `%` replayed as checker artefacts), the tasks.
- 2026-10-24: Phases 1–3: the stubs, the description, the C++ generator and vectors, the
  contract tests, the domains, the two new `Region` laws; the library changes K1–K7, each a
  commit with its failing test (`7ab5b66` … `5d961d4`), the differential test (`59f3fe2`), the
  quickcheck twins (`68bdf75`), the probe re-run (`30be646`).
- 2026-10-24: the harnesses of Phases 4–6; the per-change run (47 harnesses, then three fixed and
  re-run); P-CCAP-01; the records. The workstation then lost power during the weekly run.
- 2026-10-25: the weekly run again, the mutants, the CI legs, the records and this evaluation.

## Exclusions and tool limits (FR-002, FR-003) (spec 008)

| Entry or statement | Reason | Evidence instead |
|--------------------|--------|------------------|
| `Display for Cap` | `core::fmt` | — |
| `Deserialize for Cap` (P-CPDES-01) | serde is out of Kani's reach, as in 004–007 | `cap_tests::deserialise_rejects_invalid_caps` |
| P-CPREG-04, the covering covers every contained cell | the level depends on `asin`'s value, which the contract leaves open | `CAP_BOUNDS`, `CAP_CELL` (bit for bit) and the quickcheck twins |
| the values of `angle_radius`, `rect_bound`, `union`, `cell_union_bound`, `approx_eq` | they read `asin` or `atan2`: crash proofs under contract only | the C++ vectors (`cpp_cap_diff`) |
| `union`'s meaning | no property holds: neither union contains both caps on every row (K9, U-015) | the pinned counts in `cpp_cap_diff` |
| `union`'s crash freedom (`c_union`) | out of memory at 8 GiB; alone at 16 GiB, timeout at 60 minutes (2026-10-25). In no CI leg (the runner would report the OOM) | the C++ vectors, the native probe of T028 |
| `cell_union_bound`'s crash freedom (P-CPBND-03), both cell tests on a real cell, `from_center_area`'s and the centroid's products (P-CPCON-01, P-CPMEA-01), L-CP-06a | timeout at 60 minutes (2026-10-25), `verify/known-timeouts.txt` | the C++ vectors; L-CP-06a is trusted as an IEEE-754 fact |
| P-CPBND-02 at an infinite radius | CBMC reports `x / ∞` as NaN (memory, "CBMC NaN check"); replayed natively: level −1 both ways | `cap_tests::covering_level_*` |

## Upstream findings (FR-025) (spec 008)

- **U-015** (`UPSTREAM.md`): `S2Cap::Union` (`s2cap.cc:121–141`) does not contain both caps on
  337 of the 1,089 `CAP_OPS` and `CAP_SPECIAL` rows, by rounding (an ulp of the radius in the
  example), though `s2cap.h` says it "encloses" them. Follow-up T060; not reported (needs the
  maintainer's approval).
- **Kani 0.68.0's compiler crash** on a stub with a `thread_local!` table
  (`kani-compiler/src/intrinsics.rs:243`). Recorded as KI-003 in `KANI_ISSUES.md` (T057,
  2026-10-25), with a minimal reproduction: any harness reaching a `thread_local!` whose value
  has a destructor, stub or not. Not filed with Kani.
- **Checker artefacts, replayed natively** (research R0; memory): CBMC's `log2` and `cos` are
  loose over-approximations, and division by ±∞ is flagged as NaN. Not upstream findings;
  recorded in `TRUSTED_BASE.md`.

## `# Panics` and domain documentation added (SC-010) (spec 008)

- `contains_point`, `interior_contains_point`: a unit point, as upstream checks in debug builds,
  and what another point gives (T020).
- `expanded`: `# Panics` in debug builds for a negative distance (upstream's check) and for +∞
  (the chord-angle sum's check, 007 FR-019).
- `from_center_angle`: `# Panics` in debug builds when the result is invalid (a non-unit
  centre; a NaN angle gives `STRAIGHT`, still valid).
- `union`: the documented deviation (the added margin) and K9; `cell_union_bound`: the empty
  cap's covering, upstream's since K8;
  `Deserialize`: why it validates; `contains_cell`, `intersects_cell`: parity with upstream.
- **No precondition beyond these** (T028): every crash harness's domain is a documented one or
  wider. `union` and `cell_union_bound` are proven over valid caps, because their meaning needs
  unit centres; a native probe found no panic for non-unit centres either (a zero centre,
  centres of length 2, parallel and antipodal).

## Defects found (spec 008)

One proof failed on a library defect:

- **D-009** (`cp_mea::p_cpmea_01__area`, weekly, failed in 1.5 s on 2026-10-25): the area of a cap
  whose radius has length² −0 (`ChordAngle::from_length2(-0.0)` keeps the sign) is −0 in a debug
  build, where upstream's `2π · std::max(0.0, height)` gives +0. `Cap::area` computes
  `0.0_f64.max(h)`, and Rust leaves the sign of `max(+0, −0)` unspecified: the unoptimised build
  returns −0 and the optimised one +0 (replayed natively on the library, both builds). The value
  compares equal to 0, so only the sign bit differs, and the centroid, which multiplies by the
  area, inherits it. **Fixed** with the maintainer's go-ahead (2026-10-25, `1b26156`, T062):
  upstream's comparison, `if 0.0 < h { h } else { 0.0 }`, with the test
  `area_of_a_negative_zero_radius_is_positive_zero`, which failed before (bits
  `0x8000000000000000`) and passes after in both builds. The line left `known-defects.txt`, and
  `p_cpmea_01__area` proves in 1,195.3 s (2026-10-25, 60 minutes and 8 GiB). `M-741` puts the old
  expression back.

The survey's findings, each fixed in its own commit with a test that failed before (SC-007):

- **K1** (FR-017, `7ab5b66`): the covering level through `floor(log2(…))` was one too fine for
  radii just below a power of two: level 2 for 0.11785113019775795, upstream's 1.
- **K2, K3** (FR-018, `a21b7e6`): `contains_cell` and `intersects_cell` tested the cell's
  bounding cap, weaker than upstream: 38 of the 65 grid cases upstream contains were refused, and
  K3's cap reported an intersection upstream does not.
- **K4** (FR-019, `75707f7`): the complement's radius was `STRAIGHT − r`: 2.99999999999999956
  for r² = 1, upstream's 3.
- **K5** (FR-020, `9766059`): `rect_bound` reduced longitudes with `rem_euclid`, so a cap at a
  tiny longitude got the rectangle `[0, 0]`, missing its own centre.
- **K6** (FR-022, `136ef45`): `approx_eq_with` compared the centres at a fixed 10⁻¹⁵: centres
  5.02·10⁻¹⁵ apart were not approximately equal at the default 10⁻¹⁴.
- **K7** (FR-023, `80e0ed8`): the derived `Deserialize` accepted invalid caps (a NaN centre, a
  centre of length 2).

Found by the C++ vectors (T008, T021):

- **K8**: `cell_union_bound` of the empty cap was empty; upstream's gives four cells at level 29.
  Both cover the empty set. First documented as a deviation (T059); the maintainer chose to match
  upstream (2026-10-25, `8c91967`), with the test `empty_cap_covering_matches_upstream`
  (`CAP_BOUNDS`'s empty row; it failed before, with no cells). `cpp_cap_diff` now compares that
  row bit for bit. `Loop`, `Polyline`, `RegionUnion` and `RegionIntersection`, which bound
  themselves through their cap bound, follow upstream for an empty bound too.

Found by the C++ vectors, documented, not changed:

- **K9**: the library's `union` misses part of one cap on 7 of 1,089 rows, all with a single-point
  or nearly single-point cap; upstream's misses on 337 (U-015). Documented; `add_cap` keeps its
  guarantee (P-CPGRO-02).
- **K10**: `RegionCoverer`'s coverings of caps differ from C++'s on 38 of 836 rows, though the
  cap's cell tests and `cell_union_bound` match on every row: in the coverer (T061).

Harness faults found on the way (no library change): P-CPBND-02 at +∞ (CBMC's `x / ∞`),
L-CP-08 at −0 (`ZERO + −0` is +0: the statement then excluded zero; since the code review it
states both zeros on both sides), and P-CCAP-01's unwinding
bound.

## Measurements (spec 008)

Per change: `make verify-values P=cap` with 4 jobs on the 8-thread workstation (2026-10-25,
T052, at `dd4a551`). 45 harnesses, 1,246.8 s of harness time, 7 min 39 s of wall time with the
crate's tests and a cached build:

| Group | Harnesses | Harness time | Longest |
|-------|-----------|--------------|---------|
| crash (`cp_crash`) | 14 | 285.8 s | `c_rect_bound` 118.9 s |
| lemmas (`cp_lem`) | 7 | 210.0 s | `l_cp_04` 111.1 s |
| properties and laws | 24 | 751.0 s | `p_cpeq_02` 173.7 s |

The final run found seven per-change crash harnesses without the `kani::cover!` that T024
asks for: `c_contains`, `c_intersects`, `c_expanded`, `c_add_point`, `c_add_cap`,
`c_rect_bound` and `c_approx_eq`. Each gained two covers, both outcomes, and all seven were
re-run alone with 4 jobs: every cover satisfied, 214.1 s in all (196.7 s before), the longest
`c_rect_bound` at 66.6 s.

Weekly: one at a time at 60 minutes and 8 GiB, 2 jobs (2026-10-25, after the power loss; P-CCAP-01 on 2026-10-24):

| Member | Result | Time |
|--------|--------|------|
| `cp_reg::p_cpreg_01__real` | SUCCESSFUL | 1,037.1 s |
| `cp_crash::c_cell_tests__edges_contains` | SUCCESSFUL | 704.2 s |
| `cp_crash::c_cell_tests__edges_intersects` | SUCCESSFUL | 402.0 s |
| `cp_lem::l_cp_05` | SUCCESSFUL | 551.6 s |
| `cl_cnt::p_ccap_01` | SUCCESSFUL | 341.9 s |
| `cp_gro::p_cpgro_03__real` | SUCCESSFUL | 307.4 s |
| `cp_mea::p_cpmea_01__area` | FAILED (D-009); after the fix SUCCESSFUL | 1.5 s; 1,195.3 s |
| `cp_con::p_cpcon_01__area`, `cp_mea::p_cpmea_01__centroid`, `cp_lem::l_cp_06a` | TIMEOUT | 60 min |
| `cp_crash::c_cell_tests__real`, `cp_crash::c_cell_union_bound` | TIMEOUT | 60 min |
| `cp_crash::c_union` | out of memory at 8 GiB; alone at 16 GiB, TIMEOUT | 60 min |

The two edge-test members re-ran after they gained their covers (2026-10-25, same limits):
`__edges_contains` took 627.4 s and `__edges_intersects` 512.8 s, both with 2 of 2 covers
satisfied.

The timeouts are in `verify/known-timeouts.txt`; D-009's line left `known-defects.txt` with the
fix. CI runs
the six proven members and `p_cpmea_01__area` in the `slow` job's `values-cap` leg, and the
timeouts in `values-limits-f` and `values-limits-g`. `c_union` is in no leg: the runner would
report its out-of-memory as a failure. In all, the value-type suite has 87 weekly members,
about 34 hours one at a time.

The whole `make verify-values` (all nine parts, 4 jobs, 12 minutes each) ran 272 harnesses in
30 min 25 s, with 6,466.8 s of harness time ("Final run").

## Non-vacuity (spec 008)

`make verify-mutants M='M-7*'` with `VERIFY_SLOW_HARNESS_TIMEOUT=60m` (2026-10-25, T052, at
`dd4a551`): all 40 of `M-701` … `M-740` killed in 22 min 25 s (`verify/MUTANTS.md`). After
D-009's fix, `M-708` was regenerated against the new `area` and `M-741` added (the fix
reverted); both killed (2026-10-25, 1 min 16 s together), so 41 of 41.

- **By Kani:** 32, among them `M-740` and `M-741` in the slow tier. `M-741` is also killed by
  its regression test, and both must fail for the kill. `M-740` (`Cell::cap_bound` loops over three
  vertices; killed by `cl_cnt::p_ccap_01`). `M-730` and `M-731` remove a fact from a lemma stub
  and make its law fail, which shows the laws depend on the stubs' facts, not just on their
  purity.
- **By ordinary tests** (the `Test:` headers, run in the mutant's worktree): 9 edits that the
  proofs cannot catch, either by design or because the member that would catch them times out:
  - `from_center_area`'s and the centroid's products (`M-702`, `M-709`), whose members time out;
  - the complement and edge tests inside the cell tests (`M-725`, `M-727`), which are
    abstracted per change;
  - the copied level and longitude expressions (`M-720`, `M-721`);
  - the description's own membership test (`M-728`);
  - two edits that keep a bound true but change the value: `add_cap` without upstream's
    round-up (`M-716`) and `expanded` returning the cap unchanged (`M-717`).

  The C++ vectors (`cpp_cap_diff`), `cap_tests` and the description's tests kill them.
- **Coverage:** every proven entry of §10 has at least one mutant (`PROPERTIES.md`, the
  Mutants column). The exceptions are P-CPBND-03, P-CPREG-04 and P-CPDES-01, which are not
  proven. All 14 per-change crash harnesses satisfy their covers, and so do the two weekly
  edge-test members.

## What next: `LatLng`, and the other `Region` implementors (spec 008)

- **`LatLng`: go.** Two `Angle`s with a range, so `angle_spec` carries its construction,
  validity and normalisation. Its conversions from and to `Point` go through `atan2` and `sin`/
  `cos`, which now have contracts: their crash freedom is provable under contract, as
  `rect_bound`'s was (84.6 s in R0b), and their values stay with the C++ vectors.
- **The generic `Region` laws reach next:**
  1. **`Rect`** (`s2::Rect`, spec 005's description): its cell tests check the vertices first,
     as `Cap`'s do, so `no_intersect_no_vertex` applies, and the abstract cell and the stubs
     carry over. Its edge tests go through `edge_crossings`, heavier than `Cap`'s.
  2. **`Cell`**, through P-CCAP-01: its `cap_bound` contains its vertices; `contains_point` and
     `rect_bound` remain.
  3. **`PointRegion`** (a cap of radius zero: `Cap`'s proofs nearly apply as they stand) and
     **`CellUnion`**'s float methods (`cap_bound`, `rect_bound`, through `Cell`'s).
  `Loop`, `Polygon` and `Polyline` need edge geometry over symbolic vertex lists: research
  first.

## Comparison with 004–007 (spec 008)

004–007 proved each value type's operations whole; spec 008 is the first to prove a composite
type by composing proofs: lemmas about `Point` and `ChordAngle` on the real code, and laws about
`Cap` with those operations replaced by pure stubs. The laws that timed out in 006 and in R0's
first round take seconds to a few minutes. It is also the first part with function contracts,
and the first whose library changes port upstream algorithms (the cell tests) rather than fix
arithmetic. As in 007, the checker's arithmetic models (`log2`, `cos`, `x / ∞`) set limits that
native replays had to sort from real failures.

## Limitations (spec 008)

- **Trust:** the function contracts (tested, not proven, against the platform's libm), the
  stubs' purity argument (each lemma proven on the real code but L-CP-06a, IEEE multiplication's
  commutativity, which times out at 60 minutes and is trusted as an IEEE-754 fact), and the
  abstract cell (its
  vertices and edges any box points, an over-approximation of every real cell). All recorded in
  `TRUSTED_BASE.md`.
- **Values through `asin` and `atan2`** have crash proofs only; the C++ vectors check them bit
  for bit. P-CPREG-04 is not proven for the same reason.
- **Copies of library expressions:** P-CPBND-02 is stated on the level expression of
  `cell_union_bound`, and P-CPBND-05 on a drift-guarded copy of the private `reduce_longitude`.
  A change to the library's expression is caught by the drift guard and the ordinary tests
  (`M-720`, `M-721`), not by these harnesses.
- **Weekly tool limits:** `from_center_area`'s and the centroid's products, `cell_union_bound`'s
  crash proof and both cell tests on a real cell time out at 60 minutes; `union`'s crash proof runs
  out of memory at 8 GiB and, alone at 16 GiB, times out. The C++ vectors check each.
- **The edge part of the cell tests** is abstracted per change (`any_edge_result`); its crash
  proofs and the law with the real edge test are weekly (both proven, over the abstract cell). The `Region` laws never reach the edge
  test's value, so its mutant (`M-727`) is killed by the tests and the C++ vectors.

## Recommendations (FR-016) (spec 008)

U-015 is recorded in `UPSTREAM.md` (T060) and Kani's `thread_local!` crash in `KANI_ISSUES.md`
(T057); D-009 is fixed and K8 matches upstream (all 2026-10-25). Ranked:

1. **`Rect` next for the `Region` laws**, reusing `laws::region`, the abstract cell and the stubs;
   then `LatLng` as the next value type, with the `atan2` contract.
2. **Use lemma stubs for laws that recompute a value**, not for bit-for-bit comparisons with an
   identical restatement; candidates in "Lemma stubs and function contracts".
3. **State P-CPBND-02 on the library's own function**: a `#[doc(hidden)]` level helper in `cap.rs`
   would let the harness call it instead of a copy of its expression.
4. **Follow-ups outside this spec:** K10 (the coverer, T061); the inexact private
   `ieee_remainder` of `core/src/s1/interval.rs:589`, whose comment says "same as C
   `remainder()`" (T058; spec 004's `expanded`).

## Final run (spec 008)

2026-10-25 (T052), at `dd4a551` (this part's records committed, before D-009's and K8's fixes), one run at a
time on the workstation:

- **`make test`** passed twice. First at 04:55–04:57, before the records. Then at 06:19, after
  the covers and the records, with cached builds: the Rust suites with `cap_tests` and
  `cpp_cap_diff`, 914 Python tests, mypy and stubtest, the wasm suite (277) and `cargo-deny`.
  `cargo test` in `verify/` passed after the covers too.
- **`make verify-values`, all nine parts** (`VERIFY_JOBS=4`, 12 minutes each, 05:02–05:32): 272
  of 272 harnesses successful in 30 min 25 s, with the `-Z stubbing` flag across every part.
  The crate's tests ran first, all passing: the unit tests (87), `libm_contracts` (4),
  `property_kinds` (3), `values_spec_vs_cpp` (11), `spec_vs_cpp`, `union_spec_vs_cpp`,
  `wire_spec_vs_cpp` and `uv_projection`, then the enumeration (7 tests, 21.6 s). Every cover
  the harnesses declare was satisfied. Seven of `Cap`'s crash harnesses declared none, so they
  gained covers and were re-run (see "Measurements").
- **`make verify-values P=cap`** (05:32–05:40): 45 of 45 successful, 7 min 39 s.
- **`make verify-mutants M='M-7*'`** (05:40–06:02): 40 killed, 0 survived.
- **The weekly members** ran before, one at a time (see "Measurements"). After they gained
  covers, the two edge-test members re-ran (627.4 s and 512.8 s, 2 of 2 covers each).

After the maintainer's answers (2026-10-25: fix D-009, match upstream on K8, keep upstream
reports in `UPSTREAM.md`, never push), at `8c91967`:

- **`p_cpmea_01__area`** (weekly, 60 minutes, 8 GiB): SUCCESSFUL in 1,195.3 s.
- **`make verify-mutants M='M-708 M-741'`**: both killed.
- **`make verify-values P=cap`**: 45 of 45 successful in 7 min 13 s, 1,219.6 s of harness
  time.
- **`make test`** passed (07:55–07:57), and `cargo test` in `verify/` passed too.

After a code review of the branch (2026-10-25):

- **L-CP-08 covers −0.** `pure_add`'s two zero rules test `== 0.0`, so they also fire for a
  −0 length, which the domain allows; L-CP-08 proved only `ZERO + b`, and P-CAARI-01 only
  `a + ZERO`. L-CP-08 now states both rules for both zeros: proven in 11.9 s, covers 2/2;
  `M-732` still killed. `pure_add` rests on L-CP-08 and L-CP-05 alone.
- **`union`'s centre is compared with C++** in `cpp_cap_diff`, within 10⁻¹⁵ (2.8·10⁻¹⁶ at
  most, from the larger radius). The 30 rows with antipodal centres are skipped: the direction
  is ill-conditioned there (up to 5.3·10⁻³ apart). A mutant that interpolates from the wrong
  cap now fails the test.
- **The empty region's bound is documented** on `Region::cell_union_bound` and
  `RegionCoverer::fast_covering`: an empty `Cap`, `Rect` or `CellUnion` gives the four
  level-29 cells around (1, 0, 0) (K8, as upstream); `covering` gives none (measured).
- **The three harnesses that never finish** (`c_cell_union_bound`, `c_cell_tests__real`,
  `c_union`) now say so in their doc comments.

## Success criteria (SC-001 … SC-013) (spec 008)

Checked one by one against the records above (T053, 2026-10-25). 11 of the 13 are met, SC-006
on the workstation only. SC-001 is met in part and SC-003 is not met.

| Criterion | Result | Evidence |
|-----------|--------|----------|
| SC-001 crash freedom, whole domain | met in part | 39 of the 42 entries are proven over their whole documented domain, with no size bound: 31 on the real code or by composition, 5 under contract, and the codec's 2 (003). The other 3 are contract entries that are tool limits: `union` (out of memory at 8 GiB, timeout at 16 GiB) and both `cell_union_bound`s (timeout at 60 minutes). See `INVENTORY.md` §9. |
| SC-002 every in-scope entry | met | 38 crash proofs and 3 tool limits, each with its 60-minute measurement. `Deserialize` is out of the checker's reach (serde, as in 004–007) and tested; `Display` is excluded with a reason ("Exclusions and tool limits"). The contracts pass their tests against the real functions (`libm_contracts`, 4 tests, 10⁶ inputs each). |
| SC-003 at least 90% of the counted entries | not met (88%) | 27 entries in §10. P-CPREG-04 (needs `asin`'s value) and P-CPDES-01 (serde) are not counted. Of the 25 counted, 22 are proven. Against: P-CPCON-01 (`from_center_area`'s product times out at 60 minutes), P-CPMEA-01 (the centroid times out; the area proves since D-009's fix) and P-CPBND-03 (times out). Fixing D-009 did not reach 90%: the centroid half of P-CPMEA-01 still times out. |
| SC-004 the generic laws | met | Three equality laws (P-CPEQ-01, `laws::eq`), the three `Region` laws (P-CPREG-01 per change with the edge test abstracted and weekly with the real one, P-CPREG-02, P-CPREG-03), and the codec round trip (P-CPCOD-01). The fourth `Region` law, P-CPREG-04, is not proven: its level needs `asin`'s value, as the amended SC-004 allows. |
| SC-005 non-vacuity | met | All 41 mutants are killed, and every proven §10 entry has one ("Non-vacuity"). All 14 per-change crash harnesses satisfy their covers, the seven added at T052 included, and so do the two weekly edge-test members. The lemmas L-CP-* are facts the stubs rely on, not catalogue entries; `M-730` and `M-731` show the laws depend on them. |
| SC-006 per change under 10 minutes on CI | met on the workstation | `P=cap`: 7 min 39 s of wall time with 4 jobs, build and crate tests included; 1,246.8 s of harness time. Not measured on the hosted runner: that needs a push (T056). |
| SC-007 regression tests; the probe's differences gone | met | K1–K7 are fixed in six commits (K2 and K3 share one), each with a test that failed before its fix (`7ab5b66` … `80e0ed8`). After the fixes, every probe case matches C++ (`probe/rust/survey_rust_after.txt`, `30be646`). |
| SC-008 differential tests | met | `cpp_cap_diff` passes on every row. Its deviation rows are those the rustdoc documents: K9 (`union`'s margin, 7 rows); K8's empty-cap row is exact since the change (`8c91967`). K10's 38 rows are the coverer's, outside `Cap` (T061). |
| SC-009 API and behaviour unchanged otherwise | met | The commits change only FR-017 to FR-023, then D-009 and K8 at the maintainer's choice (2026-10-25). `make test` passed before the records (04:55), after them, and after the two fixes ("Final run"). |
| SC-010 panics documented | met | "`# Panics` and domain documentation added". |
| SC-011 index by kind | met | Every new id appears once in `PROPERTIES_BY_KIND.md`, `property_kinds` passes, and the HTML is re-rendered. |
| SC-012 upstream findings | met | U-015 (T060) and Kani's `thread_local!` crash (T057), both under "Upstream findings", are recorded in `UPSTREAM.md` and `KANI_ISSUES.md` (2026-10-25); neither is filed. |
| SC-013 answerable from the evaluation | met | "Summary" says how much of `Cap` is proven. "What next" names the `Region` implementors the laws reach next (`Rect`, `Cell`, `PointRegion`, `CellUnion`) and gives `LatLng` a go. |

## The coverage review of 2026-10-26

A review after `074e923` found two gaps in the value types' crash-freedom records: the inputs
outside each documented domain (does the operation panic exactly as its `# Panics` says?) and the
62 floating-point entries specs 004–007 excluded. Both are recorded in
[`../COVERAGE.md`](../COVERAGE.md): sections 10 and 11 of `INVENTORY.md` carry the per-entry
results, and 12 `# Panics` sections were corrected (none in behaviour).

## Summary (spec 010)

Spec 010 verified `s2::LatLng`, the next value type on spec 008's list, together with the
list's first item, the generic `Region` laws for `s2::Rect`, since every one of them goes
through `LatLng`'s conversions. A survey against running C++ (`specs/010-formal-verify-latlng/
probe/`) found K1–K5 first; the planning probe R0 widened K4.

- **Crash freedom:** 31 in-scope entries of 33. The 26 Kani entries are proven over every pair of
  `f64` (NaN and the infinities included) but where an operation makes a new NaN, `get_distance`
  over its documented domain and, outside it, every pair without an infinity: 9 harnesses, 35.5 s,
  every cover satisfied. The four `geo-types` conversions by composition, `Deserialize` tested.
- **Meaning:** all 19 counted catalogue entries of §11 are proven (SC-003: 100%), 10 directly
  (two of them weekly) and 9 by composition or under contract; P-LLDST-04 (the distance's
  symmetry within a tolerance) and P-LLDES-01 (serde) are not counted.
- **`Rect` as a `Region`:** two of the four laws proven under contract (P-LREG-01 with the lemma
  L-LL-01, P-LREG-02 with a tested contract on `Cell::rect_bound`), and P-LMEM-03 (the point test
  receives a valid `LatLng`). The bounding-cap and covering laws need the functions' values.
- **Library changes:** FR-017 (the exact remainder, shared with `Angle::normalized`), FR-018 (the
  clamp before the root) and FR-019 (the widened bounding cap), each its own commit with a test
  that failed before; new C++ vectors (FR-021) match every row but FR-019's radii.
- **Upstream finding:** U-016, upstream's `GetCapBound` misses contained points; recorded in
  `UPSTREAM.md`, not reported.
- **Non-vacuity:** all 36 mutants `M-901` … `M-936` are killed (2026-10-28, about 40 minutes).

## Restatements, drift guards and the checker's products (spec 010)

The first draft stated the four "exact output" entries (`latitude`, `to_point`, `get_distance`,
the scalings) and the constructors as the library's result against a restatement computed again
in the harness, with pure contracts so both calls of `sin`, `cos`, `atan2` and `sqrt` saw one
value. All five timed out at 12 minutes (2026-10-28): CBMC does not see that two copies of the
same float products are one value, as spec 008's identical restatements found
(`from_center_area`'s quotient). The final statements prove the parts without a product in Kani
(the longitude, `to_point`'s `z`, the sum and difference: 12 s to 84 s) and tie the products to
the restatements through drift guards that compare the source text
(`latlng_spec_tests::conversions_are_the_restatements`, `scalings_are_angle_products`), with one
lemma where the texts differ (L-LL-02: `x.min(1.0)` is upstream's `std::min(1.0, x)`); the
constructors run with `Angle`'s uninterpreted. The C++ vectors check the restatements bit for
bit, so the chain is C++ ⇔ restatement (vectors) ⇔ library (guard, Kani for the rest).

Two more checker findings, replayed before they were believed:

- **CBMC's built-in `sqrt` of a negative argument** fails the NaN check ("NaN on division" inside
  `<builtin-library-sqrt>`), where IEEE-754 defines the NaN: `o_get_distance` proves under
  `sqrt_contract`.
- **A stub table needs its slots' unwinding:** `Table::get` loops over ten slots, so the `rl_reg`
  harnesses need `#[kani::unwind(11)]` (the first run's "unwinding assertion loop 1").

## Effort log (spec 010)

| Date | Work |
|------|------|
| 2026-10-27 | Survey (`probe/`: 132,043 cases on C++ and the library), spec, clarifications |
| 2026-10-28 | R0 (the cap-bound probe, 74.6 million points), plan and tasks; FR-017 to FR-019 with their tests; the C++ generator and diff test; the quickcheck twins; the description, stubs and contract tests; three Kani rounds (28, 10 harnesses); the mutants; the records |

## Exclusions and tool limits (spec 010)

- **Excluded:** `Display` and `to_string_in_degrees`' formatting (`core::fmt`, as every type's).
- **Outside the checker:** `Deserialize` (tested), the `geo-types` conversions (by composition:
  the verification crate does not enable the feature for four one-line functions).
- **Not proven:** P-LLDST-04 and P-LREG-03 (the functions' values), P-LREG-04 (as P-CPREG-04).

## Upstream findings (FR-022) (spec 010)

- **U-016**, `S2LatLngRect::GetCapBound` (`s2latlng_rect.cc:298–332`): the bound misses points the
  rectangle contains, by up to an ulp of π/2 in angle, in pole caps (the north pole as a rectangle
  and a point one ulp past it) and in caps around the centre (a point a few ulps inside a vertex):
  114 of the survey's 129,595 contained points, 993 of the vectors' 5,552, 10,507,883 of R0's
  74,647,924. The library widens the chosen cap (FR-019), a documented deviation. Follow-up:
  T014; not reported upstream (the maintainer's call).

## Defects found (spec 010)

None beyond the survey's K1, K3 and K4, which FR-017 to FR-019 fixed. No D-010.

## Measurements (spec 010)

Per change (`make verify-values P=latlng` and `rl_reg` in `P=rect`), from the runs of 2026-10-28
on the workstation (4 jobs):

| Harness | Time | Harness | Time |
|---------|------|---------|------|
| `ll_crash::c_construction` | 0.3 s | `ll_nrm::p_llnrm_01` | 1.5 s |
| `ll_crash::c_e_forms` | 0.3 s | `ll_nrm::p_llnrm_03` | 20.3 s |
| `ll_crash::c_from_point` | 10.7 s | `ll_val::p_llval_01` | 0.4 s |
| `ll_crash::c_to_point` | 0.4 s | `ll_dst::p_lldst_02` | 9.2 s |
| `ll_crash::c_normalized` | 0.9 s | `ll_dst::p_lldst_03` | 8.1 s |
| `ll_crash::c_get_distance` | 14.7 s | `ll_ari::p_llari_01__add` | 67.8 s |
| `ll_crash::o_get_distance` | 7.2 s | `ll_ari::p_llari_01__sub` | 84.1 s |
| `ll_crash::c_approx_eq` | 0.4 s | `ll_eq::p_lleq_01` | 1.3 s |
| `ll_crash::c_arithmetic` | 0.6 s | `ll_lem::l_ll_01` | 125.9 s |
| `ll_con::p_llcon_01` | 28.5 s | `ll_lem::l_ll_02` | 0.0 s |
| `ll_fp::p_llfp_01` | 36.9 s | `rl_reg::p_lreg_01` | 35.0 s |
| `ll_fp::p_llfp_02` | 2.6 s | `rl_reg::p_lreg_02` | 305.5 s |
| `ll_tp::p_lltp_01` | 12.0 s | `rl_reg::p_lmem_03` | 18.6 s |
| `ll_tp::p_lltp_02` | 1.5 s | | |

Weekly: `ll_apx::p_llapx_01` 466.6 s, `ll_apx::p_llapx_02` 483.7 s (the `values-latlng` slow leg).
{{final}}

## Non-vacuity (spec 010)

`make verify-mutants M='M-9*'` (2026-10-28): **36 killed, 0 survived**. Every proven entry of §11
and every `Rect` law instance has at least one: 24 are killed by Kani harnesses (three in the
weekly tier), 12 by ordinary tests through `Test:` headers. Those are the drift guards'
mutants (the latitude, `to_point`'s products, the haversine, the scalings, a restatement's
swapped sine and cosine), and the three that revert FR-017, FR-018 and FR-019, killed by
`latlng_tests`, `rect_tests` and `cpp_latlng_diff`. Six patch the description or the stubs: four
`latlng_spec` mutants (M-911, M-912 on the reduction's copy; M-928 and M-929 on `valid` and
`upstream_to_point`), and two stub mutants (M-930: `any_cell_bound` without L-LL-01's
non-emptiness fails P-LREG-01; M-931: `cell_rect_bound_contract` without its vertex clause
fails P-LREG-02), which show the laws depend on the facts the stubs carry. Every crash harness
satisfies its covers.

## What next (spec 010)

- **`LatLng`'s values** through `atan2`, `sin_cos` and `asin` stay with the C++ vectors, as every
  type's since 008; the drift guards make the formulas' text the restatements', and no proof
  can say more without the functions' values.
- **The remaining `Region` implementors:** `PointRegion` (a cap of radius zero: `Cap`'s proofs
  nearly apply), `CellUnion`'s float methods (`cap_bound`, `rect_bound` through `Cell`'s), and
  `Cell`'s own `cap_bound`, `rect_bound` and `contains_point`. `Loop`, `Polygon` and `Polyline`
  need edge geometry over symbolic vertex lists: research first.

## Comparison with 004–008 (spec 010)

`LatLng` is the smallest type since `Angle`, and most of it proved in seconds. What 010 adds is
methodical: pure contracts, a contract on a library function, and drift guards for formulas the
checker cannot compare with themselves. As in 008, the survey against running C++ found the
defects (three of the library's, one of upstream's), and the planning probe, a native sweep with
adversarial points, found that the defect was larger than the survey had shown.

## Limitations (spec 010)

- **Trust:** the contracts (tested against libm), the C library's determinism, the
  `Cell::rect_bound` contract (tested on every cell to level 5 and 10⁶ random cells), IEEE
  `fmod`'s exactness, and the drift guards' premise that identical Rust expressions compute
  identical values (no contraction). All in `TRUSTED_BASE.md`.
- **The widening (FR-019)** is measured, not proven: R0's margins doubled and quadrupled, with an
  analytic reason in the rustdoc. A point the sweep does not reach could in principle need more.
- **`Rect`'s laws** run per change in the `values-rect` leg; `p_lreg_02` takes 5 minutes.

## Recommendations (FR-016) (spec 010)

Ranked:

1. **`PointRegion`** next for the `Region` laws (it is a cap), then **`CellUnion`'s float
   bounds**; and `Cell`'s remaining float methods.
2. **The two missing upstream operations (FR-020):** `S2LatLng::Encode`/`Init` (a 16-byte codec,
   the last value type with an upstream codec and none here) and
   `S2LatLngRect::Intersects(const S2Cell&)` (the exact cell test, which differs from
   `MayIntersect` on 158 of 20,000 random pairs; the library has the edge helpers privately).
3. **The other two inexact remainders:** `s1::Interval`'s private `ieee_remainder`
   (`core/src/s1/interval.rs:594`, spec 008's T058) and the projections' `remainder`
   (`core/src/s2/projections.rs:68`), both `x − round(x / y) · y`; `s1::angle::remainder_2pi`
   is the exact reduction to use (spec 010 FR-017).
4. **Validity as a bounded type** for `LatLng` (spec 009's pattern): an API change, so a spec of its
   own.
5. **U-016 upstream:** the maintainer decides whether to report it.

## Success criteria (SC-001 … SC-013) (spec 010)

{{sc}}
