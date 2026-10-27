# Quickstart: Validating Spec 009

How to show, end to end, that the bounded types work: red before each fix, green after it, the
same results everywhere else, and the records in step. All paths are relative to the workspace
root. The tests and their expected outcomes are in
[contracts/red-green-tests.md](contracts/red-green-tests.md), and the error contract in
[contracts/binding-errors.md](contracts/binding-errors.md).

## Prerequisites

- The Rust toolchain from `rust-toolchain.toml`; `uv` (Python); `wasm-pack` and Node 24 (wasm).
- For the proofs: Kani 0.68.0 (`make verify-kani-check`), always run through the Makefile
  targets (`verify/run-verify.sh`), which cap memory, one run at a time.
- After any core change, rebuild the Python extension before its tests:
  `uv sync --directory python --reinstall-package s2rst`. Otherwise the tests run a stale
  `_core.abi3.so`.

## 1. See red (before each fix)

On the commit before a finding's fix, with only its new test added:

| Surface | Command | Expected |
|---------|---------|----------|
| Python | `uv run --directory python pytest tests/test_bounded_args.py -k <ID> -rA` | PY-01 fails with `PanicException` (31) and `OverflowError` (256, −1); PY-05 with an invalid id returned; PY-06 with a silent clamp; PY-10 with a `BaseException` |
| wasm | `wasm-pack test --node wasm --test bounded_args -- --exact <test name>` (one test per run: a trap can poison the instance) | WA-01 fails on 286 (a level-30 answer, no error) and on 31 (`RuntimeError: unreachable`); WA-03 on 7 (an invalid id); WA-04 on 0 (clamped) |
| Core runtime | `cargo test -p s2rst --all-features coords_tests::` and `region_coverer_tests::` | CO-01 and CO-02: deserialisation returns `Ok` |
| Core type-level | `cargo test -p s2rst --all-features --doc` (the `Level`, `CellId::from_face`, `RegionCoverer` and `Cell::vertex` docs) | CD-01 to CD-04: "test compiled successfully, but it's marked `compile_fail`" |

Copy each failure's key line into `BOUNDED_TYPES.md`'s red/green table, with the date and commit.

## 2. See green (after each fix)

The same commands pass. Also check these:

```sh
# FR-007 and research R1: no bounded type is taken through `impl Into`, and no face as a raw integer
grep -rnE "impl Into<(Level|Face|LevelMod|CellVertex)>|face: impl Into<u8>" core/src   # expect nothing

# FR-007: no binding method takes a raw u8 or usize for a level, face, modulus or vertex
grep -nE "\b(level|min_level|max_level|expand_level|snap_level|face|level_mod|modulo|k): (Option<)?(u8|usize|i64)\b" python/src/*.rs wasm/src/*.rs
# expect only kinds that were not adopted: DensityTree's max_level, two container indices
# (Polygon.loop_, loopAt) and s2::Rect's Python vertex(k) (BOUNDED_TYPES.md, section 3.4a)
```

JavaScript sees each rejection as a thrown exception whose value is the message string (the
wasm crate's one error funnel, `js_err`). Only the glue's own `BigInt` rejection is an `Error`
object; see contracts/binding-errors.md.

## 3. Same results everywhere else

| Check | Command | Expected |
|-------|---------|----------|
| The full gate | `make test` | passes: check, fmt, clippy, core tests and doctests, docs, Python (ruff, mypy, stubtest, pytest), wasm, cargo-deny |
| The stub is unchanged | `git diff --stat python/python/s2rst/_core.pyi` | only docstring wording, if anything; every parameter is still `int` |
| The differential tests | part of `make test` (`core/tests/*_diff.rs`) | pass with unchanged CSVs |
| The serialised forms | CO-04 | byte for byte as before |
| Hot paths (R13) | `cargo bench -p s2rst --bench cell_id --bench cell --bench region_coverer`, on the commit before the `Level` change and again after it, and likewise for `CellVertex` | gungraun shows no instruction-count increase beyond noise; record both runs in the evaluation |

## 4. Proofs and records

```sh
make verify-harness H=crash::n_level_new    # Level::new panics exactly above 30
make verify-values P='cell'                 # includes CellVertex's constructor and Cell::vertex
make verify && make verify-values           # the per-change suites: every harness SUCCESSFUL
make verify-release                         # n_*, r_*, o_* without debug assertions
make verify-mutants M='M-803 M-806 M-809 M-810 M-811'  # the re-pointed and new constructor mutants: killed
cd verify && cargo test                     # models, links, the properties-by-kind index
```

The harness names above are the planned ones (research R11). The final names are recorded in
`verify/COVERAGE.md`.

Then confirm that the records agree:

- `verify/COVERAGE.md`'s gap-1 rows for the 13 raw-level operations, `denormalize` and
  `Cell::vertex` point to the constructor proofs;
- `VERIFIED_TYPES.md` lists the four bounded types;
- `verify/PROPERTIES_BY_KIND.md` and its `.html` agree, which `cargo test` in `verify/` checks.

## 5. The evaluation answers the spec's questions

Open `BOUNDED_TYPES.md` and pick any of the 71 gap-1 operations, or any function in
[contracts/signature-audit.md](contracts/signature-audit.md). From that file alone, you should
be able to say:

- its class;
- whether a bounded type removes its panic;
- the recommended option;
- what the change costs in Rust, Python and JavaScript, with the measured numbers for the four
  prototyped kinds.

This is spec SC-007.
