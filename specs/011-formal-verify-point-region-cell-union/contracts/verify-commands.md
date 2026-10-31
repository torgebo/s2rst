# Contract: Verification Commands (changes for `PointRegion` and `CellUnion`'s floating-point methods)

The 001–010 contracts still apply. This file lists only what changes.

## `make verify-values [P=…|regions]` (extended)

- `P=regions` selects the `pr_*` modules (`PointRegion`) and the `uf_*` modules (`CellUnion`'s
  floating-point methods), each passed as `--harness <module>::`; `P=pointregion` and
  `P=unionfloat` select each half, the two CI legs (measured in implementation: together the
  part took 14 min 48 s, over SC-006's 10 minutes).
- Without `P`, the target runs all eleven parts.
- The crate's tests (`cargo test` in `verify/`) run first, as for every part: the new
  `libm_contracts` cases for the `Cell::cap_bound` and `CellId::from_point` contracts (research
  R5) and `property_kinds`.

## `make verify-slow F=values [P=…]` (unchanged)

Members over the per-change budget (research R0: `uf_crash`'s real-`Cell` members, the vertex
law) join a new `values-regions` slow leg with their filter list, as 008's and 010's did.

## `make verify-mutants M='M-95*' …` (series 9 extended)

The mutants `M-950` … of data-model §7. They share series 9 with spec 010's `M-9NN`
(research R2): the CI leg runs `M='M-9*'`, which selects both.

## CI: `.github/workflows/verify.yml`

| Job | Change |
|-----|--------|
| `verify` | The matrix gains `values-pointregion` and `values-unionfloat`. `core/src/s2/point_region.rs` and `core/tests/data/cpp_region_vectors.csv` join the path filters |
| `slow` | A `values-regions` leg with the weekly members' filters |
| `mutants` | Unchanged matrix; the comment says series 9 holds 010's and 011's |

## `make test` (unchanged definition)

It runs the new ordinary tests:

- `core/src/s2/cell_tests.rs` or `cell.rs`'s tests (FR-017), `core/src/s2/cell_union_tests.rs`
  (FR-018, FR-019's `expand_by_radius` panic), `core/src/s2/point_region.rs`'s tests (FR-019's
  deserialiser);
- the new `core/tests/cpp_region_diff.rs` (the sections of `cpp_region_vectors.csv`);
- `core/tests/trait_property_tests.rs` with the new twins (FR-010, FR-013);
- the Python and wasm suites, which inherit the fixes.
