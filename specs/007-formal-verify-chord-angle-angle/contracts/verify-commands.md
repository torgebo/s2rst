# Contract: Verification Commands (changes for `ChordAngle` and `Angle`)

The 001–006 contracts still apply. This file lists only what changes.

## `make verify-values [P=…|chord|angle]` (extended)

- `P=chord` selects the `ca_*` modules, and `P=angle` the `ag_*` modules, each passed as
  `--harness <module>::` (spec 006's final-run fix).
- Without `P`, the target runs all eight parts.
- `P='<module>::<prefix> …'` selects single harnesses, as the weekly legs do.
- The models' sanity tests (`cargo test` in `verify/`) cover `chord_spec` and `angle_spec`.

## `make verify-slow F=values [P=…]` (extended)

The weekly members of this feature (research R12) join a new leg, `values-chord-angle`, with
its filter list. A member that times out at 60 minutes is listed in
`verify/known-timeouts.txt` and joins a `values-limits-*` leg.

## `make verify-decoding P=codecs` (re-run)

003's `Cap` decoder agreement (`cd_fx::p_fx_08__*`) re-runs against the updated
`wire_spec::lib_cap` (research R5). No new target.

## CI: `.github/workflows/verify.yml`

| Job | Change |
|-----|--------|
| `verify` | The matrix gains `values-chord` and `values-angle`. `core/src/s1/angle.rs`, `core/src/s1/chord_angle.rs`, `core/src/s2/cap.rs` and `core/src/s2/encoding.rs` are already in the path filters |
| `slow` | Gains the `values-chord-angle` leg (filters from research R12), and limit filters if any member times out |
| `mutants` | The series matrix gains `6` (`M='M-6*'`) |

## `make test` (unchanged definition)

It runs the new ordinary tests:

- `core/src/s1/chord_angle_tests.rs` (FR-017, FR-018, FR-019's debug checks, FR-020);
- `core/src/s1/angle_tests.rs` (FR-021, FR-022, FR-023);
- `core/src/s2/encoding_tests.rs` (FR-017's decoder change, FR-018's NaN rejection kept);
- the extended `core/tests/cpp_values_diff.rs`;
- the Python suite, whose `ChordAngle` tests are checked against the changed constructors.
