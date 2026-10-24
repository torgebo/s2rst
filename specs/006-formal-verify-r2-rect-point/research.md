# Research: Formal Verification of `r2::Rect` and `Point`

Planning decisions for [spec.md](spec.md). Each section gives the decision, the reason and the
alternatives that were rejected. R0 is the planning probe; R1 is the re-reading of upstream.

## R0: Planning probe (run 2026-10-14)

**What runs.** `probe/r0_harnesses.rs` holds 29 Kani harnesses: 12 for the u-v rectangle and
17 for the point. They run from a scratch crate that depends on `s2rst` and `s2rst-verify`,
so 005's `real_spec` is reused as it is. The set-up is a copy of `run-verify.sh`: 4 jobs, a
4 GiB ceiling and 12 minutes per harness, on Kani 0.68.0 and CBMC 6.11.0.

**When.** The probe waited for the queue of 004's and 005's re-runs, mutants and weekly
members, because verification runs go one at a time, and ran on 2026-10-14 from 00:12 to
00:28. The plan was first written on the pilot's figures and 005's R0; the results below
reopened three decisions, amended before `/speckit-tasks`.

**What it measures, and what the plan assumed before it ran.**

| Harness | Measures | Assumed from |
|---------|----------|--------------|
| `r0_uv_valid_model`, `r0_uv_contains_model`, `r0_uv_contains_point_model` | validity, containment and membership against the product of two `real_spec`s | 005 R0: 0.1–2.5 s for the same statements on `s2::Rect` |
| `r0_uv_union_least`, `r0_uv_intersection_common_points`, `r0_uv_add_point` | the set operations against the product | 005 R0: 16–48 s for `s2::Rect`; the pilot: 8–41 s |
| `r0_uv_expanded_crash_finite`, `r0_uv_expanded_contains` | expansion keeps validity; finite non-negative margins contain the original | 005 R0: `r1` expansion containment 3.4 s; the pilot: 545 s with projection |
| `r0_uv_from_center_size`, `r0_uv_project_inside`, `r0_uv_approx_eq_reflexive`, `r0_uv_vertex` | the constructor from a centre and size, projection, approximate equality at finite bounds, `vertex` for every `i32` | the pilot; 005 R0 |
| `r0_pt_cmp_is_upstreams_order` | `cmp_point` is C++'s `lexicographical_compare` for every triple, NaN included | the pilot: 7–9 s |
| `r0_pt_cmp_transitive`, `r0_pt_ties_are_equality` | the order is a total preorder whose ties are `==`, without NaN | the pilot: 9 s |
| `r0_pt_is_normalizable_model`, `r0_pt_is_unit_model` | the classifications against their definitions | the pilot: 1802 s for both together |
| `r0_pt_ensure_normalizable_library` | expected **counterexample** (K5) | the survey |
| `r0_pt_scale_draft` | FR-020's draft is normalizable, in [1, 2), and exact | none: three products by powers of two |
| `r0_pt_from_coords_finite`, `r0_pt_from_coords_infinite` | the finite domain is crash-free; an infinite coordinate fails the NaN check (`∞ · 0`) | 005 R0's NaN-check finding |
| `r0_pt_add_sub_neg` | the arithmetic is the coordinate-wise IEEE-754 operations | none: single operations |
| `r0_pt_ortho_odd`, `r0_pt_ortho_nonzero`, `r0_pt_point_cross_antisymmetric` | the orthogonal directions' laws over the box `[−1, 1]³` | none: cross products and a `sqrt` |
| `r0_pt_chord_angle_range`, `r0_pt_distance_self` | the chord angle's range; zero distance to itself | none |
| `r0_pt_distance_symmetric`, `r0_pt_rotate_crash` | **whether the checker models `atan2`, `sin` and `cos`** | answered by codegen, below: `atan2` no, `sin` and `cos` yes |

**The trigonometric question (FR-003), answered by codegen alone (2026-10-13).** The probe
crate compiled with `cargo kani --only-codegen`, which runs no solver. Kani reported one
unsupported foreign function, and the harnesses' goto programs show which:

- **`atan2` is not modelled.** `f64::atan2` calls the C library, and Kani replaces the call with
  a failing check: "call to foreign "C" function `atan2` is not currently supported by Kani".
  `distance`, `stable_angle`, `approx_eq`, `approx_eq_angle` and `approx_eq_with` reach it
  through `r3::Vector::angle`. Their crash proofs are therefore tool-limit exclusions (5
  entries), and P-PTDST-02 (symmetry) is "not proven (tool limit)". Their values rest on the
  C++ vectors. Paths that never reach the call are still proven: `angle` returns 0 early for
  equal vectors, so P-PTDST-01's zero distance and reflexive approximate equality stand.
- **`sin` and `cos` are modelled.** They lower to CBMC's built-in library functions
  (`<builtin-library-sin>`, `<builtin-library-cos>`), which over-approximate the result within
  [−1, 1]. A crash proof over that over-approximation covers the true values, so `rotate` gets
  one (1 entry).

R0's run still measures the cost of `r0_pt_rotate_crash`, and confirms that
`r0_pt_distance_symmetric` fails on the unsupported call. The decision does not wait for it.
No stubbing is used (001–005's rule), so `atan2` is not replaced by a model of our own.

**Results (2026-10-14).** 23 of 29 succeeded. The times, and the concrete playback of each
failure, are kept in `probe/r0_results.tsv` and `probe/r0_counterexamples.txt`.

| Harness | Result | Time |
|---------|--------|------|
| the 12 `r0_uv_*` | succeeded | 0.1–28.2 s: `expanded_contains` 28.2 s, `from_center_size` 19.3 s, the rest at most 6.0 s |
| `r0_pt_cmp_is_upstreams_order`, `_cmp_transitive`, `_ties_are_equality` | succeeded | 0.4–0.9 s |
| `r0_pt_is_normalizable_model`, `r0_pt_is_unit_model` | succeeded | 0.2 s, **644.5 s** |
| `r0_pt_scale_draft` | succeeded: FR-020's draft is normalizable, in [1, 2), and exact | 113.8 s |
| `r0_pt_add_sub_neg`, `r0_pt_rotate_crash` | succeeded | 150.5 s, 134.1 s |
| `r0_pt_ortho_nonzero` | succeeded | 358.0 s |
| `r0_pt_chord_angle_range`, `r0_pt_distance_self` | succeeded | 6.2 s, 3.2 s |
| `r0_pt_ensure_normalizable_library` | failed, as expected (K5): `(−4.17e-309)³` gives a vector that is not normalizable | 8.6 s |
| `r0_pt_distance_symmetric` | failed, as expected: the `atan2` check | 14.9 s |
| `r0_pt_from_coords_infinite` | failed, as expected: `∞ · 0` in `normalize`, and the division below | 2.1 s |
| `r0_pt_from_coords_finite` | **failed**: a division by an infinity reported as NaN (finding 1) | 4.0 s |
| `r0_pt_ortho_odd` | **failed**: the zeros' signs (finding 2) | 108.3 s |
| `r0_pt_point_cross_antisymmetric` | **failed**: K11 (finding 3) | 142.3 s |

**Finding 1: the checker reports a division by an infinity as NaN.** The counterexample of
`r0_pt_from_coords_finite` is `(0, 0, −2.68e154)`: the squared length overflows, and
`normalize` computes `1 / ∞`, which IEEE-754 defines as 0 (the library returns the zero
vector, K7). CBMC 6.11's NaN check reports "NaN on division" there, and did the same for
`ensure_normalizable`'s `2 / −∞`. The check over-approximates, so the failure is spurious, not
a missed NaN.

- **Decision.** `from_coords` and `normalize` range over `NormableTriple`: finite, with a
  finite squared length, computed by the description in upstream's `Norm2` order. The rest is
  a tool limit, and ordinary tests check the zero vector returned there (FR-004, amended).
  FR-020's scaling multiplies by two exact powers of two and never divides, so it is not
  affected.
- **Fallback.** If the products in the assumption make a harness slow, the domain becomes
  coordinates within `2⁵¹⁰` (comparisons only), and the gap is recorded.

**Finding 2: the zeros' signs.** The counterexample of `r0_pt_ortho_odd` is
`(−0, −7.6e-249, 0)`. The cross product underflows, `normalize` returns `(+0, +0, +0)` for both
`a` and `−a`, and `−ortho(a)` is `(−0, −0, −0)`. At ordinary sizes, a coordinate computed as
`0 − 0` is `+0` for both signs of the input too.

- **Decision.** P-PTORT-01 and P-PTORT-03 compare with `same` (IEEE equality per coordinate),
  not bit patterns. The box has no NaN, so `same` loses only the zeros' signs.
- The other bit-for-bit statements stand: their results are single IEEE-754 operations or
  exact scalings, whose zeros' signs are determined.

**Finding 3: K11.** The counterexample of `r0_pt_point_cross_antisymmetric` is two distinct
tiny points, around 1e-301, whose cross product underflows. Both orders then take the `ortho`
branch, and the swap law fails. A follow-up found the same for distinct parallel points that
pass `is_unit`, in the library and in Go (spec K11, U-013).

- **Decision.** P-PTORT-03 states the laws wherever `(p + q) × (q − p)` is non-zero, and
  FR-022 documents the true exception, as Go's behaviour.
- **Alternative left to the maintainer.** Making the laws hold, by perturbing parallel points
  as C++'s `RobustCrossProd` does, changes behaviour at the cost of exact arithmetic, outside
  this feature.

**Confirmed:**

- FR-020's scaling (R8), by `r0_pt_scale_draft`;
- `sin` and `cos` modelled, so `rotate` has a crash proof (134.1 s);
- `atan2` reported as unsupported, as codegen predicted;
- the u-v rectangle's composition through `real_spec` is cheap: at most 28.2 s (R5).

**Tiers** follow in R12.

**Findings during implementation (2026-10-14).** The first runs of the full suite reopened
four decisions; each is amended in the data model and the catalogue:

- **Finding 4: the checker's `sqrt` is not a function.** `p_ptort_01__reference_dir` failed,
  though `reference_dir()` is a call of `ortho()`. A two-line harness confirmed it:
  `x.sqrt().to_bits() == x.sqrt().to_bits()` fails for a symbolic finite positive `x` in 2.1 s
  (CBMC's C-library model constrains the root without fixing it). So no harness can compare two
  normalisations bit for bit. **Decision:** P-PTORT-01 (oddness, `reference_dir`, `ortho`'s
  exact value), P-PTVAL-02's normalising clause and P-PTORT-03's `ortho` branch are tool limits,
  checked against the C++ vectors and pinned by `point_tests.rs`; P-PTVAL-02 keeps a proof of
  its clauses without a square root. The planned mutants of P-PTORT-01 (`ortho` at the largest
  axis, `reference_dir` not `ortho`) were dropped: any kill would come from the spurious
  failure.
- **Finding 5: the finite squared length is hard for the checker.** Over all of
  `NormableTriple`, `from_coords` and `normalize` time out at 12 minutes in every form tried
  (with the assumption written as products or as ranges); over every coordinate at most `2⁵¹¹`
  they finish in 2 s, and the band `(2⁵¹¹, 2⁵¹²)` alone times out. **Decision:** the per-change
  domain is `BoundedTriple` (`any_bounded_triple`), the band `NormableBand` is weekly, and the C++
  vectors' rows at `1e154` test it.
- **Finding 6: K12, and the robust cross product's length.** Writing `upstream_normalize`
  showed that C++ keeps the zeros' signs where the norm is zero and the library (as Go)
  returns `(+0, +0, +0)`; the new vectors showed that C++'s `RobustCrossProd` answers nearly
  parallel points from a `long double` step, twice the cross product, where the library's exact
  step returns it normalised (48 rows; the directions agree within 2·10⁻¹⁶ rad), and that
  upstream's `DotProd` sums from `+0` (an angle between vectors whose products all vanish is 0
  in C++, π in the library). **Decision:** documented (FR-021, FR-022) and stated in the
  differential test; no behaviour change.
- **Finding 7: the clause-heavy harnesses.** Statements written as one harness timed out where
  R0 had measured one clause; split by clause (`__*`), most members prove in seconds, and those
  over 6 minutes are weekly (R12). P-QSET-01's "`add_rect` is `union`, bit for bit" was false
  for two empty rectangles, as in C++ (`Union` returns the other operand, `AddRect` keeps its
  own); the statement now excepts them.

**Decisions taken from 005's R0**, which needed no re-measuring:

- **The NaN check.** Kani counts a NaN produced by arithmetic as a failed check. The point's
  crash domains therefore exclude the inputs where `∞ − ∞`, `0 · ∞` or an overflow into them
  arises. The spec was amended at planning (FR-004, FR-022).
- **Composition through descriptions.** The u-v rectangle is stated against the product of
  two `real_spec`s, never component-wise against `r1::Interval`'s own results, which timed
  out in 005 (R5).

## R1: Re-reading the pinned upstream (v0.14.0-40-g37a4501, Go at the local checkout)

Read for this plan: `r2rect.{h,cc}`, `util/math/vector.h`, `s2point.h`, `s2pointutil.{h,cc}`,
`s2edge_crossings.cc`, Go's `r2/rect.go`, `s2/point.go`, `s2/edge_crossings.go` and
`r3/vector.go`.

- **The u-v rectangle (K1–K4)** stands as surveyed. `R2Rect::FromCenterSize` builds through the
  checking constructor; `Project` checks `!is_empty()` in debug builds.
- **The order.** `Vector3::operator<` is `std::lexicographical_compare` with `<`, so a NaN pair
  counts as equal and the comparison moves on. That is exactly `cmp_point`: R0's first point
  harness states it for every triple, so K8's behaviour is pinned by the same property that
  defines the order.
- **The unit-length test.** `S2::IsUnitLength` is `fabs(p.Norm2() - 1) <= 5 * DBL_EPSILON`,
  bit for bit the library's (default build, unfused).
- **The scaling (K5, K6).** `EnsureNormalizable` is private to `s2edge_crossings.cc` and
  computes `ldexp(2, -1 - ilogb(p_max)) * p`. Its comment states the postcondition the library
  adopts: the largest component in [1, 2). Go comments the function out. U-012 is K6, with
  `RobustCrossProd(−b, b)` for `b = (2⁻¹⁰⁷⁴, 0, 1)` as the input (C++ Release: `(NaN, ∞, NaN)`).
- **`Ortho`.** C++'s `S2::Ortho` and Go's `Ortho` agree with the library: the axis before the
  largest absolute component gets 1 in the perturbation vector `(0.012, 0.0053, 0.00457)`, and
  ties go to the later axis (`LargestAbsComponent`). `Ortho(-a) = -Ortho(a)` is stated in the
  header.
- **`point_cross` (K10).** Go's `PointCross` calls `p.Ortho()`, which resolves to `r3.Vector`'s
  axis `Ortho`, not `s2.Ortho`. The library calls the S2 one, as C++'s `RobustCrossProd(a, a)`
  does. The callers follow C++, so the library keeps its choice and documents the difference
  from Go (FR-022).
- **`Rotate`** checks unit length of both arguments in debug builds; the library documents it
  in prose. It stays documented, not checked, as `chord_angle` and `stable_angle` are.

## R2: Suite gating, names and records

**Decision.** The proofs join the `values` feature and 004's suite:

| | u-v rectangle | point |
|--|---------------|-------|
| Modules | `ur_crash`, `ur_val`, `ur_mem`, `ur_rel`, `ur_set`, `ur_mea`, `ur_exp`, `ur_eq` | `pt_crash`, `pt_val`, `pt_cls`, `pt_scl`, `pt_ord`, `pt_ari`, `pt_ort`, `pt_dst` |
| Selection | `--harness ur_` | `--harness pt_` |
| Catalogue ids | `P-QVAL`, `P-QMEM`, `P-QREL`, `P-QSET`, `P-QMEA`, `P-QEXP`, `P-QEQ` | `P-PTVAL`, `P-PTCLS`, `P-PTSCL`, `P-PTORD`, `P-PTARI`, `P-PTORT`, `P-PTDST` |
| `make` | `make verify-values P=uvrect` | `make verify-values P=point` |
| CI leg | `values-uvrect` | `values-point` |

- Neither prefix is a substring of an existing harness name (checked against every `p_`, `c_`
  and `a_` function and module in `verify/src/`). `uv_` was rejected because 003's
  `cd_uv::p_uv_*` contains it.
- `P-U…` is taken by `CellUnion` (002), hence `P-Q` ("quad") for the u-v rectangle. `P-PT`
  does not collide with `P-PV` (003) or `P-PAR`.
- Records: `verify/values/INVENTORY.md` §5 (`r2::Rect`) and §6 (`Point`);
  `verify/values/PROPERTIES.md` §6 and §7 (the order laws join §3, the generic laws);
  a spec-006 part of `verify/values/EVALUATION.md`; `PROPERTIES_BY_KIND.md`; `TRUSTED_BASE.md`;
  `MUTANTS.md`; `VERIFIED_TYPES.md`.

**Alternatives rejected.** A new cargo feature for the two types: 004 and 005 share `values`,
and a feature per pair multiplies the CI matrix's set-up for nothing.

## R3: Reference descriptions

**Decision.** Two new description modules in `verify/src/`, both reading public fields only
and never calling the library's methods (FR-007):

- **`uv_spec`**, the u-v rectangle: valid when `real_spec::empty(x) == real_spec::empty(y)`;
  membership, containment, interior containment, intersection and interior intersection are
  the products of `real_spec`'s; a vertex is a pair of bounds in upstream's counter-clockwise
  order (lower left, lower right, upper right, upper left).
- **`point_spec`**, the point:
  - **the order**, `lex_less(a, b)`, C++'s `lexicographical_compare` with `<`, restated from
    `util/math/vector.h`; `cmp_point` is `Less` when `lex_less(a, b)`, `Greater` when
    `lex_less(b, a)`, `Equal` otherwise;
  - **the largest absolute coordinate** `max_abs(p)`, and `normalizable(p)`: `max_abs(p) ≥
    2⁻²⁴²`;
  - **the exponent** `exponent(x)` of a non-zero finite `x` from IEEE-754's encoding (`ilogb`),
    and **`scaled_by(p, k)`**, `p` times `2^k` in two exact steps, each at most `2^537`;
  - **the restatement of upstream's arithmetic**, kept apart: `upstream_is_unit(p)`
    (`|‖p‖² − 1| ≤ 5ε`, the products summed in upstream's order) and `upstream_scaled(p)`
    (`EnsureNormalizable`'s value where it is finite). They are checked against the C++
    vectors (R9).

**Rationale.** The point has no set meaning to describe; its meaning is the IEEE triple and
upstream's comparisons and tests. Stating the order as C++'s algorithm makes the NaN behaviour
part of the definition rather than an exception to it.

**Alternatives rejected.** Describing the order as a total order on the reals: it is false for
NaN and for signed zeros as bit patterns, and would need the same exceptions spelled out.

## R4: Domains and generators

**Decision.** `verify/src/value_domain.rs` gains:

| Generator | Domain | Used by |
|-----------|--------|---------|
| `any_uv_rect` | both bounds of both axes any non-NaN `f64`, valid by `uv_spec` | every rectangle harness |
| `any_finite_uv_rect` | the same with finite bounds | centre, size, expansion, approximate equality (FR-004) |
| `any_uv_point` | two non-NaN `f64` | membership, adding, projection |
| `any_triple` | three arbitrary `f64`, NaN and infinities included | the order, the comparisons, `new`, the conversions, the accessors, `is_normalizable`, negation |
| `any_plain_triple` | no NaN | the order's laws, `is_normalizable`'s model |
| `any_finite_triple` | finite | addition, subtraction, `is_unit`, `ensure_normalizable` |
| `any_normable_triple` | finite, with a finite squared length (`point_spec::normable`; R0 finding 1) | `from_coords`, `normalize` |
| `any_box_point` | every coordinate in `[−1, 1]` | `ortho`, `reference_dir`, `point_cross`, `chord_angle`, the distances, `rotate` |

**The box.** It contains the unit sphere, so a proof over it covers the documented "unit
length" precondition and more, as FR-004 allows. It keeps every sum below 2 and every product
below 4, so the cross products cannot overflow into NaN. Assuming unit length itself would
make every such proof carry three symbolic products in its assumption, which the pilot
measured at half an hour. Where a law needs more than the box (`ortho` returns zero for a tiny
vector), the harness also assumes `normalizable`, and FR-022 documents the precondition.

**The arguments of the geometry operations that are not points**: `Angle` for `rotate` and
the approximate equalities ranges over finite radians.

## R5: How the u-v rectangle reuses 005

**Decision.** Exactly as `s2::Rect` reused 004 (005 R5): every rectangle property is stated
against `uv_spec`, which is built from `real_spec`, and Kani re-explores `r1::Interval`'s code
inside each rectangle proof. The trusted base records, per rectangle property, the real-interval
properties whose statements it composes (for example P-QSET-01, union, composes P-RSET-01).

**Rationale.** 005 R0 measured the product statement at 0.1–62 s and the component-wise
statement at a 12-minute timeout. The u-v rectangle is the simpler product (two copies of the
same interval, no seam), so the same choice applies with more margin.

**What SC-014 measures.** The share of `P-Q` entries whose statements are products of `P-R`
statements, and the time saved: the sum of the reused `P-R` proofs' times against the `P-Q`
proofs' times (005's method).

## R6: The FR-019 fix (deserialising an `r2::Rect`)

**Decision.** A hand-written `Deserialize` for `r2::Rect`, as 005 FR-020 wrote for `s2::Rect`
(`core/src/s2/rect.rs`): a private shadow struct with the same field names derives
`Deserialize`, and the conversion rejects a rectangle unless both intervals are empty or
neither is, and no bound is NaN. The errors name the rule ("empty", "NaN"). `Serialize` stays
derived, so the format is unchanged.

- **NaN.** The library's `is_valid` accepts a NaN bound (`[NaN, 1]` is not empty), but FR-005's
  invariant excludes it, and a NaN bound contains nothing (K4). JSON cannot carry NaN, but
  other serde formats can, so the check is not dead code.
- **No tolerance.** The bounds are copied, not computed, so a valid rectangle round-trips bit
  for bit; 005 R6 measured a million honest JSON round trips of `s2::Rect` with the same
  pattern.
- Tests go in a new sibling `core/src/r2/rect_tests.rs` (Constitution III).

**Alternatives rejected.** Reusing `is_valid` alone: it would let NaN bounds through, which
the proofs' invariant excludes.

## R7: Documentation (FR-017, FR-018, FR-022)

**Decision.** Rustdoc only, each item naming upstream's matching behaviour:

- `r2::Rect::new`, `from_points`: `# Panics` (debug builds) and what release builds return;
  `project`: `# Panics` on the empty rectangle (every build, through `r1::Interval::project`);
  `from_center_size`: the precondition (non-negative size), with a new debug-build check
  (FR-017, the only behaviour change, debug builds only);
- `r2::Rect::center`, `size`, `expanded`, `expanded_by_margin`, `approx_eq_with`: the infinite
  corners (FR-018);
- `Point::from_coords`, `normalize`: the range where the result is unit length, and the zero
  vector outside it (K7);
- `Point::cmp_point` and `PartialOrd`: NaN (K8); `is_normalizable`: NaN;
- `Point::point_cross`, `ortho`, the distances, `rotate`: the box or unit-length precondition
  each needs, and for `point_cross` the deviation from Go (K10) and the true exception to its
  sign laws, a cross product that rounds to zero (K11);
- `Point::ensure_normalizable`: its domain and postcondition, with FR-020.

## R8: The FR-020 and FR-021 fix (one scaling)

**Decision.**

- `Point::ensure_normalizable` computes `e = −ilogb(p_max)` with the existing `ilogb` helper,
  which already handles subnormals, and multiplies by `2^(e/2)` and then `2^(e − e/2)`. Each
  factor is at most `2^537`, so it is a normal number, and each product is exact. R0's
  `r0_pt_scale_draft` proves the draft. Whenever upstream's single `ldexp` is finite it is
  exact too, so the two results agree bit for bit.
- `edge_crossings.rs` drops its private `is_normalizable` and `ensure_normalizable` and calls
  `Point`'s. That restores parity where the copy scaled into [0.5, 1), and removes the copy's
  `log2().floor()`, which can misjudge the exponent near a power of two.
- Two commits: FR-020 (the defect fix, with `core/src/s2/point_tests.rs`), then FR-021 (the
  shared scaling, with `core/src/s2/edge_crossings_tests.rs`), each with a test that fails
  without it. The FR-021 tests pin both the restored parity (`b = (2⁻³⁰⁰, 0, 1)` gives
  `(−0, 1, 0)`) and the deviation (`b = (2⁻¹⁰⁷⁴, 0, 1)` gives a normalizable, finite vector).
- `robust_cross_prod`'s rustdoc gains a "Deviation from upstream" section for U-012, as
  `s1::Interval::expanded` has for U-011.

**Alternatives rejected.**

- Scaling with one multiplication by `2^e`: it overflows for `e > 1023`, which is exactly the
  K6 case.
- Pre-scaling subnormal inputs by `2^54` before taking the exponent: correct, but it adds a
  branch the proofs must cover, for no benefit over two factors.

## R9: C++ reference vectors and differential tests

**Decision.** Four sections appended to `tools/refgen-cpp/generate_values.cc`, so every
earlier line of `core/tests/data/cpp_values_vectors.csv` stays byte-identical:

- **`R2RECT_OPS`**: random rectangles, points and margins: every in-scope operation with a C++
  counterpart.
- **`R2RECT_SPECIAL`**: empty, infinite, signed-zero and single-point sides; every cell's u-v
  bound at levels 0–3.
- **`S2POINT_OPS`**: random unit points and pairs: `operator<`, `==`, `IsUnitLength`,
  `Normalize`, `Ortho`, `RobustCrossProd`, `S1Angle(a, b)`, `S1ChordAngle(a, b)`, `Rotate`.
- **`S2POINT_SPECIAL`**: the zero vector, signed zeros, subnormal, tiny and huge coordinates,
  ties in the largest component, equal and antipodal pairs, NaN and the infinities, and the
  robust cross product of `(2⁻ᵏ, 0, 1)` and its antipode for k = 300, 1023, 1024, 1074. Rows
  where C++ returns a non-finite cross product carry `deviation:robust-cross-overflow`.

`core/tests/cpp_values_diff.rs` checks the library against them; `verify/tests/
values_spec_vs_cpp.rs` checks `uv_spec` and `point_spec` (the restatements included) against
them. `point_cross` has no C++ counterpart: its sign laws are proven (`P-PTORT`), and K10's
regression test pins its degenerate branch.

## R10: Non-vacuity

**Decision.** Mutants `M-501` onward in `verify/mutants/`, in 004's patch format: at least
one per proven property, a few against `uv_spec` and `point_spec`, and the FR-020 fix's
removal (expected to be killed by `P-PTSCL-01`). `run-mutants.sh` is unchanged.

## R11: Trusted base additions

- 005's real-interval results that the u-v rectangle composes (R5);
- the checker's IEEE-754 model, `sqrt` included, and the trigonometric functions only if R0
  finds them modelled;
- **the build**: the proofs cover the default build, whose dot and cross products are
  unfused; a build with fused multiply-add computes other values (K9), checked by no proof;
- `point_spec`'s restatements (`upstream_is_unit`, `upstream_scaled`) and their check against
  the C++ vectors;
- the box `[−1, 1]³` as the domain of the geometric operations (R4).

## R12: Tiers and budget

**Decision.** Per change: everything R0 measures under 6 minutes, in two legs (`values-uvrect`,
`values-point`), each under SC-005's 10 minutes on 4 CPUs. Weekly: anything between 6 and 60
minutes, as 005 decided (`slow` feature, `make verify-slow F=values P=…`).

From R0's measurements:

- **The u-v rectangle:** every property per change (at most 28.2 s in R0).
- **The point, weekly:** P-PTCLS-02 (`is_unit`'s model, 644.5 s), and P-PTCLS-03 (the
  unit-length results, not probed; the pilot took 1802 s for the classifications). If
  P-PTCLS-03 times out at 60 minutes, it is proven within bounds (for example, coordinates in
  `[2⁻⁵¹¹, 1]`).
- **The point, per change:** everything else (at most 358.0 s in R0). `ortho`'s non-zero
  result (P-PTORT-02, 358.0 s) is the closest to the limit, and the first to move if the
  first run puts it over 6 minutes.

The point's per-change harnesses took about 1,030 s together in R0 (the failing ones
included). On 4 jobs that is about 6 minutes, set by the longest; the u-v rectangle's leg
needs about 1 minute.

## R13: The evaluation (FR-016, FR-025, SC-007, SC-014)

A spec-006 part of `verify/values/EVALUATION.md`, as 005's:

- the effort and run times;
- what reusing `real_spec` saved (R5, SC-014);
- how far the checker reached into the point's geometry, and which properties need certified
  error bounds instead;
- the assessment of 004–006 against 001–003;
- **the go or no-go for `ChordAngle`, with the A/B/C validity question it waits on**, and
  whether type-level invariants should come first;
- the ranked follow-ups.

## R14: Lessons carried over

- Booleans compared only with `==` (004 R13); the order's harnesses compare `Ordering`
  values, not booleans, with `<`.
- Weekly members are run, not only written: U-011 came from one.
- Every run through `run-verify.sh` or a capped `systemd-run`, one job at a time.
- Probe evidence is kept in the repository (`probe/`), not only in the scratchpad.
- A new `__` split family name needs the module-level `#![allow(non_snake_case, reason = …)]`
  that 005's `rl_exp` and `rl_val` carry.
- A NaN-check failure is replayed under IEEE-754 before it narrows a domain: CBMC reports a
  division by an infinity as NaN (R0 finding 1).
- A bit-for-bit law over computed results needs the zeros' signs considered; `same` is the
  comparison when the law is about values (R0 finding 2).
- A weekly member's domain is checked against the branches of the code under it: spec 005's
  `p_lexp_01__mneg` assumed a non-positive longitude margin, but a zero margin takes
  upstream's growth branch (2026-10-14).
