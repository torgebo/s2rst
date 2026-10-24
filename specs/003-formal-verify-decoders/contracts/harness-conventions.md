# Contract: Harness and Artifact Conventions (changes for the decoding layer)

The 001 and 002 contracts still apply: crate-root modules, the doc-comment rule, the
forbidden stubs and flags, the resource budget, the concrete-length rule, the smallest
unwind that passes, and the mutant patch format. This file lists only additions. The
deductive track has its own contract, [`deductive-conventions.md`](deductive-conventions.md).

## Modules and names

| Module (`verify/src/`) | Content | Gate |
|------------------------|---------|------|
| `wire_spec.rs` | The reference description of every in-scope wire format (data-model §1). Plain Rust, always compiled, sanity-tested by `tests/wire_spec_vs_cpp.rs`. Its hand-made unit tests are in the sibling `wire_spec_tests.rs` (Constitution III) | none |
| `codec.rs` | The decoder contract: the `Codec` trait, one implementation per in-scope codec, and the generic check bodies (data-model §3) | `cfg(all(kani, feature = "decoding"))` |
| `wire_domain.rs` | Generators for bytes, values and value sequences (below) | same |
| `cd_crash.rs` | `cd_crash::c_<op>`, one per in-scope inventory entry that no shared property harness covers | same |
| `cd_fw.rs`, `cd_uv.rs`, `cd_cv.rs`, `cd_sv.rs`, `cd_pv.rs`, `cd_fx.rs` | Property harnesses `cd_<area>::p_<area>_<nn>`: fixed-width helpers, integer vectors (and the varint through them), cell-identifier vector, string vector, point vector, fixed-size codecs | same |
| `cd_agree.rs` | Bounded agreement between each restated copy and the library (FR-011): `cd_agree::a_<codec>_<dir>`, where `<dir>` is `dec` or `enc` | same |

**Inventory names** are the public path without the crate, for example
`encoded_uint_vector::decode_uint_vector_u64`, or `impl S2Decode for Cap`. **Crash IDs** are
`C-<inventory name>`.

## Split families

The 002 suffixes apply (`__n<k>` element counts, `__b<L>` byte lengths), with one addition:

- `__w<k>`: element width of exactly `k` bytes (integer vectors). It is used only where a
  symbolic width made a harness exceed the budget. Together with `__n<k>`, a family
  `p_uv_02__n<k>_w<j>` covers every count up to `N` and every width 1–8.
- `__t<k>`: point-vector inputs whose first byte has format tag `k` (`t0` uncompressed,
  `t1` compact), and `__tx` for tags 2–7. The three together cover every first byte
  (research R5).

A fixed-size decoder's family is `__b0 … __b<K+1>`, where `K` is its encoded length. The
extra member `__b<K+1>` shows that one byte after the encoding is left unread. With exact
consumption, that covers every longer input (FR-008).

## Generators (`wire_domain.rs`)

Every generator produces **exactly** `N` values or `L` bytes. The length is a const generic,
never a symbolic value (002 research R0).

| Generator | Domain (data-model §2) | Construction |
|-----------|------------------------|--------------|
| `with_bytes::<L>(f)` | `Bytes(L)` | 002's generator, reused (a symbolic `[u8; L]`) |
| `any_u64s::<N>()`, `any_u32s::<N>()` | `Ints(N)` | `N` symbolic values in a `Vec` with capacity `N` |
| `any_ids::<N>()` | `Ids(N)` | `N` symbolic `u64`, wrapped as `CellId` with no validity assumption (the format carries any 64 bits) |
| `any_strings::<N, M>()` | `Strings(N, M)` | `N` byte strings, each of a symbolic length up to `M`, cut from symbolic `[u8; M]` arrays |
| `any_points::<N>()` | `Points(N)` | `N` points from symbolic `f64` bit patterns (`f64::from_bits`), NaN and infinities included |
| `any_valid_cell_id()` | `ValidId` | 001's `any_valid()` (built from the model) |
| `any_valid_cap()`, `any_valid_rect()` | `ValidCap`, `ValidRect` | Symbolic bit patterns, then `assume(<documented validity predicate>)` (data-model §2) |

## Harness body shape

001's five steps, with the codec contract on top. A contract harness calls one generic body
from `codec.rs` with the codec's type parameter, so the statement is written once:

- `check_encoder::<C, N>()`: the encoder half of part 2. The implementation's encoding of
  every value sequence of up to `N` elements equals the reference description's canonical
  encoding, byte for byte.
- `check_agreement::<C, L>()`: parts 2–4 for every `L`-byte input. The implementation's
  decoder and the reference description accept the same inputs, return the same value and
  consume the same number of bytes.
- `check_model_round_trip::<C, N>()`: the reference description's own round trip,
  `ref_decode(enc(v) + rest) = (v, |enc(v)|)`. It runs on the model only.
- **Part 1 (round trip) is the composition of the three** (research R4). The inventory and
  the catalogue name all three harnesses for it. A direct encode-then-decode harness of the
  library is written only for the fixed-size codecs, where it fits. R0 showed that it runs
  out of memory for every vector format.
- Part 5 (bounded reservation) cannot be observed by the checker. Its evidence is the
  reservation measurement and the FR-019 regression test (research R7). The inventory says
  so for every decoder.

Codec-specific properties (header layout, width minimality, base and shift choice, and so
on) are ordinary `p_<area>_<nn>` harnesses.

## Attributes

- `#[kani::unwind(N)]`: the smallest value that passes, normally the byte length plus 1 for
  decoders, or the element count plus 10 for encoders (the varint loop). Research R0 lists
  the measured values.
- No stubbing. `std::io::Read for &[u8]` and `Write for Vec<u8>` run unmodified.
- Collections are compared element by element over the concrete maximum length, never with
  `==` on `Vec` or slices. That unrolls `memcmp` and failed an unwinding assertion in R0.
- An assertion on a floating-point **result** is avoided except where a property needs it.
  Computing with floats is cheap when nothing asserts on the result (the other branch's
  rule, confirmed for `Cap` in research R0).

## Mutant patches

001's format, with ids `M-200` and up, and diffs against the in-scope decoding files.

- `M-200` is the crash-proof demonstration for US1 scenario 4: an index past the input in
  `get_uint_with_length`.
- The FR-019 preallocation cap has no mutant: no proof can observe it (part 5). Its evidence
  is the regression test, written and seen failing before the fix (Constitution III).
- Mutants of the restated copies follow [`deductive-conventions.md`](deductive-conventions.md).
