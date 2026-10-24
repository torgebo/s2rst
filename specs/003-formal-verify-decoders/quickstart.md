# Quickstart: Validating the Decoding-Layer Verification

A run guide that shows the feature working end to end. The command interfaces are in
[contracts/verify-commands.md](contracts/verify-commands.md). Harness naming and generators
are in [contracts/harness-conventions.md](contracts/harness-conventions.md). The deductive
track's rules are in [contracts/deductive-conventions.md](contracts/deductive-conventions.md).
Prerequisites:

- **Kani 0.68.0** for the bounded proofs (`make verify-kani-check`).
- **Verus `0.2026.09.20.aef82ed`** and Rust **1.98.1** for the deductive track (`make
  verify-verus-check`). Point `VERUS=` at the binary if it is not on `PATH`, for example
  `VERUS=~/Downloads/verus-x86-linux/verus`.
- Always go through the Makefile targets, which apply the memory guard. Run one
  verification job at a time.
- `df -h /` first: the Kani build cache under `verify/target/kani` grows by several GB.

## 1. The bounded decoding proofs (US1, US2, SC-001, SC-003, SC-006)

```sh
make verify-decoding            # or one part: P=vectors, P=links or P=codecs
```

Expected:
- `cargo test` in `verify/` passes. That includes the reference description against
  `cpp_coding_vectors.csv` (`wire_spec_vs_cpp`) and the reservation measurement (at most
  2 MiB per K2 input, SC-008).
- `cargo test` in `verify/deductive/` passes: tested agreement and the drift guard.
- Every `cd_*` harness line reads `SUCCESSFUL`, with all covers satisfied. That includes the
  `cd_agree::*` bounded agreement harnesses.
- Exit code `0`. The whole suite took 21 minutes on an 8-core workstation with 3 jobs
  (241 harnesses); CI runs it as three parallel parts, each meant to stay under 10 minutes
  on a 4-vCPU runner. The weekly members need `make verify-slow F=decoding`, and the
  xlarge ones a 32 GB machine (`make verify-xlarge`).

Cross-check:
- every in-scope row of `verify/decoding/INVENTORY.md` names a harness or family that
  appears in the output, and its bound matches the family's largest member;
- every row with track `K` in `verify/decoding/PROPERTIES.md` names a harness in the
  output.

## 2. The deductive track (US3, SC-005)

```sh
make verify-deductive
```

Expected: one line per restated module (`varint`, `fixed_width`, `uint_vector`,
`cellid_vector`, `string_vector`) with `0 errors`, then Verus's summary, then both tests
passing. It should take well under 10 minutes; research R0 measured 3.4 s for two of the
modules. Every row with track `V` in `PROPERTIES.md` names a lemma that this run verified.

Non-vacuity of the deductive track (FR-009):

```sh
make verify-deductive MUTANTS=1
```

Expected: every `V-NNN` patch reported `killed`, meaning Verus rejected it.

## 3. The drift guard (FR-011, US3 scenario 2)

Change any mirrored function, for example add `let _ = 0;` to the body of
`decode_uint_vector_u64` in `core/src/s2/encoded_uint_vector.rs`, then:

```sh
cd verify/deductive && cargo test --test drift
```

Expected: the test fails and names `encoded_uint_vector.rs::decode_uint_vector_u64` and its
restated copy `uint_vector::decode_uint_vector_u64`. Revert the change and the test passes.
A change to a doc comment or to formatting alone does not trip it.

## 4. A deliberately broken decoder (US1 scenario 4, US2 scenario 2)

```sh
make verify-mutants M="M-200 M-203"
```

Expected: both `killed`.
- `M-200` makes `get_uint_with_length` read past its input. Its property harness
  `cd_fw::p_fw_02__d8` fails on the slice index check.
- `M-203` writes the header's width field as `len` instead of `len − 1`. The encoder's
  agreement with the reference, `cd_uv::p_uv_01_enc__n1`, fails.

The full campaign (`make verify-mutants`) reports every `M-2NN` killed. The library-side
divergence mutants `M-25N` are killed by the `cd_agree::*` harnesses.

## 5. The behaviour fix and the documentation (FR-019 to FR-022, SC-008 to SC-010)

```sh
cargo test -p s2rst --test decoder_robustness reservation
cargo test -p s2rst --test cpp_coding_diff
cargo test -p s2rst --lib encoded_
make doc
```

Expected:
- The reservation regression test passes. It runs the K2 inputs in a child process under a
  256 MiB address-space limit; each returns an error and nothing aborts. On the commit
  before the FR-019 fix, the same test fails because the child is killed by SIGABRT.
- The differential tests pass against every row of the C++ vectors. Rows marked
  `deviation` are checked against the documented behaviour.
- The sibling test files pass, including the 1,000,000-element round trips.
- The rustdoc builds with no warnings. It shows `# Panics` on the three fixed-width helpers,
  the deviation notes on the `CellId` and `Cap` decoders, the count limits, and the
  string-vector rule for invalid offsets.

On the unfixed library, the same 5-byte inputs under a 300 MB limit end with "memory
allocation of 400000000 bytes failed" and SIGABRT (spec Context, K2). The regression test
automates exactly that check.

## 6. Fuzzing (FR-023, SC-011)

```sh
cargo +nightly fuzz run decode_cell_id -- -max_total_time=60
cargo +nightly fuzz run decode_point   -- -max_total_time=60
cargo +nightly fuzz run decode_rect    -- -max_total_time=60
```

Expected: no crash. Each target's round-trip assertion holds. The nightly workflow lists all
ten decoding targets.

## 7. The quality gate (SC-007)

```sh
make test
```

Expected: it passes with no Kani or Verus installed. `git diff db24e79^ -- core/Cargo.toml
python/Cargo.toml wasm/Cargo.toml` (from the commit before this feature's first) shows no
dependency change. (The branch also carries earlier unmerged work, so `git diff master`
shows a pyo3 bump that predates this feature.)

## 8. The mutation measurement (FR-018, SC-012)

```sh
make verify-mutation-measure PHASE=after
```

Hours, on a workstation (about 3.5 hours for the tests and 2 for the proof classification,
T066). An interrupted run resumes with `MUTANTS_ITERATE=1 make verify-mutation-measure
PHASE=after` on the same tree. Expected: `verify/decoding/MUTATION.md` gains an `after` table in
which every mutant is classified. Every `undetected` mutant has a reason, and the summary
compares the counts with the `before` table recorded at the start of the work.

## 9. The evaluation (US5, SC-013, SC-014)

Read `verify/decoding/EVALUATION.md` without other context. It should answer "continue?",
"with which method or methods?", "is the deductive track worth its cost?" and "what next?".
It should also list every upstream finding with its follow-up task.
