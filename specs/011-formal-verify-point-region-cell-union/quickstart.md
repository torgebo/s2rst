# Quickstart: validating spec 011 end to end

Every Kani command runs through the Makefile, which uses the memory-guarded
`verify/run-verify.sh`; run them one at a time. Expected values are the plan's until the final
run replaces them with measured ones.

## 0. Prerequisites

- Kani 0.68.0 (`make verify-kani-check`), Rust stable, Python 3 (for the probe's comparison).
- For regenerating the C++ vectors only: the s2geometry release build in
  `/home/t/sources/s2geometry/build` (`tools/refgen-cpp/README.md`).

## 1. The library changes and the gate

```sh
make test
```

Expected: passes. It includes the `cell` tests (FR-017, the polar faces' bounds),
`cell_union_tests` (FR-018, the empty union's areas; FR-019, `expand_by_radius`'s NaN panic in
debug builds), the `point_region` tests (FR-019, the deserialiser), `cpp_region_diff` (every
row, no deviation) and the trait property tests with the two new laws' twins.

Red first: each fix's test fails on the commit before it (`git stash` the fix, run the test,
see it fail, restore).

## 2. The probe again (SC-007)

```sh
cargo run --release -q --manifest-path specs/011-formal-verify-point-region-cell-union/probe/rust/Cargo.toml \
  > specs/011-formal-verify-point-region-cell-union/probe/rust/survey_rust.txt
python3 -I specs/011-formal-verify-point-region-cell-union/probe/compare.py \
  specs/011-formal-verify-point-region-cell-union/probe/rust/survey_rust.txt \
  specs/011-formal-verify-point-region-cell-union/probe/cpp/survey_cpp.txt
```

Expected: 0 differences in every section (the C++ output is regenerated as in
`probe/README.md`).

## 3. The proofs

```sh
make verify-values P=regions
make verify-slow F=values P='<the weekly members, research R0>'
make verify-release
```

Expected: every `pr_*` and `uf_*` harness `SUCCESSFUL` per change in under 10 minutes; the
weekly members `SUCCESSFUL`; the release pass includes the new `n_`/`o_` members.

## 4. Non-vacuity

```sh
make verify-mutants M='M-95* M-96* M-97*'
```

Expected: every mutant killed (`verify/MUTANTS.md`).

## 5. Records

```sh
cd verify && cargo test --test property_kinds
```

Expected: passes: every catalogue entry appears once in `PROPERTIES_BY_KIND.md` (SC-011).
Then read `verify/values/EVALUATION.md`'s spec-011 part and check that it answers SC-013's
three questions.
