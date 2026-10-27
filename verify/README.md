# `verify/`: formal verification of `CellId`, `CellUnion`, the decoding layer, `Cell` and `s1::Interval`

Kani proofs about two S2 types and the binary decoding layer, and Verus proofs of the
decoding layer's variable-length codecs:
- **`s2rst::s2::CellId`**, the 64-bit cell identifier at the base of S2. This was the
  pilot: spec, plan and decisions are in `specs/001-formal-verify-base-type/`, and the
  outcome is in [`EVALUATION.md`](EVALUATION.md).
- **`s2rst::s2::CellUnion`**, a sorted collection of cells that represents a region.
  Because a union has no fixed length, its proofs hold within stated union-size and
  byte-length bounds. Spec: `specs/002-formal-verify-cell-union/`. Records:
  [`cell_union/INVENTORY.md`](cell_union/INVENTORY.md),
  [`cell_union/PROPERTIES.md`](cell_union/PROPERTIES.md) and
  [`cell_union/EVALUATION.md`](cell_union/EVALUATION.md).

- **The decoding layer**: the integer, cell-id, string and point vectors and the five
  fixed-size codecs (`CellId`, `Point`, `Cap`, `Rect`, `Cell`). Every decoder's contract
  (round trip, exact consumption, sound acceptance, rejection) is stated against a
  reference description of the wire formats, `src/wire_spec.rs`, written from upstream and
  checked against the C++ vectors. The five variable-length codecs are also proved for
  every length with Verus, on restated copies linked to the library (`deductive/`). Spec:
  `specs/003-formal-verify-decoders/`. Records: [`decoding/INVENTORY.md`](decoding/INVENTORY.md),
  [`decoding/PROPERTIES.md`](decoding/PROPERTIES.md),
  [`decoding/EVALUATION.md`](decoding/EVALUATION.md) and
  [`decoding/MUTATION.md`](decoding/MUTATION.md).
- **The value types `s2rst::s2::Cell` and `s2rst::s1::Interval`**, the circular interval:
  fixed-size values, so their proofs cover whole domains with no size bound, including every
  pair of `f64` bounds of the interval. Properties are stated against `src/cell_spec.rs` (a
  cell is its identifier's decomposition, with upstream's u-v projection) and `src/arc_spec.rs`
  (an interval is the set of points it denotes). It includes the proof that the seam fix
  (`length` of the one-ulp interval across ±π) changes exactly one interval's length (P-IMEA-01).
  The trait laws are stated once in `src/laws.rs` and instantiated per type. Spec:
  `specs/004-formal-verify-cell-interval/`. Records: [`values/INVENTORY.md`](values/INVENTORY.md),
  [`values/PROPERTIES.md`](values/PROPERTIES.md) and [`values/EVALUATION.md`](values/EVALUATION.md).
- **The real interval `s2rst::r1::Interval` and the rectangle `s2rst::s2::Rect`**, in the same
  suite and records: properties are stated against `src/real_spec.rs` (an interval is the
  closed set of numbers between its bounds) and `src/rect_spec.rs` (a rectangle is the product
  of that set of latitudes and `arc_spec`'s arc of longitudes), so the rectangle reuses spec
  004's description instead of restating its intervals. The real interval's arithmetic is
  proven wherever `∞ − ∞` does not arise; its infinite corners are documented. Spec:
  `specs/005-formal-verify-r1-interval-rect/`.
- **The u-v rectangle `s2rst::r2::Rect` and the point `s2rst::s2::Point`**, in the same suite
  and records: the u-v rectangle against `src/uv_spec.rs` (the product of two `real_spec` sets,
  so its statements are products of spec 005's), and the point against `src/point_spec.rs` (C++'s
  lexicographic order, the classifications, the power-of-two scaling, and restatements of
  upstream's `IsUnitLength`, `Normalize`, `Ortho` and `EnsureNormalizable` checked against the
  C++ vectors). The point's geometry is proven over the box `[−1, 1]³`. Three of the
  checker's limits shape it: `atan2` is unsupported (the distance and the approximate
  equalities), a division by an infinity is reported as NaN (`from_coords` and `normalize`
  range over the triples with a finite squared length), and `sqrt` is modelled without fixing
  its result, so no two normalisations compare bit for bit (the orthogonal directions' laws).
  Spec: `specs/006-formal-verify-r2-rect-point/`.
- **The chord angle `s2rst::s1::ChordAngle` and the angle `s2rst::s1::Angle`**, in the same
  suite and records: the chord angle against `src/chord_spec.rs` (its canonical values −1, the
  lengths in [0, 4] and +∞, their order and neighbours; since option B every chord angle is one
  of them, so its laws hold for every value) and the angle against `src/angle_spec.rs`
  (upstream's conversions and E-forms, ties to even). Two more limits: `asin` and `Angle::tan`
  are unsupported, and Kani's `%` on `f64` is wrong, so `normalized`'s value rests on the C++
  vectors and an exact property test. The E-form round trips are proven by enumerating every
  `i32` (`tests/angle_enumeration.rs`). Spec: `specs/007-formal-verify-chord-angle-angle/`.

The `CellUnion` harnesses (`src/cu_*.rs`) sit behind the crate's default `cellunion`
feature, the decoding harnesses (`src/cd_*.rs`) behind `decoding`, and the value-type
harnesses (`src/cl_*.rs` for `Cell`, `src/iv_*.rs` for the circular interval, `src/ri_*.rs`
for the real interval, `src/rl_*.rs` for the rectangle, `src/ur_*.rs` for the u-v rectangle,
`src/pt_*.rs` for the point, `src/ca_*.rs` for the chord angle, `src/ag_*.rs` for the angle)
behind `values`, so each suite can run alone
(`make verify-values [P=cell|interval|real|rect|uvrect|point|chord|angle]`). Spec 007 adds a
second method for properties over one integer: `tests/angle_enumeration.rs` checks every `i32`
natively in release mode (about 30 s), where Kani times out (status `proven-by-enumeration`). Three tiers: per change (4 GB and 5–12 minutes per harness), weekly (`slow`, 8 GB, 30
minutes) and xlarge (`xlarge`, for a 32 GB machine).

## What is proven

**Start with [`PROPERTIES_BY_KIND.md`](PROPERTIES_BY_KIND.md)**: every property of the four
suites in plain language, grouped by the kind of guarantee (never crashes, correct result,
round trip, exact output, exact input, validity, laws, model checks), with the harness that
proves it and the inputs it holds for. The records below give the exact statements.

- **Crash-freedom** ([`INVENTORY.md`](INVENTORY.md)): every in-scope public `CellId`
  operation completes without a panic, an arithmetic overflow or undefined behaviour, for
  every input in its documented domain. The inventory lists all 64 public operations: their
  scope, domain, preconditions, harness and status, and any panic the proofs exposed.
- **Functional properties** ([`PROPERTIES.md`](PROPERTIES.md)): a 49-entry catalogue
  (encoding round trips, hierarchy, ranges, traversal, common ancestors, text formats,
  neighbours), each stated against the independent reference model `src/spec.rs` and
  traced to upstream S2.
- **Non-vacuity** ([`MUTANTS.md`](MUTANTS.md)): every proven property has a mutant in
  `mutants/`, a one-defect patch to `core/` that the property must catch. Crash proofs
  carry `kani::cover!` checks instead.
- **What the proofs trust** ([`TRUSTED_BASE.md`](TRUSTED_BASE.md)): the tools, the models,
  the bounds and everything not proven, for both types.

The `CellUnion` records follow the same pattern: crash-freedom of the 27 in-scope operations
and a 27-entry property catalogue stated against `src/union_spec.rs`, the model of the leaf
cells a union covers. The mutants of every suite share `mutants/` and `MUTANTS.md` (ids
`M-0NN` for `CellId`, `M-1NN` for `CellUnion`, `M-2NN` for the decoding layer, `M-3NN` for the
value types).

## Running

Kani 0.68.0 is required (`make verify-kani-check` checks for it):

```sh
cargo install --locked kani-verifier --version 0.68.0
cargo kani setup
```

From the repository root:

| Command | What it runs |
|---------|--------------|
| `make verify` | The models' sanity tests (`cargo test`, against the C++ vectors) and the deductive crate's link tests, then every harness of the three suites except the weekly slow tier |
| `make verify-cellid` | The same for the `CellId` suite alone (`--no-default-features`). CI runs it as one leg of the per-change job |
| `make verify-union` | The same for the `CellUnion` suite alone (`--harness cu_`). CI runs it as another leg |
| `make verify-decoding [P=vectors\|links\|codecs]` | The same for the decoding suite alone (`--harness cd_`), with the deductive crate's link tests, whole or one part: `vectors` (`cd_fw`, `cd_uv`), `links` (`cd_agree`, `cd_sv`), `codecs` (the rest). CI runs the three parts as parallel legs, so each stays under 10 minutes |
| `make verify-values [P=cell\|interval\|real\|rect\|uvrect\|point\|chord\|angle]` | The value-type suite alone (specs 004–007): the models' sanity tests (including the exhaustive `uv_projection` check, about 40 s), then the `cl_*` (`Cell`), `iv_*` (`s1::Interval`), `ri_*` (`r1::Interval`), `rl_*` (`s2::Rect`), `ur_*` (`r2::Rect`), `pt_*` (`s2::Point`), `ca_*` (`s1::ChordAngle`) and `ag_*` (`s1::Angle`) harnesses, whole or one part; without `P` and with `P=angle`, also the E-form enumeration (`tests/angle_enumeration.rs`, release mode). CI runs the eight parts as legs, `values-cell` … `values-angle` |
| `make verify-deductive [MUTANTS=1]` | The Verus track (spec 003): Verus on the restated codecs, then their link tests; with `MUTANTS=1`, the `V-NNN` mutants instead. Needs Verus 0.2026.09.20.aef82ed (`VERUS=<path>`) and Rust 1.98.1. See [`deductive/README.md`](deductive/README.md) |
| `make verify-deductive-bless` | Rewrites the drift guard's fingerprints after a reviewed change to a mirrored library function |
| `make verify-harness H=crash::c_parent` | One harness, by its full name, slow-tier ones included (set `VERIFY_HARNESS_TIMEOUT=30m` for those) |
| `make verify-slow [F=<0-5\|vertex\|union\|decoding\|values>] [P=…]` | The weekly slow tier: the proofs over the per-change budget (`--features slow`), whole or one part. It runs one harness at a time with 60 minutes each, so it fits a 4-core workstation (the whole tier took 12.9 hours on 2026-10-09); CI sets `VERIFY_JOBS` and the 30-minute budget itself. `F=union` runs the whole `CellUnion` suite with its slow-tier members (the larger property members and the 25-byte decode crash proof), at an 8 GB ceiling. `F=decoding` runs the whole decoding suite with its slow-tier members (the cell-id vector encoder at one and two ids) at an 8 GB ceiling, or with `P=<vectors\|links\|codecs>` one part of it, as `make verify-decoding` does (CI runs the three as separate legs, one harness at a time). `F=values` runs the value-type suite with its slow-tier members (the deeper levels of the `Cell` bound families, the intervals' and the rectangle's expansion containment, the u-v rectangle's centre and exact constructors, the point's unit length, orthogonal directions and `rotate`), at an 8 GB ceiling, or `P=cell\|interval\|real\|rect\|uvrect\|point` one part |
| `make verify-xlarge [H=cd_xlarge::<fn>]` | The xlarge tier (spec 003): the decoding members over the weekly budget, the tool limits of T029 (`--features xlarge`, module `cd_xlarge`). For a separate machine with 32 GB of memory: one harness at a time, a 26G ceiling (`XLARGE_MEM`), 4 hours each. It refuses to start unless the ceiling plus 2 GiB is available. No CI job runs it |
| `make verify-mutants [M="M-001 M-016"]` | The mutant campaign, or the listed mutants; `M='M-5*'` selects by pattern (`M-0NN` are spec 001's, … `M-5NN` spec 006's; CI runs one leg per spec). Each runs in a throwaway git worktree, so your tree is never modified. Outcomes: `killed`, `survived`, `no-verdict` (an expected harness times out unmutated too, so it is listed in `known-timeouts.txt`) and `error`. Set `VERIFY_SLOW_HARNESS_TIMEOUT=60m` for the value types' weekly-tier mutants |

Each run prints one line per harness: name, status (`SUCCESSFUL`, `FAILED`, `OOM`,
`TIMEOUT`, `ERROR`, `KNOWN-DEFECT D-NNN` or `KNOWN-TIMEOUT`), covers satisfied and time. While the run goes
on, it prints a timestamped line as each harness finishes, with the counts done and failing
so far; after it, the sorted report, where an `ERROR` re-run alone may turn out `OOM`. A
`FAILED` harness is followed by a concrete-playback unit test with the inputs of a
counterexample. Logs are in `verify/target/run-verify/` (`progress.log`, the progress lines,
for `tail -f` from another terminal; `results.tsv`; `kani.log`).

### The memory guard

A single CBMC run can take tens of gigabytes and take a workstation down with it.
`run-verify.sh` therefore caps every harness, and so do the Makefile targets that call it:

| Variable | Default | Meaning |
|----------|---------|---------|
| `VERIFY_MEM_PER_HARNESS` | `4G` | Address-space ceiling per CBMC process (`ulimit -v`). Over it, the harness is `OOM` |
| `VERIFY_JOBS` | available memory ÷ (7/8 of the ceiling), at most `nproc` (`1` for `make verify-slow`) | Harnesses checked in parallel |
| `VERIFY_HARNESS_TIMEOUT` | `12m` (`60m` for `make verify-slow`, `4h` for `make verify-xlarge`) | Time limit per harness. Over it, the harness is `TIMEOUT`. Use `8m` on a workstation as the CI proxy |
| `VERIFY_PLAYBACK_MEM` | 3 × the ceiling, at most available memory − 2 GiB | Ceiling for the concrete-playback re-run of a failed harness, which runs alone after the main run |
| `VERIFY_RUN_DIR` | `target/run-verify` | Log directory. Give concurrent runs separate ones, and separate `CARGO_TARGET_DIR`s |
| `VERIFY_BATCH` | 32 per 4 GiB of the ceiling | Harnesses per Kani driver. The driver keeps about 20 MB for every harness it has run, under the same ceiling ([KI-010](../KANI_ISSUES.md#ki-010-the-driver-keeps-memory-for-every-harness-it-has-run)), so the run goes in batches. A harness that no driver reached is `NOT-RUN` |

On Linux, a `systemd-run --user --scope` cgroup also caps the run as a whole below the
available memory. **Never run `cargo kani` on these harnesses without a cap.** Use the
Makefile targets or `run-verify.sh`.

## Adding a property

1. **Catalogue it.** Add a row to `PROPERTIES.md`: id, statement against `spec.rs`,
   upstream source, domain, harness and status `draft` (fields in data-model §4). Extend
   `spec.rs` if the model lacks the concept, and check the extension against the C++
   vectors in `tests/spec_vs_cpp.rs`. Then list it in `PROPERTIES_BY_KIND.md` under its
   kind, in plain language, and update that kind's count in the summary table.
2. **Write the harness** in the area's module (`src/<area>.rs`) as
   `fn p_<area>_<nn>()`. Its doc comment starts with `P-<ID> (<kind>): ` and the
   plain-language statement, then, after a blank line, the catalogue statement. The kind
   tag is what `grep -rn '(<kind>)' src` finds, and `tests/property_kinds.rs` checks it
   against the page. Draw inputs from `src/domain.rs` (`any_valid()` also returns the
   model's `Decomposed`).
3. **Bound it.** Give loops a `#[kani::unwind(N)]`; unwinding assertions stay on, so a
   bound that is too small fails rather than hiding cases.
4. **Cover it.** Add `kani::cover!` for satisfiability and for each main branch.
5. **Kill a mutant.** Add `mutants/M-NNN-<slug>.patch`: header lines `Mutant`, `Targets`,
   `Expect-Fail`, `Defect` (and `Tier: slow` for slow-tier harnesses, `Memory: 8G` for one
   that needs a larger ceiling), then a one-defect `git diff` of `core/`. Run
   `make verify-mutants M=M-NNN` and expect `killed`.
6. **Check the budget** (research R15): 4 GiB and the 12-minute CI limit per harness (8
   minutes on a workstation), and 45 CPU-minutes for the whole run. Over it, split the
   domain into a family (`__f<K>` per face, `__b<L>` per length, `__q<K>` per quadrant).
   Over it even when split, the harness moves to the slow tier, behind
   `#[cfg(feature = "slow")]`. Over 30 minutes there (CI's budget; 60 for the value types),
   it is a `tool-limit` result.
7. **Check upstream** (FR-018). If the C++ or Go reference shows undefined or wrong
   behaviour where your domain ends, record a `U-NNN` in `EVALUATION.md` and a follow-up
   task in the spec's `tasks.md` before you finish the proof.

A property that fails on a real defect gets a `D-NNN` in `EVALUATION.md` and a separate fix
(FR-010). Until the fix lands, list the harness in `known-defects.txt`, so that runs report
it as `KNOWN-DEFECT` rather than `FAILED`.

A weekly member that times out at 60 minutes is a tool limit: the catalogue records it as
`not-proven (tool limit)`, and `known-timeouts.txt` lists it, so that runs report it as
`KNOWN-TIMEOUT`, which does not fail the run, and CI keeps running it weekly. When a listed
harness starts to prove, the run fails ("stale known-timeout entry"): record the proof and
remove the line.

## Rules

- **Harness names are `module::fn`.** The harness modules sit at the crate root
  (`src/lib.rs`), so `--exact --harness crash::c_parent` matches. Do not nest them.
- **Covers are enforced by `run-verify.sh`.** Kani 0.68.0 exits 0 when a cover is
  unsatisfied; `run-verify.sh` fails the run instead.
- **One harness may discharge several obligations.** A property harness can also be an
  operation's crash proof: the parser families `enc::p_enc_06__b*`, `p_enc_09__b*` and
  `p_enc_10__b*` prove both. `INVENTORY.md` names the harness in the crash column.
- **One `unsafe`.** `src/domain.rs::with_text` is the only `unsafe` code in `verify/`
  (research R6); `core/` stays `forbid(unsafe_code)`.
- **Concrete lengths for collections** (spec 002). A `CellUnion` harness never draws a union
  or byte string of symbolic length: that cost two to three orders of magnitude. Each length
  is its own harness in a family (`__n<k>`, `__b<L>`), and the family covers every length up
  to the bound.
- **Decoder results are discarded, not dropped** (spec 003). End a decoding harness with
  `wire_domain::discard(r)`: dropping some `io::Error`s makes Kani report a misaligned
  reference cast inside std (`TRUSTED_BASE.md`).
- **No capacity from data, no temporaries per field** (spec 003). In harnesses and in the
  reference description, never size a `Vec` from a decoded or symbolic length, and append
  whole fields to one buffer of concrete capacity: the symbolic-size allocation is the
  memory wall.
- **The smallest unwind that passes.** A larger `#[kani::unwind(N)]` is not harmless: CBMC
  unrolls every loop it cannot bound to that depth. In the model and the generators, write
  loops over slices as indexed loops (`for i in 0..len`), so that CBMC sees their exact trip
  count. Covers only show reachability, so use the implementation's loop-free methods in
  them rather than the model's 31-level `decompose`.

## Layout

| Path | Contents |
|------|----------|
| `PROPERTIES_BY_KIND.md` | Every property in plain language, by kind of guarantee: the place to start |
| `PROPERTIES_BY_KIND.html`, `PROPERTIES.html` | Rendered copies of those two pages; after editing a page, re-render it with `pandoc -f markdown -t html <page>.md -o <page>.html` |
| `tests/property_kinds.rs` | Keeps that page, the harness docs' kind tags and the catalogues in step |
| `src/spec.rs` | The reference model |
| `src/domain.rs` | Input generators (`any_valid`, `any_valid_on_face`, `with_text`, …) |
| `src/crash.rs` | Crash-freedom harnesses (`c_*`) |
| `src/{val,hier,range,trav,anc,enc,nbr}.rs` | Property harnesses (`p_*`); `nbr` is slow-tier only |
| `tests/spec_vs_cpp.rs` | The model's sanity tests against the committed C++ vectors |
| `src/union_spec.rs`, `src/union_spec_tests.rs` | The `CellUnion` model (the leaf cells a union covers) and its unit tests |
| `src/union_domain.rs` | `CellUnion` generators (`any_valid_union::<N>()`, `with_bytes::<L>()`, …) and the output readers |
| `src/cu_crash.rs`, `src/cu_{acc,inv,norm,mem,set,range,den,meas,enc}.rs` | `CellUnion` crash and property harnesses, behind the `cellunion` feature |
| `tests/union_spec_vs_cpp.rs` | The union model's sanity tests against `cpp_cellunion_vectors.csv` |
| `cell_union/` | The `CellUnion` inventory, property catalogue and evaluation |
| `src/wire_spec.rs`, `src/wire_spec_tests.rs` | The decoding layer's reference description and its unit tests |
| `src/codec.rs`, `src/wire_domain.rs` | The decoder contract (the `Codec` trait and its generic check bodies) and the decoding generators |
| `src/cd_{fw,uv,cv,sv,pv,fx}.rs` | Decoding property harnesses (`p_*`): fixed-width helpers, integer, cell-id, string and point vectors, fixed-size codecs |
| `src/cd_crash.rs`, `src/cd_agree.rs`, `src/cd_xlarge.rs` | Decoding crash harnesses not subsumed by a property; bounded agreement with the restated codecs; the xlarge tier |
| `tests/wire_spec_vs_cpp.rs`, `tests/reservation.rs` | The reference description against `cpp_coding_vectors.csv`; the FR-019 reservation measurement |
| `deductive/` | The Verus track: restated codecs, their proofs, the drift guard and the tested agreement |
| `decoding/` | The decoding layer's inventory, property catalogue, evaluation and mutation measurement |
| `run-verify.sh`, `run-mutants.sh`, `run-mutation-measure.sh` | The guarded runners behind the Makefile targets |
| `mutants/` | Mutant patches |
| `known-defects.txt` | Harnesses that fail on a recorded, not yet fixed defect |
| `known-timeouts.txt` | Weekly harnesses that time out at 60 minutes, the recorded tool limits |
| `research/rust_formalmethods.md` | Survey of formal-verification approaches for Rust, cited by the evaluation's assurance-level recommendation |
