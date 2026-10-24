# Contract: Harness and Artifact Conventions (changes for `Cell` and `s1::Interval`)

001's contract (`specs/001-formal-verify-base-type/contracts/harness-conventions.md`) still
applies, with 002's and 003's additions: crate-root modules, the doc-comment rule, the
forbidden stubs and flags, the resource budget and the mutant patch format. This file lists
only what this feature adds.

## Modules and names

| Module (`verify/src/`) | Content | Gate |
|------------------------|---------|------|
| `cell_spec.rs` | The reference description of a cell (data-model §1.1), on top of 001's `spec.rs`. Plain Rust, always compiled; hand-made unit tests in the sibling `cell_spec_tests.rs` | none |
| `arc_spec.rs` | The reference description of a circular interval: the set of points an arc denotes, and upstream's length arithmetic (data-model §1.2). Plain Rust, always compiled; hand-made unit tests in `arc_spec_tests.rs` | none |
| `value_domain.rs` | Generators for cells, intervals, points and floats (below) | `cfg(all(kani, feature = "values"))` |
| `laws.rs` | The generic trait laws, each stated once over a type parameter (data-model §4.3, research R15): `laws::region` (integer `Region` laws) and `laws::eq` (equivalence of `PartialEq`). Instance harnesses in `cl_cnt`, `cl_eq` and `iv_val` call them with the type's generators. Later suites instantiate the same functions (follow-up: `CellUnion`) | `cfg(kani)`, no feature gate, so every suite can use it |
| `cl_crash.rs` | `cl_crash::c_<op>`, one per in-scope `Cell` entry without a shared property harness | same |
| `cl_id.rs`, `cl_chd.rs`, `cl_cnt.rs`, `cl_bnd.rs`, `cl_eq.rs` | `cl_<area>::p_c<area>_<nn>`: identity and construction, children, containment and the region interface, bounds, equality. For example `cl_chd::p_cchd_02` | same |
| `iv_crash.rs` | `iv_crash::c_<op>`, as for `cl_crash` | same |
| `iv_val.rs`, `iv_mem.rs`, `iv_rel.rs`, `iv_set.rs`, `iv_mea.rs`, `iv_exp.rs` | `iv_<area>::p_i<area>_<nn>`: validity and representation, membership, relations, set operations, measures (with the FR-020 length proofs), expansion and approximate equality | same |

- **Catalogue ids:**
  - `Cell`: `P-CID` (identity and construction), `P-CCHD`, `P-CCNT`, `P-CBND` and `P-CEQ`.
  - The interval: `P-IVAL`, `P-IMEM`, `P-IREL`, `P-ISET`, `P-IMEA` and `P-IEXP`.
  - None of these collides with an existing area. In particular, `P-CV` is 003's cell-id
    vector, and `P-VAL` is 001's.
- **Selection:**
  - `--harness cl_` selects the `Cell` harnesses, and `--harness iv_` the interval's.
  - No existing harness path or module contains `cl_` or `iv_` (checked 2026-10-12 against
    the 172 hand-written harnesses and the macro-generated families).
- **Inventory names:**
  - `Cell::<method>`, `impl PartialEq for Cell`, `impl Default for Cell`,
    `impl From<CellId> for Cell` and `impl Region for Cell::<method>`;
  - `s1::Interval::<method>`, `impl Default for s1::Interval` and
    `impl PartialEq for s1::Interval`.
- **Crash ids:** `C-Cell::<method>` and `C-s1::Interval::<method>`.

## Split families

001's `__<k>` rule applies: a family's members together cover the stated domain, and a split
is never a restriction. There is one new suffix:

- `__l<L>`: cells at exactly level `L`, built with `domain::any_valid_at(L)`. A family
  `__l0 … __l30` covers every valid cell. It is used only for P-CBND-01, P-CBND-02 and
  P-CCHD-02, whose whole-domain harnesses are over budget (research R0, R8).
  - Each family is generated with a `macro_rules!` list, as in `cd_cv.rs` (see
    VERIFIED_TYPES S3), with one member per level up to the measured limit.
  - Members are written only for the levels the per-change or weekly budget reaches. The
    catalogue states the entry as **proven within stated bounds (levels 0–L)**, so the gap
    above L is recorded, never hidden.
  - Members above the per-change levels carry `#[cfg(feature = "slow")]`.
- `__xinv`, `__xnoninv`, `__yinv`, `__ynoninv`: an interval argument restricted to inverted
  or non-inverted bounds. A pair of these covers the domain. They are used where research R9
  splits P-IEXP-01 or P-IMEA-06.

Interval harnesses are otherwise whole. Their domains are two or three `f64` values, and the
pilot and research R0 proved them whole.

## Generators (`value_domain.rs`)

Every domain is built from the reference descriptions, never from the implementation's own
`is_valid`. A harness's domain therefore does not depend on the code under test.

| Generator | Domain (data-model §2) | Construction |
|-----------|------------------------|--------------|
| `any_cell()` | `ValidCell` | `Cell::from_cell_id(id)` for `(id, d) = domain::any_valid()`. It also returns `d` |
| `any_cell_at(L)` | `ValidCell` at level `L` | as above, with `domain::any_valid_at(L)` |
| `any_interval()` | `ValidInterval` | `Interval { lo, hi }` from two symbolic `f64`, with `kani::assume(arc_spec::valid(lo, hi))` |
| `any_point()` | `Point` | a symbolic `f64` with `kani::assume(arc_spec::in_range(p))`, that is, `−π ≤ p ≤ π` (no NaN) |
| `any_f64()` | `AnyF64` | `kani::any::<f64>()`: every bit pattern, NaN and the infinities included |
| `any_edge()` | `Edge` | one of the four `CellEdge` values |

Intervals are built as struct literals, which the public fields allow, so that a harness
reaches every valid pair, including those no constructor returns.

## Generic laws (`laws.rs`)

A law is a plain generic function that takes values of the instance's domain and asserts.
It is never a harness itself. An instance harness draws the values with its own generators
and calls the law, as 003's `Codec` bodies are called per codec:

```text
/// P-CCNT-03 (law): the generic Region laws, for Cell. …
#[kani::proof] #[kani::unwind(9)]
fn p_ccnt_03() { let (a, b) = (any_cell().0, any_cell().0); laws::region::all(&a, &b); }
```

- A law states only what every implementor must satisfy. Instance-specific meaning stays in
  the type's own entries.
- The quickcheck twin of each law lives in `core/tests/trait_property_tests.rs` under the
  same name. The two lists are kept in step by hand, and `CORE_TRAITS.md` names both.

## Harness body shape

001's five steps apply, with these additions:

- **Floating-point equality is bit equality.** A property that says two results are equal
  compares `to_bits()`, so `0.0` and `−0.0` differ and NaN equals itself. Where upstream
  promises only an approximate result, the property states its tolerance in ulps and the
  reason (FR-008), and the harness uses `arc_spec::ulps_apart`.
- **Membership is checked pointwise, relations exactly.** A membership property draws a
  symbolic point `p` with `any_point()` and asserts the reference membership for that `p`;
  Kani proves it for every `p`. A relation between intervals is compared with the real-arc
  relation of `arc_spec` (`contains`, `interior_contains`, `intersects`,
  `interior_intersects`), which decides it by the cyclic order of the bounds. Float points
  cannot decide it: a float-free gap is still a gap (data-model §1.2).
- **The FR-020 length proofs (`iv_mea`)** compare the library's `length` with
  `arc_spec::upstream_length`, which restates upstream's three lines, bit for bit, for every
  valid interval except the corner. At the corner they compare it with `arc_spec::ULP_PI`
  (research R5).
- **Debug-only checks.** Kani builds with debug assertions on, so every `debug_assert!` on a
  harness's path is an obligation of that harness. A crash harness whose domain is the
  documented one therefore proves that the check never fires there (FR-004).

## Attributes

- `Cell` harnesses that build a cell carry `#[kani::unwind(9)]`, the eight lookup rounds of
  `to_face_ij_orientation` plus one (as in 001's `crash` module).
- Interval harnesses need no unwind attribute: no interval operation loops.

## Mutant patches

001's format applies. This feature's mutants are numbered `M-3NN` (001 used `M-0NN`, 002
`M-1NN` and 003 `M-2NN`). They target `core/src/s2/cell.rs`, `core/src/s1/interval.rs` and
the `Region` impl in `core/src/s2/region.rs`. The FR-020 containment proof has two dedicated
mutants (research R5):

- one widens the fix to every inverted interval;
- one removes it.

Each must make its target harness fail.

## Records

The suite's records go in a new directory, `verify/values/`:

- `INVENTORY.md`: two tables, `Cell` (53 rows) and `s1::Interval` (31 rows).
- `PROPERTIES.md`: one catalogue in two parts.
- `EVALUATION.md`.

`verify/tests/property_kinds.rs` gains `values/PROPERTIES.md` in `CATALOGUES`, and
`PROPERTIES_BY_KIND.md` lists every new entry under its kind (FR-010, SC-012).
