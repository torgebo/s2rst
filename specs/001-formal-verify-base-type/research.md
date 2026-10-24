# Research: Formal Verification of `CellId`

Phase 0 of [plan.md](plan.md). Each entry gives the decision, the rationale, and the
alternatives considered. The spec's clarifications are fixed inputs:
- **2026-09-30:** target `CellId`; exhaustive automated model checking.
- **2026-10-01:** the neighbour operations stay in scope; crash proofs use cover-based
  non-vacuity; any domain restriction other than a loop bound means exclusion; the text
  parsers get robustness coverage; two parity fixes; SC-003 counts only properties of
  in-scope operations.
- **2026-10-01, planning decisions by the user:** one audited `unsafe` helper in `verify/`
  for the byte-level text domain (R6).
- **2026-10-01, second clarification round:**
  - input-size bounds count like loop bounds (FR-007, R6);
  - C++ differential tests for FR-015 and FR-016 (R16);
  - `# Panics` docs added in this feature (R17);
  - upstream findings become follow-up tasks (FR-018, R18);
  - the SC-005 reference runner has 4 vCPUs and 16 GB (R15).

There are no open NEEDS CLARIFICATION items. Version-specific facts below were checked
against the local Kani 0.68.0 install and the upstream checkout in `/home/t/sources/s2geometry`.
R6, R12 and R15 rest on a memory-capped probe run on 2026-10-01 (R6 describes the setup).

## R0. Feasibility spike (measured, 2026-09-30)

These are throwaway harnesses in the scratchpad, not committed. They were run with Kani 0.68.0
and CBMC 6.11.0 (CaDiCaL), `-j 4`, on an 8-core Linux workstation, against `s2rst` 0.4.0 by
path dependency.

| Harness | Domain | Result | Time |
|---------|--------|--------|------|
| `parent` is valid, `level − 1`, contains child | all valid, non-face | SUCCESSFUL | 0.4 s |
| children partition parent range | all valid, non-leaf | SUCCESSFUL | 0.6 s |
| `from_face_ij(to_face_ij_orientation(c)) == c` | all valid leaves (8 table lookups) | SUCCESSFUL | 27.8 s |
| `from_token(to_token(c)) == c` | all valid | killed | > 10 min, 3.8 GB |
| `from_token` crash-free, symbolic length ≤ 17, via `from_utf8` | ASCII | killed | > 8 min |
| debug-string round trip | all valid | killed | > 8 min |
| `from_token` crash-free, fixed length 16, via `from_utf8` | ASCII | killed | > 4 min |
| debug-string round trip | valid, fixed level 5 | killed | > 4 min, 3.5 GB |

Building `s2rst` under Kani took about 7 s.

**Conclusions**:

1. Kani builds `s2rst` from a separate crate through the public API with no changes to `core`.
2. Pure bit-manipulation proofs are essentially free, and even the lookup-table decoding is
   tractable over its whole domain.
3. Anything that pulls `core::fmt` (`format!`, `to_string`) or symbolic UTF-8 validation into
   the formula is not tractable with naive harnesses. See R6.

## R1. Verification tool and version

- **Decision**: Kani **0.68.0** with CBMC **6.11.0**. Kani is installed with
  `cargo install --locked kani-verifier --version 0.68.0` and `cargo kani setup`. The version is
  pinned in the Makefile check and in CI.
- **Rationale**:
  - This is the user's choice (clarification Q2 → A).
  - Kani is bit-precise, and it checks arithmetic overflow and panics automatically with no
    annotations.
  - It gives counterexample traces, and `--concrete-playback=print` turns them into unit tests,
    which meets FR-010 directly. In 0.68.0 this needs `-Z concrete-playback` (checked).
  - Per-harness `#[kani::solver]` is available (`cadical`, `kissat`, `minisat`, `bitwuzla`,
    `cvc5`, `z3`). The SMT ones need their binary on `PATH`. `--harness-timeout` needs
    `-Z unstable-options`; `run-verify.sh` uses it anyway to enforce the R15 time limit
    (`VERIFY_HARNESS_TIMEOUT`), because an over-budget harness otherwise runs unbounded
    (found during implementation: `p_nbr_01` ran past 20 minutes). In the R6 and R12 probe,
    CaDiCaL was never beaten, and CBMC's SMT2 path aborted on a text harness.
  - 0.68.0 is the version installed locally. It is newer than the 0.60–0.66 range cited in
    `verify/research/rust_formalmethods.md`, which itself advises confirming versions at planning
    time.
- **Alternatives considered**:
  - **Verus, Creusot, Prusti, Flux** (deductive, SMT-backed): out of scope per clarification
    Q2. Verus would also need the code in its own dialect.
  - **Aeneas/hax → Lean** (kernel-checked): out of scope, and a candidate for follow-up, to be
    decided in the evaluation.
  - **`cargo kani autoharness`**: it generates harnesses with unconstrained inputs, so it
    cannot express the `Valid`/`ValidOrEnd` domains or preconditions. It may be used as an
    optional discovery aid, but it is not a deliverable.
  - **More quickcheck properties**: sampling, which is exactly what the pilot is meant to go
    beyond.

## R2. Where the harnesses live

- **Decision**: A standalone workspace `verify/` (crate `s2rst-verify`, `publish = false`)
  with a `path = "../core"` dependency. It is modelled on `fuzz/`.
- **Rationale**:
  - The proofs need no changes in `core/` (FR-009, SC-006). The only `core/` changes in this
    feature are the FR-015/016/017 commits (R11, R13, R14).
  - The harness modules sit at the crate root (`src/crash.rs`, `src/hier.rs`, …). Kani's
    `--exact` matches the full module path. With a nested `src/harness/crash.rs`, the name is
    `harness::crash::c_parent`, and `--exact --harness crash::c_parent` fails with "Failed to
    match" (checked on 0.68.0).
  - The proofs exercise the same public API downstream users call.
  - The spike showed the public API is enough: `from_face_ij` and `Level` are re-exported, and
    `CellId.0` is public.
  - The existing `make test` and `cargo-deny` scopes are unaffected, as with `fuzz/`.
- **Alternatives considered**:
  - **In-crate `#[cfg(kani)] mod verification`**, Kani's documented default: it gives access
    to private helpers, but it needs edits to `core/src`, a `check-cfg` lint entry in
    `core/Cargo.toml`, and package excludes. No planned property needs private items, so it
    was rejected.
  - **A member of the root workspace**: `cargo build --workspace` and clippy would then pick it
    up, and the `kani` crate only exists under `cargo kani`. Rejected.

## R3. Reference specification (FR-006)

- **Decision**: `verify/src/spec.rs` holds a plain-Rust model of the bit layout documented in
  upstream `s2cell_id.h`, decomposing an id into `(face, level, path)`. The relations
  (`parent`, `child`, `ancestor`, `contains`, `range`, `successor`) are defined as arithmetic
  on the path and leaf index. They are **not** defined with the implementation's lsb tricks
  (see [data-model.md §1](data-model.md)). Every property asserts the implementation's result
  against the model.

  The Hilbert-curve tables (`kIJtoPos`, `kPosToIJ`, `kPosToOrientation`) are copied from
  upstream `s2coords_internal.h`, not from `s2rst::s2::coords`. The model applies them one
  level at a time, unlike the implementation's 4-level lookup table.

  The model is itself sanity-tested with ordinary `cargo test` in
  `verify/tests/spec_vs_cpp.rs` against the committed C++ vectors
  (`core/tests/data/cpp_cellid_vectors.csv`, sections `CELLID_CHILDREN`, `CELLID_RANGE`,
  `CELLID_CONTAINS` and `CELLID_FACE_IJ`, plus the three text sections added by R16).
  `CELLID_EDGE_NEIGHBORS` is not used. The model
  doesn't define cross-face neighbours (that needs upstream's floating-point face
  transforms), so the neighbour properties are relational (P-NBR, R12).
- **Rationale**: This gives two independent descriptions (Principle I). Checking the model
  against the C++ oracle guards against the main risk the research document names: "a bug in
  the spec is not caught by the kernel". The same holds for model checkers. The chain is C++ ⇔
  model (sampled, on stable) ⇔ implementation (proved for all inputs).
- **Alternatives considered**:
  - **Asserting one implementation call against another**, for example
    `c.parent().contains(c)`: this is only allowed for genuinely relational properties such as
    inverse pairs. As a general approach it proves self-consistency, not correctness.
  - **Porting C++ routines verbatim as the model**: a verbatim port would be a second copy of
    the same algorithm, not an independent description.

## R4. Input domains and preconditions

- **Decision**: Six domain generators, plus explicit precondition assumptions:
  - `any_u64` (the `Any` domain)
  - `any_valid`
  - `any_valid_at(level)`
  - `any_valid_or_end` (valid ∪ `end(l)` for every `l` ∪ `sentinel`)
  - `any_level` (`Level::try_new(kani::any())`, assumed `Some`)
  - `with_text::<L>(f)` (every byte string of exactly `L` bytes, as a `&str`; R6)

  Each inventory entry starts from the widest domain its argument types allow (FR-003). It
  narrows only by a documented precondition, and cites the source: rustdoc, `debug_assert!`,
  or an upstream `DCHECK`/`ASSUME`/doc comment. Operations used by upstream iteration on
  past-the-end values (range, containment, `next`/`prev`) get `ValidOrEnd`. The derived
  comparison traits are outside the FR-002 inventory.
- **Rationale**: This implements FR-003 and the spec's edge cases. It makes each assumption
  auditable, and it stops harnesses from assuming more than the documentation does (US1
  scenario 2).

  Kani builds with debug assertions and overflow checks on. A `debug_assert!` that is not
  covered by an assumed precondition therefore shows up as a failure. The proofs never rely on
  it firing.
- **Undocumented panics are Constitution II defects.** Constitution II requires every public
  function that can panic to document when (`# Panics`). Known cases, all outside the Rust
  docs:
  - `CellId(0).level()` underflows. Upstream states the precondition as
    `ABSL_ASSUME(id_ != 0)` with a comment.
  - `CellId::none().common_ancestor_level(CellId::none())` reaches `ilog2(0)`. Upstream C++
    assumes this input away with `ABSL_ASSUME(bits != 0)`, which is undefined behaviour here,
    and Go returns level 30 (U-002, R18).
  - `vertex_neighbors(30)` calls `Level::new(31)`. Upstream: `DCHECK_LT(level, this->level())`.
  - Every `impl Into<Level>` method converts raw `u8 > 30` inside its body, and that panics.
    It is documented on `From<u8> for Level`, not on the `CellId` methods.

  The proof assumes the upstream precondition. The inventory marks the row
  `undocumented_panic` (FR-003). `EVALUATION.md` lists each one as a **defect**, to be fixed
  by adding `# Panics` docs in a separate change under FR-010. This is not a pilot change,
  because FR-015/016 are the only behaviour fixes in scope.

## R5. Loops and unwinding

- **Decision**: Every harness with a loop sets `#[kani::unwind(bound + 1)]`, and unwinding
  assertions stay on. The loops and their bounds are:

  | Operation | Loop bound | Coverage |
  |-----------|------------|----------|
  | `to_face_ij_orientation` | fixed 8 iterations | full domain (spike: 27.8 s) |
  | `from_face_ij` | fixed 8 iterations | full domain |
  | `maximum_tile` | ≤ 30 iterations, one per level | full domain |
  | `from_debug_string`, `FromStr` | ≤ 30 iterations (one per digit) | **proven within bounds**: byte length ≤ 33, the `__b<L>` families (R6) |
  | `from_token` | ≤ 16 iterations (one per digit) | **proven within bounds**: byte length ≤ 17, the `__b<L>` families (R6) |
  | `to_debug_string` | ≤ 30 iterations | excluded, `tool-limit` (R6) |
  | `all_neighbors` | about `2^(level − self.level) + 2` iterations | excluded: crash-freedom intractable (R12) |
  | `spec.rs` `decompose` / Hilbert model | 31 / 30 iterations | full domain |

  `N` in `#[kani::unwind(N)]` is the largest loop bound in the harness plus 1. That counts
  loops in `spec.rs`, `domain.rs` and the standard library. For the text parsers, the part
  beyond the byte-length bound is documented as a gap. This is the only partial-domain
  result the spec allows (FR-007, SC-001). Any other domain restriction makes the operation
  a tool-limit exclusion.
- **Rationale**: This meets FR-007. With unwinding assertions on, a bound that is too small
  fails instead of passing silently, so every "full domain" claim is machine-checked.
- **Alternatives considered**: Kani loop contracts (`-Z loop-contracts`), which would give
  unbounded `all_neighbors` proofs. This is unstable, so it is deferred. It may be tried
  time-boxed, and the evaluation will report on it.

## R6. Text encodings (tokens, debug strings, `Display`, `FromStr`)

**Probe** (2026-10-01): throwaway harnesses in the scratchpad, run with Kani 0.68.0 and CBMC
6.11.0 against the pre-FR-015 code. Each harness ran alone in its own
`systemd-run --user --scope` with `MemoryMax` of 4–6 GB and swap off, under a 5–7 min timeout.
No more than three ran at once, on the 8-core workstation from R0. CaDiCaL was the solver
unless noted. "OOM" means the scope's memory cap killed CBMC.

| Harness | Domain | Result |
|---------|--------|--------|
| `from_token` crash | 4 / 16 / 17 symbolic `char`s pushed into a `String` | OOM (> 6 GB) after 19 / 106 / 70 s |
| `from_token` = C++ model | 8 / 16 symbolic chars | OOM after 34 / 118 s |
| `from_debug_string` crash | 4 chars | OOM after 25 s |
| `from_debug_string` crash | 32 / 33 chars | > 5 min (under 1 GB) |
| Building a 4-char `String` alone | — | 6 s, 0.24 GB |
| `u64::from_str_radix` alone | 4 chars | OOM (> 5 GB) after 23 s |
| Symbolic chars encoded into a fixed byte buffer | 4 chars | > 7 min |
| `from_token` crash, Kissat / MiniSat | 4 chars | OOM (the blow-up is in propositional reduction, before any solver runs) |
| `from_token` crash, Z3 | 4 chars | CBMC's SMT2 backend aborts with an internal error (`map::at`, status 6) |
| `from_utf8` + `from_token` | fixed `[u8; 4]` | 38 s, 1.1 GB |
| `from_utf8` alone | fixed `[u8; 8]` | > 7 min, 3.4 GB |
| **`from_token` crash, byte superset** | **every `[u8; L]`, L = 8 / 16 / 17** | **10.0 / 10.5 / 0.3 s, ≤ 0.4 GB** |
| **`from_token` = C++ model, byte superset** | **L = 16 / 17** | **12.4 / 0.3 s, ≤ 0.5 GB** |
| **`from_debug_string` crash, byte superset** | **L = 8 / 16 / 32 / 33** | **1.4 / 2.3 / 6.4 / 0.2 s, 0.26 GB** |
| **`from_debug_string` = C++ model, byte superset** | **L = 32 / 33** | **13.9 / 0.3 s, 0.33 GB** |
| **`FromStr` crash, byte superset** | **L = 32** | **7.1 s** |
| `to_token` crash | Any | > 5 min (CaDiCaL), > 7 min (Z3) |
| `to_debug_string` crash | Any, not valid (the `format!` branch) | OOM (> 6 GB) after 199 s |
| `to_debug_string` crash | Valid at level 5 / all Valid | > 5 min / > 5 min |
| `Display` crash | Valid at level 5 | > 5 min |

**Findings**:

1. The cost is in standard-library code that runs over a **symbolic length**
   (`from_str_radix`, UTF-8 validation, `String`/`Vec` growth) and in `core::fmt`. It is not
   in building the inputs. A fixed byte length per harness removes the symbolic length.
2. Every safe way to produce a `&str` of fixed length runs `std::str::from_utf8` on symbolic
   bytes, and that alone exceeds the budget from 8 bytes up.
3. The three parsers read only bytes. They use `len`, `as_bytes`, byte comparison,
   `from_str_radix` (which works on `as_bytes()`) and, after FR-015, `bytes().all(…)`. None
   of them decodes `char`s. On a `&str`, each parser's result is therefore a function of its
   bytes.

**Decision**:

- **Parsers in scope, over a byte-superset domain.** `domain::with_text::<L>(f)` draws a
  symbolic `[u8; L]` and passes `str::from_utf8_unchecked(&bytes)` to `f`. Every `&str` of
  `L` bytes is one of these arrays, so a proof over all of them covers every `&str` of
  length `L` (finding 3). Kani does not model UTF-8 validity, which is a safety invariant
  of `str`, not a validity invariant. The invalid members of the superset can therefore only
  add counterexamples, never hide one. Any counterexample is checked for UTF-8 validity
  before it is treated as a defect.
  - This is the only `unsafe` in `verify/`. The user approved it on 2026-10-01. `verify/`
    uses `#![deny(unsafe_code)]`, with a single `#[allow(unsafe_code, reason = "…")]` on
    `with_text`, compiled only under `cfg(kani)`. `core/` stays `forbid` (Complexity
    Tracking).
  - **Families**: `__b<L>` for every `L` in `0..=17` for `from_token` (its loop bound is 16
    bytes, and 17 shows the length check), and `0..=33` for `from_debug_string` and
    `FromStr` (30 digits after `f/`, and 33 shows the length check).
  - **Status: proven within stated bounds**, under an **input-size bound**. FR-007 accepts
    that since the 2026-10-01 clarification. The uncovered part is every text longer than
    17 or 33 bytes. Those inputs are rejected by the length check that precedes every loop.
    That is a code-inspection argument, not a proof. The FR-017 robustness tests and fuzz
    target exercise those inputs, as FR-007 requires. Three operations within bounds is 6%
    of the in-scope inventory (SC-001 allows 10%).
  - **One family per parser.** Kani checks every panic and overflow in every harness. So the
    model-agreement family (P-ENC-06, P-ENC-09, P-ENC-10) also discharges that parser's
    crash obligation (C-from_token, C-from_debug_string, C-FromStr): it uses exactly the
    crash domain and no extra assumption. The inventory names the family as the crash
    harness (data-model §3). This keeps the 86 text harnesses to about 7 CPU-minutes.
  - **Covers**: members with `L ≤ 16` (token) or `2 ≤ L ≤ 32` (debug string) cover
    "accepted" and "rejected". The longer members cover only "rejected", because an
    "accepted" cover there is unsatisfiable by design.
- **Formatters excluded as `tool-limit`**: `to_token`, `to_debug_string` and
  `impl Display`. Each exceeds the R15 budget, by time or by memory, through `core::fmt` or
  `String` growth. Their catalogue properties are `not-proven (tool-limit)`: P-ENC-05,
  P-ENC-07, P-ENC-08, P-ENC-11, and P-ENC-12 (the `Display` half of the former P-ENC-10).
  Under the 2026-10-01 SC-003 clarification, they stay in the catalogue but outside the 90%
  denominator. The evaluation reports them.
  - The formatters keep their existing coverage: unit and quickcheck tests, the C++
    differential vectors, and the FR-016 regression test.
  - The FR-016 fix is in `to_debug_string`, so `M-051` has no proof to kill. Its evidence
    is the regression test, which fails without the fix.
- **Rationale**: This keeps the three parsers, which handle untrusted input (Constitution
  II), under exhaustive proof up to their loop bounds. The formatters take only a `u64`,
  so the conventional tests already sample them well.
- **Alternatives considered**:
  - **Symbolic `char`s, or symbolic bytes validated by `from_utf8`**: OOM or over budget
    (table).
  - **Restricting text to ASCII**: a domain restriction other than a loop bound. FR-007
    forbids it.
  - **An SMT backend**: the CBMC SMT2 path aborted, and the blow-up happens before solving
    anyway.
  - **Rewriting the formatters in `core/` without `core::fmt`**: a fourth core change made
    only to suit the verifier. Not done in the pilot. The evaluation lists it as a follow-up
    option.
  - **Proving the formatters "within bounds" at a fixed level**: not a loop-bound
    restriction (FR-007), and still over budget at level 5.

## R7. Non-vacuity (FR-008)

- **Decision**: Two layers (clarification 2026-10-01, FR-008):
  1. **Assumption level, all harnesses.** Every harness contains `kani::cover!` statements.
     They show its assumptions are satisfiable and its interesting branches are reachable.
     **Kani 0.68.0 does not fail an unsatisfiable cover**: it prints `** 0 of 1 cover
     properties satisfied`, then `VERIFICATION:- SUCCESSFUL`, and exits 0 (checked). So
     `verify/run-verify.sh` reads each harness's `M of N cover properties satisfied` line and
     fails the run when M < N. For **crash-freedom proofs**, this layer is the whole
     non-vacuity evidence, plus one demo that an injected overflow is caught: mutant `M-001`
     (`advance` without its clamp; US1 scenario 3).
  2. **Property level, catalogue properties (P-\*) only.** There is a curated mutant set:
     `verify/mutants/M-NNN-*.patch`, each one defect with a header listing its target
     properties and expected failing harnesses (see
     [contracts/harness-conventions.md](contracts/harness-conventions.md)).
     `verify/run-mutants.sh` applies each patch in a throwaway `git worktree` and runs the
     expected harnesses with `--exact`. It records `killed`/`survived`/`error` in
     `verify/MUTANTS.md`. Every proven P-\* property needs at least one killing mutant (SC-004).

     The reverse of the FR-015 fix is reused as mutant `M-050`. The FR-016 fix sits in an
     excluded formatter, so its evidence is the regression test alone (R13).

  The campaign runs weekly and on demand. It is not per-PR, because each mutant costs one Kani
  build of about 7 s plus its harnesses.
- **Rationale**: FR-008 asks for a *property → defect* mapping with recorded outcomes, and
  curated patches give exactly that. Worktrees keep the caller's tree clean. The runner
  copies the caller's `verify/` into each worktree, so uncommitted harnesses and patches are
  tested too.

  Cover statements catch the other vacuity source, over-constrained `assume`s. That is the
  only way a crash proof can be vacuous, because Kani inserts the overflow and panic checks
  itself.
- **Alternatives considered**:
  - **`cargo-mutants`**: it is not installed. Its mutations are generic (for example replacing
    a function body with a default) and are not mapped to properties. Its oracle is
    `cargo test`, not Kani. It could be revisited for the follow-up.
  - **Mutating in place with `git stash`**: risky for the caller's working tree. Rejected.

## R8. CI integration (FR-012)

- **Decision**: A new workflow, `.github/workflows/verify.yml`, with permissions
  `contents: read`:
  - **Job `verify`:**
    - Runs on `pull_request` and `push` to `master`, path-filtered to these files:
      - `core/src/s2/cell_id.rs`;
      - `core/src/s2/coords.rs` (`Level`, `Face`, the Hilbert tables, and the face/uv
        transforms the neighbour wrap path uses);
      - `core/src/r3/**` (the vector maths of that path, R12);
      - `core/src/s2/mod.rs` and `core/Cargo.toml`;
      - `core/tests/data/cpp_cellid_vectors.csv`, which the model tests read;
      - `verify/**`, `Makefile`, and the workflow itself.
    - Also runs on a weekly `schedule` and on `workflow_dispatch`, and calls `make verify`.
  - **Job `mutants`:** runs only on `schedule` and `workflow_dispatch`, and calls
    `make verify-mutants`.
  - **Install and cache:**
    - Kani is installed with `cargo install --locked kani-verifier --version 0.68.0` and
      `cargo kani setup`, the same as the quickstart.
    - The cache must include the nightly toolchain that setup installs.
      `~/.kani/kani-0.68.0/toolchain` is a **symlink** into
      `~/.rustup/toolchains/nightly-2026-08-21-x86_64-unknown-linux-gnu` (checked).
    - So `actions/cache` saves `~/.cargo/bin/cargo-kani`, `~/.cargo/bin/kani`, `~/.kani` and
      `~/.rustup/toolchains/nightly-*`, keyed on `kani-0.68.0-<os>`.
    - After the restore, a sanity step runs `cargo kani --version`. It also checks that
      `~/.kani/kani-0.68.0/toolchain/bin/rustc` exists, and reruns setup if it doesn't.
- **Rationale**: The constitution requires CI to call the same Makefile targets. The path
  filter keeps unrelated PRs from paying the Kani cost (US3 scenario 2). The weekly run
  catches indirect drift, for example from a change to a shared helper outside the filter.
- **Alternatives considered**:
  - **`model-checking/kani-github-action`**: convenient, but it hides the install step behind
    a second mechanism. Acceptable as a later simplification if pinned to the same version.
  - **Adding verification to `make test`**: forbidden by FR-012, since it needs an extra
    toolchain.

## R9. Counterexamples to regression tests (FR-010)

- **Decision**: When a harness finds a genuine defect:
  1. Run `cargo kani -Z concrete-playback --concrete-playback=print --harness <h>` to get a
     reproducing `#[test]`.
  2. Record the defect as `D-NNN` in `verify/EVALUATION.md`, and open the separate fix
     change. That change ports the test to `core/src/s2/cell_id_tests.rs` and checks upstream
     parity.
  3. On the pilot branch, the harness stays as written, so it keeps failing. Its row is
     marked `not-proven (defect D-NNN)`.
  4. The harness is listed in `verify/known-defects.txt` as `<harness> D-NNN`.
     `run-verify.sh` reports a listed harness that fails as `KNOWN-DEFECT D-NNN`, not as a
     failure, so `make verify` and CI stay green. A listed harness that now *succeeds* is
     an error ("stale known-defect entry"), so the list can't outlive the fix.

  The exceptions are the two defects found before implementation, which are fixed in this
  feature (FR-015 → R11, FR-016 → R13). They never enter `known-defects.txt`.
- **Rationale**: This keeps verification work and behaviour changes separate (FR-010,
  Constitution I and III). The known-defects list keeps a red proof visible without
  blocking the gate. Its stale-entry check forces cleanup once the fix lands.
- **Alternatives considered**:
  - **Comment out or `#[cfg]`-gate the failing harness**: hides it. Rejected.
  - **Weaken the property until it passes**: proves the wrong thing. Rejected.

## R10. Reporting and traceability (FR-011)

- **Decision**: Harness names encode the property ID (`hier::p_hier_03`, `crash::c_parent`),
  and each harness doc comment repeats the catalogue or inventory statement.
  - With `-j`, Kani's terse output doesn't label results with harness names. It prints
    `Thread N: Checking harness <h>...`, and later a result block that starts with
    `Thread N:` (checked).
  - So `run-verify.sh` maps each block back to its harness. It prints one line per harness
    (`<h>  SUCCESSFUL|FAILED|OOM|KNOWN-DEFECT  covers M/N  <t>s`), one roll-up line per split
    family (`<family>__*  n/n SUCCESSFUL`), and a summary. That gives per-operation and
    per-property results. `OOM` comes from the memory guard (R15).
  - **Counterexamples (FR-007, US3).** A failed harness's terse output names only the
    failing check and its location, not the inputs. Checked on 0.68.0: `Failed Checks:
    "…"`, then the file and line, and no values. So for every `FAILED` harness that is not a
    known defect, `run-verify.sh` re-runs it under the memory guard with
    `-Z concrete-playback --concrete-playback=print --exact --harness <h>`. It prints the
    generated `#[test]`, whose `concrete_vals` carry the inputs with a decoded comment, for
    example `// 7ul`. Checked: the test appears under
    "Concrete playback unit test for `<h>`". CI logs therefore show the property and a
    concrete counterexample.
  - JSON export (`--export-json`) is unstable, so it is not relied on.
- **Rationale**: This is the simplest mechanism that meets FR-011 and US3 ("names the
  property").

## R11. `from_token` parity fix (FR-015)

- **Finding** (2026-10-01, plan review): `CellId::from_token` parses with
  `u64::from_str_radix(token, 16)`, and that function accepts a leading `+`. So
  `from_token("+1")` returns `CellId(1 << 56)`, which is the valid level-2 cell `0/00`.
  - C++ `S2CellId::FromToken` accepts only `0-9`, `a-f` and `A-F`, character by character, and
    returns `None()` for anything else.
  - Go `CellIDFromToken` uses `strconv.ParseUint`, which does not allow a sign.
  - The `+` is the only non-hex character that `from_str_radix` accepts. A lone `+` is already
    rejected.
  - No existing test, binding or caller depends on the `+` being accepted.
- **Decision**: Return `CellId::none()` unless every byte of the token is an ASCII hex digit
  (`token.bytes().all(|b| b.is_ascii_hexdigit())`). Check this before calling
  `from_str_radix`. With that check, `from_str_radix` cannot fail on at most 16 digits, so the
  result matches C++ for every input. The existing length, empty and `"X"` checks stay as they
  are.
- **Order of work**:
  1. Write a regression test in a new sibling `core/src/s2/cell_id_tests.rs` (Constitution
     III). Wire it in with `#[cfg(test)] #[path = "cell_id_tests.rs"] mod cell_id_tests;`,
     following the convention of `convex_hull_query.rs` and the other modules. The test
     asserts that `from_token("+1")` and a 16-character token such as `"+fffffffffffffff"`
     both give `none`. Confirm that it fails on the current code.
  2. Add the fix. Commit it together with the test as one change, kept apart from the
     verification work, so every commit passes `make test`. The test now passes.
  3. Record the defect in `verify/EVALUATION.md`, with the C++ and Go references.

  The Python `CellId.from_token` and the wasm `CellId.fromToken` and `CellUnion.fromTokens`
  call straight through, so they get the fix with no binding change (Principle IV). The token properties (P-ENC, R6) are
  stated against the upstream rule, so they would have caught this defect too. The reverse
  of the fix becomes mutant `M-050`, the non-vacuity evidence for the token parsing property.
- **Alternatives considered**:
  - **Port the C++ per-character loop and drop `from_str_radix`**: equivalent, but a larger
    diff. R6 found `from_str_radix` cheap at a fixed byte length (≤ 12.4 s at 16 bytes), so
    the smaller diff stands.
  - **A separate PR**: this was the default under FR-010. The user set it aside on
    2026-10-01.

## R12. Neighbour operations and the floating-point wrap path

- **Facts**: `edge_neighbors`, `vertex_neighbors` and `all_neighbors` call the private
  `from_face_ij_wrap` whenever a neighbour lies across a cube-face edge. That function uses
  `f64` arithmetic, as upstream `FromFaceIJWrap` does:
  - clamping and scaling;
  - `face_uv_to_xyz`;
  - `xyz_to_face_uv`, with two divisions and a `debug_assert!` on a dot product;
  - `st_to_ij`, with a saturating cast.

  CBMC reasons about IEEE-754 bit for bit, so these proofs are exact, not approximate.
- **Probe** (2026-10-01, set up as in R6; 4 GB cap unless noted):

  | Harness | Domain | Result | Time | Memory |
  |---------|--------|--------|------|--------|
  | `edge_neighbors` crash | all Valid | SUCCESSFUL; covers "wrap" and "same face" satisfied | 411 s | 2.5 GB |
  | `edge_neighbors` crash | Valid on face 0 / face 3 | SUCCESSFUL | 209 / 155 s | 2.6 GB |
  | `edge_neighbors` crash | leaf cells on one face edge (`j = 0`), all faces | SUCCESSFUL (CaDiCaL) | 377 s | 2.6 GB |
  | the same | the same, with Kissat / MiniSat / Z3 | timeout | > 420 s | — |
  | `vertex_neighbors` crash | all Valid × `l < level` | SUCCESSFUL | 638 s | 1.9 GB |
  | `vertex_neighbors` crash | Valid on face 0 × `l < level` | SUCCESSFUL; covers "3 cells" and "4 cells" satisfied | 574 s | 1.9 GB |
  | `all_neighbors` crash | Valid, gap 0 / 1 / 2, all faces or face 0 | OOM | 96–147 s | > 4 GB |
  | `all_neighbors` crash | Valid, gap 0, 10 GB cap | OOM during SSA conversion (1.04 M steps, 29 k VCCs left after simplification) | 105 s | > 9.5 GB |

  A concrete debug-build sweep backs this up. It is a sanity check, not a proof. The sweep
  covers 125,736 neighbour computations on cells at every level around face edges and
  corners. It found no panic, no invalid or wrong-level neighbour, and no asymmetric edge
  neighbour.

  Two Kani notes:
  - `SATISFIABLE` lines in CBMC's log are Kani's reachability checks, not failures.
  - `--cbmc-args --stop-on-fail` crashes Kani 0.68.0's output parser, so it is not used.
- **Decision**:
  - **`edge_neighbors` in scope.** P-NBR-01 (four valid, distinct neighbours at the same
    level) runs as one harness over all Valid cells. It uses exactly the crash domain, so it
    also discharges C-edge_neighbors (as in R6). Expected time: about 7 min on the
    workstation, within the R15 hard limit. If it is over the limit on CI, the fallback is
    the `__f<k>` per-face family (≤ 3.5 min each, about 18 CPU-minutes in total).
  - **P-NBR-02 (symmetry)** calls `edge_neighbors` twice. It is measured during
    implementation under the R15 budget. If no split fits, it is
    `not-proven (tool-limit)`, and the relational check stays in the existing tests.
  - **`vertex_neighbors` in scope, at risk.** P-NBR-03 discharges C-vertex_neighbors.
    Unsplit, it takes 10.6 min, above the 8 min workstation proxy for the R15 hard limit,
    so it must be split.
    Splitting by face did not help (9.6 min for face 0 alone). Implementation tries a
    four-way split on the `(i, j)` bits that pick the vertex (`i & half_size`,
    `j & half_size`). Together the four cases cover the domain, so this is a case split,
    not a restriction. If neither form fits R15 on CI, `vertex_neighbors` is excluded as
    `tool-limit: over the R15 budget`. Its crash-freedom is provable, so the FR-002
    floating-point clause doesn't apply, but SC-005 makes the budget binding. In that case
    in-scope falls to
    47/64 (73.4%) and SC-002 is missed. The miss is recorded as a pilot finding, and the
    spec is not changed without the user.
  - **`all_neighbors` excluded** as `float-geometry` under the FR-002 neighbour clause: its
    crash-freedom is intractable even at gap 0 with 10 GB. The cost comes from the loop
    pushing into a `Vec` that grows from empty, with up to four wrap calls per iteration.
    P-NBR-04 becomes `not-proven (excluded)` and leaves the SC-003 denominator. The
    evaluation records the measurements. `G` is no longer needed.
  - **Scope arithmetic**: 64 inventory entries, minus 12 floating-point, 3 formatters (R6)
    and `all_neighbors`, leaves **48 in scope (75.0%)**. That is exactly SC-002's floor.
- **Implementation outcome (2026-10-01; user decision: weekly slow tier)**:
  - `p_nbr_01` over the whole domain ran past 20 min; per face (`__f0`) it timed out at
    8 min. Only the validity-and-level conjunct fits (248 s on face 0). The distinctness
    and `≠ c` conjuncts are the expensive part.
  - `vertex_neighbors`: crash-only quadrant 0 timed out at 8 min; the property quadrants
    first ran out of memory (loops over the `Vec`), then timed out in a fixed-index form.
  - So the per-change tier first kept only `crash::c_edge_neighbors__f0…f5` (crash-only,
    221 s for face 0). With it, the clean per-change run took 27 min on a 4-vCPU machine
    (55 CPU-minutes, 22 of them this family), over SC-005, so the family moved to the slow
    tier as well (T046). `crash::c_vertex_neighbors` (unsplit) and the whole `nbr` module
    (P-NBR-01, -02, -03) move to a **weekly slow tier**: Cargo feature `slow`,
    `make verify-slow`, 30 min per harness, run by the weekly CI job (spec FR-012, SC-005
    as amended). Harnesses still over 30 min there become `tool-limit` results.
  - In the slow tier, `crash::c_vertex_neighbors` (unsplit) is proven in 776 s. P-NBR-02
    (symmetry, two nested `edge_neighbors` calls) runs out of memory at the 4 GiB ceiling
    within 35 s, per face and per face × neighbour index, so it is
    `not-proven (tool-limit)` and has no harness (T038).
  - P-NBR-01 per face (all conjuncts) took 21–27 min on two faces and timed out at 30 on
    four, so it is split into per-neighbour (`p_nbr_01__f<K>_k<N>`, at most 11 min) and
    per-pair (`p_nbr_01__f<K>_p<NM>`, at most 15 min) cases: 60 harnesses. P-NBR-03 per
    quadrant timed out at 30 min; per face × quadrant (`p_nbr_03__f<K>_q<Q>`) takes 15–29
    min with four jobs: 24 harnesses. The complete slow tier (91 harnesses, all
    `SUCCESSFUL`) took 5.4 h at 4 jobs, 20.6 CPU-hours, so CI runs it as a matrix (one leg
    per face, plus `vertex`) with `VERIFY_JOBS=2`.
- **Model**: the reference model doesn't define cross-face neighbours, because that needs
  upstream's floating-point face transforms. So the P-NBR properties are relational, and
  `CELLID_EDGE_NEIGHBORS` stays out of `spec_vs_cpp.rs` (R3).
- **Alternatives considered**:
  - **Excluding all three operations up front**: the 2026-10-01 clarification rejected
    that unless crash-freedom proves intractable, which is now shown only for
    `all_neighbors`.
  - **Stubbing `from_face_ij_wrap`**: forbidden, since it would replace code under test.
  - **A function contract on it**: needs annotations in `core`, which clarification Q2
    rules out.
  - **Other solvers**: none beat CaDiCaL on the wrap path.

## R13. `Invalid:` debug-string parity fix (FR-016)

- **Finding** (2026-10-01, `/speckit-analyze`): for an id that is not a valid cell,
  `CellId::to_debug_string` returns `format!("Invalid: {:x}", self.0)`, for example
  `Invalid: 2`. (An earlier draft used `5` as the example, but `5 = 0b101` is the valid leaf
  `0/0…02`: its lowest set bit is the leaf marker. Caught by the model's sanity test.)
  - Upstream C++ `S2CellId::ToString` (`s2cell_id.cc`, checked in the local upstream checkout)
    returns `StrCat("Invalid: ", absl::Hex(id(), absl::kZeroPad16))`, i.e.
    `Invalid: 0000000000000002`.
  - `Display` writes `to_debug_string()`, so it differs the same way.
  - Go's `String()` uses an unpadded signed hex. C++ is the canonical reference (Principle I).
  - No core, Python or wasm test depends on the current format (checked).
- **Decision**: Change the format string to `"Invalid: {:016x}"` (lowercase, like
  `absl::Hex`). Valid ids are untouched.
- **Order of work**: the same as R11, as its own commit:
  1. Add a regression test to `core/src/s2/cell_id_tests.rs`. It asserts:
     - `CellId(2).to_debug_string() == "Invalid: 0000000000000002"`;
     - `CellId::none()` gives `"Invalid: 0000000000000000"`;
     - `CellId::sentinel()` gives `"Invalid: ffffffffffffffff"`;
     - `CellId(2).to_string()` is the same as the debug string.

     Confirm the test fails on the current code.
  2. Fix and commit.
  3. Record the defect in `verify/EVALUATION.md`.

  P-ENC-11 states the fixed behaviour as a catalogue property: for every id that is not a
  valid cell, `to_debug_string()` equals the C++ `ToString` text, as modelled in
  `spec::debug_string`. R6 found the `format!` path intractable (OOM above 6 GB), so
  `to_debug_string` is a tool-limit exclusion, and P-ENC-11 is `not-proven (tool-limit)`
  with its measurement. No proof can kill the reverse of the fix, so there is no `M-051`.
  The regression test, which fails without the fix, is the evidence for FR-016 (SC-009).
- **Alternatives considered**: match Go (rejected, because C++ is canonical); fix it in a
  separate PR (set aside by the user on 2026-10-01).

## R14. Untrusted-input coverage for the text parsers (FR-017, Constitution II)

- **Decision**: Two additions, each independent of whether the text proofs succeed (R6):
  1. **Bounded robustness tests.** Add a new section, "text parsers", to
     `core/tests/decoder_robustness.rs`. It follows the file's existing style (a `ChaCha8Rng`
     seed and labelled cases). The test functions are named `text_parsers_*`, so
     `cargo test --test decoder_robustness text_parsers` selects them (quickstart §7).
     - **Deterministic cases** for `CellId::from_token`, `CellId::from_debug_string` and
       `str::parse::<CellId>`:
       - empty input; `"X"`;
       - 16 and 17 hex digits;
       - `"+1"`, `"-1"`, `" 1"`;
       - non-hex ASCII; multi-byte UTF-8 (`"é"`, `"€1"`);
       - `"0/"`, `"6/0"`, `"0/4"`, `"0/"` plus 31 digits, `"0|0"`, and a NUL byte.
     - **Random cases:** a few thousand short random strings, mixing ASCII hex, `/`, digits
       and random scalar values.
     - **Contract:** never panic.
       - Rejected input gives `none` / `None` / `Err(())`.
       - Every accepted token round-trips through `to_token`.
       - Every accepted debug string equals the input after `to_debug_string`.
  2. **Fuzz target** `fuzz/fuzz_targets/parse_cell_id_text.rs`.
     - It takes `&[u8]`, returns early unless `std::str::from_utf8` succeeds, then calls all
       three parsers.
     - It asserts the same round-trip contract and that `s.parse::<CellId>()` equals
       `from_debug_string(s).ok_or(())`.
     - Registration:
       - a `[[bin]]` entry in `fuzz/Cargo.toml`, following the `decode_cap` pattern;
       - a matrix entry in `.github/workflows/fuzz.yml`, as the constitution requires;
       - seeds `fuzz/seeds/parse_cell_id_text/seed-00…`: `"X"`, `"89c25"`, `"3/0123"`,
         `"+1"`, 16 `f`s, `"Invalid: 0000000000000002"`.
     - Commit it together with the robustness tests, as the FR-017 change.
- **Return types**: `from_token` returns `CellId` and `from_debug_string` returns `Option`,
  against II's `Result` rule. That exception is recorded in the plan's Complexity Tracking.
  The reason is upstream parity plus FR-009. `FromStr` already returns `Result<CellId, ()>`.
- **Rationale**: II's bounded-coverage and fuzz requirements are met directly. If the text
  proofs are excluded under FR-007 (R6), the parsers are still covered.
- **Alternatives considered**: let the proofs stand in for the tests (rejected, because the
  proofs may be excluded); treat the text parsers as out of II's scope (rejected, because the
  plan itself calls them untrusted input).

## R15. Resource budget and memory guard (SC-005)

- **Context**: CBMC's memory use grows fast and is hard to predict. In the R6 and R12 probe:
  - a 4-character text harness passed 6 GB in under 20 s;
  - `all_neighbors` passed 9.5 GB in under 2 minutes;
  - an earlier probe in this planning work, run without a cap, ended without any results.

  An uncapped run can exhaust the machine and take down unrelated processes, including the
  maintainer's session. The CI runner (`ubuntu-latest`, 4 vCPU, 16 GB) runs four harnesses
  at once.
- **Decision: budget per harness and per run**

  | Limit | Value | What happens above it |
  |-------|-------|-----------------------|
  | Memory per harness | 4 GB (`VERIFY_MEM_PER_HARNESS`). Four of them fit the 16 GB reference runner | `OOM`: split the harness, otherwise exclude as `tool-limit` |
  | Parallelism | `VERIFY_JOBS`, default `min(nproc, ⌊MemAvailable / (7/8 · VERIFY_MEM_PER_HARNESS)⌋)` (3.5 GiB per job at the default ceiling), at least 1. It is 4 on the reference runner (MemAvailable ≈ 15 GiB). The cgroup backstop is capped at `MemAvailable − 1 GiB`, so a worst case ends the run, not the machine. Corrected during implementation: the earlier `⌊(MemTotal − 2 GiB) / ceiling⌋` gave 3 on a "16 GB" runner (MemTotal ≈ 15.6 GiB) and 7 jobs with a 28 GiB worst case on a 30 GiB workstation | Smaller runners run fewer jobs (SC-005 clarification). The evaluation then reports both times |
  | Time per harness, target | 60 s | — |
  | Time per harness, soft limit | 5 min | Try a split. If it stays above, list it under "Measurements" in `EVALUATION.md`, with the reason |
  | Time per harness, hard limit | 12 min on CI. Until CI exists (US3), use 8 min on the workstation as the proxy, because CI cores may be slower | Split, otherwise `tool-limit`. Once US3 measures CI, re-check every harness against the 12 min limit |
  | Whole run | 45 CPU-minutes | `-j 4` then finishes in under 15 min (SC-005), with room for uneven packing. Past it, the most expensive families go first, by the same rules |
  | Weekly slow tier | 30 min per harness (`make verify-slow`, feature `slow`) | Harnesses over the per-change budget (R12). Over 30 min: `tool-limit` |

- **Projection** (workstation numbers, from R0, R6 and R12. CI times are measured during
  implementation):

  | Group | Harnesses | CPU-minutes |
  |-------|-----------|-------------|
  | Integer crash and property harnesses | about 80 | ≤ 5 |
  | Text families (R6) | 86 | about 7 |
  | P-NBR-01, `edge_neighbors`, unsplit (R12) | 1 | about 7 |
  | P-NBR-03, `vertex_neighbors` (R12) | 1–4 | about 11 |
  | P-NBR-02, symmetry (R12) | 1 or more | not measured, about 2 × P-NBR-01 |
  | Kani overhead per harness | about 170 | about 4 |

  Without P-NBR-02 the total is about 34 CPU-minutes. P-NBR-02 decides whether the
  neighbour work fits, and its fallback is `not-proven (tool-limit)`.
- **Decision: memory guard in `run-verify.sh`** (verified on Kani 0.68.0 and CBMC 6.11.0,
  2026-10-01):
  1. Build first with `cargo kani --only-codegen` and no limit, so the compiler is never
     capped.
  2. Run the proofs under `ulimit -v $VERIFY_MEM_PER_HARNESS`. The limit applies to each
     process, so `-j N` uses at most N times the ceiling.
     - **Checked:** with `ulimit -v` at 2 GB, a harness that otherwise passes 6 GB stopped
       after 20 s with `Solver ran out of memory during propositional reduction.` Kani
       printed `VERIFICATION:- FAILED` and exited 1, and the build cache was reused.
     - `run-verify.sh` reports such a harness as `OOM`.
  3. Where `systemd-run --user` works (developer machines), also wrap the run in
     `systemd-run --user --scope -p MemoryMax=<N × ceiling + 2G> -p MemorySwapMax=0` as a
     backstop.
     - **Checked:** a scope that reaches its cap is OOM-killed inside the scope only.
       systemd records `Result: oom-kill`, and the rest of the session is untouched.
     - Under systemd's default `OOMPolicy=stop`, this ends the whole run. The script then
       reports "memory backstop hit". It acts only if step 2 somehow failed.
  4. If `systemd-run --user` is unavailable, as is likely on CI runners, step 2 alone
     applies. Off Linux, `ulimit -v` is not reliably enforced. The script then warns that no
     guard is active, and the quickstart says to lower `VERIFY_JOBS`.
- **Rationale**: This answers the maintainer's concern that a verification run can bring
  the machine down. It uses mechanisms that were tested here, and it turns memory
  exhaustion into an ordinary, reported harness result.
- **Alternatives considered**:
  - **A single cgroup as the only guard**: rejected. Under the default `OOMPolicy=stop`,
    one harness's OOM ends the whole run, and the guard would need a systemd user manager.
  - **Kani's `--harness-timeout` as the memory guard**: rejected. It is unstable and limits
    time only. It is used, but for the separate time limit (R1).
  - **No guard**: rejected, given how fast memory ran away in the probe.

## R16. Differential tests against C++ for the parity fixes (FR-015, FR-016; Principle I)

- **Facts** (checked 2026-10-01):
  - `tools/refgen-cpp/generate_cellid.cc` writes `core/tests/data/cpp_cellid_vectors.csv`.
    Its sections are `CELLID_EDGE_NEIGHBORS`, `RANGE`, `CONTAINS`, `CHILDREN` and `FACE_IJ`.
    `core/tests/cpp_cellid_diff.rs` diffs the Rust code against it.
  - `CELLID_TOKENS` in `cpp_test_vectors.csv` covers `ToToken`/`FromToken` of valid cells
    only. No committed vector has an invalid token or the `ToString` of an invalid id.
  - The generator draws from one seeded RNG, section after section. So sections appended at
    the end leave every existing line byte-identical.
  - The local upstream checkout (`/home/t/sources/s2geometry`, `37a4501`) is the pinned
    oracle `8d5c2a8` plus one local example commit, and its `build/` exists. Only
    `cmake --install build` into `/tmp/s2-install` must be redone, because `/tmp` was cleared.
  - Upstream `FromToken` returns `None()` for more than 16 bytes or any byte outside
    `0-9a-fA-F`. It returns `S2CellId(0)` for the empty token.
  - Upstream `ToString` checks `is_valid()` before calling `level()`.
- **Decision**: append new sections to `generate_cellid.cc`, regenerate the CSV locally, and
  add differential tests to `core/tests/cpp_cellid_diff.rs`. CI never needs C++.
  - **`CELLID_FROM_TOKEN`**: `token_hex,id`. The token's bytes are hex-encoded, so commas,
    spaces, `+`, NUL and multi-byte UTF-8 survive the CSV. The empty token is an empty
    field. Inputs are deterministic and always valid UTF-8, because the Rust side takes
    `&str`:
    - `""`, `"X"`, `"x"`, `"0"`, `"1"`, `"+1"`, `"-1"`, `" 1"`, `"1 "`, `"+"`, `"+0"`,
      `"+fffffffffffffff"`, `"0x1"`, `"89c25"`, `"89C25"`, 16 `f`s, 17 `f`s, `"é"`, `"€1"`;
    - `ToToken` of 500 `RandCell()`s;
    - 1,000 random strings of 0–17 characters, drawn from hex digits, `+-xX /`, NUL and a
      few multi-byte characters.
  - **`CELLID_FROM_DEBUG_STRING`**: `string_hex,id` (`0` for `None`). Inputs:
    - `ToString` of 500 random cells, valid at every level;
    - edge cases: `"0/"`, `"5/"`, `"6/0"`, `"0/4"`, `"0|0"`, 31 digits, `""`, `"/"`, `"0"`;
    - 1,000 random strings over `0-9/`.

    This isn't required by FR-015/016. It costs one extra section and gives the third text
    parser a C++ oracle too.
  - **`CELLID_TO_STRING`**: `id,string`, the string in plain text (it never contains a
    comma). Inputs:
    - `0`, `2`, `5` (a valid leaf), `u64::MAX`, `1<<61`, `1<<62`, `1<<63`, `End(l)` for every level;
    - 1,000 random `u64`s, mostly invalid;
    - 500 random valid cells.
  - **Commits**, so that every commit passes `make test`:
    - the **FR-015 commit** adds `CELLID_FROM_TOKEN` and `CELLID_FROM_DEBUG_STRING`, the
      regenerated CSV, their diff tests, the unit regression test and the fix;
    - the **FR-016 commit** appends `CELLID_TO_STRING`, regenerates the CSV, and adds its
      diff test, unit test and fix.
  - Each commit's `git diff` of the CSV must show only appended lines. Each new diff test
    must fail on the code before its fix (Constitution III).
  - **Model sanity**: `verify/tests/spec_vs_cpp.rs` checks `spec::parse_token`,
    `spec::parse_debug_string` and `spec::debug_string` against the same three sections
    (R3). That extends the C++ ⇔ model chain to text.
  - **README**: add the three sections to the table in `tools/refgen-cpp/README.md`.
- **Rationale**: Principle I requires differential tests wherever an oracle exists, and the
  user chose this route on 2026-10-01. Hex-encoding the inputs keeps the CSV unambiguous.
- **Alternatives considered**:
  - **Hand-written expected values only**: violates Principle I.
  - **A recorded exception**: the user rejected it.
  - **The Go oracle**: Go prints invalid ids unpadded, so it can't serve FR-016.

## R17. `# Panics` documentation (FR-003, SC-011; Constitution II)

- **Facts**: `core/src/s2/cell_id.rs` has no `# Panics` sections today. Panics known by
  inspection:
  - the 13 public functions that take `impl Into<Level>` convert a raw `u8 > 30` and panic
    (`from_face_pos_level`, `child_position`, `parent_at_level`, `child_begin_at_level`,
    `child_end_at_level`, `begin`, `end`, `size_st`, `vertex_neighbors`, `all_neighbors`,
    `lsb_for_level`, `size_ij`, `ij_level_to_bound_uv`);
  - `level()` on ids with 62 or more trailing zeros, including `0`;
  - `common_ancestor_level` on `none` with `none`;
  - `vertex_neighbors` at level 30 (R4).

  The proofs will confirm these and may find more.
- **Decision**: one documentation-only commit (clarification 2026-10-01) that adds a
  `# Panics` section to every public `CellId` operation that the proofs show can panic.
  - It covers, by inspection, all 13 `impl Into<Level>` functions, including the excluded
    `size_st`, `all_neighbors` and `ij_level_to_bound_uv`.
  - Each section states the condition in the caller's terms, for example "Panics if
    `level` converts from a raw integer greater than 30" or "Panics if this id is 0 or has
    no level marker".
  - If a panic also differs from upstream behaviour, the section says so (Principle I), and
    the difference becomes a `D-NNN` defect (FR-010) and an FR-018 follow-up (R18).
  - The commit lands after the US1 crash proofs, so that the set is final.
  - Checks: `make doc` and `make test` pass, and the rustdoc wording is the only diff.
- **Rationale**: closes the Constitution II "MUST document when it panics" gap at no
  behaviour cost. FR-009 is respected, because no signature or behaviour changes.
- **Alternatives considered**:
  - **Record only, or open follow-up issues**: rejected by the user.
  - **Removing the panics**: that is a behaviour change, out of scope (FR-009, FR-010).

## R18. Upstream findings (FR-018, SC-012)

- **Decision**: when proof work shows undefined or wrong behaviour in the upstream
  reference, record a follow-up **before** marking the affected proof complete. The
  reference is C++ (canonical), Go, Java, or the generated vectors.
  - Add a `U-NNN` entry to the "Follow-ups" section of `tasks.md`, as a new unchecked task.
    It names the upstream location (file:line), the inputs, the observed behaviour, and the
    behaviour the Rust port documents.
  - Mirror it in `verify/EVALUATION.md` under "Upstream findings".
  - The affected harness's doc comment cites the `U-NNN` (data-model §9).
  - Upstream code is never changed.
- **Known now, the first entry (U-001)**: C++ `S2CellId::level()` (`s2cell_id.h:599–602`):
  - It is `ABSL_ASSUME(id_ != 0); return kMaxLevel - (countr_zero(id_) >> 1);`.
  - So `level()` of `0` is undefined behaviour, and ids with 62 or more trailing zeros
    (`1<<62`, `1<<63`) return −1.
  - Rust computes the same expression in `u8`. It underflows: a panic in debug builds, a
    wrapped value that `Level::new` rejects in release.
  - Expected handling:
    - U-001 records the upstream behaviour;
    - R17 documents the Rust panic;
    - the C++ −1 vs Rust panic difference is decided under FR-010 (`D-NNN`).
  - The crash harness `crash::c_level` assumes the documented domain, which excludes these
    ids, and cites U-001.
- **Known now, the second entry (U-002)**: C++ `S2CellId::GetCommonAncestorLevel`
  (`s2cell_id.cc:193–207`):
  - It computes `bits = max(id ^ other, max(lsb(), other.lsb()))`, then
    `ABSL_ASSUME(bits != 0); // Because lsb() is non-zero.`
  - `bits` is 0 only when both ids are 0. So `none.GetCommonAncestorLevel(none)` is
    undefined behaviour in C++.
  - Go's `CommonAncestorLevel` (`geo/s2/cellid.go:741–747`) returns `(30, true)` for the same
    input, because `bits.Len64(0) − 1 = −1` passes its `> 60` check. That is a well-defined
    but wrong answer.
  - Rust's `common_ancestor_level` calls `0u64.ilog2()`, which panics.
  - Expected handling:
    - U-002 records both upstream behaviours;
    - R17 documents the Rust panic;
    - whether Rust should return `None` instead is decided under FR-010 (`D-NNN`).
  - `crash::c_common_ancestor_level` assumes a domain that excludes only the pair
    (`none`, `none`), and cites U-002. The P-ANC harnesses work over valid cells, which
    never include `none`, and cite U-002 because the upstream comment is the source of that
    domain.
- **Search**: these are the only two `ABSL_ASSUME` sites in upstream `s2cell_id.h`,
  `s2cell_id.cc`, `s2coords.h` and `s2coords.cc` (checked 2026-10-01).
- **Rationale**: the maintainer asked for this on 2026-10-01. Proofs written against
  upstream semantics must not silently encode upstream undefined behaviour as a
  precondition.
