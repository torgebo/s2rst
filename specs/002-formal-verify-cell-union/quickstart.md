# Quickstart: Validating the `CellUnion` Verification

This is a run guide that shows the feature working end to end. The command interfaces are
in [contracts/verify-commands.md](contracts/verify-commands.md). Naming, generators and
patch formats are in [contracts/harness-conventions.md](contracts/harness-conventions.md).
The prerequisites and memory advice are 001's
(`specs/001-formal-verify-base-type/quickstart.md` §0):
- Kani 0.68.0 is required.
- Always go through the Makefile targets, which apply the memory guard.
- Run one verification job at a time.

## 1. The `CellUnion` proofs (US1, US2, SC-001, SC-003, SC-005)

```sh
make verify-union
```

Expected result:
- The model's sanity tests pass. They check `verify/src/union_spec.rs`, and 001's
  `spec.rs`, against the C++ vectors.
- Every `cu_*` harness line reads `SUCCESSFUL` with all covers satisfied (120 per change).
- Exit code `0`, in under 10 minutes on a 4-vCPU machine (7.9 minutes on the workstation
  pinned to 4 CPUs).

To cross-check, confirm three things:
- every `in-scope` row of `verify/cell_union/INVENTORY.md` names a harness or `__` family
  that appears in the output;
- each row's `bounds` matches the family's largest member;
- every `proven*` row of `verify/cell_union/PROPERTIES.md` names a harness in the output.

## 1a. The weekly tier

```sh
make verify-slow F=union
```

Expected: the whole `CellUnion` suite with its slow-tier members (the larger property
members and the 25-byte decode crash proof), at an 8 GB ceiling and one harness at a time,
all `SUCCESSFUL` (165 harnesses). It took 49 minutes on the workstation.

## 2. Both suites, and each alone

```sh
make verify          # CellId + CellUnion per-change proofs
make verify-cellid   # exactly 001's suite (--no-default-features)
```

Expected: `make verify-cellid` reports the same 196 harnesses as 001's last run, all
`SUCCESSFUL`. `make verify` reports those 196 plus the `cu_*` harnesses.

## 3. One property

```sh
make verify-harness H=cu_mem::p_umem_01__n3
```

## 4. Non-vacuity (US2 scenario 2, FR-008, SC-004)

```sh
make verify-mutants M="M-100 M-150 M-154"   # the overflow demo, the FR-015 reversal, a decode mutant
make verify-mutants                          # everything, 001's mutants included
```

Expected: each mutant is `killed`, and `verify/MUTANTS.md` is updated. `M-100`'s run
prints a concrete-playback test whose union holds two face cells (US1 scenario 4). `M-150`
and `M-154` carry `Tier: slow`, because their expected harnesses are weekly members. The
planned FR-017 reversal, `M-151`, was dropped: only a two-cell decode can kill it, and that
runs out of memory (`verify/cell_union/EVALUATION.md`, "Limitations").

## 5. Regression detection (US1 scenario 4, US3)

Apply `verify/mutants/M-100-*.patch` by hand. Run
`make verify-harness H=cu_crash::c_leaf_cells_covered__n2`. Expected: `FAILED`, with the
overflow check named and a concrete counterexample printed. Revert, and the harness
passes again.

## 6. FR-015: denormalise clamp (SC-008)

```sh
cargo test -p s2rst cell_union_tests::denormalize      # regression tests
cargo test -p s2rst --test cpp_cellunion_diff denormalize
```

Expected: both pass. With the fix reverted (`git stash` the one-line change), the
regression test fails with `level must be 0..=30`.

## 7. FR-017: decode verbatim, reject invalid unions (SC-010)

```sh
cargo test -p s2rst cell_union_tests::decode
cargo test -p s2rst --test cpp_cellunion_diff decode
cargo test -p s2rst --test decoder_robustness
cd python && uv run pytest tests/test_encoding.py
cd wasm && wasm-pack test --node
```

Expected: all pass. With the fix reverted, the "four siblings round-trip unchanged" test
fails, because the decoded union holds one parent cell instead of four. The message tests
confirm that each decoder error displays the same text as before. The text is now
formatted only when displayed (research R11).

The fuzz target is unchanged and still holds. Optionally, smoke-run it:

```sh
cd fuzz && cargo +nightly fuzz run decode_cell_union -- -max_total_time=60
```

## 8. FR-016 and `# Panics` documentation (SC-009, SC-011)

```sh
make doc
grep -n '# Panics' core/src/s2/cell_union.rs
```

Expected: the docs build. `leaf_cells_covered` documents the valid-union precondition, its
panic and its signed result type. `from_verbatim` documents the valid-union requirement.
Every operation the proofs show can panic has a `# Panics` section, matching the list in
`verify/cell_union/EVALUATION.md`.

## 9. C++ vectors (Principle I)

Only needed to regenerate. CI never runs it:

```sh
cd tools/refgen-cpp
cmake -B build -DCMAKE_BUILD_TYPE=Release -DCMAKE_PREFIX_PATH=/tmp/s2-install
cmake --build build --target generate_cellunion
./build/generate_cellunion > ../../core/tests/data/cpp_cellunion_vectors.csv
git diff --stat ../../core/tests/data/cpp_cellunion_vectors.csv   # no change on a rerun
```

## 10. Unchanged gate (SC-006)

```sh
make test
```

Expected: it passes. It runs the new regression, differential and robustness tests. It
never runs Kani.

## 11. Artifacts to review (FR-002, FR-005, FR-013, FR-014, FR-019)

- `verify/cell_union/INVENTORY.md`: 42 rows, 27 in scope, each with its bounds. The
  five tool-limit rows carry their measurements.
- `verify/cell_union/PROPERTIES.md`: 27 or more properties, with status and bound. The
  four "not proven" rows name the property tests and C++ vectors that check them instead.
- `verify/TRUSTED_BASE.md`, `CellUnion` section: reliance on 001, the bounds, and the
  written arguments.
- `verify/cell_union/EVALUATION.md`: the bounds reached, the comparison with 001, the
  collections verdict, the defects, the documentation added, and the upstream findings
  (U-003).
- `specs/002-formal-verify-cell-union/tasks.md`: the follow-up task for each `U-NNN`.
