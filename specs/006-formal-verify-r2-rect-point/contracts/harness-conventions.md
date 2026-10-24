# Contract: Harness and Artifact Conventions (changes for `r2::Rect` and `Point`)

004's and 005's contracts still apply. This file lists only what this feature adds.

## Modules and names

| Module (`verify/src/`) | Content | Gate |
|------------------------|---------|------|
| `uv_spec.rs`, `uv_spec_tests.rs` | The u-v rectangle as a product of two `real_spec`s (data-model §1.1) and its unit tests | none |
| `point_spec.rs`, `point_spec_tests.rs` | The point's order, classifications and scaling, and the restatements of upstream's arithmetic (§1.2), with unit tests | none |
| `laws.rs` (extended) | `laws::ord`: the generic order laws | `cfg(kani)` |
| `value_domain.rs` (extended) | `any_uv_bounds`, `any_uv_rect`, `any_finite_uv_rect`, `any_uv_point`, `any_triple`, `any_plain_triple`, `any_finite_triple`, `any_bounded_triple`, `any_normable_band_triple`, `any_box_point`, `any_normalizable_box_point`, `any_finite_angle` | `cfg(all(kani, feature = "values"))` |
| `ur_crash.rs`; `ur_val.rs`, `ur_mem.rs`, `ur_rel.rs`, `ur_set.rs`, `ur_mea.rs`, `ur_exp.rs`, `ur_eq.rs` | `r2::Rect` harnesses: `ur_<area>::p_q<area>_<nn>` | same |
| `pt_crash.rs`; `pt_val.rs`, `pt_cls.rs`, `pt_scl.rs`, `pt_ord.rs`, `pt_ari.rs`, `pt_ort.rs`, `pt_dst.rs` | `Point` harnesses: `pt_<area>::p_pt<area>_<nn>` | same |

- **Selection:** `--harness ur_` and `--harness pt_`. Neither is a substring of an existing
  harness or module name (research R2). The reverse was not checked: Kani matches a filter
  anywhere in a name, so the older prefixes matched the new names (`ri_` in
  `pt_ari::p_ptari_01`, `cl_` in `pt_scl`; `iv_` had matched `ri_val::p_riv_01` since spec 005).
  Found in the final run (T052): `make verify-values P=<part>` now passes each of the part's
  modules as `--harness <module>::`.
- **Catalogue ids:** `P-QVAL`, `P-QMEM`, `P-QREL`, `P-QSET`, `P-QMEA`, `P-QEXP`, `P-QEQ`;
  `P-PTVAL`, `P-PTCLS`, `P-PTSCL`, `P-PTORD`, `P-PTARI`, `P-PTORT`, `P-PTDST`.
- **Inventory names:** `r2::Rect::<method>`, `s2::Point::<method>`, `s2::ortho`, `s2::rotate`,
  and the trait implementations by name.
- **Crash ids:** `C-r2::Rect::<method>`, `C-s2::Point::<method>`.

## Statements

- **Against the product, never component-wise.** A u-v rectangle property compares the library
  with `uv_spec`, as 005's rectangle does with `rect_spec`.
- **The order is upstream's algorithm.** P-PTORD-01 compares `cmp_point` with
  `point_spec::order` for every triple, NaN included; the laws of P-PTORD-02 assume no NaN.
- **Bit for bit means bit patterns.** Equality of results is `to_bits` equality, so signed
  zeros and NaN payloads count; the IEEE `==` appears only where the property is about `==`.
  The orthogonal directions' laws (P-PTORT-01, P-PTORT-03) are about values and compare with
  `point_spec::same`: their results' zeros carry signs the laws do not fix (research R0).
- **NaN is a failed check.** A harness of an arithmetic operation assumes the domain where no
  operation produces NaN (`FiniteTriple`, `BoxPoint`, finite rectangles), exactly as FR-004
  and FR-022 document; it never filters NaN out after the fact. CBMC also reports a division
  by an infinity as NaN, though IEEE-754 gives zero; so `from_coords` and `normalize` assume
  `NormableTriple` (per change its part `BoundedTriple`, weekly the band), and the rest is a
  tool limit with ordinary tests (research R0, findings 1 and 5).
- **Never compare two square roots.** The checker's `sqrt` constrains its result without fixing
  it, so two normalisations of one vector may differ in a proof (research R0, finding 4). A
  harness states properties of one result; comparisons of two belong to the C++ vectors.
- **Booleans** are compared only with `==` and `!=`; orders compare `Ordering` values.
- **The generic laws** are 004's `laws::eq` (P-QEQ-01, P-PTVAL-03) and the new `laws::ord`
  (P-PTORD-02).
- **Trigonometry.** `sin` and `cos` are modelled (over-approximated in [−1, 1]), so `rotate`'s
  crash harness is written. No harness may reach `atan2`, which Kani replaces with a failing
  check (research R0); the five operations through it are tool-limit rows in the inventory,
  and P-PTDST-01 reaches only `angle`'s early return for equal points.

## Split families

- `__mpos`, `__mneg` (a margin's sign) for the u-v rectangle's expansion containment.
- One member per clause where a statement's clauses together time out (`__result`,
  `__reflexive`, `__symmetric`, `__product`, `__swap`, `__neg_p`, …), and per domain part for
  `from_coords` and `normalize` (`__bounded`, `__band`).
- A module with a `__` family carries
  `#![allow(non_snake_case, reason = "`__` marks a split-family member (harness-conventions)")]`.

## Mutant patches

004's format, numbered `M-5NN`. Three patch the descriptions (`verify/src/uv_spec.rs`,
`verify/src/point_spec.rs`).

## Records

Sections 5 (`r2::Rect`) and 6 (`Point`) of `verify/values/INVENTORY.md`, sections 6 and 7 of
`verify/values/PROPERTIES.md` (the order laws in its section 3), and a spec-006 part of
`verify/values/EVALUATION.md`. `PROPERTIES_BY_KIND.md` lists every new entry;
`property_kinds.rs` needs no change.
