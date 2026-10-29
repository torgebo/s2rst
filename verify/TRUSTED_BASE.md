# Trusted Base (FR-013)

Everything the `CellId` proofs rely on without proving it. A proof in this directory means
"Kani found no counterexample within the bounds below, assuming everything on this page".
Design: `specs/001-formal-verify-base-type/` (data-model §7, research R6, R12, R15, R18).

## Tools

| Component | Version | Role |
|-----------|---------|------|
| Kani (`cargo kani`) | 0.68.0 (pinned in the `Makefile` and `.github/workflows/verify.yml`) | Compiles the harnesses to CBMC's goto programs and drives CBMC |
| Kani's compiler toolchain | `nightly-2026-08-21` (rustc 1.100.0-nightly `8925ea358`), installed by `cargo kani setup` | Produces the MIR that is verified. Release builds use stable rustc 1.98.1, so the two compilers are trusted to agree on the semantics of this code |
| CBMC | 6.11.0, from the Kani 0.68.0 bundle | Bounded model checking: symbolic execution, then a SAT query per harness |
| SAT solver | CaDiCaL, Kani's default, for every harness | Decides each query. No harness overrides it (`grep -rn kani::solver verify/src` is empty) |

The known bugs of these tools, with reproducers and the workarounds below, are in
`KANI_ISSUES.md` at the repository root.

Kani builds with debug assertions and overflow checks on. A `debug_assert!` that can fail,
or integer arithmetic that can overflow, is therefore a failed check. A release build
compiles both out, but on a proven domain no overflow happens, so release computes the same
values. The proofs say nothing about release behaviour outside their domains.

## Models

- **Kani's models of `std`:** heap allocation, `Vec` (`vertex_neighbors` returns one) and
  `&str`/byte-slice access in the parsers. Kani substitutes its own implementations for
  some of these; the proofs trust them to match `std`.
- **rustc MIR semantics** as Kani translates them, including integer wrapping and shifts and
  the `wrapping_*` methods.
- **CBMC's bit-precise IEEE-754 semantics** for the floating-point path that
  `edge_neighbors` and `vertex_neighbors` take to cross a cube-face edge (research R12).
  The proofs trust that CBMC models `f64` arithmetic (round-to-nearest, no fast-math) as
  the hardware computes it.

Kani 0.68.0 reports two constructs it does not support in the compiled code: one
`caller_location` and one foreign function (where they sit was not traced). Kani replaces
each with a failing check, so a harness that reached one would fail. No successful harness
reaches them.

## The text-domain argument (research R6)

`from_token`, `from_debug_string` and `FromStr` read their input only as bytes. Each parser
harness feeds every byte string of one length `L` through the one `unsafe` helper in
`verify/`, `domain::with_text`, which calls `std::str::from_utf8_unchecked` without checking
UTF-8. That domain is a superset of every `&str` of length `L`, so a proof over it covers
all real inputs. Trusted:

- that the parsers inspect bytes only, so a byte string that is not UTF-8 cannot change
  their result on one that is (checked by reading the code);
- that Kani does not exploit the UTF-8 invariant, which the helper breaks for non-UTF-8
  bytes. A counterexample made of invalid UTF-8 would be spurious. Check a counterexample's
  bytes before reporting a defect. None has occurred: every parser harness succeeds.

## The reference model

`verify/src/spec.rs` restates the `CellId` encoding from upstream S2 independently of the
implementation: a search-based `decompose`, the Hilbert tables copied from upstream
`s2coords_internal.h`, and the text formats of C++ `FromToken`, `ToToken`, `ToString` and
`FromDebugString`. Every functional property (`PROPERTIES.md`) is stated against it.

It is validated only by sampling. `verify/tests/spec_vs_cpp.rs` checks it against the
committed C++ vectors (`core/tests/data/cpp_cellid_vectors.csv`: the `CHILDREN`, `RANGE`,
`CONTAINS`, `FACE_IJ`, `CELLID_FROM_TOKEN`, `CELLID_FROM_DEBUG_STRING` and
`CELLID_TO_STRING` sections), plus hand-written cases and model round trips. A mistake that
the model and the implementation share, and that the vectors miss, would go unnoticed.

The domain generators in `verify/src/domain.rs` build valid ids through the model's
`compose`. `val::p_val_01`, `p_val_02` and `p_val_03` together prove that this produces
exactly the ids `is_valid()` accepts (`spec::valid(x)` is `decompose(x).is_some()`), so the
generated Valid domain is neither too narrow nor too wide.

## Bounds and gaps

### Loop bounds

Kani's unwinding assertions are on (the default), so a bound that is too small fails the
harness. These bounds are checked, not assumed:

| Bound | Harnesses |
|-------|-----------|
| `unwind(40)` | the parser families `enc::p_enc_06__b*`, `enc::p_enc_09__b*`, `enc::p_enc_10__b*` |
| `unwind(32)` | `val::p_val_01`, `p_val_02`, `p_val_03`, `p_val_05`; `range::p_range_06`; `anc::p_anc_01` (the model's 30-level searches) |
| `unwind(31)` | `crash::c_maximum_tile`; `enc::p_enc_01`, `p_enc_02`, `p_enc_04` (the Hilbert model) |
| `unwind(10)` | the neighbour harnesses: `crash::c_edge_neighbors__f*`, `crash::c_vertex_neighbors`, `nbr::p_nbr_01__f*`, `nbr::p_nbr_02__f*`, `nbr::p_nbr_03__q*` |
| `unwind(9)` | `crash::c_get_center_si_ti`, `c_to_face_ij_orientation`, `c_from_face_ij`; `enc::p_enc_03` (the 8-step lookup loops) |

### Input-size bounds (FR-007)

The three parsers are `proven-within-bounds`, not `proven`:

- `from_token` is proven for every byte string of length 0–17 (`enc::p_enc_06__b0…b17`);
- `from_debug_string` and `FromStr` are proven for lengths 0–33 (`enc::p_enc_09__b*`,
  `enc::p_enc_10__b*`).

Longer inputs are rejected by a length check before any loop. That step is by reading the
code, and the FR-017 robustness tests (`core/tests/decoder_robustness.rs`) and the
`parse_cell_id_text` fuzz target exercise it. It is not proven.

### Domain preconditions

Each crash proof covers its operation's documented domain, not every input. The
preconditions, with their sources, are in the `preconditions` column of `INVENTORY.md`. Two
come from upstream findings (FR-018):

- `level()` excludes ids with 62 or more trailing zero bits, including `0`: upstream C++
  assumes a non-zero id and returns −1 for the others (**U-001**, follow-up task T054).
- `common_ancestor_level` excludes (`none`, `none`): C++ is undefined behaviour there, and
  Go returns level 30 (**U-002**, follow-up task T055).

`s2::from_face_ij` is proven for `i, j ∈ [0, 2^30)`, the range upstream documents. Outside
it, Rust panics in debug builds where C++ wraps: defect D-001 (`EVALUATION.md`, task T056),
documented in the `# Panics` rustdoc.

### Exclusions and unproven items

- **Floating-point conversions** (FR-002): 12 operations, listed in `INVENTORY.md` and
  `EVALUATION.md`.
- **Tool limits:** `to_token`, `to_debug_string` and `Display` (`core::fmt` is too costly
  for CBMC), and `all_neighbors` (out of memory even for crash-freedom).
- **Not-proven properties:** P-ENC-05, P-ENC-07, P-ENC-08, P-ENC-11 and P-ENC-12 (they
  involve the excluded formatters), and P-NBR-04 (`all_neighbors`). `PROPERTIES.md` gives
  each reason.
- **The weekly slow tier:** the crash proofs of `edge_neighbors` and `vertex_neighbors`
  and the neighbour properties P-NBR-01 and P-NBR-03 run only in the weekly job
  (`make verify-slow`). A regression in either neighbour operation, even a new panic, can
  merge and be caught up to a week later. Per-change CI still runs the existing unit tests
  of both operations, and the C++ differential test of `edge_neighbors`.

### Non-vacuity

A proof is only as good as its assumptions. Crash harnesses carry `kani::cover!` checks that
`run-verify.sh` requires to be satisfied, so no assumption set is empty. Every proven
property has a mutant that makes it fail (`MUTANTS.md`), so no property is trivially true.

## `CellUnion` (spec 002)

Everything the `CellUnion` proofs rely on, in addition to the sections above, which apply
unchanged: the same tools, Kani's models of `std`, and MIR semantics. Design:
`specs/002-formal-verify-cell-union/` (data-model §7; research R3, R4, R5, R6). Records:
`cell_union/INVENTORY.md`, `cell_union/PROPERTIES.md`, `cell_union/EVALUATION.md`.

### Reliance on 001

A property is stated against `union_spec.rs`, the model of the leaf cells a union covers.
Implementation outputs are read through a few `CellId` methods. Each method's agreement with
the cell model is taken from 001's proof instead of being proven again:

| `CellId` method | Used for | 001 property |
|-----------------|----------|--------------|
| `range_min`, `range_max` | whether an output union covers a symbolic leaf (`union_domain::out_covers`) | P-RANGE-01 |
| `is_valid` | whether an output id is valid (`out_decomposed`), and covers | P-VAL-03 |
| `face`, `level`, `pos` | the decomposition of an output id, to check validity and normalisation (`out_decomposed`) | P-VAL-06 |
| `is_valid`, `range_min`, `range_max` | the validity of each id in P-UENC-03's model decode (`union_spec::decode_with` with `union_domain::impl_cell_range`), and output validity (`out_valid`) | P-VAL-03, P-RANGE-01 |
| `contains`, `intersects` | covers only (reachability, not a property statement) | P-RANGE-02, P-RANGE-03 |

Nothing is stubbed (spec 008's stubs apply only to its own harnesses). Every `CellUnion` proof executes the real `CellId` code (`parent`,
`children`, `child_begin_at_level`, `maximum_tile` where reached, `to_face_ij_orientation`
for the `Region` queries), so a bad interaction between the two types would still be found.

### The standard library

`sort_unstable` (in `normalize`), `Vec`, `slice::partition_point` (in the queries) and
`io::Read` for `&[u8]` (in the decoder) are executed, not modelled. Only Kani's allocation
model and its `std` build are trusted, as above. Assertions that compare byte vectors run
CBMC's built-in `memcmp`, which is trusted to be a correct byte comparison.

### The model

`union_spec.rs` is plain Rust, written from upstream `s2cell_union.h`/`.cc` on top of 001's
cell model and validated by sampling: `tests/union_spec_vs_cpp.rs` checks it against every
section of `core/tests/data/cpp_cellunion_vectors.csv`. Its decode is upstream `Decode`
plus the library's documented FR-017 deviation (rejecting input that is not a valid union),
not upstream's behaviour (U-003). The model's loops are written as indexed loops over
concrete bounds, which CBMC can bound exactly; that is a change of form, and the sanity
tests check the result.

### Bounds and gaps

Every proof covers unions or byte strings up to a stated size, one harness per concrete
length (research R4). The full list, with what each bound leaves unreached and the tests that
exercise it, is the `bounds` column of `cell_union/INVENTORY.md` and the `Bound` column of
`cell_union/PROPERTIES.md`. In summary:
- **Read-only operations** reach 3 or 4 cells (the invariant checks 4, which includes a
  sibling group), and the union queries 3 × 3.
- **Operations that build a union by normalising** reach 2 cells (`union`: 1 × 1). The
  sibling merge needs 4 cells and is unreached.
- **`intersection`** reaches 1 × 1, and **`denormalize`** 2 cells (properties: 1 cell)
  within a loop bound of a level gap of 2.
- **Decoding:** the crash proof covers every input of up to 24 bytes per change and 25
  bytes weekly (the slow tier, at an 8 GB ceiling). The functional property P-UENC-03
  reaches 24 bytes, one cell, so FR-017's order check is unreached by it.
- **The weekly tier:** the larger members of the property families run only weekly
  (`make verify-slow F=union`). `cell_union/PROPERTIES.md` marks them.
- **Tool-limit exclusions:** `difference`, `from_begin_end`, `from_min_max` and `from_range`
  have no proofs. The C++ differential tests and the R17 property tests check them.

### Written arguments

Three results extend beyond the proofs by argument, not by proof:
- **The leaf count of any valid union fits in `i64`** (FR-016). A valid union's cells are
  disjoint, so they cover at most all `6·4^30` leaves, and `6·4^30 < 2^63`. P-UMEAS-01 proves
  the count exact, without overflow, for valid unions of up to 3 cells.
- **The order of `normalize`'s input never matters** (P-UNORM-04). Swapping two members
  leaves the result unchanged, proven for two cells, and transpositions generate every
  permutation. The argument assumes the swap result holds at every size, which the proofs
  do not reach. `prop_cell_union_normalize_preserves_coverage` samples larger inputs.
- **`denormalize` cannot panic below the proven gap** when its own level is below
  `min_level`: that branch only sets the level to `min_level`, with no arithmetic. Its
  outputs have `4^gap` cells, so the regression test covers only gaps up to 2.

## The decoding layer (spec 003)

Everything above applies, plus:

- **From spec 001:** P-VAL-03 (`CellId::is_valid` agrees with the model's `spec::valid`),
  which the `CellId` and `Cell` decoders' validity rests on, and `Cell::from` as it executes
  in the `Cell` harnesses (its own proofs are 001's).
- **The standard library as executed:** `Read for &[u8]` (`read_exact`, `read`), `Write for
  Vec<u8>` (`write_all`), `Read::take` and `read_to_end` (the string vector), and `Vec`'s
  growth. They run unmodified; no harness stubs them.
- **The reference description** (`src/wire_spec.rs`) is trusted to state upstream's
  formats. It is written from upstream at `v0.14.0-40` and checked against every row of
  `core/tests/data/cpp_coding_vectors.csv` (P-PAR-01). Its float operations, inside
  `cap_valid` and `rect_valid`, are trusted to be the ones the library performs, in the same
  order (research R18).
- **The Verus track:** Verus 0.2026.09.20.aef82ed with its Z3 4.16.0 and Rust 1.98.1, and
  vstd's specifications of the standard library (`Vec`, slices, `u64::leading_zeros`,
  `trailing_zeros`, `wrapping_add`). The proofs hold for the restated copies in
  `deductive/src/`. Their link to the library is the bounded agreement (`src/cd_agree.rs`)
  within the bounds below, the tested agreement (`deductive/tests/agree.rs`) beyond them,
  and the drift guard, which fails when a mirrored function changes. The string vector
  decoder's link is the tested agreement alone.
- **Bounds** (the full list with what each leaves uncovered is the `bounds` column of
  `decoding/INVENTORY.md` and the `Bound` column of `decoding/PROPERTIES.md`):
  - the fixed-width helpers and the five fixed-size codecs: their whole domains;
  - the integer and cell-id vector decoders: every input of up to 11 bytes (from 9 bytes,
    respectively 5, weekly);
  - the vector encoders: up to 2 values (strings of up to 2 bytes);
  - the point vector decoder: every input of up to 1 byte, and every input with tag 2–7;
  - the string vector decoder: no Kani bound (a tool limit); the Verus proof covers every
    length of the restated copy.

- **`io::Error`'s destructor is not run by the decoding harnesses.** They end with
  `wire_domain::discard(result)`, which forgets a decoder's error instead of dropping it, and
  drops a decoded value normally. Kani 0.68.0 reports "misaligned pointer to reference cast"
  in std's `io::error::repr_bitpacked::decode_repr` (line 273, the `SimpleMessage` arm)
  when it drops some errors. The trigger is the `SimpleMessage` of a short `read_exact`,
  returned through `Point::decode` or `Cap::decode`: 56 members of T040's first run failed
  only on this check. The same error dropped straight after `read_exact`, or returned
  through `CellId::decode`, passes. A real misalignment of one static object cannot depend
  on which function returns the pointer to it, so the report comes from the checker's
  model, not from the code (probes in `decoding/EVALUATION.md`). The library builds every
  error through std's safe API (`io::Error::new` and `read_exact`), so their destructor is
  std's code and trusted here like the rest of `std`.
- **`Cap::is_valid` (research R18's fallback).** The `Cap` agreement and round trip state
  acceptance relative to the library's own predicate: the restated float predicate
  `wire_spec::cap_valid`, asserted against the library's, timed out (`p_fx_03`,
  `p_fx_08__b32`, `__b33` over 5 minutes in T048). The fields, their order, the clamp of
  `length2` and the bytes consumed are still checked against the reference description.
  `wire_spec::cap_valid` itself is checked against the C++ vectors by
  `verify/tests/wire_spec_vs_cpp.rs`.
- **The point vector's compact branch and the string vector decoder** are tool limits
  (`decoding/EVALUATION.md`, "Exclusions and tool limits"). The string vector decoder's only
  link to its Verus proof is tested agreement (`verify/deductive/tests/agree.rs`,
  FR-011's fallback); it has no bounded agreement harness.
- **The xlarge tier** (`make verify-xlarge`, the `cd_xlarge` module) is not part of any
  result until it has been run on a machine with 32 GB of memory and its runs are recorded.

## `Cell` and `s1::Interval` (spec 004)

Everything the value-type proofs rely on, in addition to the sections above, which apply
unchanged: the same tools, Kani's models of `std`, and MIR semantics. Design:
`specs/004-formal-verify-cell-interval/` (data-model §1, §2, §4.3; research R3, R4, R8, R13,
R15). Records: `values/INVENTORY.md`, `values/PROPERTIES.md`, `values/EVALUATION.md`.

### Floating-point arithmetic

This is the first suite whose properties are about floating-point values. They rest on:

- **CBMC's IEEE-754 model**: bit-precise binary64 with round-to-nearest-even, for addition,
  subtraction, multiplication, division and comparison. Every float property compares bit
  patterns (`to_bits`), so `0.0` and `−0.0` differ and nothing is approximated, except where a
  statement gives a tolerance in ulps (P-ISET-05: one ulp, for upstream's own rounding).
- **The standard library's float functions on the proof paths**, as Kani's `std` build
  implements them (executed, not stubbed):
  - `f64::midpoint` (`center`, `complement_center`; P-IMEA-04, P-IMEA-05);
  - `f64::round`, inside `ieee_remainder` (`expanded`, `approx_eq_with`; P-IEXP-01,
    P-IEXP-02), an intrinsic that CBMC models;
  - `abs`, `to_bits`, `from_bits`, `next_up`, `next_down` (the reference descriptions).

### Reliance on 001 and 003

| Used | For | Taken from |
|------|-----|------------|
| `spec.rs` (the cell model), `domain::any_valid`, `any_valid_at` | every `Cell` domain and the model of a cell (`cell_spec`) | 001's model and generators, sanity-tested by `tests/spec_vs_cpp.rs` |
| `CellId::to_face_ij_orientation` equals the model's walk | the `(i, j)` range in the bound families (`cl_bnd`), to avoid the model's 30-step walk there | P-ENC-01 |
| `CellId::contains` equals the model's containment | the covering law (`laws::region::covering_covers`) | P-RANGE-02 |
| the binary codec of `Cell` | not re-proven: `decoding/INVENTORY.md` | spec 003 (`CellC`) |

`Cell`'s proofs execute the real `CellId` code (`to_face_ij_orientation`, `level`,
`child_begin`, `next`, `contains`, `intersects`, `center_uv`), so a bad interaction between the
two types would still be found.

### The reference descriptions

- `cell_spec.rs` restates upstream's quadratic `STtoUV` and `IJtoSTMin` in upstream's
  operation order (`s2coords.h:324–327`): the bounds are defined by that expression, so bit
  equality with it is parity with upstream.
- `arc_spec.rs` describes membership as a property of float points, and the relations between
  intervals as those of real arcs, decided by the cyclic order of the bounds with exact float
  comparisons (a float-free gap is still a gap: data-model §1.2).
- `arc_spec::upstream_length` is the one restatement of upstream code by design: it is the
  statement of P-IMEA-01 (upstream `GetLength`, `s1interval.cc:41–47`).
- Both are tested against every row of `core/tests/data/cpp_values_vectors.csv` by
  `tests/values_spec_vs_cpp.rs`, and by hand-made unit tests.

### Bounds and gaps

- **The level families.** P-CBND-01, P-CBND-02 and P-CCHD-02 square a symbolic float and are
  proven per level, up to the levels in `values/PROPERTIES.md`. Above them, two things stand
  in for a proof:
  - `tests/uv_projection.rs` checks every boundary value `k ∈ [0, 2^30]` exhaustively: the
    library's projection is bit for bit upstream's formula, and the centre and edge scalings
    agree. That is complete for the float step, but it is a test.
  - The link from a cell's identifier to the `k` of its stored edges above those levels is
    the library's code (`ij_level_to_bound_uv`, `children`), checked at every level only by
    the `CELL_FIELDS` and `CELL_CHILDREN` rows of the C++ vectors.
- **Splits, not bounds.** The `__xinv`/`__xnoninv`, `__yinv`/`__ynoninv` and margin-sign
  splits of P-IEXP-01 and P-IMEA-06 together cover their whole domains.
- **Preconditions** are the documented ones (FR-004): a valid cell identifier (FR-017); points
  in [−π, π], and a non-empty interval for `project` (FR-019); a margin that is not NaN for
  `expanded` (K5).
- **Excluded**: `Cell`'s 31 floating-point geometry entries and both types' `Display`
  (`values/INVENTORY.md`).

## `r1::Interval` and `s2::Rect` (spec 005)

Everything the real-interval and rectangle proofs rely on, in addition to the sections above,
which apply unchanged, spec 004's included. Design: `specs/005-formal-verify-r1-interval-rect/`
(data-model §1, §2; research R0, R3, R4, R5, R11). Records: sections 3–4 of
`values/INVENTORY.md`, sections 4–5 of `values/PROPERTIES.md`, and the spec-005 part of
`values/EVALUATION.md`.

### Reliance on 004

| Used | For | Taken from |
|------|-----|------------|
| `arc_spec` (the arc description: `valid`, `empty`, `full`, `inverted`, `member`, `interior`, `contains`, `interior_contains`, `intersects`, `interior_intersects`) | every rectangle statement about longitudes, through `rect_spec` | spec 004, unchanged; sanity-tested against the `S1INTERVAL_*` rows, and now also against the `RECT_*` rows through `rect_spec` |
| `laws::eq` | P-RIV-02, P-LEQ-01 | spec 004 |
| the generator pattern (struct literals filtered by a description, never by `is_valid`) | `any_real`, `any_rect` | spec 004 research R4 |

The rectangle's proofs do **not** take 004's proofs of the circular interval as lemmas: there
is no stubbing (contracts; spec 008's stubs apply only to its own harnesses). Every rectangle proof executes the circular interval's real code
(`contains`, `union`, `intersection`, `expanded`, `approx_eq_with` with `ieee_remainder`, the
debug checks of `new` and `new_checked`), so a bad interaction between the two types would
still be found. The reuse is of the description: a rectangle statement is the product of
`real_spec` and `arc_spec`, never the conjunction of the components' operations (research R0
measured that form at a 12-minute timeout, R5).

### Floating-point arithmetic

- **Kani's NaN checks are part of the crash definition.** CBMC reports "NaN on subtraction"
  (or addition, multiplication) when an operation on non-NaN operands produces NaN (`∞ − ∞`,
  `0 × ∞`). The real interval's arithmetic meets `∞ − ∞` at its documented infinite corners
  (FR-017), so its crash harnesses assume exactly those inputs away (`ri_crash`), and its
  arithmetic properties take finite bounds. What the library does there is documented, and
  checked by the `R1INTERVAL_SPECIAL` rows of the C++ vectors, not proven.
- **More of the standard library on the proof paths**, as Kani's `std` build implements them
  (executed, not stubbed): `f64::min` and `f64::max` (union, intersection, `add_interval`, the
  Hausdorff distance), `f64::clamp` (`project`, whose assertion is an obligation) and
  `f64::midpoint` (`center`).
- **Signed zeros.** The real interval's properties compare bounds by value where `f64::min` and
  `max` choose between `0` and `−0` (their sign is not fixed); the differential test documents
  the same (`cpp_values_diff.rs`).

### The reference descriptions

- `real_spec.rs` describes an interval as the closed set of real numbers between its bounds;
  its relations are those of the sets, decided exactly by the bounds.
- `rect_spec.rs` describes a rectangle as the product of `real_spec`'s latitudes and
  `arc_spec`'s longitudes, restricted to valid latitude–longitudes, and its validity as
  upstream's `is_valid` (`s2latlng_rect.h:417`).
- Both are tested by hand (`real_spec_tests.rs`, `rect_spec_tests.rs`) and against every row of
  the `R1INTERVAL_*` and `RECT_*` sections of `core/tests/data/cpp_values_vectors.csv`
  (`tests/values_spec_vs_cpp.rs`).

### Bounds and gaps

- **Preconditions** are the documented ones (FR-004): non-NaN bounds and numbers; for the real
  interval's arithmetic, no `∞ − ∞` (FR-017); a non-empty interval for `project` (FR-019); a
  valid rectangle; a valid latitude–longitude for `from_lat_lng`, `from_point_pair`,
  `from_center_size`'s centre and `interior_contains_lat_lng` (K3, FR-019); non-NaN margins
  and sizes (FR-019). `add_point` and `contains_lat_lng` take every latitude–longitude, NaN
  included (K2).
- **Splits, not bounds.** P-LEXP-01's containment is split by the margins' sign (`__mpos`,
  `__mneg`, weekly); mixed signs make no containment claim.
- **Excluded**: the rectangle's 18 floating-point geometry entries and both types' `Display`
  (`values/INVENTORY.md` §3–4). The geometry is covered by the generic trait tests
  (`region_rect_*` in `core/tests/trait_property_tests.rs`) and D-006's `rect_tests.rs`; 004's
  `laws::region` laws, which go through `contains_cell` and `intersects_cell`, are not
  instantiated for the rectangle.

## `r2::Rect` and `s2::Point` (spec 006)

Everything the u-v rectangle and point proofs rely on, in addition to the sections above, which
apply unchanged, specs 004's and 005's included. Design: `specs/006-formal-verify-r2-rect-point/`
(data-model §1, §2; research R0, R3, R4, R5, R11). Records: sections 5–6 of
`values/INVENTORY.md`, sections 6–7 (and the order laws in section 3) of `values/PROPERTIES.md`,
and the spec-006 part of `values/EVALUATION.md`.

### Reliance on 005

| Used | For | Taken from |
|------|-----|------------|
| `real_spec` (`empty`, `finite`, `member`, `interior`, `contains`, `interior_contains`, `intersects`, `interior_intersects`) | every u-v rectangle statement, through `uv_spec`: a statement is the product of two `real_spec` statements, one per axis | spec 005, unchanged; sanity-tested against the `R1INTERVAL_*` rows, and now also against the `R2RECT_*` rows through `uv_spec` |
| `r1::Interval::expanded` and `approx_eq_with` | P-QEXP-01's result and P-QEXP-03's product clause are stated through the factors' own operations, whose meaning is 005's P-REXP-01 and P-REXP-03 | spec 005's proofs, as cited statements (research R5) |
| `any_real`, `any_number` and the generator pattern | `any_uv_rect`, `any_uv_point`, the constructors' domains | spec 005 |
| `laws::eq` | P-QEQ-01, P-PTVAL-03 | spec 004 |

As in 005, nothing is stubbed (spec 008's stubs apply only to its own harnesses): every u-v rectangle proof executes `r1::Interval`'s real code
(the comparisons, `f64::min` and `max`, `f64::clamp`, `f64::midpoint`, `expanded`,
`approx_eq_with`). The new generic laws `laws::ord` (reflexive, dual, transitive, ties are
equality) are instantiated by P-PTORD-02 and are ready for `ChordAngle` (FR-025).

### The checker's model of floating point

- **IEEE-754 arithmetic and comparison** are bit-precise, as for 004 and 005.
- **`sqrt`** (`SqrtF64`) is CBMC's C-library model, which constrains its result without fixing
  it: `x.sqrt() == x.sqrt()` fails for a symbolic `x` (spec 006, 2026-10-14). A property of one
  square root holds for every value the model allows; the base assumes that range contains the
  true root (`4.0.sqrt() == 2.0` proves, but the range was not inspected). Two square roots of
  one number may differ, so no harness compares two normalisations bit for bit: P-PTVAL-02's normalising clause and P-PTORT-01 are tool limits, checked against the
  C++ vectors instead.
- **`sin` and `cos`** are over-approximated in [−1, 1] (research R0): `rotate`'s crash proof
  holds for every value in that range, the true ones included.
- **`atan2`** is a foreign C call that Kani replaces with a failing check (research R0): the
  distance, the stable angle and the approximate equalities have no proof beyond equal points,
  where `r3::Vector::angle` returns before it.
- **The NaN checks are part of the crash definition**, as in 005, and CBMC 6.11 also reports a
  division by an infinity as "NaN on division", though IEEE-754 gives zero (research R0). So
  `from_coords` and `normalize` range over the triples with a finite squared length; per
  change over those with every coordinate at most `2⁵¹¹` (the whole set times out at 12
  minutes), weekly over the band `(2⁵¹¹, 2⁵¹²)`.

### The build

The proofs cover the default build. `r3::Vector`'s `dot` and `cross` fuse their products when
the target has hardware FMA (`cfg!(target_feature = "fma")`); Kani's build has none, so the
proofs cover the unfused form, as the C++ vectors and the property test of `distance`'s
symmetry do (K9). The verify crate refuses to build with the feature (`compile_error!` in
`src/lib.rs`), so a `-C target-cpu=native` build cannot pair the unfused restatements with the
fused library.

### The reference descriptions

- `uv_spec.rs` describes a u-v rectangle as the product of two `real_spec` sets; its validity
  is upstream's emptiness rule plus "no bound is NaN", and `emptiness_rule` alone is upstream's
  `is_valid` (K4).
- `point_spec.rs` describes the point's order (C++'s `std::lexicographical_compare`), its
  classifications, the exact power-of-two scaling and the box `[−1, 1]³`, and, in a separate
  section, restates upstream's `IsUnitLength`, `Normalize`, `Ortho`, `EnsureNormalizable`'s
  scaling and Go's `PointCross` kernel.
- Both are tested by hand (`uv_spec_tests.rs`, `point_spec_tests.rs`) and against every row of
  the `R2RECT_*` and `S2POINT_*` sections (`tests/values_spec_vs_cpp.rs`): the order, the
  restatements and the U-012 overflow, bit for bit.

### Bounds and gaps

- **Preconditions** are the documented ones (FR-004): a valid rectangle and non-NaN points;
  finite bounds and margins for the u-v arithmetic (FR-018); a non-empty rectangle for
  `project` (FR-017); every triple, NaN included, for the point's order, conversions and
  classifications; no `∞ − ∞` for its addition and subtraction; the box for its geometry.
- **Splits, not bounds.** Long statements are split into members by clause (`__*`); the members
  over the per-change budget are weekly (`values/PROPERTIES.md` says which).
- **Excluded or limited**: both types' `Display`; the five operations through `atan2`; the
  finite triples whose squared length overflows (`from_coords`, `normalize`); the comparisons
  of two normalisations (`sqrt`). Each is covered by the C++ vectors
  (`core/tests/cpp_values_diff.rs`) or ordinary tests (`point_tests.rs`, `property_tests.rs`).

## `s1::ChordAngle` and `s1::Angle` (spec 007)

Everything the chord-angle and angle proofs rely on, in addition to the sections above, which
apply unchanged. Design: `specs/007-formal-verify-chord-angle-angle/` (data-model §1, §2;
research R0, R3, R4, R6, R11, R12). Records: sections 7–8 of `values/INVENTORY.md`, sections
8–9 of `values/PROPERTIES.md`, and the spec-007 part of `values/EVALUATION.md`.

### Reliance on 004 and 006

| Used | For | Taken from |
|------|-----|------------|
| `laws::eq` | P-CAORD-01, P-AGVAL-02 | spec 004 |
| `laws::ord` | P-CAORD-01 (every canonical value), P-AGVAL-02 (angles without NaN) | spec 006, ready for this type since its FR-025 |
| `any_finite_angle` and the generator pattern | `any_chord`, `any_chord_length`, `any_angle` | spec 006 |

Nothing is stubbed (spec 008's stubs apply only to its own harnesses): every proof executes the library's code, `f64::sin`, `f64::sqrt`,
`round_ties_even` and the `%` of `normalized` included.

### The checker's model of floating point

- **IEEE-754 arithmetic and comparison** are bit-precise, as for 004–006.
- **`sin`** is CBMC's built-in model, over-approximated within [−1, 1] (spec 006). `from_angle`'s
  range [0, 4] holds for every value the model allows; its exact value at a NaN angle (4,
  through `sin(π / 2)`) is out of reach and rests on the C++ vectors (P-CAVAL-04).
- **`sqrt`** constrains its result without fixing it (spec 006): `Add`, `Sub`, `sin` and `tan`
  of a chord angle get properties of one result each, never comparisons of two.
- **`round_ties_even` and `floor`** are modelled correctly: the concrete ties and the
  definition through `floor` over the `i32` range hold (research R0, 6.5 s).
- **`asin` and `tan`** are C library calls that Kani replaces with a failing check (research
  R0): the 7 chord-angle entries through `to_angle` and `Angle::tan` have no proof; the C++
  vectors check them bit for bit (`core/tests/cpp_values_diff.rs`).
- **`%` on `f64` is wrong** (research R0, Finding 1): Kani computes `1.0 % 2.0` as 0 and
  `−0.0 % 2π` as `+0`. No harness or description uses it. `normalized` computes with it, so its
  value has no Kani proof; its crash proof rests on the argument that every operation after the
  `%` is a comparison, an addition or a subtraction of a finite value and 2π, none of which can
  panic, whatever the `%` returns. Its value is checked by the C++ vectors (no tolerance) and by
  a quickcheck property against an exact big-integer remainder (`core/src/s1/angle_tests.rs`,
  100,000 bit patterns spanning every binade on 2026-10-16). No proven code of 001–006 uses float
  `%` (checked 2026-10-16).
- **Division by an infinity** is reported as NaN, as spec 006 found: the angle's quotients by an
  infinite divisor are a tool limit, checked by the C++ vectors (`ANGLE` rows with divisor +∞).

### Proof by enumeration

`tests/angle_enumeration.rs` proves P-AGEF-02 (the E5, E6 and E7 round trips) and P-AGCNV-02's
integer identities by checking every `i32` (status `proven-by-enumeration`). It trusts what any
test trusts: the compiler's release build of the library and this machine's IEEE-754
arithmetic. The functions it runs use no fused multiply-add and no libm call (one product, a
division-free scale, `round_ties_even`), so the result does not depend on the target's FMA or
math library. Run: `cargo test --release --test angle_enumeration -- --ignored`, 19.5 s for
its 7 tests with five threads on the workstation (2026-10-18).

### The reference descriptions

- **`chord_spec`** states the canonical set, its order and its neighbours from bit patterns,
  never with `f64::next_up` or the library's stepping. Its restatements of upstream's
  arithmetic (`upstream_from_length2` with option B, `upstream_plus_error` with C++'s
  `std::min`/`std::max` on NaN, `upstream_sin2`, `upstream_cos`, the two error bounds) are
  checked against every `CHORD_ANGLE` row (`tests/values_spec_vs_cpp.rs`).
- **`angle_spec`** restates upstream's products factor for factor and its E-form rounding from
  `floor` (ties to even, saturated as the library documents), checked against every `ANGLE` row.
- **003's `wire_spec::lib_cap`** now models option B: a negative `length2` decodes as −1
  (spec 007 FR-017); `ref_cap` is unchanged, and P-FX-08 names the deviation. `CapC`'s
  reference decoder states D-004 (a NaN `length2` is rejected) itself: it relied on
  `from_length2(NaN)` being NaN, and since FR-018 that gives `STRAIGHT`, upstream's full cap.
  The weekly `p_fx_08__b32` and `__b33` found the gap (2026-10-18).

### Bounds and gaps

- **Preconditions** are the documented ones (FR-004): every input for the constructors and
  `plus_error`; non-special operands for the chord angle's arithmetic and trigonometry
  (FR-019); every angle, NaN included, for the angle, except exactly where an operation makes
  a new NaN.
- **Excluded or limited**: both types' `Display`; the 7 chord-angle entries through `asin` and
  `Angle::tan`; `normalized`'s value (Kani's `%`); a NaN angle's exact chord length (`sin`); the
  angle's quotients by an infinity; the members that time out at 60 minutes (`ChordAngle`'s
  `sin2` and `sin`, the angle's two quotients). Each is covered by the C++ vectors or ordinary
  tests.

## `s2::Cap` (spec 008)

Everything the cap proofs rely on, in addition to the sections above, which apply unchanged.
Design: `specs/008-formal-verify-cap/` (data-model §1–§3; research R0, R4–R7). Records:
section 9 of `values/INVENTORY.md`, section 10 of `values/PROPERTIES.md`, and the spec-008 part
of `values/EVALUATION.md`.

**Spec 008 is the first to replace code in a proof.** Every proof of specs 001–007 runs the
library's code, and still does. Spec 008's proofs replace some functions, always through
`src/stubs.rs`, and always named in the harness's documentation; a row's status says which:
`proven` (nothing replaced), `proven-by-composition` (lemma stubs), `proven-under-contract`
(function contracts).

### Reliance on 004, 006 and 007

| Used | For | Taken from |
|------|-----|------------|
| `laws::eq` | P-CPEQ-01 | spec 004 |
| `laws::region::contains_implies_intersects` | P-CPREG-01 | spec 004 |
| `any_box_point` (the box `[−1, 1]³` holds every unit point) | `BoxCap`, `BoxPoint` | spec 006 |
| `any_chord_length` | `NonEmptyCap`, the lemmas | spec 007 |
| P-CAVAL-04 (`from_angle`'s range) | the facts `pure_from_angle` allows (`pure_add`'s zero rules rest on L-CP-08, which includes P-CAARI-01's `a + ZERO`) | spec 007 |
| the `Cap` codec's proofs (P-FX-03) | P-CPCOD-01 states the trait law; the bytes are 003's | spec 003 |

### Function contracts (spec 008 FR-003)

`asin_contract`, `atan2_contract` and `cos_contract` replace `f64::asin`, `f64::atan2` and
`f64::cos` in the harnesses that name them: any value in the function's documented range,
narrowed by the arguments' signs, NaN for NaN.

- **Why:** `asin` and `atan2` are C library calls Kani replaces with a failing check; CBMC
  models `cos`, but let `cos(1.1e-308)` and `cos(−0)` be 0 (research R0), which faked a division
  by zero in `rect_bound`.
- **What it trusts:** that the platform's `libm` returns, for every input, a value its contract
  allows. `tests/libm_contracts.rs` checks every special value (±0, ±1, ±(1 ± ε), ±∞, NaN,
  subnormals, `FRAC_PI_2` and its neighbours, π, `atan2`'s signed-zero and infinite pairs) and
  10⁶ random inputs per function (uniform bit patterns and uniform values in the domain), 0.07 s
  in release mode. It is a test, not a proof; the ranges are the C standard's (C11 F.10.1).
- **What it cannot give:** any property that needs two calls of the function on one input to
  agree, or the exact value (the covering law, P-CPREG-04; `union`'s values).
- **Proofs relying on it:** P-CPBND-04, and the crash proofs of `angle_radius`, `rect_bound`,
  `approx_eq`, `approx_eq_with`, and (weekly) `cell_union_bound` and `union`.

### Lemma stubs (spec 008 FR-003a)

A law that recomputes a float value the operation computed (a distance, `is_unit`, a sum)
times out in CBMC, which does not see that two copies of the same products are one value
(research R0: `add_point`, `add_cap`, `contains ⇒ intersects`, `intersects` symmetric, the
unit-centre domain). A lemma stub replaces the recomputed function by a pure function: one value
per input, from a fixed-size table, any value the named lemmas allow.

| Stub | Replaces | Allows | Rests on |
|------|----------|--------|----------|
| `pure_chord` | `Point::chord_angle` | 0 for `(p, p)`; else any length in [0, 4] | L-CP-01, L-CP-02 |
| `pure_chord_sym` | the same | as `pure_chord`, by unordered pair | L-CP-01 to L-CP-04 |
| `pure_add` | `<ChordAngle as Add>::add` | the left operand for a zero right one, the right one for a zero left one (either zero `+0` or −0); else [0, 4] and at least each operand | L-CP-08, L-CP-05 |
| `pure_add_sym` | the same | the zero rules, then by unordered pair | also L-CP-06 |
| `pure_is_unit` | `Point::is_unit` | true at (±1, 0, 0); else one value per point, signs ignored | L-CP-07, L-CP-04 |
| `pure_from_angle` | `ChordAngle::from_angle` | `NEGATIVE`, `INFINITY`, `STRAIGHT` at negative, +∞, NaN; else [0, 4] | P-CAVAL-04 |
| `pure_approx_eq_angle` | `Point::approx_eq_angle` | one boolean per pair and tolerance | purity only |
| `pure_normalize` | `Vector::normalize` | one finite vector per input (two slots) | purity only (P-CCAP-01: `Cell::cap_bound` normalises the centre, and the law reads the same centre) |

- **Soundness:** for each input the real function returns a value its lemmas allow (proven on
  the real code) and returns it every time (purity), so every behaviour of the real code is one
  of the stubbed code. A stub that lacks a fact only adds behaviours: it can fail a law, never
  pass one. Three R0b laws failed for that reason (`chord(p, p) = 0`, adding zero, the
  constants' unit centre) and pass with the facts stated (R0c).
- **Purity** is a property of the library: Rust never contracts floating-point expressions on
  its own, `r3::Vector` chooses `mul_add` per build (`cfg!(target_feature = "fma")`), and
  `sqrt` is correctly rounded (IEEE-754).
- **L-CP-06a** (`u · v` is `v · u`, bit for bit) does not finish in 12 minutes (two symbolic
  multipliers) and timed out at 60 minutes weekly too (2026-10-25), so it is trusted: IEEE-754
  defines the product as the correctly rounded exact product, which is symmetric. L-CP-06 (the chord-angle sum is commutative for non-zero lengths)
  rests on it and on L-CP-06b (addition, 32 s); the composition argument is in `cp_lem.rs`. A
  direct L-CP-06 failed on the checker's two different square roots of one subnormal argument
  (replayed natively: the sums agree) and timed out with `sqrt` stubbed purely.
- **Tables** are `static Mutex<Table<K>>` with ten slots and element-wise keys. A `thread_local!`
  table crashes Kani 0.68.0's compiler (`kani-compiler/src/intrinsics.rs:243`, an intrinsic
  expected to return `i32`), and a growable `Vec` table runs the cell laws out of memory
  (research R0b).

### The abstract cell (research R7)

The cell laws and the cell tests' crash proofs stub `Cell::vertex`, `Cell::edge_raw` and
`Cell::contains_point` with a table of any four box points and any four box vectors, and, where
named, upstream's edge test (`Cap::intersects_cell_edges`) with any answer. The laws depend only
on the order of upstream's tests (vertices first), so a law proven for any values holds for every
real cell; the crash proofs cover every value a real cell's vertices and edges can take, since
those are box values. The real vertices' computation is spec 006's normalisation.

### The checker's model of floating point

- As for 004–007, plus: **`log2`** is loose (R0: it let `floor(log2(1.9375))` be at least 1);
  after FR-017 no proven code uses it.
- **`%`** is wrong (spec 007): after FR-020 `rect_bound` no longer uses it (the library's
  `rem_euclid` did; R0b's validity failure on the old code was that).

### The reference description and the drift guard

- **`cap_spec`** recomputes the squared chord as upstream's `S1ChordAngle(a, b)` defines it and
  reads a cap through `center()` and `chord_radius()` only. Its restatements (height, area,
  centroid, the two constructors with upstream's `FromLength2`, the complement, `AddCap`'s
  margin, the covering level through an exact `ilogb`) are checked against every `CAP_*` row of
  `core/tests/data/cpp_cap_vectors.csv` (`tests/values_spec_vs_cpp.rs`).
- **`reduce_longitude_copy`** is a copy of the library's private `reduce_longitude`, on which
  P-CPBND-05 is proven; `cap_spec_tests` fails if the bodies differ.

### Bounds and gaps

- **Not proven:** P-CPREG-04 (the covering law: the covering's level depends on `asin`'s value,
  which the contract does not fix); `union`'s values (C++ vectors: the radius within 10⁻¹² of
  C++'s, and K9's containment counts); the deserialiser (tested, as 004–007's).
- **Weekly:** listed in `values/PROPERTIES.md` §10.

## `s2::LatLng` and `s2::Rect` as a `Region` (spec 010)

Everything the latitude–longitude proofs and `Rect`'s law instances rely on, in addition to the
sections above, which apply unchanged. Design: `specs/010-formal-verify-latlng/` (data-model
§1–§3; research R3, R5, R6). Records: section 12 of `values/INVENTORY.md`, section 11 and
`Rect`'s rows in section 5 of `values/PROPERTIES.md`, and the spec-010 part of
`values/EVALUATION.md`.

### Reliance on 004, 005, 007 and 008

| Used | For | Taken from |
|------|-----|------------|
| `laws::eq`, `laws::region` | P-LLEQ-01, P-LREG-01, P-LREG-02 | specs 004, 008 |
| `rect_spec` (validity, membership) and P-LMEM-01, P-LREL-02 | the `Rect` laws' statements and the `Cell::rect_bound` stubs | spec 005 |
| `Angle`'s constructors and conversions (P-AGCNV-01, P-AGEF-01) | P-LLCON-01 compares with them; P-LLGEO-01's composition | spec 007 |
| the contracts of `atan2`, `sin`, `cos`, `sin_cos`, `asin` and the abstract cell (`fixed_vertex`) | the conversions, the distance, P-LREG-02 | spec 008, the 2026-10-26 coverage review |
| gap 2's abstract u-v bound (`any_uv_bound`) | L-LL-01, the `Rect` laws' cells | the 2026-10-27 coverage review |

### Pure contracts and `sqrt`'s contract (research R5)

`pure_sin`, `pure_cos`, `pure_asin`, `pure_atan2`, `pure_sin_cos` and `pure_sqrt` return, for an
input they have seen, the value they returned, and otherwise a value the existing contract (and
`sqrt_contract`) allows. A restatement that calls the function again (P-LLFP-01, P-LLTP-01,
P-LLDST-01, P-LMEM-03) then gets the library's value.

- **What it trusts:** the contracts, as above (`tests/libm_contracts.rs`, now with `sqrt`), and
  that the C library's functions are deterministic, one value per input, as IEEE-754's correctly
  rounded `sqrt` is by definition. The checker's own `sqrt` is not (memory "CBMC sqrt is
  nondeterministic"), which is why the restatements stub it.
- **`sqrt_contract`:** NaN below zero and for NaN, `x` itself at ±0 and +∞, otherwise positive
  and finite, at most 1 for `x ≤ 1` and at least 1 for `x ≥ 1`.

### The contract on `Cell::rect_bound` (research R6)

`cell_rect_bound_contract` replaces `Cell::rect_bound` in P-LREG-02 by one valid rectangle per
cell that contains the `LatLng` of each of the cell's four vertices (as `fixed_vertex` and
`pure_from_point` give them). It is the first contract on a function of the library itself: the
relation goes through `atan2`'s value in both `Cell::rect_bound` and `from_point`, which no
contract fixes.

- **What it trusts:** that upstream's conservative bound contains its cell's vertices, which
  `libm_contracts::cell_rect_bound_satisfies_its_contract` checks with the real functions on
  every cell to level 5 (8,190) and 10⁶ random cells at every level, and the C++ vectors'
  `RECT_REGION` rows check indirectly (0 violations of the vertex law in C++ and here).
- **`any_cell_bound`** (P-LREG-01) rests on a proven lemma instead, L-LL-01 (the bound of every
  valid cell is valid and non-empty, under `atan2_contract`); **`pure_from_point`** on P-LLFP-02
  (a point without NaN has a valid `LatLng`) and purity.

### IEEE `fmod` (research R3)

`LatLng::normalized` and `Angle::normalized` reduce through `s1::angle::remainder_2pi`: `x % 2π`,
then a correction step. The correction step is proven (P-LLNRM-03, on a drift-guarded copy);
that Rust's `%` on `f64` is IEEE-754's exact `fmod` (the sign of `x`, `|r| < |y|`, no rounding)
is trusted, since Kani's model of `%` is wrong (`KANI_ISSUES.md`). P-LLNRM-02, -04 and -05 are
compositions over it. `latlng_tests::prop_normalized_is_the_exact_remainder` and
`angle_tests::prop_shared_remainder_is_exact` check the whole against the exact `num-bigint`
remainder at every binade, and the `LATLNG_NORM` rows against C++.

### Bounds and gaps

- **Not proven:** P-LLDST-04 (the distance's symmetry within a tolerance needs the functions'
  values); P-LREG-03 (the bounding cap: it needs `atan2`'s and `sin_cos`'s values; tested
  natively, by the quickcheck twin and by the `RECT_CAPBOUND` rows); P-LREG-04 (the covering:
  `asin`'s value, as P-CPREG-04); the deserialiser (tested).
- **Outside the checker:** the four `geo-types` conversions (by composition from `Angle`'s
  entries; the verification crate does not enable the feature) and the text output.

## `PointRegion` and `CellUnion`'s floating-point methods (spec 011)

Everything the point-region proofs and `CellUnion`'s floating-point proofs rely on, in addition
to the sections above, which apply unchanged. Design:
`specs/011-formal-verify-point-region-cell-union/` (data-model §1–§3; research R0, R5, R6).
Records: section 13 of `values/INVENTORY.md`, section 12 of `values/PROPERTIES.md`, the nine
floating-point rows of `cell_union/INVENTORY.md` and the spec-011 entries of
`cell_union/PROPERTIES.md`, and the spec-011 part of `values/EVALUATION.md`.

### Reliance on 002, 004, 008 and 010

| Used | For | Taken from |
|------|-----|------------|
| `union_domain` (valid unions of exact length), `union_spec`, P-UMEM-01 to -05 | every `uf_*` harness; P-UPT-01's composition | spec 002 |
| `laws::eq`, `laws::region` (and the two laws added here) | P-PREQ-01, P-PRREG-*, P-UREG-01 | specs 004, 008, 011 |
| P-CPGRO-02 (`add_cap` contains its argument), `pure_chord`, `pure_add`, P-CPBND-02 (the metric's level) | P-UBND-02's composition and L-PR-02; P-UEXP-01 | spec 008 |
| `pure_from_point` (P-LLFP-02), `any_cell_bound` (L-LL-01), `sqrt_contract`, `atan2_contract`, `asin_contract` | P-PRBND-02, P-PRREG-05, P-UBND-01, the crash proofs | specs 008, 010 |
| the abstract u-v bound (`any_uv_bound`) and `Cell`'s float crash proofs (`c_fg_*`) | every harness with a cell; the crash compositions | the 2026-10-26 coverage review |

### Contracts and stubs on library functions (research R5)

- **`pure_leaf`** replaces `CellId::from_point` by one valid leaf per point. It rests on L-PR-01
  (`cl_crash::c_fg_from_point` asserts the leaf is valid, weekly, over its box-point domain) and on
  `libm_contracts::cell_id_from_point_returns_a_valid_leaf` (every special triple and 10⁶ random
  ones: wherever the function returns, the leaf is valid, and it panics only where its
  documentation says, spec 011 K4). Purity: the library's `from_point` is a function of its
  argument; the checker's projection takes a square root, which is not (KI-007).
- **`pure_cell_cap`** (any non-empty cap per cell, its centre in the unit box, its squared radius
  in [0, 4]) and **`any_center`** (any box point for `CellId::to_point`) rest on
  `libm_contracts::cell_cap_bound_satisfies_its_contract` (every cell to level 5 and 10⁶ random
  cells), and on the crash proofs of the functions they replace.
- **`pure_cell_area`**, **`pure_avg_area`** and **`pure_cell_contains`** are uninterpreted: any
  value (finite and non-negative, positive, any answer), one per input. A statement proven with
  them holds for every function of the input, the real one among them. `pure_avg_area` is needed
  because the checker's `powi` is not a function (KI-013); the real `average_area_for_level` is
  pinned to upstream's `ldexp` by enumerating its 31 levels (`tests/area_enumeration.rs`).
- **`sqrt_between`** is `sqrt_contract` with the root between `x` and 1, true of the correctly
  rounded root; `libm_contracts::sqrt` checks the clause. Spec 011's R0 found that
  `sqrt_contract` alone lets the root of `1 + u² + v²` be 10³⁰⁰, which no real root is.

### The drift guards (research R4)

`region_spec_tests` pins the library's text of `CellUnion::cap_bound`, `rect_bound`, both
`cell_union_bound`s, the two area sums and `expand_by_radius`'s level arithmetic, each against a
listing whose comments name upstream's lines. P-UBND-04 and P-UEXP-01 rest on the listings being
upstream's, which a reviewer checks when a guard fails, and on the `CELLUNION_*` C++ vectors.

### Bounds and gaps

- **Size bounds:** `CellUnion` proofs at concrete lengths, at most 3 cells (P-UAREA-01 at 1);
  P-UBND-02 is by composition for every length.
- **Not proven:** P-UEXP-02 (`expand_at_level`'s `all_neighbors`, 001 R12); P-UREG-02, -04, -05,
  -06 (they wait on `Cell`'s own bounding laws and `asin`'s value); the deserialiser (tested).
  P-UREG-03 is not a law of `CellUnion`.
- **The checker's NaN check** reports `∞ · 0` before a debug panic, so the panics of K4 that
  need an infinite coordinate (`Cell::contains_point` off the axis, `CellId::from_point` with two
  infinities) are shown by native `should_panic` tests and the probe's sweep, not by `n_*`
  harnesses; their release behaviour is (`r_from_point_infinities`, `r_intersects_cell`).

## The documented panics (2026-10-26)

The `n_*`, `r_*` and `o_*` harnesses of the crash modules (inventories: 001 section 3,
`cell_union` section 3, decoding section 2, values section 10) trust, beyond the above:

- **`should_panic` and the "returns" cover.** Kani passes a `#[kani::should_panic]` harness when
  one input panics and every failed check is a panic (`KANI_ISSUES.md`, limits). That every
  input panics rests on CBMC's reachability of the one cover after the call: unsatisfied means
  no path reaches it, so no input returns. Unwinding checks stay on, so a loop cut short by the
  bound fails the harness rather than hiding a path.
- **The release build** is the dev profile with `debug-assertions = false`
  (`CARGO_PROFILE_DEV_DEBUG_ASSERTIONS`, set by `run-verify.sh` for `VERIFY_DEBUG_ASSERTIONS=0`).
  Kani keeps rustc's overflow checks in it (KI-012), so an `r_*` proof says "no panic and no
  arithmetic overflow", more than a release build does; the inputs where a release build wraps
  are outside it, named in the inventories. The pass drops CBMC's NaN check (Kani's
  `--no-overflow-checks`), since Rust computes NaN without a panic; rustc's division-by-zero
  checks stay.
- **The complements' domains** are written from the models: invalid cell ids through
  `spec::valid_by_marker`, `spec::valid` without its level search (`tests/spec_vs_cpp.rs` checks
  that they agree on every lowest-set-bit position and face, and on the C++ vectors' ids); points
  outside [−π, π] through `arc_spec::in_range`; the rectangles through `uv_spec` and `rect_spec`'s
  emptiness. `ur_crash::n_from_center_size` states its domain with the formula of the `# Panics`
  (`c − s/2 > c + s/2`), the library's own arithmetic.
- **A lemma stub** in `cp_crash::n_from_center_angle` and `r_from_center_angle` (`pure_is_unit`,
  as `c_from_center_angle`): it allows more behaviours than the real `is_unit`, so every one of
  them panicking covers the real one.
- **Bounds.** The union complements hold for unions of one or two ids, as the crash proofs hold
  for bounded unions; `get_uint_with_length`'s for inputs of up to 9 bytes. Inputs a domain
  excludes so that Kani's NaN check does not fire first (infinite latitudes in
  `s2::Rect::from_center_size`, infinite centres and sizes in `r2::Rect::from_center_size`) are
  outside the debug proofs.
- **Not proven, documented:** where not every input outside a domain panics, the `# Panics` now
  says "may panic" (`from_face_ij`, the union operations on invalid unions, `denormalize` on
  invalid ids with a marker), and the float-geometry rows' sections (`CellUnion`'s areas and
  bounds) rest on a native check, not a proof.

## Floating-point geometry (gap 2, 2026-10-26)

The `c_fg_*` harnesses of `cl_crash`, `rl_crash`, `pt_crash`, `ca_crash` and `ag_crash` prove the
crash freedom of the value types' floating-point entries that specs 004–007 excluded (values
inventory section 11). Beyond spec 008's base they trust:

- **More function contracts** (`src/stubs.rs`), each checked against the real `libm` by
  `tests/libm_contracts.rs` (every special value and 10⁶ random inputs): `tan` (finite, the sign
  on [−π/2, π/2], at most 1.7·10¹⁶ where |x| ≤ 2, checked on the 128 doubles nearest ±π/2),
  `atan` (in [−π/2, π/2] with the argument's sign), `sin` (in [−1, 1], non-negative on (0, π]
  and non-positive on [−π, 0)) and `sin_cos` (both in [−1, 1], the larger at least 0.7). The last
  two replace CBMC's own models where those failed a proof that should pass (`KANI_ISSUES.md`,
  limits): a crash proof that passes under the loose models stands without them.
- **The abstract cell and the abstract pair.** Where an entry reads its cell's geometry only
  through `Cell::vertex` (`approx_area`, `exact_area`, `cap_bound`'s vertices, the distances to
  an edge and to a cell), the harness stubs `vertex` with any four box points (`fixed_vertex`,
  spec 008 research R7) or, for two cells, with four for each cell id (`pair_vertex`; one table
  for both would tie the cells together), and `contains_point` with any answer
  (`any_contains_point`). Every real cell's vertices are unit points, so among the abstract
  ones, and a crash proof over the abstraction covers them.
- **The abstract u-v bound.** The `Cell` harnesses stub `s2::ij_level_to_bound_uv`, which
  `Cell::from_cell_id` stores as the cell's u-v bound, with any rectangle inside [−1, 1]² with
  ordered bounds (`any_uv_bound`): the quadratic projection maps [0, 1] onto [−1, 1], increasing,
  so every real cell's bound is such a rectangle. The cell's face, level, orientation and id
  stay its own. The projection of a symbolic (i, j) was most of the cost (`KANI_ISSUES.md`,
  limits), and the per-change proofs of 004 (`cl_crash::c_from_cell_id` and the others) still
  run it on the real code.
- **The domains.** Points are `BoxPoint` (`Cell::from_point` without the origin, whose debug
  panic `n_from_point` proves); rectangles are `ValidRect`; the distance entries with debug
  preconditions (`Rect::get_distance_to_latlng`, `get_distance`) range over them, and their
  complements are proven to panic (`n_get_distance_to_latlng`, `n_get_distance`, which stub the
  code after the checks with any value, `any_segment_distance`: no input reaches it).
- **The NaN check.** The debug pass keeps Kani's NaN check, so a harness whose path computes a
  NaN fails, as everywhere in the suite; the domains above avoid the inputs that would (the
  zero vector, infinite coordinates).
- **Tiers.** Twelve harnesses (24 entries) prove per change in at most 90 s, ten (19 entries)
  weekly; those that time out at 60 minutes are listed in `known-timeouts.txt`, those that run out
  of memory at 8 GiB are in the xlarge tier, and four entries fail the NaN check (the harnesses are
  removed). All of these rest on the tests as before (`COVERAGE.md`, "Tool limits").
