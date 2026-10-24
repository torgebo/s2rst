# Quickstart: Validating the `ChordAngle` and `Angle` Proofs

How to check, end to end, that the feature works. Commands run from the repository root. The
toolchains are 001's: Kani 0.68.0 (`make verify-kani-check`) and, for the vectors only, the
local s2geometry install.

## 1. The gate is unchanged and Kani-free

```sh
make test
```

Expected: passes, including:
- the new `core/src/s1/chord_angle_tests.rs` and `core/src/s1/angle_tests.rs`;
- the `Cap` decoder's tests in `core/src/s2/encoding_tests.rs`;
- the extended `core/tests/cpp_values_diff.rs`;
- the Python suite.

## 2. The descriptions agree with C++

```sh
cd verify && cargo test
```

Expected: `chord_spec_tests`, `angle_spec_tests` and `values_spec_vs_cpp` pass. The last
reports the `CHORD_ANGLE` and `ANGLE` row counts, with the `deviation:negative-canonical` and
`deviation:eform-saturate` rows (FR-024). With `-- --nocapture` (2026-10-19): `CHORD_ANGLE: 965
rows, 18 negative-canonical, 5 Infinity()` and `ANGLE: 1045 rows, 472 out of the int32 range,
45 exact ties`.

## 3. The per-change proofs and the enumeration

```sh
make verify-values P=chord
make verify-values P=angle
```

Expected:
- Every per-change Kani harness is `SUCCESSFUL`: 28 for `chord` (159.5 s of harness time, the
  longest 27.9 s) and 14 for `angle` (66.5 s), each leg a few minutes on 5 jobs (SC-005).
- `P=angle` also runs `verify/tests/angle_enumeration.rs` in release mode: 7 tests enumerate
  every `i32` (and `u32`) for the E-form round trips, the E-form constructors and upstream's
  integer identities, 19.5 s with 5 threads (2026-10-18), with no failure.
- The members that compare two float products or quotients bit for bit are weekly:
  `make verify-slow F=values P='ca_ari::p_caari_03__sin ag_cnv::p_agcnv_01 ag_eform::p_agef_01 ag_ari::p_agari_01__mul ag_ari::p_agari_01__div ag_ari::p_agari_01__ratio'`
  (60 minutes each; results in `verify/values/PROPERTIES.md` §8–9). On 2026-10-18 the six
  `ag_cnv`, `ag_eform` and `__mul` members proved in 5 to 16 minutes each, and the four others
  (`sin2`, `sin`, `__div`, `__ratio`) ended `KNOWN-TIMEOUT` (`verify/known-timeouts.txt`).
  CI runs them in the `slow` job's `values-chord-angle` and `values-limits-e` legs.
- A failure prints:
  - the harness, whose name is the property id (`ca_nbr::p_canbr_01` for P-CANBR-01);
  - the failed check;
  - a concrete-playback test.
- Making `successor` of `STRAIGHT` step by one ulp fails `p_canbr_01` in seconds.

## 4. Non-vacuity

```sh
make verify-mutants M='M-6*'
```

Expected:
- every `M-6NN` is killed, recorded in `verify/MUTANTS.md` (SC-004);
- the `normalized` mutants (`M-646`, `M-647`) and the enumeration's (`M-636`, `M-637`,
  `M-644`, `M-648`) are killed by the ordinary tests their `Test:` headers name, not by Kani (research
  R0, Finding 1); `run-mutants.sh` runs those tests in the mutant's worktree;
- the `Tier: slow` mutants need `VERIFY_SLOW_HARNESS_TIMEOUT=60m`.

## 5. The fixes, by hand

```sh
cargo test -p s2rst --lib chord_angle
cargo test -p s2rst --lib angle
cargo test -p s2rst --lib encoding_tests::cap
```

Expected: pass. With any of FR-017, FR-018, FR-020, FR-021 or FR-022 reverted, its tests fail;
the commit messages record the failing values:
- `from_length2(−5)` is −5 instead of `NEGATIVE`;
- `from_length2(NaN)` is NaN instead of `STRAIGHT`;
- `normalized(−2.462748394469102e16)` is 4.0;
- `e6` at the tie 2.5 is 3.

## 6. The records

- `verify/values/INVENTORY.md` §7–8: 36 and 31 entries, every exclusion with a reason (8 tool
  limits: 7 through `asin`, and `Angle::tan`).
- `verify/values/PROPERTIES.md` §8–9: 14 and 8 entries with statuses. P-AGEF-02 is
  `proven-by-enumeration`, and P-AGCNV-01's and P-AGCNV-02's integer clauses are proven by
  enumeration. P-AGNRM-01 (Kani's `%`), P-CAARI-03 and P-AGARI-01 (timeouts at 60 minutes) are
  `not-proven (tool limit)`, each with its evidence.
- `verify/PROPERTIES_BY_KIND.md` and its HTML render: every new id exactly once
  (`cargo test property_kinds`).
- `verify/decoding/PROPERTIES.md`: P-FX-08 names the negative-radius deviation.
- `UPSTREAM.md`: U-014, with its follow-up task.
- `VERIFIED_TYPES.md`: `ChordAngle` and `Angle` have a section of their own.
- `verify/values/EVALUATION.md`, spec-007 part: the invariant's assessment, the go or no-go for
  `LatLng` and `Cap`, and the ranked follow-ups.
