# Data Model: A Monotone `atan2`, and the Bounding Laws It Opens

## 1. The contracts (`verify/src/stubs.rs`, spec 013 section)

| Stub | Replaces | Rules | Justified by |
|------|----------|-------|--------------|
| `atan2_order` | `f64::atan2` | research R1: range and sign; one value per pair; the axes; `|r| ≥ π/2` for negative `x`; quadrant dominance against earlier calls and six anchors | the platform (glibc, x86-64), tested: K1 and `libm_contracts::atan2_keeps_quadrant_order` |
| `sqrt_order` | `f64::sqrt` | R2: `sqrt_contract`; one value per argument; monotone; `√2` bounds | IEEE-754 (correctly rounded); tested |
| `sumsq` (inside `latitude_lemma`) | the library's `x·x + y·y` | R2: one value per pair; [0, 2]; monotone in `|x|`, `|y|`; anchors | IEEE-754 (monotone rounding); tested (`sum_of_squares_is_monotone`) |
| `latitude_lemma` | `LatLng::latitude` | the library's text with `sumsq` | drift guard `latitude_is_the_librarys` |
| `quadrant_uv_bound` | `ij_level_to_bound_uv` | R3: `any_uv_bound` plus no axis crossing below level 0, no `−0`; the face at level 0 | `uv_projection::projection_at_every_boundary_value` (exhaustive) and the integer fact |

Each keeps a log of at most ten calls (`SLOTS`); an eleventh call gets the base contract's value,
unrelated to the others (more behaviours, never a false proof). `stubs::reset` clears the logs.

## 2. The catalogue

| ID | Statement | Harness | Status |
|----|-----------|---------|--------|
| P-CREG-02 | For every valid cell and `(u, v)` in its u-v bound, `rect_bound()` contains `LatLng::from_point(face_uv_to_xyz(face, u, v))`; the normalised point not proven (R5) | `cr_reg::p_creg_02__l0`, `__lat`, `__lng_f<0..5>` (weekly) | proven-under-contract |
| L-CR-01 | `|x₁| ≤ |x₂| ∧ |y₁| ≤ |y₂| ⇒ x₁x₁ + y₁y₁ ≤ x₂x₂ + y₂y₂` (rounded), on [−1, 1] | — (the checker times out) | trusted (IEEE-754), tested |
| L-CR-02 | Below level 0 a cell's u-v intervals do not cross 0; no bound is `−0` | — | proven by exhaustion (`uv_projection.rs`) and the integer fact |
| P-UREG-05 | `CellUnion`'s rectangle holds every point of its cells' own u-v bounds | — | proven-by-composition (P-UBND-01, P-CREG-02) |
| P-LREG-03 | unchanged statement | — | not-proven, reason sharpened (R6) |
| P-ADREG-04 | unchanged | — | not-proven (P-LREG-03) |
| P-PTDST-02 | unchanged | — | not-proven (tool limit), new measurement (R7) |

## 3. Native tests (`verify/tests/`)

- `libm_contracts.rs`: `atan2_keeps_quadrant_order` (each quadrant rule on ten million pairs:
  near ties within four ulps, one coordinate shared, signed zeros, subnormals; release, `#[ignore]`
  if the debug run is slow, as `angle_enumeration`); `atan2_anchors_are_exact`;
  `sqrt_is_monotone`; `sum_of_squares_is_monotone` (ten million adversarial pairs).
- `uv_projection.rs`: the sign assertions of R3.
- `verify/src/region_spec_tests.rs` (or a sibling): `latitude_is_the_librarys`.
- `core/tests/trait_property_tests.rs`: the twin `region_cell_rect_contains_directions` (random
  cells at every level and face, corners, edges, interiors, as directions).

## 4. Mutants (`M-988` …)

| Mutant | Patch (library) | Killed by |
|--------|-----------------|-----------|
| M-988 | `Cell::rect_bound`: the u-axis corner choice inverted (`i`) | `p_creg_02__lat` |
| M-989 | the latitude's second corner `(1 − i, 1 − j)` → `(1 − i, j)` | `p_creg_02__lat` |
| M-990 | the longitude's corner `(i, 1 − j)` → `(i, j)` | `p_creg_02__lng_f*` |
| M-991 | `polar_closure` dropped | `p_creg_02__lng_f2` |
| M-992 | face 1's fixed longitude bound starts at `π/2` | `p_creg_02__l0` |

The `2ε` expansion has no mutant here: under the order contract the corners bound the point
exactly, so the expansion serves rounding the contract does not model (the normalised point);
the C++ rows check it bit for bit.

## 5. Library changes

None (FR-012).
