# Contract: Harness and Artifact Conventions (changes for `r1::Interval` and `s2::Rect`)

004's contract (`specs/004-formal-verify-cell-interval/contracts/harness-conventions.md`) and
the earlier ones still apply. This file lists only what this feature adds.

## Modules and names

| Module (`verify/src/`) | Content | Gate |
|------------------------|---------|------|
| `real_spec.rs`, `real_spec_tests.rs` | The real-set description (data-model §1.1) and its unit tests | none |
| `rect_spec.rs`, `rect_spec_tests.rs` | The rectangle as a product of `real_spec` and 004's `arc_spec` (§1.2) and its unit tests | none |
| `value_domain.rs` (extended) | `any_real`, `any_finite_real`, `any_number`, `any_rect`, `any_lat_lng`, `any_valid_lat_lng` | `cfg(all(kani, feature = "values"))` |
| `ri_crash.rs`; `ri_val.rs`, `ri_mem.rs`, `ri_rel.rs`, `ri_set.rs`, `ri_mea.rs`, `ri_exp.rs` | `r1::Interval` crash and property harnesses: `ri_<area>::p_r<area>_<nn>` | same |
| `rl_crash.rs`; `rl_val.rs`, `rl_mem.rs`, `rl_rel.rs`, `rl_set.rs`, `rl_bnd.rs`, `rl_exp.rs`, `rl_eq.rs` | `s2::Rect` crash and property harnesses: `rl_<area>::p_l<area>_<nn>` | same |

- **Selection:** `--harness ri_` and `--harness rl_`. Neither prefix matches an existing name
  (research R2).
- **Catalogue ids:** `P-RIV`, `P-RMEM`, `P-RREL`, `P-RSET`, `P-RMEA`, `P-REXP`, `P-LVAL`,
  `P-LMEM`, `P-LREL`, `P-LSET`, `P-LBND`, `P-LEXP`, `P-LEQ`.
- **Inventory names:** `r1::Interval::<method>`, `s2::Rect::<method>`,
  `impl Region for s2::Rect::<method>`, and the trait implementations by name.
- **Crash ids:** `C-r1::Interval::<method>`, `C-s2::Rect::<method>`.

## Statements

- **Against the product, never component-wise.** A rectangle property compares the library with
  `rect_spec`, built from `real_spec` and `arc_spec`. Asserting that a rectangle operation
  equals its components' operations restates the code, and R0 measured it at a timeout.
- **NaN is a failed check.** Kani reports a NaN produced by arithmetic. A harness of an
  arithmetic operation of the real interval assumes `FiniteReal` and finite arguments, exactly
  as FR-004 and FR-017 document; it never filters NaN out after the fact.
- **Booleans** are compared only with `==` and `!=` (004 R13).
- **The generic laws** are 004's `laws::eq`, instantiated in `ri_val` (P-RIV-02) and `rl_eq`
  (P-LEQ-01).

## Split families

- `__mpos`, `__mneg` (a margin's sign) for the rectangle's expansion containment (weekly).
- The other members stay whole. 004's `__xinv` and `__xnoninv` apply where a longitude arc
  needs them.

## Mutant patches

004's format, numbered `M-4NN`. Two patch the descriptions (`verify/src/real_spec.rs`,
`verify/src/rect_spec.rs`); `run-mutants.sh` applies `verify/` patches in its worktree, as
004's `M-340` to `M-342` do.

## Records

Sections 3 (`r1::Interval`) and 4 (`s2::Rect`) of `verify/values/INVENTORY.md`, sections 4
and 5 of `verify/values/PROPERTIES.md` (its section 3 is 004's generic laws), and a spec-005 part of `verify/values/EVALUATION.md`.
`PROPERTIES_BY_KIND.md` lists every new entry; `property_kinds.rs` needs no change.
