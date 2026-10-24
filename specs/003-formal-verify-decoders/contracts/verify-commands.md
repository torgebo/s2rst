# Contract: Verification Commands (changes for the decoding layer)

The 001 and 002 contracts (`specs/001-formal-verify-base-type/contracts/verify-commands.md`,
`specs/002-formal-verify-cell-union/contracts/verify-commands.md`) still apply. This file
lists only what changes. CI still calls the same Makefile targets as a local run
(Constitution, Development Workflow).

## Suite selection (research R2)

The `verify/` crate gains a second **default** Cargo feature, `decoding`. It gates every
decoding harness module (`cd_*`), their generators and the codec contract. The 001 modules
stay ungated, and the 002 modules keep their `cellunion` gate.

- With default features, a build contains all three suites.
- `--no-default-features` still builds the `CellId` suite alone.
- `--harness cu_` still selects the `CellUnion` suite alone.
- `--harness cd_` selects the decoding suite alone. No 001 or 002 harness path contains
  `cd_` (checked 2026-10-04).

## `make verify` (changed scope)

It runs all three suites' per-change proofs.

| Aspect | Contract |
|--------|----------|
| Steps | 1. `cargo test` in `verify/`: the sanity tests of the three reference descriptions against the C++ vectors (`spec.rs`, `union_spec.rs` and the new `wire_spec.rs`), and the reservation measurement (research R7). 2. `cargo test` in `verify/deductive/`: the tested agreement and the drift guard (research R11, R12). 3. `run-verify.sh`: every per-change harness |
| Output, exit code, memory guard, playback | Unchanged from 002 |
| Duration | `CellId` under 15 min, `CellUnion` under 10 min, decoding under 10 min on the reference runner (SC-006) |

## `make verify-decoding` (new, FR-015)

The decoding proofs alone. It runs `cargo test` in `verify/`, then `cargo test` in
`verify/deductive/`, then `run-verify.sh --harness cd_`. Those are the bounded proofs,
including the bounded agreement harnesses of FR-011. Output format, memory guard and
exit-code rules are those of `run-verify.sh`. CI uses it for the decoding leg.

Duration: under 10 min on the reference runner (SC-006).

## `make verify-deductive` (new, FR-010, FR-012)

The deductive track. It needs the pinned Verus release and its Rust toolchain
(`make verify-verus-check` checks both). It does not need Kani.

| Aspect | Contract |
|--------|----------|
| Steps | 1. `verify/deductive/run-verus.sh`: Verus on the restated crate (`src/lib.rs`, `--crate-type=lib`), once, under the memory guard. 2. The same crate's `cargo test`: tested agreement and drift guard |
| Locating Verus | `VERUS=<path to the verus binary>`, else `verus` on `PATH`. The script checks the reported version against `VERUS_VERSION` in the Makefile and fails on a mismatch |
| Memory guard | `ulimit -v` at `VERUS_MEM` (default 4G) inside a `systemd-run --user --scope` cgroup when available. Verus peaked at 0.33 GB in research R0 |
| Output | One line per verified module: `<module>  <verified> verified, <errors> errors  <seconds> s  <peak MiB>`. Then Verus's own summary line. On failure, Verus's error report is printed unchanged |
| Exit code | 0 only if every module verifies with 0 errors and both tests pass |
| Duration | Under 10 min on the reference runner, with the toolchain restored from cache (SC-005). Research R0 measured 3.4 s for the probe |

## `make verify-verus-check` (new)

It checks that `$(VERUS)` (or `verus` on `PATH`) reports `VERUS_VERSION`
(`0.2026.09.20.aef82ed`) and that its toolchain (`1.98.1`) is installed. On failure it
prints how to install both.

## `make verify-deductive-bless` (new, FR-011)

After a reviewer has checked that a restated copy still mirrors a changed library function,
this target rewrites the drift guard's fingerprints (`verify/deductive/mirrors.toml`). It
prints every fingerprint it changed. It never runs in CI.

## `make verify-mutation-measure [PHASE=before|after]` (new, FR-018)

The tool-generated mutation measurement. It runs on a workstation, not in CI.

| Aspect | Contract |
|--------|----------|
| Steps | 1. `cargo mutants` (pinned version, research R14) over the in-scope source files, against the selected test suite. 2. The surviving mutants' diffs, exported as patches. 3. Each surviving patch run against the `cd_*` harnesses through `run-mutants.sh`. 4. A classification table written to `verify/decoding/MUTATION.md` under the given `PHASE` |
| Memory guard | `run-mutants.sh` rules (one throwaway worktree per mutant; `run-verify.sh` caps) |
| Duration | Hours (research R14). Not part of any CI job |

## `make verify-harness H=<filter>`, `make verify-slow`, `make verify-mutants` (unchanged interfaces)

- `H=cd_uv::p_uv_03__b5` works like any 001 name.
- `make verify-slow` gains a part `F=decoding` only if research R5 puts a decoding harness
  in the weekly tier.
- `make verify-mutants` also runs the `M-2NN` patches, against the in-scope decoding files.

## `make verify-decoding [P=vectors|links|codecs]` (amended 2026-10-06)

The whole decoding suite took 21 minutes per change on the workstation (2,191 s of harness
time), over SC-006's 10 minutes. CI therefore runs it as three parallel legs: `P=vectors`
(`cd_fw`, `cd_uv`), `P=links` (`cd_agree`, `cd_sv`) and `P=codecs` (`cd_cv`, `cd_pv`, `cd_fx`,
`cd_crash`). They are balanced on harness time plus Kani's set-up of about 5 s per harness,
which dominates the many small fixed-size members. Without `P`, the target runs all of them.

## `make verify-xlarge [H=<module>::<fn>] [XLARGE_MEM=<n>G]` (new, 2026-10-06)

Added at the user's request during implementation. It runs the members that are over the
weekly budget on a separate machine with 32 GB of memory.

| Aspect | Contract |
|--------|----------|
| Harnesses | `--features xlarge`: the `cd_xlarge` module, which holds the T029 tool limits (the string vector decoder from 1 byte, the point vector decoder from 2 bytes). `H=` selects one by its full name |
| Memory guard | `run-verify.sh` with `VERIFY_MEM_PER_HARNESS=$(XLARGE_MEM)` (default `26G`) and `VERIFY_JOBS=1`. The cgroup backstop and the playback ceiling stay below the available memory, as for every target. A preflight refuses to start unless the ceiling plus 2 GiB is available |
| Time | `VERIFY_HARNESS_TIMEOUT` defaults to `4h` per harness |
| Logs | `verify/target/run-xlarge/` |
| CI | None. Results are recorded by hand in `decoding/EVALUATION.md` ("Bounds measured first"), with the machine, the ceiling and the time |

## `make test` (unchanged definition)

It gains no Kani or Verus dependency. It does run the new ordinary tests, because they live
in `core/` and `core/tests/`:

- the FR-019 regression tests (research R7);
- the new sibling `<module>_tests.rs` files of the coding modules, with property tests for
  sizes the proofs do not reach (research R13);
- the differential tests `core/tests/cpp_coding_diff.rs` (research R10);
- the updated robustness tests in `core/tests/decoder_robustness.rs`.

## CI: `.github/workflows/verify.yml`

| Job | Trigger | Runs |
|-----|---------|------|
| `verify` | As in 002, with these paths added: `core/src/s2/encoded_uint_vector.rs`, `core/src/s2/encoded_s2cell_id_vector.rs`, `core/src/s2/encoded_string_vector.rs`, `core/src/s2/encoded_s2point_vector.rs`, `core/src/s2/cap.rs`, `core/src/s2/rect.rs`, `core/src/s2/point.rs`, `core/src/s1/chord_angle.rs`, `core/src/r1/interval.rs`, `core/src/s1/interval.rs`, `core/tests/data/cpp_coding_vectors.csv` | The matrix becomes `suite: [cellid, union, decoding]`. Each leg runs `make verify-<suite>` and has a 30-minute job limit |
| `deductive` (new) | The same path filter, plus the weekly schedule | Restores (or installs) the pinned Verus release and Rust 1.98.1, then runs `make verify-deductive`. 20-minute job limit |
| `slow` | Unchanged; adds a `decoding` leg only if research R5 puts a harness there | `make verify-slow F=<part>` |
| `mutants` | Unchanged | `make verify-mutants`, including the `M-2NN` patches. The time limit is revisited once their cost is known (research R15) |

## CI: `.github/workflows/fuzz.yml`

Three new targets join the nightly matrix (FR-023, research R9): `decode_cell_id`,
`decode_point` and `decode_rect`. The existing `decode_uint_vector_u32`,
`decode_uint_vector_u64`, `decode_s2cell_id_vector`, `decode_string_vector`,
`decode_s2point_vector`, `decode_cap` and `decode_cell` targets keep their assertions, and
are updated where FR-019 changes behaviour.
