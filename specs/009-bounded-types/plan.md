# Implementation Plan: Bounded Types for Safer APIs, from the Core to the Bindings

**Branch**: `009-bounded-types/1` | **Date**: 2026-10-27 (probes P1–P7 run the same day) |
**Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/009-bounded-types/spec.md`. Two planning requests
from the maintainer:

1. "Add red/green tests to missing functionality/crashes (for example in python)."
2. "Check if related function signatures containing type T should be rewritten to
   `impl Into<T>`."

## Summary

The feature makes four range-restricted parameters into types that are checked when they are
created. It carries that check through the Python and WebAssembly bindings and records the
evidence for each decision in `BOUNDED_TYPES.md`.

The four candidates (spec FR-005, FR-012):

| Candidate | Today | After |
|-----------|-------|-------|
| Level, 0..=30 | `Level` exists, but `From<u8>` panics. 23 functions take `impl Into<Level>` | `From<u8>` is removed. The 23 take `Level`. `TryFrom` returns `OutOfRange` |
| Face, 0..=5 | `Face` exists, but 4 constructors take `impl Into<u8>`; `from_face(7)` is a silent invalid id | the 4 take `Face` |
| Level modulus, 1..=3 | a `u8` that `denormalize` asserts in every build and the coverer's setter clamps; public fields bypass the clamp | new `LevelMod` |
| Cell vertex index, 0..=3 | a `usize` that panics; upstream wraps modulo 4 | new `CellVertex` enum, adopted if R5's gate passes |

**The `impl Into<T>` question (research R1): no, the change goes the other way.**

- `impl Into<T>` is right only where `T` has infallible sources that callers hold.
- A bounded type's sources are raw integers, and converting one can fail by definition.
  `impl Into<Level>` works today only through a `From<u8>` that panics, which breaks `From`'s
  contract. That panic is what Python users see as `PanicException`.
- The probe P3 showed that literal inference through `impl Into<Level>` breaks as soon as a
  second integer source exists.
- So every bounded parameter takes plain `T`. Once `From<u8>` is gone, the compiler lists every
  unchecked call site, in the core and in both bindings. That list is both the measurement and
  the enforcement of one conversion per binding.
- The typed identifiers (`ShapeId`, `VertexId`, `EdgeId`, `InputEdgeId`) keep `impl Into`,
  because their conversions are infallible.
- [contracts/signature-audit.md](contracts/signature-audit.md) gives a verdict for each of the
  about 60 signatures.

**Red/green tests (research R10)**: every crash, silent wrong result and missing check has a
test that is written first, seen failing on the unfixed tree, and committed with its fix.
[contracts/red-green-tests.md](contracts/red-green-tests.md) lists them:

- 10 Python tests, including `PanicException`, `OverflowError`, `from_face(7)` and the silent
  clamp;
- 6 WebAssembly tests through the real JavaScript glue, where wasm-bindgen's `as u8` turns 286
  into level 30 (P4);
- 4 core runtime tests (B2: `Level` deserialises 31);
- 5 `compile_fail` doctest pairs for the type-level guarantees.

The red output of each goes into `BOUNDED_TYPES.md`.

**Bindings**:

- Python takes each kind through one `FromPyObject` extractor. The parameter is still an `int`
  in the stub, and a bad value now raises `ValueError` (`IndexError` for vertices).
- WebAssembly takes each kind as an `f64`, the only parameter type through which a JavaScript
  value arrives exactly, through one helper per kind. A bad value now throws a catchable
  `Error` instead of trapping or wrapping.

## Technical Context

**Language/Version**: Rust, edition 2024. The toolchain is from `rust-toolchain.toml` (1.99.1
on the workstation), with MSRV 1.88. Inline `const { }` blocks and panics in constant
evaluation are stable since 1.79. Python ≥ 3.9 (abi3) through PyO3 0.30. WebAssembly through
wasm-bindgen 0.2.129, with Node 24 for the tests.

**Primary Dependencies**: no changes. `serde` is optional in the core, as today. `js-sys` is
already a runtime dependency of `wasm/`, and the tests use it. Kani 0.68.0 for the proofs.

**Storage**: N/A. Serialised forms of in-range values stay byte-identical (FR-015, CO-04).

**Testing**:

- `make test`, the gate, unchanged: core tests and doctests (`--all-features`), Python (ruff,
  mypy + stubtest, pytest), and wasm (`wasm-pack test --node`).
- New test files:
  - `python/tests/test_bounded_args.py`;
  - `wasm/tests/bounded_args.rs`;
  - `core/src/s2/coords_tests.rs`, `region_coverer_tests.rs` and `bounded_tests.rs`, new
    siblings;
  - `compile_fail` doctests.
- Proofs: `make verify`, `make verify-values`, `make verify-release` and `make verify-mutants`.
- Benchmarks: gungraun's `cell_id`, `cell` and `region_coverer`, run before and after (R13).

**Target Platform**: Linux x86_64 (the workstation and GitHub `ubuntu-latest`);
`wasm32-unknown-unknown` under Node; CPython (abi3).

**Project Type**: a library with two binding crates, in one workspace with a shared version.

**Performance Goals**: no instruction-count regression on the touched hot paths (SC-006).

**Constraints**:

- breaking changes only for the four candidates, in one minor release, each with a migration
  note (FR-013, [contracts/migration-notes.md](contracts/migration-notes.md));
- no geometric result changes (FR-015);
- no `unsafe`: the wasm path uses `f64`, not a hand-written `FromWasmAbi`;
- the Python stub keeps `int`;
- every Kani run is memory-capped and run one at a time.

**Scale/Scope** (P6 and P7 are grep estimates; the exact counts come from the compiler in step
4):

| Item | Count |
|------|-------|
| Signatures that change | 23 `impl Into<Level>` → `Level`; 4 faces → `Face`; 2 vertices → `CellVertex` (gated); 4 moduli → `LevelMod` |
| Call sites | about 820 for levels (about 280 with a literal); about 500 for `from_face`; about 79 for `Cell::vertex`; a handful for moduli |
| Binding methods | at least 14 Python and 16 wasm methods for levels, plus faces, moduli and vertices (FR-007) |
| New core items | `OutOfRange`, `LevelMod`, `CellVertex`, and `TryFrom` impls |
| Red/green tests | 10 Python, 6 wasm, 4 core runtime, 5 doctest pairs |
| Gap-1 complements re-pointed | `n_raw_level` (13 operations), `denormalize`'s modulus, `Cell::vertex` |
| Mutants | M-803 and M-806 regenerated; about 3 new |

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle / rule | Assessment | Status |
|------------------|-----------|--------|
| **I. Upstream Parity** | No result changes for valid input; the differential tests and their CSVs are untouched. The behaviours that newly differ from upstream are documented in rustdoc and `BOUNDED_TYPES.md` (FR-011): rejecting a `level_mod` that upstream clamps in release builds; faces 6 and 7 and vertex index 4 becoming unrepresentable, where upstream gives a garbage id or wraps. The existing `max_cells` clamp, a deviation found by the audit, is recorded as a follow-up | ✅ Pass |
| **II. Robustness & Memory Safety** | Removes every-build panics (`From<u8>`, `denormalize`'s `assert!`, `Cell::vertex`) and a silent invalid id. Closes B2's deserialisation gap. Removes binding traps and `PanicException`s. No `unsafe`. Every remaining panic (`Level::new`, `LevelMod::new`, `Face::from_u8`, the operators, the `min ≤ max` debug check) is documented in `# Panics` | ✅ Pass |
| **III. Test-Backed Changes** | Each finding is red/green, with the red output recorded (R10). New core tests go in sibling `*_tests.rs` files. Python tests use pytest, and wasm tests use `wasm-pack test --node` (Rust tests driving the JavaScript glue through `js_sys::Reflect`) | ✅ Pass |
| **IV. Thin, Synchronized Bindings** | One conversion per kind per binding, with no geometry. Bindings turn invalid input into native errors, which satisfies the SHOULD this principle states. The stub keeps `int`, and stubtest checks it. The bindings and the core change in the same release | ✅ Pass |
| **V. Measured Performance** | Not a performance change. The touched hot paths are measured before and after with gungraun (R13). No `target-cpu=native` | ✅ Pass (measured in the change) |
| **Tech constraints** | No new dependency. MSRV unchanged. SPDX headers on new files (the original-test header for the test files). Every `#[allow]` carries a reason. Public items documented | ✅ Pass |
| **Workflow: the gate** | `make test` green at every commit: tests land with their fixes | ✅ Pass |
| **Workflow: verification records (v1.2.0)** | The harness changes update `INVENTORY.md`, `COVERAGE.md`, `MUTANTS.md`, `VERIFIED_TYPES.md` and the index of properties by kind (and its HTML) in the same commits (R11) | ✅ Pass |
| **Workflow: focused PRs** | One feature, several commits (R14), each a single concern; breaking changes batched into one minor release, as the maintainer chose | ⚠️ Justified, see Complexity Tracking |

**Gate result: PASS**, with the exception recorded below.

**Post-design re-check (after Phase 1)**: still PASS.

- The data model adds two types (`LevelMod` and `CellVertex`) and one error (`OutOfRange`),
  and nothing else to the public API beyond the listed `TryFrom` impls and `checked_*`.
- The contracts confine the bindings to one conversion module each, and keep their signatures.
- The red/green tests run through stable surfaces, so the red runs are honest.
- No contract changes a geometric algorithm, an encoding or a stub type.

**Spec criteria at risk:**

- **SC-002 for `CellVertex`**: if R5's gate fails, the candidate's recommendation becomes O1,
  backed by the measurement, and nothing is adopted for it. FR-012 allows this.
- **PY-01's exception type**: R8 assumes that PyO3 0.30 passes a non-`TypeError` from argument
  extraction through unchanged. If it does not, the extractor is called in each method body.
  That is still one conversion per kind, so FR-007 holds. PY-01 is written first, and settles
  it.

## Project Structure

### Documentation (this feature)

```text
specs/009-bounded-types/
├── plan.md                      # This file
├── research.md                  # R0 (probes P1–P7) to R14
├── data-model.md                # OutOfRange, Level, Face, LevelMod, CellVertex, coverer settings, binding conversions, records
├── quickstart.md                # red → green → same results → proofs → evaluation
├── contracts/
│   ├── signature-audit.md       # T versus impl Into<T>: a verdict per signature (the maintainer's question)
│   ├── red-green-tests.md       # every test, its red and its green (the maintainer's request)
│   ├── binding-errors.md        # what Python and JavaScript callers see
│   └── migration-notes.md       # the old call and the new one (FR-013)
├── probe/
│   ├── README.md
│   └── into_inference.rs        # P3: literal inference through impl Into<Level>
├── checklists/requirements.md
└── tasks.md                     # /speckit-tasks (not created here)
```

### Source Code (repository root)

```text
BOUNDED_TYPES.md                         # NEW: the evaluation, red/green table, deviations, migration notes, follow-ups (R12)
CONTRIBUTING.md                          # one line: range-restricted parameters take a bounded type; TryFrom, never From
VERIFIED_TYPES.md                        # a "bounded types" subsection

core/src/s2/
├── bounded.rs                           # NEW: OutOfRange, LevelMod
├── bounded_tests.rs                     # NEW: TryFrom exactness, Display, branching_factor
├── coords.rs                            # Level: From<u8> removed, TryFrom<u8|i64>, checked_*, Deserialize; Face: TryFrom error
├── coords_tests.rs                      # NEW: CO-01, CO-04 (Level)
├── cell_id.rs                           # 13 signatures → Level; from_face, from_face_pos_level → Face; compile_fail docs
├── cell.rs                              # CellVertex (gated); vertex, vertex_raw; from_face, from_face_pos_level, average_area_for_level
├── cell_union.rs                        # denormalize(Level, LevelMod)
├── region_coverer.rs                    # level_mod: LevelMod; min_level, max_level setters take Level
├── region_coverer_tests.rs              # NEW: CO-02, CO-04 (RegionCoverer)
├── region_term_indexer.rs               # Options::level_mod: LevelMod
├── metric.rs, polygon.rs, testing.rs, builder/snap.rs   # signatures → Level
└── mod.rs                               # re-export OutOfRange, LevelMod, CellVertex
core/tests/, core/benches/, core/examples/, fuzz/       # call sites (compiler-driven)

python/src/bounded.rs                    # NEW: LevelArg, FaceArg, LevelModArg, CellVertexArg, check_level_order
python/src/{cells,coverer,term_index,metric,snap}.rs   # parameters → extractor types
python/tests/test_bounded_args.py        # NEW: PY-01 … PY-10

wasm/src/bounded.rs                      # NEW: level_arg, face_arg, level_mod_arg, cell_vertex_arg, check_level_order
wasm/src/{cell_id,cell,cell_union,region_coverer,term_indexer,snap}.rs   # f64 parameters, Result returns
wasm/tests/bounded_args.rs               # NEW: WA-01 … WA-06 through the JavaScript glue

verify/src/{domain,crash,cu_crash,cl_crash,…}.rs        # any_level/any_face/…; constructor proofs; call sites
verify/mutants/M-803…, M-806…, M-809…                   # regenerated and new
verify/{INVENTORY,COVERAGE,MUTANTS,PROPERTIES_BY_KIND}.md, cell_union/INVENTORY.md, PROPERTIES_BY_KIND.html
```

**Structure Decision**: the existing three-crate workspace. Each binding gets one new module,
`bounded.rs`, as the single conversion point of FR-007. The core gets one new module,
`s2::bounded`, for the shared error and the new modulus type. `Level` and `Face` stay in
`coords.rs` and `CellVertex` goes beside `CellEdge` in `cell.rs`, so that existing imports
keep working.

## Implementation phases (for /speckit-tasks)

The order is research R14. Each of phases 2–7 is one commit containing its red/green tests (red
run recorded first), the core change, both bindings, the documentation and any verify call
sites needed to compile. Each keeps `make test` green.

| Phase | Content | Exit check |
|-------|---------|------------|
| 1 | The survey: `BOUNDED_TYPES.md`'s classification of the 71 gap-1 operations and the audit's signatures (FR-001, FR-002), with upstream's behaviour per row | every row classified (SC-001) |
| 2 | B2: `Level`'s and the coverer's validating deserialisation (FR-009) | CO-01 to CO-04 red, then green |
| 3 | `OutOfRange`; the new `TryFrom` impls (additive) | the new-API unit tests |
| 4 | `Level`: `From<u8>` removed; 23 signatures; `checked_*`; Python `LevelArg`; wasm `level_arg`; counts from the compiler's error list | PY-01 to PY-04, PY-10, WA-01, WA-02, CD-01 and CD-05 red, then green; benchmarks before and after |
| 5 | `Face`: 4 signatures; `FaceArg` and `face_arg` | PY-05, WA-03 and CD-02 red, then green |
| 6 | `LevelMod`; `denormalize`; the coverer and term indexer; `check_level_order` in both bindings (FR-010) | PY-06 to PY-08, WA-04, WA-05 and CD-03 red, then green |
| 7 | `CellVertex`, after R5's gate is measured; `CellVertexArg` and `cell_vertex_arg` | PY-09, WA-06 and CD-04 red, then green, or the O1 record; benchmarks |
| 8 | Verification: constructor proofs; re-pointed complements; mutants; records (FR-016, FR-017) | `make verify`, `verify-values`, `verify-release` and the mutants, all green |
| 9 | The evaluation: measured comparison, red/green table, deviations, migration notes, ranked follow-ups (FR-003, FR-004, FR-011, FR-013, FR-018) | SC-007's walk-through (quickstart §5) |

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| Breaking public API changes across about 1,400 call sites, mostly in tests, in one minor release | The maintainer chose breaking changes for the four candidates (FR-013). The compiler-driven rewrite is what measures the ergonomic cost (FR-005) and enforces one conversion per binding (FR-007) | Additive signatures with deprecation, and validation in the bindings only, were offered at clarification and not chosen |
| One feature, several commits instead of several PRs | Each candidate changes the core and both bindings together, and they share `OutOfRange`. The evaluation compares them side by side | Separate PRs would ship the error type before its users, and spread one decision record over four reviews |
