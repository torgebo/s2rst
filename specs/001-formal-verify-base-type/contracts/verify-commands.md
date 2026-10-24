# Contract: Verification Commands

These are the command-line interfaces that maintainers and CI call (FR-011, FR-012). CI calls
the **same Makefile targets**, as the constitution requires (Development Workflow: "The CI
`test` workflow MUST call the same Makefile targets").

## `make verify`

Runs the full proof suite for `CellId`.

| Aspect | Contract |
|--------|----------|
| Prerequisites | `cargo-kani` at the pinned version (research R1) on `PATH`. The target fails fast with an install hint if it is missing or at the wrong version |
| Steps | 1. `cargo test` in `verify/`: sanity-tests the reference model against the committed C++ vectors. 2. `verify/run-verify.sh`: builds the harnesses once (`cargo kani --only-codegen`), then runs every harness with `cargo kani -j $VERIFY_JOBS --output-format terse` under the memory guard below. 3. For each `FAILED` harness that is not a known defect, it re-runs that harness under the guard with `-Z concrete-playback --concrete-playback=print --exact --harness <h>`, alone and with its own ceiling, `VERIFY_PLAYBACK_MEM` (default 3 × the per-harness ceiling, at most `MemAvailable` − 2 GiB): Kani's driver holds the whole trace, and on the neighbour harnesses it ran out of memory at 4 GiB. If the re-run produces no test, the script says so, with the log path, and suggests raising `VERIFY_PLAYBACK_MEM` when the log shows an out-of-memory error |
| Output | One line per harness, `<module>::<fn>  SUCCESSFUL \| FAILED \| OOM \| TIMEOUT \| ERROR \| KNOWN-DEFECT D-NNN  covers M/N  <t>s`. `TIMEOUT` means the harness hit `VERIFY_HARNESS_TIMEOUT` (default `12m`, the R15 hard limit on CI; use `8m` on a workstation), enforced with Kani's unstable `--harness-timeout`. `ERROR` is a tool failure without a failed check that is not an OOM. In terse `-j` output both OOM and ERROR show as `FAILED` with `0 of N failed`, so the script re-runs such a harness alone, in regular output, to tell them apart. Then one roll-up line per split family (`<family>__*  n/n SUCCESSFUL`), and a summary line with the counts (research R10) |
| On failure | Kani's terse output names the failing check and its location, but not the inputs (checked on 0.68.0). Step 3 prints the concrete counterexample as a generated `#[test]`, whose `concrete_vals` carry decoded comments such as `// 7ul` (FR-007, US3; research R10) |
| Exit code | `0` only if every harness is `SUCCESSFUL` or a listed `KNOWN-DEFECT`, and every harness reports all its cover properties satisfied. Kani 0.68.0 exits `0` on an unsatisfied cover, so `run-verify.sh` checks the `M of N cover properties satisfied` line itself (research R7). A harness listed in `verify/known-defects.txt` that now succeeds is an error ("stale known-defect entry", research R9). `OOM` is a failure |
| Memory guard | Each harness has a memory ceiling, `VERIFY_MEM_PER_HARNESS` (default `4G`; research R15). After an uncapped codegen step, the proofs run under `ulimit -v <ceiling>`, which caps each process. A harness that hits the ceiling prints `Solver ran out of memory…` and is reported as `OOM`. Where `systemd-run --user` works, the run is also wrapped in `systemd-run --user --scope -p MemoryMax=<jobs × ceiling + 2G> -p MemorySwapMax=0` as a backstop. If the backstop is hit, the run ends with "memory backstop hit". Off Linux, the script warns that no guard is active |
| Parallelism | `VERIFY_JOBS`. The default is `min(nproc, ⌊MemAvailable / (7/8 · VERIFY_MEM_PER_HARNESS)⌋)` (3.5 GiB per job at the default ceiling), at least 1, and the cgroup backstop is capped at `MemAvailable − 1 GiB` (research R15). That is 4 on the 4-vCPU / 16 GB reference runner (SC-005), and fewer on smaller machines (research R15) |
| Duration | < 15 min on the 4-vCPU / 16 GB reference runner (SC-005) |
| Side effects | Writes only under `verify/target/` |

## `make verify-slow [F=<0-5|vertex>]`

Runs the weekly slow tier: the harnesses behind the `verify/` Cargo feature `slow` (the
neighbour properties in `nbr`, and `crash::c_vertex_neighbors`), through `run-verify.sh
--features slow`, with `VERIFY_HARNESS_TIMEOUT` defaulting to `30m`. `F=<face>` runs one
face's neighbour properties, and `F=vertex` the vertex crash proof; CI runs the seven parts
as a matrix, with `VERIFY_JOBS=2`. Output, exit code and
memory guard follow `make verify`. It is not part of `make verify` (spec FR-012, SC-005 as
amended 2026-10-01).

## `make verify-harness H=<filter>`

Runs a subset: `run-verify.sh --features slow --exact --harness <filter>` in `verify/`.
`<filter>` is a fully qualified harness name at the crate root, for example
`crash::c_parent` (research R2). The `slow` feature makes the slow-tier harnesses reachable
too; set `VERIFY_HARNESS_TIMEOUT=30m` for those. The output, exit code and memory guard
follow `make verify`. This is for iterating on one property.

## `make verify-mutants [M="<id> …"]`

Runs the non-vacuity campaign (FR-008). It is **not** part of `make verify` or per-PR CI.

| Aspect | Contract |
|--------|----------|
| Steps | For each `verify/mutants/M-*.patch` (or only the ids listed in `M`): create a temporary `git worktree` at `HEAD`, copy the caller's `verify/` into it (so uncommitted harnesses and patches are tested), apply the patch, run `run-verify.sh --exact --harness <h>` for every `Expect-Fail` harness (with `--features slow` and the 30-minute limit for a `Tier: slow` mutant), then remove the worktree |
| Killed | Every expected harness ends `FAILED`, with or without unsatisfied covers (a failed check usually leaves the covers undetermined) |
| Output | One line per mutant: `M-NNN killed \| survived \| error`, then a summary |
| Record | Rewrites the outcome column of `verify/MUTANTS.md` |
| Exit code | `0` only if every mutant is `killed`. An expected harness that ends in `OOM` does not count as killing its mutant |
| Safety | Never modifies the caller's working tree. The worktree is always removed, including on failure or interrupt. The memory guard of `make verify` applies to every run |

## `make test` (unchanged)

`make test` gains no dependency on Kani and no verification step (FR-012, SC-006). It does
run the new ordinary tests of FR-015, FR-016 and FR-017, because they live in `core/`.

## CI: `.github/workflows/verify.yml`

| Job | Trigger | Runs |
|-----|---------|------|
| `verify` | `pull_request` and `push` to `master` with a path filter (`core/src/s2/cell_id.rs`, `core/src/s2/coords.rs`, `core/src/r3/**`, `core/src/s2/mod.rs`, `core/Cargo.toml`, `core/tests/data/cpp_cellid_vectors.csv`, `verify/**`, `Makefile`, the workflow file); weekly `schedule`; `workflow_dispatch` | `make verify` |
| `slow` | Weekly `schedule`, `workflow_dispatch` | `make verify-slow F=<part>`, a matrix over the six faces and `vertex`, with `VERIFY_JOBS=2` |
| `mutants` | Weekly `schedule`, `workflow_dispatch` | `make verify-mutants` |

Permissions are `contents: read`. There are no secrets. Kani is installed and cached as
described in research R8.

## CI: `.github/workflows/fuzz.yml` (one new matrix entry)

The nightly matrix gains `parse_cell_id_text`, the FR-017 fuzz target (research R14). No
other change.
