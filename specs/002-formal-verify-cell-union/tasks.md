---

description: "Task list for the formal verification of CellUnion (spec 002)"
---

# Tasks: Formal Verification of `CellUnion`

**Input**: Design documents from `specs/002-formal-verify-cell-union/`: plan.md, spec.md,
research.md (R0–R18), data-model.md, contracts/, quickstart.md. The feature extends 001's
`verify/` crate. 001's task list (`specs/001-formal-verify-base-type/tasks.md`) shows the
established patterns, and the existing files in `verify/` are the style reference.

**Prerequisites**:

- Kani 0.68.0 is installed: `make verify-kani-check` passes.
- For T006–T008 only: a C++ toolchain (`cmake`, `g++`), the upstream checkout at
  `/home/t/sources/s2geometry`, and its install at `/tmp/s2-install` (research R13).

**Tests**: The spec asks for these ordinary tests, so they appear as tasks:

- regression tests that fail without their fix (FR-015, FR-017; Constitution III);
- differential tests against C++ reference vectors (FR-015, FR-017, and the parity sections
  that back the four tool-limit exclusions; Principle I, FR-002);
- quickcheck properties for the sizes and operations the proofs cannot reach (FR-007, R17);
- robustness seeds and a Python test for the decoder change (FR-017, FR-018);
- reference-model sanity tests against the C++ vectors (FR-006, R13).

The Kani harnesses are the verification deliverable itself, so they are implementation tasks.

**Organization**: Tasks are grouped by user story.

- Phase 2 holds the model, the generators, the C++ vectors and the inventory, which every
  story needs.
- Phase 3 belongs to no story. It holds the parity tests, the FR-015 and FR-017 fixes, and
  the R17 property tests. It lands before the proofs that state the fixed behaviour.
- The FR-016 documentation and the `# Panics` sections land in US1 (T030), once the proofs
  have fixed the list (R14).
- Phase 9 holds follow-ups (FR-019). They are **not** implemented in this feature.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on unfinished tasks)
- **[Story]**: The user story the task belongs to (US1–US4)
- Paths are relative to the repository root.

## Memory safety while implementing (read first)

A single CBMC run on a union can pass 14 GB within a minute (research R0). An uncapped run
can take down the session.

- **Every** Kani run goes through a Makefile target: `make verify-union`, `make verify-cellid`,
  `make verify`, `make verify-harness H=…` or `make verify-mutants`. They all apply the
  memory guard of `verify/run-verify.sh`.
- For a throwaway measurement outside the Makefile, cap the run by hand, from `verify/`:
  `timeout -k 10 600 systemd-run --user --scope --unit=cu-probe-<n> -p MemoryMax=4G -p MemorySwapMax=0 cargo kani --exact --harness <h>`.
  Use a unique unit name each time, and run `systemctl --user reset-failed 'cu-probe-*'`
  before reusing names. Read the peak from the scope's `memory.peak`, not `/usr/bin/time`.
- Run separate verification jobs **one after another**: full runs, mutant campaigns and
  measurements. `run-verify.sh` sizes its own parallelism inside one job.
- Never raise a normalising family (`normalize`, `from_cell_ids`, `FromIterator`, `union`)
  above two cells per input: three cells need about 14 GB (R0).

## Upstream findings rule (FR-019, read first)

If a harness, a C++ vector, or reading upstream code shows **undefined or wrong behaviour in
the upstream reference** (C++ canonical, Go, Java, or the generated vectors), do these
**before** marking the affected proof complete:

1. Append a new task to Phase 9 with the next free task ID. Give it a `U-NNN` id (the next
   free one after U-003), the upstream `file:line`, the inputs, the observed behaviour, and
   the behaviour the Rust port documents (data-model §9).
2. Add the same entry under "Upstream findings" in `verify/cell_union/EVALUATION.md`.
3. Cite the `U-NNN` in the affected harness's doc comment.

Never change upstream code. U-003 (C++ `Decode` accepts invalid unions) is already recorded
as T054.

## Facts these tasks rely on

1. **001's Kani facts still hold** (001 tasks.md, "Facts verified on Kani 0.68.0"):
   - `--exact --harness <name>` matches the full module path, so harness modules sit at
     the crate root;
   - an unsatisfiable `kani::cover!` does not fail Kani's own run, but `run-verify.sh` fails
     on it;
   - a failure prints no input values unless the harness is re-run with concrete playback,
     which `run-verify.sh` does.
2. **Concrete lengths** (R0, R4). A symbolic union length is 100–1000× slower. Every union
   and byte input has a const-generic length, and a family `__n0 … __nN` (or `__b0 … __bL`)
   covers every length up to the bound.
3. **The smallest unwind** (R5). An unwind of 6 instead of 3 turned an 85 s intersection
   harness into an out-of-memory run. Start at the largest concrete length plus 1, and raise
   it one step at a time, only when an unwinding assertion fails.
4. **`run-verify.sh` passes its arguments to `cargo kani`**, so `--no-default-features` and
   `--harness cu_` work. It drops `--harness` and `--exact` for its single-harness re-runs.
5. **`--harness cu_` selects only the new suite.** No 001 harness path contains `cu_`
   (checked 2026-10-02). `--harness crash::` would also match `cu_crash::`, which is why the
   001 suite is selected by the feature gate instead (R1).
6. **`io::Error::new(kind, e)` with a custom error type** boxes `e` and formats nothing until
   the error is displayed. `Display` for that `io::Error` prints `e`'s `Display`, so
   `to_string()` is unchanged. Python's `io_err` (`python/src/encoding.rs:20`) and wasm's
   `js_err` both use `Display` (R11).
7. **A `Cell` is built with `Cell::from_cell_id`** (`core/src/s2/cell.rs:119`). It runs
   `to_face_ij_orientation` and the floating-point `ij_level_to_bound_uv`. The two `Region`
   cell-query harnesses must build their `Cell` this way, and R0 did not measure that cost
   (T021 does).
8. **`Level + u8` calls `Level::new`**, which `assert!`s `≤ 30` in release builds too
   (`core/src/s2/coords.rs:72`). That is the FR-015 panic.
9. **`Default` for `CellUnion` is derived** (`core/src/s2/cell_union.rs:52`), so it is
   outside the 42-entry inventory.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Gate the new suite behind a Cargo feature, fix the runner's out-of-memory
classification, and add the Makefile entry points (research R1, R18;
contracts/verify-commands.md).

- [X] T001 Add the `cellunion` feature and the empty `CellUnion` modules to `verify/Cargo.toml` and `verify/src/lib.rs` (research R1, R2; contracts/harness-conventions.md).
  - **`verify/Cargo.toml`:** in `[features]`, add `default = ["cellunion"]` and
    `cellunion = []`, with a comment: the `CellUnion` proofs (spec 002); `make verify-cellid`
    builds without them through `--no-default-features`. Update the header comment so it
    names both `CellId` and `CellUnion` and both specs.
  - **New files**, each starting with the two SPDX lines and
    `// Written for this crate (not ported from upstream S2).` as in `verify/src/domain.rs`:
    `verify/src/union_spec.rs`, `verify/src/union_domain.rs`, `verify/src/cu_crash.rs`, and
    `verify/src/cu_{acc,inv,norm,mem,set,range,den,meas,enc}.rs`. Each gets a one-line `//!`
    doc comment naming its data-model section or property area.
  - **`verify/src/lib.rs`:**
    - `pub mod union_spec;`, declared unconditionally, next to `pub mod spec;`;
    - `union_domain` and the ten `cu_*` modules, each behind
      `#[cfg(all(kani, feature = "cellunion"))]`;
    - extend the crate doc comment to name spec 002 and link `cell_union/INVENTORY.md` and
      `cell_union/PROPERTIES.md`.
  - **Checks:**
    - `cd verify && cargo build && cargo test` succeeds on the pinned stable toolchain;
    - `cd verify && cargo build --no-default-features` succeeds;
    - `cargo kani --only-codegen` and `cargo kani --only-codegen --no-default-features` both
      succeed. Codegen needs no memory cap.
- [X] T002 [P] Make `verify/run-verify.sh` report CBMC's own out-of-memory exit as `OOM` (research R18).
  - **Find the real text first.** Today the `ERROR` re-run at `verify/run-verify.sh:211–217`
    greps its log for `out of memory`, yet R0 saw CBMC's pre-solver out-of-memory exit
    (status 6) reported as `ERROR`. Reproduce it with a throwaway harness in a scratch copy
    of `verify/`, not committed: the R0 shape that dies before the solver,
    `from_cell_ids` on 3 valid cells with `VERIFY_MEM_PER_HARNESS=2G`. Read
    `target/run-verify/rerun.log` and the main log, and note exactly what Kani prints
    (for example a `CBMC failed with status 6` line, or nothing at all).
  - **Change:** classify the harness as `OOM` when the re-run log matches that text, or when
    the re-run's exit status shows CBMC's status 6. Keep the existing match. Update the
    "Statuses" comment at the top of the script.
  - **Check:** the scratch harness is now reported as `OOM`, and a deliberately failing
    harness is still `FAILED` with its playback test.
- [X] T003 Add `verify-cellid` and `verify-union` to `Makefile`, and widen `verify` to both suites (contracts/verify-commands.md). Needs T001.
  - Follow the existing style: `.PHONY`, a `## ` help comment, and a dependency on
    `verify-kani-check`.
    - **`verify-cellid`:** `cd verify && cargo test`, then
      `cd verify && ./run-verify.sh --no-default-features`. Help: "Run the CellId proofs only
      (spec 001)".
    - **`verify-union`:** `cd verify && cargo test`, then
      `cd verify && ./run-verify.sh --harness cu_`. Help: "Run the CellUnion proofs only
      (spec 002)".
    - **`verify`:** the recipe is unchanged. With the default feature it now runs both suites.
      Change its help comment to "Run the CellId and CellUnion proofs …".
  - Update the section's header comment, which names only `CellId` and spec 001.
  - Leave `test`, `verify-harness`, `verify-mutants` and `verify-slow` unchanged here.
  - **Check:** `make help` lists the new targets. Then run `make verify-cellid` **once, on its
    own**: it must report the same 196 harnesses as 001's last run, all `SUCCESSFUL`. Note its
    wall-clock time for the CellId baseline in T009.

**Checkpoint**: `make verify-cellid` reproduces 001's result. `make verify-union` runs and
finds no harnesses yet. `make test` is unchanged.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The reference model of a union, the union and byte generators, the C++
reference vectors that sanity-check the model, and the final operation inventory.

**⚠️ CRITICAL**: No user-story work starts until this phase is complete.

- [X] T004 Implement the reference model in `verify/src/union_spec.rs` (data-model §1; research R3).
  - **Independence (FR-006):** use only `crate::spec` (`Decomposed`, `compose`, `decompose`,
    `range`, `contains`, `parent`, `child_position`, `span`, `pow4`) and plain integer
    arithmetic. Never call `s2rst::s2::CellUnion` or any `CellId` method. Write each function
    from upstream `s2cell_union.h`/`.cc`, and cite the upstream function in its doc comment.
  - **Functions** (inputs are slices of decompositions unless they say raw `u64`):
    - `covers(ds: &[Decomposed], x: u64) -> bool`: some `i` has
      `range(ds[i]).0 ≤ x ≤ range(ds[i]).1`. `x` is a leaf index.
    - `valid_union_d(ds) -> bool`: `range(ds[i−1]).1 < range(ds[i]).0` for every `i ≥ 1`.
    - `valid_union(ids: &[u64]) -> bool`: every id decomposes, and the decompositions satisfy
      `valid_union_d`.
    - `sibling_group(a, b, c, d: Decomposed) -> bool`: same face, the same level `ℓ ≥ 1`, the
      same `parent`, and `child_position(·, ℓ)` equal to 0, 1, 2, 3 in that order.
    - `normalized_union_d(ds)` and `normalized_union(ids: &[u64])`: valid, and no window of
      four consecutive members is a `sibling_group`.
    - `leaf_count(ds) -> u128`: `Σ span(level_i)`. It is `u128` so that it is total over
      any sequence.
    - `contains_id(ds, c: Decomposed) -> bool`: some `i` has `spec::contains(ds[i], c)`.
    - `intersects_id(ds, c) -> bool`: some `i` has overlapping leaf ranges with `c`.
    - `denorm_level(level: u8, m: u8, k: u8) -> u8`: with `L = max(m, level)`, it is
      `min(30, L + (30 − (L − m)) mod k)` for `k > 1`, and `L` for `k = 1`. This is C++'s
      formula (`s2cell_union.cc:215–223`), not the Rust `offset` form.
    - `denormalize(ds, m, k) -> Vec<u64>`: for each `d` in order, with `nl = denorm_level`:
      `compose(d)` if `nl == d.level`, otherwise `compose({face, nl, d.path·4^(nl−level) + p})`
      for `p` in `0..4^(nl−level)`.
    - `encoding(ids: &[u64]) -> Vec<u8>`: byte `1`, then `n` as 8 little-endian bytes, then
      each id as 8 little-endian bytes.
    - `decode(bytes: &[u8]) -> Option<Vec<u64>>`: `None` unless the input has a version byte
      `≤ 1`, then a count `n ≤ 1_000_000`, then at least `8n` more bytes whose `n` ids satisfy
      `valid_union`. Otherwise it returns those ids in order. Trailing bytes are ignored.
      Compare `n` with `(len − 9) / 8` so that nothing overflows.
  - **Unit tests** go in a new sibling file `verify/src/union_spec_tests.rs` (Constitution
    III). It starts with the two SPDX lines and a `//!` comment saying the tests are original,
    and it is wired in at the end of `union_spec.rs` with `#[cfg(test)]`,
    `#[path = "union_spec_tests.rs"]` and `mod union_spec_tests;`. The hand-made cases:
    - the four children of face 0 form a `sibling_group`, and the same four out of order do
      not;
    - `denorm_level(29, 1, 3) == 30` and `denorm_level(30, 1, 2) == 30`;
    - `decode(&encoding(&[f0, f3]))` is `Some`, and `decode` of the unsorted `[f3, f0]` is
      `None`.
  - **Check:** `cd verify && cargo test` and `cargo build --no-default-features` pass.
- [X] T005 Implement the generators in `verify/src/union_domain.rs` (data-model §2; contracts/harness-conventions.md, "Generators"). Needs T004.
  - The module is `#[cfg(all(kani, feature = "cellunion"))]`. Build every union from the
    model, through `crate::domain::any_decomposed` and `spec::compose`. Never use
    `kani::assume(u.is_valid())` or the implementation's `normalize`.
  - **Every generator produces exactly `N` cells or `L` bytes.** The length is a const
    generic, never symbolic (fact 2).
    - `any_seq::<N>() -> (Vec<CellId>, [u64; N])`: `N` symbolic `u64` values, wrapped in
      `CellId(..)` and pushed into a `Vec` with capacity `N`, plus the raw values. Harnesses
      compare the model against the raw values, never against `CellId::id()`.
    - `any_valid_seq::<N>() -> (Vec<CellId>, [Decomposed; N])`: each element
      `CellId(spec::compose(d))` with `d = any_decomposed()`.
    - `any_valid_union::<N>()`: `any_valid_seq`, plus
      `kani::assume(spec::range(d[i−1]).1 < spec::range(d[i]).0)` for `0 < i < N`.
    - `any_norm_union::<N>()`: `any_valid_union`, plus
      `kani::assume(!union_spec::sibling_group(..))` on every window of four.
    - `any_level_mod() -> u8`: a `u8` with `kani::assume((1..=3).contains(&k))`.
    - `with_bytes::<L>(f: impl FnOnce(&[u8]))`: a symbolic `[u8; L]` passed whole as
      `&bytes[..]`. No `unsafe`.
    - `any_leaf() -> (CellId, u64)`: `crate::domain::any_valid_at(30)`, returned as the leaf
      cell and its `spec::leaf_index`.
  - **Output readers.** These are the only places that read an implementation output, and
    they use only `CellId` methods proven in 001 (data-model §7):
    - `out_covers(ids: &[CellId], leaf: CellId) -> bool`: some `c` has
      `c.range_min() <= leaf && leaf <= c.range_max()` (P-RANGE-01);
    - `out_decomposed(ids: &[CellId]) -> Option<Vec<Decomposed>>`: `spec::decompose` of
      each id. It is used only to check `normalized_union_d` on outputs (R3).
  - Each function's doc comment names its data-model §2 domain.
- [X] T006 [P] Prepare the C++ reference build out of tree, and check that it reproduces the committed `CellId` vectors (research R13; `tools/refgen-cpp/README.md`).
  - Run `cd /home/t/sources/s2geometry && cmake --install build`. It installs to
    `/tmp/s2-install`.
  - Build outside the repository:
    `cmake -S tools/refgen-cpp -B /tmp/refgen-cpp-build -DCMAKE_BUILD_TYPE=Release -DCMAKE_PREFIX_PATH=/tmp/s2-install && cmake --build /tmp/refgen-cpp-build`.
  - **Baseline:**
    `LD_LIBRARY_PATH=/tmp/s2-install/lib /tmp/refgen-cpp-build/generate_cellid | cmp - core/tests/data/cpp_cellid_vectors.csv`
    must report no difference. If it does, **stop and report**: the oracle has drifted.
- [X] T007 Create the generator `tools/refgen-cpp/generate_cellunion.cc` and the committed vectors `core/tests/data/cpp_cellunion_vectors.csv` (data-model §10; research R13). Needs T006.
  - **Header:** copy the SPDX lines and the purpose comment from `generate_cellid.cc`, and
    adapt the comment: the oracle for `core/tests/cpp_cellunion_diff.rs` and
    `verify/tests/union_spec_vs_cpp.rs`, deterministic, with a fixed seed.
  - **Randomness:** copy the `rng` and `RandCell()` helpers from `generate_cellid.cc` with a
    new seed constant. Add `RandUnion(max_cells)`. It picks an anchor cell at a random level
    0–20 and draws 0–`max_cells` cells among the anchor's descendants up to four levels
    below it, so that overlaps, nesting and sibling groups are frequent. All cells are
    valid, because upstream `Normalize` requires it.
  - **Lists and bytes:** a list of ids is lowercase hex with `;` between ids, and an empty
    field for an empty list. Bytes are lowercase hex. Booleans are `0`/`1`.
  - **Sections**, in this order, each starting with a `# NAME` line:
    - `# CELLUNION_NORMALIZE`: `input,output`, where output is
      `S2CellUnion(input).cell_ids()`. Inputs:
      - 300 `RandUnion(8)`, unsorted and with duplicates;
      - the four children of each face;
      - a cascade of seven: three children of a cell plus the four children of its fourth
        child;
      - a cell together with one of its descendants;
      - the six faces.
    - `# CELLUNION_DENORMALIZE`: `input,min_level,level_mod,output`, where the input is a
      normalised union and the output is `Denormalize(min_level, level_mod, &out)`.
      - For every `min_level` 0–30, `level_mod` 1–3 and cell level `ℓ` 0–30 with
        `max(min_level, ℓ) − ℓ ≤ 2`: one random cell at level `ℓ`. This bounds each output at
        16 cells, and it includes every combination FR-015 names, such as level 29 with
        `min_level` 1 and `level_mod` 3.
      - 200 random normalised unions whose cells all have levels `≥ min_level − 2`.
    - `# CELLUNION_SETOPS`: `a,b,union,intersection,difference,contains,intersects`. A and B
      are normalised. 300 pairs, with B drawn around A's anchor half of the time, so that
      they overlap. Add the empty union on either side, and the whole sphere.
      `contains` and `intersects` are `a.Contains(b)` and `a.Intersects(b)`.
    - `# CELLUNION_FROM_BEGIN_END`: `begin,end,output`, where output is
      `FromBeginEnd(begin, end).cell_ids()`. Inputs:
      - `(Begin(30), End(30))`;
      - an empty range (`begin == end`);
      - a single leaf;
      - a range that crosses a face boundary;
      - ranges that end at `End(30)`;
      - 100 random leaf pairs, sorted;
      - 200 near pairs `(b, b.advance(k))`, with `k` random below `4^12`.
    - `# CELLUNION_LEAF_CELLS`: `ids,count`, with the count from `LeafCellsCovered()` in
      decimal. Use 200 normalised unions and 100 valid non-normalised unions, built with
      `FromVerbatim` of a normalised union with one cell replaced by its four children.
    - `# CELLUNION_ENCODE`: `ids,bytes`, from `Encode` into an `Encoder`, for the same
      unions as `LEAF_CELLS`.
    - `# CELLUNION_DECODE`: `bytes,ok,valid,ids`. `ok` is `Decode`'s return value, and
      `valid` is the decoded union's `IsValid()` (0 when `ok` is 0). `ids` are the decoded
      ids, empty when `ok` is 0. Inputs:
      - the `ENCODE` encodings;
      - four sibling cells and the cascade of seven, encoded verbatim;
      - truncations of a 3-cell encoding to 0, 1, 8, 9, 16 and 24 bytes;
      - version 0 and version 2;
      - count 1,000,001 with no ids;
      - a count larger than the bytes that follow;
      - a valid encoding with trailing bytes;
      - invalid ids: `0`, `0xd000000000000000` (face 6) and `0x1000000000000002` (bad
        marker);
      - `[face 1, face 0]` (unsorted), a duplicate `[c, c]`, and an overlap `[parent, child]`;
      - 200 random byte strings of length 0–40.
  - **Build:** add `add_executable(generate_cellunion generate_cellunion.cc)` and its
    `target_link_libraries` to `tools/refgen-cpp/CMakeLists.txt`. Add the seven sections to
    the "Sections emitted" table in `tools/refgen-cpp/README.md`, with
    `cpp_cellunion_vectors.csv` named as their file.
  - **Generate:**
    `LD_LIBRARY_PATH=/tmp/s2-install/lib /tmp/refgen-cpp-build/generate_cellunion > core/tests/data/cpp_cellunion_vectors.csv`.
    A second run must give a byte-identical file. Keep the file under about 1 MB, and lower
    the random row counts if it is larger.
  - **Check the C++ results against FR-015 and FR-017's premises.** The `DECODE` rows for
    `[face 1, face 0]` and `[c, c]` must have `ok = 1, valid = 0`: that is U-003 (T054).
    Anything else unexpected in upstream behaviour falls under the upstream findings rule.
- [X] T008 Write the model sanity tests in `verify/tests/union_spec_vs_cpp.rs` (research R13). Needs T004 and T007. They run under plain `cargo test`.
  - **Loading:** `include_str!("../../core/tests/data/cpp_cellunion_vectors.csv")`, with the
    `section_lines` helper from `verify/tests/spec_vs_cpp.rs` and a hex-list parser. Every
    test asserts that its section is non-empty.
  - **Test-only helper:** `intervals(ids) -> Vec<(u64, u64)>`, the merged leaf-index
    intervals of a list of valid ids, computed with `spec::decompose` and `spec::range`. It is
    used to compare covered sets.
  - **One test per section:**
    - `NORMALIZE`: `normalized_union(output)`, and `intervals(output) == intervals(input)`.
    - `DENORMALIZE`: `union_spec::denormalize(input, m, k) == output`.
    - `SETOPS`:
      - the intervals of `union`, `intersection` and `difference` equal the set union,
        intersection and difference of `intervals(a)` and `intervals(b)`;
      - `contains == b.iter().all(|x| contains_id(a, x))`;
      - `intersects == b.iter().any(|x| intersects_id(a, x))`.
    - `FROM_BEGIN_END`: `normalized_union(output)`, and `intervals(output)` is
      `[li(begin), li(end) − 1]`, or empty when `begin == end`. `li` is the leaf index, and
      `li(End(30)) = 6·4^30`.
    - `LEAF_CELLS`: `leaf_count == count`.
    - `ENCODE`: `encoding(ids) == bytes`.
    - `DECODE`: `union_spec::decode(bytes) == (ok && valid).then_some(ids)`.
  - **Check:** `cd verify && cargo test` passes. A failure means the model or the vectors
    are wrong. Fix the model only where it disagrees with upstream's documented meaning,
    never to match an implementation detail.
  - **Commit** T007 and T008 together, for example
    `test(verify): C++ CellUnion reference vectors and model sanity tests`.
- [X] T009 [P] Write `verify/cell_union/INVENTORY.md` (FR-002; data-model §3), and start `verify/cell_union/EVALUATION.md`.
  - **Layout:** the two tables of `verify/INVENTORY.md`, with these changes from data-model
    §3:
    - table 1 gains `union_domain` and `bounds` columns;
    - the `kind` values are `constructor | query | set-op | transform | measure | encoding | access | trait`.

    The intro states the C-&lt;op&gt; obligation and the domain rule of spec FR-003: union
    domains `Seq | ValidSeq | ValidUnion | NormUnion`; preconditions from rustdoc,
    `debug_assert!`, or upstream (`FromVerbatim` REQUIRES `IsValid()` is the class
    invariant).
  - **Rows:** 42, checked against `core/src/s2/cell_union.rs` and `core/src/s2/encoding.rs`:
    29 inherent methods in file order, then the 13 trait methods. The planned crash harness
    is the family name.
    - **Accessors** (`Seq`, `n ≤ 3`, shared harness `cu_acc::p_uacc_01__n*`): `new` (no
      union argument, whole domain), `from_verbatim`, `num_cells`, `cell_ids`, `is_empty`,
      `Deref::deref`, `IntoIterator for CellUnion`, `IntoIterator for &CellUnion`.
    - **Invariant checks** (`Seq`, `n ≤ 4`): `is_valid` → `cu_inv::p_uinv_01__n*`, and
      `is_normalized` → `cu_inv::p_uinv_02__n*` (shared).
    - **Normalising** (`ValidSeq`, `n ≤ 2`, `cu_crash::c_<op>__n*`): `from_cell_ids`,
      `normalize` and `FromIterator::from_iter`. Precondition: every id valid (upstream
      `Normalize`; not in rustdoc). Unreached: the sibling merge and its cascades, which
      need 4 cells.
    - **Cell queries** (`ValidUnion` × `Valid`, `n ≤ 3`): `contains_cell_id`,
      `intersects_cell_id` (`debug_assert!(id.is_valid())`), `Region::contains_cell` and
      `Region::intersects_cell` (`cu_crash::c_region_contains_cell__n*` and
      `c_region_intersects_cell__n*`; the `Cell` is built from a valid id, fact 7).
    - **Union queries** (`ValidUnion` × 2, `2 × 2`): `contains_union`, `intersects_union`.
    - **Measure** (`ValidUnion`, `n ≤ 3`): `leaf_cells_covered`, citing FR-016.
    - **Set operations** (`ValidUnion` × 2): `union` and `intersection` at `1 × 1`;
      `intersection_with_cell_id` at `n ≤ 2` × `Valid`. Unreached: merges, and interleavings
      of more than one cell per side.
    - **Transform:** `denormalize`. `ValidUnion`, `n ≤ 1`, `min_level` any `Level`,
      `level_mod` 1–3 (`debug_assert!`, upstream `DCHECK`). Loop bound: level gap `≤ 2`.
      Unreached: more than one cell, and gaps above 2.
    - **Constant:** `whole_sphere`, whole domain, `cu_crash::c_whole_sphere`.
    - **Encoding:** `S2Encode::encode` (`Seq`, `n ≤ 3`, shared `cu_enc::p_uenc_01__n*`), and
      `S2Decode::decode` (`Bytes`, `L ≤ 17`, shared `cu_enc::p_uenc_04__b*`; in scope on the
      condition in SC-001).
    - **`float-geometry` (9):** `contains_point`, `expand_by_radius`, `average_based_area`,
      `approx_area`, `exact_area`, and the `Region` methods `cap_bound`, `rect_bound`,
      `cell_union_bound` (it goes through `cap_bound`) and `contains_point`.
    - **`excluded: depends on CellId::all_neighbors` (1):** `expand_at_level`.
    - **`tool-limit` (5), each with its measurement:**
      - `Display::fmt`, through `CellId::to_token`, as 001 measured;
      - `difference`: over 5 minutes at `1 × 1` cells in a one-level window, and out of
        memory at 6 GB in a two-level window (R0);
      - `from_begin_end`: out of memory at 6 GB for a range inside one level-29 cell, also
        with a per-loop unwind (R0);
      - `from_min_max`: calls `from_begin_end`, so it inherits that measurement;
      - `from_range`: not measured in R0; T021 measures it.

      Each of these rows names the R17 property test and the C++ vector section that check
      it instead.
  - **Table 2:** pre-fill `properties` from data-model §4.1. Leave `result_validity`,
    `undocumented_panic`, `panics_doc` and `upstream_findings` to T029, T030 and T044. Set
    `upstream_findings = U-003` on the `S2Decode::decode` row.
  - **Summary line:** 27 in scope (64%; SC-002 needs 60%), 15 excluded. Add a footer: the
    derived impls (`Clone`, `Debug`, `Default`, `PartialEq`, `Eq`, `Hash`) and the optional
    `serde` derives are outside FR-002's definition (spec Assumptions).
  - **`verify/cell_union/EVALUATION.md`:** create it with the sections of
    `verify/EVALUATION.md`, initially empty except as noted:
    - "Effort log": a date / phase / person-hours table;
    - "Exclusions and tool limits": copy R0's table, linked to research.md;
    - "Bounds reached": empty for now;
    - "Upstream findings": U-003 from research R12, pointing to T054;
    - "`# Panics` documentation added": empty;
    - "Defects found": FR-015, FR-016 and FR-017, to be filled in;
    - "Measurements": record the `make verify-cellid` baseline from T003, with the machine.

**Checkpoint**: `cd verify && cargo test` passes against the C++ vectors, and the inventory
lists 27 in-scope rows.

---

## Phase 3: Parity Tests, Known-Defect Fixes and Untrusted-Input Coverage (FR-015, FR-017, FR-018), no story

**Purpose**: The behaviour-level changes, each its own commit, kept apart from the
verification work (research R9, R11, R13, R16, R17).

- Every commit passes `make test`.
- T013 must land before T027 (the `denormalize` crash proof), and T016 before T020 and
  T028 (the decode proofs). The proofs state the fixed behaviour.
- C++ is needed only to regenerate vectors, never by `make test` or CI.

- [X] T010 Write the parity differential tests in `core/tests/cpp_cellunion_diff.rs` (Principle I; research R13). Needs T007.
  - **Header:** the two SPDX lines and the "Written for this crate" line, then a `//!` doc
    comment modelled on `core/tests/cpp_cellid_diff.rs` that names the generator and the
    CSV. Load the CSV with `include_str!("data/cpp_cellunion_vectors.csv")`, and copy that
    file's `section_lines` helper.
  - **Tests**, each asserting that its section is non-empty:
    - `normalize_matches_cpp`: `CellUnion::from_cell_ids(input).cell_ids() == output`.
    - `set_ops_match_cpp`: `a.union(&b)`, `a.intersection(&b)` and `a.difference(&b)` equal
      the C++ lists, and `a.contains_union(&b)` and `a.intersects_union(&b)` equal the flags.
      Build `a` and `b` with `from_verbatim`.
    - `from_begin_end_matches_cpp`:
      - `CellUnion::from_begin_end(begin, end).cell_ids() == output`;
      - when `begin != end`, `CellUnion::from_min_max(begin, end.prev())` gives the same
        ids;
      - `CellUnion::from_range(begin, end)`, normalised with `from_cell_ids`, gives the
        same ids.
    - `leaf_cells_covered_matches_cpp`: `from_verbatim(ids).leaf_cells_covered()` as `u64`
      equals the count.
    - `encode_matches_cpp`: the bytes from `S2Encode::encode` equal the C++ bytes.
  - **Check:** `cargo test -p s2rst --test cpp_cellunion_diff` passes. Any failure is a new
    parity defect: **stop and report** it before going on (FR-010). Do not fix it inside
    this task.
  - **Commit** on its own, for example
    `test(s2): differential tests for CellUnion against C++`.
  - **Outcome (2026-10-03):** the `from_range` check failed on 301 of 307 rows. That is
    D-002 (named D-001 in its fix commit; 001 already has a D-001): `from_range` overshoots
    past `end` and drops leaves, and through
    `ShapeIndex::skip_cell_range` it built indexes with overlapping cells and gaps. The user
    chose to fix it on this branch as its own commit (FR-010). The other parity tests
    passed and were committed first. The D-002 commit then added `from_range_matches_cpp`,
    a unit test, a `ShapeIndex` regression test, binding tests and the fix: `from_range`
    delegates to `from_begin_end`, and the internal callers use the C++ functions
    (`verify/cell_union/EVALUATION.md`, "Defects found").
- [X] T011 Write the FR-015 regression test in a new file `core/src/s2/cell_union_tests.rs` (Constitution III; research R9).
  - **Header:** the two SPDX lines from `core/src/s2/cell_id_tests.rs`, then a `//!`
    comment saying the tests are original and not ported from upstream S2.
  - **Test** `denormalize_clamps_to_max_level`, using `use super::*;`:
    - a level-29 cell with `min_level` 1 and `level_mod` 3 gives its four level-30
      children;
    - a leaf with `min_level` 1 and `level_mod` 2 is returned unchanged;
    - for every `min_level` 0–30, `level_mod` 1–3, and cell level `ℓ` with
      `max(min_level, ℓ) − ℓ ≤ 2`, the call does not panic. Every member then has level
      `≥ min_level`, and `(level − min_level) % level_mod == 0` unless its level is 30.

    Add a comment that cites C++ `S2CellUnion::Denormalize` (`s2cell_union.cc:215–223`).
    The remaining combinations, `ℓ < min_level − 2`, take the `level < min_level` branch,
    which does no level arithmetic. Their outputs have `4^(min_level − ℓ)` cells, so they are
    left to that argument, which T047 records in the evaluation.
  - **Wiring:** append `#[cfg(test)]`, `#[path = "cell_union_tests.rs"]` and
    `mod cell_union_tests;` at the end of `core/src/s2/cell_union.rs`, as
    `core/src/s2/cell_id.rs` does.
  - **Check:** `cargo test -p s2rst --lib s2::cell_union::cell_union_tests` must **fail**
    with `level must be 0..=30`.
- [X] T012 Add `denormalize_matches_cpp` to `core/tests/cpp_cellunion_diff.rs`: `from_verbatim(input).denormalize(Level::new(min_level), level_mod).cell_ids() == output` for every `DENORMALIZE` row. Needs T010.
  - **Check:** it **fails** (panics) on the level-29 and level-30 rows before T013.
- [X] T013 Fix `CellUnion::denormalize` in `core/src/s2/cell_union.rs` (research R9), then commit FR-015.
  - **Change:** in the `level_mod > 1` branch, compute the rounded level without overflow,
    as `level.as_u8().saturating_add(level_mod - offset)` (implemented this way instead of
    the planned `u32`, which needed a cast). Clamp it with `.min(30)` and build the `Level`
    with `Level::new` on the clamped value. Then remove the now-redundant
    `.min(Level::MAX)` line, or keep it if it still clamps the other branches. Add a one-line
    comment: C++ rounds in `int` and clamps to `kMaxLevel`.
  - **Rustdoc:** document the parameters and the clamp ("levels are rounded up to a multiple
    of `level_mod` above `min_level`, but never past 30, as C++ `Denormalize` does") and the
    `level_mod` 1–3 precondition. The `# Panics` section waits for T030.
  - **Checks:** T011 and T012 pass, and `make test` passes.
  - **Commit:** T011, T012 and T013 together, for example
    `fix(s2): clamp CellUnion::denormalize to the maximum level like C++`.
- [X] T014 Add the FR-017 regression tests to `core/src/s2/cell_union_tests.rs` (research R11). Needs T011.
  - **Tests** (decode with `<CellUnion as S2Decode>::decode(&mut &bytes[..])`):
    - `decode_keeps_four_siblings`: the encoding of `from_verbatim` of the four children of
      a cell decodes to exactly those four ids. It fails before the fix.
    - `decode_rejects_unsorted`: `[face 1, face 0]` is an `InvalidData` error. It fails
      before the fix.
    - `decode_rejects_duplicate` and `decode_rejects_overlap` (`[parent, child]`).
    - `decode_rejects_invalid_id`: id `0`. This already passes.
    - `decode_error_messages_unchanged`: `to_string()` of the errors for version 2 and for
      count 1,000,001 equals `unsupported encoding version 2` and
      `too many cells (1000001; max is 1000000)`. The error for an unsorted input reads
      `cell ids are not a valid cell union: unsorted or overlapping`.
  - Add a comment that cites C++ `S2CellUnion::Decode` (`s2cell_union.cc:544–562`) and
    names the deliberate deviation.
  - **Check:** the four-sibling, unsorted, duplicate and overlap tests **fail** on the
    current code.
- [X] T015 Add `decode_matches_cpp` to `core/tests/cpp_cellunion_diff.rs` (data-model §10). Needs T010.
  - **Rule:** a row with `ok = 1, valid = 1` must decode to `Ok` with exactly `ids`. Every
    other row must be `Err`: either C++ rejected it too, or it is a U-003 row
    (`ok = 1, valid = 0`), the documented deviation. Count the rows in each class, and
    assert that each class is non-empty.
  - **Check:** it **fails** on the four-sibling and unsorted rows before T016.
- [X] T016 Fix `S2Decode for CellUnion` in `core/src/s2/encoding.rs` (research R11). Needs T014 and T015.
  - **Errors formatted only when displayed** (fact 6): add a private enum
    `CellUnionDecodeError { Version(u8), TooManyCells(u64), NotValid }` that derives
    `Debug`. Implement `Display` with exactly today's texts and the new one:
    - `unsupported encoding version {v}`;
    - `too many cells ({n}; max is {MAX_ENCODED_CELLS})`;
    - `cell ids are not a valid cell union: unsorted or overlapping`.

    Implement `std::error::Error`, and pass the value to
    `io::Error::new(io::ErrorKind::InvalidData, …)`. Leave the other decoders'
    `format!` calls alone, for example `Rect`'s at line 238: they are outside this feature.
  - **Validation:** in the read loop, after `CellId::decode`, reject an id unless
    `prev.range_max() < id.range_min()`, where `prev` is the previously pushed id. Return
    `CellUnion::from_verbatim(ids)` instead of `from_cell_ids`.
  - **Rustdoc** on the `impl S2Decode for CellUnion` block, with `# Errors`:
    - the identifiers are kept exactly as encoded and are not normalised, as in C++;
    - deliberate deviations from C++ (Constitution I): invalid ids are rejected, and so are
      ids that are unsorted, duplicated or overlapping. The reason is that a decoded union
      then always satisfies `is_valid()`, which the other operations rely on, even for
      untrusted bytes. Name U-003.
  - Update the comment above `impl S2Decode for CellId` (`encoding.rs:151–153`), which says
    the `CellUnion` decoder normalises.
  - **Checks:** T014 and T015 pass, and the existing `encoding.rs` tests
    (`test_cell_union_roundtrip`, `test_cell_union_wire_format`) still pass.
- [X] T017 [P] Update the untrusted-input coverage for FR-017 in `core/tests/decoder_robustness.rs` and `fuzz/fuzz_targets/decode_cell_union.rs` (research R16; FR-018).
  - **`core/tests/decoder_robustness.rs`:** in the seed list near line 830, add two seeds:
    - `enc(&CellUnion::from_verbatim(…))` of the four children of a cell;
    - a hand-built unsorted encoding: version `1`, count `2`, then the ids of face 1 and
      face 0, each as 8 little-endian bytes.

    The existing truncation and mutation sweeps then cover them.
  - **`fuzz/fuzz_targets/decode_cell_union.rs`:** keep the assertion, and rewrite the doc
    comment. The decoder now keeps ids verbatim and rejects anything that is not a valid
    union, so a successful decode is a valid union, and its re-encoding decodes to the same
    ids.
  - **Check:** `cargo test -p s2rst --test decoder_robustness` passes. Smoke-run the fuzz
    target:
    `cd fuzz && cargo +nightly fuzz run decode_cell_union -- -max_total_time=60`. It must
    finish without a crash.
- [X] T018 [P] Add the binding tests for FR-017 to `python/tests/test_encoding.py` and `wasm/tests/web.rs` (Constitution III, IV), then commit FR-017.
  - **Python:** add `test_decode_cell_union_rejects_unsorted`. Build the unsorted encoding
    as `bytes([1]) + (2).to_bytes(8, "little")`, then the ids `0x3000000000000000` and
    `0x1000000000000000`, each with `.to_bytes(8, "little")`. Assert that
    `s2rst.decode_cell_union(data)` raises `ValueError` matching `not a valid cell union`.
    Add a second test: four sibling cells round-trip with `len(...) == 4`. Build their
    bytes by hand the same way, from the ids of the four children of face 0 (from `s2rst`'s
    `CellId` API, or as literals).
  - **wasm:** add `#[wasm_bindgen_test] fn test_cell_union_decode_rejects_unsorted()` next
    to `test_encode_decode_roundtrips` (`wasm/tests/web.rs:2496`). Build the same bytes with
    `vec![1u8]` and `extend_from_slice(&x.to_le_bytes())`, and assert
    `CellUnion::decode(&bytes).is_err()`.
  - The binding code and the stub are unchanged (plan, Constitution Check IV).
  - **Checks:** `cd python && make test` and `cd wasm && wasm-pack test --node` pass, and so
    does `make test`.
  - **Commit:** T014–T018 together as the FR-017 change, for example
    `fix(s2): decode CellUnion verbatim and reject invalid unions`.
- [X] T019 Add the `CellUnion` quickcheck properties to `core/tests/property_tests.rs` (research R17; FR-007). Needs T013 and T016.
  - **Helper** `rand_union(seed: u64, max_cells: usize, sibling_bias: bool) -> Vec<CellId>`:
    - a SplitMix64 over `seed`;
    - an anchor cell at a random level 0–20, and up to `max_cells` cells among its
      descendants up to four levels below it;
    - with `sibling_bias`, sometimes add all four children of a random descendant, or a
      full cascade.
  - **Helper** `covered(ids, leaf) -> bool`: a linear scan with
    `range_min() <= leaf <= range_max()`.
  - **Sample leaves:** `range_min`, `range_max`, the leaf before and the leaf after each
    input and output cell (when valid), plus 16 random leaves in the anchor's range.
    Membership must agree on every sample.
  - **Properties** (`#[quickcheck]`, the default `QUICKCHECK_TESTS=500` applies):
    - `prop_cell_union_normalize_preserves_coverage`: unions of up to 20 cells, biased to
      siblings. The result is normalised, covers the same samples, and normalising it again
      changes nothing.
    - `prop_cell_union_set_ops_match_covered_sets`: `union`, `intersection` and `difference`
      cover the set union, intersection and difference on every sample. `union` is
      normalised, and the other two are normalised when their inputs are.
    - `prop_cell_union_queries_match_covered_sets`: `contains_cell_id`,
      `intersects_cell_id`, `intersection_with_cell_id`, `contains_union` and
      `intersects_union` agree with sampled coverage on normalised unions of up to 20 cells.
    - `prop_cell_union_ranges_cover_exactly`: for leaves `b ≤ e` around the anchor,
      `from_begin_end(b, e)` covers exactly `[b, e)` on the samples and is normalised. It
      equals `from_min_max(b, e.prev())` when `b < e`, and `from_range(b, e)` after
      normalising.
    - `prop_cell_union_denormalize_preserves_coverage`: `min_level` 0–30, `level_mod` 1–3,
      and cells at levels `≥ min_level − 2`. Coverage is preserved, and every member meets
      the level constraints unless its level is 30.
    - `prop_cell_union_decode_roundtrips_verbatim`: a valid non-normalised union (a
      normalised one with one cell split into its children) decodes to exactly its ids.
    - `prop_cell_union_decode_rejects_invalid_order`: swapping two members of a valid union
      of 2 or more cells, or duplicating one, makes `decode` fail.
  - **Check:** `cargo test -p s2rst --test property_tests cell_union` passes. A failure is
    a parity defect: **stop and report** (FR-010).
  - **Commit** on its own, for example
    `test(s2): CellUnion quickcheck properties for larger unions`.

**Checkpoint**:

- SC-008 and SC-010 are met, and `make test` passes.
- `git log --oneline master..` shows the parity-test, FR-015, FR-017 and property-test
  commits, each touching only its own files.

---

## Phase 4: User Story 1 — Proven absence of crashes for `CellUnion`, including decoding (Priority: P1) 🎯 MVP

**Goal**: All 27 in-scope operations are proven free of panics, overflow and undefined
behaviour within their stated bounds. That includes every byte string up to 17 bytes given
to the decoder. Every proven panic, and FR-016's preconditions, are documented.

**Independent Test**:

- `make verify-union` reports every crash harness and shared family named in
  `verify/cell_union/INVENTORY.md` as `SUCCESSFUL`, with all covers satisfied, and exits 0.
- With `M-100` applied, `cu_crash::c_leaf_cells_covered__n2` fails, and the output prints a
  concrete-playback test whose union holds two face cells.

Every harness follows contracts/harness-conventions.md:
- a `#[kani::proof]` whose doc comment's first line is the row's C-&lt;op&gt; or P-* statement;
- inputs from `union_domain`, and `kani::assume` of exactly the inventory preconditions;
- the smallest passing `#[kani::unwind(N)]` (fact 3);
- covers for a non-empty domain and for each distinct outcome (FR-008);
- families generated with a `macro_rules!`, as in `verify/src/enc.rs`.

Develop one member at a time with `make verify-harness H=<module>::<name>`, then the whole
family with `make verify-union`. Apply the upstream findings rule to anything new.

- [X] T020 [US1] Measure the shapes R0 did not measure, before writing the families, and record them in `verify/cell_union/EVALUATION.md` (research R4 "to confirm"; fact 7). Needs Phase 2 and T016.
  - Write these as the first real members, in their final modules:
    - `cu_crash::c_intersection_with_cell_id__n2`;
    - `cu_crash::c_contains_union__n2_n2`;
    - `cu_crash::c_region_contains_cell__n0` (this measures building the `Cell`);
    - `cu_enc::p_uenc_04__b17` (decoding, after the R11 message change);
    - `cu_enc::p_uenc_04__b25`, the first length that can hold two cells and so reach
      FR-017's order check.

    Run each alone with `make verify-harness H=…`, and record the time and peak memory in
    "Measurements" of `verify/cell_union/EVALUATION.md`.
  - **Rules:**
    - A member over the 5-minute soft limit or 4 GB drops to the largest smaller member
      that fits. Record that in the inventory's `bounds`. A bound below SC-001's minimum
      needs a written justification in the evaluation (spec Assumptions).
    - **`Region` rows:** if `c_region_contains_cell__n0` does not fit, both `Region`
      cell-query rows become `tool-limit` with these measurements. SC-002 then falls to
      25/42 (59.5%), below 60%. **Stop and report to the user.** Do not change the spec.
    - **Decoding:** if `p_uenc_04__b17` does not fit, `S2Decode::decode` becomes a
      `tool-limit` row, as SC-001 allows. Update the inventory, P-UENC-02/03/04 leave the
      SC-003 count (data-model §4), and continue. If `__b25` fits, the family's bound becomes
      `L ≤ 25`. Otherwise, name FR-017's order and overlap check as unreached behaviour. T014,
      T015, T019 and the fuzz target exercise it.
- [X] T021 [US1] Measure `from_range`, the one tool-limit row without an R0 measurement, and record it in `verify/cell_union/INVENTORY.md` (FR-002, SC-002).
  - Write a throwaway crash harness in a scratch copy of `verify/`, not committed. Use
    `LeafOrEnd` × `LeafOrEnd` with `b ≤ e`, both inside one level-29 cell (range depth 1),
    and the smallest unwind that passes. Run it capped, as in "Memory safety".
  - **If it does not fit:** record the measurement in the `from_range` row and in
    "Exclusions and tool limits".
  - **If it fits:** make the row `in-scope`, with a `cu_crash::c_from_range__d<k>` family
    over the depths that fit, the bound, and the uncovered part. Move P-URANGE-03 to
    `proven-within-bounds` in T034. Note in the evaluation that this beats the plan.
  - **Outcome (2026-10-03):** no separate probe was needed. Since the D-002 fix, `from_range`
    calls `from_begin_end`, so it inherits that row's R0 measurement.
  - **Outcome (2026-10-03):** decoding fits (`__b17` in 24.7 s). `__b25` needs 7.8 GB, so it
    runs in the slow tier with an 8 GB ceiling, and the per-change family is `__b0 … __b24`.
    The `Region` rows fit (the `Cell` costs 15.7 s with an unwind of 9). Several bounds were
    raised where they fit: `intersection_with_cell_id` to `n ≤ 3`, the union queries to
    `3 × 3`, and `denormalize` to `n ≤ 2` (`verify/cell_union/EVALUATION.md`, "Bounds
    measured first").
- [X] T022 [P] [US1] Write `cu_acc::p_uacc_01__n0 … __n3` (P-UACC-01) in `verify/src/cu_acc.rs`. It discharges `C-CellUnion::new`, `C-CellUnion::from_verbatim`, `C-CellUnion::num_cells`, `C-CellUnion::cell_ids`, `C-CellUnion::is_empty`, `C-CellUnion::deref` and both `C-CellUnion::into_iter` obligations.
  - **Body:** `let (v, raw) = any_seq::<N>();` and
    `let u = CellUnion::from_verbatim(v.clone());`.
    Assert:
    - `u.cell_ids() == &v[..]` and `&*u == &v[..]`;
    - `(&u).into_iter().copied()` and `u.clone().into_iter()` both yield exactly `v`;
    - `u.num_cells() == N`, and `u.is_empty() == (N == 0)`.

    In `__n0` also assert `CellUnion::new().is_empty()` and
    `CellUnion::default() == CellUnion::new()`.
  - **Covers:** `__n0` covers the empty case. Other members cover a valid and an invalid
    first id.
- [X] T023 [P] [US1] Write `cu_inv::p_uinv_01__n0 … __n4` and `cu_inv::p_uinv_02__n0 … __n4` (P-UINV-01, P-UINV-02) in `verify/src/cu_inv.rs`. They discharge `C-CellUnion::is_valid` and `C-CellUnion::is_normalized` over `Seq`.
  - **Body:** over `let (v, raw) = any_seq::<N>()`, assert
    `from_verbatim(v).is_valid() == union_spec::valid_union(&raw)`, and the same for
    `is_normalized` with `normalized_union`.
  - **Covers** for each member with `N ≥ 2`: valid, invalid id, unsorted, and overlapping.
    For `p_uinv_02__n4`, also "four siblings", the only member that reaches the sibling
    check.
  - These run `spec::decompose`, a 31-level search, on every symbolic id. Measure `__n4`
    first. If it does not fit, keep `__n3` and record the bound, which still meets SC-001.
- [X] T024 [US1] Write the normalising crash families in `verify/src/cu_crash.rs`: `c_from_cell_ids__n0 … __n2`, `c_normalize__n0 … __n2` and `c_from_iter__n0 … __n2` (research R4, R5).
  - **Domain:** `any_valid_seq::<N>()`, valid ids in any order, possibly overlapping or
    repeated. `c_from_iter` collects through `FromIterator`.
  - **Covers** for `__n2`: a duplicate removed, a contained cell discarded with the
    contained cell first, the same with the containing cell first (the `pop` path), and two
    disjoint cells kept.
  - **Unwind:** R0 measured `from_cell_ids` at `n = 2` as 16.5 s and 0.9 GB. Never add
    `__n3` (14 GB).
- [X] T025 [US1] Write the query and measure crash families in `verify/src/cu_crash.rs`. Needs T024 (same file).
  - **Families:**
    - `c_contains_cell_id__n0 … __n3` and `c_intersects_cell_id__n0 … __n3`: over
      `any_valid_union::<N>()` and `domain::any_valid()`;
    - `c_region_contains_cell__n0 … __n3` and `c_region_intersects_cell__n0 … __n3`: the
      same, with `Cell::from_cell_id(c)` passed to the `Region` method;
    - `c_contains_union__n<a>_n<b>` and `c_intersects_union__n<a>_n<b>` for
      `a, b ∈ 0..=2`;
    - `c_leaf_cells_covered__n0 … __n3`;
    - `c_whole_sphere`.
  - **Covers:** contained and not contained, intersecting and disjoint. For
    `leaf_cells_covered`, a face cell and a leaf.
  - **Unwind:** the `Region` families need at least the unwind of 001's
    `to_face_ij_orientation` loops (`#[kani::unwind(9)]` in 001), plus the union length.
    Use the bounds T020 measured.
- [X] T026 [US1] Write the set-operation crash families in `verify/src/cu_crash.rs`: `c_union__n<a>_n<b>` and `c_intersection__n<a>_n<b>` for `a, b ∈ 0..=1`, and `c_intersection_with_cell_id__n0 … __n2`. Needs T025.
  - **Domain:** `any_valid_union` on each side, and `domain::any_valid()` for the cell.
  - **Covers:** an empty result, a non-empty result, a containment on either side, and
    (for `union`) a result with fewer cells than its inputs.
  - **Unwind:** `intersection` passed at unwind 3 in 85 s and 3.9 GB (R0). Unwind 6 ran out
    of memory. Keep the members within the bounds T020 measured.
- [X] T027 [US1] Write `c_denormalize__n0` and `c_denormalize__n1` in `verify/src/cu_crash.rs` (research R5). Needs T026 and T013.
  - **Domain:** `any_valid_union::<N>()`, `min_level` from `domain::any_level()`, and
    `level_mod` from `any_level_mod()`. Loop bound: assume
    `union_spec::denorm_level(d.level, m, k) − d.level ≤ 2` for each cell, so each cell
    has at most 16 descendants. The doc comment states the bound and its uncovered part
    (gaps 3–30), which T011, T012 and T019 exercise.
  - **Covers:** the cell kept, the cell replaced by its children, and the level-29 or
    level-30 clamp (`min_level` 1, `level_mod` 3).
  - **Measured:** R0 measured `n = 1` at 9.5 s.
- [X] T028 [P] [US1] Write `cu_enc::p_uenc_01__n0 … __n3` (P-UENC-01) and `cu_enc::p_uenc_04__b0 … __b<L>` (P-UENC-04) in `verify/src/cu_enc.rs`. They discharge `C-CellUnion::encode` and `C-CellUnion::decode`. Needs T016 and T020.
  - **`p_uenc_01`:** over `any_seq::<N>()`, encode into a `Vec<u8>` and assert
    `buf == union_spec::encoding(&raw)` and `buf.len() == 9 + 8·N`.
  - **`p_uenc_04`:** `with_bytes::<L>(|b| { let _ = <CellUnion as S2Decode>::decode(&mut &b[..]); })`.
    - **Bound:** `L` is the bound T020 set (17 or 25).
    - **Covers:** members with `L ≥ 9` cover `Ok` and `Err`. Members with `L < 9` cover
      only `Err`, since `Ok` cannot happen there. Members that can hold one or two cells
      cover a decoded cell and the `NotValid` rejection respectively.
    - **Unwind:** start at `(L − 9) / 8 + 3` and lower or raise it as fact 3 says.
  - **Doc comments** cite U-003 for the decode family.
  - **Budget:** if the family threatens the per-change budget, move its longest members to
    the slow tier (contracts/verify-commands.md, `F=union`) rather than dropping them.
- [X] T029 [US1] Update `verify/cell_union/INVENTORY.md` with the crash results.
  - **Every in-scope row:** `crash_harness` (the final family), `covers` (`M/N`),
    `crash_status = proven-within-bounds` (`proven` for `new` and `whole_sphere`), and
    `bounds`. `bounds` gives the union-size, byte or loop bound, the uncovered part, the
    unreached behaviour, and the tests that exercise it (FR-007).
  - **Undocumented panics:** for each row whose proof assumed a precondition that rustdoc
    does not state, set `undocumented_panic` and say why the panic is real. The candidates
    are research R14's table: an invalid id in the union or the cell argument, and
    `level_mod` outside 1–3. Where it is cheap, confirm that the panic is reachable with a
    throwaway wider-domain harness at `n = 1`, run capped and not committed. If a panic also
    differs from upstream, record a `D-NNN` in the evaluation (FR-010).
  - **Debug-only checks** (spec edge case): for each row whose precondition is guarded by a
    `debug_assert!`, state in `preconditions (source)` that the harness assumes the guarded
    condition. The proof therefore never relies on the check firing, in either build.
  - **Check:** SC-001's bounds are met, or the shortfall is justified in the evaluation,
    and SC-002's share is stated.
- [X] T030 [US1] Add the `# Panics` sections and the FR-016 documentation in `core/src/s2/cell_union.rs` and `core/src/s2/encoding.rs`, as one documentation-only commit (FR-003, FR-016, SC-009, SC-011; research R10, R14).
  - **FR-016:**
    - `from_verbatim`: "The ids must satisfy `is_valid()` (sorted, non-overlapping, valid
      cells), as C++ `FromVerbatim` requires. Operations on a union that does not are
      unspecified, and some may panic."
    - `leaf_cells_covered`: the union must be valid. Add a `# Panics` section: in debug
      builds the sum can overflow when the union is not valid, and `CellId::level` panics
      on an invalid id. Add a deviation note: it returns `i64`, where C++ returns
      `uint64_t`, and the values agree for every valid union, because the count is at most
      `6·4^30 < 2^63`.
  - **`# Panics`, for every public function** (Constitution II). Cover every public function
    and trait method in `core/src/s2/cell_union.rs`, not only the proven ones:
    - **Proven rows:** each row with an `undocumented_panic` from T029.
    - **Excluded and tool-limit rows, by reading the code, as 001's T028 did:**
      - `from_begin_end`: in debug builds, if `begin` or `end` is not a leaf or `end(30)`,
        or `begin > end`;
      - `from_min_max`: in debug builds, if `max_id` is not valid, plus `from_begin_end`'s
        conditions;
      - `from_range`: if `begin` is not a valid cell, through `CellId::level`;
      - `difference`, `expand_at_level`, `expand_by_radius`, the area functions,
        `contains_point` and the `Region` methods: whatever an invalid id in the union can
        trigger.

      Check each candidate against the code rather than copying this list. A function that
      cannot panic gets no section.
    - **`denormalize`:** "in debug builds, if `level_mod` is not 1–3".

    State each condition in the caller's terms, for example "Panics in debug builds if `id`
    is not a valid cell". Where a panic differs from upstream, say so in one sentence and
    cite the `D-NNN`.
  - **Records:** fill `panics_doc` for all 42 inventory rows, including the excluded ones.
    List each section under "`# Panics` documentation added" in the evaluation.
  - **Checks:** `make doc` and `make test` pass, and `git diff` shows rustdoc lines only.
  - **Commit** separately, for example `docs(s2): document CellUnion preconditions and panics`.
- [X] T031 [US1] Create `verify/mutants/M-100-leaf-count-overflow.patch` in the format of `verify/mutants/M-001-advance-unclamped.patch` (research R7; US1 scenario 4).
  - **Header:**
    - `Mutant: M-100`;
    - `Targets: C-CellUnion::leaf_cells_covered, P-UMEAS-01`;
    - `Expect-Fail: cu_crash::c_leaf_cells_covered__n2, cu_meas::p_umeas_01__n1`;
    - `Defect: leaf_cells_covered shifts by 2·(31 − level), so two face cells sum to 2^63 and overflow`.
  - Make the diff against `core/src/s2/cell_union.rs` in a throwaway `git worktree`.
  - **Outcome (2026-10-04):** the crash half was checked by hand in a throwaway worktree:
    `cu_crash::c_leaf_cells_covered__n2` `FAILED` with "attempt to add with overflow" in
    `leaf_cells_covered`, and the playback test shows two face cells (faces 3 and 5). The
    full `make verify-mutants M=M-100` runs with the US2 campaign (T043).
  - **Check:** `make verify-mutants M=M-100`. Until T037 exists, it reports `error`,
    because `p_umeas_01__n1` is missing. Before that, check the crash half by hand in the
    worktree: `make verify-harness H=cu_crash::c_leaf_cells_covered__n2` must report
    `FAILED` with the overflow check, followed by a playback test with two face cells.
- [X] T032 [US1] Run `make verify-union` on its own, and record the results in `verify/cell_union/EVALUATION.md`.
  - Every harness must be `SUCCESSFUL` with all covers satisfied, and the exit code must be 0.
  - In "Measurements", record:
    - each family's time and the slowest member;
    - the total wall-clock and CPU time, with the machine and `VERIFY_JOBS`;
    - a comparison with R15's budget: 35 CPU-minutes, the 8-minute workstation proxy per
      harness, and 4 GB.
  - Append the US1 hours to the "Effort log".

**Checkpoint**: US1 is complete and the MVP is shippable. SC-001, SC-002, SC-009 and SC-011
hold.

---

## Phase 5: User Story 2 — Proven set semantics (Priority: P2)

**Goal**: The 27-entry catalogue of data-model §4.1 is in `verify/cell_union/PROPERTIES.md`.
Every entry is proven within its bounds against `union_spec.rs`, or marked not proven with
its reason. Every proven entry has a killing mutant (FR-008, SC-004).

**Independent Test**:

- `make verify-union` reports every `cu_*::p_*` harness `SUCCESSFUL`, or the entry's row
  says why not.
- `make verify-mutants` reports every `M-1NN` mutant `killed`.

- [X] T033 [US2] Create `verify/cell_union/PROPERTIES.md` with one row per entry of data-model §4.1: 27 rows, with the columns of `verify/PROPERTIES.md`.
  - **`Statement`:** copied verbatim from the table.
  - **`Upstream source`:** the `s2cell_union.h`/`.cc` function or doc comment, or
    "mathematical (no upstream counterpart)" for P-URANGE-03.
  - **`Harness`:** `cu_<area>::p_u<area>_<nn>`, or the family name.
  - **`Status`**, quoted from data-model §4: `proven` | `proven-within-bounds` |
    `not-proven (<reason>)`. `Bound` is required for `proven-within-bounds`.
    - `not-proven (excluded: tool-limit)` for P-USET-04 and P-URANGE-01/02/03 (unless T021
      brought `from_range` into scope). Each names its R17 property and C++ section
      instead, and is marked "outside the SC-003 denominator".
    - `draft` for everything else.
  - **Rows already discharged by US1:** P-UACC-01, P-UINV-01/02, P-UENC-01 and P-UENC-04.
- [X] T034 [P] [US2] Write P-UNORM-01 … P-UNORM-04 in `verify/src/cu_norm.rs`, families `__n0 … __n2`.
  - **P-UNORM-01:** `from_cell_ids(v)`, `normalize` and `FromIterator` each return a union
    whose `out_decomposed(..)` is `Some(ds)` with `normalized_union_d(&ds)`.
  - **P-UNORM-02:** for `(x_cell, x) = any_leaf()`,
    `out_covers(out, x_cell) == union_spec::covers(&ds, x)`.
  - **P-UNORM-03:** over `any_norm_union::<N>()`, `normalize` leaves the ids unchanged.
  - **P-UNORM-04:** over `any_valid_seq::<2>()`, normalising `[a, b]` and `[b, a]` gives
    equal results. Its doc comment states the transposition argument (data-model §7).
  - **Bound note** on every row: the sibling merge and cascades are unreached (4 cells).
    The NORMALIZE vectors and `prop_cell_union_normalize_preserves_coverage` exercise them.
- [X] T035 [P] [US2] Write P-UMEM-01 … P-UMEM-05 in `verify/src/cu_mem.rs`.
  - **P-UMEM-01 and P-UMEM-02** (`__n0 … __n3`): `contains_cell_id` and
    `intersects_cell_id` equal `union_spec::contains_id` and `intersects_id` on the
    decompositions.
  - **P-UMEM-03** (`__n0 … __n3`): if `contains_cell_id(c)`, then for a leaf `x` with
    `spec::contains(dc, dx)`, `union_spec::covers(&ds, x)` holds.
  - **P-UMEM-04** (`__n<a>_n<b>`, `a, b ≤ 2`): `contains_union(B)` equals
    `∀ b: contains_id(ds_a, db)`, and `intersects_union(B)` equals
    `∃ b: intersects_id(ds_a, db)`. Both sides are stated in the model, which includes
    upstream's caveat for non-normalised unions.
  - **P-UMEM-05** (`__n0 … __n3`): `Region::contains_cell(&Cell::from_cell_id(c))` and
    `Region::intersects_cell(…)` equal the model results for `c`. Use T020's unwind.
- [X] T036 [P] [US2] Write P-USET-01 … P-USET-03 in `verify/src/cu_set.rs`.
  - **P-USET-01** (`__n<a>_n<b>`, `a, b ≤ 1`): `union` covers `covers(A, x) ∨ covers(B, x)`
    for every leaf, and its result is `normalized_union_d`.
  - **P-USET-02** (same family shape): `intersection` covers `covers(A, x) ∧ covers(B, x)`.
    The result is `valid_union_d`, and also `normalized_union_d` when both inputs are
    (`any_norm_union`, the same as `ValidUnion` at these sizes).
  - **P-USET-03** (`__n0 … __n2` × `Valid`): `intersection_with_cell_id` covers
    `covers(U, x) ∧ x ∈ range(dc)`, and the result is normalised when `U` is.
  - No harness for P-USET-04 (`difference`, tool limit).
- [X] T037 [P] [US2] Write P-URANGE-04 in `verify/src/cu_range.rs` and P-UMEAS-01 in `verify/src/cu_meas.rs`.
  - **P-URANGE-04:** `whole_sphere().cell_ids()` is the six face cells in order, and for
    every leaf `x`, `out_covers` holds. Its status is `proven` (whole domain).
  - **P-UMEAS-01** (`__n0 … __n3`): `leaf_cells_covered()` is non-negative and equals
    `union_spec::leaf_count(&ds)`, which is at most `6·4^30`.
  - If T021 brought `from_range` into scope, add P-URANGE-03's family to `cu_range.rs`.
- [X] T038 [P] [US2] Write P-UDEN-01 and P-UDEN-02 in `verify/src/cu_den.rs`, families `__n0`, `__n1`, under the same level-gap bound as T027.
  - **P-UDEN-01:** `denormalize(m, k).cell_ids()`, as raw ids, equals
    `union_spec::denormalize(&ds, m, k)`. The result is `valid_union`.
  - **P-UDEN-02:** every member has level `≥ m` and `(level − m) % k == 0` unless its level
    is 30, and the result covers exactly what `U` covers, for a symbolic leaf.
- [X] T039 [US2] Write P-UENC-02 and P-UENC-03 in `verify/src/cu_enc.rs`, next to the T028 families.
  - **P-UENC-02** (`__n0 … __n3`): over `any_valid_union::<N>()`, `decode(encode(u))` is
    `Ok` with exactly the same ids. Measure `__n4` with `any_valid_union::<4>()` too: if it
    fits, it is the first proof with a non-normalised union (four siblings).
  - **P-UENC-03** (`__b0 … __b<L>`, the bound from T020): `decode(b)` is `Ok(ids)` exactly
    when `union_spec::decode(b) == Some(ids)`. The family has the same covers as
    `p_uenc_04`.
  - If T020 made decoding a tool limit, P-UENC-02 may still be proven over encoded unions,
    but P-UENC-02/03/04 leave the SC-003 count (data-model §4).
- [X] T040 [P] [US2] Write the ACC, INV and NORM mutants as `verify/mutants/M-101 … M-119-<slug>.patch` (research R7). Each patch holds one defect in `core/src/s2/cell_union.rs`, made in a throwaway worktree.
  - **Mutants:**
    - `num_cells` returns `len + 1` → `cu_acc::p_uacc_01__n1`;
    - `is_empty` inverted → `cu_acc::p_uacc_01__n0`;
    - `is_valid` uses `>` instead of `>=` on the overlap check → `cu_inv::p_uinv_01__n2`;
    - `is_normalized` skips the window at index 0 → `cu_inv::p_uinv_02__n4` (or `__n3` with a
      recorded reason if T023 stopped there; then choose a mutant P-UINV-02 can kill);
    - the merge loop's `last.contains(ci)` becomes `ci.contains(last)` →
      `cu_norm::p_unorm_02__n2`;
    - the "discard contained" loop breaks instead of popping →
      `cu_norm::p_unorm_01__n2`;
    - `normalize` sorts in descending order (`sort_unstable_by(|a, b| b.cmp(a))`) →
      `cu_norm::p_unorm_03__n2`. The two mutants above change nothing on an input that is
      already normalised, so P-UNORM-03 needs its own;
    - the `sort_unstable` call removed → `cu_norm::p_unorm_04__n2`. Every other `normalize`
      mutant acts after the sort, so it cannot make the result depend on input order.
  - **No mutants in the collapse loop** (`ci = ci.parent()`, `are_siblings`): the proofs
    cannot reach it within their bounds (R7). Say so in the evaluation.
- [X] T041 [P] [US2] Write the MEM, SET and RANGE mutants as `verify/mutants/M-120 … M-139-<slug>.patch`.
  - **Mutants:**
    - `contains_cell_id`'s `partition_point` predicate uses `range_min` for `range_max` →
      `cu_mem::p_umem_01__n1`;
    - `contains_cell_id` calls `intersects` instead of `contains` →
      `cu_mem::p_umem_03__n1` and `cu_mem::p_umem_01__n1`. P-UMEM-03 is an implication
      ("if contained, then covered"), so it needs a mutant that answers `true` wrongly, such
      as a union holding a child of the queried cell;
    - `intersects_cell_id` uses `contains` → `cu_mem::p_umem_02__n1`;
    - `intersects_union` uses `all` → `cu_mem::p_umem_04__n1_n2`;
    - `get_intersection` pushes the larger cell on `Equal` → `cu_set::p_uset_02__n1_n1`;
    - `union` uses `from_verbatim` instead of `from_cell_ids` → `cu_set::p_uset_01__n1_n1`;
    - `intersection_with_cell_id` uses `<` for `<=` → `cu_set::p_uset_03__n1`;
    - `Region::contains_cell` calls `intersects_cell_id` → `cu_mem::p_umem_05__n1`;
    - `whole_sphere` builds faces `0..5`, five faces instead of six →
      `cu_range::p_urange_04`.
- [X] T042 [P] [US2] Write the DEN, MEAS and ENC mutants as `verify/mutants/M-140 … M-159-<slug>.patch`. Needs T013 and T016.
  - **Mutants:**
    - `denormalize`'s rounding uses `level_mod − offset + 1` → `cu_den::p_uden_01__n1` and
      `cu_den::p_uden_02__n1` (a rounded level is then off the `level_mod` grid);
    - `leaf_cells_covered` counts `1 << (2·level)` → `cu_meas::p_umeas_01__n1`;
    - **`M-150`**, the reverse of the T013 fix → `cu_den::p_uden_01__n1` (the level-29 /
      1 / 3 input panics);
    - **`M-151`**, the reverse of the T016 fix (decode normalises again and does not check
      order) → `cu_enc::p_uenc_03__b25`. If T020 stopped below 25 bytes, `M-151` cannot be
      killed within the bounds: record that in `verify/MUTANTS.md` and the evaluation, and
      name T014 and T015 as FR-017's evidence;
    - the version byte written as 0 → `cu_enc::p_uenc_01__n0`;
    - decode accepts version 2 → `cu_enc::p_uenc_03__b9`;
    - decode drops the last id → `cu_enc::p_uenc_02__n1` and `cu_enc::p_uenc_03__b17`;
    - the order check unwraps `ids.last()` before the first id is pushed →
      `cu_enc::p_uenc_04__b17`. P-UENC-04 is a catalogue property, so it needs a mutant
      that injects a reachable panic.
  - The other crash-only harnesses need no mutant beyond `M-100`. Their covers carry
    FR-008.
- [X] T043 [US2] Run `make verify-mutants M="<every M-1NN id>"` on its own, and record the outcomes in `verify/MUTANTS.md`. Then run the full `make verify-mutants` once, which includes 001's mutants.
  - **Pre-check:** every row of `verify/cell_union/PROPERTIES.md` that is to be `proven*` is
    named in the `Targets` line of at least one `M-1NN` patch. Add a mutant for any row
    that is missing before running the campaign (SC-004).
  - **Survivors:** for every `survived`, strengthen the property or its domain, never weaken
    the mutant. Re-run until every mutant is `killed`.
  - **Errors:** an `error` from `OOM` means the expected harness is too large for the
    mutated code. Choose a smaller expected member. Fix patches that do not apply.
  - **`PROPERTIES.md`:** fill in the `Mutants` column. A `survived` blocks `proven`.
- [X] T044 [US2] Run `make verify-union`, then finalize `verify/cell_union/PROPERTIES.md` and `verify/cell_union/INVENTORY.md`.
  - **`PROPERTIES.md`:** set the final `Status`, `Bound` and reason of every row.
  - **`INVENTORY.md`:**
    - check the `properties` column;
    - fill `result_validity` (FR-004) for every row whose operation returns a `CellUnion`,
      naming the `P-*` that proves the invariant: normalised for `from_cell_ids`,
      `normalize`, `FromIterator`, `union` and `whole_sphere`; normalised when the inputs
      are, for `intersection` and `intersection_with_cell_id`; valid for `denormalize`;
      "none: stores its input (FR-016 documents the requirement)" for `from_verbatim`;
      "a valid union" for `decode`.
  - **SC-003:** at least 90% of the entries whose operations are all in scope are proven
    or proven within bounds that meet SC-001: 21 of 23, or 18 of 20 if decoding became a
    tool limit.
  - **SC-004:** every `proven*` row has a killing mutant.
  - **`EVALUATION.md`:** append the harness times to "Measurements" and the US2 hours to the
    "Effort log".
  - **Outcome (2026-10-04):**
    - The first US2 run had 30 of 93 harnesses run out of memory. Four cost traps were
      fixed: iterator adapters, loops over outputs of symbolic length, decoding a
      `Vec`-encoded buffer, and the model's 31-level search in covers and readers
      (`verify/cell_union/EVALUATION.md`, "Notes from implementation").
    - After the fixes, all 94 property harnesses passed. P-USET-03 stops at 2 cells and
      P-UENC-03 at 24 bytes (25 ran out of memory at 8 GB, also split by the count field).
      P-UDEN-01 is split by conjunct.
    - The suite was then over the per-change budget, so the larger property members and
      `c_denormalize__n2` moved to the weekly tier: 176 harnesses per change, 39 weekly.
    - 25 of the 26 mutants were killed. M-151 (FR-017 reversed) could not be killed within
      the bounds, so its patch was dropped and the reason recorded (T042's rule).
    - All 23 SC-003 entries are proven (22 within bounds), and every proven entry has a
      killing mutant (SC-004).

**Checkpoint**: US1 and US2 both pass independently, and SC-003 and SC-004 are met.

---

## Phase 6: User Story 3 — Regression protection for future changes (Priority: P3)

**Goal**: The `CellUnion` proofs run in CI, as their own leg, on changes to `CellUnion`, its
encoding, `CellId` or the verification layer. The default gate is unaffected.

**Independent Test**:

- A change that breaks a proven `CellUnion` property fails the `union` leg of the `verify`
  job. The log names the harness and shows a concrete counterexample.
- Reverting the change makes the leg pass again.

- [X] T045 [US3] Update `.github/workflows/verify.yml` (research R8; contracts/verify-commands.md).
  - **Header comment:** name both suites and specs, and describe the two-leg `verify` job.
  - **Paths**, in both `pull_request` and `push`: add `core/src/s2/cell_union.rs`,
    `core/src/s2/encoding.rs`, `core/src/s2/cell.rs` and
    `core/tests/data/cpp_cellunion_vectors.csv`.
  - **Job `verify`:** add `strategy: { fail-fast: false, matrix: { suite: [cellid, union] } }`.
    Run `make verify-${{ matrix.suite }}`, keep `timeout-minutes: 30`, and name the log
    artifact `verify-logs-${{ matrix.suite }}`.
  - **Job `slow`:** only if T028 or any other task moved `cu_*` members to the slow tier:
    - add a `union` leg to the matrix;
    - in `Makefile`, extend `SLOW_HARNESSES` so that `F=union` selects
      `--harness cu_` with `--features slow`;
    - gate those members with `#[cfg(feature = "slow")]`;
    - update `verify-slow`'s help comment to `F=<0-5|vertex|union>`.
  - **Job `mutants`:** `timeout-minutes: 180`.
  - **Check:** `.github/workflows/test.yml` and the `make test` target are unchanged
    (FR-012).
- [ ] T046 [US3] Validate `.github/workflows/verify.yml` and measure SC-005.
  - **Static checks:** run `actionlint .github/workflows/verify.yml` if it is installed;
    otherwise review the file carefully.
  - **Live check:** **ask the user first**, because it pushes to the remote.
    1. Open a draft PR from a scratch branch with `M-100` applied.
    2. Confirm that the `union` leg fails, names `cu_crash::c_leaf_cells_covered__n2`, and
       prints the playback test.
    3. Confirm that the `cellid` leg passes.
    4. Close the PR and delete the branch.
  - **Measurements** for `verify/cell_union/EVALUATION.md`:
    - each leg's wall-clock time, with the runner's `nproc` and `MemTotal`;
    - the slowest harness;
    - any `OOM` or `TIMEOUT`.
  - **If the runner is smaller than 4 vCPU / 16 GB:** also time `make verify-union` on a
    reference-sized machine, for example
    `systemd-run --user --scope -p AllowedCPUs=0-3 -p MemoryMax=16G make verify-union`, and
    report both times.
  - **Budgets (SC-005):** `union` under 10 minutes, and `cellid` still under 15. If `union`
    is over, move its longest family members to the slow tier (T045) and re-measure.
  - **Outcome (2026-10-04):** `actionlint` 1.7.12 reports nothing. On 4 pinned CPUs the
    per-change `union` leg first took 10.9 minutes. Moving more members to the weekly tier
    and removing 50 crash members that per-change properties already cover (shared
    harnesses) brought it to 120 harnesses in 7.9 minutes. `cellid`: 196 SUCCESSFUL. The
    weekly `union` tier: 165 SUCCESSFUL in 49 minutes, after giving the runner's cgroup
    backstop 2 GiB of headroom (it had stopped the 7.8 GB decode proof). M-104 now carries
    `Tier: slow`. **Deferred:** the user chose on 2026-10-04 to skip the live CI check for
    now, so SC-005 is unmeasured on a hosted runner and this task stays open.

**Checkpoint**: US3 is complete, and SC-005 is measured.

---

## Phase 7: User Story 4 — Evaluation of collection proofs (Priority: P3)

**Goal**: A short, self-contained evaluation that answers whether bounded proofs over
collections are worth their cost, and what comes next.

**Independent Test**: A maintainer who did not take part can answer "continue or stop?",
"with which approach?", "are bounded proofs over collections worth it?" and "what next?"
from `verify/cell_union/EVALUATION.md` alone (SC-007).

- [X] T047 [US4] Complete `verify/cell_union/EVALUATION.md` (FR-014; data-model §8).
  - **Effort:** the "Effort log" total, in person-days per phase.
  - **Run time:** the CI time per leg, the reference-machine time, the slowest harness and
    the peak memory.
  - **Bounds reached:** per operation, the `N`, `L` and loop bounds proven, their cost, and
    the unreached behaviour, each with the tests that exercise it.
  - **Limitations:**
    - the four or five tool-limit exclusions, with their measurements;
    - the normalising wall at 3 cells, and why it makes the sibling merge unreachable;
    - mutants that cannot be killed within the bounds (the collapse loop, and `M-151` if
      decoding stopped below 25 bytes);
    - the written arguments: the leaf-count bound for any size (FR-016), the
      transposition argument for P-UNORM-04, and the `denormalize` branch argument from
      T011.
  - **Findings and defects:**
    - FR-015, FR-016 and FR-017, each with its commit, the C++ reference, its vector
      section, and its regression and differential tests as evidence;
    - every `D-NNN`;
    - any parity defect found by T010 or T019.
  - **`# Panics` documentation added:** the T030 list (SC-011).
  - **Upstream findings:** U-003 and every later `U-NNN`, each with its Phase 9 task
    (SC-012).
  - **Comparison with 001:** effort, CPU time, defects found per day, and the
    whole-domain share (001 proved most operations over every input, 002 almost none).
  - **Collections verdict:** whether bounded proofs over collections were worth their
    cost, separately for read-only operations, operations that build a union, and
    decoding. Say whether a deductive tool (Verus, Creusot) or kernel-checked proofs would
    remove the size bound for `normalize`, citing `verify/research/rust_formalmethods.md`.
  - **Go/no-go and follow-ups:** a clear recommendation, and a ranked list of candidates
    with one-line reasons. Consider the integer coding layer (`encoded_uint_vector`,
    `encoded_s2cell_id_vector`, varint), `CellIndex`, and loop contracts for `normalize`
    once they are stable in Kani.
- [X] T048 [US4] Read `verify/cell_union/EVALUATION.md` as a non-participant would (SC-007).
  - **Check:** each of the four questions must be answerable from the document alone. If one
    isn't, revise the document.
  - **Links:** every finding links to its row in `cell_union/INVENTORY.md`,
    `cell_union/PROPERTIES.md` or `MUTANTS.md`, or to its Phase 9 task.

**Checkpoint**: All four user stories are complete.

---

## Phase 8: Polish & Cross-Cutting Concerns

- [X] T049 [P] Add a "`CellUnion` (spec 002)" section to `verify/TRUSTED_BASE.md` (FR-013; data-model §7).
  - **Reliance on 001:** list each `CellId` method a property statement reads, with its
    001 property:
    - `range_min` and `range_max`: P-RANGE-01;
    - `is_valid`: P-VAL-03;
    - `contains` and `intersects`: P-RANGE-02 and P-RANGE-03;
    - `parent` and `children`: P-HIER-03 and P-HIER-01.

    State that nothing is stubbed: the `CellId` code runs inside every proof.
  - **The standard library:** `sort_unstable`, `Vec`, `partition_point` and `io::Read` for
    `&[u8]` are executed, not modelled. Only Kani's allocation model and its `std` build are
    trusted.
  - **The model:** `union_spec.rs`, validated by sampling against the C++ vectors (T008).
  - **Bounds:** every `N`, `L` and loop bound, with its uncovered part and the tests that
    exercise it, from T029.
  - **Written arguments:** the leaf-count bound for any size, and P-UNORM-04's
    transpositions.
- [X] T050 [P] Update `verify/README.md` for two suites.
  - The title and "What is proven" cover `CellUnion`, with links to
    `cell_union/INVENTORY.md`, `cell_union/PROPERTIES.md` and `cell_union/EVALUATION.md`.
  - "Running" documents `make verify-cellid`, `make verify-union` and the `cellunion`
    feature (contracts/verify-commands.md).
  - "Rules" adds concrete lengths and the smallest unwind (facts 2 and 3).
  - "Layout" adds `union_spec.rs`, `union_domain.rs`, the `cu_*` modules and
    `cell_union/`.
- [X] T051 [P] Lint and check headers in `verify/`, `core/src/s2/cell_union_tests.rs`, `core/tests/cpp_cellunion_diff.rs` and `tools/refgen-cpp/generate_cellunion.cc`.
  - In `verify/`, run `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`,
    and fix any findings. Every `#[allow]` carries a `reason`.
  - `grep -rn "unsafe" verify/src` still shows only `domain::with_text`.
  - Every new file has an SPDX header:
    `find verify -path verify/target -prune -o \( -name '*.rs' -o -name '*.sh' -o -name '*.toml' \) -print | xargs grep -L SPDX-License-Identifier`
    must print nothing. Also check the three files outside `verify/`.
- [X] T052 Run `specs/002-formal-verify-cell-union/quickstart.md` end to end (§1–§11), one verification job at a time. Fix any step that does not match reality, either in quickstart.md or in the code.
- [X] T053 Cross-check every success criterion, and record the results in an "Acceptance" section at the end of `verify/cell_union/EVALUATION.md`.
  - **SC-001:** every in-scope row meets its minimum bound, or has a written justification.
  - **SC-002:** at least 60% in scope, with every exclusion's reason and measurements.
  - **SC-003 and SC-004:** as checked in T044.
  - **SC-005:** from T046.
  - **SC-006:** `git diff master --stat -- core python wasm` shows only T007's CSV, the
    Phase 3 commits and T030. No public API changes, no new dependencies, and `make test` passes.
  - **SC-007:** as checked in T048.
  - **SC-008, SC-009 and SC-010:** from Phase 3 and T030.
  - **SC-011:** every proven panic is documented (T030).
  - **SC-012:** every `U-NNN` has a Phase 9 task and an evaluation entry.

---

## Phase 9: Follow-ups (FR-019) — recorded, not implemented in this feature

**Purpose**: Each task here records undefined or wrong behaviour found in the upstream
reference. Resolve each in a **separate change** after this feature. `/speckit-implement`
must not execute these tasks. It only appends new ones, following the upstream findings
rule above. The next free task ID is T055.

- [ ] T054 U-003: decide whether to report upstream that `S2CellUnion::Decode` accepts unions that break its own invariant; the Rust side is `core/src/s2/encoding.rs` (research R12; FR-017).
  - **Upstream:** C++ `S2CellUnion::Decode` (`s2cell_union.cc:544–562`) together with
    `S2CellId::Decode` (`s2cell_id.cc:261–265`). Neither validates the ids.
    `FromVerbatim` REQUIRES `IsValid()` (`s2cell_union.h:142`), and `Contains` relies on it.
  - **Inputs:** version 1, count 2, ids `[0x3000000000000000, 0x1000000000000000]` (face 1,
    then face 0); or count 1 with id `0`.
  - **Observed:** `Decode` returns `true` and a union whose `IsValid()` is `false`. Later
    `Contains` calls can silently give wrong answers, and `LeafCellsCovered` can wrap. The
    `CELLUNION_DECODE` rows with `ok = 1, valid = 0` record it (T007).
  - **Rust:** `S2Decode for CellUnion` rejects such input with `InvalidData`, which is
    documented as a deliberate deviation (T016).
  - **Decision needed:** whether to report it upstream, and whether the Go and Java
    decoders behave the same. Record their behaviour. This feature does not change upstream
    code.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: T001 first. T002 is independent. T003 needs T001.
- **Foundational (Phase 2)**: needs Phase 1.
  - T004, then T005.
  - T006 → T007 → T008. T008 also needs T004.
  - T009 can start any time.
  - **Phase 2 blocks all user stories.**
- **Phase 3**: needs T007 (the CSV).
  - T010 → T011 → T012 → T013 (the FR-015 commit), then T014 → T015 → T016, with T017 and
    T018 (the FR-017 commit), then T019. They are serial because they share
    `cell_union_tests.rs`, `cpp_cellunion_diff.rs` and the commit order.
  - T017 and T018 can run in parallel with each other after T016.
  - **T013 must land before T027. T016 must land before T020 and T028.**
- **US1 (Phase 4)**: needs Phase 2, T013 and T016.
  - T020 and T021 first, because they set the bounds.
  - T022 (`cu_acc.rs`), T023 (`cu_inv.rs`) and T028 (`cu_enc.rs`) in parallel.
  - T024 → T025 → T026 → T027, because they edit `cu_crash.rs`.
  - Then T029 → T030 → T031 → T032.
- **US2 (Phase 5)**: needs Phase 2 and US1's shared families (T022, T023, T028).
  - T033, then T034–T039 in parallel. T039 edits `cu_enc.rs` after T028.
  - T040–T042 in parallel (disjoint mutant ranges). T031's `M-100` needs T037 to be killed
    in full.
  - Then T043, then T044.
- **US3 (Phase 6)**: needs T003 and US1. T046 is most useful after US2.
- **US4 (Phase 7)**: needs the results of US1–US3.
- **Polish (Phase 8)**: needs all the stories.
- **Follow-ups (Phase 9)**: not executed in this feature. Entries are appended whenever the
  upstream findings rule fires.

### User Story Dependencies

- **US1 (P1)**: depends on Foundational and the two behaviour fixes. It is the MVP.
- **US2 (P2)**: depends on Foundational and on US1's shared families. Its own harnesses can
  be tested independently of US1's crash harnesses.
- **US3 (P3)**: needs `make verify-union`. A CI leg with only US1's harnesses is already
  useful.
- **US4 (P3)**: written last, from everything else.

### Parallel Opportunities

- **Setup:** T001 and T002.
- **Foundational:** T004/T005, T006–T008 and T009 are three independent chains.
- **Phase 3:** T017 and T018.
- **US1:** `cu_acc.rs` (T022), `cu_inv.rs` (T023), `cu_enc.rs` (T028), and the `cu_crash.rs`
  chain (T024–T027).
- **US2:** the six property files (T034–T039), and the three mutant sets (T040–T042).
- **Polish:** T049, T050 and T051.

Writing files in parallel is fine. **Running** Kani is limited by memory: run verification
jobs one after another (see "Memory safety").

---

## Parallel Example: User Story 1

```bash
# After Phase 2, T013, T016, T020 and T021, four files at once:
Task: "T022 P-UACC-01 family in verify/src/cu_acc.rs"
Task: "T023 P-UINV-01/02 families in verify/src/cu_inv.rs"
Task: "T028 P-UENC-01 and P-UENC-04 families in verify/src/cu_enc.rs"
Task: "T024–T027 crash families in verify/src/cu_crash.rs (in order)"
```

## Parallel Example: User Story 2

```bash
# After T033, the property files at once:
Task: "P-UNORM-01…04 in verify/src/cu_norm.rs"
Task: "P-UMEM-01…05 in verify/src/cu_mem.rs"
Task: "P-USET-01…03 in verify/src/cu_set.rs"
Task: "P-URANGE-04 and P-UMEAS-01 in verify/src/cu_range.rs and cu_meas.rs"
Task: "P-UDEN-01/02 in verify/src/cu_den.rs"
Task: "P-UENC-02/03 in verify/src/cu_enc.rs"

# Then the mutant sets at once (disjoint ID ranges):
Task: "M-101…M-119 for ACC/INV/NORM"
Task: "M-120…M-139 for MEM/SET/RANGE"
Task: "M-140…M-159 for DEN/MEAS/ENC"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1 (Setup) and Phase 2 (Foundational): the feature gate, the model, the generators,
   the C++ vectors and the inventory.
2. Phase 3, at least through T016: the FR-015 and FR-017 fixes, which the proofs state.
3. Phase 4 (US1): T020 and T021 set the bounds first. Stop at T020 if the `Region` rows do
   not fit (SC-002).
4. **Stop and validate**: `make verify-union` is green, and `M-100` is caught with a
   concrete counterexample.

### Incremental Delivery

1. Setup + Foundational → the model, validated against C++, and the final inventory.
2. Phase 3 → the parity tests, FR-015, FR-017 and the R17 properties: four commits.
3. US1 → crash-freedom proofs, FR-016 and the `# Panics` docs (MVP).
4. US2 → set semantics and the mutant campaign.
5. US3 → the CI leg. It can come right after US1 to protect the MVP.
6. US4 → the evaluation. Polish → trusted base, README and acceptance.
7. Phase 9 follow-ups → separate changes after the feature.

---

## Notes

- [P] tasks touch different files and do not depend on unfinished tasks.
- Do not stub `s2rst` code, and never pass `--no-unwinding-checks` (contracts).
- Every union and byte length is concrete, and every unwind is the smallest that passes.
- Generate mutant patches in throwaway worktrees. Never leave a mutant applied in the main
  tree.
- Every Kani run is memory-capped, and verification jobs run one after another.
- Apply the upstream findings rule (FR-019) before marking any affected proof complete.
- Commit after each task or logical group. The parity tests, FR-015, FR-017, the R17
  properties and the documentation are each a commit of their own.
