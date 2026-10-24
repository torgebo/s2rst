# Contract: Harness and Artifact Conventions (changes for `CellUnion`)

001's contract (`specs/001-formal-verify-base-type/contracts/harness-conventions.md`)
still applies: crate-root modules, the doc-comment rule, the forbidden stubs and flags, the
resource budget and the mutant patch format. This file lists only additions.

## Modules and names

| Module (`verify/src/`) | Content | Gate |
|------------------------|---------|------|
| `union_spec.rs` | The reference model of a union (data-model §1). Plain Rust, always compiled, sanity-tested by `tests/union_spec_vs_cpp.rs`; its hand-made unit tests are in the sibling `union_spec_tests.rs` (Constitution III) | none |
| `union_domain.rs` | Generators for unions and bytes (below) | `cfg(all(kani, feature = "cellunion"))` |
| `cu_crash.rs` | `cu_crash::c_<op>`, one per in-scope inventory entry without a shared property harness | same |
| `cu_acc.rs`, `cu_inv.rs`, `cu_norm.rs`, `cu_mem.rs`, `cu_set.rs`, `cu_range.rs`, `cu_den.rs`, `cu_meas.rs`, `cu_enc.rs` | `cu_<area>::p_u<area>_<nn>`, for example `cu_set::p_uset_02` | same |

**Inventory names** are `CellUnion::<method>`, or `impl <Trait> for CellUnion` for the
trait methods. **Crash IDs** are `C-CellUnion::<method>`.

## Split families

001's `__<k>` rule applies: a family's cases together cover the stated domain, and a split
is never a restriction. The new suffixes are:

- `__n<k>`: unions of exactly `k` cells. A family `__n0 … __nN` covers every length up
  to `N`. Lengths are always concrete: a symbolic length cost two to three orders of
  magnitude more in research R0.
- `__b<L>`: decoder input of exactly `L` bytes, as 001 used for text (research R5).
- Two parameters join with `_`, as in 001, for example `cu_set::p_uset_02__n1_n1`.

## Generators (`union_domain.rs`)

Every union domain is built from 001's model decompositions (`spec::compose`,
`spec::range`), never from the implementation's `is_valid` or `normalize`. A harness's
domain therefore does not depend on the code under test.

Every generator produces **exactly** `N` cells or `L` bytes. The length is a const generic,
never a symbolic value (research R0, R4). A domain "at most `N`" in data-model §2 is
covered by the family `__n0 … __nN` (or `__b0 … __bL`), one member per length.

| Generator | Domain (data-model §2), at exactly length `N` or `L` | Construction |
|-----------|------------------------|--------------|
| `any_seq::<N>()` | `Seq` | `N` symbolic `u64` values, pushed into a `Vec` with capacity `N` |
| `any_valid_seq::<N>()` | `ValidSeq` | As above, each element `spec::compose(any_decomposed())`; it also returns the `[Decomposed; N]` |
| `any_valid_union::<N>()` | `ValidUnion` | `any_valid_seq`, plus `assume(range(d[i−1]).1 < range(d[i]).0)` for `0 < i < N` |
| `any_norm_union::<N>()` | `NormUnion` | `any_valid_union`, plus `assume(!union_spec::sibling_group(..))` on every window of four |
| `any_level_mod()` | `LevelMod` | a `u8` with `assume(1 ≤ k ≤ 3)` |
| `with_bytes::<L>(f)` | `Bytes` | A symbolic `[u8; L]`, passed whole as `&bytes[..]`. No `unsafe` is needed |

Unions are built with `CellUnion::from_verbatim`, which stores its argument unchanged
(P-UACC-01).

## Harness body shape

001's five steps, with one addition. A set-semantics property draws a symbolic leaf `x`
with `any_valid_at(30)` and asserts the model's `covers` relation for that `x`. Kani
proves the assertion for every leaf (data-model §1). The output union is read through the
implementation's `range_min` and `range_max`, which 001 proved equal to the model
(TRUSTED_BASE, reliance on 001). Outputs are decomposed, to check validity and
normalisation, through `is_valid`, `face`, `level` and `pos` (001's P-VAL-03, P-VAL-06),
not through `spec::decompose`, whose 31-level loop would force an unwind of 32.

## Attributes

- `#[kani::unwind(N)]`: the **smallest** value that passes the unwinding assertions,
  normally the largest concrete length plus 1. Count the union lengths, the sort
  (insertion sort below 20 elements), the `normalize` loops, `partition_point`'s binary
  search and the decoder's per-cell loop. A larger value is not harmless here. CBMC
  unrolls every loop with a symbolic trip count to the bound, and in research R0 an unwind
  of 6 instead of 3 turned a 1 × 1 intersection that passed in 85 s into an
  out-of-memory run.
- Per-loop `--unwindset` is not used. Its loop identifiers contain the crate hash, which
  changes with the toolchain.
- Stubbing stays forbidden, including `CellId` methods and `std` sorting. The `CellUnion`
  proofs execute the real `CellId` code (research R6).

## Mutant patches

001's format, with ids `M-100` and up and diffs against `core/src/s2/cell_union.rs` or
`core/src/s2/encoding.rs`. An optional `Memory: 8G` header sets the mutant's per-harness
ceiling, for expected harnesses in the slow tier that need more than 4 GB (the 25-byte
decode proofs). `M-100` is the crash-proof overflow demonstration (US1
scenario 4). `M-150` reverses the FR-015 fix, and `M-151` reverses the FR-017 fix.
