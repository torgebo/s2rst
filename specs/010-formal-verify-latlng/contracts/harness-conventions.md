# Contract: Harness and Artifact Conventions (changes for `LatLng` and `Rect` as a `Region`)

004's to 008's contracts still apply. This file lists only what this feature adds.

## Modules and names

| Module (`verify/src/`) | Content | Gate |
|------------------------|---------|------|
| `latlng_spec.rs`, `latlng_spec_tests.rs` | The description, the restatements and the reduction's copy (data-model §1), with unit tests and the drift guard | none |
| `stubs.rs` (extended) | The pure contracts, `sqrt_contract`, `pure_from_point`, `any_cell_bound`, `cell_rect_bound_contract` (data-model §3) | `cfg(kani)` |
| `value_domain.rs` (extended) | `any_lat_lng_bits`, `any_finite_lat_lng` | `cfg(all(kani, feature = "values"))` |
| `ll_crash.rs`; `ll_con.rs`, `ll_fp.rs`, `ll_tp.rs`, `ll_nrm.rs`, `ll_val.rs`, `ll_dst.rs`, `ll_apx.rs`, `ll_ari.rs`, `ll_eq.rs`, `ll_lem.rs` | `LatLng` harnesses: `ll_<area>::p_ll<area>_<nn>`, crash `ll_crash::c_<op>`, the lemma `ll_lem::l_ll_01` | same as `value_domain` |
| `ll_reg.rs` | `Rect`'s law instances `p_lreg_<nn>` and the composition `p_lmem_03` | same |
| `verify/tests/libm_contracts.rs` (extended) | `sqrt_contract`, and the `Cell::rect_bound` contract on real cells | `cargo test` |

- **Catalogue ids:** `P-LLCON`, `P-LLFP`, `P-LLTP`, `P-LLNRM`, `P-LLVAL`, `P-LLDST`, `P-LLAPX`,
  `P-LLARI`, `P-LLEQ`, `P-LLGEO`, `P-LLDES`; `Rect`'s `P-LREG`, `P-LMEM-03`; lemma `L-LL-01`.
- **Inventory names:** `s2::LatLng::<method>`, `<Trait> for LatLng`; crash ids
  `C-s2::LatLng::<method>`.

## Stubs

008's rules, and:

- **A pure contract** keeps one value per input in a `Table<1>` (two for `atan2`): a harness
  that restates a formula uses the pure stub, so the library's call and the restatement's get
  the same value. A harness that does not restate uses the plain contract.
- **A contract on a library function** (`cell_rect_bound_contract`) is tested on the real
  functions in `libm_contracts.rs`, like the C library's, and named in `TRUSTED_BASE.md`.
- **The status follows the stubs**, as in 008: any contract, pure or plain, makes the harness
  `proven-under-contract`.

## Statements

- **Against the description.** Validity and the clamp against `latlng_spec`; a formula with its
  restatement, bit for bit (`to_bits`).
- **No `f64 %`** in a harness: `normalized`'s reduction is stated on `reduce_fmod_copy`
  (P-LLNRM-03), its crash proof does not depend on `%`'s value.
- **Booleans** with `==` and `!=` only.
- **The generic laws** by name: `laws::eq::all` (P-LLEQ-01),
  `laws::region::contains_implies_intersects` (P-LREG-01),
  `laws::region::no_intersect_no_vertex` (P-LREG-02).

## Mutant patches

004's format, numbered `M-9NN`. At least two patch `latlng_spec.rs` and two patch `stubs.rs`
(data-model §7). The fixes' reverting mutants (M-932 to M-934) name `cargo test` targets.

## Records

Section 12 of `verify/values/INVENTORY.md`, section 11 of `verify/values/PROPERTIES.md`,
`Rect`'s new rows in section 5, a spec-010 part of `verify/values/EVALUATION.md`,
`PROPERTIES_BY_KIND.md` and its HTML render, `TRUSTED_BASE.md`, `MUTANTS.md`,
`VERIFIED_TYPES.md`, `CORE_TRAITS.md`, `verify/COVERAGE.md` and `UPSTREAM.md`, in the same
change as the harnesses (constitution v1.2.0).
