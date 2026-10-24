# Research: Formal Verification of `CellUnion`

Phase 0 of [plan.md](plan.md). Each entry gives the decision, the rationale, and the
alternatives considered. The fixed inputs are:
- **001's setup and results.** Kani 0.68.0 / CBMC 6.11.0, the `verify/` crate, the
  memory-guarded runner, the mutant campaign, CI and the weekly tier. The 001 evaluation
  (`verify/EVALUATION.md`) recommends continuing for integer code.
- **The spec clarifications of 2026-10-02.**
  - Decoding keeps the identifiers exactly as encoded and rejects anything that is not a
    valid union (FR-017).
  - **After the R0 probe:** keep Kani at the measured bounds. `difference` and the
    range constructors become tool-limit exclusions. The decoder's error messages are
    formatted only when displayed (R4, R5, R11).

There are no open NEEDS CLARIFICATION items. Facts were checked against the working tree
at `e0b3736`, the local Kani 0.68.0 install, and the upstream checkout in
`/home/t/sources/s2geometry`. R0 is a memory-capped probe run on 2026-10-02. R4, R5, R11
and R15 rest on it.

## R0. Feasibility probe (measured, 2026-10-02)

These were throwaway harnesses in a temporary `verify/src/probe.rs`, removed afterwards.
Each ran alone under the memory guard, with `ulimit -v` and a `systemd-run` scope. The
setup was Kani 0.68.0, CBMC 6.11.0 and CaDiCaL on the 8-thread workstation. Unions were
built from model decompositions as in data-model §2. A size `n` is a concrete length,
`__n<k>`, unless the table says "symbolic".

| Harness | Domain | Result | Time | Peak memory |
|---------|--------|--------|------|-------------|
| `leaf_cells_covered` | valid union, 3 cells | ✓ | 2.7 s | small |
| `leaf_cells_covered` | valid union, symbolic `n ≤ 3` | killed | > 5.5 min | — |
| `contains_cell_id` = model | valid union, 3 cells; any valid cell | ✓ | 28.5 s | < 4 GB |
| `denormalize` crash | 1 cell, any `min_level`, `level_mod` 1–3, gap ≤ 2 | ✓ | 9.5 s | < 4 GB |
| `sort_unstable` alone | 3 valid cells | ✓ | 1.5 s | 0.4 GB |
| `from_cell_ids` (normalise) crash | 1 cell | ✓ | 1.5 s | small |
| `from_cell_ids` crash | 2 cells | ✓ | 16.5 s | 0.9 GB |
| `from_cell_ids` crash | 3 cells | OOM at 6 and 8 GB. At 14 GB: every check passes, but the cover and one `Vec::pop` check end in `ERROR` (solver out of memory) | 40 s | 13.3 GB |
| `from_cell_ids` crash | 3 cells, input already sorted | OOM at 8 GB | 23 s | 8 GB |
| `from_cell_ids` crash | 3 and 4 cells, unwind = `n + 1` | OOM at 6 GB | ~20 s | 6 GB |
| `from_cell_ids` crash, `--no-memory-safety-checks` | 3 cells | OOM at 8 GB | — | 8 GB |
| `from_cell_ids` crash, `--arrays-uf-always` / `--refine-arrays` | 3 cells | OOM at 8 GB | — | 8 GB |
| `union` covers `A ∪ B` | 1 × 1 cells | ✓ | 26.4 s | 0.9 GB |
| `intersection` covers `A ∩ B` | 1 × 1 cells, unwind 3 | ✓ | 84.7 s | 3.9 GB |
| `intersection` covers `A ∩ B` | 1 × 1 cells, unwind 6 | OOM at 6 GB | 46 s | 6 GB |
| `intersection` covers `A ∩ B` | 2 × 2 cells, unwind 5 | OOM at 6 GB | 39 s | 6 GB |
| `difference` covers `A − B` | 1 × 1 cells, level window 1, unwind 3 | timeout | > 5 min | 5.4 GB |
| `difference` covers `A − B` | 1 × 1 cells, level window 2, unwind 4 | OOM at 6 GB | 156 s | 6 GB |
| `from_begin_end` crash | range depth 1 (inside one level-29 cell), unwind 32 | OOM at 6 GB | 123 s | 6 GB |
| `from_begin_end` crash | range depth 1, `--unwindset` 9 on its own loop | OOM at 6 GB | 43 s | 5.7 GB |
| `S2Decode` | every 9-byte input (no cells) | timeout; symbolic execution never finishes, inside `core::fmt` | > 5 min | 5.3 GB |
| `S2Decode` | every 17-byte input (≤ 1 cell) | OOM at 6 GB | 267 s | 6 GB |

**What the probe shows**:

1. **Concrete lengths are essential.** A symbolic length `n ≤ N` makes CBMC unroll every
   loop over the union to the harness bound on every path. With a concrete length, the
   same three-cell sum fell from more than 5.5 minutes to 2.7 s (R4).
2. **Read-only operations are affordable.** These cost about as much as 001's harnesses,
   at 3 cells or more: the queries (`contains_cell_id`, `intersects_cell_id`), the
   measures (`leaf_cells_covered`), the invariant checks (`is_valid`, `is_normalized`
   over a concrete length), and `denormalize` of one cell.
3. **Building a union is not affordable beyond 1–2 cells.** `normalize` needs about
   14 GB at 3 cells, and its merge loop is the cost: sorting is cheap, and pre-sorted
   input is no cheaper. The cost is in the propositional reduction, not the solver.
   Disabling the standard library's pointer checks, or changing CBMC's array encoding,
   does not help. Every operation that normalises is limited the same way: `union`,
   `from_cell_ids`, `FromIterator`, and today's decoder. Four cells, the smallest case
   with a sibling merge, is out of reach at any budget this project uses.
4. **`difference`** recurses four ways per level and does not fit even for two single
   cells within one level. **`from_begin_end`** does not fit even for the narrowest
   ranges, because every output cell runs `maximum_tile`'s 30-level loop, which 001
   measured at 122 s alone.
5. **The decoder is dominated by `core::fmt`.** Its two error paths build messages with
   `format!` from symbolic values (the version byte and the cell count). This is the
   same wall as 001's formatters (001 R6). Even the 9-byte input, which can never hold a
   cell, does not finish symbolic execution in 5 minutes.
6. **The runner misclassifies one out-of-memory case.** When CBMC exhausts memory before
   the SAT solver ("Out of memory", exit status 6), `run-verify.sh` reports `ERROR`
   rather than `OOM`. Only the solver's own out-of-memory message is matched. The fix
   is small and belongs to this feature's runner changes.

## R1. Tool, crate and suite selection

- **Decision**:
  - Keep Kani 0.68.0, CBMC 6.11.0 and CaDiCaL, exactly as pinned for 001.
  - Extend the existing `verify/` crate rather than adding a second one.
  - Gate the `CellUnion` modules behind a new **default** Cargo feature, `cellunion`.
  - Name every `CellUnion` harness module `cu_<area>`.

  `make verify` builds both suites. `make verify-cellid` passes `--no-default-features`.
  `make verify-union` passes `--harness cu_` (contracts/verify-commands.md).
- **Rationale**:
  - One crate shares the model (`spec.rs`), the generators, the runner, the playback
    handling, the mutant runner, the CI cache and the Kani pin.
  - The feature gate is the only robust way to select the 001 suite alone. Kani's
    `--harness` filter matches substrings, so `--harness crash::` would also select
    `cu_crash::…`.
  - No 001 harness path contains `cu_` (checked), so `--harness cu_` selects the new
    suite exactly.
- **Alternatives considered**:
  - **A second crate, `verify-union/`**: rejected. It duplicates the runner, the cache
    and the model, and the `CellUnion` model is built on the `CellId` model.
  - **Upgrading Kani**: rejected. Mixing a tool upgrade into a new type's proofs makes
    the 001 timings incomparable. If an upgrade is wanted, it is a separate change.
  - **Module names without a feature gate** (`--harness crash:: --harness val:: …` for the
    001 suite): rejected because of the substring collisions above.

## R2. Where the harnesses live

- **Decision**: new files in `verify/src/`:
  - `union_spec.rs`, the model (data-model §1). It is always compiled, like `spec.rs`.
  - `union_domain.rs`, the generators.
  - `cu_crash.rs` and one `cu_<area>.rs` per property area (contracts/harness-conventions).

  The new artifacts go in `verify/cell_union/`: `INVENTORY.md`, `PROPERTIES.md` and
  `EVALUATION.md`. The shared files are extended in place:
  - `verify/TRUSTED_BASE.md` gains a `CellUnion` section;
  - `verify/MUTANTS.md` gains rows, regenerated by `run-mutants.sh`;
  - `verify/README.md` gains the two new targets and a note on the layout.

  The model's sanity tests go in `verify/tests/union_spec_vs_cpp.rs`.
- **Rationale**:
  - Crate-root harness modules are required for Kani's `--exact` matching (001 R2).
  - A per-type document folder keeps 001's finished records unchanged and their links
    valid. The tools and mutants are shared, so their documents stay shared.
- **Alternatives considered**:
  - **Appending to 001's `INVENTORY.md` and `PROPERTIES.md`**: rejected. Those files are
    001's acceptance record, and SC-002 and SC-003 are counted per type.
  - **Moving 001's files into `verify/cell_id/`**: rejected. It churns finished work and
    breaks links from the 001 spec, the evaluation and the CI comments.

## R3. Reference model of a union (FR-006)

- **Decision**: the meaning of a union is the set of leaf cells it covers (data-model §1).
  The model is written over 001's `Decomposed` cells and leaf indices, from upstream
  `s2cell_union.h`/`.cc`. Functional properties are stated for a symbolic leaf `x`, which
  Kani proves for every leaf.

  The two sides of each property are read differently:
  - **Inputs** are generated from model decompositions, so their side of the property is
    evaluated entirely in the model.
  - **Outputs** come from the implementation. They are read through `CellId::range_min`
    and `range_max`, which 001 proved equal to the model's `range` for every valid cell
    (P-RANGE-01). This is recorded as a reliance on 001 (FR-013, data-model §7).
  - **Validity and normalisation of outputs** are checked with the model's
    `valid_union` and `normalized_union` predicates on the outputs' decompositions. Those
    are read through `CellId::is_valid`, `face`, `level` and `pos`, which 001 proved equal
    to the model (P-VAL-03, P-VAL-06). (Planned with 001's `spec::decompose`; changed during
    implementation because its 31-level loop forced an unwind of 32 onto the normalising
    harnesses.) The implementation's own `is_valid` and
    `is_normalized` are checked separately (P-UINV-01/02), so a defect in them cannot
    mask a defect in an operation.
- **Rationale**:
  - A symbolic witness turns "the two unions cover the same set" into a single assertion.
    No enumeration of the 6·4^30 leaves is needed.
  - Reading outputs through 001's proven `range_min` and `range_max` avoids running the
    model's 31-level `decompose` search on every output cell when only coverage matters.
    That search made 001's `p_range_06` cost over 8 minutes before it was made
    compositional (001 R15).
  - The model is plain Rust, so `cargo test` can sanity-check it against the C++
    vectors (R13), as 001 did.
- **Alternatives considered**:
  - **Comparing outputs to a model-computed union element by element**: kept only where
    upstream fixes the exact output (`denormalize`, P-UDEN-01; encoding and decoding). For
    the set operations, upstream fixes the covered set and the invariant, not a particular
    list. Pinning the list would over-specify them.
  - **Stating outputs through the implementation's `contains`**: rejected. Every property
    would then rely on one more 001 result, with no saving.

## R4. Input domains and union-size bounds (FR-007, SC-001)

- **Decision**:
  - **Concrete lengths.** Every union argument has a *concrete* length per harness. A
    family `__n0 … __nN` covers every length up to `N`, and a two-union operation gets
    one harness per pair of lengths, joined with `_`. Byte input for decoding has a
    concrete length per harness, `__b<L>`.
  - **Bounds per operation**, set from R0:

    | Operation (inventory) | Union domain | Bound proven (from R0) | Family |
    |-----------------------|--------------|------------------------|--------|
    | `is_valid`, `is_normalized` | `Seq` | `n ≤ 4` (read-only; 4 reaches the sibling check) | `__n0…__n4` |
    | `num_cells`, `cell_ids`, `is_empty`, `Deref`, both `IntoIterator`s, `from_verbatim`, `new` (with the derived `Default`, which is outside the inventory) | `Seq` | `n ≤ 3` | `__n0…__n3` |
    | `contains_cell_id`, `intersects_cell_id`, `Region::contains_cell`, `Region::intersects_cell` | `ValidUnion` | `n ≤ 3` (28.5 s at 3) | `__n0…__n3` |
    | `leaf_cells_covered` | `ValidUnion` | `n ≤ 3` (2.7 s at 3) | `__n0…__n3` |
    | `encode` | `Seq` | `n ≤ 3` | `__n0…__n3` |
    | `normalize`, `from_cell_ids`, `FromIterator` | `ValidSeq` | `n ≤ 2` (16.5 s and 0.9 GB at 2; about 14 GB at 3) | `__n0…__n2` |
    | `intersection_with_cell_id` | `ValidUnion` | `n ≤ 2` (to confirm; it pushes at most `n` cells) | `__n0…__n2` |
    | `denormalize` | `ValidUnion` (upstream states no normalisation precondition; at `n ≤ 1` every valid union is normalised anyway) | `n ≤ 1`, level gap ≤ 2 (9.5 s) | `__n0`, `__n1` |
    | `union`, `intersection` | `ValidUnion` × 2 | `1 × 1` (26 s; 85 s at 3.9 GB with unwind 3) | `__n0_n0 … __n1_n1` |
    | `contains_union`, `intersects_union` | `ValidUnion` × 2 | `2 × 2` (read-only; to confirm) | `__n0_n0 … __n2_n2` |
    | `S2Decode` | `Bytes` | every input of exactly `L` bytes, `L ≤ 17`, after the R11 message change (to confirm) | `__b0…__b17` |
    | `whole_sphere` | — | whole domain (a constant) | — |
    | `difference`, `from_begin_end`, `from_min_max`, `from_range` | — | **tool-limit exclusion**: over the budget at 1 cell (R0, R5) | — |

    "To confirm" rows were not measured exactly in R0. They are the first harnesses to run
    during implementation, under the same limits. If one does not fit, its bound drops to
    the largest member that does, and the evaluation records it.
  - **Unreached behaviour.** It is named in the inventory (FR-007):
    - the sibling merge in `normalize` and its callers (needs 4 cells);
    - cascades of merges;
    - `intersection` interleavings of more than one cell per side;
    - `denormalize` across more than one cell, or a level gap above 2.

  - **Uncovered sizes.** Longer unions and longer byte strings are the input-size
    remainder (FR-007). Four kinds of test exercise them:
    - the existing unit tests in `cell_union.rs` and `encoding.rs`;
    - the new quickcheck properties (R17);
    - the C++ vectors (R13);
    - the nightly `decode_cell_union` fuzz target.
- **Rationale**: in R0, a symbolic length `n ≤ N` made CBMC unwind every loop over the
  union up to the harness bound on every path. That cost minutes even for a three-cell
  sum. A concrete length lets CBMC's constant propagation bound each loop exactly. 001
  used the same tactic for text (`__b<L>`).
- **Alternatives considered**:
  - **One harness per operation with a symbolic length**: measured in R0 and rejected.
    It was slower by two to three orders of magnitude on the same operation.
  - **`kani::vec::any_vec`**: same symbolic-length problem.

## R5. Loops and recursion

The operations whose loop count does not follow from the union length:

| Operation | Loop | Bound and why it is sound |
|-----------|------|---------------------------|
| `normalize` (and every constructor or set operation that calls it) | the sort, the merge loop, and the sibling-collapse loop | All are bounded by the concrete union length; each collapse removes three cells. Unwinding assertions confirm it |
| `contains_cell_id`, `intersects_cell_id` | `partition_point` (binary search) | `⌈log2(len)⌉ + 1` for a concrete length |
| `difference` | recursion into children (`get_difference_internal`) | **Tool-limit exclusion.** Depth ≤ 31 by level, but each level branches four ways, so the call tree is exponential in the depth. Even 1 × 1 cells within a one-level window timed out (> 5 min), and a two-level window ran out of memory (R0). Covered by R17's property tests and the `CELLUNION_SETOPS` vectors |
| `from_begin_end`, `from_min_max`, `from_range` | one `maximum_tile` (or parent climb) per output cell, with up to about 186 output cells | **Tool-limit exclusion.** Every output cell runs `maximum_tile`'s 30-level loop. Even ranges inside one level-29 cell ran out of memory, also with a per-loop `--unwindset` (R0). Covered by R17 and the `CELLUNION_FROM_BEGIN_END` vectors |
| `denormalize` | the loop over each cell's descendants, `4^(new_level − level)` | **Level-gap bound**: `new_level − level ≤ 2`, so at most 16 descendants per cell (9.5 s for one cell, R0). Larger gaps are the uncovered part, exercised by the C++ differential vectors (R13) and R17 |
| `S2Decode` | the per-cell read loop | Bounded by the concrete byte length: each iteration consumes 8 bytes or fails |

- **Unwind bounds.** Each harness uses the smallest global unwind that passes its
  unwinding assertions, normally `n + 1`. In R0, a loose bound (6 instead of 3) turned a
  passing 1 × 1 intersection into an out-of-memory run, because CBMC unrolls every loop
  with a symbolic trip count to the bound.
- **Rationale**: these are the only places where exhaustive checking cannot reach the
  whole domain at any union size. The `denormalize` bound is input-dependent (a loop
  count), which FR-007 allows when it is stated, documented and covered by tests.
  `difference` and the range constructors cannot be checked even at the smallest sizes,
  so FR-002 makes them tool-limit exclusions (clarification 2026-10-02).
- **Alternatives considered**:
  - **Kani loop contracts** (`-Z loop-contracts`): rejected. They need
    `#[kani::loop_invariant]` annotations inside `core/` code, which FR-009 rules out
    (no verification code in the library). They are also unstable in 0.68.0.
  - **Stubbing `maximum_tile` with a cheaper model**: rejected. It would replace code
    under test (the contracts forbid stubbing), and the model's `maximum_tile` is no
    cheaper.
  - **Restricting `difference` to unions on one face**: rejected. It is a domain
    restriction, not a loop bound, so FR-007 would make it a tool-limit exclusion anyway.
  - **A level-window recursion bound for `difference`, or a range-depth bound for the
    constructors**: tried in R0, and over the budget at the smallest window and depth.

## R6. No stubbing; reliance on 001 (FR-013)

- **Decision**: no stubs. Every `CellUnion` proof executes the real `CellId` code, the
  real `std` sort and the real `io::Read` for `&[u8]`. Where a property statement reads an
  output through a `CellId` method, that method's agreement with the model is taken from
  001. Each such method is listed in `TRUSTED_BASE.md` with its 001 property (data-model
  §7).
- **Rationale**:
  - **Stubbing.** The contract forbids stubbing code under test. `CellId` is not under
    test here, but stubbing it would also hide any bad interaction between `CellUnion` and
    real `CellId` behaviour, such as a debug assertion firing.
  - **Cost.** R0 shows the real code is affordable at the chosen bounds.
- **Alternatives considered**:
  - **`#[kani::stub_verified]` with function contracts on `CellId`**: rejected. Contracts
    must be written as attributes on `core/` functions (FR-009), and contracts are
    unstable in 0.68.0.

## R7. Non-vacuity (FR-008, SC-004)

- **Decision**: as in 001.
  - Each `proven*` property gets at least one mutant, `M-1NN`, against
    `core/src/s2/cell_union.rs` or `core/src/s2/encoding.rs`.
  - Each crash harness has cover checks for its interesting outcomes (a merge and no
    merge; a decode that succeeds and one that fails; an empty and a non-empty result).
  - **`M-100`** is the overflow demonstration (US1 scenario 4). In `leaf_cells_covered`,
    the shift `2 · (30 − level)` becomes `2 · (31 − level)`. Two face cells then sum to
    2^63, which overflows, and `cu_crash::c_leaf_cells_covered__n2` fails.
  - **`M-150` reverses FR-015** (P-UDEN-01), and **`M-151` reverses FR-017**: decode
    normalises again (P-UENC-02).

  The planned mutants, about 25:

  | Area | Mutant ideas |
  |------|--------------|
  | ACC | `num_cells` off by one; `is_empty` inverted |
  | INV | `is_valid` uses `>` instead of `>=` on the overlap check; `is_normalized` skips the sibling check at index 0 |
  | NORM | `contains` direction swapped in the merge; the "discard contained" loop keeps the last cell; the sort reversed (P-UNORM-03); the sort removed (P-UNORM-04). Mutants in the collapse loop (`ci = ci.parent()` dropped, `are_siblings` ignoring `d.is_face()`) are not planned: the proofs cannot reach that code within their bounds (R4). The unit tests and the C++ vectors guard it, and the evaluation says so |
  | MEM | `partition_point` predicate uses `range_min` for `range_max`; `intersects_cell_id` uses `contains`; `contains_cell_id` uses `intersects` (P-UMEM-03, an implication, needs a wrong `true`) |
  | RANGE | `whole_sphere` builds five faces (P-URANGE-04) |
  | SET | `get_intersection` pushes the larger cell on `Equal`; `union` does not normalise; `intersection_with_cell_id` uses `<` for `<=` |
  | DEN | the `offset` rounding is off by one (P-UDEN-01 and P-UDEN-02); the clamp is removed (M-150) |
  | MEAS | `leaf_cells_covered` counts `1 << (2·level)` |
  | ENC | the version byte is written as 0; decode accepts version 2; decode drops the last id; decode normalises (M-151, killable only if the decode bound reaches 25 bytes); the order check unwraps `ids.last()` on the first id (P-UENC-04) |
- **Rationale**: the same reasoning as 001 R7. These are cheap checks that the proofs can
  fail.
- **Alternatives considered**: `cargo-mutants`. Rejected for the same reason as in 001: it
  cannot target a property, and it would need a custom test command per mutant.

## R8. CI integration (FR-012, SC-005)

- **Decision**:
  - **Paths.** `verify.yml` gains four: `core/src/s2/cell_union.rs`,
    `core/src/s2/encoding.rs`, `core/tests/data/cpp_cellunion_vectors.csv`, and
    `core/src/s2/cell.rs`, because the `Region` cell-query proofs build their `Cell`
    argument with `Cell::from_cell_id`.
  - **Jobs.** The per-change job becomes a two-leg matrix, `cellid` and `union`, which
    run `make verify-cellid` and `make verify-union` in parallel. Each leg keeps the
    30-minute limit.
  - **Mutants.** The weekly mutant job's limit rises from 120 to 180 minutes.
  - **Slow tier.** It gains a `union` leg only if R4 puts any harness there.
- **Rationale**:
  - **Parallel legs.** They keep each suite's wall time comparable to its budget (SC-005
    measures them separately), and they give faster feedback than one 25-minute job.
  - **Both legs on every match.** `CellUnion` is built on `CellId`, so a `CellId` change
    re-checks both.
- **Alternatives considered**:
  - **One job running `make verify`**: rejected. Its wall time is the sum of the two
    budgets.
  - **Per-job path filters** (a paths-filter action): rejected. It adds a third-party
    action, and a `CellId` change must run both legs anyway.

## R9. FR-015: `denormalize` clamp

- **Facts** (`core/src/s2/cell_union.rs:384–420`): for `level ≥ min_level` and
  `level_mod > 1`, the code computes `level + (level_mod − offset)` with `Level`'s `Add`.
  That calls `Level::new`, which asserts `≤ 30` before the following `.min(Level::MAX)`
  can clamp. The probe test on 2026-10-02 gave `level must be 0..=30` for a level-29 cell
  with `min_level` 1 and `level_mod` 3.

  C++ (`s2cell_union.cc:215–223`) computes `new_level` as an `int` and then clamps it
  with `min(kMaxLevel, new_level)`. The two rounding formulas agree, because 30 is a
  multiple of 1, 2 and 3.
- **Decision**: compute the rounded level without overflow, clamp it to 30, then build
  the `Level`. The implementation uses a saturating `u8` add, which cannot overflow even
  for a `level_mod` outside 1–3 in a release build, where the `debug_assert!` is off, and
  needs no cast. Keep the rest of the function unchanged.

  The fix lands in its own commit with three things:
  - a regression test in a new sibling `core/src/s2/cell_union_tests.rs` (Constitution
    III), covering the level-29 / 1 / 3 case and the leaf / 1 / 2 case;
  - the `CELLUNION_DENORMALIZE` differential test (R13);
  - a listing in the evaluation.
- **Rationale**: it is the smallest change that gives C++'s results for every input
  inside the upstream preconditions.
- **Alternatives considered**:
  - **`Level::try_new(..).unwrap_or(Level::MAX)`**: equivalent, but it hides the clamp's
    intent.
  - **Only documenting the panic**: rejected. It breaks parity for inputs that upstream
    accepts (FR-015).

## R10. FR-016: leaf-count precondition

- **Facts**:
  - Upstream `FromVerbatim` REQUIRES `IsValid()` (`s2cell_union.h:142`), and
    `LeafCellsCovered` says "no more than 6·2^60" (`s2cell_union.h:303–305`).
  - A valid union's cells are disjoint, so the count is at most 6·4^30 = 6·2^60, which is
    below `i64::MAX` (about 8·2^60).
  - Eight overlapping face cells sum to exactly 2^63, which overflows; the probe test with
    12 faces panicked in debug.
- **Decision**: documentation only, in the R14 commit:
  - `from_verbatim` gets "the ids must satisfy `is_valid()`; operations on a union that
    does not are unspecified, and some may panic".
  - `leaf_cells_covered` gets three things:
    - "requires a valid union";
    - a `# Panics` section: in debug builds, it can overflow when the union is not valid,
      or panic on an invalid identifier through `CellId::level`;
    - a deviation note: it returns `i64`, where C++ returns `uint64_t`. The values agree
      for every valid union.
  - **Proof and argument.**
    - P-UMEAS-01 proves no overflow for every valid union up to the bound.
    - The evaluation adds the written argument for any size: disjoint cells cover
      disjoint leaf sets, so the sum is at most `6·4^30 < 2^63`.
- **Rationale**: the overflow is outside the upstream contract, so documenting the
  contract is a parity fix and changes no behaviour. Changing the return type would break
  the public API (FR-009) and the Python stub.
- **Alternatives considered**:
  - **`u64` return type**: rejected (API break).
  - **Saturating arithmetic**: rejected. It changes behaviour for no valid input, and
    still returns a wrong answer for invalid ones.
  - **Validating in `from_verbatim`**: rejected. It is a behaviour change, and a cost on a
    hot path that upstream leaves unchecked.

## R11. FR-017: decode verbatim, reject invalid unions

- **Facts** (`core/src/s2/encoding.rs:282–303`): the decoder reads a version byte (≤ 1),
  a `u64` count (≤ 1,000,000), then `n` ids through `CellId::decode`. That rejects invalid
  ids, which C++ does not (`s2cell_id.cc:261–265`). It then returns
  `CellUnion::from_cell_ids(ids)`, which normalises. C++ keeps the ids verbatim
  (`s2cell_union.cc:544–562`).

  Callers of the decoder:
  - the Python `decode_cell_union` and the wasm `CellUnion.decode`;
  - the `decode_cell_union` fuzz target;
  - the `cellunion` robustness test.

  No other `core/` code decodes a `CellUnion`.
- **Decision**: after the ids are read, check the union invariant. A failed check returns
  `io::ErrorKind::InvalidData` ("cell ids are not a valid cell union: unsorted or
  overlapping"). A passing check returns `CellUnion::from_verbatim(ids)`.
  - **The check.** It is the same comparison `is_valid` makes for consecutive ids:
    `ids[i−1].range_max() < ids[i].range_min()`. That gives one linear pass, and no
    second validity check per id.
  - **Documentation.** A rustdoc comment on `impl S2Decode for CellUnion` states three
    deviations from C++: invalid ids are rejected, unsorted or overlapping ids are
    rejected, and the ids are kept as encoded.
  - **Tests.** Regression tests in `cell_union_tests.rs`:
    - four sibling cells round-trip unchanged (this fails before the fix);
    - `[c2, c0]` is rejected;
    - a duplicate is rejected;
    - an overlap is rejected.

    The `CELLUNION_DECODE` differential test (R13) covers decoding, and the
    `decode_cell_union` fuzz target keeps its assertion.
  - **Bindings.** The Python and wasm decoders inherit the change. A Python test asserts
    that an unsorted encoding raises `ValueError`, through the existing `io_err` mapping.
  - **Deferred error formatting** (clarification 2026-10-02). The decoder's errors carry
    a small private error type with one variant per reason: version, count, not a valid
    union. It implements `Display` with exactly today's message text and is passed to
    `io::Error::new`. The message is formatted only when someone displays the error, not
    while decoding. R0 found that `format!` with the symbolic version byte and count puts
    `core::fmt` into every decoding proof. That is the formatter wall of 001's R6, and it
    stopped even the 9-byte proof.
    - **Unchanged.** `to_string()` and `Display` output are the same, and that is what
      the Python and wasm error mappings use.
    - **Changed.** Only `{:?}` of the `io::Error` differs: it shows the variant instead of
      a string. Nothing in the repository matches on it (checked with `grep`).
    - **Tests.** A regression test asserts each message's text.
- **Rationale**: this is the user's choice (clarification 2026-10-02). The linear check
  costs one comparison per id, and the read loop already touches every id.
- **Alternatives considered**:
  - **Validating with `CellUnion::is_valid()` after building**: equivalent, but it
    re-checks each id's validity a second time.
  - **Sorting instead of rejecting**: rejected. It changes the identifiers, which
    defeats the purpose of decoding them as encoded.

## R12. Upstream findings (FR-019, SC-012)

- **Decision**: as in 001 R18. Entries are numbered from **U-003**. One candidate is known
  now:
  - **U-003.** C++ `S2CellUnion::Decode`, together with `S2CellId::Decode`, accepts input
    whose ids are invalid, unsorted or overlapping. It returns `true` and a union for
    which `IsValid()` is false, although `FromVerbatim` REQUIRES `IsValid()` and
    `Contains` relies on it.
  - **Inputs:** version 1, count 2, ids `[0x3000000000000000, 0x1000000000000000]`
    (face 1, then face 0), or count 1 with id 0.
  - **Effect:** in C++ a later `Contains` call silently returns wrong answers, and
    `LeafCellsCovered` can wrap around.

  The follow-up task is created in `tasks.md` before the decode proofs are marked
  complete. The `CELLUNION_DECODE` vectors record what C++ returns for these inputs.
- **Rationale**: the user's standing rule, and FR-019.

## R13. C++ reference vectors (Principle I)

- **Decision**:
  - **Generator.** A new `tools/refgen-cpp/generate_cellunion.cc`, built from the same
    `CMakeLists.txt` as an extra `add_executable`. It writes
    `core/tests/data/cpp_cellunion_vectors.csv` with the seven sections in data-model §10.
    It uses a fixed seed and is deterministic.
  - **Inputs.** Random unions of 0–8 cells at mixed levels, with edge cases added
    explicitly:
    - four siblings, and the cascade of seven;
    - the whole sphere;
    - ranges across faces, and to `end(30)`;
    - every `min_level` 0–30 × `level_mod` 1–3 on cells at levels 28, 29 and 30;
    - unsorted, duplicate, overlapping and invalid encodings.
  - **Tests.**
    - `core/tests/cpp_cellunion_diff.rs` diffs the library against every section. The
      `DENORMALIZE` and `DECODE` diffs are FR-015's and FR-017's differential tests.
    - `verify/tests/union_spec_vs_cpp.rs` checks the model against the same file.
  - **Toolchain.** CI needs no C++ toolchain. The CSV is committed, and regenerated
    locally with the install under `/tmp/s2-install` (present on the workstation).
- **Rationale**: the FR-015 and FR-017 fixes need an oracle (Principle I, as with 001's
  R16). The same vectors make the model's sanity tests meaningful.
- **Alternatives considered**:
  - **Appending sections to `cpp_cellid_vectors.csv`**: rejected. The file is
    `CellId`'s oracle and is diffed by `cpp_cellid_diff.rs`. A separate file keeps each
    oracle with its type.
  - **Go vectors**: rejected. C++ is canonical, and the C++ install is at hand.

## R14. `# Panics` documentation (FR-003, SC-011)

- **Facts**: `cell_union.rs` has no `# Panics` section today. The candidates found by
  reading the code are below; the proofs fix the final list.

  | Function | Panics when |
  |----------|-------------|
  | `contains_cell_id`, `intersects_cell_id`, `intersection_with_cell_id`, `Region::contains_cell`, `Region::intersects_cell` | in debug builds, the cell id is invalid |
  | `normalize`, `from_cell_ids`, `union`, `FromIterator` | an id is invalid. Then `range_max` overflows (debug) or `CellId::level` panics; upstream DCHECKs validity |
  | `from_begin_end`, `from_min_max` | in debug builds, the documented preconditions are broken (`begin`/`end` leaves, `begin ≤ end`) |
  | `denormalize` | in debug builds, `level_mod` is not 1–3 (after the FR-015 fix, there is no release-build panic) |
  | `leaf_cells_covered` | R10 |
- **Decision**: one documentation-only commit, as in 001 R17, listed in the evaluation.
  It covers every public function and trait method in `cell_union.rs`, not only the proven
  ones. Constitution II requires `# Panics` on any public function that can panic, so the
  excluded and tool-limit operations are documented by reading the code, as 001 did. A
  panic that also differs from upstream behaviour becomes a `D-NNN` (FR-010).

## R15. Resource budget (SC-005)

- **Decision**: 001's per-harness limits are unchanged:
  - 4 GB per harness;
  - a 60 s target, a 5-minute soft limit, and a 12-minute hard limit on CI (8 minutes on
    the workstation);
  - 30 minutes in the weekly tier.

  The `CellUnion` per-change suite gets its own total, **35 CPU-minutes**. With 4 jobs,
  that is under 10 minutes on the reference runner, with room for uneven packing. The
  `CellId` suite keeps its 45 CPU-minutes.
- **Projection** (from R0; the "to confirm" rows of R4 are estimated):

  | Group | Harnesses | CPU-minutes |
  |-------|-----------|-------------|
  | Read-only families (`__n0…__n3/4`), about 12 operations | about 55 | about 8 |
  | Normalising families (`__n0…__n2`), about 4 operations | about 12 | about 2 |
  | `denormalize`, `intersection_with_cell_id` | about 6 | about 1 |
  | `union` and `intersection` (`1 × 1`), the union-pair queries (`2 × 2`) | about 20 | about 8, with `intersection`'s 85 s members the largest |
  | Decoding (`__b0…__b17`), after R11 | 18 | unknown; R0 could not measure it before the message change |
  | Properties sharing those harnesses' shapes | about 30 | about 8 |
  | Kani overhead | about 140 | about 3 |

  That is about 30 CPU-minutes without decoding. Decoding is the risk. If its family does
  not fit the remaining 5 minutes, its longest members move to the weekly tier. If even
  that fails, decoding becomes a tool-limit exclusion (spec SC-001).
- **Rationale**: SC-005 sets separate wall-clock budgets per suite. The CPU totals follow
  001's 4-job packing measurements: 33.4 CPU-minutes gave 12.5 minutes of wall time.

## R16. Untrusted-input coverage (FR-018, Constitution II)

- **Facts**: decoding already has the `cellunion` entry in `decoder_robustness.rs`. That
  covers the truncation and mutation sweeps over the seed encodings, which include a
  `from_cell_ids` union. It also has the nightly `decode_cell_union` fuzz target with a
  round-trip assertion.
- **Decision**:
  - Keep both.
  - Add robustness seeds for a non-normalised valid union (four siblings) and an
    unsorted encoding.
  - Add the formal proofs P-UENC-03 and P-UENC-04 over `Bytes(L)`.

  The fuzz target needs no change. After FR-017 every successful decode is a valid union,
  so its round trip is exact.
- **Rationale**: Constitution II asks for bounded robustness tests and a fuzz target. Both
  exist. The proofs add exhaustive coverage of short inputs, where the header logic lives.

## R17. Property tests for the uncovered sizes (FR-007, Constitution III)

- **Facts**: `core/tests/property_tests.rs` has four `CellUnion` quickcheck properties.
  They use unions of 2–3 cells and cover normalisation and point containment only. That
  is not enough to exercise what the proofs leave uncovered: sibling merges, set algebra
  on larger unions, `difference`, the range constructors and multi-cell `denormalize`.
- **Decision**: add quickcheck properties to `core/tests/property_tests.rs`. Each is
  stated against a sampled version of the covered-set meaning: random leaves inside and
  around the cells of random unions.
  - **Inputs.** Random unions of 0–20 cells at mixed levels. Some generators are biased to
    produce complete sibling groups and cascades.
  - **Properties:**
    - `normalize` preserves coverage and returns a normalised union;
    - `union`, `intersection` and `difference` cover the set union, intersection and
      difference;
    - `from_begin_end` and `from_min_max` cover exactly their leaf range and agree with
      `from_range`;
    - `denormalize` preserves coverage and meets the level constraints;
    - `decode(encode(u)) == u` for valid, non-normalised unions.

  The default `QUICKCHECK_TESTS=500` applies.
- **Rationale**: FR-007 requires that tests exercise the sizes and behaviours that the
  proofs cannot reach, and Constitution III asks for invariants as quickcheck properties
  where practical. Together with the C++ vectors (R13), they are the evidence for the
  four tool-limit exclusions.
- **Alternatives considered**: relying on the C++ vectors alone. Rejected: the vectors
  are a fixed sample, while quickcheck draws new inputs on every run.

## R18. Runner: classify CBMC's out-of-memory exit (R0 finding 6)

- **Decision**: `run-verify.sh` also classifies a harness as `OOM` when CBMC's log shows
  `Out of memory` or the process exits with status 6. Today it matches only the solver's
  own out-of-memory message. The change ships in the commit that wires up the
  `CellUnion` suite.
- **Rationale**: in R0, half of the out-of-memory runs ran out during symbolic execution
  or post-processing, before the SAT solver, and were reported as `ERROR`. `OOM` tells a
  maintainer that a split or a smaller bound is needed, while `ERROR` points at a tool
  fault.

