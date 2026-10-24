# Contract: Harness and Artifact Conventions

These conventions tie the Markdown catalogue (FR-002, FR-005) to the executable proofs. Each
row can then be traced to exactly one harness, and each harness failure names a property
(US3).

## Harness naming

- **Crate-root modules.** Harness modules sit at the crate root (`verify/src/crash.rs`,
  `verify/src/hier.rs`, …), so a harness's full path is `<module>::<fn>`. Kani's `--exact`
  matches the full path, and `make verify-harness H=…` takes exactly that name (research R2).
- **Crash-freedom**: `crash::c_<op>`, one per in-scope inventory entry. For example
  `crash::c_parent` and `crash::c_advance_wrap`.
- **Properties**: `<area>::p_<area>_<nn>`, where `<area>` is `val`, `hier`, `range`, `trav`,
  `enc`, `anc` or `nbr`. For example `hier::p_hier_03`.
- **Split harnesses**: `<base>__<k>` for a harness split over a concrete parameter `k`. The
  catalogue and inventory list the split family once, with its range.
  - `__b<L>`: text of exactly `L` bytes (research R6), for example `enc::p_enc_06__b16`.
  - `__f<K>`: cells on face `K`, for example `nbr::p_nbr_01__f3` (research R12).
  - `__q<K>`: one of the four `(i, j)` vertex cases of `vertex_neighbors`, for example
    `nbr::p_nbr_03__q2` (research R12).
  - `__l<L>`: cells at level `L`, for example `trav::p_trav_05__l12` (research R15: a
    concrete level turns divisions by `4^level` into constants).
  - Two parameters join with `_`, for example `nbr::p_nbr_03__f2_q1` (face 2, quadrant 1).
  - A split family's cases together cover the whole domain. A split is never a restriction.
- **Shared harnesses**: when a property harness assumes exactly an operation's crash domain
  and preconditions, it also discharges that crash obligation, and the inventory names it as
  the crash harness. The text parsers and the neighbour operations use this (data-model §3).
- Each harness has a doc comment. The first line is the property or crash ID and its
  statement, verbatim from `PROPERTIES.md` or `INVENTORY.md`. If an upstream finding shaped
  the harness's assumptions, the doc comment cites its `U-NNN` (FR-018; data-model §9).

## Harness body shape

1. Draw the inputs from a **domain generator** in `verify/src/domain.rs`: `any_valid()`,
   `any_valid_at(level)`, `any_valid_on_face(face)`, `any_valid_or_end()`, `any_level()`,
   or `with_text::<L>(|s| …)`.
   `with_text` builds a symbolic `[u8; L]` and passes it as `&str` through
   `str::from_utf8_unchecked`, so `s` ranges over every byte string of exactly `L` bytes.
   That is a superset of every `&str` of that length (research R6). It is the only
   `unsafe` in `verify/`. A counterexample from it counts as a defect only if its bytes are
   valid UTF-8.
2. `kani::assume` **only** the documented preconditions for the operation.
3. Call the operation under test through the **public** `s2rst` API.
4. `assert!` against the reference model in `verify/src/spec.rs`, not against another call to
   the implementation, unless the property is itself a relation between operations (for
   example "next and prev are inverse").
5. Add at least one `kani::cover!` that shows the assumptions are satisfiable, and that each
   branch of interest (for example the wrap and no-wrap cases) is reachable.

## Attributes

- A harness with any loop MUST declare `#[kani::unwind(N)]`. `N` is the largest loop bound
  in the harness plus 1, counting loops in `spec.rs`, `domain.rs` and the standard library
  as well as in `s2rst`. Unwinding assertions stay **on**, so an insufficient bound fails
  instead of passing silently.
- `#[kani::solver(...)]` is allowed per harness. The solver choice is recorded in
  `TRUSTED_BASE.md`. CaDiCaL is the default; Kissat and MiniSat are allowed. The SMT
  backends are not used, because CBMC 6.11.0's SMT2 path failed on these harnesses
  (research R6).
- **Forbidden**: `#[kani::stub]` of any `s2rst` item, and stubbing of `core`/`std` functions
  that the code under test calls. Stubbing would replace the code being verified.
  `--no-unwinding-checks` is also forbidden.

## Resource budget

- The budget is set in research R15. Each harness MUST stay under the
  `VERIFY_MEM_PER_HARNESS` ceiling (default 4 GB) and under the 12-minute hard limit on CI.
  Until CI is measured, the proxy is 8 minutes on the workstation. The target is 60 s.
- A harness over the 5-minute soft limit is first split into a `__` family. If it stays
  over, it is listed in `EVALUATION.md` "Measurements" with a reason.
- A harness over a hard limit is split. If splitting does not bring every member under the
  limit, the operation is excluded as `tool-limit` (FR-007), or the property becomes
  `not-proven (tool-limit)`. Either way, the measured time and memory go into
  `EVALUATION.md`.
- The whole run MUST fit in 45 CPU-minutes, so that `make verify` meets SC-005.

## Mutant patch format (`verify/mutants/M-NNN-<slug>.patch`)

```
Mutant: M-NNN
Targets: P-HIER-03
Expect-Fail: hier::p_hier_03
Defect: parent() keeps the old marker bit (off-by-one in lsb shift)

diff --git a/core/src/s2/cell_id.rs b/core/src/s2/cell_id.rs
...
```

- The header lines come before the first `diff --git`. `git apply` ignores them.
- There is exactly one defect per patch. Each `proven*` property is the target of at least one
  mutant (SC-004).
- `Targets` lists catalogue properties (`P-*`). The only crash target is `C-advance`, on
  `M-001`, the overflow demonstration of US1 scenario 3. Crash proofs otherwise get their
  non-vacuity from cover checks (FR-008).
- An optional `Tier: slow` header marks a mutant whose `Expect-Fail` harnesses are in the
  weekly slow tier. `run-mutants.sh` then builds with `--features slow` and uses the
  slow-tier time limit (30 minutes, `VERIFY_SLOW_HARNESS_TIMEOUT`).

## Files and headers

- Every new `.rs` and `.sh` file carries the two-line SPDX header for original files, plus
  the note "Written for this crate; not ported from upstream S2" (Constitution, Licensing).
- `verify/` is a standalone workspace (`[workspace]` in its `Cargo.toml`), like `fuzz/`. It is
  `publish = false`, and has `s2rst = { path = "../core" }` as its only dependency.
- `verify/src/lib.rs` sets `#![deny(unsafe_code)]`. The single exception is
  `#[allow(unsafe_code, reason = "…")]` on `domain::with_text`, which is compiled only under
  `cfg(kani)` (research R6, approved by the user on 2026-10-01). `core/` stays
  `unsafe_code = "forbid"`.
