# Contract: Verification Commands (changes for `Cap`)

The 001–007 contracts still apply. This file lists only what changes.

## `run-verify.sh` (changed)

Every `cargo kani` call it makes (codegen, main run, single-harness re-runs, playback) passes
`-Z stubbing`, because the crate now has `#[kani::stub]` attributes (research R2). Harnesses
without stubs are unaffected. A caller never needs to pass the flag.

## `make verify-values [P=…|cap]` (extended)

- `P=cap` selects the `cp_*` modules, each passed as `--harness <module>::`.
- Without `P`, the target runs all nine parts.
- `P='<module>::<prefix> …'` selects single harnesses, as the weekly legs do.
- The description's sanity tests and the contract tests (`cargo test` in `verify/`) cover
  `cap_spec` and `tests/libm_contracts.rs` (research R5); the values leg runs them.

## `make verify-slow F=values [P=…]` (extended)

The weekly members of this feature (research R10) join a new leg, `values-cap`, with its
filter list. A member that times out at 60 minutes is listed in `verify/known-timeouts.txt`
(`union`'s crash, `cell_union_bound`'s length and the covering law, if they do). 006's
P-PTDST-01 symmetry entry leaves `known-timeouts.txt` (research R12), so the runner's stale-entry
check does not fail.

## CI: `.github/workflows/verify.yml`

| Job | Change |
|-----|--------|
| `verify` | The matrix gains `values-cap`. `core/src/s2/region.rs`, `core/src/s2/metric.rs` and `core/tests/trait_property_tests.rs` join the path filters (`cap.rs` is already there) |
| `slow` | Gains the `values-cap` leg (filters from research R10) |
| `mutants` | The series matrix gains `7` (`M='M-7*'`) |

## `make test` (unchanged definition)

It runs the new ordinary tests:

- `core/src/s2/cap_tests.rs` (FR-017 to FR-020, FR-022, FR-023);
- the new `core/tests/cpp_cap_diff.rs` (four `CAP_*` sections of `cpp_cap_vectors.csv`);
- `core/tests/trait_property_tests.rs` with the `f64` cap generator and K5's example;
- the Python and wasm suites, which pin no exact coverings of caps (checked 2026-10-24:
  `test_coverer.py` and `test_encoding.py` assert sizes only).
