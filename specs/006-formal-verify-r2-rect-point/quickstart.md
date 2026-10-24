# Quickstart: Validating the `r2::Rect` and `Point` Proofs

How to check, end to end, that the feature works. Commands run from the repository root. The
toolchains are 001's: Kani 0.68.0 (`make verify-kani-check`) and, for the vectors only, the
local s2geometry install.

## 1. The gate is unchanged and Kani-free

```sh
make test
```

Expected: passes, including the new `core/src/r2/rect_tests.rs`, `core/src/s2/point_tests.rs`,
`core/src/s2/edge_crossings_tests.rs` and the extended `core/tests/cpp_values_diff.rs`.

## 2. The descriptions agree with C++

```sh
cd verify && cargo test
```

Expected: `uv_spec_tests`, `point_spec_tests` and `values_spec_vs_cpp` pass; the latter reports
the `R2RECT_*` and `S2POINT_*` row counts and the `deviation:robust-cross-overflow` rows
(FR-021).

## 3. The per-change proofs

```sh
make verify-values P=uvrect
make verify-values P=point
```

Expected: every harness `SUCCESSFUL`, each leg under 10 minutes on 4 CPUs (SC-005): 31
harnesses in about 2 minutes and 20 in about 6 on the workstation with `VERIFY_JOBS=4`
(2026-10-14). A failure prints the harness, whose name is the property id (`pt_ord::p_ptord_01`
for P-PTORD-01), the failed check with its file and line, and a concrete-playback test: making
`cmp_point` skip y fails `p_ptord_01` and `p_ptord_02` in seconds (T047).

## 4. The weekly members

```sh
make verify-slow F=values P=uvrect
make verify-slow F=values P=point
```

Each runs its whole part with the weekly members, one harness at a time, 60 minutes each.
Expected (2026-10-14): the u-v rectangle's 5 weekly members prove (the longest in 1,751 s); of
the point's 14, 6 prove and 8 time out at 60 minutes, recorded as "not proven (tool limit)"
with the measurement: unit length after `normalize` and `ortho`, the band of `from_coords`, the
three sign laws of `point_cross` and the chord angle's symmetry. The runner reports those 8 as
`KNOWN-TIMEOUT` (`verify/known-timeouts.txt`), which does not fail the run; CI runs them in the
`values-limits-*` legs of the weekly `slow` job. With one job the point part
takes about 9 hours; with `VERIFY_JOBS=3`, 18 of the 19 weekly members
(all but `p_ptval_02__zero_length`, 516 s, measured per change) took 3 h 23 min.

## 5. Non-vacuity

```sh
VERIFY_SLOW_HARNESS_TIMEOUT=60m make verify-mutants M='M-5*'
```

The `Tier: slow` mutants need the weekly 60 minutes; the runner's default is 30.

Expected: every `M-5NN` killed, recorded in `verify/MUTANTS.md` (SC-004), except `M-539`,
which is `no-verdict`: its target member times out unmutated too (`verify/known-timeouts.txt`),
and the property test `prop_from_coords_unit` catches it. The campaign took 2 h 53 min (2026-10-14), an hour of it the concrete-playback re-run
after `M-540`'s kill (145 s), which the runner now skips.

## 6. The fixes, by hand

```sh
cargo test -p s2rst --lib ensure_normalizable
cargo test -p s2rst --lib robust_cross_prod
```

Expected: pass. With FR-020 (`073d7b0`) or FR-021 (`36a4a9c`) reverted, their tests fail; the
commit messages record the failing values.

## 7. The records

- `verify/values/INVENTORY.md` §5–6: 33 and 31 entries, every exclusion with a reason.
- `verify/values/PROPERTIES.md` §6–7: 16 and 16 entries with statuses; §3 lists `laws::ord`.
- `verify/PROPERTIES_BY_KIND.md`: every new id exactly once (`cargo test property_kinds`).
- `UPSTREAM.md`: U-012 and U-013, each with its follow-up task.
- `verify/values/EVALUATION.md`, spec-006 part: SC-014's numbers and the `ChordAngle` follow-up
  with its open question.
