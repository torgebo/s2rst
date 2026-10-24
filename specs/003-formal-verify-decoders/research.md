# Research: Formal Verification of the Decoding Layer

Phase 0 of [plan.md](plan.md). Each entry gives the decision, the rationale, and the
alternatives considered. The fixed inputs are:

- **The 001 and 002 setups and results.** Kani 0.68.0 / CBMC 6.11.0, the `verify/` crate, the
  memory-guarded runner, the mutant campaign, CI and the weekly tier. 002's evaluation
  recommends continuing with Kani for byte-level code, with small bounds, paired with
  differential and property tests.
- **The spec clarifications of 2026-10-04.** The target is the decoding layer, and a
  deductive track for unbounded results is a tracked deliverable with its own CI job.
- **The other branch** (`priv/claude/formal-methods-type-invariance-ept6u0`, written
  against v0.3.2), as analysed in the spec's Context section.

There are no open NEEDS CLARIFICATION items. Facts were checked against the working tree
at `774b06c`, the local Kani 0.68.0 install, the local Verus release 0.2026.09.20.aef82ed
(`/home/t/Downloads/verus-x86-linux/`), and the upstream checkout
`/home/t/sources/s2geometry` at `v0.14.0-40-g37a4501`. That is the pinned oracle
`v0.14.0-39-g8d5c2a8` plus one commit that only adds a test-vector example. R0 is a
memory-capped probe run on 2026-10-04 and 2026-10-05. R1 corrects the spec where the survey had read an
older upstream.

## R0. Feasibility probe (measured, 2026-10-04 and 2026-10-05)

### Kani

These were throwaway harnesses in a temporary `verify/src/probe.rs`, removed afterwards.
They ran through `run-verify.sh`: three at a time, a 4 GiB ceiling per CBMC process, an
8-minute limit per harness, and the `systemd-run` cgroup backstop. The machine was an
8-thread workstation with 30 GB. Byte strings and element counts were concrete lengths, as
002's R0 requires. There were five runs:

- **Run 1** used the unmodified library.
- **Runs 2–5** used a throwaway git worktree with probe-only changes to `core/`:
  - **Run 2** capped the decoders' up-front reservation at 65,536 elements, the geometry
    decoders' pattern. It also replaced the `format!` calls in the `Rect` and point-vector
    decoders with fixed strings, the R6 change in its simplest form.
  - **Run 3** replaced the `u64` decoder's reservation with `Vec::new()`, to isolate the
    cause.
  - **Run 4** grew every result on demand (`Vec::new()`, and `Read::take` + `read_to_end`
    for string data), which is R7's fix, and kept the fixed messages. The round-trip
    harnesses gave their writer a fixed capacity of 64 bytes.
  - **Run 5** split the point vector by format tag and tried smaller string vectors, with
    run 4's library.

The table also has one Kani harness on the restated Verus code (last row).

| Harness | Domain | Run 1 (as is) | Run 2 (cap 65,536) | Runs 3–5 (grow on demand) |
|---------|--------|---------------|--------------------|---------------------------|
| `encode_uint_with_length` crash, writes `len` bytes | every `u64`, every `len` ≤ 8 | ✓ 1.6 s | | |
| fixed-width round trip via `get_` and `decode_uint_with_length` | every `len` ≤ 8, every value that fits | ✓ 4.1 s | | |
| `CellId` decode, covers ok and err | every 8-byte input | ✓ 2.7 s | | |
| `Cell` decode, covers ok and err | every 8-byte input | ✓ 25.6 s | | |
| `Point` decode then encode returns the input | every 24-byte input | ✓ 3.8 s | | |
| `Cap` decode, covers ok and err | every 32-byte input | ✓ 5.2 s | | |
| `Cap` decode, no covers | every 32-byte input | ✓ 2.4 s | | |
| `Rect` decode, covers ok and err | every 33-byte input | TIMEOUT | ✓ 3.3 s (fixed message) | |
| `Rect` decode then encode | every 33-byte input | TIMEOUT | OOM | |
| `decode_uint_vector_u64` | every 2-byte input | ERROR (Kani driver panic) | OOM | ✓ 4.6 s (run 3), 4.7 s (run 4) |
| `decode_uint_vector_u64` | every 5-byte input | OOM after 19 s | OOM | ✓ 8.5 s (run 3), 9.5 s (run 4) |
| `decode_uint_vector_u64` | every 11-byte input | OOM after 34 s | OOM | ✓ 23.6 s |
| `decode_uint_vector_u32` | every 11-byte input | OOM after 35 s | OOM | ✓ 20.6 s |
| `decode_s2cell_id_vector` | every 11-byte input | OOM after 55 s | OOM | ✓ 82.3 s |
| `decode_string_vector` | every 4-, 6- and 11-byte input | | OOM (11) | OOM at 4, 6 and 11 bytes |
| `decode_s2point_vector` | every 11-byte input | TIMEOUT | OOM | OOM |
| `decode_s2point_vector`, uncompressed tag only | every 25-byte input | | | OOM |
| `decode_s2point_vector`, compact tag only | every 11-byte input | | | OOM |
| `decode_s2point_vector` | every 25-byte input | TIMEOUT | OOM | OOM |
| `encode_uint_vector_u64` crash | every 2 values | ✓ 9.6 s | ✓ 7.2 s | |
| `u64` vector round trip, nothing left unread | every 1 value | OOM after 58 s | OOM | OOM (fixed-capacity writer) |
| `u64` vector round trip | every 2 values | OOM after 51 s | OOM | OOM |
| cell-id vector round trip | every 1 id | OOM after 94 s | OOM | OOM |
| string vector round trip | 2 strings of up to 2 bytes | OOM | OOM | OOM |
| **Agreement**: library `decode_uint_vector_u64` vs the restated Verus decoder (same acceptance, values, bytes consumed) | every 6-byte input | | | ✓ 27.2 s (5 GiB cgroup cap) |

**What the probe shows**:

1. **Fixed-size codecs are cheap.** Every fixed-size decoder is proven crash-free over every
   input of its encoded length in 2–26 s. That includes `Rect`, once its message is fixed.
   The `Point` decode-then-encode identity costs the same. `Cap` validation multiplies
   floats (`is_unit`), but the probe asserted on no float result, so CBMC kept it cheap.
   That confirms the other branch's rule. The `Rect` decode-then-encode harness ran out of
   memory. The round trip goes in the other direction instead, encode then decode on valid
   rectangles, which took 4.9 s on the other branch.
2. **A reservation sized from decoded data is the verification wall, capped or not.** Every
   vector decoder ran out of memory at 4 GiB, even on a 2-byte input. They all reach
   `Vec::with_capacity(count)` with a symbolic count. Capping the count (run 2) did not
   help, because the allocation size stays symbolic. Growing on demand (runs 3 and 4) made
   the integer vector decoders verify at 11 bytes in about 20 s, and the cell-id vector in
   82 s. The same change fixes K2, so the fix and the proof need the same code (R7). The
   other branch had seen the same wall.
3. **`Rect` and point-vector decoding stall in `core::fmt`.** Both build error messages with
   `format!` from decoded values, which is 002's R0 finding 5. Fixed messages took `Rect`
   from a timeout to 3.3 s (R6).
4. **The string vector and the point vector stay out of reach at 4 GiB.**
   - The string vector runs out of memory even at 4 bytes. It reads its data, of a symbolic
     total length, through `read_to_end`, and builds one `Vec<u8>` per entry, so a harness
     owns several allocation-backed containers of symbolic size. That is the boundary the
     other branch named.
   - The point vector runs out of memory on both branches. The uncompressed branch was
     tried at 25 bytes, the first length that holds a point, and the compact branch at 11
     bytes. The compact branch runs a string-vector decode and integer-vector decodes
     before its float projections.

   R5 sets the plan for both: measure smaller lengths and the 8 GiB weekly tier first,
   then record tool limits with measurements.
5. **Encoding and decoding in one harness does not fit**, even for one element, and even
   with a fixed-capacity writer, while each half verifies alone. The round trip is
   therefore proven by composition (R4).
6. **The restated code is checkable against the library.** The Verus-proven restated `u64`
   decoder and the library's decoder agree on every 6-byte input (last row). The first
   attempt failed an unwinding assertion in `memcmp`, from comparing two `Vec`s with `==`.
   This is the trap in 002's notes. Element-wise comparison over the concrete maximum fixed
   it.
7. **The Kani driver panic** ("data did not match any variant of untagged enum
   `ParserItem`") recurred on a run near its memory limit, as in 002's evaluation.

### Verus

The probe restated two of the library's codecs for Verus 0.2026.09.20: the varint
(`write_uvarint`, `read_uvarint`) and the 64-bit integer vector (`encode_uint_vector_u64`,
`decode_uint_vector_u64`, with `encode_uint_with_length`, `decode_uint_with_length` and
`byte_length_u64`). The restatement keeps the library's control flow, checks and limits
statement by statement. Its reader is a slice and a cursor, and its writer a `Vec<u8>`.
It ran under `ulimit -v 8000000`. The file is kept as
[`probe/uint_vector.rs`](probe/uint_vector.rs), the starting draft for
`verify/deductive/`.

| Part | Proved, for every input and every length | Obligations | Time | Peak memory |
|------|------------------------------------------|-------------|------|-------------|
| varint alone (first draft) | The encoder appends exactly `varint(x)`. The decoder returns exactly what the spec decoder `dec` returns, for every byte sequence and position, so it terminates and never overflows or indexes out of bounds. Round trip with exact consumption for every `u64`, whatever precedes and follows | 19 | 1.4 s | 0.31 GB |
| `probe/uint_vector.rs` (varint + vector) | The encoder appends exactly `varint(header) + elems(v, width)`, and every element fits the width it chooses. The decoder returns exactly the spec decoder's result for every input. Round trip with exact consumption for every vector of up to 50,000,000 elements (the library's limit), whatever precedes and follows | 51 | 3.4 s (6.3 s cold, after a reboot) | 0.33 GB |

The file has 609 lines. About 100 of them are executable code mirroring the library; the
rest is specification and proof. Nineteen goals are discharged by Z3's bit-vector theory
(`by (bit_vector)`). The work took about two hours, including the decoder's loop invariant
and the induction over the element sequence. The round trip holds only up to the library's
count limit, because the decoder rejects longer vectors, though the encoder writes them (R1).

### Link from restated code to the library

The same file was compiled as an ordinary library module in a scratch crate. Its
dependencies were `vstd = "=0.0.0-2026-09-20-0158"`, `verus_builtin =
"=0.0.0-2026-09-16-0054"` and `verus_builtin_macros = "=0.0.0-2026-09-20-0158"` from
crates.io (the versions matching the Verus release; all MIT), plus `s2rst` by path. It
built with the stable toolchain in 30 s, because `verus!` erases ghost code outside Verus.
Kani's nightly built it too, for the agreement harness in the table (✓ on every 6-byte
input, 27 s). So all three links of FR-011 have a working mechanism.

## R1. Upstream re-confirmed at the pinned version; spec corrections

**Decision**: The spec's survey read C++ at `v0.11.1-25`. Planning re-read every cited
behaviour at the pinned oracle:

| Survey item | At `v0.14.0-40` | Consequence |
|-------------|-----------------|-------------|
| K1: `EncodeUintWithLength` and `GetUintWithLength` check `length <= sizeof(T)` with `ABSL_DCHECK` only (`encoded_uint_vector.h:145, 159`) | Unchanged | FR-021 stands |
| K2: `EncodedUintVector<T>::Init` checks `decoder->avail() < bytes` before accepting a count (`encoded_uint_vector.h:229–240`) | Unchanged | FR-019 stands |
| K3: `EncodedStringVector::Init` rejects decreasing offsets | **Changed.** Upstream commit `95b1e15` (2025-12-29, "EncodedStringVector: Make safety check lazy") removed the check. `Init` now accepts any offsets. `Decode()` and `operator[]` return an empty string for an entry whose range is invalid and continue with `start = limit` (`encoded_string_vector.cc:57–84`, `encoded_string_vector.h:148–160`) | **K3 is not a defect.** The library's `decode_string_vector` does exactly this: `["abcde", "", "cdef"]` for offsets `[5, 2, 6]` is the pinned upstream's answer too. FR-020 is rewritten to document and prove the behaviour, with no behaviour change |
| K4: `S2CellId::Decode` validates nothing (`s2cell_id.cc:261–265`). `S2Cap::Decode` checks `is_valid()` only under `FLAGS_s2debug`, with `ABSL_CHECK` (`s2cap.cc:340–353`). `S2LatLngRect::Decode` always rejects an invalid rectangle | Unchanged | FR-022 stands: only the `CellId` and `Cap` decoders deviate |
| `S2Cell::Decode` constructs `S2Cell(id)` from an unvalidated id (`s2cell.cc:326–332`) | Unchanged | R16 |

Planning also found two facts the survey missed:

- **Count limits are deviations.** The library rejects integer vectors of more than
  50,000,000 elements, point vectors of more than 50,000,000 points, and string vectors whose
  data exceeds 500,000,000 bytes. Upstream has none of these limits. It rejects a count
  only when the bytes are not there. A valid upstream encoding above a limit is therefore
  rejected by the library. FR-022 now requires these limits to be documented as deliberate
  deviations. They stay as safety limits.
- **D-003 candidate: the varint's tenth byte.** Upstream `Varint::Parse64WithLimit` accepts
  a tenth byte only if it is 0 or 1 ("Value is too long to be a varint64",
  `util/coding/varint.cc:260–262`). The library's `read_uvarint` accepts a tenth byte from 2
  to 127 and drops the bits beyond 64. Through the in-scope decoders this cannot be
  observed: every 10-byte varint read there is an element count far above the decoders'
  limits, so both sides reject the input. It can be observed through `read_uvarint`'s
  out-of-scope callers (polylines, loops, polygons, shape indexes). The catalogue property
  "varint decoding agrees with upstream" will expose it. It is handled under FR-014: a
  `D-003` entry in the evaluation, a known-defect listing while it is open, and a separate,
  tested fix. The user is asked before that fix lands, as for 002's D-002.

**Spec amendments** (applied to `spec.md` with this plan; checklist iteration 2):

- Context, K3 bullet: replaced by the finding that the library matches the pinned upstream.
- FR-006, string vectors area; FR-013; SC-007: no behaviour fix for string vectors.
- FR-020: now "decreasing offsets decode as empty entries, as upstream; documented and
  proven".
- FR-022: the count limits are documented deviations.
- SC-009: rewritten to match.
- FR-006's last bullet: the parity entry is a test entry, which SC-003 does not count.

**Rationale**: Constitution I makes the pinned C++ the canonical reference. Rejecting
decreasing offsets would create a new deviation from upstream, not remove one. The user
made no decision on K3; it was a factual error in the survey.

**Alternatives considered**: Keep FR-020 as written, as a deliberately stricter decoder
like 002's FR-017. Rejected: it would need its own justification, and upstream moved in the
opposite direction on purpose. Ask the user. Not needed: no requirement the user chose
changes, and the constitution decides the question. The completion report states the
correction.

## R2. Suite, crate and layout

**Decision**: Extend the `verify/` crate with a second default feature, `decoding`, gating
crate-root modules `cd_*`, as 002 did with `cellunion` (contracts/verify-commands.md).
Per-suite records go under `verify/decoding/` (`INVENTORY.md`, `PROPERTIES.md`,
`EVALUATION.md`, `MUTATION.md`). The deductive track is a separate unpublished crate,
`verify/deductive/` (R11). CI gains a third per-change leg (`decoding`) and a `deductive`
job.

**Rationale**: It reuses 001/002's runner, generators, mutants and CI. The restated code
needs `vstd` and the Verus macros. In a separate crate those dependencies stay out of
`verify/`'s Kani build except where agreement harnesses need them (R12).

**Alternatives considered**: harnesses inside `core/` under `cfg(kani)`, as on the other
branch. Rejected: 001/002 keep `core/` free of verification code, and `core/Cargo.toml` would
need a feature for the large harnesses. A new top-level verification crate for decoding.
Rejected: it would duplicate the runner and the generators.

## R3. Reference description of the wire formats (FR-007)

**Decision**: `verify/src/wire_spec.rs` holds one **reference decoder** per format. It
returns `Option<(value, consumed)>` for any byte slice, plus the canonical encoder for the
round-trip direction. It is written from upstream's format documentation and code
comments, not from `core/`:

| Format | Upstream source |
|--------|-----------------|
| varint | `util/coding/varint.h` (`Parse64WithLimit`: little-endian 7-bit groups, a tenth byte of at most 1) |
| fixed-width little-endian | `encoded_uint_vector.h` (`EncodeUintWithLength`, `GetUintWithLength`) |
| integer vectors | `encoded_uint_vector.h` header comment: varint of `(size * sizeof(T)) | (len - 1)`, then `size` elements of `len` bytes; `len` minimal for the OR of the values with 1 |
| cell-identifier vector | `encoded_s2cell_id_vector.h/.cc`: the `shift_code`/`base_len` byte, the extended shift byte, the base's high bytes, then an integer vector of deltas; value = `base + (delta << shift)` |
| string vector | `encoded_string_vector.h/.cc` at v0.14: an integer vector of end offsets, then the data; an entry with an invalid range is empty, and decoding continues from its end |
| point vector, uncompressed | `encoded_s2point_vector.cc` (`EncodeS2PointVectorFast`, the format tag in the low 3 bits of the first varint) |
| `CellId`, `Point`, `Cap`, `Rect`, `Cell` | the respective `Encode`/`Decode` in `s2cell_id.cc`, `s2point` coding, `s2cap.cc`, `s2latlng_rect.cc`, `s2cell.cc` |

The **library's documented deviations** (FR-022) are separate, named predicates applied on
top of the upstream meaning: the count limits; `CellId` and `Cell` validity, through 001's
model; `Cap` validity (R18). A property then states "the decoder accepts exactly what
upstream accepts, minus the documented deviations". Each deviation is visible in the
statement. `tests/wire_spec_vs_cpp.rs` checks every reference decoder and encoder against
the C++ vectors (R10).

**Rationale**: FR-007 needs a second, independent description. A reference decoder lets one
generic statement cover parts 2–4 of the contract (agreement on every input). The varint
and the integer vectors match the restated Verus spec functions in meaning. They are two
renderings of the same upstream text, linked through the restated code (R12).

**Alternatives considered**: describing formats only by their encoders. Rejected: decoders
of untrusted bytes see inputs no encoder produces, such as truncations, over-long varints
and corrupt offsets. A grammar or parser-combinator library. Rejected: it would be an
extra dependency and a second implementation to trust.

## R4. The decoder contract in code (FR-004)

**Decision**: `verify/src/codec.rs` defines a `Codec` trait. Its members are `Value`,
`encode`, `decode` (the library), `reference_decode` (R3), `encodable(&Value)` (the
documented domain of the round trip) and `same(&Value, &Value)` (bit-exact, so NaN payloads
compare equal to themselves). It has one implementation per in-scope codec. Two generic
bodies state the contract once:

- `check_encoder::<C, N>()`: the library's encoder writes exactly the reference
  description's canonical encoding, for every value sequence of up to `N` elements. This is
  the encoder half of part 2.
- `check_agreement::<C, L>()`: the library's decoder agrees with the reference decoder on
  every `L`-byte input: the same acceptance, the same value, the same bytes consumed. These
  are the decoder half of part 2, and parts 3 and 4.
- **Part 1, the round trip, is discharged by composition.** For every encodable `v` whose
  canonical encoding has at most `L` bytes:
  `decode(encode(v)) = decode(enc(v)) = ref_decode(enc(v)) = v`, using the two harnesses
  above and the reference description's own round trip, `ref_decode(enc(v) + rest) = (v,
  |enc(v)|)`. That last lemma concerns only `wire_spec.rs`. It is proven by its own Kani
  harnesses over the model, which are cheap because the model's writer and reader are plain
  slices, and tested against the C++ vectors. A direct harness that runs the library's
  encoder and decoder together is kept only where it fits (the fixed-size codecs).

Part 5 is recorded per decoder as "test evidence" (R7). A new codec gets the contract by
implementing `Codec` and adding three harness families (encoder, agreement, model round
trip).

**Rationale**: It meets FR-004's "state once, check per codec". It gives SC-003 a
countable shape: one round-trip and one agreement property per codec, plus codec-specific
ones. Composition is forced by R0: every harness that ran the library's vector encoder and
decoder together ran out of memory at 4 GiB, even for one element and with a fixed-capacity
writer, while each half alone verified in seconds.

**Alternatives considered**: one macro per codec with copied bodies. Rejected: the
statement would drift between codecs. Direct round-trip harnesses for the vectors.
Rejected: out of memory in R0 (runs 1, 2 and 4). Kani function contracts (`#[kani::requires]` /
`#[kani::ensures]` on `core/` functions). Rejected: they need annotations in `core/`, and
they are unstable in 0.68.0.

## R5. Domains, bounds and unwinding (FR-008, SC-001)

**Decision**: The bounds below. They come from R0 and the user's choice of measured bounds
(spec clarification 2026-10-05). Rows marked *measure* are set during implementation, from
the smallest length up, before the rest of their family is written.

| Inventory entries | Family | Bound | Basis |
|-------------------|--------|-------|-------|
| fixed-width helpers | none (symbolic `len` ≤ 8, symbolic value) | whole domain | R0: 1.6 s, 4.1 s |
| `CellId`, `Cell` decoders | `__b0 … __b9` | whole domain (every input of up to 8 bytes, plus one longer) | R0: 2.7 s, 25.6 s at 8 bytes |
| `Point` decoder | `__b0 … __b25` | whole domain | R0: 3.8 s at 24 bytes |
| `Cap` decoder | `__b0 … __b33` | whole domain | R0: 5.2 s at 32 bytes |
| `Rect` decoder | `__b0 … __b34` | whole domain, after R6 | R0 run 2: 3.3 s at 33 bytes with a fixed message |
| fixed-size encoders | none | whole domain (every value) | cheap, like the round trips |
| integer vector decoders (u32, u64) | `__b0 … __b11` | every input of up to 11 bytes, after FR-019 | R0 run 4: 20.6 s / 23.6 s at 11 bytes |
| cell-id vector decoder | `__b0 … __b11` (the 10- and 11-byte members may go weekly) | as above | R0 run 4: 82.3 s at 11 bytes |
| string vector decoder | `__b0 … __b<K>` | *measure*: the largest `K` that fits 4 GiB per change, or 8 GiB weekly; tool limit if no `K ≥ 3` fits | R0 runs 4–5: OOM at 4, 6 and 11 bytes (4 GiB) |
| point vector decoder | per format tag: `__t0_b<K>` (uncompressed), `__t1_b<K>` (compact), `__tx_b<K>` (tags 2–7, rejected) | *measure*, as for the string vector. The compact branch is the likely tool limit | R0 runs 4–5: OOM at 25 bytes (uncompressed) and 11 bytes (compact) |
| vector encoders | `__n0 … __n2` (crash and encoder agreement, R4), more where they fit | every sequence of up to 2 values | R0: 2 values 7–10 s (crash). The round trip is by composition (R4); direct round-trip harnesses ran out of memory |

Unwinding is the byte length plus one for decoders, and 11 for encoders (the 10-iteration
varint loop plus one), as in R0. A decoder family member whose length cannot reach the
next element (for example 3 bytes for a `u64` element of width 8) still runs, because
truncation is a behaviour to prove.

**Rationale**: The fixed-size decoders get whole-domain proofs: every input of up to the
encoded length, plus one longer member, together with the exact-consumption property.
For the integer and cell-id vectors, 11 bytes covers every varint header (1–10 bytes) and
at least one element. The string vector and the point vector own several symbolic-size
containers (R0 finding 4), so their bound is found by measurement. The deductive track
covers every length for the varint, both integer vectors, the cell-id vector and the string
vector (R11). For the point vector beyond its bound, the R13 property tests and the fuzz
target are the evidence.

**Alternatives considered**: symbolic lengths. Rejected: two to three orders of magnitude
slower in 002's R0. Bounds chosen without measuring. Rejected: 002 had to amend its spec
after planning for exactly that reason.

## R6. Deferred formatting of decoder messages (FR-013)

**Decision**: The `Rect` decoder and the point-vector decoder build `io::Error` messages
with `format!` from decoded values ("unsupported encoding version {version}", "unknown
S2PointVector format {format}", "invalid level {level}"). They switch to 002's pattern: a
private error enum that implements `Display` with the same text, wrapped with
`io::Error::new(kind, err)`. The text is formatted only when the error is displayed. A
unit test per message compares the displayed text with today's.

**Rationale**: R0 run 1 timed out on both decoders even at 11 bytes. With a fixed message
(run 2), `Rect` verified in 3.3 s. The point vector then ran into the R0 finding 4 memory
wall instead of the formatting one. This is the same wall 002 met, and 002's change kept
every displayed message.

**Alternatives considered**: excluding both decoders as tool limits. Rejected: the change
is small, keeps behaviour, and 002 established it. Fixed strings without the values.
Rejected: they change the displayed message, which FR-013 forbids.

## R7. FR-019: capped reservation, and its evidence

**Decision**:

- **Fix: grow on demand.** No in-scope decoder reserves memory from a size it has decoded:
  - `decode_uint_vector_u32` and `decode_uint_vector_u64` start from `Vec::new()` and push;
  - the point vector's uncompressed and cell-id branches do the same;
  - `decode_s2cell_id_vector` and `decode_string_vector`'s result vector, which size
    themselves from an already decoded vector, do the same;
  - `decode_string_vector` reads its data through `Read::take(total_len)` into a growing
    `Vec`, instead of `vec![0u8; total_len]` followed by `read_exact`. If fewer than
    `total_len` bytes arrive, it returns `UnexpectedEof`, as `read_exact` does today.

  Results for valid inputs are unchanged. The geometry decoders' `MAX_PREALLOC` cap is not
  copied (see Alternatives).
- **Regression test** (Constitution III), in `core/tests/decoder_robustness.rs`, Linux only.
  A parent test runs the same test binary as a child, `sh -c 'ulimit -v 262144; exec
  "$0" …' --exact <child> --ignored`. The child is an `#[ignore]`d test that decodes the
  K2 inputs (one per decoder, at most 16 bytes, each declaring the largest accepted count)
  and asserts an error. Without the fix the child aborts (SIGABRT), so the parent fails.
  With the fix it exits 0. R0 checked that a Rust binary runs normally at 256 MiB, and
  that the K2 input still aborts there.
- **Reservation measurement** (SC-008's 2 MiB), in `verify/tests/reservation.rs`, which
  `make verify*` runs. A `stats_alloc` instrumented global allocator (a dev-dependency of
  `verify/` only, MIT, with no `unsafe` in our code) measures the bytes allocated while
  each K2 input is decoded, and asserts at most 2 MiB.
- **Valid large inputs**: a test decodes encodings of 1,000,000 elements for each vector
  format and compares the values (SC-008).
- **Differential**: the C++ vectors include each K2 input, which upstream rejects (R10).

**Rationale**: R0 settled the form of the fix. A capped reservation
(`with_capacity(count.min(65_536))`, run 2) still left every vector decoder out of memory at
4 GiB, because the allocation size stays symbolic to CBMC. Growing on demand (runs 3 and 4)
made the integer and cell-id vector decoders verify at 11 bytes in 21–82 s. The same change
removes K2. The cost is a few extra reallocations per decoded vector, which the
iai-callgrind bench records (Constitution V). The checker cannot observe capacity, so part
5 of the contract needs test evidence. An address-space limit is exactly the condition
under which the defect aborts.

**Alternatives considered**: the geometry decoders' `MAX_PREALLOC` cap. Rejected: it fixes
K2, but R0 run 2 shows the decoders stay beyond the checker's reach with it. Checking
available bytes first, as upstream does. Not possible: a `dyn Read` cannot report what is
left. A counting allocator in `core/tests`. Rejected:
`core`'s lints forbid `unsafe` in every target, including tests. Lowering the count
limits instead. Rejected: it rejects more valid input and still lets a 5-byte input reserve
megabytes.

## R8. Documentation (FR-020, FR-021, FR-022)

**Decision**: one documentation-only commit:

- **FR-021**: `# Panics` sections for `encode_uint_with_length` (length above 8),
  `get_uint_with_length` (length above 8, or a slice shorter than the length) and
  `decode_uint_with_length` (length above 8).
- **FR-022**: rustdoc on `impl S2Decode for CellId` and `impl S2Decode for Cap`, stating the
  rejection as a deliberate deviation from upstream (`s2cell_id.cc:261`, `s2cap.cc:340`),
  with the reason. Every in-scope decoder's `# Errors` names each rejection condition:
  truncation, varint overflow, the count limits (as deviations, R1), invalid shift, unknown
  format, invalid level, unsupported version, invalid value. It also names what the
  decoder does not validate: the cell-id vector's identifiers, and the point vector's unit
  length.
- **FR-020** (amended, R1): `decode_string_vector` documents that an entry whose offsets are
  out of order or out of range decodes as an empty string, and that decoding continues from
  its end, as upstream does since `95b1e15`.

**Rationale**: Constitution I and II require the deviations, panics and errors to be in the
rustdoc. One commit keeps them reviewable, as in 002.

**Alternatives considered**: documenting the limits in `CONTRIBUTING.md` only. Rejected:
users read the rustdoc.

## R9. Fuzz targets (FR-023)

**Decision**: three new `cargo-fuzz` targets, `fuzz/fuzz_targets/decode_cell_id.rs`,
`decode_point.rs` and `decode_rect.rs`. Each decodes the input. On success it encodes the
value, decodes that again, and asserts equality (bit-exact for `Point`). They join the
nightly matrix in `.github/workflows/fuzz.yml` and `fuzz/Cargo.toml`. The existing targets
for the vector decoders need no change for FR-019. Their round trips still hold.

**Rationale**: Constitution II requires a fuzz target per decoder. The other ten already
have one.

**Alternatives considered**: relying on the generic `no_decoder_panics_on_any_input` test.
Rejected: the constitution asks for both.

## R10. C++ reference vectors (FR-024)

**Decision**: a new generator, `tools/refgen-cpp/generate_coding.cc`, writes
`core/tests/data/cpp_coding_vectors.csv`. It follows `generate_cellunion.cc`'s layout,
`kind,input,expected` with hex bytes. Its sections:

| Section | Input | Expected (from C++) |
|---------|-------|---------------------|
| `VARINT_DECODE` | byte strings: canonical, over-long, tenth byte 0–2 and 127, truncated | value and length, or reject |
| `UINT32_ENCODE`, `UINT64_ENCODE` | value lists (every width, empty, max) | bytes |
| `UINT32_DECODE`, `UINT64_DECODE` | byte strings, including K2 headers | values and length, or reject |
| `CELLID_VECTOR_ENCODE` / `_DECODE` | id lists covering every base length and shift code, including the extended code; malformed headers | bytes; ids or reject |
| `STRING_VECTOR_ENCODE` / `_DECODE` | string lists; decreasing and out-of-range offsets | bytes; strings or reject |
| `POINT_VECTOR_ENCODE` / `_DECODE` | point lists (FAST); unknown formats | bytes; points or reject |
| `FIXED_ENCODE` / `FIXED_DECODE` | `CellId`, `Point`, `Cap`, `Rect`, `Cell` values; malformed and invalid inputs | bytes; value or reject. Rows where the library deviates (K4, count limits) are marked `deviation` |

`core/tests/cpp_coding_diff.rs` checks the library against every row. `verify/tests/
wire_spec_vs_cpp.rs` checks the reference description against every row. CI never needs a
C++ toolchain.

**Rationale**: Constitution I's differential oracle. It is also what turns the reference
description into a checked one.

**Alternatives considered**: Go vectors. Rejected: C++ is canonical, and `refgen-cpp`
already exists.

## R11. The deductive track: tool, pin, crate and CI (FR-010, FR-012)

**Decision**:

- **Tool**: Verus, pinned to release `0.2026.09.20.aef82ed` (installed locally). The
  Makefile records it as `VERUS_VERSION`. CI downloads the release archive for that tag
  from the Verus GitHub releases, installs Rust `1.98.1` with `rustup`, and caches both by
  version.
- **Crate**: `verify/deductive/` (`s2rst-deductive`, `publish = false`, its own
  `[workspace]`). It depends on `vstd`, `verus_builtin` and `verus_builtin_macros`, pinned
  with `=` to the crates.io versions matching the release (R0). Its modules restate the
  varint, the fixed-width helpers, both integer vectors, the cell-id vector and the string
  vector (contracts/deductive-conventions.md).
- **Running**: `verify/deductive/run-verus.sh` runs `verus --crate-type=lib src/lib.rs
  --rlimit 60` under `ulimit -v` (default 4G) and a `systemd-run` scope when available. It
  prints per-module obligations, time and peak memory. `make verify-deductive` calls it.
- **Order of work**: varint and integer vectors first (R0 already holds proven drafts),
  then the cell-id vector (base and shift arithmetic), then the string vector (offsets,
  empty entries).

**Rationale**: R0 shows Verus proving the two hardest round trips in seconds at a third of
a gigabyte. The crates.io crates let the same file build with stable `rustc` and Kani's
nightly, which the agreement checks need (R12). No published crate changes.

**Alternatives considered**:
- **Creusot** (Why3). Rejected: there is no local install and no pilot evidence, and its
  floating-point treatment is no better.
- **Kani loop contracts.** Rejected: unstable in 0.68.0, and they need annotations in
  `core/`.
- **Verifying `core/` in place**, with `verus!` and `cfg(verus_keep_ghost)`. Rejected:
  `vstd` would become a dependency of the published crate, against the constitution's
  dependency rule and FR-012.
- **The latest Verus release.** Rejected for now. The local release matches crates.io
  versions that exist, and bumping is a one-line change with a re-run.

## R12. Linking the restated code to the library (FR-011)

**Decision**:

- **Bounded agreement** (Kani): `verify/src/cd_agree.rs`, with `s2rst-deductive` as a path
  dependency of `verify/`. For each codec, the decoders are compared on every input of the
  R5 byte bound, and the encoders on every value sequence of the R5 element bound. R0
  showed this works: the library's `decode_uint_vector_u64` and the restated decoder agree
  on every 6-byte input in 27 s. Values are compared element-wise over the concrete
  maximum, never with `==` on `Vec`s, which unrolls `memcmp`. Where the library's decoder
  is itself out of reach (the string vector, R5), there is no bounded agreement. The tested
  agreement is then that codec's only link, as FR-011 allows, and the trusted base says so.
- **Tested agreement**: `verify/deductive/tests/agree.rs` (stable `cargo test`) compares the
  same pairs on quickcheck inputs (byte strings up to 4 KiB, vectors up to 10,000
  elements) and on every row of `cpp_coding_vectors.csv`.
- **Drift guard**: `verify/deductive/mirrors.toml` lists each mirrored library function,
  its restated counterpart, the permitted differences and a fingerprint. The fingerprint
  is the FNV-1a 64-bit hash of the function's tokens without doc attributes, taken with
  `syn` and `quote` (dev-dependencies). `tests/drift.rs` recomputes every fingerprint and
  fails on a mismatch, naming the function. `make verify-deductive-bless` rewrites the
  fingerprints after review. The guard runs in the per-change decoding leg (cheap, stable
  toolchain) and in the deductive job.

**Rationale**: FR-011 asks for three links. Token fingerprints ignore formatting and
comments but catch any code change. Running the guard in the per-change leg catches drift
on every change to `core/`, even when the deductive job is skipped.

**Alternatives considered**: hashing raw file text. Rejected: a comment edit elsewhere in
the file would trip it. Generating the restated code from `core/` automatically. Rejected:
the restated code needs proof annotations interleaved with the code, so it cannot be
generated.

## R13. Property tests beyond the bounds (Constitution III, FR-008)

**Decision**: quickcheck round trips for each vector codec over generated vectors (default
quickcheck sizes, `QUICKCHECK_TESTS` honoured). They go in new sibling test files
`encoded_uint_vector_tests.rs`, `encoded_s2cell_id_vector_tests.rs`,
`encoded_string_vector_tests.rs` and `encoded_s2point_vector_tests.rs` (the
`<module>_tests.rs` rule). Each also has one deterministic large case of 1,000,000
elements (SC-008). Every behaviour R5 names as unreached is mapped to one of these tests
or to a fuzz target in the inventory.

**Rationale**: FR-008 requires tests for uncovered sizes, and today only the fuzz targets
cover vector round trips.

**Alternatives considered**: relying on the deductive track alone. Rejected: it proves the
restated code, and the tests exercise the library itself at large sizes.

## R14. Mutation measurement (FR-018)

**Decision**: `cargo-mutants`, pinned to the latest release at the start of implementation
(recorded in `MUTATION.md`; not installed today). It runs over:

- `core/src/s2/encoded_uint_vector.rs`;
- `core/src/s2/encoded_s2cell_id_vector.rs`;
- `core/src/s2/encoded_string_vector.rs`;
- `core/src/s2/encoded_s2point_vector.rs`, with the compact encoder excluded by `--exclude-re`;
- `core/src/s2/encoding.rs`, restricted by `--re` to the in-scope impls and helpers
  (`CellId`, `Point`, `Cap`, `Rect` and `Cell` encode/decode, the `read_*`/`write_*`
  helpers, the varint).

The test command is `cargo test -p s2rst --lib --test decoder_robustness --test
cpp_coding_diff --test property_tests`.

Survivors are exported as patches and run through `run-mutants.sh` against `--harness cd_`
(proof kills). Whatever survives that gets a Kani equivalence check or a written reason.
The measurement runs twice: `PHASE=before` on the branch's starting commit, and
`PHASE=after` at the end. It is a workstation run, estimated from the other branch (about
40 s per mutant with two jobs) at 4–8 hours for an estimated 400–600 mutants. It is not a
CI job.

**Rationale**: FR-018 wants a measurement, not a gate. Restricting the files and the test
selection keeps it to an afternoon.

**Alternatives considered**: a weekly CI job. Rejected for now. The cost is high and the
value is the before/after comparison, though the evaluation may recommend it.

## R15. Resource budget (SC-005, SC-006)

**Decision**: the per-change decoding leg targets under 10 min wall-clock on the 4-vCPU /
16 GB reference runner, within 30 CPU-minutes, with 002's per-harness limits (60 s target,
5 min soft, 12 min hard, 4 GB). Kani spends about 5 s per harness outside its reported
time (002's measurement), so the leg is planned at **at most about 150 harnesses**.
Families are trimmed by sharing: a crash obligation is discharged by the property harness
with the same domain, as in 002. The deductive job is budgeted at 10 minutes, with Verus
itself at seconds (R0). The decoding leg runs in parallel with the other two legs, so the
`CellId` and `CellUnion` budgets are unaffected.

Projection from R0, per change:
- fixed-size decoders, about 75 family members at 1–26 s;
- vector decoders, 2 × 12 integer and 12 cell-id members at 5–80 s;
- encoders and agreement harnesses, about 20 at 10–30 s;
- model round trips, about 10, cheap.

That is about 140 harnesses and 25–30 CPU-minutes, so about 7–8 minutes on 4 vCPUs. The
first task measures the whole leg, then moves the slowest members (the 10–11-byte cell-id
vector ones) to the weekly tier if needed.

**Rationale**: 002's measurement put the harness count, not the solver time, at the centre
of the budget.

**Alternatives considered**: one leg for both 002 and 003. Rejected: the budgets would add
up past 10 minutes.

## R16. Upstream findings (FR-025) and known defects

**Decision**: two candidates are recorded now, and confirmed or dismissed when their proof is
written:

- **U-004 (candidate)**: C++ `S2Cell::Decode` (`s2cell.cc:326–332`) builds `S2Cell(id)` from
  an id that `S2CellId::Decode` does not validate. For the 8 zero bytes, the constructor
  calls `id.level()`, which is U-001's `ABSL_ASSUME(id_ != 0)` undefined behaviour,
  reachable from untrusted input. Ids with face 6 or 7 also reach the face-indexed tables.
  The library rejects such ids (FR-022).
- **U-005 (candidate)**: `GetUintWithLength` with `length > sizeof(T)` is guarded only by
  `ABSL_DCHECK`. In release builds the read goes beyond the value. The library panics
  instead (FR-021 documents it).

**D-003 (candidate)** is the varint tenth byte (R1), handled under FR-014.

Each confirmed finding gets a follow-up task before its proof is marked complete, and a row
in the evaluation.

**Rationale**: the [[upstream-findings-followup]] rule, in place since 001.

**Alternatives considered**: none.

## R17. Reliance on 001 and 002 (FR-016)

**Decision**: `CellId` validity in the `CellId`, `Cell` and cell-id vector properties is
stated through 001's model (`spec::is_valid`), which 001 proved equal to
`CellId::is_valid` for every id (P-VAL-03). The `Cell` decoder's proof executes
`Cell::from(id)`, the face/ij table walk proved in 001. Nothing from 002 is relied on.

`TRUSTED_BASE.md` gains a decoding section. It covers:
- the 001 reliance above;
- `std::io::Read for &[u8]`, `Write for Vec<u8>`, and `Read::take`/`read_to_end`;
- vstd's specifications of `Vec` and slices;
- Verus with Z3 4.16.0 and Rust 1.98.1;
- the restated copies and their links;
- the `f64` comparisons inside `Cap::is_valid` and `Rect::is_valid` (R18);
- every bound.

**Rationale**: FR-016.

**Alternatives considered**: none.

## R18. Validity of `Cap` and `Rect` in the reference description

**Decision**: the reference decoders for `Cap` and `Rect` apply the library's documented
validity predicates, restated in `wire_spec.rs` from their documentation and upstream:
- `Cap`: the centre is unit length (`|‖c‖² − 1| ≤ 5ε`) and `length² ≤ 4`;
- `Rect`: valid latitude and longitude intervals, both empty or both non-empty.

The restatement uses the same floating-point operations in the same order as the library.
If the agreement harness for `Cap` exceeds the budget (an assertion on a float result,
R0 finding 1), the fallback states `Cap`'s acceptance relative to `Cap::is_valid` itself.
The predicate then joins the trusted base, and the inventory says so.

**Rationale**: Validity is part of what the decoder documents (FR-004 part 3). R0 shows the
crash proof is cheap. The agreement proof asserts on the predicate's result, and that cost
is unknown.

**Alternatives considered**: excluding validity from the `Cap` and `Rect` properties.
Rejected: then parts 3 and 4 of the contract would be unproven for two of the five
fixed-size codecs.
