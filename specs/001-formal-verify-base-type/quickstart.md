# Quickstart: Validating the `CellId` Verification Pilot

This is a run guide that shows the feature working end to end. The command interfaces are
specified in [contracts/verify-commands.md](contracts/verify-commands.md), and naming and
patch formats in [contracts/harness-conventions.md](contracts/harness-conventions.md).

## 0. Prerequisites (one-time)

```sh
cargo install --locked kani-verifier --version 0.68.0
cargo kani setup            # downloads Kani's pinned nightly toolchain + CBMC
cargo kani --version        # expect: Kani Rust Verifier 0.68.0, CBMC 6.11.0
```

Linux x86_64 or macOS is required. The `verify/` crate is a standalone workspace, so nothing
here affects `cargo build` or `make test` at the repository root.

**Memory.** A single CBMC run can use several GB, and a naive text harness was measured
exhausting 6 GB within 20 seconds (research R6). Always go through the Makefile targets,
which apply the memory guard (research R15). Don't run `cargo kani` directly on the whole
crate. On a machine with less than 16 GB, lower the parallelism: `make verify VERIFY_JOBS=2`.

## 1. Full proof run (US1, US2, SC-001, SC-003, SC-005)

```sh
make verify
```

Expected result:

- The reference-model sanity tests pass. They check `verify/src/spec.rs` against
  `core/tests/data/cpp_cellid_vectors.csv`.
- Every harness line reads `SUCCESSFUL` with `covers N/N`, or `KNOWN-DEFECT D-NNN` for a
  harness listed in `verify/known-defects.txt`. No line reads `FAILED` or `OOM`.
- The exit code is `0`, and the run takes under 15 minutes on a 4-vCPU machine.

To cross-check coverage, confirm that every `in-scope` row in `verify/INVENTORY.md` names a
`crash::c_*` harness (or `__` split family) that appears in the output. Also confirm that
every `proven` row in `verify/PROPERTIES.md` names a harness that appears in the output. The
exceptions are the rows marked "weekly slow tier": the neighbour proofs, which are not part
of `make verify`:

```sh
make verify-slow               # all of it: about 21 CPU-hours, up to 30 min per harness
make verify-slow F=2           # one face's neighbour proofs; F=vertex for vertex_neighbors
```

## 2. One property (fast iteration)

```sh
make verify-harness H=hier::p_hier_03
```

Expected: exactly one harness runs and reports `SUCCESSFUL` with all covers satisfied.

## 3. Non-vacuity campaign (US2 scenario 2, FR-008, SC-004)

```sh
make verify-mutants            # all mutants: seconds each, up to 30 min for `Tier: slow`
make verify-mutants M=M-001    # a single mutant
```

Expected:

- Every line reads `M-NNN killed`.
- `verify/MUTANTS.md` is updated with the outcomes.
- The exit code is `0`.
- `git status` shows only `verify/MUTANTS.md` modified. The worktrees are temporary and are
  removed.

## 4. Regression detection and counterexample (US1 scenario 3, US3)

Apply the overflow mutant by hand to see what a contributor would see:

```sh
git apply verify/mutants/M-001-advance-unclamped.patch
make verify-harness H=crash::c_advance    # expect: FAILED, names the harness and the
                                          #         overflow check, then prints the
                                          #         concrete-playback #[test] with the inputs
git apply -R verify/mutants/M-001-advance-unclamped.patch
```

`run-verify.sh` prints the concrete counterexample itself, so the CI log shows the same
thing (FR-007, US3; research R10).

When verification finds a real defect, the printed test is the starting point for the
regression test. That test goes into `core/src/s2/cell_id_tests.rs` in a separate fix change
(FR-010, Constitution III).

## 5. Token parity fix (FR-015, SC-008)

```sh
cd core && cargo test --lib s2::cell_id::cell_id_tests && cd ..
cargo test -p s2rst --test cpp_cellid_diff
```

Expected:

- The tests pass, including `CellId::from_token("+1") == CellId::none()`, as in C++ and Go.
- The differential tests for `CELLID_FROM_TOKEN` and `CELLID_FROM_DEBUG_STRING` pass against
  the committed C++ vectors (research R16).
- `git log -p -- core/tests/data/cpp_cellid_vectors.csv` shows only appended lines, so the
  existing sections are unchanged.

To confirm that the test fails without the fix, temporarily remove the hex-digit check in
`CellId::from_token`, rerun the command, and expect the `+1` test to fail. Then restore the
check.

## 6. Debug-string parity fix (FR-016, SC-009)

The same commands as §5 also run the FR-016 tests and the `CELLID_TO_STRING` differential
test. Expected: `CellId(2).to_debug_string() == "Invalid: 0000000000000002"`,
`CellId(2).to_string()` gives the same text, and every C++ `ToString` vector matches.

To confirm that both tests fail without the fix, temporarily change the format string back
to `"Invalid: {:x}"`, expect both tests to fail, and restore it.

## 7. Untrusted-input coverage (FR-017, SC-010)

```sh
cd core && cargo test --test decoder_robustness text_parsers && cd ..
cargo +nightly fuzz run parse_cell_id_text \
    fuzz/corpus/parse_cell_id_text fuzz/seeds/parse_cell_id_text -- -max_total_time=60
```

Expected:

- The robustness tests pass, with no panic on empty, over-long, sign-prefixed, non-hex or
  non-ASCII input.
- The fuzz run finishes its 60 seconds without a crash.
- `grep parse_cell_id_text .github/workflows/fuzz.yml` finds the new matrix entry.

## 8. `# Panics` documentation and upstream findings (SC-011, SC-012)

```sh
make doc                                     # must build with warnings denied
grep -c "# Panics" core/src/s2/cell_id.rs    # one per row with a panics_doc in INVENTORY.md
```

Expected:

- Every row in `verify/INVENTORY.md` with a `panics_doc` entry has a matching `# Panics`
  section, and the docs commit changes nothing but rustdoc (research R17).
- The "Follow-ups" section of `tasks.md` holds one task per `U-NNN`, starting with U-001
  (C++ `level()`) and U-002 (C++ `GetCommonAncestorLevel`).
- `verify/EVALUATION.md` lists each finding under "Upstream findings" (FR-018; research R18).

## 9. Unchanged gate (SC-006)

```sh
make test                      # must not invoke Kani; see the note below
git diff --stat master -- core fuzz tools .github/workflows/fuzz.yml
```

The diff shows only these files:

- `core/src/s2/cell_id.rs`: the FR-015 and FR-016 fixes, the test-module wiring, and the
  `# Panics` docs;
- `core/src/s2/cell_id_tests.rs`: new;
- `core/tests/cpp_cellid_diff.rs`: the three new differential tests;
- `core/tests/data/cpp_cellid_vectors.csv`: three appended sections;
- `tools/refgen-cpp/generate_cellid.cc` and its `README.md`: the new sections;
- `core/tests/decoder_robustness.rs`: the FR-017 section;
- the FR-017 fuzz target, its `[[bin]]` entry, its seeds, and its matrix entry;
- `fuzz/Cargo.lock`: one line, the `s2rst` version (`0.3.2` → `0.4.0`), which was already
  stale on `master` and was refreshed when the fuzz crate was built.

`core/Cargo.toml` is unchanged.

**Known gate drift (2026-10-01):** `make test` fails on this branch for reasons that
predate the feature: clippy 1.98.1 lints in files the feature does not touch, and two
`cargo-deny` advisories against pyo3 0.24.2 (`verify/EVALUATION.md`). Every other stage
passes. SC-006 is judged on "no new failures, and no Kani in the gate".

## 10. Artifacts to review (FR-002, FR-005, FR-013, FR-014)

| File | Check |
|------|-------|
| `verify/INVENTORY.md` | All 64 FR-002 items are listed. Excluded rows have reasons. The in-scope share is at least 75%. Every in-scope row has a crash harness, satisfied covers and at least one `P-*` property |
| `verify/PROPERTIES.md` | Every FR-005 area, plus `NBR`, is present. Each row has an upstream source, a harness, a status, and at least one killing mutant |
| `verify/TRUSTED_BASE.md` | Tool and solver versions, the floating-point semantics relied on, bounds, tool-limit exclusions and not-proven items |
| `verify/EVALUATION.md` | A go/no-go recommendation, an assurance-level recommendation, ranked follow-up types, the defects found (FR-015, FR-016 and any `D-NNN`), the `# Panics` docs added, the upstream findings, and an SC-001…SC-012 acceptance check (SC-005 on the reference runner, and on the actual runner if that is smaller) |
