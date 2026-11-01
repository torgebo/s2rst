# Contract: Verification Commands (changes for `Cell`'s bounding laws and the adaptors)

The 001–011 contracts still apply.

- `make verify-values P=adaptors` selects the `ad_*` modules; `P=…` without it runs all twelve
  parts.
- `make verify-slow F=values P='<the values-adaptors slow leg's filters>'` runs the weekly
  members (the bounds at 1 and 2 members, the rectangle law at 2 and 3, the vertex law at 2).
- `make verify-mutants M='M-97* M-98*'` runs this feature's mutants (and 011's `M-97x`).
- CI: the `verify` matrix gains `values-adaptors`; the `slow` job a `values-adaptors` leg; the
  path filters gain `core/src/s2/region_union.rs`, `region_intersection.rs` and
  `cpp_cell_bounds_vectors.csv`.
- `make test` runs `cpp_cell_bounds_diff`, the new `cell_tests` and adaptor tests, and the twins.
