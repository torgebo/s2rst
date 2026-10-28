# Quickstart: validating spec 010 end to end

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

Expected: passes. It includes `latlng_tests` (FR-017, FR-018), `rect_tests` (FR-019, the
boundary sweep), `angle_tests` (the shared remainder), `cpp_latlng_diff` (every row; the
`RECT_CAPBOUND` radii as the documented deviation) and the trait property tests.

Red first: each fix's test fails on the commit before it (`git stash` the fix, run
`cargo test -p s2rst latlng_tests`, see it fail, restore).

## 2. The probe again (SC-007)

```sh
cargo run --release -q --manifest-path specs/010-formal-verify-latlng/probe/rust/Cargo.toml \
  > specs/010-formal-verify-latlng/probe/rust/survey_rust.txt
python3 -I specs/010-formal-verify-latlng/probe/compare.py \
  specs/010-formal-verify-latlng/probe/rust/survey_rust.txt \
  specs/010-formal-verify-latlng/probe/cpp/survey_cpp.txt
cargo run --release -q --manifest-path specs/010-formal-verify-latlng/probe/rust/Cargo.toml \
  --bin capbound 100000
```

Expected: 0 differences in every section but G, whose differences are the widened radii only
(the probe prints the bound's membership bits, so G's rows match where upstream's cap already
contained the point); the library's G misses are 0; `capbound` reports 0 misses for the
library's bound. (The C++ output is regenerated as in `probe/README.md`.)

## 3. The proofs

```sh
make verify-values P=latlng
make verify-values P=rect
```

Expected: every `ll_*` harness and `rl_reg`'s three `SUCCESSFUL`, every cover satisfied, in
under 10 minutes each with 4 jobs (SC-006); `rect`'s existing harnesses unchanged.

## 4. Non-vacuity

```sh
make verify-mutants M='M-9*'
```

Expected: every `M-9NN` killed (`verify/MUTANTS.md`).

## 5. Records

```sh
cd verify && cargo test --test property_kinds
```

Expected: passes: every catalogue entry appears once in `PROPERTIES_BY_KIND.md` (SC-011).
Then read `verify/values/EVALUATION.md`'s spec-010 part and check that it answers SC-013's
three questions.
