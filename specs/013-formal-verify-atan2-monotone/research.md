# Research: A Monotone `atan2`, and the Bounding Laws It Opens

Spec: [spec.md](spec.md). Survey: `probe/README.md` (K1–K4). Planning probe: `probe/r0_harnesses.rs`,
included in the verification crate through a temporary `#[path]` module.

## R0: Planning probe (run 2026-11-01)

Kani 0.68.0, `run-verify.sh`, 12-minute limit, 2–3 jobs. Contracts as R1 defines them.

| Harness | What | Result |
|---------|------|--------|
| `r0_creg02_dir_l0` | P-CREG-02, level 0, as directions | SUCCESSFUL, 103.9 s |
| `r0_creg02_dir` | levels ≥ 1, every face, latitude and longitude | TIMEOUT |
| `r0_creg02__f{0,2}_lat` | one face, latitude (the library's `x·x + y·y`) | TIMEOUT |
| `r0_creg02__f{0,2}_lng` | one face, longitude | SUCCESSFUL, 526.5 s, 691.6 s |
| `r0_creg02l__f{0,2}` | one face, latitude through `latitude_lemma` (R2) | SUCCESSFUL, 205.0 s, 194.9 s |
| `r0_creg02c__all_lat` | every face, latitude through the lemma | SUCCESSFUL, 260.6 s |
| `r0_creg02c__f{0,2}_lng` | one face, longitude, latitude stubbed by the lemma | SUCCESSFUL, 341.4 s, 374.5 s |
| `r0_creg02c__all_lng` | every face, longitude | TIMEOUT |
| `r0_square_monotone`, `r0_sumsq_monotone` | R2's lemma, in the checker | TIMEOUT (both) |
| `r0_uv_quadrant` | R3's premise on the real projection | TIMEOUT |
| `r0_creg02_norm` | P-CREG-02 for the normalised point | FAILED, 358.8 s (playback out of memory) |
| `r0_lreg03` | P-LREG-03 under the order contracts and pure `sin`/`cos`/`sin_cos`/`asin` | FAILED, 400.1 s |
| `r0_ptdst02` | P-PTDST-02, pure `atan2` | FAILED, 541.0 s; with pure `sqrt` too: TIMEOUT |

**Decision:** P-CREG-02 is provable for the cell's points as directions, under the order
contracts and the sum-of-squares lemma: level 0 (104 s) and latitude on every face (261 s) per
change, longitude one harness per face (about 6 minutes each) weekly. The normalised form,
P-LREG-03 and P-PTDST-02 are recorded with sharpened reasons (R5, R6, R7).

## R1: The order contract for `atan2`

**Decision:** `stubs::atan2_order(y, x)`: spec 008's range contract (`atan2_contract`), plus:

- **one value per argument pair** across a proof's calls (a log of ten calls, as `Table`);
- **the axes exactly** (C11 F.10.1.4): `atan2(±0, x)` is `±0` for a clear `x` sign bit and `±π`
  for a set one; `atan2(y, ±0)` is `±π/2` for `y ≠ 0`;
- **the left half-plane:** `|r| ≥ π/2` when `x`'s sign bit is set;
- **dominance within a closed quadrant** (quadrant by sign bits): in the first quadrant
  `y₁ ≤ y₂ ∧ x₁ ≥ x₂ ⇒ r₁ ≤ r₂`; the second (`x` negative) `y₁ ≥ y₂ ∧ x₁ ≥ x₂ ⇒ r₁ ≤ r₂`; the
  fourth (`y` negative) `y₁ ≤ y₂ ∧ x₁ ≤ x₂ ⇒ r₁ ≤ r₂`; the third `y₁ ≥ y₂ ∧ x₁ ≤ x₂ ⇒ r₁ ≤ r₂`;
  against every earlier call and six **anchors**, virtual calls with exact values:
  `(±1, 1) → ±FRAC_PI_4`, `(±1, −1) → ±3·FRAC_PI_4`, `(±1, √2) → ±POLE_MIN_LAT`.

**Rationale:** Dominance implies the exact angle order, so a monotone `atan2` satisfies it; it
uses only float comparisons, cheap in the checker, and it is all the proofs need: a point of a
cell (level ≥ 1) and the cell's corners share a quadrant of their face (R3), where every
coordinate the angles depend on moves the angle one way. The anchors are the values the face
cells' fixed bounds use (`3·FRAC_PI_4` is `atan2(1, −1)` bit for bit, `POLE_MIN_LAT` is
`atan2(1, √2)`; checked natively).

**Evidence:** K1 (20,000,000 near-tie pairs, 8,335,328 dominance pairs, no counterexample).
FR-002's test (data-model §3) runs ten million pairs per quadrant rule, with signed zeros,
subnormals and axes, and the anchors exactly.

**Alternatives considered:** an exact-order contract (cross-product sign in big integers):
needs 106-bit products and exponent alignment beyond `u128` in the checker; dominance suffices.
An accuracy contract (`|r − θ| ≤ 1 ulp`): would also serve the normalised point, but the checker
cannot express `θ`. Order within one ulp: weaker than needed, since the latitude bound compares
corner and point without slack under the contract.

## R2: `sqrt`, and the library's sum of squares

**Decision:** `stubs::sqrt_order(a)`: spec 010's `sqrt_contract`, one value per argument, results
ordered as their arguments, and `sqrt(2) = √2` bounds (`a ≤ 2 ⇒ r ≤ SQRT_2`, and the mirror).
`LatLng::latitude` is stubbed by `stubs::latitude_lemma`, the library's text with `x·x + y·y`
replaced by `stubs::sumsq(x, y)`: one value per pair, in [0, 2], monotone in `|x|` and `|y|`,
exact at `(0, 0)`, `(1, 0)`, `(0, 1)`, `(1, 1)`. A drift guard pins `LatLng::latitude`'s text.

**Rationale:** IEEE-754 makes `×`, `+` and `sqrt` correctly rounded, and rounding to nearest is
monotone, so `|x₁| ≤ |x₂| ∧ |y₁| ≤ |y₂| ⇒ x₁⊗x₁ ⊕ y₁⊗y₁ ≤ x₂⊗x₂ ⊕ y₂⊗y₂`, and `sqrt` keeps the
order. These are facts of the standard, not of the platform; but the checker cannot prove the
squares' monotonicity (R0: `a·a ≤ b·b` for `|a| ≤ |b|` times out at 12 minutes), so the lemma is
trusted by that argument and checked natively on adversarial pairs (near ties, subnormals,
exact squares). Without the lemma, latitude times out per face.

**Alternatives considered:** splitting by exponent ranges (the squares' cost is the mantissa
product, not the range); proving the lemma weekly at 60 minutes (kept as an attempt in tasks:
if it proves, the lemma moves from trusted to proven).

## R3: The quadrant premise

**Decision:** Stub `ij_level_to_bound_uv` with `stubs::quadrant_uv_bound`: spec 012's
`any_uv_bound` (`−1 ≤ lo ≤ hi ≤ 1`), plus below level 0 neither interval crosses 0 and no bound
is `−0`, and at level 0 the face `[−1, 1]²`. Justify it by exhaustion: `uv_projection.rs` already
walks every edge value `st_to_uv(ij_to_st_min(k))`, `k ∈ [0, 2³⁰]`; it gains the assertions that
the value is negative for `k < 2²⁹`, `+0` at `k = 2²⁹` and positive above. A cell below level 0
has `size ≤ 2²⁹` and an aligned `x_lo`, so its two edges `x_lo` and `x_lo + size` lie on one side
of `2²⁹` (the integer fact, written in the stub's documentation).

**Rationale:** The checker times out on the real projection over a symbolic `(i, j)` (R0); the
edge values are finite in number.

## R4: P-CREG-02's statement and harnesses

**Decision:** For every valid cell and every `(u, v)` in its u-v bound, `rect_bound()` contains
`LatLng::from_point(face_uv_to_xyz(face, u, v))`. Harnesses (`cr_reg.rs`):
`p_creg_02__l0` (level 0, per change), `p_creg_02__lat` (levels ≥ 1, every face, per change),
`p_creg_02__lng_f<0..5>` (weekly). Covers: a leaf cell, a cell with a pole corner, a point
strictly inside.

**Rationale:** R0's times; the per-change part (two harnesses) is about 6 minutes of solver time.

## R5: The normalised point

**Decision:** Not proven. Normalising rounds each coordinate separately, so the point's
arguments no longer dominate the corner's; only an accuracy contract would bound the angle's
shift by the bound's `2ε` expansion. Evidence: K3 (5,200,000 normalised points, none outside),
the twin, the C++ rows. Recorded in the P-CREG-02 row.

## R6: P-LREG-03 (and so P-ADREG-04)

**Decision:** Not proven; the reason is sharpened. The cap's vertices come from `sin_cos` of the
rectangle's bounds, the point's latitude and longitude from `atan2` of its coordinates, and the
cap compares chord distances between them. Under the order contracts and pure `sin`, `cos`,
`sin_cos` and `asin` the checker finds a counterexample in 400 s: nothing relates `sin_cos(θ)` to
the arguments whose `atan2` is `θ`. A proof needs an accuracy contract linking them
(`sin(atan2(y, x)) ≈ y/|(x, y)|`, within ulps) and the vertex-dominance geometry of the cap, a
research item of its own. P-ADREG-04 keeps its reason.

## R7: P-PTDST-02

**Decision:** Stays a tool limit with the new measurement: with a pure `atan2` only, the checker
reports a difference (541 s) because `norm` takes `sqrt` twice and CBMC's `sqrt` is not a
function; with pure `atan2` and `sqrt` it times out at 12 minutes, the recomputed products of
spec 006's P-PTDST-01 symmetry.

## R8: Composition

**Decision:** `CellUnion`'s P-UREG-05 is re-stated by composition for the points of its cells'
own u-v bounds: P-UBND-01 (the union's rectangle contains each cell's, spec 011) and P-CREG-02.
The margin (U-017) stays excepted. P-UREG-04 waits on P-CREG-01.

## R9: Names, records, CI

- **Module** `verify/src/cr_reg.rs` (values feature); `make verify-values P=cellrect` selects it.
  The contracts and the lemma join `stubs.rs` (spec 013 section), cleared by `stubs::reset`.
- **Mutants** `M-988` onwards (data-model §6).
- **CI:** `values-cellrect` per change; a `values-cellrect` weekly leg with the six longitude
  harnesses; path filters gain `core/src/s2/latlng.rs` if missing.
- **Records:** `PROPERTIES.md` §1 (P-CREG-02, new lemma rows L-CR-01, -02), §7's P-PTDST-02,
  §9's P-LREG-03; `cell_union/PROPERTIES.md` P-UREG-05; §13's P-ADREG-04; the index by kind;
  `TRUSTED_BASE.md` (the order contracts, the lemma, the premise); `MUTANTS.md`,
  `VERIFIED_TYPES.md` §2.14, `CORE_TRAITS.md`; the evaluation's spec-013 part. No inventory
  change (no new operation). `COVERAGE.md` unaffected (crash freedom unchanged).

## R10: Measured in implementation (2026-11-01)

- `P=cellrect`: `p_creg_02__l0` 109.2 s, `__lat` 259.2 s (covers 2/2, 3/3), 314 s wall with the
  crate tests and build. Weekly: `__lng_f0` … `__lng_f5` 362.6, 205.1, 326.2, 191.4, 213.8,
  349.1 s, 893 s with 2 jobs. Through the latitude lemma the longitude members ran faster than
  R0's (341–375 s for faces 0 and 2).
- The native tests: the order contract on ten million pairs per quadrant rule and the lemma on
  ten million pairs, 5 s in release; the quadrant premise over the 2³⁰ + 1 edge values, 5 s.
- Mutants `M-988` … `M-992`: all killed. M-990 was retargeted from face 0 to face 2 before the
  run: on the equatorial faces the longitude does not depend on v, so the mutant is equivalent
  there.
