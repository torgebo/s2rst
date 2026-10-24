# Data Model: Formal Verification of `CellUnion`

This feature adds no runtime data. The entities below are the **verification artifacts**
(spec Key Entities) and the **reference model** of a cell union that the proofs are written
against. The schemas extend 001's (`specs/001-formal-verify-base-type/data-model.md`).
Where a field is unchanged, this document says so instead of repeating it.

## 1. Reference model of a cell union (`verify/src/union_spec.rs`)

The model is written from upstream `s2cell_union.h`/`.cc` and from the mathematical meaning
of a union, the set of leaf cells it covers. It is built on 001's cell model
(`verify/src/spec.rs`: `Decomposed`, `range`, `contains`, `parent`, `span`) and never
calls `s2rst::s2::CellUnion` (FR-006). Like `spec.rs`, it is plain Rust, compiled outside
Kani, and sanity-tested against C++ vectors (research R3, R13).

A union in the model is a sequence `U = [u_0, …, u_{n−1}]` of raw `u64` identifiers. When
every `u_i` is valid, `d_i = decompose(u_i)` and `(lo_i, hi_i) = range(d_i)` is its
inclusive leaf-index interval.

| Concept | Definition | Upstream source |
|---------|------------|-----------------|
| **covers(U, x)** | For a leaf-index `x`: some `i` has `lo_i ≤ x ≤ hi_i` | the meaning of a region; `S2CellUnion` class comment |
| **valid_union(U)** | Every `u_i` is valid, and `hi_{i−1} < lo_i` for every `i ≥ 1` (sorted by leaf range and pairwise disjoint) | `IsValid()` (`s2cell_union.h:197–199`) |
| **sibling_group(a, b, c, d)** | Same face and level `ℓ ≥ 1`, the same parent, and child positions 0, 1, 2, 3 in that order | `AreSiblings`, as used by `IsNormalized` |
| **normalized_union(U)** | `valid_union(U)`, and no four consecutive members form a `sibling_group` | `IsNormalized()` (`s2cell_union.h:201–205`) |
| **leaf_count(U)** | `Σ span(d_i.level)`, which is `|{x : covers(U, x)}|` when `valid_union(U)`, and at most `6·4^30` | `LeafCellsCovered()` (`s2cell_union.h:303–305`) |
| **contains_id(U, c)** | Some `i` has `contains(d_i, c)` | `Contains(S2CellId)`, including its caveat for non-normalised unions |
| **intersects_id(U, c)** | Some `i` has `range(d_i)` ∩ `range(c)` ≠ ∅ | `Intersects(S2CellId)` |
| **denorm_level(ℓ, m, k)** | `min(30, max(m, ℓ) + (30 − (max(m, ℓ) − m)) mod k)` for `k > 1`; `max(m, ℓ)` for `k = 1` | `Denormalize` (`s2cell_union.cc:215–223`) |
| **denormalize(U, m, k)** | Each `u_i` replaced in order by its descendants at `denorm_level(level_i, m, k)`, or kept when that level equals its own | `Denormalize` |
| **encoding(U)** | Byte `1` (version), then `n` as 8 bytes little-endian, then each `u_i` as 8 bytes little-endian; length `9 + 8n` | `Encode()` |
| **decode(B)** | `Ok(U)` exactly when `B` starts with a version ≤ 1, then `n ≤ 1 000 000`, then at least `8n` bytes whose `n` ids form a `valid_union`; `U` is those ids in order. `Err` otherwise. Trailing bytes are ignored | `Decode()` plus the FR-017 deviation (research R11) |

**Set semantics.** The functional properties quantify over leaves with a symbolic leaf
`x`. A harness that asserts `covers(A, x) ⇔ covers(B, x)` for an unconstrained `x` proves
the equality for every leaf (research R3). The operation results are checked against
these definitions:

- `union`: `covers(A, x) ∨ covers(B, x)`;
- `intersection`: `covers(A, x) ∧ covers(B, x)`;
- `intersection_with_cell_id`: `covers(U, x) ∧ x ∈ range(c)`;
- `difference`: `covers(A, x) ∧ ¬covers(B, x)`;
- `from_begin_end(b, e)`: `li(b) ≤ x < li(e)`, where `li` is 001's leaf index;
- `from_min_max(b, e)`: `li(b) ≤ x ≤ li(e)`;
- `whole_sphere`: every `x`.

**Validation rules**:
- `union_spec.rs` has `cargo test` sanity tests against the new C++ vector sections (§10):
  normalisation, validity, leaf counts, denormalisation, set operations, range
  constructors and decoding.
- The model is independent of the implementation. Every model function works on
  `Decomposed` values from `spec.rs` and leaf indices, never on `CellId` methods.

## 2. Input domain classes

The cell, level and face domains are 001's (`Valid`, `ValidOrEnd`, `Level`, and so on).
This feature adds union and byte domains. Each harness builds a union of exactly `k`
cells from `k` model decompositions, and a family `__n0 … __nN` covers every length up to
`N`. A symbolic length was two to three orders of magnitude slower in the probe (research
R0, R4).

| Domain | Members | Used for |
|--------|---------|----------|
| `Seq(N)` | every sequence of at most `N` raw `u64` values | `is_valid`, `is_normalized`, `from_verbatim`, `num_cells`, `cell_ids`, `is_empty`, slice and iterator access, `encode` |
| `ValidSeq(N)` | every sequence of at most `N` valid cells, in any order, possibly overlapping or repeated | `normalize`, `from_cell_ids`, `FromIterator` (upstream `Normalize` DCHECKs `id.is_valid()`) |
| `ValidUnion(N)` | `valid_union(U)` and `|U| ≤ N` | every operation on an existing union whose upstream contract rests on the class invariant (`FromVerbatim` REQUIRES `IsValid()`): the cell and union queries, `leaf_cells_covered`, `union`, `intersection`, `intersection_with_cell_id` and `denormalize` (upstream `Denormalize` states no normalisation precondition) |
| `NormUnion(N)` | `normalized_union(U)` and `|U| ≤ N` | the "normalised when the inputs are" claims (FR-004) and P-UNORM-03. `difference` would use it for its FR-004 claim, but it is a tool-limit exclusion |
| `Leaf` / `LeafOrEnd` | valid level-30 cells, and `end(30)` as well for `LeafOrEnd` | the documented domains of `from_begin_end` and `from_min_max` (upstream REQUIRES). These are tool-limit exclusions, so no proof uses the domain unless the implementation-time measurement of `from_range` (tasks T021) brings it into scope; the inventory records it, and the R17 property tests sample it |
| `LevelMod` | `1..=3` | `denormalize`'s `level_mod`, by the upstream `DCHECK` and the library's `debug_assert!` |
| `Bytes(L)` | every byte string of length at most `L` | `S2Decode for CellUnion` |

**Preconditions** cite their source as in 001: rustdoc, `debug_assert!`, or an upstream
`DCHECK`/`REQUIRES`. Each harness assumes exactly its operation's documented
preconditions. A domain narrower than this, for any reason other than a union-size,
byte-length or loop bound, makes the operation a tool-limit exclusion (FR-007).

## 3. Operation inventory entry (`verify/cell_union/INVENTORY.md`)

The fields are 001's §3, with three additions and one change:

| Field | Content |
|-------|---------|
| `union_domain` | **New.** `Seq` \| `ValidSeq` \| `ValidUnion` \| `NormUnion` \| `—` (no union argument) |
| `bounds` | **New.** The union-size bound `N` per union argument, the byte bound `L` for decoding, and any loop bound. Each comes with the uncovered part, the documented behaviour it leaves unreached (for example the sibling merge), and the tests that exercise it (FR-007) |
| `crash_harness` | `cu_crash::c_<op>`, or a split family `cu_crash::c_<op>__n<k>` / `__b<L>` |
| `kind` | **Changed values.** `constructor` \| `query` \| `set-op` \| `transform` \| `measure` \| `encoding` \| `access` \| `trait` |

**The inventory (42 entries)** is 29 inherent methods plus 13 hand-written trait methods:
`Deref::deref`, `Display::fmt`, `FromIterator::from_iter`, two `IntoIterator::into_iter`
(owned and borrowed), six `Region` methods, `S2Encode::encode` and `S2Decode::decode`. The
projected status, from research R0:

- **`float-geometry` (9 entries)**: `contains_point`, `expand_by_radius`,
  `average_based_area`, `approx_area`, `exact_area`, and the `Region` methods
  `cap_bound`, `rect_bound`, `cell_union_bound` and `contains_point`.
- **`excluded: depends on CellId::all_neighbors` (1 entry)**: `expand_at_level`.
- **`tool-limit` (5 entries)**:
  - `Display::fmt`, through `CellId::to_token`, as in 001;
  - `difference`, `from_begin_end`, `from_min_max` and `from_range`, measured over the
    budget at one cell (research R0, R5). Each row carries its R0 measurement and names
    the R17 property tests and the C++ vector section that cover it.
- **In scope (27 entries)**: everything else, including `Region::contains_cell` and
  `Region::intersects_cell`, which only read `cell.id()`.

That is 64% in scope; SC-002, as amended on 2026-10-02, requires 60%. `S2Decode` is in
scope on the condition stated in SC-001. If it falls back to a tool limit, the share is
26/42 = 62%.

**Validation rules**: as in 001, plus:
- every in-scope row has `bounds` filled;
- every row whose operation returns a `CellUnion` has `result_validity` naming the `P-*`
  that proves the invariant FR-004 requires for it.

## 4. Property (`verify/cell_union/PROPERTIES.md`)

The fields are 001's §4. `id` is `P-U<AREA>-<NN>` with AREA ∈ `ACC`, `INV`, `NORM`, `MEM`,
`SET`, `RANGE`, `DEN`, `MEAS`, `ENC` (the FR-005 areas, plus `ACC` for the accessors so that
SC-003 holds for every operation). `bound` is required for every `proven-within-bounds`
entry. Harnesses are `cu_<area>::p_u<area>_<nn>` (contracts/harness-conventions.md).

### 4.1 Initial catalogue (27 entries)

| ID | Statement | Upstream source | Domain |
|----|-----------|-----------------|--------|
| P-UACC-01 | `new()` is empty (and so is the derived `Default`, which is outside the inventory); `from_verbatim(v)` stores exactly `v`; `cell_ids`, `Deref`, both `into_iter`s yield exactly the stored ids in order; `num_cells` is their count and `is_empty` holds iff it is 0 | `cell_ids()`, `size()`, `empty()`, `FromVerbatim` | `Seq(N)` |
| P-UINV-01 | `is_valid(U) ⇔ valid_union(U)` | `IsValid()` | `Seq(N)` |
| P-UINV-02 | `is_normalized(U) ⇔ normalized_union(U)` | `IsNormalized()` | `Seq(N)` |
| P-UNORM-01 | `normalize(S)` (and `from_cell_ids`, `FromIterator`) returns a `normalized_union` | `Normalize()` doc | `ValidSeq(N)` |
| P-UNORM-02 | `covers(normalize(S), x) ⇔ covers(S, x)` for every leaf `x` | `Normalize()` doc | `ValidSeq(N)` |
| P-UNORM-03 | `normalize(U) = U` for every normalised `U` (so normalising twice changes nothing) | `Normalize()` | `NormUnion(N)` |
| P-UNORM-04 | Swapping any two members of `S` does not change `normalize(S)`; transpositions generate every permutation, so the order of the input never matters | `Normalize()` sorts first | `ValidSeq(N)` |
| P-UMEM-01 | `contains_cell_id(U, c) ⇔ contains_id(U, c)` | `Contains(S2CellId)` | `ValidUnion(N)`, `c ∈ Valid` |
| P-UMEM-02 | `intersects_cell_id(U, c) ⇔ intersects_id(U, c)` | `Intersects(S2CellId)` | `ValidUnion(N)`, `c ∈ Valid` |
| P-UMEM-03 | If `contains_cell_id(U, c)` then `covers(U, x)` for every leaf `x` of `c` | `Contains` | `ValidUnion(N)`, `c ∈ Valid` |
| P-UMEM-04 | `contains_union(A, B) ⇔ ∀ b ∈ B: contains_id(A, b)`, and `intersects_union(A, B) ⇔ ∃ b ∈ B: intersects_id(A, b)`, both stated in the model (this is upstream's caveat: a non-normalised `A` does not contain a parent that only its four children cover) | `Contains/Intersects(S2CellUnion)` | `ValidUnion(N)` × `ValidUnion(M)` |
| P-UMEM-05 | `Region::contains_cell` and `Region::intersects_cell` equal the cell-id queries on `cell.id()` | `Contains/MayIntersect(S2Cell)` | `ValidUnion(N)`, `c ∈ Valid` |
| P-USET-01 | `union(A, B)` is normalised and `covers(·, x) ⇔ covers(A, x) ∨ covers(B, x)` | `Union()` | `ValidUnion(N)` × `ValidUnion(M)` |
| P-USET-02 | `intersection(A, B)` covers exactly `covers(A, x) ∧ covers(B, x)`; it is valid, and normalised when both inputs are | `Intersection()`, `GetIntersection` | `ValidUnion(N)` × `ValidUnion(M)` |
| P-USET-03 | `intersection_with_cell_id(U, c)` covers exactly `covers(U, x) ∧ x ∈ range(c)`; it is normalised when `U` is | `Intersection(S2CellId)` | `ValidUnion(N)`, `c ∈ Valid` |
| P-USET-04 | `difference(A, B)` covers exactly `covers(A, x) ∧ ¬covers(B, x)`; it is normalised when `A` is | `Difference()` | `ValidUnion(N)` × `ValidUnion(M)` |
| P-URANGE-01 | `from_begin_end(b, e)` covers exactly `li(b) ≤ x < li(e)` and is normalised | `FromBeginEnd` | `LeafOrEnd` × `LeafOrEnd`, `b ≤ e` |
| P-URANGE-02 | `from_min_max(b, e)` covers exactly `li(b) ≤ x ≤ li(e)` and is normalised | `FromMinMax` | `Leaf` × `Leaf`, `b ≤ e` |
| P-URANGE-03 | `from_range(b, e)` covers exactly `li(b) ≤ x < li(e)` | mathematical (no upstream counterpart) | `LeafOrEnd` × `LeafOrEnd`, `b ≤ e` |
| P-URANGE-04 | `whole_sphere()` is the six face cells in order and covers every leaf | `WholeSphere()` | — |
| P-UDEN-01 | `denormalize(U, m, k)` equals the model `denormalize(U, m, k)`, the C++ algorithm with its clamp (FR-015); the result is valid (FR-004) | `Denormalize` | `ValidUnion(N)` (wider than FR-004's normalised input), `m ∈ Level`, `k ∈ LevelMod` |
| P-UDEN-02 | Every member of `denormalize(U, m, k)` has level ≥ `m` and `(level − m) mod k = 0`, unless its level is 30; it covers exactly what `U` covers | `Denormalize` doc | as P-UDEN-01 |
| P-UMEAS-01 | `leaf_cells_covered(U) = leaf_count(U)`, which is at most `6·4^30` and never overflows (FR-016) | `LeafCellsCovered()` | `ValidUnion(N)` |
| P-UENC-01 | `encode(U)` writes exactly `encoding(U)` | `Encode()` | `Seq(N)` |
| P-UENC-02 | `decode(encode(U)) = U` for every valid union, normalised or not (FR-017) | `Decode()`, `Encode()` | `ValidUnion(N)` |
| P-UENC-03 | `decode(B)` agrees with the model `decode(B)`: it accepts exactly the encodings of valid unions, returns their ids unchanged, and rejects everything else | `Decode()` plus the FR-017 deviation | `Bytes(L)` |
| P-UENC-04 | `S2Decode for CellUnion` never panics or hangs on any input (also the crash proof) | Constitution II | `Bytes(L)` |

**Expected status** (research R0, R4):
- **Not proven (excluded operation):** P-USET-04 (`difference`) and P-URANGE-01/02/03
  (the range constructors). They stay in the catalogue with that status, and each names
  the R17 property test and the C++ vector section that check it instead.
- **Proven within bounds:** every other entry except P-URANGE-04, at the bounds of R4.
  - P-UNORM-01/02/03/04 hold for inputs of at most 2 cells. Their bound note names the
    sibling merge as unreached.
- **Proven, whole domain:** P-URANGE-04 (`whole_sphere`, a constant).

**Counting for SC-003**: 23 entries exercise only in-scope operations. 90% means at
least 21 of them must be proven, or proven within bounds that meet SC-001. P-UENC-02/03/04
call the decoder, so they depend on the decoding condition in SC-001. If decoding becomes
a tool limit, all three leave the count, and the floor is 18 of 20. P-UENC-02 may still be
proven over encoded unions, but it then no longer counts toward SC-003. P-UENC-04
duplicates the decode crash proof on purpose: the decoder is the one untrusted-input path.

## 5. Mutant (`verify/mutants/M-1NN-<slug>.patch`)

The fields are 001's §5, with these changes:
- `id` is `M-1NN`. The 100 range keeps `CellUnion` mutants apart from 001's.
- `patch` is against `core/src/s2/cell_union.rs` or `core/src/s2/encoding.rs`.
- `targets`: the only crash target is `C-CellUnion::leaf_cells_covered`, on `M-100`, the US1
  scenario 4 overflow demonstration (research R7).

**Rules**: as in 001. `M-150` is the reverse of the FR-015 fix (P-UDEN-01), and `M-151` is
the reverse of the FR-017 fix (P-UENC-02).

## 6. Known defect entry (`verify/known-defects.txt`)

Unchanged. FR-015 and FR-017 are fixed before their proofs run, so they never appear in
the file.

## 7. Trusted base statement (`verify/TRUSTED_BASE.md`, new `CellUnion` section)

In addition to 001's entries:
- **Reliance on 001.** Wherever a property statement calls a `CellId` method of the
  implementation instead of the model, it relies on 001's proof that the two agree. Each
  such method is listed with its 001 property:
  - `range_min` and `range_max`: P-RANGE-01;
  - `is_valid`: P-VAL-03;
  - `face`, `level` and `pos`, to decompose outputs: P-VAL-06;
  - `contains` and `intersects`: P-RANGE-02 and P-RANGE-03;
  - `parent` and `children`: P-HIER-03 and P-HIER-01.

  The `CellId` code still runs inside every `CellUnion` proof. Nothing is stubbed.
- **The standard library** (`sort_unstable`, `Vec`, `partition_point`, `io::Read` for
  `&[u8]`) is executed by the checker like any other code. Only Kani's allocation model and
  its `std` build are trusted.
- **Bounds.** Every `N`, `L` and loop bound, with its uncovered part and the tests that
  exercise it, comes from research R4 and R5.
- **Written arguments.** Two results are extended beyond the bounds by an argument instead
  of a proof:
  - the leaf-count bound for valid unions of any size (FR-016);
  - permutation independence from transpositions (P-UNORM-04).

## 8. Evaluation (`verify/cell_union/EVALUATION.md`)

The sections are 001's §8, plus three:
- **Bounds reached**: per operation, the `N`, `L` and loop bounds proven, and what they
  cost.
- **Comparison with 001**: effort, CPU time, defects found per day, and how the
  whole-domain share changed.
- **Collections verdict**: whether bounded proofs over collections were worth their cost
  (SC-007).

## 9. Upstream finding (`U-NNN`)

Unchanged from 001 §9, numbered from `U-003`. One candidate is known before
implementation (research R12):
- **U-003**: C++ `S2CellUnion::Decode` (`s2cell_union.cc:544–562`), with
  `S2CellId::Decode` (`s2cell_id.cc:261–265`). It accepts identifiers that are invalid,
  unsorted or overlapping, so it returns a union for which `IsValid()` is false, although
  `FromVerbatim` REQUIRES `IsValid()`. Its follow-up task is created before the decode
  proofs are marked complete.

## 10. C++ reference vectors (`core/tests/data/cpp_cellunion_vectors.csv`)

These are generated by the new `tools/refgen-cpp/generate_cellunion.cc` (research R13).
The format follows `cpp_cellid_vectors.csv`: `#` section headers, then one CSV row per
case. Ids are hexadecimal `u64` values, and a list of ids is `;`-separated.

| Section | Row | Checks |
|---------|-----|--------|
| `CELLUNION_NORMALIZE` | `input ids, normalised ids` | `normalize`; model P-UNORM-01/02 |
| `CELLUNION_DENORMALIZE` | `ids, min_level, level_mod, output ids` | FR-015 differential test; model `denormalize` |
| `CELLUNION_SETOPS` | `a ids, b ids, union, intersection, difference, a contains b, a intersects b` (the last two as 0/1) | set operations and union queries; model `covers`, `contains_id`, `intersects_id` |
| `CELLUNION_FROM_BEGIN_END` | `begin, end, ids` | range constructors; model |
| `CELLUNION_LEAF_CELLS` | `ids, count` | `leaf_cells_covered`; model `leaf_count` |
| `CELLUNION_ENCODE` | `ids, encoded bytes (hex)` | `encode`; model `encoding` |
| `CELLUNION_DECODE` | `bytes (hex), ok, valid, ids` (`ok` is C++ `Decode`'s result and `valid` the decoded union's `IsValid()`, both 0/1) | FR-017 differential test: rows with `ok = 1, valid = 1` must decode to the same ids, and every other row must be rejected. The rows with `ok = 1, valid = 0` record U-003 |

The inputs include, for each section, every case FR-015 and FR-017 name: levels 29 and 30
with `min_level` 1 and `level_mod` 2 and 3, four sibling cells, and unsorted, duplicate,
overlapping and invalid ids.
