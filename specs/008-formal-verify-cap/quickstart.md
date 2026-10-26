# Quickstart: Validating the `Cap` Proofs

How to check, end to end, that the feature works. Commands run from the repository root. The
toolchains are 001's: Kani 0.68.0 (`make verify-kani-check`), and, for the C++ vectors only,
the existing s2geometry build installed to a prefix (`tools/refgen-cpp/README.md`).

## 1. The gate is unchanged and Kani-free

```sh
make test
```

Expected: passes, including:
- `core/src/s2/cap_tests.rs` (the fixes FR-017 to FR-020, FR-022, FR-023);
- the new `core/tests/cpp_cap_diff.rs`, over the four `CAP_*` sections of `cpp_cap_vectors.csv`;
- `core/tests/trait_property_tests.rs`, with the `f64` cap generator and K5's example;
- the Python and wasm suites, unchanged (they pin no exact coverings of caps).

Measured (2026-10-25, T052): passes. The Rust suites, 914 Python tests, mypy and stubtest, the
wasm suite (277) and `cargo-deny`.

## 2. The description and the contracts agree with reality

```sh
cd verify && cargo test
```

Expected:
- `cap_spec_tests` and `values_spec_vs_cpp` pass; the latter reports the `CAP_*` row counts
  and no deviation rows other than `Union`'s documented margin (data-model §1, research R9);
- `libm_contracts` passes: every special value and 10⁶ random inputs each satisfy the
  `asin`, `atan2` and `cos` contracts (data-model §3, research R5).

Measured (2026-10-25, as the first step of `make verify-values`): all pass. The unit tests (87),
`values_spec_vs_cpp` (11), `libm_contracts` (4, 0.06 s), `property_kinds` (3).

## 3. The per-change proofs

```sh
make verify-values P=cap
```

Expected:
- every per-change harness of `cp_*` is `SUCCESSFUL`, within SC-006's 10 minutes on the CI
  machine (R0c's laws took under 3 minutes at 3 jobs);
- each harness's documentation names its catalogue id, kind, statement, and stubs; the
  report's statuses follow data-model §4: "proven", "by composition", "under contract";
- a failure prints the harness (named after its id, `cp_gro::p_cpgro_01` for P-CPGRO-01),
  the failed check and a concrete-playback test. Playback of a harness with stubs replays the
  stub values as given (Kani's warning says so); replay such a counterexample natively before
  reading it as a defect (research R0).

Measured (2026-10-25, 4 jobs, at `dd4a551`): 45 of 45 `SUCCESSFUL` in 7 min 39 s of wall time.
The harnesses are 14 crash, 7 lemma and 24 property members, with 1,246.8 s of harness time;
the longest is `cp_eq::p_cpeq_02` at 173.7 s. Every crash harness satisfies its covers: seven
gained theirs at T052 and were re-run. The whole `make verify-values` passes 272 of 272 in
30 min 25 s.

## 4. The weekly members

```sh
make verify-slow F=values P='cp_crash::c_cell_tests__edges_ cp_reg::p_cpreg_01__real cp_gro::p_cpgro_03__real cp_lem::l_cp_05 cp_mea::p_cpmea_01__area cl_cnt::p_ccap_01'
make verify-slow F=values P='cp_con::p_cpcon_01__area cp_mea::p_cpmea_01__centroid cp_lem::l_cp_06a'
make verify-slow F=values P='cp_crash::c_cell_tests__real cp_crash::c_cell_union_bound'
```

These are CI's `values-cap`, `values-limits-f` and `values-limits-g` legs.

Expected:
- L-CP-05, the edge test's crash proofs, P-CPREG-01's real-edge member and P-CPGRO-03's
  real-code member prove (R0: 6 to 12 minutes each);
- `union`'s crash, P-CPBND-03 and P-CPREG-04 prove or end `KNOWN-TIMEOUT`, as recorded in
  `verify/known-timeouts.txt` and the catalogue;
- 006's P-PTDST-01 symmetry member is no longer listed as a known timeout (research R12).

Measured (2026-10-25, 60 minutes, 8 GiB). These prove:

- `p_cpreg_01__real` (1,037.1 s)
- `c_cell_tests__edges_contains` (704.2 s) and `__edges_intersects` (402.0 s)
- `l_cp_05` (551.6 s)
- `p_ccap_01` (341.9 s, 2026-10-24)
- `p_cpgro_03__real` (307.4 s)

`p_cpmea_01__area` first failed in 1.5 s on D-009. Since the fix (`1b26156`) it proves in
1,195.3 s.

These end `KNOWN-TIMEOUT`: `p_cpcon_01__area`, `p_cpmea_01__centroid`, `l_cp_06a`,
`c_cell_tests__real` and `c_cell_union_bound`.

`c_union` runs out of memory at 8 GiB and, alone at 16 GiB, times out. It is in no CI leg.

P-CPREG-04 has no harness: its level needs `asin`'s value. P-PTDST-01 is proven by
composition, and its weekly member is gone.

## 5. Non-vacuity

```sh
make verify-mutants M='M-7*'
```

Expected:
- every `M-7NN` is killed and recorded in `verify/MUTANTS.md` (SC-005), apart from the ones
  data-model §7 expects to survive, recorded with the reason;
- the two stub mutants (`pure_chord` without `(p, p)`, `pure_add` without the zero case) make
  their laws fail: the facts in the stubs are needed;
- K1 to K6 reverted one at a time make their `cap_tests` fail, and K4's and K1's also fail
  P-CPCMP-01 and P-CPBND-02.

Measured (2026-10-25, `VERIFY_SLOW_HARNESS_TIMEOUT=60m`, at `dd4a551`): all 40 `M-701` …
`M-740` killed in 22 min 25 s. Then `M-741` (D-009's fix reverted) and the regenerated `M-708`
were killed too, so 41 of 41. Kani kills 32, `M-740` and `M-741` in the slow tier among them.
Ordinary tests and the C++ vectors kill 9, through the `Test:` headers. None is expected to
survive. `M-730` and `M-731` (the stub facts) make their laws fail.

## 6. The fixes, by hand

```sh
cargo test -p s2rst --lib cap_tests
```

Expected: pass. With a fix reverted, its test fails with the probe's value:
- level 2 for the angle radius 0.11785113019775795 (K1);
- 30 of 65 instead of 38 on K2's grid; K3's cap intersecting face 0;
- complement length² 2.99999999999999956 for length² 1 (K4);
- K5's rectangle `[0, 0]` without the centre;
- K6's pair not approximately equal;
- a NaN centre deserialised (K7);
- after the maintainer's answers (2026-10-25): area −0 for r² = −0 in a debug build (D-009,
  `area_of_a_negative_zero_radius_is_positive_zero`), and no cells for the empty cap's covering
  (K8, `empty_cap_covering_matches_upstream`).

Measured: passes, both in `make test` and in each fix's commit (`7ab5b66` … `80e0ed8`; K2 and
K3 share `a21b7e6`), whose message records the failing value before the fix.

## 7. The records

- `verify/values/INVENTORY.md` §9: 43 entries; 31 crash proofs, 8 under contract (3 of them
  weekly tool limits: `union`, both `cell_union_bound`s), 2 by 003, `Display` excluded,
  `Deserialize` tested.
- `verify/values/PROPERTIES.md` §10: 27 entries with statuses (22 of the 25 counted proven,
  SC-003's 88%), and the 10 lemmas (L-CP-01 to L-CP-08, L-CP-06a, L-CP-06b); `Cell`'s
  P-CCAP-01 (proven by composition) and `Point`'s P-PTDST-01 (now proven by composition)
  updated.
- `verify/TRUSTED_BASE.md`: the three contracts with their tests, the lemma stubs with their
  lemmas, the purity argument, and the Kani compiler crash with `thread_local!`.
- `verify/PROPERTIES_BY_KIND.md` and its HTML render: every new id exactly once
  (`cargo test property_kinds`).
- `VERIFIED_TYPES.md`: `Cap` has section 2.9; §3's `Region`, `PartialEq` and codec rows name
  `Cap`'s instances.
- `CORE_TRAITS.md`: the `Region` invariants name their proven instances.
- `verify/values/EVALUATION.md`, spec-008 part: what is proven and how, the lemma stubs' cost
  and gain, the next `Region` implementors, the go or no-go for `LatLng`, ranked follow-ups.
