# LACKING — closing audit vs. the C++ reference implementation

**Closing audit, 2026-06-11.** Re-audit of this Rust port (`s2rst`, `core/`)
against Google's C++ s2geometry at `~/sources/s2geometry`, commit `8d5c2a87` —
the same anchor as the opening audit (2026-06-09, preserved in git history at
`517d64d`). All work tracked in [PLAN_LACKING.md](PLAN_LACKING.md).

Method: every original finding's closure was re-verified against the *code*
(not the ledger) by an independent review pass — 13/13 closure claims
confirmed with file:line evidence, including a C++-side re-check of the
upstream-parity claim. A fresh-eyes sweep then looked for missed gaps via
in-code admissions, public-API spot checks of the five most-changed modules,
and compile-guarantee checks.

---

## Disposition of the original findings

| ID | Original finding | Disposition |
|----|------------------|-------------|
| H1 | Boolean-op `GetCrossedVertexIndex` degenerate cases (3 ignored tests) | **Fixed** (`0ebc4cc`): root cause was `predicates::sign` (= non-robust `SimpleCCW`) used where C++ calls robust `s2pred::Sign`; renamed to `simple_ccw`, call sites fixed, tests un-ignored. |
| H2 | `LaxPolygonLayer` undirected edges | **Retracted — false finding**: upstream C++ is equally unimplemented (`TODO(b/281698554)`); Rust is at exact parity (re-verified upstream this audit). |
| H3 | Eager "Encoded" family | **Closed** (`07284c1`..`50eb131`): four lazy vector views (O(1) access, `lower_bound`), encoded lax shapes over shared `Arc` buffers, and a truly lazy `EncodedS2ShapeIndex` (header-only init + `OnceLock` caches) served by the generic queries. |
| H4 | Add-only `ShapeIndex`, explicit build | **Closed** (`eeee542`, `daece2e`): `release`/`release_all`/`clear`/`minimize` with C++ id-gap semantics; lazy thread-safe build via `OnceLock` (explicit `build()` now optional); `Options`, `space_used`, memory-tracker accounting. |
| M1 | No abstract cell-iterator framework | **Closed** (`2b7fe71`, `226f470`, `a205052`): `CellIterator` + GAT `ShapeIndexLike`; generic range iterator and join; `CellUnion`/`S2PointIndex` iterators; all edge/crossing queries and `ShapeIndexRegion` generic. `CellIndex` deliberately excluded (nested cells violate the disjointness contract; upstream agrees). |
| M2 | `S2R2Rect` missing | **Closed** (`3388c75`/`807b9f0`). |
| M3 | `S2PointLoopSpan` missing | **Closed** (`3156092`/`ab527ef`). |
| M4 | Text-format gaps | **Closed** (`7049152`/`49dc28e`), including a behavioral fix: `make_polygon` now normalizes loops like C++ (it had silently behaved like `MakeVerbatimPolygon`). |
| M5 | Term-indexer prefix support | **Closed** (`68d0752`/`e9b3d52`). |
| M6 | `CodingHint` not selectable | **Closed** (`998eafb`): `Polyline`/`Polygon::encode_with_hint` with the C++ lossless-fallback semantics. `Loop` has no hint API because upstream `S2Loop::Encode` has none. Won't-dos recorded: `S2Coder` trait (no consumer), structured decode errors (`io::Error` kept), partial decode overloads. |
| M7 | No intermediate predicate tier | **Closed: measured won't-do.** Exact-arithmetic engagement is 0.0000% in both a realistic boolean union and the adversarial sliver union (the stable tier absorbs them); the exact+symbolic path costs 349ns vs 11ns triage (31×) only on a literally-zero determinant. A double-double tier would optimize a never-taken path. |
| M8 | `VertexOutMap::edges(v0,v1)` | **Retracted — false finding**: present as `edge_ids_between` with equal-range semantics. |
| M9 | No `s2debug` override | **Closed by policy**: per-layer `validate` options cover the C++ per-object overrides; no global flag (recorded). |
| L1–L14 | Low-severity batch | **Done or recorded** (`9f66041`, `aec974d`, `a984e9a`): shipped L1, L2-a (`from_cell_union_border`), L3, L4, L5, L7, L8, L9, L10, L12 (including fixing latently-broken derived projection `Default`s), L14 (`project()` modernized to current C++); recorded skips with rationale: L2-b (`init_to_simplified_in_cell`), L6, L8-b (`pack`), L13, `Shape::user_data`. |

---

## Remaining findings

None. The closing audit's one finding (R1) was fixed; see below.

### R1. [was high, narrow] split_crossing_edges did not re-route edges — FIXED (`9b87da3`)

`S2Builder::add_edge_crossings` found the self-crossings and added the
intersection points as input vertices, but never set `snapping_needed = true`.
Phase 2.7 (`add_extra_sites`) — the pass that routes crossing edges through the
nearby intersection sites — is gated on that flag, so it was skipped: the
intersection sites existed but no edge was split, and the output kept the
self-crossing (manifesting downstream as `BuilderEdgesDoNotFormLoops` /
the multiplicity assertion). C++ `s2builder.cc` sets `snapping_needed_ = true`
inside `AddEdgeCrossings` (line 709) for exactly this reason; the port omitted
it. Diagnosed with the Phase 0 both-sides trace recipe: replaying the identical
52-vertex self-crossing winding-operation input through C++
`S2WindingOperation` produced a valid 48-vertex output while Rust kept all 52
vertices unchanged, isolating the failure to the builder's crossing resolution
rather than the buffer geometry. The fix also removed the dead
`edge_crossing_vertices` recording (written, never read — the real routing is
the generic `edge_sites`/`add_extra_sites` mechanism, as in C++). The ported
`ZigZagLoop` test is un-ignored and passes.

### Recorded deferrals (each with revisit criteria)

- **Incremental index updates** (C++ `BatchGenerator`): full rebuild measured
  at ~3.5ms / 30ms / 365ms for 10k / 100k / 1M edges, amortized over any
  number of mutations — revisit for mutating workloads beyond ~100k edges.
- **Lazy-index init speed vs the a-priori 100× gate**: measured 9–12× faster
  than eager (1.2ms vs 14.8ms at 2.9MB; 11.9ms vs 111.7ms at 28MB). The gap
  to "header-only" is the deliberate fuzz-safety cell-id validation scan
  (O(cells), allocation-free) plus the input buffer copy; shape/cell/edge
  decoding — the dominant eager cost — is fully lazy. First queries pay
  4–11µs to decode one cell on demand.
- **Memory-tracker mid-build abort** (accounting-only today; C++ can cancel).
- **`init_to_simplified_in_cell`** (interior-snapping machinery, no consumer).
- **C++-generated hint-bytes vectors**: `encode_with_hint` round-trips are
  pinned in Rust both directions; a `refgen-cpp` section for C++-produced
  FAST/COMPACT bytes is the remaining cross-validation extension.
- **Slow-test ignores** in `builder/snap.rs` (worst-case ratio searches,
  ~5–10s each): performance markers with documented reasons, not gaps.

---

## Checked and NOT lacking (false leads, cumulative)

- `LaxPolygonLayer` undirected edges (ex-H2) and `VertexOutMap::edges(v0,v1)`
  (ex-M8) — see dispositions above.
- **`ShapeIndex` serialization** — flagged by this audit's sweep as missing;
  false: `encode_to_writer`/`encode_with_hint`/`decode_from_reader` are
  public, defined in `shape_index_encoding.rs:367-432` rather than
  `shape_index.rs`.
- The opening audit's false leads (binary Encode/Decode coverage, CellUnion's
  `Region` impl, generic `S2PointIndex`, polygon distance/overlap ops,
  `get_surface_integral`, `uninterpolate`, boolean-op options, CellIndex
  iterators, tagged-shape coding, query targets) remain valid — none
  regressed.

---

## Verification evidence (2026-06-11, R1 fix 2026-06-13)

- Test suite: **3,534 passed, 0 failed** (incl. doctests and the now-active
  `ZigZagLoop` test); `send_sync.rs` also proves
  `EncodedS2ShapeIndex: Send + Sync` at compile time.
- `cargo clippy --all-targets` and `RUSTDOCFLAGS="-D warnings" cargo doc`:
  clean; workspace (python/wasm bindings) compiles.
- iai-callgrind benches across the refactor surface: 0 regressions
  (shape_index 19, closest_edge_query 22, contains_point_query 6,
  cell_iterator_join 3).
- Decoder robustness suite (incl. the fuzz regression corpus) green — it
  caught and gated one real out-of-bounds during Phase 3 (cell records with
  out-of-range edge ids), now cross-validated lazily.
- Independent closure verification: 13/13 PASS with file:line evidence.

## Conclusion

Every finding is now resolved. The opening audit's high- and medium-severity
items are fixed, closed with measurements, or retracted as false — verified
against the code on both sides — and the one new finding the closing sweep
surfaced (**R1**, the buffer-operation zigzag) was root-caused and fixed
(`9b87da3`). No open correctness items remain; the deferrals above are
feature/optimization items with recorded revisit criteria, not gaps against
passing upstream behavior.
