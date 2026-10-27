# Contract: Harness and Artifact Conventions (changes for `Cap`)

004's to 007's contracts still apply. This file lists only what this feature adds.

## Modules and names

| Module (`verify/src/`) | Content | Gate |
|------------------------|---------|------|
| `cap_spec.rs`, `cap_spec_tests.rs` | The description and the restatements (data-model §1), with unit tests | none |
| `stubs.rs` | The three function contracts and the lemma stubs, on one `Table<K>` type (data-model §3) | `cfg(kani)` |
| `value_domain.rs` (extended) | `any_box_cap`, `any_valid_cap`, `any_nonempty_cap`, `draw_abstract_cell` | `cfg(all(kani, feature = "values"))` |
| `laws.rs` (extended) | `region::no_intersect_no_vertex`, `region::cap_bound_contains_points` | none |
| `cp_crash.rs`; `cp_con.rs`, `cp_cls.rs`, `cp_mea.rs`, `cp_mem.rs`, `cp_rel.rs`, `cp_gro.rs`, `cp_cmp.rs`, `cp_bnd.rs`, `cp_eq.rs`, `cp_reg.rs`, `cp_lem.rs` | `Cap` harnesses: `cp_<area>::p_cp<area>_<nn>`, lemmas `cp_lem::l_cp_<nn>` | same as `value_domain` |
| `verify/tests/libm_contracts.rs` | Native tests of the three contracts against the real functions (research R5) | `cargo test` |

- **Catalogue ids:** `P-CPCON`, `P-CPCLS`, `P-CPMEA`, `P-CPMEM`, `P-CPREL`, `P-CPGRO`,
  `P-CPCMP`, `P-CPBND`, `P-CPEQ`, `P-CPREG`, `P-CPCOD`, `P-CPDES`; lemmas `L-CP-NN`.
- **Inventory names:** `s2::Cap::<method>`, `Region for Cap::<method>`; crash ids
  `C-s2::Cap::<method>`.

## Stubs

- **Every stub attribute names a function of `stubs.rs`**, never an inline closure, and the
  harness's documentation lists them after its statement: "Stubs: `pure_chord` (L-CP-01,
  L-CP-02), `pure_add` (P-CAARI-01, L-CP-05)."
- **The status follows the stubs:** none → proven; lemma stubs only → by composition; any
  function contract → under a stated contract.
- **A lemma stub returns, for each input, a value its lemmas allow, and the same value for
  the same input.** It never returns a value the real function cannot: the lemma harness
  proves the real function satisfies exactly the stated facts.
- **Tables** are `static Mutex<Table<K>>` with fixed capacity and element-wise key
  comparison: no `thread_local!` (it crashes Kani 0.68.0's compiler), no `Vec`, no `==` on
  arrays or tuples inside a stub (research R0b).
- **Each harness that uses a table resets it first** (`stubs::reset()`), so harnesses do not
  share values; Kani runs each harness from a fresh state anyway, and the reset keeps the
  intent explicit.
- **A private function** is stubbed by its full path (`s2rst::s2::cap::<name>`), which Kani
  resolves (research R0b).

## Statements

- **Against the description.** A membership or relation property compares the library with
  `cap_spec`; a formula property with the restatement, bit for bit (`to_bits`).
- **One computation per value.** A harness never recomputes, on the real code, a float value
  the operation already computed: it either uses a lemma stub, or states the property of the
  one result.
- **Booleans** are compared only with `==` and `!=` (Kani's bool-ordering bug).
- **No `f64 %`** in any harness, description or reduction.
- **The generic laws** are instantiated by name: `laws::eq::all` (P-CPEQ-01),
  `laws::region::contains_implies_intersects` (P-CPREG-01), `laws::region::no_intersect_no_vertex`
  (P-CPREG-02), `laws::region::cap_bound_contains_points` (P-CPREG-03),
  `laws::region::covering_covers::<Cap, 6>` (P-CPREG-04), 003's codec law (P-CPCOD-01).

## Split families

- `__real` / `__abstract` for the cell laws (the real edge test weekly, abstracted per change).
- `__empty`, `__nonempty` where a growth or complement statement times out over both.
- A module with a `__` family carries
  `#![allow(non_snake_case, reason = "`__` marks a split-family member (harness-conventions)")]`.

## Mutant patches

004's format, numbered `M-7NN`. At least two patch `cap_spec.rs` and two patch `stubs.rs`
(data-model §7).

## Records

Section 9 of `verify/values/INVENTORY.md`, section 10 of `verify/values/PROPERTIES.md`, the new
`Cell` entry P-CCAP-01 and the updated P-PTDST-01, a spec-008 part of
`verify/values/EVALUATION.md`, `PROPERTIES_BY_KIND.md` and its HTML render, `TRUSTED_BASE.md`,
`MUTANTS.md`, `VERIFIED_TYPES.md` and `CORE_TRAITS.md`, in the same change as the harnesses
(constitution v1.2.0). `property_kinds.rs` gains the two new statuses if it checks statuses.
