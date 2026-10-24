---

description: "Task list for the CellId formal-verification pilot"
---

# Tasks: Formal Verification of a Core Base Type (`CellId`)

**Input**: Design documents from `specs/001-formal-verify-base-type/`: plan.md, spec.md,
research.md (R0–R18), data-model.md, contracts/, quickstart.md.

**Prerequisites**:

- Kani 0.68.0 is installed locally: `cargo kani --version` prints
  `Kani Rust Verifier 0.68.0` and `CBMC 6.11.0`.
- For Phase 3 only, there is a C++ toolchain (`cmake`, `g++`) and the pinned upstream
  checkout at `/home/t/sources/s2geometry` (research R16).

**Tests**: The spec asks for these ordinary tests, so they appear as tasks:

- regression tests that fail without their fix (FR-015, FR-016);
- differential tests against C++ reference vectors (FR-015, FR-016; Principle I);
- bounded robustness tests (FR-017);
- reference-model sanity tests (research R3, R16).

The Kani harnesses are the verification deliverable itself, so they are implementation tasks.

**Organization**: Tasks are grouped by user story. Phase 3 belongs to no story: it holds the
known-defect fixes, their C++ vectors, and FR-017. It lands before US1, because P-ENC-06
states the fixed `from_token` rule. Phase 9 lists the follow-ups that FR-018 requires. They
are **not** implemented in this feature.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on unfinished tasks)
- **[Story]**: The user story the task belongs to (US1–US4)
- Paths are relative to the repository root.

## Memory safety while implementing (read first)

A single CBMC run can pass 6–10 GB within seconds (research R6, R12, R15). An uncapped run can
take down the session.

- **Every** Kani run goes through `make verify`, `make verify-harness H=…` or
  `make verify-mutants`, which apply the memory guard.
- Before T003 exists, cap each run by hand:
  `timeout -k 10 600 systemd-run --user --scope -p MemoryMax=4G -p MemorySwapMax=0 cargo kani …`
  Never run `cargo kani` uncapped over the whole crate.
- Run at most three capped runs at once.
- Give each scope a unique unit name, and run `systemctl --user reset-failed` before reusing
  one.

## Upstream findings rule (FR-018, read first)

If a harness, a C++ vector, or reading upstream code shows **undefined or wrong behaviour in
the upstream reference** (C++ canonical, Go, Java, or the generated vectors), do these
**before** marking the affected proof complete:

1. Append a new task to Phase 9 with the next free task ID. Give it a `U-NNN` id, the upstream
   `file:line`, the inputs, the observed behaviour, and the behaviour the Rust port documents
   (data-model §9).
2. Add the same entry under "Upstream findings" in `verify/EVALUATION.md`.
3. Cite the `U-NNN` in the affected harness's doc comment.

Never change upstream code. U-001 and U-002 are already recorded (T054, T055).

## Facts verified on Kani 0.68.0 that these tasks rely on

1. `--exact --harness <name>` matches the full module path, so harness modules sit at the
   crate root (`verify/src/crash.rs` gives `crash::c_parent`) (research R2).
2. `--concrete-playback=print` needs `-Z concrete-playback`. `--harness-timeout` and
   `--cbmc-args` need `-Z unstable-options`. `--cbmc-args --stop-on-fail` crashes Kani's
   output parser, so don't use it (research R12).
3. An unsatisfiable `kani::cover!` prints `** 0 of 1 cover properties satisfied`, yet the
   harness still reports `VERIFICATION:- SUCCESSFUL` and exits 0 (research R7).
4. With `-j … --output-format terse`, results are not labelled by name. Each harness prints
   `Thread N: Checking harness <name>...`, and its result block later starts with a
   `Thread N:` line (research R10).
5. On failure, terse output prints `Failed Checks: "<msg>"` and the file and line, but **no
   input values**. Running `-Z concrete-playback --concrete-playback=print --exact --harness <h>`
   prints a `#[test]` under "Concrete playback unit test for `<h>`", with
   `concrete_vals` and decoded comments such as `// 7ul` (research R10).
6. Under `ulimit -v`, CBMC stops with `Solver ran out of memory during propositional
   reduction.`, and Kani reports `VERIFICATION:- FAILED` and exits 1 (research R15).
7. `SATISFIABLE` lines in CBMC's log are Kani's reachability checks, not failures.
8. Every `impl Into<Level>` operation converts inside its body, and `Level::new` panics above
   30, in release builds too (`assert!`).

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create the `verify/` workspace, its runner with the memory guard and
counterexample printing, and the Makefile entry points.

- [X] T001 [P] Create `verify/Cargo.toml` as a standalone workspace modelled on `fuzz/Cargo.toml`.
  - **Header:** start with the two SPDX lines from `core/tests/cpp_cellid_diff.rs` as `#`
    comments. Follow them with a comment that explains the purpose and the standalone
    `[workspace]`.
  - **`[package]`:** `name = "s2rst-verify"`, `version = "0.0.0"`, `edition = "2024"`,
    `publish = false`.
  - **`[dependencies]`:** `s2rst = { path = "../core" }` and nothing else.
  - **`[lints.rust]`:** `unsafe_code = "deny"` and
    `unexpected_cfgs = { level = "warn", check-cfg = ['cfg(kani)'] }`. It is `deny`, not
    `forbid`, so that `domain::with_text` can carry the single approved `#[allow]`
    (research R6).
  - End with an empty `[workspace]` table, which keeps the crate out of the parent workspace.
- [X] T002 Create `verify/src/lib.rs`, an empty file for each module, and `verify/known-defects.txt`.
  - **Modules:** `verify/src/{spec,domain,crash,val,hier,range,trav,enc,anc,nbr}.rs`.
  - **Headers:** every `.rs` file starts with the two SPDX lines from
    `core/tests/cpp_cellid_diff.rs`, then `// Written for this crate (not ported from upstream S2).`
  - **`lib.rs`:**
    - a crate doc comment that names FR-002 and FR-005 and links `INVENTORY.md` and
      `PROPERTIES.md`;
    - `#![deny(unsafe_code)]`;
    - `pub mod spec;`, declared unconditionally;
    - `domain` and the eight harness modules, each behind `#[cfg(kani)]`.
  - **`verify/known-defects.txt`:** only `#` comment lines that explain the format,
    `<fully qualified harness> D-NNN` (data-model §6). FR-015 and FR-016 never appear here.
  - **Check:** `cd verify && cargo build && cargo test` succeeds on the pinned stable
    toolchain.
- [X] T003 [P] Create `verify/run-verify.sh` (contracts/verify-commands.md; research R7, R9, R10, R15).
  - **Shell:** bash with `set -euo pipefail`, and the SPDX header as `#` comments. Extra
    arguments are passed to `cargo kani`, for example `--exact --harness crash::c_parent`.
  - **Environment:**
    - `VERIFY_MEM_PER_HARNESS` (default `4G`; accept `G`/`M`, and convert to KiB for
      `ulimit -v`).
    - `VERIFY_JOBS`, defaulting to `min(nproc, ⌊MemAvailable / (7/8 · VERIFY_MEM_PER_HARNESS)⌋)` (3.5 GiB per job at the default ceiling), at least 1, read
      from `/proc/meminfo`. The backstop's `MemoryMax` is capped at `MemAvailable − 1 GiB`
      (research R15, corrected during implementation).
  - **Step 1, build:** `cargo kani --only-codegen`, with no memory limit.
  - **Step 2, guarded run:** in a subshell, run
    `ulimit -v <KiB>; cargo kani -j "$VERIFY_JOBS" --output-format terse "$@" 2>&1 | tee target/kani.log`.
    - Where `systemd-run --user --scope --quiet true` succeeds, also wrap the subshell in
      `systemd-run --user --scope --quiet --unit=s2rst-verify-$$ -p MemoryMax=<jobs × ceiling + 2G> -p MemorySwapMax=0`.
    - If that scope ends with result `oom-kill`
      (`systemctl --user show -p Result --value s2rst-verify-$$.scope`), print
      `memory backstop hit` and exit non-zero. Run `systemctl --user reset-failed` on the
      unit afterwards.
    - If `uname -s` is not Linux, print a warning that no memory guard is active.
  - **Parse `target/kani.log`** (facts 3, 4 and 6):
    - Map `Thread N: Checking harness <h>...` lines to harnesses.
    - Read each harness's result block: `** M of N cover properties satisfied`,
      `VERIFICATION:- SUCCESSFUL|FAILED`, and `Verification Time: <t>s`.
    - A FAILED block that contains `out of memory` is `OOM`.
    - A FAILED harness listed in `verify/known-defects.txt` is `KNOWN-DEFECT D-NNN`. Ignore
      blank lines and `#` lines in that file.
    - A listed harness that succeeds is an error: "stale known-defect entry".
  - **Step 3, counterexamples** (fact 5; FR-007, US3): for every `FAILED` harness that is not
    `OOM` or a known defect, re-run it under the same guard with
    `cargo kani -Z concrete-playback --concrete-playback=print --exact --harness <h>`. Print
    the generated `#[test]` block under a heading naming the harness.
  - **Print:**
    - one line per harness, sorted by name:
      `<h>  SUCCESSFUL|FAILED|OOM|KNOWN-DEFECT D-NNN  covers M/N  <t>s`;
    - one roll-up line per `__` split family: `<family>__*  n/n SUCCESSFUL`;
    - then a summary of the counts (FR-011).
  - **Exit** non-zero on any of these:
    - Kani exited non-zero, other than for known defects;
    - any harness is `FAILED` or `OOM`;
    - any harness has `M < N` covers satisfied;
    - there is a stale known-defect entry;
    - the backstop was hit.
  - **Validate the script on real logs before relying on it.** Use a scratch copy of
    `verify/` with throwaway harnesses, and don't commit them. Cover:
    - one passing harness;
    - one with a deliberately unsatisfiable cover;
    - one failing harness (for example `assert!(x.wrapping_mul(3) != 21)` on a symbolic
      `u64`); its playback must show `7`;
    - one that runs out of memory, for example `u64::from_str_radix` on 4 symbolic chars
      pushed into a `String` (research R6), with `VERIFY_MEM_PER_HARNESS=1G`;
    - a multi-harness `-j` run, including one `__` family.
- [X] T004 Add `verify-kani-check`, `verify` and `verify-harness` targets to `Makefile`. Follow its style: `.PHONY` plus a `## ` help comment on each target.
  - **`KANI_VERSION := 0.68.0`.** `verify-kani-check` fails with the hint
    `cargo install --locked kani-verifier --version $(KANI_VERSION) && cargo kani setup`
    unless `cargo kani --version` prints `Kani Rust Verifier $(KANI_VERSION)`.
  - **`verify`:** depends on the check, then runs `cd verify && cargo test`, then
    `cd verify && ./run-verify.sh`. Pass `VERIFY_JOBS` and `VERIFY_MEM_PER_HARNESS` through
    the environment when they are set.
  - **`verify-harness`:** runs `cd verify && ./run-verify.sh --exact --harness $(H)`.
  - Leave the `test` target unchanged (FR-012).

**Checkpoint**: `make verify` runs, with no harnesses yet, and `make test` is unchanged.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The reference model, the domain generators, and the final operation inventory
that every story depends on.

**⚠️ CRITICAL**: No user-story work starts until this phase is complete.

- [X] T005 Implement the bit-layout model in `verify/src/spec.rs` (data-model §1). Use plain `u64` arithmetic only, and nothing from `s2rst` except type names in signatures.
  - **`Decomposed`:** `pub struct Decomposed { pub face: u8, pub level: u8, pub path: u64 }`,
    deriving `Clone, Copy, Debug, PartialEq, Eq`. Well-formed means `face ≤ 5`, `level ≤ 30`
    and `path < 4^level`.
  - **`pow4(n)`:** `pub const fn pow4(n: u32) -> u64`.
  - **`compose(d)`:** `(face << 61) | (path << (61 − 2·level)) | (1 << (60 − 2·level))`.
  - **`decompose(x) -> Option<Decomposed>`:** defined by a **search over `level in 0..=30`**.
    - A level matches when bit `60 − 2·level` is set and every lower bit is 0.
    - Then require `x >> 61 ≤ 5`, and take `path = (x >> (61 − 2·level)) & (pow4(level) − 1)`.
    - Do **not** use `trailing_zeros` or `x & x.wrapping_neg()`. The model must not share
      the implementation's lsb tricks (FR-006).
  - **Other functions:**
    - `valid(x)`;
    - `lsb_for_level(level) = 1 << (60 − 2·level)`;
    - `size_ij(level) = 1 << (30 − level)`;
    - `end(level) = (6 << 61) | (1 << (60 − 2·level))`.
  - **Constants:** `NONE = 0`, `SENTINEL = u64::MAX`.
- [X] T006 Add the hierarchy, range, position and ancestor relations to `verify/src/spec.rs` (data-model §1). Define them on `Decomposed` only.
  - **Hierarchy:**
    - `parent(d)` for level ≥ 1: `{face, level−1, path / 4}`.
    - `child(d, k)` for level ≤ 29 and `k < 4`: `{face, level+1, 4·path + k}`.
    - `ancestor(d, l)` for `l ≤ level`: `{face, l, path / 4^(level−l)}`.
    - `contains(a, b)`: same face, `a.level ≤ b.level`, and `ancestor(b, a.level) == a`.
    - `child_position(d, l)` for `1 ≤ l ≤ level`: `(path >> 2·(level − l)) & 3`.
  - **Leaves and ranges:**
    - `leaf_index(d)`: `face·4^30 + path·4^(30−level)`.
    - `span(level)`: `4^(30−level)`.
    - `range(d)`: `(leaf_index, leaf_index + span − 1)`.
    - `leaf_id(li)`: `compose({li >> 60, 30, li & (4^30 − 1)})`.
  - **Positions:**
    - `position(d)`: `face·4^level + path`.
    - `from_position(level, p) -> u64` for `p ≤ 6·4^level`: compose
      `{p / 4^level, level, p % 4^level}`, giving `end(level)` when `p == 6·4^level`.
  - **Common ancestor:** `common_ancestor_level(a, b) -> Option<u8>` is the largest
    `l ≤ min(levels)` with `ancestor(a, l) == ancestor(b, l)`. It is `None` on different
    faces.
- [X] T007 Add the Hilbert-curve and text models to `verify/src/spec.rs` (data-model §1).
  - **Tables:** copy them from upstream `s2coords.cc`, **not** from `s2rst::s2::coords`. T008's
    `CELLID_FACE_IJ` test confirms them.
    - `POS_TO_IJ = [[0,1,3,2],[0,2,3,1],[3,2,0,1],[3,1,0,2]]`
    - `IJ_TO_POS = [[0,1,3,2],[0,3,1,2],[2,3,1,0],[2,1,3,0]]`
    - `POS_TO_ORIENTATION = [1,0,0,3]`
    - `SWAP = 1`, `INVERT = 2`
  - **`face_ij_orientation(d) -> (u8, u32, u32, u8)`:** follows upstream
    `S2CellId::ToFaceIJOrientation`.
    - Start with orientation `face & 1`.
    - Walk the cell's own `level` digits. If `level < 30`, continue with digit 2, then
      `29 − level` zeros.
    - For each digit: `ij = POS_TO_IJ[o][digit]`, `i = i<<1 | ij>>1`, `j = j<<1 | ij&1`, and
      `o ^= POS_TO_ORIENTATION[digit]`.
    - Return the orientation reached after the cell's own digits.
  - **`leaf_from_face_ij(face, i, j)`** for `i, j < 2^30`: for `k` from 29 down to 0, take
    `digit = IJ_TO_POS[o][((i>>k)&1)<<1 | ((j>>k)&1)]`, append the digit and update `o`.
    Compose `{face, 30, path}`.
  - **`center_si_ti(d) -> (u8, u32, u32)`:** follows upstream `GetCenterSiTi`. `(i0, j0)` is
    the cell's own-digit `(i, j)`, shifted left by `30 − level`, and
    `size = 1 << (30 − level)`. The result is `(face, 2·i0 + size, 2·j0 + size)`.
  - **Tokens:**
    - `token(x) -> ([u8; 16], usize)`: `"X"` for 0, otherwise the lowercase hex digits of `x`
      with the trailing `'0'`s stripped (C++ `ToToken`).
    - `parse_token(s: &[u8]) -> u64` follows C++ `FromToken` (`s2cell_id.cc`, checked in
      research R16). It returns 0 if `s` is longer than 16 bytes or contains any byte outside
      `0-9a-fA-F`. Otherwise it ORs `digit << (60 − 4·idx)`. So `""` gives 0, and so do
      `"X"` and `"+1"`.
  - **Debug strings:**
    - `debug_string(x) -> ([u8; 32], usize)`: for a valid `x`, the face digit, `/`, then one
      digit per level. Otherwise `Invalid: ` plus 16 zero-padded lowercase hex digits
      (C++ `ToString`; FR-016).
    - `parse_debug_string(s: &[u8]) -> Option<u64>` follows C++ `FromDebugString`:
      `level = len − 2` must be in `0..=30`, the face byte in `'0'..='5'`, `s[1] == b'/'`,
      and every digit in `'0'..='3'`.
- [X] T008 [P] Write the reference-model sanity tests for the existing vector sections in `verify/tests/spec_vs_cpp.rs` (research R3). They run under plain `cargo test`.
  - **Loading:** `include_str!("../../core/tests/data/cpp_cellid_vectors.csv")`, plus a
    `section_lines(name)` helper like the one in `core/tests/cpp_cellid_diff.rs`.
  - **One test per section.** Assert that the section is non-empty and that every id is
    `spec::valid`, and count the rows checked:
    - `CELLID_CHILDREN` (`id,c0,c1,c2,c3`): `compose(child(d,k)) == ck`.
    - `CELLID_RANGE` (`id,range_min,range_max`): `leaf_id(range(d).0)` and
      `leaf_id(range(d).1)`.
    - `CELLID_CONTAINS` (`a,b,contains,intersects`, as 0/1): `contains(a,b)`, and
      `contains(a,b) || contains(b,a)`.
    - `CELLID_FACE_IJ` (`id,face,i,j,orientation`): `face_ij_orientation`.
    - Do not use `CELLID_EDGE_NEIGHBORS`. The model has no cross-face neighbours
      (research R12).
  - **Hand-written cases:**
    - `parse_token(b"+1") == 0`, `parse_token(b"X") == 0`, `parse_token(b"") == 0` and
      `parse_token(b"1") == 1 << 60`.
    - `debug_string` of `2` is `Invalid: 0000000000000002`, and of `5`, the valid leaf `0b101`, is `0/` followed by 29 zeros and a `2`.
    - `end(0) == CellId::end(0u8).0`.
  - The text sections are checked later, in T020.
- [X] T009 [P] Implement the domain generators in `verify/src/domain.rs` (data-model §2; research R4, R6). The module is `#[cfg(kani)]`. Build every cell domain from the model (`spec::compose` or `spec::valid`), never with `kani::assume(c.is_valid())`.
  - `any_u64() -> CellId`.
  - `any_level() -> Level`: `Level::try_new(kani::any())`, assumed `Some`.
  - `any_face() -> u8` with `≤ 5`, and `any_face_t() -> Face`.
  - `any_decomposed() -> spec::Decomposed`, assumed well-formed.
  - `any_valid() -> (CellId, spec::Decomposed)`, `any_valid_at(level: u8)`, and
    `any_valid_on_face(face: u8)` (for the `__f<K>` splits).
  - `any_valid_or_end() -> CellId`: chosen symbolically from a valid cell, `spec::end(l)` for
    any `l ≤ 30`, and `sentinel`.
  - **`with_text<const L: usize>(f: impl FnOnce(&str))`:** draw `let b: [u8; L] = kani::any();`
    and call `f(unsafe { std::str::from_utf8_unchecked(&b) })`.
    - Put `#[allow(unsafe_code, reason = "byte-superset text domain for the parsers, research R6")]`
      on the function, and add a `// SAFETY:`-style comment with the R6 argument: the parsers
      read only bytes, so this covers every `&str` of `L` bytes.
    - It is the only `unsafe` in `verify/`.
- [X] T010 [P] Write `verify/INVENTORY.md` (FR-002; data-model §3) and start `verify/EVALUATION.md`.
  - **Rows:** one per item, 64 in total.
    - **The 54 inherent methods:** none, sentinel, from_face, from_face_pos_level,
      from_point, from_lat_lng, id, is_valid, face, pos, level, lsb, is_leaf, is_face,
      child_position, parent_at_level, parent, children, child_begin, child_end,
      child_begin_at_level, child_end_at_level, begin, end, range_min, range_max, contains,
      intersects, next, prev, next_wrap, prev_wrap, advance, advance_wrap,
      distance_from_begin, get_center_si_ti, to_point_raw, to_point, to_lat_lng, center_st,
      center_uv, size_st, bound_st, bound_uv, to_face_ij_orientation, to_token, from_token,
      edge_neighbors, vertex_neighbors, all_neighbors, common_ancestor_level, maximum_tile,
      to_debug_string, from_debug_string.
    - **The 3 free functions:** `s2::lsb_for_level`, `s2::size_ij`, `s2::from_face_ij`.
    - **The 7 trait impls:** `Default`, `From<u64> for CellId`, `From<CellId> for u64`,
      `From<&Point>`, `From<&LatLng>`, `Display`, `FromStr`.
  - **Columns:** every field of data-model §3, including `result_validity`, `panics_doc` and
    `upstream_findings`.
  - **Exclusions:** 16 rows, from research R6 and R12.
    - `float-geometry` (12): from_point, from_lat_lng, to_point_raw, to_point, to_lat_lng,
      center_st, center_uv, size_st, bound_st, bound_uv, `From<&Point>`, `From<&LatLng>`.
    - `tool-limit` (3), each with its R6 measurement:
      - `to_token`: over 5 min;
      - `to_debug_string`: OOM above 6 GB on the `format!` branch;
      - `Display`: over 5 min.
    - `float-geometry: crash-freedom intractable` (1): `all_neighbors`, OOM above 9.5 GB at
      gap 0 (R12).
    - The other **48** rows are `in-scope`. Mark `vertex_neighbors` "at risk (R12)".
  - **Domains and preconditions:** start from the widest domain the argument types allow
    (FR-003). Narrow only by a documented precondition, citing its source: rustdoc,
    `debug_assert!`, or upstream `s2cell_id.h`/`s2coords.h`. Known cases:
    - `level`: the domain excludes ids with 62 or more trailing zeros, including `0`
      (fact 8; research R18).
      - Upstream: `ABSL_ASSUME(id_ != 0)`, and −1 for the others.
      - Set `undocumented_panic` and `upstream_findings = U-001`.
    - `parent`: not a face (rustdoc and `debug_assert!`).
    - `children`, `child_begin`, `child_end`: valid, not a leaf (`debug_assert!`).
    - `child_position(l)`: valid, `1 ≤ l ≤ level` (`debug_assert!`).
    - `parent_at_level(l)`: `l ≤ level` (rustdoc).
    - `child_*_at_level(l)`: `l ≥ level` (rustdoc).
    - `next_wrap`, `prev_wrap`: valid (`debug_assert!`).
    - `vertex_neighbors(l)`: `l < level`, from upstream `DCHECK_LT` only. Set
      `undocumented_panic`, because `Level::new(l+1)` panics at `l = 30`.
    - `from_face`: face 0–5 (rustdoc).
    - `from_face_pos_level`: `pos < 2^61` (rustdoc "61-bit").
    - `from_face_ij`: Face × `[0, 2^30 − 1]` × `[0, 2^30 − 1]`, the `IJ` domain
      (data-model §2).
      - Upstream documents this range (`s2coords.h:44–45`, cited by
        `s2cell_id.h:484–486`); cite it in the row.
      - The crash harness first tries Face × all `i32` × all `i32`. If that passes within
        the R15 budget, record the wider domain, which is the stronger claim.
      - If Rust panics outside the documented range, set `undocumented_panic` and cover it
        in T028.
    - `common_ancestor_level`: the domain excludes only the pair (`none`, `none`), where
      Rust panics in `ilog2(0)` (research R4, R18).
      - Upstream: C++ `ABSL_ASSUME(bits != 0)` (`s2cell_id.cc:197–198`), undefined
        behaviour on that pair; Go returns level 30.
      - Set `undocumented_panic` and `upstream_findings = U-002`.
    - Every `impl Into<Level>` argument: a raw `u8 > 30` panics in the conversion (fact 8).
      Record that in `undocumented_panic`.
  - **Planned crash harnesses:**
    - `crash::c_<op>` for the 43 integer-only rows.
    - `enc::p_enc_06__b{0..=17}` for `from_token`, `enc::p_enc_09__b{0..=33}` for
      `from_debug_string`, and `enc::p_enc_10__b{0..=33}` for `FromStr`. These are shared
      harnesses (data-model §3; research R6).
    - `nbr::p_nbr_01` for `edge_neighbors`, and `nbr::p_nbr_03__q{0..3}` for
      `vertex_neighbors` (research R12).
  - **`properties` column:** pre-fill it from the Operations column of data-model §4.1.
  - **Footer:**
    - `ij_level_to_bound_uv` and `expanded_by_distance_uv` are outside FR-002's
      definition, because they are floating-point, not `CellId` constructors.
    - The derived impls (Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash) are
      outside it too, being compiler-generated.
  - **`verify/EVALUATION.md`:** create it with these sections:
    - "Effort log": a table with columns date / phase / person-hours.
    - "Exclusions and tool limits": copy the R6 and R12 probe tables, with a link to
      research.md (FR-002).
    - "Upstream findings": U-001 and U-002, copied from research R18, pointing to T054 and
      T055.
    - "`# Panics` documentation added": empty for now.
    - "Measurements": empty for now.

**Checkpoint**: `cd verify && cargo test` passes, and the inventory lists 48 in-scope rows.

---

## Phase 3: Known-Defect Fixes, C++ Vectors and Untrusted-Input Coverage (FR-015, FR-016, FR-017), no story

**Purpose**: Three behaviour-level changes, each its own commit, kept apart from all
verification work (research R11, R13, R14, R16).

- The FR-015 and FR-016 commits each carry their C++ vector sections and differential tests,
  so every commit passes `make test`.
- Land T014 before T025, because P-ENC-06 states the fixed rule.
- C++ is needed only to regenerate vectors, never by `make test` or CI.

- [X] T011 Prepare the C++ reference build out of tree and check that the generator reproduces the committed vectors (research R16; `tools/refgen-cpp/README.md`).
  - Run `cd /home/t/sources/s2geometry && cmake --install build`. That installs to
    `/tmp/s2-install`, the `CMAKE_INSTALL_PREFIX` in `build/CMakeCache.txt`.
  - Confirm the checkout is the pinned oracle: `git log --oneline -2` shows `8d5c2a8` as
    the upstream base.
  - Build the generator **outside the repository**, because `tools/refgen-cpp/build` is not
    gitignored:
    `cmake -S tools/refgen-cpp -B /tmp/refgen-cpp-build -DCMAKE_BUILD_TYPE=Release -DCMAKE_PREFIX_PATH=/tmp/s2-install && cmake --build /tmp/refgen-cpp-build`.
  - **Baseline:** `LD_LIBRARY_PATH=/tmp/s2-install/lib /tmp/refgen-cpp-build/generate_cellid | cmp - core/tests/data/cpp_cellid_vectors.csv`
    must report no difference. If it does, **stop and report**: the vectors can't be
    extended safely.
- [X] T012 Write the FR-015 regression test first, in a new file `core/src/s2/cell_id_tests.rs`.
  - **Header:** the two SPDX lines from `core/src/s2/convex_hull_query_tests.rs`, then a
    `//!` comment saying the tests are original and not ported from upstream S2.
  - **Test:** `use super::*;` and `from_token_rejects_non_hex_characters`. It asserts
    `CellId::from_token(t) == CellId::none()` for `"+1"`, `"+fffffffffffffff"`, `"+"`, `"-1"`
    and `"+0"`. Add a comment that cites C++ `S2CellId::FromToken` and Go
    `CellIDFromToken`.
  - **Wiring:** append `#[cfg(test)]`, `#[path = "cell_id_tests.rs"]` and
    `mod cell_id_tests;` at the end of `core/src/s2/cell_id.rs`, as
    `core/src/s2/convex_hull_query.rs` does.
  - **Check:** `cd core && cargo test --lib s2::cell_id::cell_id_tests` must **fail** on
    `"+1"` (Constitution III).
- [X] T013 Add the `CELLID_FROM_TOKEN` and `CELLID_FROM_DEBUG_STRING` sections to `tools/refgen-cpp/generate_cellid.cc`, regenerate `core/tests/data/cpp_cellid_vectors.csv`, and add their differential tests to `core/tests/cpp_cellid_diff.rs` (research R16).
  - **Generator:** append both sections **after** `CELLID_FACE_IJ`, so the existing RNG
    stream and lines stay unchanged.
    - **`# CELLID_FROM_TOKEN`:** rows of `token_hex,id`. `token_hex` is the token's bytes as
      lowercase hex, and an empty field for `""`. `id` is `FromToken(token).id()`. Inputs,
      all valid UTF-8:
      - `""`, `"X"`, `"x"`, `"0"`, `"1"`, `"+1"`, `"-1"`, `" 1"`, `"1 "`, `"+"`, `"+0"`,
        `"+fffffffffffffff"`, `"0x1"`, `"89c25"`, `"89C25"`, 16 `f`s, 17 `f`s, `"é"`,
        `"€1"`;
      - `ToToken()` of 500 `RandCell()`s;
      - 1,000 random strings of length 0–17, drawn from `0-9a-fA-F`, `+-xX /`, `"\0"`, `"é"`
        and `"€"`.
    - **`# CELLID_FROM_DEBUG_STRING`:** rows of `string_hex,id`. `id` is
      `FromDebugString(s).id()`, which is 0 for `None`. Inputs:
      - `ToString()` of 500 `RandCell()`s;
      - `"0/"`, `"5/"`, `"6/0"`, `"0/4"`, `"0|0"`, `"0/"` plus 31 digits, `""`, `"/"`,
        `"0"`;
      - 1,000 random strings over `0-9/` of length 0–33.
    - Update the "Sections emitted" table in `tools/refgen-cpp/README.md`.
  - **Regenerate:** run the T011 command into the committed CSV.
    `git diff core/tests/data/cpp_cellid_vectors.csv` must show **only appended lines**.
  - **Diff tests:**
    - `from_token_matches_cpp`: hex-decode, `std::str::from_utf8(...).unwrap()`, then
      `CellId::from_token(s).id() == id`.
    - `from_debug_string_matches_cpp`: `CellId::from_debug_string(s).map_or(0, CellId::id) == id`.
    - Both assert that their section is non-empty.
  - **Checks:**
    - `cargo test -p s2rst --test cpp_cellid_diff` shows `from_token_matches_cpp`
      **failing** on the `+1` rows before T014.
    - `from_debug_string_matches_cpp` should already pass. If it fails, that is a new parity
      defect: **stop and report** before going on (FR-010).
- [X] T014 Fix `CellId::from_token` in `core/src/s2/cell_id.rs` (research R11), then commit FR-015.
  - **Change:** after the `token == "X"` check, add
    `if !token.bytes().all(|b| b.is_ascii_hexdigit()) { return CellId::none(); }`. Add a
    one-line comment that `from_str_radix` alone accepts a leading `+`.
  - **Rustdoc:** "Returns `CellId::none()` if the token is empty, longer than 16 characters,
    or contains anything other than hex digits (as C++ `S2CellId::FromToken`)."
  - **Checks:** the T012 and T013 tests pass, and `make test` passes.
  - **Commit:** T012, T013 and T014 together as one commit, for example
    `fix(s2): reject non-hex characters in CellId::from_token`.
- [X] T015 Add the FR-016 regression test `invalid_debug_string_is_zero_padded` to `core/src/s2/cell_id_tests.rs` (research R13).
  - **Asserts:**
    - `CellId(2).to_debug_string() == "Invalid: 0000000000000002"`;
    - `CellId::none().to_debug_string() == "Invalid: 0000000000000000"`;
    - `CellId::sentinel().to_debug_string() == "Invalid: ffffffffffffffff"`;
    - `CellId(2).to_string()` equals the debug string;
    - a valid cell's debug string is unchanged, for example `CellId::from_face(3u8)` gives
      `"3/"`.
  - Add a comment citing C++ `S2CellId::ToString` (`absl::Hex(id(), absl::kZeroPad16)`).
  - **Check:** confirm that the test **fails** on the current code.
- [X] T016 Add the `CELLID_TO_STRING` section to `tools/refgen-cpp/generate_cellid.cc`, regenerate `core/tests/data/cpp_cellid_vectors.csv`, and add `to_debug_string_matches_cpp` to `core/tests/cpp_cellid_diff.rs` (research R16).
  - **Generator:** append `# CELLID_TO_STRING` after the T013 sections, with rows of
    `id,string`. `string` is `S2CellId(id).ToString()`, which never contains a comma. Inputs:
    - `0`, `2`, `5` (a valid leaf), `u64::MAX`, `1<<61`, `1<<62`, `1<<63`, and `End(l)` for `l` in `0..=30`;
    - 1,000 random `u64`s;
    - 500 `RandCell()`s.

    Update the README table.
  - **Regenerate:** only appended lines may appear in the diff.
  - **Diff test:** for each row, `CellId(id).to_debug_string() == string` and
    `CellId(id).to_string() == string`.
  - **Check:** the test **fails** on the invalid rows before T017.
- [X] T017 Fix `CellId::to_debug_string` in `core/src/s2/cell_id.rs`: change `"Invalid: {:x}"` to `"Invalid: {:016x}"` (research R13). Then commit FR-016.
  - Update the rustdoc to say how invalid ids are printed, as C++ `ToString` does.
  - **Checks:** the T015 and T016 tests pass, and `make test` passes.
  - **Commit:** T015, T016 and T017 together, for example
    `fix(s2): zero-pad invalid CellId debug strings like C++ ToString`.
- [X] T018 [P] Add the FR-017 bounded robustness tests to `core/tests/decoder_robustness.rs` (research R14). Use a new section headed "text parsers", in the file's existing style (`rng(seed)` helper, labelled cases).
  - **Names:** every test function starts with `text_parsers_`, so that
    `cargo test --test decoder_robustness text_parsers` selects them (quickstart §7).
  - **Deterministic cases**, for `CellId::from_token`, `CellId::from_debug_string` and
    `str::parse::<CellId>`:
    - `""`, `"X"`, 16 and 17 hex digits;
    - `"+1"`, `"-1"`, `" 1"`;
    - non-hex ASCII, `"é"`, `"€1"`;
    - `"0/"`, `"6/0"`, `"0/4"`, `"0/"` plus 31 digits, `"0|0"`, and a string containing `'\0'`.
  - **Random cases:** a few thousand short strings mixing hex digits, `/`, decimal digits and
    random `char`s.
  - **Contract:** no panic, and:
    - a rejected token gives `CellId::none()`;
    - an accepted token satisfies `from_token(&from_token(s).to_token()) == from_token(s)`;
    - `from_debug_string(s) == Some(c)` implies `c.to_debug_string() == s`;
    - `s.parse::<CellId>() == CellId::from_debug_string(s).ok_or(())`.
- [X] T019 [P] Add the FR-017 fuzz target `fuzz/fuzz_targets/parse_cell_id_text.rs` and register it (research R14). Then commit FR-017.
  - **Target:**
    - Use the header and doc style of `fuzz/fuzz_targets/decode_cell.rs`.
    - Take `|data: &[u8]|`, and return early unless `std::str::from_utf8(data)` succeeds.
    - Call all three parsers, and assert the T018 contract.
  - **Registration:**
    - a `[[bin]]` entry in `fuzz/Cargo.toml`, following `decode_cap`;
    - seeds `fuzz/seeds/parse_cell_id_text/seed-00`…`seed-05`: `X`, `89c25`, `3/0123`,
      `+1`, 16 `f`s, and `Invalid: 0000000000000002`;
    - a matrix entry `parse_cell_id_text` in `.github/workflows/fuzz.yml`.
  - **Smoke run:**
    `cargo +nightly fuzz run parse_cell_id_text fuzz/corpus/parse_cell_id_text fuzz/seeds/parse_cell_id_text -- -max_total_time=60`
    finishes without a crash.
  - **Commit:** T018 and T019 together as the FR-017 change.
- [X] T020 Add the text-model sanity tests to `verify/tests/spec_vs_cpp.rs` (research R16). Needs T007, T013 and T016.
  - `CELLID_FROM_TOKEN`: `spec::parse_token(bytes) == id`.
  - `CELLID_FROM_DEBUG_STRING`: `spec::parse_debug_string(bytes).unwrap_or(0) == id`.
  - `CELLID_TO_STRING`: `spec::debug_string(id) == string`.
  - Each test asserts that its section is non-empty.

**Checkpoint**:

- SC-008, SC-009 and SC-010 are met, and `make test` passes.
- `git diff --stat master -- core fuzz tools .github/workflows/fuzz.yml` shows only these
  three commits.

---

## Phase 4: User Story 1 — Proven absence of crashes for `CellId` (Priority: P1) 🎯 MVP

**Goal**: Every one of the 48 in-scope operations is proven free of panics, overflow and
undefined behaviour. The 45 integer and neighbour operations are proven over their whole
documented domain. The three text parsers are proven up to their input-size bound. Every
proven panic is documented (SC-011).

**Independent Test**:

- `make verify` reports every crash harness in `INVENTORY.md` as `SUCCESSFUL`, with all
  covers satisfied, and exits 0.
- Applying `M-001` makes `crash::c_advance` fail. The output prints a concrete-playback test
  with the inputs.

- [X] T021 [US1] Write the 17 crash harnesses for constructors, queries, free functions and conversions in `verify/src/crash.rs` (contracts/harness-conventions.md).
  - **Harnesses:** `c_none`, `c_sentinel`, `c_from_face`, `c_from_face_pos_level`, `c_id`,
    `c_is_valid`, `c_face`, `c_pos`, `c_level`, `c_lsb`, `c_is_leaf`, `c_is_face`,
    `c_lsb_for_level`, `c_size_ij`, `c_default`, `c_from_u64`, `c_u64_from`.
  - **Shape:**
    - `#[kani::proof]`, with a doc comment whose first line is the row's `C-<op>`
      statement from `INVENTORY.md`.
    - Draw from the inventory domain, `kani::assume` exactly the inventory preconditions,
      and call through the public API.
    - `c_level` cites U-001 in its doc comment.
  - **Covers:** one showing that the domain is non-empty, plus one per distinct outcome, for
    example a valid and an invalid result (FR-008).
  - **Develop** one harness at a time with `make verify-harness H=crash::c_level` and so on.
    Apply the upstream findings rule to anything new.
- [X] T022 [US1] Write the 10 hierarchy crash harnesses in `verify/src/crash.rs`.
  - **Harnesses:** `c_child_position`, `c_parent_at_level`, `c_parent`, `c_children`,
    `c_child_begin`, `c_child_end`, `c_child_begin_at_level`, `c_child_end_at_level`,
    `c_begin`, `c_end`.
  - Level arguments come from `any_level()`. Covers include face, leaf and mid-level inputs.
- [X] T023 [US1] Write the 13 range, traversal and ancestor crash harnesses in `verify/src/crash.rs`.
  - **Harnesses:** `c_range_min`, `c_range_max`, `c_contains`, `c_intersects`, `c_next`,
    `c_prev`, `c_next_wrap`, `c_prev_wrap`, `c_advance`, `c_advance_wrap`,
    `c_distance_from_begin`, `c_maximum_tile`, `c_common_ancestor_level`.
  - **Domains:** use `any_valid_or_end()` where the inventory says `ValidOrEnd`. `steps` is
    `kani::any::<i64>()`. Covers include `i64::MIN`, `i64::MAX`, and the wrap and clamp
    cases.
  - **Unwind:** `c_maximum_tile` uses `#[kani::unwind(31)]` (research R5).
  - `c_common_ancestor_level` draws two ids and assumes only `!(a == none && b == none)`.
    Its doc comment cites U-002.
- [X] T024 [US1] Write the 3 face/(i,j) crash harnesses in `verify/src/crash.rs`: `c_get_center_si_ti`, `c_to_face_ij_orientation` and `c_from_face_ij`.
  - **Domain:** as recorded for each row in T010. For `c_from_face_ij`, try all `i32`
    first, and fall back to the documented `IJ` domain (data-model §2).
  - **Unwind:** the lookup-table loops use `#[kani::unwind(9)]` (research R5).
- [X] T025 [US1] Write the parser harness families in `verify/src/enc.rs` (research R6). They discharge C-from_token, C-from_debug_string and C-FromStr, and they are also P-ENC-06, P-ENC-09 and P-ENC-10. Needs T014.
  - **Macro:** generate one `#[kani::proof] #[kani::unwind(40)]` function per length. The
    probe measured `unwind(40)` as sufficient, and unwinding assertions stay on.
    - `p_enc_06__b0` … `p_enc_06__b17`:
      `with_text::<L>(|s| assert_eq!(CellId::from_token(s).id(), spec::parse_token(s.as_bytes())))`.
    - `p_enc_09__b0` … `p_enc_09__b33`:
      `CellId::from_debug_string(s).map(CellId::id) == spec::parse_debug_string(s.as_bytes())`.
    - `p_enc_10__b0` … `p_enc_10__b33`:
      `s.parse::<CellId>().map(CellId::id) == spec::parse_debug_string(s.as_bytes()).ok_or(())`.
      This is stated against the model, not the implementation (FR-006).
  - **Doc comments:** the first line of each harness is the P-ENC statement, which also names
    the crash obligation it discharges.
  - **Covers:**
    - members that can accept input (`L` 1–16 for tokens, 2–32 for debug strings) cover both
      "accepted" and "rejected";
    - the other members cover "rejected" only, because "accepted" is unsatisfiable there by
      design.
  - **Budget:** each member should take seconds. The probe measured 0.2–14 s and at most
    0.5 GB.
  - **Counterexamples:** before reporting one as a defect, check that its bytes are valid
    UTF-8 (research R6).
- [X] T026 [US1] Write the neighbour harnesses in `verify/src/nbr.rs` (research R12, R15). `p_nbr_01` discharges C-edge_neighbors, and the `p_nbr_03__q*` family discharges C-vertex_neighbors.
  - **`p_nbr_01`** (P-NBR-01): over `any_valid()`, assert that `edge_neighbors()` gives
    four valid cells at `level()`, pairwise distinct, none equal to `c`.
    - `#[kani::unwind(10)]`, with covers "wrap" and "same face".
    - The probe took 411 s and 2.5 GB, under the 8-minute workstation proxy.
    - If it is over the proxy, use the per-face family `p_nbr_01__f0` … `p_nbr_01__f5`
      instead, built on `any_valid_on_face(K)`. Each took ≤ 3.5 min in the probe.
  - **`p_nbr_03__q0` … `p_nbr_03__q3`** (P-NBR-03): the probe took 10.6 min unsplit, over the
    8-minute proxy, so split from the start.
    - Each case assumes one combination of `(i & half_size != 0, j & half_size != 0)` for
      the cell's `(i, j)` (from `spec::face_ij_orientation`), where `half_size` is the size
      at level `l+1`. The four cases together cover the domain.
    - Each draws `any_valid()` × `l < level`, the documented upstream precondition.
    - Each asserts that `vertex_neighbors(l)` gives 3 or 4 valid, pairwise distinct cells at
      level `l`, one of them `parent_at_level(l)`.
    - Covers: "3 cells" and "4 cells".
  - **If any member stays over the proxy**, also try splitting by face.
  - **If nothing fits:**
    - mark `vertex_neighbors` `excluded (tool-limit: over the R15 budget)` in
      `verify/INVENTORY.md`;
    - record the measurements in `verify/EVALUATION.md`;
    - **STOP and report to the user** that SC-002 falls to 47/64 (73.4%). Do not change the
      spec yourself.
  - **Outcome (2026-10-01):** nothing fit the 8-minute proxy (`verify/EVALUATION.md`,
    "Measurements"). The user chose a **weekly slow tier** over the exclusion (spec
    clarification, FR-012, SC-005 as amended). The crash-only family
    `crash::c_edge_neighbors__f0…f5` discharges C-edge_neighbors; it first stayed in the
    per-change tier, then moved to the slow tier under T046's budget rule.
    `crash::c_vertex_neighbors` (unsplit) and the whole `nbr` module (P-NBR-01/02/03) sit
    behind the `verify/` feature `slow` and run through `make verify-slow` with 30 minutes
    per harness. A slow-tier harness over 30 minutes is a `tool-limit` result.
- [X] T027 [US1] Update `verify/INVENTORY.md` with the crash results.
  - **Every in-scope row:** `crash_harness` (the final names, including splits), `covers`
    (`M/N`), and `crash_status`.
  - **The three parsers:** `crash_status` is `proven-within-bounds`. `crash_bound` is
    "input size: byte length ≤ 17 (token) / ≤ 33 (debug string); longer texts are rejected by
    the length check before any loop; exercised by the FR-017 tests and fuzzing" (FR-007).
  - **Undocumented panics:** record each one the harnesses forced you to assume (FR-003). If
    a panic also differs from upstream, record a `D-NNN` in `verify/EVALUATION.md` (FR-010).
  - **Check:** SC-001 needs every in-scope row proven, and at least 90% of them over the
    whole domain. Expect 45/48 (94%).
- [X] T028 [US1] Add the `# Panics` rustdoc sections in `core/src/s2/cell_id.rs`, as one documentation-only commit (FR-003, SC-011; research R17).
  - **Scope:** every row with an `undocumented_panic` in `verify/INVENTORY.md`, plus all 13
    `impl Into<Level>` functions by inspection. That includes the excluded `size_st`,
    `all_neighbors` and `ij_level_to_bound_uv`.
  - **Wording:** state the condition in the caller's terms, for example "Panics if `level`
    converts from a raw integer greater than 30", or for `level()`, "Panics if this id is 0
    or has 62 or more trailing zero bits (no level marker)". Where the panic differs from
    upstream, say so in one sentence (Principle I), and cite the `D-NNN`/`U-NNN`.
  - **Records:** fill the `panics_doc` column of `verify/INVENTORY.md`, and list each
    section under "`# Panics` documentation added" in `verify/EVALUATION.md`.
  - **Checks:**
    - `make doc` and `make test` pass.
    - `git diff` shows rustdoc lines only.
    - Commit it separately, for example `docs(s2): document CellId panics`.
- [X] T029 [US1] Create `verify/mutants/M-001-advance-unclamped.patch` in the header-plus-diff format of contracts/harness-conventions.md.
  - **Header:** `Mutant: M-001`, `Targets: C-advance`, `Expect-Fail: crash::c_advance`,
    and `Defect: advance() no longer clamps positive steps (steps.min(max_steps) removed), so self.0 + (steps << shift) can overflow`.
  - Make the patch in a throwaway `git worktree`, so the working tree is never touched.
  - **Manual check of US1 scenario 3:**
    1. `git apply` the patch.
    2. `make verify-harness H=crash::c_advance` must report `FAILED` with the overflow
       check, followed by the concrete-playback test showing the inputs.
    3. `git apply -R` the patch.
  - **Done via `run-mutants.sh M-001`** in a throwaway worktree, because verification runs
    were building from the working tree at the time: `crash::c_advance` `FAILED` ("attempt
    to add with overflow"), followed by the concrete-playback test.
- [X] T030 [US1] Run `make verify` and record the results in `verify/EVALUATION.md`.
  - Every crash harness must be `SUCCESSFUL` with all covers satisfied, and the exit code
    must be 0.
  - In "Measurements", record each harness's time and the total wall-clock and CPU time,
    with the machine and `VERIFY_JOBS` used. Compare them with the R15 budget:
    45 CPU-minutes, the 8-minute workstation proxy, and 4 GB per harness.
  - Append the US1 hours to the "Effort log".

**Checkpoint**: US1 is complete and the MVP is shippable. SC-001 and SC-011 hold.

---

## Phase 5: User Story 2 — Proven functional correctness properties (Priority: P2)

**Goal**: The 49-entry catalogue of data-model §4.1 is in `verify/PROPERTIES.md`, and every
entry is proven against `spec.rs` or marked not proven with its reason. Every proven entry has
a killing mutant (FR-008, SC-004).

**Independent Test**:

- `make verify` reports every P-* harness `SUCCESSFUL`, or the entry's row says why not.
- `make verify-mutants` reports every mutant `killed`, and `verify/MUTANTS.md` records the
  outcomes.

- [X] T031 [US2] Create `verify/PROPERTIES.md` with one row per entry of the data-model §4.1 table: 49 rows, fields per data-model §4.
  - **`statement`:** copied verbatim from the table, including the revised P-ENC-10 and
    P-TRAV-07.
  - **`upstream_source`:** the `s2cell_id.h`/`.cc` or `s2coords.cc` function or doc comment.
  - **`harness`:** `<area>::p_<area>_<nn>`, or the family name.
  - **`status`:**
    - `not-proven (tool-limit: <R6 measurement>)` for P-ENC-05, P-ENC-07, P-ENC-08,
      P-ENC-11 and P-ENC-12. Mark them "outside the SC-003 denominator" (spec SC-003).
    - `not-proven (excluded: all_neighbors)` for P-NBR-04, also outside the denominator.
    - `draft` for everything else.
  - **Status values:** `proven` | `proven-within-bounds` | `not-proven (<reason>)`, quoted
    verbatim from data-model §4. The reason is one of `tool-limit: <detail>`,
    `excluded: <operation>` or `defect D-NNN`. `bound` is required for
    `proven-within-bounds`.
- [X] T032 [P] [US2] Write the P-VAL-01…P-VAL-10 harnesses in `verify/src/val.rs`.
  - Each doc comment's first line is copied from `PROPERTIES.md`.
  - Compare against `spec.rs`, preferably through the `Decomposed` that `any_valid()`
    returns.
  - Loops in `decompose` need `#[kani::unwind(32)]`.
  - Add covers for satisfiability and for the main branches.
- [X] T033 [P] [US2] Write the P-HIER-01…P-HIER-08 harnesses in `verify/src/hier.rs`. P-HIER-03 is `hier::p_hier_03`, the parent-vs-model property.
- [X] T034 [P] [US2] Write the P-RANGE-01…P-RANGE-06 harnesses in `verify/src/range.rs`. P-RANGE-06 (`maximum_tile`) uses `#[kani::unwind(31)]`.
- [X] T035 [P] [US2] Write the P-TRAV-01…P-TRAV-07 harnesses in `verify/src/trav.rs`.
  - The model arithmetic for `advance` and `advance_wrap` uses `i128`, so that steps of
    `i64::MIN` and `i64::MAX` are exact.
  - P-TRAV-07 is the inductive step: for `0 ≤ pos(d) + n < 6·4^level`,
    `advance(n).next() == advance(n + 1)`. When also `pos(d) + n ≥ 1`,
    `advance(n).prev() == advance(n − 1)`.
  - Add a cover for `advance(0) == c`.
- [X] T036 [P] [US2] Write the P-ENC-01…P-ENC-04 harnesses in `verify/src/enc.rs`, next to the T025 families.
  - The Hilbert-model harnesses (P-ENC-01, P-ENC-02) use `#[kani::unwind(31)]`.
  - If one is over the R15 budget, mark it `not-proven (tool-limit)` with its measured time,
    and keep P-ENC-03, the relational round trip.
  - Do not write harnesses for the five tool-limit rows (R6).
- [X] T037 [P] [US2] Write the P-ANC-01 and P-ANC-02 harnesses in `verify/src/anc.rs`.
  - Their domain is Valid × Valid, which never contains `none`. Their doc comments still
    cite U-002, because the upstream comment ("lsb() is non-zero") is the source of that
    domain (FR-018).
- [X] T038 [P] [US2] Write `p_nbr_02` (P-NBR-02, edge-neighbour symmetry) in `verify/src/nbr.rs` (research R12, R15).
  - Draw `k < 4` symbolically, and assert `c ∈ c.edge_neighbors()[k].edge_neighbors()`.
  - Measure it. If it is over the proxy, try `__f<K>` and then `__f<K>` × `k` splits.
  - If nothing fits, the status is `not-proven (tool-limit)` with the measurements. SC-003
    then has no slack left (data-model §4.1).
  - **Outcome (2026-10-01):** out of memory at 4 GiB within 35 s for `__f<K>` and for
    `__f0_k0`. P-NBR-02 is `not-proven (tool-limit)`; the harnesses and mutant M-061 were
    removed.
- [X] T039 [US2] Create `verify/run-mutants.sh` and add the `verify-mutants` target to `Makefile`, depending on `verify-kani-check`, with a `## ` help comment and an optional `M=<id>` (contracts/verify-commands.md).
  - **Setup:** `set -euo pipefail`, the SPDX header as `#` comments, and a `trap` that always
    runs `git worktree remove --force`.
  - **Per patch,** for each `verify/mutants/M-*.patch`, or only `$M`:
    - Create a temporary worktree at `HEAD`. Then `rsync` the caller's `verify/` into it
      with `--exclude target`, so that uncommitted harnesses and patches are tested.
    - Warn if `git status --porcelain core/` is not empty.
    - `git apply` the patch. If it fails, the outcome is `error`.
    - Run the worktree's `verify/run-verify.sh --exact --harness <h>` for each
      `Expect-Fail` harness, with the memory guard. A build failure is `error`.
    - `killed` requires `FAILED` for every expected harness. `OOM` does not count. Anything
      else is `survived`.
  - **Output:** print `M-NNN killed|survived|error` and a summary, and rewrite the outcome
    column of `verify/MUTANTS.md`.
  - **Exit:** 0 only if every mutant is `killed`.
- [X] T040 [P] [US2] Write the VAL and HIER mutants as `verify/mutants/M-010…M-029-<slug>.patch`.
  - Each patch holds one defect in `core/src/s2/cell_id.rs` or `core/src/s2/coords.rs`, made
    in a throwaway worktree.
  - Together they kill every proven P-VAL and P-HIER row. For example, `parent()` using
    `lsb() << 1` instead of `<< 2` targets P-HIER-03.
- [X] T041 [P] [US2] Write the RANGE, TRAV and ANC mutants as `verify/mutants/M-030…M-049-<slug>.patch`, covering every proven P-RANGE, P-TRAV and P-ANC row.
  - Also add `trav::p_trav_04` to `M-001`'s `Expect-Fail`, and `P-TRAV-04` to its `Targets`.
- [X] T042 [P] [US2] Write the ENC and NBR mutants as `verify/mutants/M-050…M-069-<slug>.patch`, covering every proven or within-bounds P-ENC row and every proven P-NBR row.
  - `M-050-from-token-accepts-plus.patch` is the reverse of the T014 fix. Its `Expect-Fail`
    is `enc::p_enc_06__b2` (the input `"+1"`).
  - Include a mutant for P-ENC-10. For example, `FromStr` that also accepts a trailing `/`
    is killed by `p_enc_10__b3`.
  - Include a lookup-table mutant in `coords.rs`, with one `IJ_TO_POS` or `POS_TO_IJ` entry
    swapped.
  - For P-NBR-01 and P-NBR-02, mutate the neighbour offsets in `edge_neighbors`. Check that
    the mutant is not equivalent.
  - There is no FR-016 mutant: the fix sits in an excluded formatter (research R13).
  - Crash proofs need no mutants beyond `M-001`. They are covered by the cover checks
    (FR-008).
- [X] T043 [US2] Run `make verify-mutants` and record the outcomes in `verify/MUTANTS.md`.
  - **Records:** a table of id, defect, targets, expect-fail and outcome.
  - **Survivors:** for every `survived`, strengthen the property or its domain, never weaken
    the mutant. Re-run until every mutant is `killed`.
  - **Errors:** for every `error`, fix the patch.
  - **`PROPERTIES.md`:** fill in the `mutants` column. A `survived` outcome blocks `proven`.
- [X] T044 [US2] Run `make verify`, then finalize `verify/PROPERTIES.md` and `verify/INVENTORY.md`.
  - **`PROPERTIES.md`:** set the final `status`, `bound` and `reason` of every P-* row.
    P-ENC-06, P-ENC-09 and P-ENC-10 are `proven-within-bounds` with the T027 input-size
    bound.
  - **`INVENTORY.md`:**
    - Check the `properties` column.
    - Fill `result_validity` for every row whose operation returns a `CellId` (FR-004). Give
      the `P-*` that proves the result valid, or the upstream-documented exception, for
      example "may return `end(l)`" or "any `u64`, as upstream `FromToken`". Every such row
      must have one.
  - **SC-003 (amended):** every in-scope row has at least one P-* id. At least 90% of the
    entries whose operations are all in scope are `proven`. Expect 40/43, or 39/43 if
    P-NBR-02 is not proven.
  - **SC-004:** every `proven*` row has a killing mutant.
  - **`EVALUATION.md`:** append the harness times to "Measurements" and the US2 hours to the
    "Effort log".

**Checkpoint**: US1 and US2 both pass independently, and SC-003 and SC-004 are met.

---

## Phase 6: User Story 3 — Regression protection for future changes (Priority: P3)

**Goal**: The proofs run in CI on relevant changes and weekly, through the same Makefile
targets. Unrelated PRs and `make test` are unaffected.

**Independent Test**:

- A change that breaks a proven property fails the `verify` job. The log names the harness
  and shows a concrete counterexample.
- Reverting the change makes the job pass again.

- [X] T045 [US3] Create `.github/workflows/verify.yml` (research R8; contracts/verify-commands.md). Follow the style of `.github/workflows/fuzz.yml`.
  - **Header and permissions:** a comment header that explains the two jobs, and
    `permissions: contents: read`.
  - **Triggers:**
    - `pull_request`, and `push` to `master`, with these `paths:`:
      - `core/src/s2/cell_id.rs`, `core/src/s2/coords.rs`, `core/src/r3/**`;
      - `core/src/s2/mod.rs`, `core/Cargo.toml`;
      - `core/tests/data/cpp_cellid_vectors.csv`;
      - `verify/**`, `Makefile`, `.github/workflows/verify.yml`.
    - A weekly `schedule` off the hour (for example `'23 5 * * 1'`), and
      `workflow_dispatch`.
  - **Job `verify`:**
    1. `actions/checkout@v4`, then `dtolnay/rust-toolchain@stable`.
    2. `actions/cache@v4` on `~/.cargo/bin/cargo-kani`, `~/.cargo/bin/kani`, `~/.kani` and
       `~/.rustup/toolchains/nightly-*`, keyed `kani-0.68.0-${{ runner.os }}`.
       `~/.kani/kani-0.68.0/toolchain` is a symlink into the nightly toolchain.
    3. On a cache miss, `cargo install --locked kani-verifier --version 0.68.0 && cargo kani setup`.
    4. A sanity step: `cargo kani --version`, plus a check that
       `~/.kani/kani-0.68.0/toolchain/bin/rustc` exists. Rerun the setup if it doesn't.
    5. A step that prints `nproc` and `MemTotal`, so the evaluation knows the runner.
    6. `make verify` with `VERIFY_JOBS` left unset, so it adapts to the runner (R15), and
       `timeout-minutes: 30`.
  - **Job `slow`:** only when `github.event_name` is `schedule` or `workflow_dispatch`.
    Same setup, then `make verify-slow F=<part>` (the weekly slow tier, spec FR-012/SC-005
    as amended) as a matrix over the parts `0`…`5` and `vertex`, with `VERIFY_JOBS=2` and
    `timeout-minutes: 360`.
  - **Job `mutants`:** only when `github.event_name` is `schedule` or `workflow_dispatch`.
    Same setup, then `make verify-mutants`, with `timeout-minutes: 120`.
- [ ] T046 [US3] Validate `.github/workflows/verify.yml` and measure SC-005.
  - **Static checks:** run `actionlint .github/workflows/verify.yml` if it is installed,
    otherwise review the file carefully. Confirm that `.github/workflows/test.yml` and the
    `make test` target are unchanged (FR-012).
  - **Live check:** **ask the user first**, because it pushes to the remote.
    1. Open a draft PR from a scratch branch with `M-001` applied.
    2. Confirm that the `verify` job fails, names `crash::c_advance`, and prints the playback
       test.
    3. Close the PR and delete the branch.
  - **Measurements** for `verify/EVALUATION.md`:
    - the CI clean-run wall-clock time, with the runner's `nproc` and `MemTotal`;
    - the slowest harness, and any `OOM`;
    - every harness re-checked against the 12-minute CI hard limit. Over it means split,
      otherwise `tool-limit`, then re-run T026's stop rule if `vertex_neighbors` is affected.
  - **If the runner is smaller than 4 vCPU / 16 GB (SC-005 clarification):** also time
    `make verify` on a reference-sized machine. For example run
    `systemd-run --user --scope -p AllowedCPUs=0-3 -p MemoryMax=16G make verify` on the
    workstation, and report both times.
  - **If the reference time exceeds 15 min:** apply the R15 rules to the most expensive
    families, and re-measure.

**Checkpoint**: US3 is complete, and SC-005 is measured.

---

## Phase 7: User Story 4 — Pilot evaluation and recommendation (Priority: P3)

**Goal**: A short, self-contained evaluation that lets a maintainer decide what comes next.

**Independent Test**: A maintainer who did not take part can answer "continue?", "with which
approach?" and "which type next?" from `verify/EVALUATION.md` alone (SC-007).

- [X] T047 [US4] Complete `verify/EVALUATION.md` (FR-014; data-model §8).
  - **Effort:** the "Effort log" total, in person-days per phase.
  - **Run time:** CI wall-clock time per run, the time on the reference machine, the slowest
    harness, and peak memory, from "Measurements".
  - **Limitations:**
    - The tool-limit exclusions and the `all_neighbors` exclusion, with the R6 and R12
      numbers.
    - The parsers proven within an input-size bound.
    - The SC-002 share (48/64, or 47/64).
    - The byte-superset text domain and its one `unsafe` helper.
    - Kani issues met:
      - covers don't fail a run;
      - no counterexample values without playback;
      - `--stop-on-fail` crashes the parser;
      - the SMT2 backend aborted;
      - memory blow-up over symbolic lengths and `core::fmt`.
  - **Findings and defects:**
    - FR-015 (`from_token` `+`): its commit, the C++ and Go references, its vector section,
      and how P-ENC-06 and M-050 reproduce it.
    - FR-016 (`Invalid:` padding): its commit, the C++ reference, its vector section, and
      the regression and differential tests as its evidence.
    - Every `D-NNN`.
  - **`# Panics` documentation added:** the T028 list (SC-011).
  - **Upstream findings:** every `U-NNN`, with its Phase 9 task (SC-012).
  - **Go/no-go:** a clear recommendation, with its rationale.
  - **Assurance level:** whether a follow-up should add deductive (Verus, Creusot) or
    kernel-checked (Aeneas or hax → Lean) proofs, citing `verify/research/rust_formalmethods.md`.
    Discuss whether rewriting the formatters without `core::fmt` would be worth it
    (research R6).
  - **Follow-up types:** a ranked list with one-line reasons. Consider the untrusted-input
    decoders (Constitution II), `CellUnion` normalisation, `s1::ChordAngle` and
    `r1::Interval`.
- [X] T048 [US4] Read `verify/EVALUATION.md` as a non-participant would (SC-007).
  - **Check:** each of the three questions must be answerable from the document alone,
    without opening other files. If one isn't, revise the document.
  - **Links:** every finding links to its row in `INVENTORY.md`, `PROPERTIES.md` or
    `MUTANTS.md`, or to its Phase 9 task.

**Checkpoint**: All four user stories are complete.

---

## Phase 8: Polish & Cross-Cutting Concerns

- [X] T049 [P] Write `verify/TRUSTED_BASE.md` (FR-013; data-model §7).
  - **Tools:** the Kani and CBMC versions, and the solver for each harness. The default is
    CaDiCaL; list every `#[kani::solver]` override.
  - **Models:**
    - Kani's models of `std`: allocation, `String` and `Vec`;
    - rustc MIR semantics;
    - CBMC's bit-precise IEEE-754 semantics for the neighbour wrap path.
  - **The text-domain argument:** research R6, and the one `unsafe` helper.
  - **The spec model:** `spec.rs` itself, validated only by sampling against the C++
    vectors, including the text sections.
  - **Bounds and gaps:**
    - every unwind bound;
    - the parsers' input-size bounds;
    - every domain precondition, with the `U-NNN` that shaped it;
    - every tool-limit exclusion and every `not-proven` row.
- [X] T050 [P] Write `verify/README.md`.
  - **What is proven:** a summary with links to `INVENTORY.md`, `PROPERTIES.md` and
    `MUTANTS.md`.
  - **How to run:** `make verify`, `make verify-harness H=…` and `make verify-mutants`, with
    the Kani install commands.
  - **The memory guard:** `VERIFY_JOBS` and `VERIFY_MEM_PER_HARNESS`. Never run an uncapped
    `cargo kani` (research R15).
  - **How to add a property:**
    - a catalogue row;
    - a harness that follows the doc-comment rule;
    - covers and an unwind bound;
    - a killing mutant;
    - a check against the R15 budget;
    - the upstream findings rule (FR-018).
  - **Rules:** the crate-root naming rule (fact 1), covers enforced by `run-verify.sh`
    (fact 3), and the shared-harness rule.
- [X] T051 [P] Lint and check headers in `verify/`, `core/src/s2/cell_id_tests.rs` and `fuzz/fuzz_targets/parse_cell_id_text.rs`.
  - In `verify/`, run `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`,
    and fix any findings. Every `#[allow]` carries a `reason`.
  - `grep -rn "unsafe" verify/src` shows only `domain::with_text`.
  - Every new source file has an SPDX header:
    `find verify -path verify/target -prune -o \( -name '*.rs' -o -name '*.sh' -o -name '*.toml' \) -print | xargs grep -L SPDX-License-Identifier`
    must print nothing. Also check `core/src/s2/cell_id_tests.rs` and
    `fuzz/fuzz_targets/parse_cell_id_text.rs`.
- [X] T052 Run `specs/001-formal-verify-base-type/quickstart.md` end to end (§0–§10). Fix any step that does not match reality, either in quickstart.md or in the code.
- [X] T053 Cross-check every success criterion, and record the results in an "Acceptance" section at the end of `verify/EVALUATION.md`.
  - **SC-001:** every in-scope operation has a crash proof, and at least 90% cover the
    whole domain.
  - **SC-002:** at least 75% in scope.
  - **SC-003 (amended) and SC-004:** as checked in T044.
  - **SC-005:** under 15 min on the reference runner, and the actual runner's time, from
    T046.
  - **SC-006:** no API change, no new runtime dependencies, and the gate unchanged.
  - **SC-007:** as checked in T048.
  - **SC-008, SC-009 and SC-010:** from Phase 3 and T052.
  - **SC-011:** every proven panic is documented (T028).
  - **SC-012:** every `U-NNN` has a Phase 9 task and an evaluation entry.

---

## Phase 9: Follow-ups (FR-018) — recorded, not implemented in this feature

**Purpose**: Each task here records undefined or wrong behaviour found in the upstream
reference. Resolve each in a **separate change** under FR-010, after this feature.
`/speckit-implement` must not execute these tasks; it only appends new ones, following the
upstream findings rule above.

- [ ] T054 U-001: decide how `CellId::level()` in `core/src/s2/cell_id.rs` should treat ids with 62 or more trailing zero bits, including `0` (research R18).
  - **Upstream:** C++ `s2cell_id.h:599–602`:
    `ABSL_ASSUME(id_ != 0); return kMaxLevel - (absl::countr_zero(id_) >> 1);`.
  - **Inputs:** `0`, `1<<62`, `1<<63`.
  - **Observed:** `level()` of `0` is undefined behaviour, and the other two return −1.
  - **Rust:** the `u8` subtraction underflows. Debug builds panic on the subtraction, and
    release builds panic in `Level::new`. That is documented by T028.
  - **Decision needed:** keep the documented panic, mirror C++ with some result type, or
    reject these ids earlier. Check Go and Java behaviour, and record a `D-NNN` if the
    behaviour should change.
- [ ] T055 U-002: decide what `CellId::common_ancestor_level` in `core/src/s2/cell_id.rs` should return for (`none`, `none`) (research R18).
  - **Upstream:**
    - C++ `s2cell_id.cc:193–207`: `bits = max(id() ^ other.id(), max(lsb(), other.lsb()));`
      then `ABSL_ASSUME(bits != 0); // Because lsb() is non-zero.`
    - Go `geo/s2/cellid.go:741–747`: `CommonAncestorLevel`.
  - **Inputs:** `CellId::none()` with `CellId::none()`. `bits` is 0 only for this pair.
  - **Observed:** C++ has undefined behaviour. Go returns `(30, true)`, because
    `bits.Len64(0) − 1 = −1` passes its `> 60` check. That is well-defined but wrong.
  - **Rust:** `0u64.ilog2()` panics. That is documented by T028.
  - **Decision needed:** keep the documented panic, or return `None` (the Rust analogue of
    "no common ancestor"). Check Java behaviour, and record a `D-NNN` if the behaviour
    should change.
- [ ] T056 D-001: decide whether `s2::from_face_ij` in `core/src/s2/cell_id.rs` should
  match upstream for `i` or `j` outside `[0, 2^30)` (FR-003, FR-010; found by
  `crash::c_from_face_ij`, `verify/EVALUATION.md` "Defects found").
  - **Upstream:** C++ `S2CellId::FromFaceIJ` (`s2cell_id.cc:267`, oracle `8d5c2a8`) masks each
    4-bit chunk of `i` and `j` and computes `n * 2 + 1` in `uint64_t`, which wraps. The
    range is documented in `s2coords.h:44–45`; outside it the result is meaningless but
    well-defined.
  - **Inputs:** for example `i = −2147483641`: the top chunk sets bits 60–63 of `n`.
  - **Rust:** release builds wrap like C++; debug builds panic on `n * 2` (documented by the
    T028 `# Panics` section).
  - **Decision needed:** use `wrapping_mul`/`wrapping_add` to match upstream in every build,
    or keep the debug panic as an accepted, documented divergence. Either way, as its own
    change with a regression test (FR-010, Constitution III).

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: T001 → T002. T003 is independent. T004 needs T003.
- **Foundational (Phase 2)**: needs T001–T004.
  - T005 → T006 → T007, because they edit the same file.
  - T008 needs T007, and T009 needs T005.
  - T010 can start any time.
  - **Phase 2 blocks all user stories.**
- **Phase 3**: independent of Phases 1 and 2, except that T020 needs T007.
  - T011 → T012 → T013 → T014 (the FR-015 commit) → T015 → T016 → T017 (the FR-016 commit).
    They are serial because they share `cell_id.rs`, `cell_id_tests.rs`, the generator and
    the CSV.
  - T018 and T019 (the FR-017 commit) can run in parallel with the rest.
  - T020 needs T007, T013 and T016.
  - **T014 must land before T025.**
- **US1 (Phase 4)**: needs Phase 2 and T014.
  - T021 → T022 → T023 → T024, because they edit `crash.rs`.
  - T025 (`enc.rs`) and T026 (`nbr.rs`) can run in parallel with them.
  - Then T027 → T028 → T029 → T030.
- **US2 (Phase 5)**: needs Phase 2, plus T025 and T026 for the shared families.
  - T031, then T032–T038 in parallel, then T039, then T040–T042 in parallel, then T043,
    then T044.
  - T041 edits `M-001`, so it needs T029.
  - T042 needs T014, because `M-050` reverses it.
- **US3 (Phase 6)**: needs T004, and T039 for the `mutants` job. It is most useful after US1.
- **US4 (Phase 7)**: needs the results of US1–US3.
- **Polish (Phase 8)**: needs all the stories.
- **Follow-ups (Phase 9)**: not executed in this feature. Entries are appended whenever the
  upstream findings rule fires.

### User Story Dependencies

- **US1 (P1)**: depends only on Foundational and the FR-015 fix. It is the MVP.
- **US2 (P2)**: depends on Foundational and on US1's shared parser and neighbour families
  (T025, T026). Its own harnesses can be tested independently of US1's crash harnesses.
- **US3 (P3)**: needs `make verify`. A CI lane with only US1's harnesses is already useful.
- **US4 (P3)**: written last, from everything else.

### Parallel Opportunities

- **Setup:** T001 and T003.
- **Foundational:** T008, T009 and T010, once their inputs exist.
- **Phase 3:** T018 and T019 run alongside the FR-015 and FR-016 chain.
- **US1:** `crash.rs` (T021–T024), `enc.rs` (T025) and `nbr.rs` (T026).
- **US2:** the seven property files (T032–T038), and the three mutant sets (T040–T042), whose
  ID ranges don't overlap.
- **Polish:** T049, T050 and T051.

Running Kani in parallel is limited by memory, not by files. Keep at most three capped runs at
once (see "Memory safety").

---

## Parallel Example: User Story 1

```bash
# After Phase 2 and T014, three files at once:
Task: "T021–T024 integer crash harnesses in verify/src/crash.rs"
Task: "T025 parser families p_enc_06/09/10__b<L> in verify/src/enc.rs"
Task: "T026 p_nbr_01 / p_nbr_03__q* in verify/src/nbr.rs"
```

## Parallel Example: User Story 2

```bash
# After T031, the seven property files at once:
Task: "P-VAL-01…10 in verify/src/val.rs"
Task: "P-HIER-01…08 in verify/src/hier.rs"
Task: "P-RANGE-01…06 in verify/src/range.rs"
Task: "P-TRAV-01…07 in verify/src/trav.rs"
Task: "P-ENC-01…04 in verify/src/enc.rs"
Task: "P-ANC-01…02 in verify/src/anc.rs"
Task: "P-NBR-02 in verify/src/nbr.rs"

# After T039, the mutant sets at once (disjoint ID ranges):
Task: "M-010…M-029 for VAL/HIER"
Task: "M-030…M-049 for RANGE/TRAV/ANC"
Task: "M-050…M-069 for ENC/NBR"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1 (Setup), including the memory guard and counterexample printing, and Phase 2
   (Foundational).
2. Phase 3: at least T011–T014, the FR-015 commit that P-ENC-06 relies on.
3. Phase 4 (US1): crash proofs for all 48 in-scope operations, and the `# Panics` docs. Stop
   at T026 if `vertex_neighbors` cannot fit the budget.
4. **Stop and validate**: `make verify` is green, and M-001 is caught with a concrete
   counterexample.

### Incremental Delivery

1. Setup + Foundational → the model, the domains and the final inventory.
2. Phase 3 → the FR-015 and FR-016 fixes with their C++ vectors, and FR-017: three commits.
3. US1 → crash-freedom proofs and the `# Panics` docs (MVP).
4. US2 → functional properties and the mutant campaign.
5. US3 → the CI lane. It can come earlier, right after US1, to protect the MVP.
6. US4 → the evaluation. Polish → trusted base, README and acceptance.
7. Phase 9 follow-ups → separate changes after the pilot.

---

## Notes

- [P] tasks touch different files and do not depend on unfinished tasks.
- Do not stub `s2rst` code, and never pass `--no-unwinding-checks` (contracts).
- Generate mutant patches in throwaway worktrees. Never leave a mutant applied in the main
  tree.
- Every Kani run is memory-capped (see "Memory safety").
- Apply the upstream findings rule (FR-018) before marking any affected proof complete.
- Commit after each task or logical group. FR-015, FR-016, FR-017 and the `# Panics` docs are
  each a commit of their own.
