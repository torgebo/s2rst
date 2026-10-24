# Contract: Harness and Artifact Conventions (changes for `ChordAngle` and `Angle`)

004's to 006's contracts still apply. This file lists only what this feature adds.

## Modules and names

| Module (`verify/src/`) | Content | Gate |
|------------------------|---------|------|
| `chord_spec.rs`, `chord_spec_tests.rs` | The canonical set, its order and neighbours, and the restatements of upstream's formulas (data-model §1.1), with unit tests | none |
| `angle_spec.rs`, `angle_spec_tests.rs` | Radians, the restated conversions and E-forms, and the remainder by 2π (§1.2), with unit tests | none |
| `value_domain.rs` (extended) | `any_chord`, `any_chord_length`, `any_angle` | `cfg(all(kani, feature = "values"))` |
| `ca_crash.rs`; `ca_val.rs`, `ca_cls.rs`, `ca_ord.rs`, `ca_nbr.rs`, `ca_err.rs`, `ca_ari.rs` | `ChordAngle` harnesses: `ca_<area>::p_ca<area>_<nn>` | same |
| `ag_crash.rs`; `ag_val.rs`, `ag_cnv.rs`, `ag_eform.rs`, `ag_ari.rs` (no `ag_nrm.rs`: P-AGNRM-01 has no harness) | `Angle` harnesses: `ag_<area>::p_ag<area>_<nn>` | same |
| `verify/tests/angle_enumeration.rs` | The enumeration tests of P-AGCNV-02 (the integer identities) and P-AGEF-02, one `#[test] #[ignore]` per statement, each named after its id (`p_agef_02_e7`), run in release mode with `--ignored` | `cargo test --release` |

- **Selection:** `make verify-values P=chord|angle` passes each module as `--harness <module>::`.
- **Catalogue ids:** `P-CAVAL`, `P-CACLS`, `P-CAORD`, `P-CANBR`, `P-CAERR`, `P-CAARI`;
  `P-AGVAL`, `P-AGCNV`, `P-AGEF`, `P-AGNRM`, `P-AGARI`.
- **Inventory names:** `s1::ChordAngle::<method>`, `s1::Angle::<method>`, and the trait
  implementations by name.
- **Crash ids:** `C-s1::ChordAngle::<method>`, `C-s1::Angle::<method>`.

## Statements

- **Against the description.** A chord-angle property compares the library with `chord_spec`
  (the canonical set, its order, its neighbours); an angle property with `angle_spec`.
- **Bit for bit means bit patterns.** Equality of results is `to_bits` equality; the IEEE `==`
  appears only where the property is about `==` (the order laws, the neighbours as values).
- **Canonical values only.** After FR-017, `any_chord` yields every value the type can hold.
  A harness never builds a non-canonical chord angle; the property that none can be built is
  P-CAVAL-02, over every `f64` input.
- **Special operands.** The arithmetic and trigonometry of a chord angle assume non-special
  operands (FR-019's documented precondition); their crash proofs show the debug checks never
  fire there.
- **No `asin`, no `tan` of an `Angle`.** Kani replaces both C-library calls with a failing
  check (research R0). No harness reaches them; the entries through them are tool-limit rows,
  with their values checked against the C++ vectors.
- **Never compare two square roots** (006 R0): `Add`, `Sub`, `sin` and `tan` of a chord angle
  get properties of one result each.
- **NaN is a failed check.** An arithmetic harness assumes the domain where no operation
  produces NaN; the E-forms are the exception, proven over every angle with the saturated
  results stated.
- **Booleans** are compared only with `==` and `!=`; orders compare `Ordering` values.
- **The generic laws** are 004's `laws::eq` and 006's `laws::ord`, instantiated by
  P-CAORD-01 (every canonical value) and P-AGVAL-02 (angles without NaN).

## Split families

- `__neg`, `__len`, `__inf` (which canonical value) where a neighbour statement times out
  over all of them.
- `__k5`, `__k6`, `__k7` (the E-form) if P-AGEF-01's statement times out over all three.
  The round trips are not Kani harnesses: they are enumerated (research R12).
- A module with a `__` family carries
  `#![allow(non_snake_case, reason = "`__` marks a split-family member (harness-conventions)")]`.

## Mutant patches

004's format, numbered `M-6NN`. At least two patch the descriptions (`chord_spec.rs`,
`angle_spec.rs`).

## Records

Sections 7 (`ChordAngle`) and 8 (`Angle`) of `verify/values/INVENTORY.md`, sections 8 and 9
of `verify/values/PROPERTIES.md`, a spec-007 part of `verify/values/EVALUATION.md`, and
`PROPERTIES_BY_KIND.md` with its HTML render (`pandoc -f markdown -t html`), in the same
change as the harnesses (constitution v1.2.0). `property_kinds.rs` needs no change.
