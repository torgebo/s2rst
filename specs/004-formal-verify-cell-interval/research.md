# Research: Formal Verification of `Cell` and `s1::Interval`

Phase 0 of the plan. Each section gives the decision, the rationale and the alternatives
considered. R0 is the planning probe of 2026-10-12. R1 is the re-check of the pinned upstream.

## R0: Planning probe (2026-10-12)

**What ran.** `probe/r0_harnesses.rs` holds 32 Kani harnesses in two rounds: `r0_*`, and
`r1_*`, written to narrow round 1's timeouts and failures. They ran from a scratch crate that
depends on `s2rst` (`core/` at `ae19586`), through a copy of `verify/run-verify.sh`: a
4 GiB ceiling and 12 minutes per harness, Kani 0.68.0, CBMC 6.11.0, on the 8-thread, 30 GB
workstation. They measure three things:

- the FR-020 containment proof, against a restated copy of the proposed fix;
- the five operations FR-003 leaves to planning;
- the two stored-bound readers of FR-002.

**Results.**

Times are wall-clock per harness. Round 1 ran 4 jobs and round 2 ran 5, on 8 threads, so
times under contention run high. ✅ proved, ❌ counterexample, ⏱ hit the 12-minute limit.

| Harness | Measures | Result |
|---------|----------|--------|
| `r0_len_unique` | P-IMEA-03: upstream's length is negative only at the corner | ✅ 0.2 s, cover reached |
| `r0_len_contained` | P-IMEA-01 against the proposed fix, three-way: fix, library, upstream | ✅ 525 s |
| `r1_len_2way` | P-IMEA-01's real shape: fix against upstream | ✅ 217 s (split by inversion: 318 s + 121 s, no gain) |
| `r0_center` | P-IMEA-04 | ✅ 30 s |
| `r0_complement_center` | P-IMEA-05 | ✅ 30 s |
| `r0_approx_eq_any_error` | crash freedom, every error bound including NaN | ✅ 1.3 s |
| `r0_approx_eq_reflexive` | P-IEXP-02, reflexivity | ✅ 254 s |
| `r0_expanded_any_margin` | crash freedom, every margin | ❌ 6.7 s: a NaN margin trips `new`'s debug check (K5, below) |
| `r1_expanded_non_nan` | crash freedom and validity, every margin except NaN | ✅ 91 s |
| `r0_expanded_contains` | P-IEXP-01 | ❌ 58 s: the counterexample is the corner with margin 3.34, K4's knock-on, found independently |
| `r1_expanded_contains_off_corner` | P-IEXP-01 without the corner | ⏱ |
| `r0_dhd` | P-IMEA-06, whole | ⏱ |
| `r1_dhd_crash` | crash freedom | ❌ 44 s: D-005, below |
| `r1_dhd_zero_and_pi` | P-IMEA-06's 0 and π clauses | ❌ 50 s: D-005 |
| `r1_dhd_range__yinv`, `__ynoninv` | P-IMEA-06's range clause, split | ✅ 137 s; ⏱ |
| `r0_cell_uv_edge` | P-CBND-03, every valid cell | ✅ 26 s |
| `r0/r1_cell_uv_ordered__l<L>` | P-CBND-02 at level L | L=3 ✅ 9 s; L=10 ✅ 98 s; L=15, 20, 30 ⏱ |
| `r1_cell_bound_model__l<L>` | P-CBND-01 (u bounds against the restated projection) | L=3 ✅ 9.5 s; L=10 ✅ 575 s; L=20 ⏱ |
| `r0/r1_cell_children_uv__l<L>` | P-CCHD-02 at level L | L=3 ✅ 109 s; L=5 ✅ 638 s; L=7, 10, 29 ⏱ |

Outside Kani, an exhaustive check compared every boundary value `k ∈ [0, 2^30]` with the
restated projection. It found 0 differences between the library's
`st_to_uv(ij_to_st_min(k))` and upstream's formula, and 0 between `si_ti_to_st(2k)` and
`ij_to_st_min(k)`, the two scalings that `children()` and the constructor use. It took
4.3 s in a release build and 88 s in a debug build (contended).

**What it shows.**

- **The FR-020 containment proof is in reach.** P-IMEA-01 proves in 217 s and P-IMEA-03
  in 0.2 s, over every valid interval, against a restated copy of the proposed fix. The
  design is therefore known to be provable and minimal before the library changes (R5).
- **Comparisons and additions prove; products of symbolic floats do not scale.**
  - The interval's operations prove in 1–254 s, whole.
  - `Cell`'s bounds go through `st_to_uv`, which squares a symbolic `s`. Asserting
    anything on them costs about 10× per few levels, and stops within 12 minutes at levels
    5–10. This matches the pilot (no result in 2 hours over all levels) and
    VERIFIED_TYPES's note. R8 sets the levels.
- **Two new findings, both found by the crash proofs.**
  - **D-005 (port defect).** `directed_hausdorff_distance` builds its two helper arcs
    with `new_checked`, which does not turn −π into π. Upstream uses the normalising
    constructor `S1Interval(lo, hi)` (`s1interval.cc:155–158`). When `y` is a short
    interval near 0, its complement's centre rounds to exactly −π, and the debug check in
    `new_checked` fires on valid input. Example: `x = [1, 2]` and `y = [0, 1e−17]` panic
    in a debug build. A release build returns 2, which is correct: `[lo, −π]` and
    `[−π, hi]` contain the same normalised points as upstream's arcs, as the argument in
    R9 shows. Fixed under FR-012 (R9).
  - **K5 (an undocumented precondition).** `expanded` with a NaN margin panics in debug
    builds, through `new`'s check, and returns an invalid interval in release builds.
    Upstream behaves the same: its constructor `ABSL_DCHECK`s validity
    (`s1interval.h:233`). FR-004 requires such a precondition to be recorded and
    documented, and FR-019 covers it (R7). Every other margin, the infinities included,
    is crash-free and gives a valid interval (91 s).
- **Kani found K4's knock-on in `expanded` without being told.** The first counterexample to
  P-IEXP-01 was the corner. After FR-020, the corner leaves the domain of failures.

## R1: Re-reading the pinned upstream (v0.14.0-40-g37a4501)

**Decision.** No survey finding is withdrawn. The upstream records change as follows:

- **K4 (the corner):** it becomes **U-007** in `UPSTREAM.md`, and moves out of "Other
  upstream behaviours that s2rst inherits", because the library now deviates (FR-020).
  - Upstream's `GetLength` (`s1interval.cc:41–47`) is called by `Expanded` (`:213`, `:218`),
    `Intersection` (`:268`), `ApproxEquals` (`:286–295`) and `S2LatLngRect::Area`
    (`s2latlng_rect.cc:94–99`). Each gives the wrong answer at the corner, as the s2rst
    probe of the knock-on effects showed for the Rust copies.
  - A follow-up task reports it upstream, together with the open U-004 and U-005 reports
    (FR-022).
- **K1 (`Cell` from an invalid identifier):**
  - Upstream's constructor (`s2cell.cc:64–71`) checks nothing. The null identifier reaches
    U-001's undefined behaviour.
  - Face 6 or 7 builds a cell whose later use indexes `kFaceUVWAxes` out of bounds. That is
    U-004's analysis, which reached the constructor through `Decode`.
  - No new entry is needed. U-004 gains a sentence saying that the constructor itself
    accepts the same identifiers. The library panics instead, which FR-017 documents. A
    panic on an undocumented precondition is safer than upstream's undefined behaviour, and
    is not a deviation in results.
- **K3 (debug-only checks):** upstream checks the same preconditions with `ABSL_DCHECK`
  (`s1interval.cc:72, 79, 165, 186–187, 197–198`; `s1interval.h:233`), so release builds
  behave the same in both. FR-019 documents it. This is not a finding against upstream.
- **`ieee_remainder` versus C's `remainder`:**
  - The library computes `x − round(x/y)·y`, where `round` takes halves away from zero.
    C's `remainder` takes them to even, and is exact.
  - On the inputs that `expanded` and `approx_eq_with` can pass, `x` lies in (−2π, 2π) and
    the quotient rounds to −1, 0 or 1, so `x − q·2π` is exact by Sterbenz's lemma.
  - A half arises only at `x = ±π`. There the two results differ in sign, and the
    interval's normalisation (−π to π), or the absolute value in `approx_eq_with`, removes
    the difference.
  - P-IEXP-01 and the differential vectors (R10) check this. Any row that disagrees is a
    finding under FR-012.

- **D-005 (R0, R9) is the port's, not upstream's.** Upstream's
  `GetDirectedHausdorffDistance` (`s1interval.cc:145–162`) builds its helper arcs with the
  normalising constructor.
- **K5 matches upstream.** `Expanded` with a NaN margin reaches the `ABSL_DCHECK` in the
  constructor, as the library's `new` does.

**Rationale.** 003's R1 showed that re-reading the pinned oracle can withdraw a finding. Here
it confirms all four, places D-005 on the port's side, and settles which `UPSTREAM.md`
entries change.

## R2: Suite gating, names and records

**Decision.**

- **Gating:** a third default feature, `values`, in `verify/Cargo.toml`. It gates every
  `cl_*` and `iv_*` module and `value_domain.rs`.
- **Make target:** `make verify-values [P=cell|interval]`, with
  `--harness cl_ --harness iv_`.
- **CI:** one new matrix leg, `values`, in `verify.yml`.
- **Records:** in `verify/values/` (`INVENTORY.md`, `PROPERTIES.md`, `EVALUATION.md`).
  `property_kinds.rs` gains `values/PROPERTIES.md`.
- **Mutants:** `M-3NN`.
- **Catalogue areas:** `P-CID`, `P-CCHD`, `P-CCNT`, `P-CBND`, `P-CEQ`, `P-IVAL`, `P-IMEM`,
  `P-IREL`, `P-ISET`, `P-IMEA` and `P-IEXP`.

**Rationale.**

- One suite for "the value types" fits VERIFIED_TYPES step 3. Later value types
  (`r1::Interval`, `Rect`, `ChordAngle`) can join it without a new feature.
- The prefixes `cl_` and `iv_` match no existing harness path. That was checked against all
  172 hand-written harness names and every module name. The 003 prefixes were chosen the
  same way.
- One records directory per suite follows `cell_union/` and `decoding/`.

**Alternatives.**

- Two features (`cell`, `interval`): more flags for no gain, because the two parts already
  select by prefix.
- Records split per type: two more files that `property_kinds.rs` must list, for the same
  content.

## R3: Reference descriptions

**Decision.**

- **`cell_spec.rs`** builds on 001's `spec.rs`. It takes a cell's face, level, orientation
  and (i, j) range from the model's decomposition. It defines the u-v bounds by upstream's
  quadratic `STtoUV` (`s2coords.h:324–327`, the default projection, `:291`) and `IJtoSTMin`,
  written in upstream's operation order.
- **`arc_spec.rs`** describes an interval as the set of points it denotes, written from the
  `S1Interval` header. It also holds `upstream_length`, the three lines of `GetLength`.
- Both are listed in data-model §1.

**Rationale.**

- **The cell.** 001's model already gives everything integer about a cell, and P-CID-01 needs
  nothing new. The bounds are floating-point values defined by one upstream expression.
  Restating that expression from upstream, not from `coords.rs`, makes "bit for bit" a
  parity statement. The model's `st_to_uv` and the library's are separately written copies
  of one formula, so a change to the library's copy, such as a reordered product, is
  caught.
- **The interval.** A pointwise description is independent of the library's case analysis
  over inverted and non-inverted bounds. The pilot's laws were consistency checks among the
  library's own operations. Under FR-007 each relation now also gets a meaning:
  - soundness, for every point;
  - exactness, against the relation of real arcs, decided by the cyclic order of the bounds
    with exact float comparisons. A first draft used finite float witnesses for
    completeness; the C++ vectors refuted it in T010, because `[next_up(−π), π]` holds
    every float but does not contain the full circle (data-model §1.2). P-IREL-05 checks
    that the real-arc relations are consistent with point membership.
- **`upstream_length`** has to be a restatement. The FR-020 containment proof says "the same
  as upstream's arithmetic everywhere else", and the arithmetic is the statement.

**Alternatives.**

- An exact-real model of arcs, with lengths in exact arithmetic. CBMC has no exact reals, and
  modelling them in bit-vectors would cost more than the library's own code. The
  floating-point points *are* the domain, so a set of floats is the right semantics.
- Comparing with the library's `coords::st_to_uv`. That restates the code under test
  (FR-007).

## R4: Domains and generators

**Decision.**

- **Cells:** the domains come from 001's `domain::any_valid()` and `any_valid_at(L)`.
- **Intervals:** struct literals over two symbolic `f64`, constrained by `arc_spec::valid`.
- **Points:** symbolic `f64` constrained by `arc_spec::in_range`.
- **Margins and error bounds:** every `f64` for crash freedom.
- **Equality:** floating-point equality in properties is bit equality.
- **No stubs.** Contract: harness-conventions.

**Rationale.**

- The domains do not depend on the code under test, as in 001–003.
- Struct literals reach intervals that no constructor returns. The public fields make them
  reachable to every caller, so FR-004 includes them.
- Bit equality keeps `−0.0` and `0.0` apart. Both appear, for example in `[0, −0]`, whose
  length is `−0.0`.

**Alternatives.** Building intervals through `Interval::new` would narrow the domain to its
normalised outputs. Struct literals cover every valid pair, which is the FR-004 domain.

## R5: The FR-020 fix and the proof that contains it

The user asked for "a proof to contain the correction to certain values only".

**Decision.**

- **The fix** changes only `s1::Interval::length`. Upstream's arithmetic runs first,
  unchanged. Only where it gives no positive length does the function tell the empty
  interval apart from the rest: the empty interval keeps −1, and anything else gets the
  stable form `(hi + π) − (lo − π)`. The rustdoc states the deviation and its reason.
- **The proof (P-IMEA-01, exact output, `iv_mea::p_imea_01`).** For every valid interval,
  including NaN-free struct literals that no constructor returns:
  - if it is not `CORNER`, `length().to_bits() == arc_spec::upstream_length(x).to_bits()`;
  - at `CORNER`, `length() == ULP_PI`.

  This is the containment proof: the correction changes the result for exactly one valid
  interval, and to the true value.
- **Uniqueness (P-IMEA-03, model check, `iv_mea::p_imea_03`).** For every valid, non-empty
  interval, `upstream_length(x) < 0` implies `x = CORNER`. Together with P-IMEA-01, this
  shows that the fix corrects every interval upstream gets wrong, and only those.
- **The law (P-IMEA-02, `iv_mea::p_imea_02`).** The length of a non-empty interval is
  non-negative, and positive unless the interval is a single point. The empty interval's is
  −1, and the full one's is 2π. The pilot had to carry an exception here; the fixed library
  proves the law as stated.
- **The callers.**
  - `expanded`, `intersection`, `approx_eq_with`, `approx_eq`, `Rect::area`, `Rect::size`,
    `Loop`'s bound test (`s2loop.rs:382`) and the other `lng.length()` uses are unchanged
    code. Their results can differ from upstream's only in calls where `length` is
    evaluated on `CORNER`, and P-IMEA-01 is what proves that.
  - The regression tests pin the three visible cases: `expanded(3.2)` is full,
    `approx_eq_with(empty, CORNER, 0.0)` is false, and the rectangle's area is positive.
  - The differential vectors check that every other row still matches C++ (R10).
- **Non-vacuity.** There are two dedicated mutants:
  - `seam-fix-widened` applies the stable form to every inverted interval. That changes
    the last bit for about 22% of inverted intervals, and P-IMEA-01 must
    fail on it with a concrete one. The 22% figure comes from 10⁶ random inverted
    intervals, measured while the spec was clarified. This is what shows that the proof,
    not the code review, holds the fix in place.
  - `seam-fix-removed` takes the fix out. P-IMEA-01 and P-IMEA-02 must fail.
- **Tests.**
  - `core/src/s1/interval_tests.rs` (new sibling file) covers the three visible results
    above, plus the length itself. Each fails without the fix (Principle III).
  - `python/tests/test_interval.py` checks `S1Interval.from_point_pair(math.pi,
    math.nextafter(-math.pi, 0)).length()`. The binding inherits the fix (Principle IV),
    and wasm exposes no `s1::Interval`.
- **Records.** `UPSTREAM.md` U-007 (R1). The vectors' corner rows are marked
  `deviation:seam-length` (R10).

**Rationale.**

- The fix is narrow, so parity holds everywhere else (Principle I), and the proof turns that
  claim into a checked fact.
- R0 ran P-IMEA-01 and P-IMEA-03 against a restated copy of the proposed fix before any
  library change. So the design is known to be provable, and known to be minimal.

**Alternatives.**

- The stable form everywhere: simpler code, but 22% of inverted lengths would lose
  bit-identity with C++.
- A proof over each caller separately: it would restate `expanded` and `approx_eq_with` in
  the reference, which FR-007 rules out. It would add nothing that "only `length` changed,
  and only at one input" does not already give.

## R6: The FR-018 fix (deserialising a `Cell`)

**Decision.** `Cell` keeps `#[derive(Serialize)]` and gets a hand-written `Deserialize`:

- It reads the same five fields through a private shadow struct with the derived format.
- It rejects an identifier that is not a valid cell.
- It rejects a face, level or orientation that differs from the identifier's.
- It rejects a u-v bound more than one ulp from the identifier's.
- Otherwise it returns `Cell::from_cell_id(id)`, the exact consistent cell.

The error messages name the field.

**Rationale.**

- **The evidence (2026-10-12, release build).** Through `serde_json` with its default
  float parsing, the derived round trip changed at least one bound of 375,836 of 1,598,279
  cells (every cell to level 7 on all faces, plus about 1.6 million random deeper cells), by
  exactly one ulp, never more.
- **Why not exact equality.** It would reject 23% of honest round trips through the most
  common text format.
- **Why not ignore the stored bounds.** That would accept arbitrary bounds silently, against
  "reject what is not a valid value" (the `CellUnion` decision).
- **One rule for both ways in.** Rejecting an identifier that is not a valid cell is 003's
  `DEV_VALID_ID` for `Cell`'s binary decoder (`decoding/PROPERTIES.md`, `wire_spec`). FR-018
  applies the same rule to serde, so both ways of deserialising a `Cell` share one stated
  deviation.
- **The one-ulp tolerance.** It absorbs the parser's documented imprecision (serde_json's
  `float_roundtrip` feature exists for this reason). Rebuilding from the identifier
  restores the exact bounds, so "a consistent cell deserialises unchanged" holds bit for bit.
- **The spec.** It was amended: FR-018 and SC-009 now state the tolerance and the
  canonicalisation, and the K2 context records the measurement.
- **Python.** The binding exposes no serialisation of `Cell`, so FR-018 needs no Python
  test.

**Alternatives.**

- Enabling `float_roundtrip` in the library's dependency on `serde_json`: the library does
  not depend on `serde_json`. Users pick their own formats.
- A wider tolerance: there is no measured need. One ulp is the observed maximum, and
  anything wider accepts edits.

## R7: The FR-017 and FR-019 documentation

**Decision.**

- **`Cell` (FR-017).** `from_cell_id`, `from_face`, `from_face_pos_level` and
  `From<CellId>` each get a `# Panics` section. The identifier must be a valid cell, and
  the null identifier or a face above 5 panics. `from_face` and `from_face_pos_level` panic
  through their identifier constructors, which already document it; the section says so.
- **The interval (FR-019).** `new`, `from_point_pair`, `contains`, `interior_contains`,
  `add_point` and `project` each get a `# Panics` section. In debug builds they panic when a
  point lies outside [−π, π] (and `project` also on an empty interval). In release builds
  `new` returns an interval that is not valid, and the others return an unspecified
  result.
- **`expanded` (K5):** a `# Panics` section. In debug builds a NaN margin panics, and in
  release builds it returns an interval that is not valid. Every other margin, including the
  infinities, is fine (R0).

**Rationale.** Constitution II requires `# Panics`. The wording follows 003's FR-020 to
FR-022 additions.

## R8: The stored-bound readers of `Cell` (FR-002)

**Decision.**

- **`uv_coord_of_edge`** is in scope with a whole-domain proof (P-CBND-03, 26 s).
- **`bound_uv`** is in scope:
  - its crash freedom is that of `from_cell_id`;
  - its properties P-CBND-01 (exact output against the restated projection) and P-CBND-02
    (ordered, within [−1, 1]) are proved per level, as `__l<L>` families;
  - P-CCHD-02 (children's bounds) is proved the same way.
- **Tiers.**
  - Per change: the members that R0 or the implementation measures under 2 minutes, which
    is about levels 0–3 for P-CCHD-02 and 0–8 for the other two.
  - Weekly (`make verify-slow F=values`, 60 minutes per member): the next levels, until a
    member times out.
  - Each entry's status is **proven within stated bounds (levels 0–L)**, with `L` set by
    measurement. The levels above are **not proven (tool limit)**, with the measurements in
    the evaluation.
- **The complement for the levels the checker cannot reach.**
  - `verify/tests/uv_projection.rs` checks every boundary value `k ∈ [0, 2^30]`
    exhaustively, by enumeration. The library's projection equals upstream's formula bit
    for bit, and the two scalings agree (R0: 0 differences).
  - With P-CID-01's integer facts, this leaves one link unproved above level L: that the
    stored bound *is* the projection of the identifier's (i, j) range. The `CELL_FIELDS`
    and `CELL_CHILDREN` vectors test that link at every level (R10).
  - The evaluation states this split plainly.

**Rationale.**

- The cost comes from squaring a symbolic float, so CBMC cannot avoid it.
- Splitting by level shrinks the significant bits of `s`, which is why low levels prove.
  No split covers level 30 within budget: at level 30 every bit of `s` is free.
- Enumeration is complete for the float lemma, because its domain has only 2^30 + 1 values.

**Alternatives.**

- **Excluding both readers as tool limits (FR-002's fallback):** this would throw away
  proofs that do fit.
- **Stubbing `st_to_uv`:** forbidden (harness conventions). It would also replace the code
  being verified.
- **Proving the link through the library's internals:** that needs either hooks in `core/`
  or a restated `ij_level_to_bound_uv`, which FR-007 and FR-011 rule out.

## R9: The five operations FR-003 left to planning

**Decision.** All five are in scope.

| Operation | Crash freedom | Properties |
|-----------|---------------|------------|
| `center` | ✅ R0 | P-IMEA-04 ✅ 30 s |
| `complement_center` | ✅ R0 | P-IMEA-05 ✅ 30 s |
| `approx_eq_with` | ✅ 1.3 s, every error bound | P-IEXP-02 reflexivity ✅ 254 s. Symmetry is measured in implementation |
| `expanded` | ✅ 91 s for every margin except NaN. NaN is K5's documented precondition | P-IEXP-01's validity clause ✅ (same harness). Its containment clause timed out off the corner. It is split by the margin's sign and by inversion in implementation, and goes to the weekly tier if a member needs it. If none fits, that clause is **not proven (tool limit)**, and its upstream test cases go into the vectors |
| `directed_hausdorff_distance` | ❌ until D-005 is fixed | P-IMEA-06: the range clause proves for an inverted `y` (137 s). The non-inverted case is split by `x`'s inversion. The 0 and π clauses are re-measured after D-005 |

**D-005's fix** is a separate commit, under FR-012:

- `directed_hausdorff_distance` builds its helper arcs with `Interval::new`, the
  normalising constructor, as upstream does.
- **The regression test** in `interval_tests.rs` takes `x = [1, 2]` and `y = [0, 1e−17]`.
  It panics in a debug build without the fix, and returns 2 with it.
- **Parity.** Release results do not change:
  - `Interval { lo: a, hi: −π }` with `a ≠ π` is inverted, so it contains a normalised point
    `p` exactly when `p ≥ a`, as `[a, π]` does.
  - `Interval { lo: −π, hi: b }` contains `p` exactly when `p ≤ b`. That differs from
    upstream's `[π, b]` only at `p = π`, and the function never asks it about π: its `x.lo`
    would then be π, and `x` would contain the complement's centre and return earlier.
  - The vectors confirm it (R10).
- While the fix is open, `known-defects.txt` lists `iv_crash::c_directed_hausdorff_distance
  D-005`.

**Rationale.** FR-003 asked for these five to be measured first. Four prove whole. The fifth
found a defect, which is the point of crash proofs.

## R10: C++ reference vectors and differential tests (FR-021)

**Decision.**

- **The generator.** A new `tools/refgen-cpp/generate_values.cc` writes
  `core/tests/data/cpp_values_vectors.csv`, with these sections:
  - `S1INTERVAL_OPS`: about 5,000 rows. Random valid intervals, including struct-literal
    ones, and points, with every in-scope operation's result. Floats are written as hex
    bits.
  - `S1INTERVAL_SPECIAL`: the empty, full, single-point and quadrant intervals of
    upstream's test, both forms of the seam point, `CORNER` and its neighbours, and margins
    at 0, ±π, ±2π and the subnormals.
  - `CELL_FIELDS`: about 5,000 rows. Random valid identifiers at every level, with face,
    level, orientation and the bit patterns of the bounds.
  - `CELL_CHILDREN`: about 1,000 rows. Each child's identifier, orientation and bounds.
- **The differential test.** `core/tests/cpp_values_diff.rs` checks the library against every
  row. Rows whose C++ result depends on the corner's length are marked
  `deviation:seam-length`, as 003 marked `deviation:reject`, and are checked against the
  documented value.
- **The sanity test.** `verify/tests/values_spec_vs_cpp.rs` checks both reference
  descriptions against the same rows.

**Rationale.** This is Constitution I's oracle pattern and 003's layout. The existing
`CELL_AREA` and `CELL_VERTICES` sections cover the float half of `Cell`, which is out of
scope.

**Alternatives.** Extending `cpp_test_vectors.csv`: that file belongs to the older generator,
and mixing the two layouts would complicate both readers.

## R11: Non-vacuity

**Decision.** About 34 `M-3NN` mutants, at least one per proven entry (data-model §7). There
are also two model-check mutants against the reference descriptions:

- `arc_spec::member` ignoring the seam: P-IMEM-01 must fail;
- `contains` without its `ccw_le(x.lo, y.hi, x.hi)` clause: P-IREL-05 must fail.

Crash harnesses carry covers for the reachable outcomes that FR-009 names:

- inverted and non-inverted, empty and full intervals;
- the corner;
- a leaf cell, and a cell with children.

**Rationale.** These are 001's and 003's rules. The FR-020 pair is in R5.

## R12: Budget and CI

**Decision.**

- **Per change:** CI runs the values suite as two legs, `values-cell` (`P=cell`) and
  `values-interval` (`P=interval`), each with the existing 30-minute job limit.
- **Weekly:** `make verify-slow F=values` gains the members R8 and R9 put there. The weekly
  `slow` job gains a `values` leg, one harness at a time, 60 minutes each.

**Estimate**, from R0's contended times, on the 4-vCPU runner with 4 jobs:

| Leg | Harnesses | CPU time | Wall-clock |
|-----|-----------|----------|------------|
| `values-interval` | about 30 | 25–35 min. The heaviest are P-IMEA-01 at 217 s and P-IMEA-02 at about the same, P-IEXP-02 at 254 s, and the pilot's ten laws at 2–141 s | 7–9 min |
| `values-cell` | about 25 | 15–20 min: the integer properties (pilot: 10 s and 125 s) and the per-change `__l<L>` members | 4–6 min |

The implementation re-measures each leg on the CI runner and moves members to the weekly
tier until both legs stay under SC-005's 10 minutes.

**Rationale.** One leg would be 12–15 minutes, over SC-005. 003 split decoding the same way.

**Alternatives.** Moving P-IMEA-01 to the weekly tier: rejected. The containment proof is the
regression guard for FR-020's deviation, and it must run on every change to
`s1/interval.rs`.

## R13: Trusted base additions (FR-015)

**Decision.** `TRUSTED_BASE.md` gains a values section:

- **CBMC's IEEE-754 model.** Bit-precise binary64 with round-to-nearest-even: addition,
  subtraction, multiplication, division and comparisons. This is new to this suite; 001–003
  had no float arithmetic on a proof path except 003's validity checks.
- **The standard library's float functions on proof paths**, as Kani models them:
  - `f64::midpoint` (`center`);
  - `f64::round` (`ieee_remainder`, in `expanded` and `approx_eq_with`);
  - `f64::abs`, `to_bits` and `from_bits`.

  Each harness that reaches them is listed.
- **Reliance on 001.** `spec.rs` (the cell model) and `domain::any_valid*`. The `CellId`
  results that a `Cell` property uses instead of re-proving are `children`, `contains`,
  `intersects`, `level` and `to_face_ij_orientation`, through P-HIER, P-ANC and P-VAL.
- **Reliance on 003.** The binary codec of `Cell`, by reference only.
- **The `__l<L>` families**, if R8 uses them, as a stated split. A split is not a bound.
- **Tests, not proofs.** `serde_json`'s parsing behaviour (R6) is a test input. The
  exhaustive `uv_projection` check (R8) is complete for its finite domain, but it is a test,
  and the trusted base says that the link from a cell's identifier to its stored bounds
  above level L rests on it and on the vectors.

## R14: The evaluation (FR-016)

**Decision.** `verify/values/EVALUATION.md` records the following:

- the effort, the harness times and memory, and the floating-point cost by operation mix
  (from R0 and the implementation);
- the findings (K1–K4 and R6's ulp drift);
- a comparison with 001–003;
- the FR-016 questions:
  - go or no-go for `r1::Interval` (likely go: the pilot proved its laws in seconds) and
    for the rectangle (go for its integer-free laws; its area and its bounds from points
    need products);
  - `ChordAngle`: no-go until its validity check is settled, which is a type-level
    invariant question;
  - `Point`: ordering only;
  - whether type-level invariants should come first. The evidence from this feature is
    that public fields forced every interval proof to assume validity, and that a
    struct-literal domain is what reached the corner.

**Rationale.** SC-007: a maintainer must be able to decide from the evaluation alone.

## R15: Reusing the trait properties of earlier specs

Added after `/speckit-analyze` (2026-10-12), at the user's request: "check that we reuse the
trait properties from previous specs".

**Decision.** Trait laws are stated once and instantiated per type, at two layers.

- **The binary codec** stays 003's generic `Codec` contract (`verify/src/codec.rs`, `CellC`).
  004 refers to it and does not re-prove it.
- **The `CellId` facts** of 001 (P-HIER, P-ANC, P-VAL) and its generators are reused as the
  trusted base names them (R13).
- **Kani layer (new `verify/src/laws.rs`, data-model §4.3).**
  - `laws::region` holds the integer `Region` laws: containing a cell implies intersecting
    it, and the covering bound covers every contained cell.
  - `laws::eq` holds the equivalence laws of `PartialEq`.
  - 004 instantiates them for `Cell` (P-CCNT-03, P-CEQ-01) and the interval (P-IVAL-05).
  - A follow-up task instantiates `laws::region` for `CellUnion`, within 002's size bounds.
    002's P-UMEM-05 already implies it, through the model.
- **Quickcheck layer (Constitution III, `CORE_TRAITS.md`).**
  - `core/tests/trait_property_tests.rs`, the generic property tests that `CORE_TRAITS.md`
    names, is brought onto this line. It was only on `tb/python-bindings/1` (commit
    `f9805f8`).
  - The file compiled unchanged against the current tree and gained the SPDX header for
    original tests. 109 of its 110 tests passed. The one failure is D-006 (below).
  - 004 adds `Cell`'s missing instances of the integer `Region` laws.
  - `CORE_TRAITS.md`, which the constitution cites, is still untracked. It is committed with
    the tests, after the update its own task note asks for (`CORE_TRAITS_PROPTEST.md`:
    "Super traits etc changed").

**D-006 (a port defect, found by the brought-in tests).**

- **The defect.** `Rect::cap_bound` tried the centre cap when
  `(hi − lo).rem_euclid(2π) ≥ 0 ∧ hi − lo ≤ π`, a port of Go's `math.Remainder(...) ≥ 0 ∧ ...`.
  `rem_euclid` is never negative, so every rectangle whose longitude crosses the antimeridian
  took the centre cap, whatever its width. Upstream tests `lng_.GetLength() <= M_PI`
  (`s2latlng_rect.cc:323`).
- **The failing input.** `region_union_cap_contains` found a 248° rectangle whose centre cap
  is a hair smaller than the pole cap, and misses its western side.
- **What it reaches.** `Rect::cap_bound` feeds `Rect::cell_union_bound` and the cap bounds of
  `Loop`, `Polygon`, `Polyline` and `RegionUnion`, so coverings of such rectangles could miss
  cells.
- **The fix**, applied to the working tree as its own change: upstream's condition,
  `self.lng.length() <= PI`. This restores parity (Principle I).
- **Tests.** `core/src/s2/rect_tests.rs` (new sibling file) holds the failing rectangle,
  which failed before the fix, and a narrow antimeridian rectangle that keeps the centre cap.
- **Results with the fix:**
  - the generic tests: 110 of 110, also at 5,000 cases each;
  - the core suite: 3,738 passed, 0 failed;
  - the Python suite: 912 passed;
  - clippy and fmt: clean.
- **Interaction with FR-020.** The corner's length (−1 now, one ulp after FR-020) is at most
  π either way, so FR-020 does not change which branch `cap_bound` takes.

**Rationale.**

- VERIFIED_TYPES §3 and step 4 set the pattern: one generic statement per trait law,
  instantiated per implementor.
- Constitution III requires `CORE_TRAITS.md`'s invariants as quickcheck properties where
  practical.
- Without the generic layer, 002 and 004 each restated the `Region` laws for one type, and
  equality got a law for `Cell` but not for the interval (analysis findings C1, C2, I1).

**Alternatives.**

- Re-expressing 002's `CellUnion` harnesses now: it would re-run 002's proven, size-bounded
  members inside this feature. It is a follow-up task.
- Leaving the generic tests on the other branch: the constitution names them, and they found
  a coverage defect on their first run here.
