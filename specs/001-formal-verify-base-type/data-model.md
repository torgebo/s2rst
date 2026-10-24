# Data Model: Formal Verification of `CellId`

This feature adds no runtime data. The "entities" below are the **verification artifacts**
(spec Key Entities) and the **reference model** of a cell identifier that the proofs are
written against. Field lists describe content, not storage. Every artifact is plain Markdown
or Rust in the repository.

## 1. Reference model of a cell identifier (`spec` module)

This is the mathematical model from upstream `s2cell_id.h` and `s2coords_internal.h`. It is
written independently of `core/src/s2/cell_id.rs` (FR-006), and every property is stated in
terms of it.

| Concept | Definition |
|---------|------------|
| **Bit layout** | 64 bits: `face` (3 bits) · `path` (2 bits per level, `level` digits) · marker `1` · `0` padding |
| **Decomposed** | `{ face: 0..=5, level: 0..=30, path: 0..4^level }`, with `path` read as base-4 digits (child positions), most significant first |
| **compose(d)** | `face << 61 \| path << (61 − 2·level) \| 1 << (60 − 2·level)` |
| **decompose(x)** | The inverse of `compose` on its image, found by **searching** the 31 levels for the marker bit. It deliberately avoids the implementation's `trailing_zeros`/lsb tricks. Returns *none* for any `x` not in the image, which is the definition of *invalid* |
| **valid(x)** | `decompose(x)` is some value |
| **leaf-index(d)** | Position of the cell's first leaf along the whole curve: `face·4^30 + path·4^(30−level)` |
| **span(d)** | Number of leaves covered: `4^(30−level)` |
| **position(d)** | Index of the cell along the curve at its own level: `face·4^level + path` |
| **from_position(l, p)** | `compose({p div 4^l, l, p mod 4^l})` for `p < 6·4^l`; `end(l)` for `p = 6·4^l` |
| **end(l)** | One past the last cell of face 5 at level `l`: `6 << 61 \| 1 << (60 − 2l)` |

Relations used by properties are defined **on the model**, not with implementation bit tricks:

- **parent(d)**: `{face, level−1, path div 4}`, defined for `level ≥ 1`.
- **child(d, k)**: `{face, level+1, 4·path + k}`, defined for `level ≤ 29` and `k ∈ 0..4`.
- **ancestor(d, l)**: `{face, l, path div 4^(level−l)}`, defined for `l ≤ level`.
- **contains(a, b)**: `a.face = b.face`, `a.level ≤ b.level`, and `ancestor(b, a.level) = a`.
  This is the prefix relation on paths.
- **range(d)**: the leaf interval `[leaf-index(d), leaf-index(d) + span(d) − 1]`.
- **common ancestor level(a, b)**: the largest `l ≤ min(levels)` with
  `ancestor(a, l) = ancestor(b, l)`. It is none on different faces.
- **Hilbert decoding** (`face_ij_orientation`, `leaf_from_face_ij`, `center_si_ti`): one level
  at a time, with `kPosToIJ`, `kIJtoPos` and `kPosToOrientation` copied from upstream
  `s2coords_internal.h`. A non-leaf cell decodes as its path extended by digit 2 and then 0s,
  as in upstream `ToFaceIJOrientation`.
- **Text**, following upstream:
  - **`token(x)`:** C++ `ToToken`, i.e. lowercase hex with trailing zeros stripped, and `"X"`
    for 0.
  - **`parse_token(s)`:** C++ `FromToken`, i.e. 0 unless `s` is 1–16 hex digits.
  - **`debug_string(x)`:** C++ `ToString`. A valid id prints as `face/digits`. Any other id
    prints as `Invalid: ` plus 16 zero-padded lowercase hex digits (FR-016).
  - **`parse_debug_string(s)`:** C++ `FromDebugString`.

**Validation rules**:

- `decompose ∘ compose = id` on every well-formed `Decomposed`.
- `compose ∘ decompose = id` on every valid `x`.
- Both identities are proven exhaustively (P-VAL-01, P-VAL-02). They are also sanity-tested
  with ordinary `cargo test` against the committed C++ vectors in
  `core/tests/data/cpp_cellid_vectors.csv`: `CELLID_CHILDREN`, `CELLID_RANGE`,
  `CELLID_CONTAINS` and `CELLID_FACE_IJ` (research R3). The text model (`parse_token`,
  `parse_debug_string`, `debug_string`) is checked against `CELLID_FROM_TOKEN`,
  `CELLID_FROM_DEBUG_STRING` and `CELLID_TO_STRING` (research R16).

## 2. Input domain classes

Each inventory entry and each property declares exactly one domain class, plus any explicit
preconditions (spec edge cases; FR-003). A domain is the full range of the argument type,
narrowed only by a documented precondition.

| Domain | Members | Used for |
|--------|---------|----------|
| `Any` | every `u64` bit pattern | `is_valid`, `id`, `none`, `sentinel`, `From<u64>`, `From<CellId> for u64`, `Default`. Also the formatters `to_token`, `to_debug_string` and `Display`, which are excluded as `tool-limit` (research R6) |
| `Valid` | `valid(x)` | Most hierarchy and navigation operations |
| `ValidOrEnd` | `Valid` ∪ `{ end(l) : l ∈ 0..=30 }` ∪ `{ sentinel }` | Range, containment and traversal operations that upstream iteration calls on past-the-end values (per-op justification in the inventory) |
| `Level` | `0..=30` | Every `impl Into<Level>` argument, taken as a `Level` value. A raw `u8 > 30` panics in the documented conversion (`From<u8> for Level`), recorded per operation in the inventory |
| `Face` | `0..=5` | Face arguments documented as a face number |
| `I32` / `I64` / `U64` | the whole integer type | `steps` (`advance*`), and `pos` (narrowed to `< 2^61` by `from_face_pos_level`'s rustdoc) |
| `IJ` | `[0, 2^30 − 1]` for each of `i` and `j` | `from_face_ij`. Upstream documents this range (`s2coords.h:44–45`, cited by `s2cell_id.h:484–486`), so FR-003 allows the narrowing. The crash proof first tries the whole `i32` range, the stronger claim. If Rust panics outside the documented range, that is an undocumented panic (FR-003): record it and document it (research R17) |
| `Text(L)` | every byte string of exactly `L` bytes, passed as a `&str` by `domain::with_text::<L>`. This is a superset of every `&str` of `L` bytes. The parsers read only bytes, so it covers them all (research R6) | `from_token` (`__b0`…`__b17`), `from_debug_string` and `FromStr` (`__b0`…`__b33`). Longer texts are the input-size remainder, so the status is `proven-within-bounds` (FR-007) |

**Preconditions** are named predicates such as `not_face`, `not_leaf`, `level ≤ self.level`,
`level ≥ self.level`, `level < self.level` and `pos < 2^61`. Each cites its source: rustdoc,
`debug_assert!`, or an upstream `DCHECK`/`ASSUME`/doc comment. A harness assumes exactly the
documented precondition and nothing more (US1 scenario 2).

A harness whose domain is narrower than this for any reason other than a loop bound or an
input-size bound (for example ASCII-only text) does not count as a proof. Its operation is
excluded as `tool-limit` (FR-007).

## 3. Operation Inventory entry (`verify/INVENTORY.md`)

The inventory also records each operation's **crash-freedom proof** (`C-<op>`). Crash proofs
are not catalogue properties (spec Key Entities; clarification 2026-10-01).

| Field | Content |
|-------|---------|
| `name` | Fully qualified operation, e.g. `CellId::parent`, `impl FromStr for CellId`, `s2::lsb_for_level` |
| `kind` | `constructor` \| `query` \| `navigation` \| `encoding` \| `neighbour` \| `trait` \| `free-fn` |
| `status` | `in-scope` \| `excluded` |
| `exclusion_reason` | Required when `excluded`. One of `float-geometry`, `float-geometry: crash-freedom intractable` (the FR-002 neighbour clause), or `tool-limit: <detail>`. A tool-limit reason covers a domain restriction other than a loop bound, or a harness over the R15 budget. Every reason other than plain `float-geometry` must also appear in the evaluation, with its measurements |
| `domain` | One domain class from §2 |
| `preconditions` | Named precondition list with sources, or `—` |
| `undocumented_panic` | `—`, or a note when the proof had to assume a precondition that the Rust docs don't state. Each such note is a Constitution II defect listed in the evaluation (FR-003, R4) |
| `crash_id` | `C-<op>`, with the statement "`<op>` completes without panic, arithmetic overflow or undefined behaviour for every input in `<domain>`, assuming `<preconditions>`" |
| `crash_harness` | `crash::c_<op>` (or the split family `crash::c_<op>__<k>`). Alternatively, a property harness or family that assumes exactly the crash domain and preconditions and nothing more. Kani checks every panic and overflow in every harness, so that harness also discharges the crash obligation. It is used for the text parsers and the neighbour operations (research R6, R12) |
| `crash_status` | `proven` \| `proven-within-bounds` (a loop or input-size bound only, with `crash_bound`) \| `not-proven (defect D-NNN)` |
| `crash_bound` | Required for `proven-within-bounds`: the bound and the uncovered part |
| `covers` | `M/N` satisfied cover checks from the last `make verify`. Must be all satisfied (FR-008, SC-004) |
| `properties` | Catalogue property IDs covering this operation (≥ 1 `P-*` for each in-scope entry; SC-003) |
| `result_validity` | For an operation that returns a `CellId`: the `P-*` that proves its result valid (FR-004), or the upstream-documented exception, for example "may return `end(l)`" or "any `u64`, as upstream `FromToken`". `—` for other operations |
| `panics_doc` | `—`, or the `# Panics` text added by the R17 docs commit (SC-011) |
| `upstream_findings` | `U-NNN` ids that affect this row (§9; FR-018) |

**Validation rules**:
- Every item in the FR-002 inventory appears exactly once: 64 rows, i.e. 54 methods, 3 free
  functions and 7 trait impls.
- A footer lists items outside FR-002's definition: `ij_level_to_bound_uv`,
  `expanded_by_distance_uv`, and the derived trait impls.
- `excluded` requires a reason.
- Projected from research R6 and R12: 12 `float-geometry`; 3 `tool-limit` (`to_token`,
  `to_debug_string`, `impl Display`); and `all_neighbors` as
  `float-geometry: crash-freedom intractable`. That leaves 48 in scope, which is exactly
  the SC-002 floor. `vertex_neighbors` is at risk under the R15 budget (research R12).
- `in-scope` requires a crash harness, all covers satisfied, and at least one `P-*` property.
- Every row whose operation returns a `CellId` has `result_validity` filled (FR-004).
- The in-scope share must be at least 75%, i.e. ≥ 48 of 64 (SC-002).

**State**: `in-scope` → `excluded (tool-limit)` is allowed only with an evaluation entry. No
other status transitions exist.

## 4. Property (`verify/PROPERTIES.md`): the functional catalogue

| Field | Content |
|-------|---------|
| `id` | `P-<AREA>-<NN>` with AREA ∈ `VAL`, `HIER`, `RANGE`, `TRAV`, `ENC`, `ANC` (the FR-005 areas) and `NBR` (neighbour operations, so SC-003 holds for every operation) |
| `statement` | Plain-language statement against the reference model (§1) |
| `upstream_source` | C++ `s2cell_id.h`/`.cc` or `s2coords_internal.h` function or doc comment |
| `domain` | Domain class and preconditions |
| `harness` | Fully qualified harness name (`<module>::<fn>`, crate-root module), see contracts/harness-conventions.md |
| `status` | `proven` \| `proven-within-bounds` \| `not-proven (<reason>)` |
| `bound` | Required for `proven-within-bounds`: the loop bound and the uncovered part of the domain (FR-005, FR-007) |
| `reason` | Required for `not-proven`. One of: `tool-limit: <detail>` with the measured time and memory; `excluded: <operation>` when the operation is excluded from the inventory; or `defect D-NNN` (R9) |
| `mutants` | Mutant IDs that kill this property (≥ 1 for `proven*`; SC-004) |

### 4.1 Initial catalogue (49 entries)

This is the starting content of `verify/PROPERTIES.md`. Implementation fills in the
`upstream_source`, `harness`, `status` and `mutants` fields. The model functions are in
`verify/src/spec.rs`. For a valid cell `c`, `d` is its decomposition, and `pos(d)` is
`spec::position(d)`. The "Domain" column gives the domain class from §2 and the
preconditions. Where it cites R6 or R12, the harness shape and the status that research
decided apply.

| ID | Statement | Operations | Domain |
|----|-----------|------------|--------|
| P-VAL-01 | `decompose(compose(d)) == Some(d)` for every well-formed `d` | (model) | well-formed `Decomposed` |
| P-VAL-02 | `compose(decompose(x)?) == x` whenever `decompose(x)` is some | (model) | Any |
| P-VAL-03 | `CellId(x).is_valid() == spec::valid(x)` | is_valid | Any |
| P-VAL-04 | `CellId::from(x).id() == x` and `u64::from(CellId(x)) == x` | From<u64>, id, From<CellId> for u64 | Any |
| P-VAL-05 | `none().id() == 0`, `sentinel().id() == u64::MAX`, `CellId::default() == none()`, and neither is valid | none, sentinel, Default | — |
| P-VAL-06 | `face() == d.face`, `level() == d.level`, `pos() == id & (2^61 − 1)` | face, level, pos | Valid |
| P-VAL-07 | `lsb() == spec::lsb_for_level(d.level)`, `is_leaf() ⇔ d.level == 30`, `is_face() ⇔ d.level == 0` | lsb, is_leaf, is_face | Valid |
| P-VAL-08 | `lsb_for_level(l) == 1 << (60 − 2l)` and `size_ij(l) == 1 << (30 − l)` | lsb_for_level, size_ij | Level |
| P-VAL-09 | `from_face(f) == compose({f, 0, 0})` | from_face | Face |
| P-VAL-10 | `from_face_pos_level(f, pos, l) == compose(ancestor({f, 30, pos >> 1}, l))` | from_face_pos_level | Face × `pos < 2^61` × Level |
| P-HIER-01 | `children()[k] == compose(child(d, k))` for `k < 4` | children | Valid, level < 30 |
| P-HIER-02 | The four children partition the parent in curve order: `children()[0].range_min() == range_min()`, `children()[3].range_max() == range_max()`, `children()[k].range_max().id() + 2 == children()[k+1].range_min().id()` | children, range_min, range_max | Valid, level < 30 |
| P-HIER-03 | `parent() == compose(parent(d))` | parent | Valid, level ≥ 1 |
| P-HIER-04 | `parent_at_level(l) == compose(ancestor(d, l))` | parent_at_level | Valid, `l ≤ level` |
| P-HIER-05 | `child_position(l) == spec::child_position(d, l)` | child_position | Valid, `1 ≤ l ≤ level` |
| P-HIER-06 | `child_begin() == compose(child(d, 0))` and `child_end() == from_position(level+1, pos(child(d, 3)) + 1)` | child_begin, child_end | Valid, level < 30 |
| P-HIER-07 | `child_begin_at_level(l) == compose({face, l, path·4^(l−level)})` and `child_end_at_level(l) == from_position(l, pos(that) + 4^(l−level))` | child_begin_at_level, child_end_at_level | Valid, `l ≥ level` |
| P-HIER-08 | `begin(l) == compose({0, l, 0})` and `end(l) == spec::end(l)` | begin, end | Level |
| P-RANGE-01 | `range_min() == leaf_id(range(d).0)` and `range_max() == leaf_id(range(d).1)` | range_min, range_max | Valid |
| P-RANGE-02 | `a.contains(b) == spec::contains(da, db)` | contains | Valid × Valid |
| P-RANGE-03 | `a.intersects(b) == (contains(da, db) ∨ contains(db, da))` | intersects | Valid × Valid |
| P-RANGE-04 | `a.contains(b) ⇔ a.range_min().id() ≤ b.id() ≤ a.range_max().id()` | contains, range_min, range_max | Valid × ValidOrEnd |
| P-RANGE-05 | Nested or disjoint: `a.contains(b) ∨ b.contains(a) ∨ ¬a.intersects(b)` | contains, intersects | Valid × Valid |
| P-RANGE-06 | `maximum_tile(limit)` is `limit` if `range_min() ≥ limit.range_min()`. Otherwise it is the largest valid cell `r` with `r.range_min() == range_min()` and `r.range_max() < limit.range_min()` (`r` is a face, or `r.parent()` breaks one of the two conditions) | maximum_tile | Valid × ValidOrEnd |
| P-TRAV-01 | `next() == from_position(level, pos(d) + 1)`. One past the last cell is `end(level)` | next | Valid |
| P-TRAV-02 | `prev() == from_position(level, pos(d) − 1)` for `pos(d) ≥ 1`, `end(l).prev()` is the last cell at `l`, and `next().prev() == c` | prev, next | Valid ∪ {end(l)} |
| P-TRAV-03 | `next_wrap()` equals `next()` except that the last cell at a level wraps to `begin(level)`. `prev_wrap()` equals `prev()` except that `begin(level)` wraps to the last cell | next_wrap, prev_wrap | Valid |
| P-TRAV-04 | `advance(n) == from_position(level, clamp(pos(d) + n, 0, 6·4^level))` (i128) | advance | Valid × i64 |
| P-TRAV-05 | `advance_wrap(n) == from_position(level, (pos(d) + n).rem_euclid(6·4^level))` (i128) | advance_wrap | Valid × i64 |
| P-TRAV-06 | `distance_from_begin() == pos(d)` | distance_from_begin | Valid |
| P-TRAV-07 | Stepping by n agrees with n single steps. For every `n` with `0 ≤ pos(d) + n < 6·4^level`: `advance(n).next() == advance(n + 1)`. When also `pos(d) + n ≥ 1`: `advance(n).prev() == advance(n − 1)`. With `advance(0) == c`, induction gives the FR-005 statement | advance, next, prev | Valid × i64 |
| P-ENC-01 | `to_face_ij_orientation() == spec::face_ij_orientation(d)` | to_face_ij_orientation | Valid |
| P-ENC-02 | `from_face_ij(f, i, j).id() == spec::leaf_from_face_ij(f, i, j)` | from_face_ij | Face × `[0, 2^30)²` |
| P-ENC-03 | With `(f, i, j, _) = to_face_ij_orientation()`: `from_face_ij(f, i, j).parent_at_level(level()) == c` | to_face_ij_orientation, from_face_ij | Valid |
| P-ENC-04 | `get_center_si_ti() == spec::center_si_ti(d)` | get_center_si_ti | Valid |
| P-ENC-05 | `CellId::from_token(&c.to_token()) == c` | to_token, from_token | Any (R6) |
| P-ENC-06 | `from_token(s).id() == spec::parse_token(s.as_bytes())`: none unless 1–16 hex digits (FR-015) | from_token | Text (R6) |
| P-ENC-07 | `c.to_token().as_bytes() == spec::token(x)` | to_token | Any (R6) |
| P-ENC-08 | `to_debug_string()` equals `spec::debug_string(d)`, and `from_debug_string(&c.to_debug_string()) == Some(c)` | to_debug_string, from_debug_string | Valid (R6) |
| P-ENC-09 | `from_debug_string(s).map(CellId::id) == spec::parse_debug_string(s.as_bytes())` | from_debug_string | Text (R6) |
| P-ENC-10 | `s.parse::<CellId>().map(CellId::id) == spec::parse_debug_string(s.as_bytes()).ok_or(())` | FromStr | Text (R6) |
| P-ENC-11 | For every id that is not a valid cell, `to_debug_string()` is `Invalid: ` plus 16 zero-padded lowercase hex digits (FR-016) | to_debug_string | Any, `¬valid(x)` (R6, R13) |
| P-ENC-12 | `c.to_string() == c.to_debug_string()` | Display | Any (R6) |
| P-ANC-01 | `a.common_ancestor_level(b).map(u8::from) == spec::common_ancestor_level(da, db)` | common_ancestor_level | Valid × Valid |
| P-ANC-02 | Symmetric (`cal(a, b) == cal(b, a)`) and reflexive (`cal(a, a) == Some(a.level())`), and `a.parent_at_level(l) == b.parent_at_level(l)` for every `l ≤` the result | common_ancestor_level, parent_at_level | Valid × Valid |
| P-NBR-01 | `edge_neighbors()` gives four valid cells at `level()`, pairwise distinct, none equal to `c` | edge_neighbors | Valid (R12) |
| P-NBR-02 | Symmetry: `c` is in `n.edge_neighbors()` for every `n` in `c.edge_neighbors()` | edge_neighbors | Valid (R12) |
| P-NBR-03 | `vertex_neighbors(l)` gives 3 or 4 valid, pairwise distinct cells at level `l`, one of them `parent_at_level(l)` | vertex_neighbors | Valid, `l < level` (R12) |
| P-NBR-04 | `all_neighbors(l)` is `None` iff `l < level()`. Otherwise every returned cell is valid at level `l` and does not intersect `c` | all_neighbors | Valid × Level (operation excluded, R12) |

**Coverage check**: every in-scope operation in the inventory appears in the Operations
column (SC-003).

**SC-003 counting** (clarification 2026-10-01): only entries whose operations are all in
scope count towards the 90%. Projected from research R6 and R12:

| Expected status | Entries |
|-----------------|---------|
| `not-proven (tool-limit)`, outside the denominator (an excluded formatter) | P-ENC-05, P-ENC-07, P-ENC-08, P-ENC-11, P-ENC-12 |
| `not-proven (excluded)`, outside the denominator (`all_neighbors`) | P-NBR-04 |
| `proven-within-bounds` (text length) | P-ENC-06, P-ENC-09, P-ENC-10 |
| At risk under the R15 budget | P-NBR-02, P-NBR-03 |
| `proven` | the other 38, plus P-NBR-02 and P-NBR-03 if they fit |

That gives a denominator of 43, with at most 40 entries `proven`. SC-003 needs 39.

- If P-NBR-02 ends up `not-proven (tool-limit)`, the result is 39/43 (90.7%), and SC-003
  still holds.
- If `vertex_neighbors` is excluded, P-NBR-03 leaves the denominator, and SC-003 still
  holds (38/42 at worst). SC-002 is then missed instead (research R12).
- Any further shortfall misses SC-003, and is recorded in the evaluation.

**State transitions** (driven by `make verify` and `make verify-mutants` results):

```
draft ──harness passes, covers satisfied──▶ proven | proven-within-bounds ──mutant killed──▶ evidence recorded
  │                                                       │
  │                                                       └──every mutant survived──▶ back to draft (strengthen)
  ├──timeout / intractable──▶ not-proven (tool-limit)
  └──genuine counterexample──▶ not-proven (defect D-NNN), harness listed in known-defects.txt
```

## 5. Mutant (`verify/mutants/*.patch` + `verify/MUTANTS.md`)

| Field | Content |
|-------|---------|
| `id` | `M-<NNN>` |
| `patch` | A unified diff against `core/src/s2/cell_id.rs` (or `coords.rs`) introducing one defect |
| `targets` | Catalogue property IDs the defect violates. The only crash target is `C-advance`, on `M-001`, the US1 scenario 3 overflow demo |
| `expect_fail` | Harness names that must report FAILURE |
| `outcome` | `killed` (every expected harness failed) \| `survived` (property is vacuous or too weak, which is a defect in the proof) \| `error` (the patch does not apply or compile) |

**Rules**:
- A `survived` outcome blocks marking the targeted property `proven` (FR-008).
- `M-050` is the reverse of the FR-015 fix (it targets P-ENC-06).
- The FR-016 fix has no mutant. It sits in `to_debug_string`, which is excluded, so its
  evidence is the regression test (research R13).

## 6. Known defect entry (`verify/known-defects.txt`)

One line per harness that fails on a recorded defect whose separate fix has not landed yet
(R9): `<fully qualified harness> D-NNN`. `run-verify.sh` reports such a harness as
`KNOWN-DEFECT D-NNN` instead of failing the run. If a listed harness succeeds, that is an
error, and the entry must be removed. FR-015 and FR-016 never appear here.

## 7. Trusted Base Statement (`verify/TRUSTED_BASE.md`)

It lists everything the proofs rely on without proving it (FR-013):
- **Tools:** the Kani version, the CBMC version, and the SAT/SMT solver per harness.
- **Models:** Kani's model of `std` (allocation, `String`, `Vec`), rustc MIR semantics, and
  CBMC's bit-precise IEEE-754 semantics for the neighbour operations' wrap path (R12).
- **The text domain argument** (R6): the parsers read only bytes, so proving them over every
  byte string of length `L` covers every `&str` of that length. That relies on the one
  audited `unsafe` helper, `domain::with_text`.
- **The reference model:** `spec.rs`, validated by sampling against the C++ vectors.
- **Limits:** unwind bounds, the `proven-within-bounds` gaps, domain preconditions, every
  tool-limit exclusion, and every `not-proven` item.

## 8. Pilot Evaluation (`verify/EVALUATION.md`)

It records:
- **Effort log** (A1): person-days per phase, appended at each phase checkpoint, so effort
  is measured rather than estimated afterwards.
- **Measurements:** per-harness times from `run-verify.sh`, and CI wall-clock time per run
  and for the slowest harness.
- **Limitations:** tool limits, exclusions, the loop-bound gaps, and the SC-002 share.
- **Defects found**, each with a link to its fix and regression test:
  - FR-015 (`from_token` `+`) and FR-016 (`Invalid:` padding), fixed by commits in this
    feature;
  - every `D-NNN`, fixed in a separate change under FR-010.
- **`# Panics` documentation added** by the R17 commit: one line per operation, with the
  condition (SC-011, Constitution II).
- **Upstream findings**: every `U-NNN` (§9), with its follow-up task (FR-018, SC-012).
- **Recommendations:** go or no-go; whether to add a stronger assurance level (deductive or
  kernel-checked); and ranked follow-up types (FR-014, SC-007).
- **Acceptance:** a short section that checks SC-001…SC-012. For SC-005, it gives the time
  on the 4-vCPU/16 GB reference runner, and on the project's actual runner if that is
  smaller.

## 9. Upstream finding (`U-NNN`; FR-018, SC-012; research R18)

This is a recorded case of undefined or wrong behaviour in the upstream reference: C++, Go,
Java, or the generated vectors.

| Field | Content |
|-------|---------|
| `id` | `U-NNN` |
| `upstream` | The implementation and `file:line`, for example `s2cell_id.h:599–602` |
| `inputs` | The concrete inputs that show it |
| `observed` | What upstream does: undefined behaviour, a wrong result, or a contradiction with its own documentation |
| `rust` | What the Rust port does and documents for the same inputs |
| `affects` | The inventory rows and harnesses whose proof assumptions it shapes |
| `follow_up` | The task id in the "Follow-ups" section of `tasks.md`, plus `D-NNN` if Rust behaviour should change (FR-010) |

**Rules**:
- An entry is created, and its follow-up task is added, **before** the affected proof is
  marked complete.
- The affected harness's doc comment cites it.
- `EVALUATION.md` lists every entry (SC-012).
- U-001 is known in advance: C++ `level()` is undefined behaviour on `0`, and returns −1 on
  ids with 62 or more trailing zeros (research R18).
- U-002 is known in advance: C++ `GetCommonAncestorLevel` is undefined behaviour on
  (`none`, `none`), and Go returns level 30 for it (research R18).
