# Contract: Verification Commands (changes for `Cell` and `s1::Interval`)

The 001, 002 and 003 contracts (`specs/00{1,2,3}-*/contracts/verify-commands.md`) still
apply. This file lists only what changes. CI still calls the same Makefile targets as a local
run (Constitution, Development Workflow).

## Suite selection (research R2)

The `verify/` crate gains a third **default** Cargo feature, `values`. It gates every
`cl_*` and `iv_*` harness module and `value_domain.rs`. The reference descriptions
`cell_spec.rs` and `arc_spec.rs` are always compiled, like `spec.rs`, so that `cargo test`
checks them without Kani.

- With default features, a build contains all four suites.
- `--no-default-features` still builds the `CellId` suite alone.
- `--harness cu_` still selects `CellUnion` alone, and `--harness cd_` the decoding suite.
- `--harness cl_` selects the `Cell` harnesses, and `--harness iv_` the interval harnesses.

## `make verify` (changed scope)

It runs all four suites' per-change proofs.

| Aspect | Contract |
|--------|----------|
| Steps | Unchanged: `cargo test` in `verify/` (now including the sanity tests of `cell_spec.rs` and `arc_spec.rs` against `cpp_values_vectors.csv`, and the exhaustive `uv_projection` check, about 90 s in a debug build), `cargo test` in `verify/deductive/`, then `run-verify.sh` |
| Output, exit code, memory guard, playback | Unchanged |
| Duration | Each existing suite unchanged. Each values part (`P=cell`, `P=interval`) under 10 min on the reference runner (SC-005) |

## `make verify-values [P=cell|interval]` (new, FR-013)

This feature's proofs alone.

- It runs `cargo test` in `verify/`, then `run-verify.sh --harness cl_ --harness iv_`.
- `P=cell` keeps only `--harness cl_`, and `P=interval` only `--harness iv_`.
- Output format, memory guard and exit-code rules are those of `run-verify.sh`.
- CI uses it for the two values legs.

Duration: CI runs `P=cell` and `P=interval` as two legs, as 003 did for decoding. Each leg
takes under 10 min on the reference runner (SC-005). Research R12 estimates 4–6 and 7–9
minutes; without `P`, the whole suite takes 12–15 minutes.

## `make verify-slow F=values` (new part)

It runs the values suite with its slow-tier members (feature `slow`), one at a time, 60
minutes each, like `F=union`. The slow-tier members are:

- the `__l<L>` members of P-CBND-01, P-CBND-02 and P-CCHD-02 above the per-change levels;
- any split member of P-IEXP-01 or P-IMEA-06 that is over the per-change budget (research R8,
  R9).

The weekly `slow` job gains a `values` leg. Without `F`, the target's default parts are
unchanged.

## `make verify-harness H=<filter>`, `make verify-mutants` (unchanged interfaces)

- `H=iv_mea::p_imea_01` works like any 001 name.
- `make verify-mutants` also runs the `M-3NN` patches, against `cell.rs`, `region.rs` and
  `s1/interval.rs`.

## `make test` (unchanged definition)

It gains no Kani dependency. It does run the new ordinary tests, because they live in `core/`
and `core/tests/`:

- the regression tests of FR-018, FR-020 and D-005, in the new sibling files
  `core/src/s2/cell_tests.rs` and `core/src/s1/interval_tests.rs` (research R5, R6, R9);
- the differential tests `core/tests/cpp_values_diff.rs` (research R10);
- the existing property tests in `interval.rs` and `cell.rs`, unchanged. The one that K4
  contradicts passes once FR-020 is in.

The Python suite (part of `make test`) gains one test in
`python/tests/test_interval.py` for the FR-020 corner (research R5).

## CI: `.github/workflows/verify.yml`

| Job | Change |
|-----|--------|
| `verify` | The matrix gains `values-cell` and `values-interval`. They run `make verify-values P=cell` and `P=interval`, with the existing 30-minute job limit. The path filters gain `core/src/s2/region.rs` (the `Region` impl for `Cell`), `core/src/r2/**` (the cell's bounds are an `r2::Rect`) and `core/tests/data/cpp_values_vectors.csv`. `cell.rs`, `cell_id.rs`, `coords.rs`, `r1/interval.rs` and `s1/interval.rs` are already listed. The header comment names spec 004 |
| `slow` | A `values` leg: `make verify-slow F=values` |
| `mutants` | Runs the `M-3NN` patches with the rest. No new job |
| `deductive` | Unchanged |
