# Contract: Harness and Artifact Conventions (changes for `PointRegion` and `CellUnion`'s floating-point methods)

004's to 010's contracts still apply. This file lists only what this feature adds.

## Modules and names

| Module (`verify/src/`) | Content | Gate |
|------------------------|---------|------|
| `region_spec.rs`, `region_spec_tests.rs` | The description: a point region is its point, a union is the points whose leaf it contains; the drift guards of the bounds' constructions (data-model §1) | none |
| `stubs.rs` (extended) | `pure_leaf` (`CellId::from_point`), `pure_cell_cap` (`Cell::cap_bound`), `any_center` (`CellId::to_point`), `pure_cell_area` (`Cell::approx_area`, `exact_area`) (data-model §3) | `cfg(kani)` |
| `laws.rs` (extended) | `region::rect_bound_contains_points`, `region::covering_contains_points` (FR-010) | `cfg(kani)` |
| `pr_crash.rs`; `pr_con.rs`, `pr_bnd.rs`, `pr_cel.rs`, `pr_eq.rs`, `pr_reg.rs` | `PointRegion` harnesses: `pr_<area>::p_pr<area>_<nn>`, crash `pr_crash::c_<op>`, the laws `pr_reg::p_prreg_<nn>` | `cfg(all(kani, feature = "values"))` |
| `uf_crash.rs`; `uf_pt.rs`, `uf_bnd.rs`, `uf_area.rs`, `uf_exp.rs`, `uf_reg.rs` | `CellUnion`'s floating-point harnesses: `uf_<area>::p_u<area>_<nn>__n<k>` (the union length as 002's `__n<k>`), crash `uf_crash::c_<op>__n<k>` | `cfg(all(kani, feature = "values", feature = "cellunion"))` |
| `verify/tests/libm_contracts.rs` (extended) | the `CellId::from_point` and `Cell::cap_bound` contracts on real points and cells | `cargo test` |

- **Catalogue ids:** `P-PRCON`, `P-PRBND`, `P-PRCEL`, `P-PREQ`, `P-PRREG`, `P-PRDES` for
  `PointRegion`; `P-UPT`, `P-UBND`, `P-UAREA`, `P-UEXP`, `P-UREG` for `CellUnion`'s new entries
  (none of them is taken: 002 uses `P-UACC`, `P-UDEN`, `P-UENC`, `P-UINV`, `P-UMEAS`, `P-UMEM`,
  `P-UNORM`, `P-URANGE`, `P-USET`, `P-UV`); lemmas `L-PR-NN`.
- **Inventory names:** `s2::point_region::PointRegion::<method>`, `Region for PointRegion`;
  `CellUnion`'s rows keep 002's numbers (12, 25–28, 35–37, 40) and gain a crash harness or an
  inherited limit.
- **Union lengths** are concrete (`__n<k>`), as 002 requires: a symbolic length costs two to
  three orders of magnitude.

## Stubs

008's and 010's rules, and:

- **A contract on a library function** (`pure_leaf`: any valid leaf; `pure_cell_cap`: any
  non-empty cap with a unit centre) is tested on the real function in `libm_contracts.rs` and
  named in `TRUSTED_BASE.md`, as 010's `cell_rect_bound_contract`.
- **Uninterpreted stubs** (`any_center`, `pure_cell_area`, 010's `any_cell_bound`) allow every
  value the real function can return; a statement proven with them holds for the real one.
- **The status follows the stubs**, as in 008 and 010.
- **Unwinding:** a harness that builds a `Cell` needs 9 (`to_face_ij_orientation`); one that
  uses a stub table needs 11 (ten slots).

## Statements

- **Against the description.** The union's point test against `union_spec::contains_id` of the
  point's leaf; the bounds by containment of each cell's bound (the meaning), not by
  recomputing the centroid's products (008's and 010's lesson); the constructions' text by drift
  guards on the source.
- **Booleans** with `==` and `!=` only; points compared bit for bit through `to_bits`.
- **The generic laws** by name: `laws::region::contains_implies_intersects`,
  `covering_covers`, `no_intersect_no_vertex`, `cap_bound_contains_points` and the two new
  laws; `laws::eq::all` for `PointRegion`.

## Mutant patches

004's format, numbered `M-950` onwards (series 9, research R2). At least two patch
`region_spec.rs` or `stubs.rs`. FR-017 to FR-019's fixes get a reverting mutant each, killed by
their `cargo test` targets (`Test:` headers).

## Records

Section 13 of `verify/values/INVENTORY.md` and section 12 of `verify/values/PROPERTIES.md` for
`PointRegion`; `CellUnion`'s nine rows in `verify/cell_union/INVENTORY.md` and new entries in
`verify/cell_union/PROPERTIES.md`; a spec-011 part of `verify/values/EVALUATION.md`;
`PROPERTIES_BY_KIND.md` and its HTML render, `TRUSTED_BASE.md`, `MUTANTS.md`,
`VERIFIED_TYPES.md`, `CORE_TRAITS.md` and `verify/COVERAGE.md`, in the same change as the
harnesses (constitution v1.2.0).
