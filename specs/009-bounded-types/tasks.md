---

description: "Task list for bounded types from the core to the bindings (spec 009)"
---

# Tasks: Bounded Types for Safer APIs, from the Core to the Bindings

**Input**: Design documents from `specs/009-bounded-types/`:

- **spec.md**: clarified 2026-10-27. FR-012: adopt the recommendation for the four prototyped
  candidates. FR-013: breaking changes in the next minor release, with migration notes.
- **plan.md**: research.md (R0 probes P1–P7, R1–R14) and data-model.md.
- **contracts/**:
  - `signature-audit.md`: `T` versus `impl Into<T>`, one verdict per signature;
  - `red-green-tests.md`: every test, its red and its green;
  - `binding-errors.md`;
  - `migration-notes.md`.
- **quickstart.md**, and **probe/** (P3, literal inference).

**Tests**: The maintainer asked for red/green tests at planning ("Add red/green tests to missing
functionality/crashes (for example in python)"), and the spec's success criteria need them, so
they appear as tasks. Each is written first, seen failing, and committed with its fix. The
Kani harnesses are the verification deliverable, so they are implementation tasks.

**Organization**: tasks are grouped by user story (spec.md):

| Story | Priority | Phase | What it delivers |
|-------|----------|-------|------------------|
| US1: an evidence-based decision for every candidate | P1 | 3 (MVP) | `BOUNDED_TYPES.md`: the classification, the options compared, the recommendations |
| US2: a prototype through core, Python and wasm | P1 | 4 | `Level`, `Face`, `LevelMod` and `CellVertex`, each one red/green commit through all three layers, with measured costs. This is also the adoption (FR-012) |
| US3: adopting what the study recommends | P2 | 5 | the adoption criteria checked and closed: `# Panics` sections, deviations, docs, migration notes, contribution rule |
| US4: proofs and records kept in step | P3 | 6 | constructor proofs, re-pointed gap-1 complements, mutants, verification records |

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel: different files, and no dependency on an unfinished task. For
  Kani **runs**, "parallel" applies only to writing the code; the runs go one after another.
- **[Story]**: the user story the task belongs to (US1–US4).
- Paths are relative to the repository root, `/home/t/sources/s2rst_multi/3`.

## Protocols (read first)

### Red/green (research R10)

Every task marked **RED** does the following:

1. **Write the tests** exactly as [contracts/red-green-tests.md](contracts/red-green-tests.md)
   gives them, and with the messages of [contracts/binding-errors.md](contracts/binding-errors.md).
2. **Rebuild what they run against.** For Python:
   `uv sync --directory python --reinstall-package s2rst`, or the tests see a stale
   `_core.abi3.so`.
3. **Run them on the unfixed tree.** Each must fail the way the contract's Red column says. If a
   test passes, or fails differently, stop: either the finding or the test is wrong. Record what
   happened and re-check the code before you go on.

   The exception is a row whose Red column says the behaviour is already right: the guards
   (PY-02 to PY-04, WA-02, CD-05) and the rows already fixed by earlier work (`denormalize`'s
   `ValueError`, A12; `Cell.vertex(4)`'s `IndexError`, A1; wasm's `vertex(4)` `Err`). Those must
   pass at red time, and are recorded as "already green".
4. **Record the failure** in `BOUNDED_TYPES.md` §4 (the red/green table): the test ID, the key
   output line verbatim, the date, and `git rev-parse --short HEAD`.
5. **Do not commit a red test on its own.** It is committed with its fix, so `make test` stays
   green at every commit.

The matching **GREEN** task re-runs the same tests and adds the green commit to the table.

### Commits

- Every commit task makes a **local commit only. Never push, and never open a PR.**
- Make the commit only once the maintainer has authorised commits for the implementation
  session. If they have not, stop at the commit task and ask.
- Use conventional commits. A breaking commit ends with a
  `BREAKING CHANGE: … see BOUNDED_TYPES.md#migration-notes (M…)` footer.
- Every commit message ends with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

### Kani and memory

- **Every** Kani run goes through a Makefile target (`make verify`, `verify-values`,
  `verify-release`, `verify-harness H=…`, `verify-mutants M=…`). These apply the memory guard
  of `verify/run-verify.sh`. Never call `cargo kani` uncapped.
- Run one Kani job at a time, and never kill another session's run.
- Before a Kani run, `df -h /` must show at least 20 GB free, because the Kani cache fills the
  root disk.
- `make verify-harness H=<any cheap harness>` builds every harness first (`--only-codegen`), so
  it also checks that `verify/` compiles. It reports "codegen failed" if it does not.
- Use `pgrep -f '[c]bmc'` (not `cbmc`) to check for a running job.

### Shell

- Use absolute paths, `-C`, `--directory` or `--manifest-path`. Do not use `cd … &&` compound
  commands.

---

## Phase 1: Setup

**Purpose**: a clean starting point, the baselines, and the evaluation file.

- [X] T001 Check `git -C /home/t/sources/s2rst_multi/3 status --short`. The crash-freedom
  coverage work of 2026-10-26/27 (`verify/COVERAGE.md`, mutants `M-801`…`M-808`, about 50
  modified files) must be committed before this feature starts, so that spec 009's commits
  contain only spec 009.
  - If it is still uncommitted, **stop and ask the maintainer** to commit it, or to authorise
    committing it. Do not mix the two.
  - `IMG_3412.jpeg` and `logs_72753408598.zip` stay untracked, and are never added.
- [X] T002 Run `make test` on the unfixed tree and confirm that it passes. This is the baseline
  that every later commit must keep.
- [X] T003 [P] Record the benchmark baseline (research R13). Run
  `cargo bench -p s2rst --bench cell_id --bench cell --bench region_coverer` on the unfixed tree,
  and save the complete gungraun output to
  `specs/009-bounded-types/bench/before-<short rev>.txt`, creating the directory.
- [X] T004 [P] Create `BOUNDED_TYPES.md` at the repository root with the seven sections of
  data-model.md §8:
  1. Scope and method;
  2. Classification;
  3. Comparison per candidate;
  4. Red/green table, with columns ID, finding, red output, red date and commit, green commit;
  5. Deviations from upstream;
  6. Migration notes (anchor `migration-notes`);
  7. Follow-ups.

  §1 states the purpose, the candidates (spec FR-005), the options O1–O5 (spec FR-003) and the
  dimensions (spec FR-004), and links `specs/009-bounded-types/`.
- [X] T005 [P] Rebuild the Python extension
  (`uv sync --directory python --reinstall-package s2rst`) and re-run research probes P1 and P2.
  The commands are in the plan's research R0: `python -I -c` with
  `CellId.from_face(0).parent_at_level(31)`, `from_face(7)`, `RegionCoverer(level_mod=0)`, and
  the others. Paste the results into `BOUNDED_TYPES.md` §1 as the starting state.

---

## Phase 2: Foundational (blocking prerequisites)

**Purpose**: the shared error type that every candidate's `TryFrom` returns, and the stand-alone
deserialisation fix (B2, FR-009). Later candidates copy its pattern, and its red must be seen
before `LevelMod` changes the coverer.

**⚠️ CRITICAL**: no user-story work begins until T011 is committed.

- [X] T006 [P] Write `core/src/s2/bounded_tests.rs`, new: an original test file with the two-line
  SPDX header and a note that it is original. Wire it in with
  `#[cfg(test)] #[path = "bounded_tests.rs"] mod bounded_tests;` at the bottom of
  `core/src/s2/bounded.rs`. Test that:
  - `BoundedKind::{Level, Face, LevelMod, CellVertex, CellEdge}` have the `name()` and `range()`
    of data-model.md §1: `level` `0..=30`, `face` `0..=5`, `level_mod` `1..=3`,
    `cell vertex index` `0..=3`, `cell edge index` `0..=3`;
  - `OutOfRange` displays exactly `"level must be in 0..=30, got 31"`, and likewise for each
    kind;
  - `kind()`, `value()` and `range()` return what it was built with.
- [X] T007 Implement `core/src/s2/bounded.rs`, new, with the two-line SPDX header:
  - `pub enum BoundedKind` with `name()` and `range() -> RangeInclusive<i64>`;
  - `pub struct OutOfRange { kind: BoundedKind, value: i64 }`, with a `pub(crate) fn new` and the
    accessors `kind()`, `value()` and `range()`;
  - `Display` as above;
  - `std::error::Error`, `Clone`, `Copy`, `Debug`, `PartialEq` and `Eq`.

  Add `pub mod bounded;` and `pub use bounded::{BoundedKind, OutOfRange};` to
  `core/src/s2/mod.rs`. Document every public item (`missing_docs`). Then
  `cargo test -p s2rst --all-features s2::bounded::` passes.
- [X] T008 Commit: `feat(s2): add BoundedKind and OutOfRange for bounded conversions` (additive). *Done as part of T011's commit: `OutOfRange::new` is `pub(crate)`, and its first caller is T010, so committed alone it fails the gate on dead code.*
- [X] T009 **RED** CO-01, CO-02 (`max_level` half) and CO-04:
  - **CO-01**, in `core/src/s2/coords_tests.rs` (new, original header; wired with
    `#[cfg(test)] #[path = "coords_tests.rs"] mod coords_tests;` at the bottom of
    `core/src/s2/coords.rs`, under `#[cfg(all(test, feature = "serde"))]` where needed):
    `serde_json::from_str::<Level>("31")` and `("255")` must be `Err`, with the message
    containing `level must be in 0..=30`.
  - **CO-02**, in `core/src/s2/region_coverer_tests.rs` (new, wired at the bottom of
    `core/src/s2/region_coverer.rs`): a `RegionCoverer` JSON with `"max_level": 31` must fail to
    deserialise.
  - **CO-04**: first run `serde_json::to_string` once on the unfixed tree for `Level::new(30)`,
    `RegionCoverer::default()` and `Cell::from_cell_id(CellId::from_face(Face::F0))` (or the
    pre-change equivalent call), then pin those exact strings as round-trip expectations.
  - Run `cargo test -p s2rst --all-features coords_tests region_coverer_tests`. CO-01 and CO-02
    must fail with `Ok(..)`. Record them in §4.
- [X] T010 **GREEN** B2 in `core/src/s2/coords.rs`:
  - Remove `serde::Deserialize` from `Level`'s `cfg_attr` derive, and keep `Serialize`.
  - Add a hand-written `#[cfg(feature = "serde")] impl<'de> serde::Deserialize<'de> for Level`
    on the pattern of `Cell`'s (`core/src/s2/cell.rs:828`): a private
    `#[derive(serde::Deserialize)] #[serde(rename = "Level")] struct Raw(u8);`, then
    `Level::try_new(raw.0)`, mapping `None` to
    `D::Error::custom(OutOfRange::new(BoundedKind::Level, raw.0.into()))`.
  - Document the impl the way `Cell`'s is documented: what it accepts, and why a derived impl is
    wrong.
  - Re-run T009's tests: they pass. `RegionCoverer`'s derived `Deserialize` now rejects
    `max_level` 31 through `Level`. Run `make test`, and record green in §4.
- [X] T011 Commit: `fix(s2): reject out-of-range levels when deserialising` (spec 009 B2).

**Checkpoint**: `OutOfRange` exists, and `Level` cannot be deserialised out of range. The user
stories can begin.

---

## Phase 3: User Story 1, an evidence-based decision for every candidate (P1) 🎯 MVP

**Goal**: `BOUNDED_TYPES.md` classifies every inventoried precondition, compares the options
for each candidate, and recommends one (FR-001 to FR-004, FR-018). This phase changes no code.

**Independent Test**: a maintainer who did not take part picks any of the 71 gap-1 operations,
or any row of `contracts/signature-audit.md`, and answers from `BOUNDED_TYPES.md` alone:

- its class;
- whether a bounded type removes the panic;
- the option recommended;
- the cost in Rust, Python and JavaScript.

For the four prototyped candidates, the numbers are marked "estimate" until T053 replaces them.

- [X] T012 [US1] Fill `BOUNDED_TYPES.md` §2 with one row per gap-1 operation, 71 in all. The
  sources are `verify/COVERAGE.md` ("Results"), `verify/INVENTORY.md` §3,
  `verify/cell_union/INVENTORY.md` §3, `verify/decoding/INVENTORY.md` §2 and
  `verify/values/INVENTORY.md` §10. The columns (data-model.md §8):
  - operation;
  - argument or receiver;
  - class: (a) absolute range, (b) finite set, (c) relation, (d) floating-point invariant, or
    (e) validity of a value type;
  - out-of-domain behaviour in debug builds;
  - out-of-domain behaviour in release builds;
  - upstream's behaviour, read from `/home/t/sources/s2geometry/src/s2/` (cite the file and the
    `ABSL_DCHECK` or clamp);
  - the gap-1 harness;
  - the recommended option, O1–O5;
  - its status: adopted, follow-up, or not applicable.

  The relational rows (class c, B9) state why no type removes them.
- [X] T013 [P] [US1] Add §2 rows for the signatures of `contracts/signature-audit.md` that gap 1
  does not cover:
  - the four face constructors (a silent invalid id: B3);
  - `RegionCoverer::level_mod` (a clamp) and its field (bypassed: B5);
  - `region_term_indexer::Options::level_mod` (`usize`);
  - `get_uvw_axis` and `get_uvw_face` (`axis`, `direction`);
  - `face_si_ti_to_xyz`;
  - `fractal::set_max_level` and `num_vertices_at_level`;
  - `DensityTree::new`;
  - `RegionCoverer::max_cells` (a clamp where upstream has none);
  - `r2::Rect::vertex` (wraps modulo 4, as upstream);
  - the typed identifiers (`impl Into<ShapeId>` and the others: rule 2, appropriate).
- [X] T014 [P] [US1] Add §2 rows for the class (d) and (e) invariants (FR-001):
  - a unit-length point: list the public operations whose documentation or `debug_assert!`
    requires `is_unit()`, found with `grep -rn "is_unit" core/src/s2 core/src/s1`;
  - latitude and longitude ranges (`LatLng`);
  - validity of `s1::Interval`, `r1::Interval`, `s2::Rect`, `r2::Rect` and `Cap`;
  - a normalised `CellUnion`.

  Each row gets O1 for now, with a follow-up rank and a cost estimate (FR-018).
- [X] T015 [US1] Write §3, the comparison. For each candidate group, compare O1–O5 and
  compile-time constants across every FR-004 dimension:
  - safety, and where an error is caught;
  - ergonomics: literal, arithmetic and iteration;
  - downstream: Python and JavaScript, from contracts/binding-errors.md and research R8 and R9;
  - serialisation, verification, performance and parity.

  Include research R1's answer to "`T` versus `impl Into<T>`" with probe P3's table. For
  `Level`, `Face`, `LevelMod` and `CellVertex`, use research P7's call-site estimates, each
  marked **estimate (replaced by T053)**.
- [X] T016 [P] [US1] Start §5, the deviations, with the four known ones:
  - `Cell::vertex(4)`, where upstream's `GetVertex` wraps modulo 4 (B4/B10, `r2rect.h:214`);
  - `level_mod` rejected, where upstream clamps in release builds (`s2region_coverer.cc:79`);
  - `RegionCoverer::max_cells` clamped to at least 1, where upstream does not clamp
    (`s2region_coverer.cc:56`);
  - faces 6 and 7 made unrepresentable, where upstream's `FromFace(int)` is unchecked.
- [X] T017 [US1] Validate US1. Count the §2 rows against FR-001's inventory (SC-001: 100%). Walk
  quickstart.md §5 for three operations, `CellId::parent_at_level`, `CellUnion::from_min_max`
  and `Cap::expanded`, and fix any row that cannot answer the four questions. Then commit:
  `docs: classify range and validity preconditions for bounded types (spec 009)`.

**Checkpoint**: the decision record exists. Its recommendations for the four prototyped
candidates drive Phase 4.

---

## Phase 4: User Story 2, a prototype through core, Python and wasm (P1)

**Goal**: each of the four candidates changes through the core, the Python binding and the
WebAssembly binding in one red/green commit. The compiler's error lists give the exact
call-site counts (FR-005), and every binding converts each kind in exactly one place (FR-007).
Because the maintainer chose to adopt the four (FR-012), these commits are the adoption.

**Independent Test**: after each candidate's commit, its PY, WA, CO and CD tests in
contracts/red-green-tests.md are green, `make test` passes, and `BOUNDED_TYPES.md` §3 has its
measured counts.

### Candidate 1: `Level` (R1, R2)

- [X] T018 [P] [US2] **RED** PY-01, PY-02, PY-03, PY-04 and PY-10 for levels, in
  `python/tests/test_bounded_args.py` (new; SPDX header as in
  `python/tests/test_bounds_checks.py`; module docstring "red/green tests for spec 009's bounded
  arguments").
  - Parametrise over a table of `(label, callable taking the level)`, covering:
    - `CellId.parent_at_level`, `child_position`, `child_begin_at_level`, `child_end_at_level`,
      `vertex_neighbors` and `all_neighbors`, on `CellId.from_face(0).child_begin_at_level(30)`,
      a leaf, so that a level in range is legal;
    - `CellId.from_face_pos_level(0, 0, level)`;
    - `CellUnion.expand_at_level`;
    - `CellUnion.denormalize(level, 1)`;
    - `RegionCoverer(min_level=…)` and `RegionCoverer(max_level=…)`;
    - `RegionTermIndexer(min_level=…)` and `(max_level=…)`;
    - `Metric.value`, on one metric instance;
    - `S2CellIdSnapFunction(level)`;
    - `Polygon.snapped(polygon, level)` and `Polygon.simplified(polygon, level)`.
  - Inputs:
    - 31, 255, 256, −1, 2**40 and 2**70 → `ValueError` matching `level must be in 0\.\.=30`;
    - 0 and 30 → no exception;
    - 3.5, `"5"` and `None` → `TypeError`;
    - `True` → the same result as 1.
  - PY-10: a helper that runs every out-of-range call and asserts that each raised exception is
    an instance of `Exception`, so that `PanicException` fails it.
  - Run `uv run --directory python pytest tests/test_bounded_args.py -rA` and record red.
- [X] T019 [P] [US2] **RED** WA-01 and WA-02 in `wasm/tests/bounded_args.rs` (new, original
  header).
  - A helper `call(target: &JsValue, method: &str, args: &[JsValue]) -> Result<JsValue, JsValue>`:
    `js_sys::Reflect::get` the method, cast it to `js_sys::Function`, and call `.apply` with a
    `js_sys::Array` of the arguments.
  - Instance targets: `JsValue::from(s2rst_wasm::CellId::…)`, a leaf at level 30.
  - Static targets (`begin`, `end`, `fromFacePosLevel`, `averageAreaForLevel`, `SnapFunction`'s
    `cellId`): `Reflect::get(&instance, &"constructor".into())`.
  - Methods: `parentAtLevel`, `childPosition`, `childBeginAtLevel`, `childEndAtLevel`,
    `vertexNeighbors`, `allNeighbors`, `begin`, `end`, `fromFacePosLevel`,
    `Cell.averageAreaForLevel`, `CellUnion.expandAtLevel`, `RegionCoverer.setMinLevel` and
    `setMaxLevel`, `RegionTermIndexer.setMinLevel` and `setMaxLevel`, `Polygon.snapped`, and
    `SnapFunction.cellId`.
  - Inputs:
    - `31.0`, `286.0`, `-1.0`, `f64::NAN`, `3.5` and `f64::INFINITY` must return `Err`, an
      `Error` whose message contains `level must be`;
    - `js_sys::BigInt::from(5)` must return `Err`, a `TypeError`;
    - after each, a valid call (level 5) on a fresh object must succeed;
    - WA-02: 0, 30 and 5.0 succeed.
  - One `#[wasm_bindgen_test]` per method. At red time, run each test alone, because a trap can
    poison the instance:
    `wasm-pack test --node /home/t/sources/s2rst_multi/3/wasm --test bounded_args -- <name>`.
  - Record red. 286 should answer for level 30 without an error, 31 should give
    `RuntimeError: unreachable`, and `NaN` should give level 0.
- [X] T020 [US2] **RED** CD-01 and CD-05, as doctests on `Level`'s rustdoc in
  `core/src/s2/coords.rs`:
  - `compile_fail` blocks `let l: s2rst::s2::Level = 31u8.into();` and
    `s2rst::s2::CellId::from_face(s2rst::s2::Face::F0).parent_at_level(5u8);`;
  - beside them, twin blocks that compile: `Level::new(30)` and `parent_at_level(Level::new(5))`;
  - a `compile_fail` block `const L: s2rst::s2::Level = s2rst::s2::Level::new(31);`, with the
    twin `Level::new(30)`.

  Run `cargo test -p s2rst --all-features --doc Level` and record red for CD-01 ("test compiled
  successfully"). CD-05 already passes.
- [X] T021 [US2] Change `Level` in `core/src/s2/coords.rs`:
  - remove `impl From<u8> for Level`;
  - add `TryFrom<u8>` and `TryFrom<i64>`;
  - change the error type of the existing `TryFrom<u16>`, `TryFrom<i32>` and `TryFrom<usize>`
    from `&'static str` to `OutOfRange` (`BoundedKind::Level`);
  - add `pub const fn checked_add(self, rhs: u8) -> Option<Level>` and
    `checked_sub(self, rhs: u8) -> Option<Level>`;
  - switch the T010 deserialiser to `Level::try_from(raw.0)`.

  Keep `new`, `try_new`, the operators, `PartialEq<u8>` and `PartialOrd<u8>`. Rewrite the type's
  documentation: literals use `Level::new(n)` (in a constant context the check happens when the
  code is compiled), and values computed at run time use `Level::try_from(n)?`.
- [X] T022 [US2] Change the 23 signatures of `contracts/signature-audit.md`'s first table from
  `level: impl Into<Level>` to `level: Level`, and drop the `.into()` in their bodies:
  - `core/src/s2/metric.rs`;
  - `core/src/s2/cell_id.rs`, 13 signatures;
  - `core/src/s2/testing.rs`;
  - `core/src/s2/region_coverer.rs`, 2;
  - `core/src/s2/polygon.rs`;
  - `core/src/s2/cell.rs`, 2;
  - `core/src/s2/builder/snap.rs`, 3.

  Delete every `# Panics` clause that says "the conversion to `Level` panics", for example in
  `CellId::from_face_pos_level`. Leave the face parameters for T033.
- [X] T023 [US2] Fix the call sites crate by crate, guided by the compiler, and count each kind
  of rewrite. Run
  `cargo check -p s2rst --all-targets --all-features --message-format short 2>&1 | grep -cE "error\[E0(277|308)\]"`
  before each crate, and record:
  - **literal**: `f(5)` → `f(Level::new(5))`;
  - **conversion**: a `u8` or `i32` variable → `Level::try_from(v)` with `?` or `.expect("…")`,
    or a restructure so the value is held as a `Level`;
  - **loop**: `for level in 0..=30` → `for level in (0..=30).map(Level::new)`;
  - **already typed**: no change.

  The crates, in order: `core/src` (including the inline test modules), `core/tests`,
  `core/benches`, `core/examples` and `fuzz/fuzz_targets`. Put the per-crate table in
  `BOUNDED_TYPES.md` §3 (Level).
- [X] T024 [US2] Python. In `python/src/bounded.rs` (new, two-line SPDX header; `mod bounded;` in
  `python/src/lib.rs`):
  - `extract_integer(obj, kind: BoundedKind) -> PyResult<i64>`: PyO3's `extract::<i64>()`,
    through `__index__`. A `TypeError` is kept. An `OverflowError` becomes a `ValueError` with
    `"{kind.name()} must be in {lo}..={hi}, got {str(obj)}"`.
  - `value_error(OutOfRange) -> PyErr` and `index_error(OutOfRange) -> PyErr`.
  - `pub(crate) struct LevelArg(pub s2rst::s2::Level)`, implementing `FromPyObject` through
    `extract_integer(…, BoundedKind::Level)`, then `Level::try_from`, then `value_error`.

  Change every level parameter the compiler flags to `LevelArg`. That is at least `cells.rs`
  (`from_face_pos_level`, `child_position`, `parent_at_level`, `child_begin_at_level`,
  `child_end_at_level`, `vertex_neighbors`, `all_neighbors`, `expand_at_level`, and
  `denormalize`'s `min_level`), `coverer.rs` (`Option<LevelArg>`), `term_index.rs`,
  `metric.rs`, `snap.rs` and `geometry.rs` (`snapped`, `simplified`).

  Delete the ad-hoc `to_level` (`term_index.rs:21`) and `denormalize`'s hand-written level check
  (`cells.rs:667–684`, the `min_level` part). Each docstring states "Raises `ValueError` unless
  0 <= level <= 30". Keep `int` in `python/python/s2rst/_core.pyi`.
- [X] T025 [US2] Check research R8's assumption. Rebuild, then run PY-01:
  - If the `ValueError` reaches the caller unchanged, record "confirmed" in research.md R8 and in
    `BOUNDED_TYPES.md` §3.
  - If it arrives as a `TypeError("argument 'level': …")`, change the methods to take
    `&Bound<'_, PyAny>` and call `LevelArg::extract_bound` (or the 0.30 equivalent) in the body.
    That is still one conversion per kind. Record the deviation from R8, and confirm that
    stubtest still passes.
- [X] T026 [US2] wasm. In `wasm/src/bounded.rs` (new, two-line SPDX header; `mod bounded;` in
  `wasm/src/lib.rs`):
  - `integer_arg(v: f64, kind: BoundedKind) -> Result<i64, JsValue>`: reject `NaN`, ±∞,
    `v.fract() != 0.0` and |v| > 2⁵³ with
    `js_err("{name} must be an integer in {lo}..={hi}, got {v}")`;
  - `range_error(OutOfRange) -> JsValue` (`js_err(e)`);
  - `level_arg(v: f64) -> Result<Level, JsValue>`.

  Change every level parameter the compiler flags from `u8` to `f64`, with a
  `Result<_, JsValue>` return:
  - `cell_id.rs`: `parent_at_level`, `from_face_pos_level`'s level, `child_position`,
    `child_begin_at_level`, `child_end_at_level`, `begin`, `end`, `vertex_neighbors` and
    `all_neighbors`;
  - `cell.rs`: `average_area_for_level`;
  - `cell_union.rs`: `expand_at_level`;
  - `region_coverer.rs`: `set_min_level` and `set_max_level`;
  - `term_indexer.rs`, replacing `Level::new(level)`;
  - `polygon.rs`: `snapped`;
  - `snap.rs`: `cell_id`.

  Each rustdoc states the range and "Throws an `Error` unless …". Update `wasm/tests/web.rs`
  where existing tests now pass `f64`, or need `.unwrap()`.
- [X] T027 [US2] Make `verify/` compile with the new signatures (FR-016's redesign is
  T063–T065):
  - in `verify/src/*.rs`, convert raw levels with `Level::new(…)` or `Level::try_from(…)` where a
    harness passes a `u8`;
  - `crash::n_raw_level` keeps its 13 operations for now, converting first, and is re-pointed in
    T065.

  Check with `cargo test --manifest-path verify/Cargo.toml` and
  `make verify-harness H=crash::<the cheapest existing c_ harness>`; the codegen must succeed.
- [X] T028 [US2] **GREEN** `Level`:
  - T018's PY tests, T019's WA tests and T020's CD tests pass;
  - `make test` passes;
  - rerun the T003 benchmark command and save the output as
    `specs/009-bounded-types/bench/after-level-<rev>.txt`;
  - record the green results and the instruction-count comparison in `BOUNDED_TYPES.md` §3 and
    §4.
- [X] T029 [US2] Commit: `feat(s2)!: take Level, not impl Into<Level>; bindings reject bad
  levels`, with `BREAKING CHANGE` footers for M1, M2 and M3 of contracts/migration-notes.md.

### Candidate 2: `Face` (R4)

- [X] T030 [P] [US2] **RED** PY-05 in `python/tests/test_bounded_args.py`:
  - `CellId.from_face(n)` and `CellId.from_face_pos_level(n, 0, 0)` for n in 6, 7, 255, −1 and
    256 must raise `ValueError` matching `face must be in 0\.\.=5`;
  - 0..=5 work;
  - add the face calls to PY-10's list;
  - `toolkit.face_uv_to_xyz` and `get_u_axis`/`get_v_axis` (`ValueError "invalid face"` today)
    move to the shared message.

  Record red: 6 and 7 return an invalid id, and −1 and 256 give `OverflowError`.
- [X] T031 [P] [US2] **RED** WA-03 in `wasm/tests/bounded_args.rs`:
  - `CellId.fromFace`, `Cell.fromFace` and `CellId.fromFacePosLevel`'s face, with 6, 7, −1,
    `NaN` and 3.5, must throw an `Error` containing `face must be`;
  - the instance stays usable.

  Record red: 7 gives an invalid id, `NaN` gives face 0, and 3.5 gives face 3.
- [X] T032 [US2] **RED** CD-02: a `compile_fail` block `s2rst::s2::CellId::from_face(7u8);` with
  the twin `CellId::from_face(Face::F5)`, on `CellId::from_face`'s rustdoc in
  `core/src/s2/cell_id.rs`. Run the doctests and record red.
- [X] T033 [US2] Core:
  - `CellId::from_face`, `Cell::from_face`, `CellId::from_face_pos_level` and
    `Cell::from_face_pos_level` take `face: Face` (`core/src/s2/cell_id.rs:224,239`,
    `core/src/s2/cell.rs:169,181`);
  - `Face`'s `TryFrom<u8>` returns `OutOfRange` (`BoundedKind::Face`) instead of `u8`;
  - add `TryFrom<i64>`;
  - document on `CellId::from_face` that upstream's `FromFace(int)` gives an undefined id for 6
    and 7, which this signature makes unrepresentable (FR-011).
- [X] T034 [US2] Fix the call sites crate by crate, as in T023: `from_face(n)` →
  `from_face(Face::Fn)`, `for f in 0..6` → `for f in Face::ALL`, and a value →
  `Face::try_from(v)`. The crates are `core/src`, `core/tests`, `core/benches`, `core/examples`,
  `fuzz/` and `verify/src`. Record the per-crate counts in §3 (Face).
- [X] T035 [US2] Bindings:
  - Python: a `FaceArg` in `python/src/bounded.rs`, used by `cells.rs` (`from_face`, and
    `from_face_pos_level`'s face) and `toolkit.rs` (replacing `face_from_int`; the message
    becomes the shared one);
  - wasm: `face_arg(f64)` in `wasm/src/bounded.rs`, used by `cell_id.rs` (`from_face`,
    `from_face_pos_level`) and `cell.rs` (`from_face`), with `Result` returns;
  - update `wasm/tests/web.rs`, and any Python test pinning `"invalid face"`.
- [X] T036 [US2] **GREEN** `Face`: PY-05, WA-03 and CD-02 pass, `make test` passes, and the
  `verify/` codegen check of T027 succeeds. Record in §3 and §4.
- [X] T037 [US2] Commit: `feat(s2)!: face constructors take Face; bindings reject bad faces`,
  with `BREAKING CHANGE` footers for M4 and M5.

### Candidate 3: `LevelMod` and the coverer's settings (R6, FR-010)

- [X] T038 [P] [US2] **RED** PY-06, PY-07 and PY-08 in `python/tests/test_bounded_args.py`:
  - `RegionCoverer(level_mod=n)` and `RegionTermIndexer(level_mod=n)` for n in 0, 4, 255 and −1
    must raise `ValueError` matching `level_mod must be in 1\.\.=3`;
  - `CellUnion.denormalize(0, n)` for n in 0, 4, −1 and 2**40 raises the same;
  - `RegionCoverer(min_level=10, max_level=5)` and the term indexer likewise must raise
    `ValueError` matching `min_level \(10\) must not exceed max_level \(5\)`.

  Record red: `RegionCoverer` clamps silently, and the min > max coverer constructs.
- [X] T039 [P] [US2] **RED** WA-04 and WA-05 in `wasm/tests/bounded_args.rs`:
  - `RegionCoverer.setLevelMod` with 0, 4, `NaN` and 2.5 must throw `level_mod must be`;
  - `new RegionCoverer().setMinLevel(10).setMaxLevel(5)` followed by each covering method
    (`coveringCap`, `coveringRect`, `coveringLoop`, `coveringPolygon`, `coveringCellUnion`, the
    `interiorCovering*` methods and the `fastCovering*` methods), on a small `Cap`, must throw
    `min_level (10) must not exceed max_level (5)`.

  Record red: a silent clamp, and a trap in the debug build.
- [X] T040 [US2] **RED** CD-03 and CO-02's `level_mod` half:
  - `compile_fail`: `RegionCoverer { level_mod: 7, ..Default::default() }`, with the twin
    `level_mod: LevelMod::new(3)`, on `RegionCoverer`'s rustdoc in
    `core/src/s2/region_coverer.rs`;
  - in `core/src/s2/region_coverer_tests.rs`, a coverer JSON with `"level_mod": 7` must fail to
    deserialise;
  - CO-03 does not apply: `region_term_indexer::Options` does not derive serde (checked
    2026-10-27). Record that in §4.

  Record red.
- [X] T041 [US2] Add `LevelMod` to `core/src/s2/bounded.rs`, with tests in
  `core/src/s2/bounded_tests.rs` written first. **"Invariant: 1 ≤ value ≤ 3."**
  - `pub const fn new(v: u8) -> Self`, which panics outside 1..=3, documented in `# Panics`;
  - `pub const DEFAULT: LevelMod`, which is 1, and `impl Default`;
  - `TryFrom<u8>`, `TryFrom<usize>` and `TryFrom<i64>`, returning `OutOfRange`
    (`BoundedKind::LevelMod`);
  - `get() -> u8`;
  - `branching_factor() -> u32`, which is 4, 16 or 64;
  - `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash` and `Display`;
  - `Serialize` derived; `Deserialize` hand-written as in T010, with
    `#[serde(rename = "LevelMod")] struct Raw(u8)`.

  The tests cover every `u8`, the ends of `i64`, `branching_factor`, `Display`, and a serde
  round trip. Re-export `LevelMod` from `core/src/s2/mod.rs`.
- [X] T042 [US2] Core uses:
  - `CellUnion::denormalize(&self, min_level: Level, level_mod: LevelMod)` in
    `core/src/s2/cell_union.rs:529`: remove the `assert!` and the `level_mod` clause of
    `# Panics`, and use `level_mod.get()` where an integer is needed;
  - `RegionCoverer` in `core/src/s2/region_coverer.rs`: `pub level_mod: LevelMod`, `Default`
    using `LevelMod::DEFAULT`, and `pub fn level_mod(mut self, m: LevelMod) -> Self` with no
    clamp. Its rustdoc documents the deviation from upstream's release-build clamp (FR-011);
  - the `# Panics` of `covering` and `interior_covering` state the debug-build check
    `min_level ≤ max_level` (upstream's `DCHECK`);
  - the internal `level_mod: u8` at `region_coverer.rs:449` becomes `LevelMod`;
  - `region_term_indexer::Options::level_mod: LevelMod` in
    `core/src/s2/region_term_indexer.rs:40`, with `Default` `LevelMod::DEFAULT`.
- [X] T043 [US2] Fix the call sites crate by crate, as in T023, including
  `verify/src/cu_crash.rs`'s `n_denormalize__n2` (convert for now, and re-point in T065). Record
  the counts in §3 (LevelMod).
- [X] T044 [US2] Bindings:
  - Python: `LevelModArg` in `python/src/bounded.rs`, and
    `check_level_order(min: Level, max: Level) -> PyResult<()>` raising
    `ValueError("min_level ({min}) must not exceed max_level ({max})")`. Use them in
    `coverer.rs` (the constructor: convert, then check the relation once both are known),
    `term_index.rs` (the constructor) and `cells.rs` (`denormalize`, deleting its hand-written
    `level_mod` check).
  - wasm: `level_mod_arg(f64)` and `check_level_order` in `wasm/src/bounded.rs`.
    `region_coverer.rs`'s `set_level_mod` takes `f64` and returns `Result`, and every covering
    method calls `check_level_order(self.0.min_level, self.0.max_level)?` first and returns
    `Result<CellUnion, JsValue>`. Apply the same to `term_indexer.rs`'s term methods, if they
    cover.
  - Update `wasm/tests/web.rs` and the Python tests that relied on the clamp, such as A12's
    expectations in `python/tests/test_bounds_checks.py`, if their messages change.
- [X] T045 [US2] **GREEN** `LevelMod`: PY-06 to PY-08, WA-04, WA-05, CD-03 and CO-02 pass,
  `make test` passes, and the `verify/` codegen check succeeds. Record in §3 and §4.
- [X] T046 [US2] Commit: `feat(s2)!: LevelMod for level moduli; bindings check coverer
  levels`, with `BREAKING CHANGE` footers for M6, M7, M8 and M10.

### Candidate 4: `CellVertex`, gated (R5)

- [X] T047 [US2] Measure the gate before changing anything. Classify every call of
  `Cell::vertex` and `Cell::vertex_raw` in `core/src`, `core/tests`, `core/benches`,
  `python/src`, `wasm/src` and `verify/src`. Find them with
  `grep -rnE "\.vertex(_raw)?\(" --include=*.rs`, then filter to `Cell` receivers by reading
  each. The categories:
  - **literal**: 0–3;
  - **loop**: `for k in 0..4`;
  - **wrap arithmetic**: `(k + 1) % 4`, `k & 3`;
  - **integer needed**: the index also addresses another array, or comes from external data.

  Record the table and the share of "integer needed" in `BOUNDED_TYPES.md` §3 (CellVertex).
  - **If the share is above 25%** (choose O1): skip T048 and T050. Do T049, then T051 with each
    binding's conversion producing a checked `usize` index (the kind is still converted in one
    place), then T052 without CD-04 and with the commit message
    `fix(bindings): one checked conversion for cell vertex and edge indices`. Document the O1
    decision, and the deviation from upstream's wrap, in `Cell::vertex`'s rustdoc and in §5.
    Then go to T053.
  - **Otherwise**: continue with T048.
- [X] T048 [US2] **RED** CD-04: a `compile_fail` block `cell.vertex(4usize)`, with the twin
  `cell.vertex(CellVertex::UpperLeft)`, on `Cell::vertex`'s rustdoc in `core/src/s2/cell.rs`.
  Record red.
- [X] T049 [P] [US2] **RED** PY-09 and WA-06:
  - Python: `Cell.vertex` and `Cell.vertex_raw` with 4, −1 and 2**70, and `Cell.edge` and
    `Cell.edge_raw` with 4 and −1, must raise `IndexError` matching
    `cell (vertex|edge) index must be in 0\.\.=3`;
  - wasm: `Cell.vertex` (and `vertexRaw`, if exported) and `Cell.edge`, with 4, −1, `NaN` and
    3.5, must throw.

  Record red: −1 gives `OverflowError` in Python, and `NaN`/3.5 give vertex 0 and 3 in wasm.
- [X] T050 [US2] Core: add `CellVertex` beside `CellEdge` in `core/src/s2/cell.rs`, with tests
  first in `core/src/s2/cell_tests.rs`:
  - `#[must_use] #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]`, with serde
    derived as for `CellEdge`;
  - the variants `LowerLeft = 0` (the default), `LowerRight = 1`, `UpperRight = 2` and
    `UpperLeft = 3`, documented as the corners of the (u, v) rectangle, counter-clockwise;
  - `ALL`, and `next()` and `prev()`, which wrap modulo 4 as `RectVertex`'s do;
  - `index() -> usize`;
  - `TryFrom<usize>` and `TryFrom<i64>`, returning `OutOfRange` (`BoundedKind::CellVertex`).

  Also add `TryFrom<i64> for CellEdge`, returning `OutOfRange` (`BoundedKind::CellEdge`).
  `Cell::vertex` and `vertex_raw` take `CellVertex`, and their `# Panics` sections are deleted.
  Re-export `CellVertex` from `core/src/s2/mod.rs`. Then fix the call sites crate by crate (T023
  style), including `verify/src/cl_crash.rs`'s `n_vertex`, which converts for now and is
  re-pointed in T065. Record the counts.
- [X] T051 [US2] Bindings:
  - Python: `CellVertexArg` and `CellEdgeArg` in `python/src/bounded.rs`, raising `IndexError`.
    Use them in `cells.rs` (`vertex`, `vertex_raw`, `edge`, `edge_raw`), deleting
    `cell_edge_from_index` and the hand-written vertex checks.
  - wasm: `cell_vertex_arg(f64)` and `cell_edge_arg(f64)`, used in `cell.rs` (`vertex`, `edge`).
  - Keep A1–A3 of `python/tests/test_bounds_checks.py` passing. Update their expected messages
    only if they pin text.
- [X] T052 [US2] **GREEN** `CellVertex`:
  - PY-09, WA-06 and CD-04 pass;
  - `make test` passes;
  - the `verify/` codegen check succeeds;
  - rerun the benchmarks and save them as `bench/after-cellvertex-<rev>.txt`, with the cell
    benches compared to T028's.

  Record in §3 and §4. Commit: `feat(s2)!: CellVertex for cell vertex indices`, with a
  `BREAKING CHANGE` footer for M9.
- [X] T053 [US2] Replace every "estimate (replaced by T053)" in `BOUNDED_TYPES.md` §3 with the
  measured numbers from T023, T034, T043, T047/T050 and the benchmark files:
  - call sites per crate and per kind of rewrite;
  - panics removed;
  - each binding's before and after behaviour, from the red/green table;
  - the instruction-count deltas.

  Re-state the recommendation for each candidate (SC-002).

**Checkpoint**: all four candidates are through all three layers, green, and measured.

---

## Phase 5: User Story 3, adopting what the study recommends (P2)

**Goal**: check every adoption criterion of the spec and close it: FR-009 to FR-011, FR-013 to
FR-015, SC-004, SC-005, SC-008 and SC-009.

**Independent Test**:

- no `# Panics` section mentions an adopted argument's range;
- the deviations are documented;
- the stub types are unchanged;
- the serialised forms are unchanged;
- the migration notes match the commits.

- [X] T054 [US3] Audit the `# Panics` sections (FR-014, SC-004). Run
  `grep -rnE -A4 "# Panics" core/src | grep -nE "greater than 30|> 30|conversion to .Level|level_mod|k > 3|face"`
  and read every hit. No operation may document a panic for an adopted argument's range, except
  `Level::new`, `LevelMod::new`, `Face::from_u8` and the `Level` operators. Record the count
  before (from `git show <T011 commit>:<file>`) and after in `BOUNDED_TYPES.md` §3.
- [X] T055 [P] [US3] Complete the deviation documentation (FR-011, SC-008). The rustdoc of each of
  these states upstream's behaviour and the reason:
  - `RegionCoverer::level_mod`;
  - `LevelMod`;
  - `CellId::from_face`;
  - `Cell::vertex` or `CellVertex`;
  - `RegionCoverer::max_cells`, a pre-existing deviation that is documented, not changed.

  `BOUNDED_TYPES.md` §5 lists each, with item, upstream, library, reason and the rustdoc
  location.
- [X] T056 [P] [US3] Check the docstrings and the stub (FR-014):
  - every `#[pymethods]` method that takes a bounded kind says "Raises `ValueError`" (or
    `IndexError`) and gives its range;
  - `git diff <T011 commit> -- python/python/s2rst/_core.pyi` shows no parameter-type change;
  - `make -C python test` (which runs stubtest) passes.
- [X] T057 [P] [US3] Check the TypeScript declarations (FR-014). Build with
  `wasm-pack build --dev --target nodejs --out-dir <scratchpad>/pkg /home/t/sources/s2rst_multi/3/wasm`,
  then check `<scratchpad>/pkg/*.d.ts`: every changed method still takes `number`, and its JSDoc
  states the range and that it throws. Record the result in §3.
- [X] T058 [US3] Check the serialised forms and parity (FR-015):
  - CO-04 is green;
  - `git diff --stat <T011 commit> -- core/tests/data tools/` is empty, so the CSVs are
    unchanged;
  - the differential tests pass, as part of `make test`.

  Record the result in §4.
- [X] T059 [US3] Finalise the migration notes. Move contracts/migration-notes.md into
  `BOUNDED_TYPES.md` §6 (anchor `migration-notes`), corrected to what the commits did: entries
  that changed, and entries that the gate dropped (M9, if T047 chose O1). Check that each
  breaking commit's footer names its entries: `git log --format=%B <T011 commit>..HEAD`.
- [X] T060 [P] [US3] Add the rule to `CONTRIBUTING.md`, in the section on public API: "A
  parameter restricted to a range or a small set takes a bounded type (`Level`, `Face`,
  `LevelMod`, `CellVertex`, or a new one in `s2::bounded`), never a raw integer. Conversions
  from raw integers are `TryFrom` returning `OutOfRange`, never `From`. A parameter of a bounded
  type is plain `T`, not `impl Into<T>` (see `BOUNDED_TYPES.md`)."
- [X] T061 [US3] Run the signature checks of quickstart.md §2 (the two `grep`s). Both must show
  nothing, apart from `bounded.rs`. Record the output in §3, then commit:
  `docs: bounded types adoption: deviations, migration notes, contribution rule`.

**Checkpoint**: the adoption is complete and documented.

---

## Phase 6: User Story 4, proofs and records kept in step (P3)

**Goal**: each adopted type's constructor is proven exact. The operations that lost a range
panic are proven over the type. Gap 1's complements are re-pointed, and the mutants show the
proofs non-vacuous. All records agree (FR-016, FR-017).

**Independent Test**:

- `make verify`, `make verify-values` and `make verify-release` are all SUCCESSFUL;
- the new and re-pointed mutants are killed;
- `cargo test --manifest-path verify/Cargo.toml` passes;
- `verify/COVERAGE.md` points the 13 raw-level operations, `denormalize` and `Cell::vertex` to
  the constructor proofs.

- [X] T062 [US4] Check the preconditions: `make verify-kani-check` passes, `df -h /` shows at
  least 20 GB free, and `pgrep -f '[c]bmc'` shows no other run.
- [X] T063 [US4] In `verify/src/domain.rs`, add `any_level() -> Level` (a symbolic `u8` with
  `kani::assume(v <= 30)`, then `Level::new(v)`), `any_face()`, `any_level_mod()` and
  `any_cell_vertex()` (from `ALL` with a symbolic index). Switch the crash proofs of the
  operations that take these types to draw them:
  - `verify/src/crash.rs`, for the 13 raw-level operations and the face constructors;
  - `verify/src/cu_crash.rs`, for `denormalize`;
  - `verify/src/cl_crash.rs`, for `Cell::vertex` and `vertex_raw`.
- [X] T064 [US4] Write the constructor proofs, each `n_` harness with the trailing
  `kani::cover!(true, "returns")` convention of `verify/COVERAGE.md` (covers `never`):
  - `verify/src/crash.rs`:
    - `c_level_try_from`: `Level::try_from(u8)` is `Ok` exactly for v ≤ 30, and `as_u8` gives
      `v` back;
    - `n_level_new`: `Level::new` panics exactly for v > 30;
    - `c_face_try_from`, and `n_face_from_u8`;
  - `verify/src/cu_crash.rs`: `c_level_mod_try_from`, and `n_level_mod_new` (outside 1..=3);
  - `verify/src/cl_crash.rs`: `c_cell_vertex_try_from` (`Ok` exactly for k ≤ 3).

  Add each `n_` harness to `make verify-release`'s every-build set, if `Makefile`'s pattern does
  not already match it.
- [X] T065 [US4] Re-point gap 1's complements:
  - **retire** `crash::n_raw_level`: its 13 operations take `Level`, and no input panics on the
    range. `n_level_new` covers the range;
  - **retire** the `level_mod` half of `cu_crash::n_denormalize__n2`, and keep its id-validity
    half;
  - **retire** `cl_crash::n_vertex` under O4 (under O1, keep it).

  Update `verify/known-timeouts.txt` only if a listed name changed.
- [X] T066 [US4] Mutants in `verify/mutants/` (header format as in
  `verify/mutants/M-803-level-new-unchecked.patch`):
  - regenerate `M-803` against `Level::new`'s new location, with `Expect-Fail:
    crash::n_level_new`;
  - re-point `M-806` to `LevelMod::new`'s check, with `Expect-Fail: cu_crash::n_level_mod_new`;
  - new: `M-809-face-try-from-accepts-6`, `M-810-level-mod-try-from-accepts-0` and
    `M-811-cell-vertex-try-from-accepts-4` (the last only under O4).

  Add their rows to `verify/MUTANTS.md`. Check that every patch still applies, using the
  pre-pass of `verify/run-mutants.sh`, or `git apply --check` for each patch.
- [X] T067 [US4] Run, one at a time:
  1. `make verify`;
  2. `make verify-values`;
  3. `make verify-release`;
  4. `make verify-mutants M='M-803 M-806 M-809 M-810 M-811'`.

  Every harness must be SUCCESSFUL, every `n_` cover `never`, and every mutant killed. If an
  older mutant patch went stale from the signature edits, regenerate it with the same mutation
  lines and re-check it, as the coverage work did. Quote each harness's time from its status line.
- [X] T068 [P] [US4] Update the records in the same commit as the harnesses (constitution
  v1.2.0):
  - `verify/INVENTORY.md` §3, `verify/cell_union/INVENTORY.md` §3 and
    `verify/values/INVENTORY.md` §10: the re-pointed rows;
  - `verify/COVERAGE.md`: the gap-1 table's rows for the raw-level operations, `denormalize` and
    `Cell::vertex`, plus a "Spec 009: bounded types" subsection with the constructor proofs and
    the final run;
  - `verify/PROPERTIES_BY_KIND.md` and `verify/PROPERTIES_BY_KIND.html`: "Panics exactly where
    documented" gains the constructors and loses the retired complements;
  - `VERIFIED_TYPES.md`: a "Bounded types" subsection listing `Level`, `Face`, `LevelMod` and
    `CellVertex`, each with its constructor proof.

  Run `cargo test --manifest-path verify/Cargo.toml`, which includes the index consistency check.
- [X] T069 [US4] Commit: `feat(verify): constructor proofs for the bounded types; re-point gap-1
  complements`.

**Checkpoint**: the proofs and records match the code.

---

## Phase 7: Polish and cross-cutting concerns

- [X] T070 [P] Write `BOUNDED_TYPES.md` §7, the ranked follow-ups (FR-018). Each gets a measured
  or estimated cost:
  - the class (d) and (e) invariants of T014;
  - `r2::Rect::vertex`;
  - `get_uvw_axis` and `get_uvw_face` (`Axis`);
  - `DensityTree::new`'s level;
  - `max_cells`;
  - checking typed identifiers at construction;
  - the `level!` macro (with T023's literal count);
  - a Python `IntEnum` for faces;
  - TypeScript literal unions (`unchecked_param_type`).
- [X] T071 Check the red/green table: every ID in contracts/red-green-tests.md has a row in
  `BOUNDED_TYPES.md` §4, with red output, red date and commit, and green commit, or an
  explicit "not applicable" (CO-03; CD-04/M-811 under O1).
- [X] T072 Run the full gate from a clean build: `make test`. Then `cargo +1.88 check -p s2rst
  --all-features` if that toolchain is installed, which is the CI `msrv` job; otherwise note
  that CI covers it.
- [X] T073 Walk quickstart.md §1–§5 end to end, and fix any step that no longer matches.
- [X] T074 Update spec.md's **Status** line (implemented, with the date, the commits, the met
  and unmet success criteria, and the R5 gate's outcome), and update `checklists/requirements.md`
  if anything changed.
- [X] T075 Commit: `docs: complete the bounded-types evaluation (spec 009)`. Do not push.

---

## Dependencies and execution order

### Phase dependencies

- **Setup (Phase 1)**: T001 blocks everything. T002 and T003 must run on the unfixed tree.
- **Foundational (Phase 2)**: depends on T001. It blocks every story: `OutOfRange` is used by
  every `TryFrom`, and B2's red must be seen before T042 touches the coverer.
- **US1 (Phase 3)**: depends on Phase 2 only for §4's first rows. It can run alongside US2's red
  tasks, since both edit `BOUNDED_TYPES.md` in different sections. Its §3 estimates are replaced
  by T053.
- **US2 (Phase 4)**: depends on Phase 2. The candidates run in order: Level (T018–T029), then
  Face (T030–T037), then LevelMod (T038–T046), then CellVertex (T047–T052), then T053.
  - The order matters for Face and LevelMod: `from_face_pos_level` changes in both T022 and T033,
    and the bindings' `bounded.rs` grows per candidate.
  - LevelMod needs the T024 and T026 binding modules.
- **US3 (Phase 5)**: depends on all of US2.
- **US4 (Phase 6)**: depends on US2. It may run alongside US3 (different files), but Kani runs
  stay sequential.
- **Polish (Phase 7)**: depends on everything.

### Within each candidate

RED tasks (the tests) → core change → call sites → bindings → GREEN → commit. The core's change
comes before the call sites, because the compiler's error list is both the measurement (FR-005)
and the to-do list.

### Parallel opportunities

- T003, T004 and T005, in Phase 1.
- T006 alongside T009's test-writing (different files). But commits T008 and T011 are
  sequential.
- Within a candidate, its Python and wasm RED tasks: T018 ∥ T019, T030 ∥ T031, T038 ∥ T039,
  T049. They touch different files and different toolchains.
- T013, T014 and T016 in US1 (different sections).
- T055, T056, T057 and T060 in US3.
- T068 while T067's runs proceed. Records only: never two Kani runs at once.

---

## Parallel example: User Story 2, candidate 1 (`Level`)

```text
# Write and run the red tests together (different files and toolchains):
Task T018: "RED PY-01..PY-04, PY-10 for levels in python/tests/test_bounded_args.py"
Task T019: "RED WA-01, WA-02 in wasm/tests/bounded_args.rs (each test run alone)"

# After the core change (T021–T023), the two bindings in parallel:
Task T024: "Python LevelArg in python/src/bounded.rs; level parameters → LevelArg"
Task T026: "wasm level_arg in wasm/src/bounded.rs; level parameters → f64 + Result"
```

## Implementation strategy

### MVP first (User Story 1)

1. Phase 1, then Phase 2: the baseline, the shared error, and the B2 fix (two small commits).
2. Phase 3, US1: the decision record. **Stop and validate**: can the maintainer answer the
   spec's question, "can bounded types make these APIs safer, and at what cost?", for every
   candidate? The record alone is a useful deliverable, even if adoption pauses.

### Incremental delivery

1. US2, candidate by candidate. Each commit is complete through all three layers, green, and
   measured, and can be reviewed alone: Level, then Face, then LevelMod, then CellVertex (gated).
2. US3 closes the adoption criteria.
3. US4 brings the proofs and records into step. It is the longest wall-clock phase: the full
   per-change suites took about 1 h 36 min on 2026-10-27.
4. Polish: the follow-ups, the final gate and the status.

### Notes

- Never commit a red test without its fix.
- Never run two Kani jobs at once.
- Never push.
- If a RED task's test does not fail as its contract says, stop and re-check the finding before
  writing the fix.
- Every commit keeps `make test` green, and every breaking commit carries its migration note
  references.
