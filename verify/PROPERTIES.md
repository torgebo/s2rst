# Property Catalogue: `s2rst::s2::CellId` (FR-005)

In plain language and grouped by kind of guarantee, with the other two catalogues:
[`PROPERTIES_BY_KIND.md`](PROPERTIES_BY_KIND.md).

Functional properties, each stated against the reference model `verify/src/spec.rs` (FR-006)
and traced to upstream S2. Crash-freedom proofs are not catalogue properties; they live in
`INVENTORY.md` (spec Key Entities). Fields: `specs/001-formal-verify-base-type/data-model.md`
§4. Status values: `proven` | `proven-within-bounds` | `not-proven (<reason>)`, the reason being
`tool-limit: <detail>`, `excluded: <operation>` or `defect D-NNN`.

**SC-003 counting** (clarification 2026-10-01): only entries whose operations are all in
scope count towards the 90 %. Rows marked *outside* exercise an excluded operation.

| ID | Statement | Upstream source (`s2cell_id.h`/`.cc` unless noted) | Domain | Harness | Status | Bound | Mutants |
|----|-----------|---------------------------------------------------|--------|---------|--------|-------|---------|
| P-VAL-01 | `decompose(compose(d)) == Some(d)` for every well-formed `d` | `s2cell_id.h` class comment: the bit layout (face, Hilbert position, trailing 1 bit) | well-formed `Decomposed` | `val::p_val_01` | proven |  | M-010 |
| P-VAL-02 | `compose(decompose(x)?) == x` whenever `decompose(x)` is some | `s2cell_id.h` class comment: the bit layout | Any | `val::p_val_02` | proven |  | M-010 |
| P-VAL-03 | `CellId(x).is_valid() == spec::valid(x)` | `S2CellId::is_valid()` | Any | `val::p_val_03` | proven |  | M-011 |
| P-VAL-04 | `CellId::from(x).id() == x` and `u64::from(CellId(x)) == x` | `S2CellId(uint64_t)`, `id()` | Any | `val::p_val_04` | proven |  | M-012 |
| P-VAL-05 | `none().id() == 0`, `sentinel().id() == u64::MAX`, `CellId::default() == none()`, and neither is valid | `None()`, `Sentinel()`, default constructor | — | `val::p_val_05` | proven |  | M-013 |
| P-VAL-06 | `face() == d.face`, `level() == d.level`, `pos() == id & (2^61 − 1)` | `face()`, `level()`, `pos()` | Valid | `val::p_val_06` | proven |  | M-014 |
| P-VAL-07 | `lsb() == spec::lsb_for_level(d.level)`, `is_leaf() ⇔ d.level == 30`, `is_face() ⇔ d.level == 0` | `lsb()`, `lsb_for_level()`, `is_leaf()`, `is_face()` | Valid | `val::p_val_07` | proven |  | M-015 |
| P-VAL-08 | `lsb_for_level(l) == 1 << (60 − 2l)` and `size_ij(l) == 1 << (30 − l)` | `lsb_for_level(level)`, `GetSizeIJ(level)` | Level | `val::p_val_08` | proven |  | M-017 |
| P-VAL-09 | `from_face(f) == compose({f, 0, 0})` | `FromFace(face)` | Face | `val::p_val_09` | proven |  | M-018 |
| P-VAL-10 | `from_face_pos_level(f, pos, l) == compose(ancestor({f, 30, pos >> 1}, l))` | `FromFacePosLevel(face, pos, level)` | Face × `pos < 2^61` × Level | `val::p_val_10` | proven |  | M-019 |
| P-HIER-01 | `children()[k] == compose(child(d, k))` for `k < 4` | `child(position)` | Valid, level < 30 | `hier::p_hier_01` | proven |  | M-020 |
| P-HIER-02 | The four children partition the parent in curve order: `children()[0].range_min() == range_min()`, `children()[3].range_max() == range_max()`, `children()[k].range_max().id() + 2 == children()[k+1].range_min().id()` | `child(position)`, `range_min()`, `range_max()`; class comment (children in Hilbert order) | Valid, level < 30 | `hier::p_hier_02` | proven |  | M-020 |
| P-HIER-03 | `parent() == compose(parent(d))` | `parent()` | Valid, level ≥ 1 | `hier::p_hier_03` | proven |  | M-016 |
| P-HIER-04 | `parent_at_level(l) == compose(ancestor(d, l))` | `parent(level)` | Valid, `l ≤ level` | `hier::p_hier_04` | proven |  | M-021 |
| P-HIER-05 | `child_position(l) == spec::child_position(d, l)` | `child_position(level)` | Valid, `1 ≤ l ≤ level` | `hier::p_hier_05` | proven |  | M-022 |
| P-HIER-06 | `child_begin() == compose(child(d, 0))` and `child_end() == from_position(level+1, pos(child(d, 3)) + 1)` | `child_begin()`, `child_end()` | Valid, level < 30 | `hier::p_hier_06` | proven |  | M-023 |
| P-HIER-07 | `child_begin_at_level(l) == compose({face, l, path·4^(l−level)})` and `child_end_at_level(l) == from_position(l, pos(that) + 4^(l−level))` | `child_begin(level)`, `child_end(level)` | Valid, `l ≥ level` | `hier::p_hier_07` | proven |  | M-024 |
| P-HIER-08 | `begin(l) == compose({0, l, 0})` and `end(l) == spec::end(l)` | `Begin(level)`, `End(level)` | Level | `hier::p_hier_08` | proven |  | M-025 |
| P-RANGE-01 | `range_min() == leaf_id(range(d).0)` and `range_max() == leaf_id(range(d).1)` | `range_min()`, `range_max()` | Valid | `range::p_range_01` | proven |  | M-030 |
| P-RANGE-02 | `a.contains(b) == spec::contains(da, db)` | `contains(other)` | Valid × Valid | `range::p_range_02` | proven |  | M-031 |
| P-RANGE-03 | `a.intersects(b) == (contains(da, db) ∨ contains(db, da))` | `intersects(other)` | Valid × Valid | `range::p_range_03` | proven |  | M-032 |
| P-RANGE-04 | `a.contains(b) ⇔ a.range_min().id() ≤ b.id() ≤ a.range_max().id()` | `contains(other)`; `range_min()`/`range_max()` comment | Valid × ValidOrEnd | `range::p_range_04` | proven |  | M-031 |
| P-RANGE-05 | Nested or disjoint: `a.contains(b) ∨ b.contains(a) ∨ ¬a.intersects(b)` | class comment: the cell hierarchy (nested or disjoint) | Valid × Valid | `range::p_range_05` | proven |  | M-032 |
| P-RANGE-06 | `maximum_tile(limit)` is `limit` if `range_min() ≥ limit.range_min()`. Otherwise it is the largest valid cell `r` with `r.range_min() == range_min()` and `r.range_max() < limit.range_min()` (`r` is a face, or `r.parent()` breaks one of the two conditions) | `maximum_tile(limit)` doc comment | Valid × ValidOrEnd | `range::p_range_06` | proven |  | M-033 |
| P-TRAV-01 | `next() == from_position(level, pos(d) + 1)`. One past the last cell is `end(level)` | `next()`, `End(level)` | Valid | `trav::p_trav_01` | proven |  | M-034 |
| P-TRAV-02 | `prev() == from_position(level, pos(d) − 1)` for `pos(d) ≥ 1`, `end(l).prev()` is the last cell at `l`, and `next().prev() == c` | `prev()` | Valid ∪ {end(l)} | `trav::p_trav_02` | proven |  | M-035 |
| P-TRAV-03 | `next_wrap()` equals `next()` except that the last cell at a level wraps to `begin(level)`. `prev_wrap()` equals `prev()` except that `begin(level)` wraps to the last cell | `next_wrap()`, `prev_wrap()` | Valid | `trav::p_trav_03` | proven |  | M-036 |
| P-TRAV-04 | `advance(n) == from_position(level, clamp(pos(d) + n, 0, 6·4^level))` (i128) | `advance(steps)` doc comment (never past `End()` or before `Begin()`) | Valid × i64 | `trav::p_trav_04` | proven |  | M-001 |
| P-TRAV-05 | `advance_wrap(n) == from_position(level, (pos(d) + n).rem_euclid(6·4^level))` (i128) | `advance_wrap(steps)` | Valid × i64 | `trav::p_trav_05__l{0..30}` | proven |  | M-037 |
| P-TRAV-06 | `distance_from_begin() == pos(d)` | `distance_from_begin()` | Valid | `trav::p_trav_06` | proven |  | M-038 |
| P-TRAV-07 | Stepping by n agrees with n single steps. For every `n` with `0 ≤ pos(d) + n < 6·4^level`: `advance(n).next() == advance(n + 1)`. When also `pos(d) + n ≥ 1`: `advance(n).prev() == advance(n − 1)`. With `advance(0) == c`, induction gives the FR-005 statement | `advance(steps)`, `next()`, `prev()` | Valid × i64 | `trav::p_trav_07` | proven |  | M-034, M-035 |
| P-ENC-01 | `to_face_ij_orientation() == spec::face_ij_orientation(d)` | `ToFaceIJOrientation()`; `s2coords_internal.h` tables | Valid | `enc::p_enc_01` | proven |  | M-052, M-053 |
| P-ENC-02 | `from_face_ij(f, i, j).id() == spec::leaf_from_face_ij(f, i, j)` | `FromFaceIJ(face, i, j)`; `s2coords_internal.h` tables | Face × `[0, 2^30)²` | `enc::p_enc_02` | proven |  | M-053, M-054 |
| P-ENC-03 | With `(f, i, j, _) = to_face_ij_orientation()`: `from_face_ij(f, i, j).parent_at_level(level()) == c` | `FromFaceIJ()`, `ToFaceIJOrientation()` | Valid | `enc::p_enc_03` | proven |  | M-054 |
| P-ENC-04 | `get_center_si_ti() == spec::center_si_ti(d)` | `GetCenterSiTi()` | Valid | `enc::p_enc_04` | proven |  | M-055 |
| P-ENC-05 | `CellId::from_token(&c.to_token()) == c` | `ToToken()`, `FromToken()` | Any (R6) | — | not-proven (tool-limit: to_token: `format!` over 5 min) — *outside* |  | — |
| P-ENC-06 | `from_token(s).id() == spec::parse_token(s.as_bytes())`: none unless 1–16 hex digits (FR-015) | `FromToken()` (`s2cell_id.cc`) | Text (R6) | `enc::p_enc_06__b{0..=17}` | proven-within-bounds | input size: byte length ≤ 17; longer texts are rejected by the length check before any loop (FR-007) | M-050 |
| P-ENC-07 | `c.to_token().as_bytes() == spec::token(x)` | `ToToken()` | Any (R6) | — | not-proven (tool-limit: to_token: `format!` over 5 min) — *outside* |  | — |
| P-ENC-08 | `to_debug_string()` equals `spec::debug_string(d)`, and `from_debug_string(&c.to_debug_string()) == Some(c)` | `ToString()`, `FromDebugString()` | Valid (R6) | — | not-proven (tool-limit: to_debug_string: over 5 min (valid branch)) — *outside* |  | — |
| P-ENC-09 | `from_debug_string(s).map(CellId::id) == spec::parse_debug_string(s.as_bytes())` | `FromDebugString()` | Text (R6) | `enc::p_enc_09__b{0..=33}` | proven-within-bounds | input size: byte length ≤ 33; longer texts are rejected by the length check before any loop (FR-007) | M-056, M-058 |
| P-ENC-10 | `s.parse::<CellId>().map(CellId::id) == spec::parse_debug_string(s.as_bytes()).ok_or(())` | `FromDebugString()` (Rust `FromStr` delegates to it) | Text (R6) | `enc::p_enc_10__b{0..=33}` | proven-within-bounds | input size: byte length ≤ 33; longer texts are rejected by the length check before any loop (FR-007) | M-057, M-058 |
| P-ENC-11 | For every id that is not a valid cell, `to_debug_string()` is `Invalid: ` plus 16 zero-padded lowercase hex digits (FR-016) | `ToString()` (invalid branch, `absl::kZeroPad16`) | Any, `¬valid(x)` (R6, R13) | — | not-proven (tool-limit: to_debug_string: `format!` OOM above 6 GB) — *outside* |  | — |
| P-ENC-12 | `c.to_string() == c.to_debug_string()` | `operator<<` (writes `ToString()`) | Any (R6) | — | not-proven (tool-limit: Display: `core::fmt` over 5 min) — *outside* |  | — |
| P-ANC-01 | `a.common_ancestor_level(b).map(u8::from) == spec::common_ancestor_level(da, db)` | `GetCommonAncestorLevel(other)` | Valid × Valid | `anc::p_anc_01` | proven |  | M-039, M-040 |
| P-ANC-02 | Symmetric (`cal(a, b) == cal(b, a)`) and reflexive (`cal(a, a) == Some(a.level())`), and `a.parent_at_level(l) == b.parent_at_level(l)` for every `l ≤` the result | `GetCommonAncestorLevel(other)`, `parent(level)` | Valid × Valid | `anc::p_anc_02` | proven |  | M-040 |
| P-NBR-01 | `edge_neighbors()` gives four valid cells at `level()`, pairwise distinct, none equal to `c` | `GetEdgeNeighbors()` | Valid (R12) | `nbr::p_nbr_01__f{0..5}_k{0..3}` and `nbr::p_nbr_01__f{0..5}_p{01,02,03,12,13,23}` (weekly slow tier) | proven |  | M-060 |
| P-NBR-02 | Symmetry: `c` is in `n.edge_neighbors()` for every `n` in `c.edge_neighbors()` | `GetEdgeNeighbors()` | Valid (R12) | — | not-proven (tool-limit: CBMC out of memory at the 4 GiB ceiling within 35 s, per face and per face × neighbour index) |  | — |
| P-NBR-03 | `vertex_neighbors(l)` gives 3 or 4 valid, pairwise distinct cells at level `l`, one of them `parent_at_level(l)` | `AppendVertexNeighbors(level)` | Valid, `l < level` (R12) | `nbr::p_nbr_03__f{0..5}_q{0..3}` (weekly slow tier) | proven |  | M-062 |
| P-NBR-04 | `all_neighbors(l)` is `None` iff `l < level()`. Otherwise every returned cell is valid at level `l` and does not intersect `c` | `AppendAllNeighbors(level)` | Valid × Level (operation excluded, R12) | — | not-proven (excluded: all_neighbors) — *outside* |  | — |
