# Contract: Verification Commands (changes for `CellUnion`)

001's contract (`specs/001-formal-verify-base-type/contracts/verify-commands.md`) still
applies. This file lists only what changes. CI still calls the same Makefile targets as a
local run (Constitution, Development Workflow).

## Suite selection (research R8)

The `verify/` crate gains a **default** Cargo feature, `cellunion`. It gates every
`CellUnion` harness module (`cu_*`) and its generators. The 001 modules stay ungated.

- With default features, a build contains both suites.
- `--no-default-features` builds the `CellId` suite alone.
- `--harness cu_` selects the `CellUnion` suite alone. No 001 harness path contains `cu_`
  (checked 2026-10-02).

The feature gate is needed because substring filters cannot pick out the 001 suite.
`--harness crash::` would also match `cu_crash::`.

## `make verify` (changed scope)

Runs both suites' per-change proofs.

| Aspect | Contract |
|--------|----------|
| Steps | 1. `cargo test` in `verify/`: sanity tests of both reference models against the C++ vectors (`spec.rs` and the new `union_spec.rs`). 2. `run-verify.sh`: every per-change harness of both suites |
| Output, exit code, memory guard, playback | Unchanged from 001, except that CBMC's own out-of-memory exit (status 6, "Out of memory" before the solver) is now reported as `OOM` instead of `ERROR` (research R18) |
| Duration | The `CellId` share stays under 15 min on the 4-vCPU / 16 GB reference runner, and the `CellUnion` share under 10 min (SC-005). Together, under 25 min |

## `make verify-cellid` (new)

`cargo test` in `verify/`, then `run-verify.sh --no-default-features`. This is exactly
001's `make verify`. CI uses it for the `CellId` leg.

## `make verify-union` (new, FR-011)

`cargo test` in `verify/`, then `run-verify.sh --harness cu_`. These are the `CellUnion`
proofs only, under the same memory guard, output format and exit-code rules. CI uses it
for the `CellUnion` leg. Duration: under 10 min on the reference runner (SC-005).

## `make verify-harness H=<filter>` (unchanged)

`H=cu_set::p_uset_04` works like any 001 name.

## `make verify-slow [F=<0-5|vertex|union>]` (one new part)

`F=union` runs the `CellUnion` harnesses behind the `slow` feature, if any harness ends up
over the per-change budget (research R4). The existing parts are unchanged. The weekly
matrix gains a `union` leg only if that set is non-empty.

## `make verify-mutants [M="<id> …"]` (unchanged interface)

It now also runs the `M-1NN` patches against `core/src/s2/cell_union.rs` and
`core/src/s2/encoding.rs`. The kill rule, worktree handling and `MUTANTS.md` record are
unchanged. The campaign gains about 25 mutants. Each runs one or two `cu_*` harnesses,
so the expected cost is 1–3 min per mutant (research R7).

## `make test` (unchanged definition)

It gains no Kani dependency. It does run the new ordinary tests, because they live in
`core/` and `core/tests/`:

- the FR-015 and FR-017 regression tests in `core/src/s2/cell_union_tests.rs`;
- the differential tests `core/tests/cpp_cellunion_diff.rs`;
- the updated `cellunion` robustness tests in `core/tests/decoder_robustness.rs`.

## CI: `.github/workflows/verify.yml`

| Job | Trigger | Runs |
|-----|---------|------|
| `verify` | As in 001, with these paths added: `core/src/s2/cell_union.rs`, `core/src/s2/encoding.rs`, `core/src/s2/cell.rs` (the `Region` cell-query proofs build a `Cell`), `core/tests/data/cpp_cellunion_vectors.csv` | A matrix, `suite: [cellid, union]`, running `make verify-cellid` and `make verify-union` as parallel legs. Each leg has a 30-minute job limit |
| `slow` | Unchanged; adds a `union` leg only if research R4 puts any harness there | `make verify-slow F=<part>` |
| `mutants` | Unchanged | `make verify-mutants`. The job's time limit rises from 120 to 180 minutes to fit the M-1NN mutants |

Both legs run on every matching change, so a `CellId` change also re-checks `CellUnion`,
which is built on it.

## CI: `.github/workflows/fuzz.yml`

No new target: `decode_cell_union` already exists (FR-018). Its round-trip assertion still
holds after FR-017. A decoded union is valid, so encoding it and decoding it again gives
the same identifiers.
