# Contract: Verification Commands (changes for `LatLng` and `Rect` as a `Region`)

The 001–008 contracts still apply. This file lists only what changes.

## `make verify-values [P=…|latlng]` (extended)

- `P=latlng` selects the `ll_*` modules, each passed as `--harness <module>::`.
- `P=latlng` includes `ll_reg`, `Rect`'s `Region` laws (moved from `Rect`'s part at the final run: with them the `P=rect` leg took 11 min 8 s, over SC-006's 10 minutes).
- Without `P`, the target runs all ten parts.
- The description's sanity tests and the contract tests (`cargo test` in `verify/`) cover
  `latlng_spec`, the `LATLNG_*` rows and the `Cell::rect_bound` contract.

## `make verify-slow F=values [P=…]` (unchanged)

No weekly member is planned (research R2). A harness that takes more than the per-change budget
in implementation joins a new `values-latlng` slow leg with its filter list, as 008's did.

## `make verify-mutants M='M-9*'` (new series)

The mutants `M-901` … of data-model §7.

## CI: `.github/workflows/verify.yml`

| Job | Change |
|-----|--------|
| `verify` | The matrix gains `values-latlng`. `core/src/s2/latlng.rs`, `core/src/s1/angle.rs`, `core/src/s2/rect.rs` and `core/src/s2/region.rs` are in the path filters (added where missing) |
| `mutants` | The series matrix gains `9` (`M='M-9*'`) |

## `make test` (unchanged definition)

It runs the new ordinary tests:

- `core/src/s2/latlng_tests.rs` (FR-017, FR-018), `core/src/s2/rect_tests.rs` (FR-019),
  `core/src/s1/angle_tests.rs` (the shared remainder);
- the new `core/tests/cpp_latlng_diff.rs` (the seven sections of `cpp_latlng_vectors.csv`);
- `core/tests/trait_property_tests.rs` with the rectangle boundary points (FR-011);
- the Python and wasm suites, which inherit the fixes.
