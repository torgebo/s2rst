// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Input-domain generators for the `CellUnion` harnesses (data-model §2; research R4).
//!
//! Every union is built from the reference model ([`crate::domain::any_decomposed`] and
//! [`spec::compose`]), never by assuming the implementation's `is_valid` or `normalize`, so a
//! harness's domain does not depend on the code under test.
//!
//! Every generator produces **exactly** `N` cells or `L` bytes: the length is a const
//! generic, never a symbolic value. A symbolic length made CBMC unwind every loop over the
//! union to the harness bound on every path, which cost two to three orders of magnitude in
//! the planning probe (research R0). A domain "of at most `N` cells" is covered by the family
//! `__n0 … __nN`, one harness per length. Loops over the generated arrays index with
//! `0..N`, so that CBMC sees their exact trip count even under a large unwind.

use s2rst::s2::CellId;

use crate::domain::{any_decomposed, any_valid_at};
use crate::spec::{self, Decomposed};
use crate::union_spec;

/// `Seq`: exactly `N` ids of any bit pattern, with the raw values. Harnesses compare the
/// model against the raw values, never against the implementation's `CellId::id()`.
pub fn any_seq<const N: usize>() -> (Vec<CellId>, [u64; N]) {
    let raw: [u64; N] = kani::any();
    let mut v = Vec::with_capacity(N);
    for i in 0..N {
        v.push(CellId(raw[i]));
    }
    (v, raw)
}

/// `ValidSeq`: exactly `N` valid cells in any order, possibly overlapping or repeated, with
/// their decompositions.
pub fn any_valid_seq<const N: usize>() -> (Vec<CellId>, [Decomposed; N]) {
    let ds: [Decomposed; N] = core::array::from_fn(|_| any_decomposed());
    let mut v = Vec::with_capacity(N);
    for i in 0..N {
        v.push(CellId(spec::compose(ds[i])));
    }
    (v, ds)
}

/// `ValidUnion`: exactly `N` valid cells whose leaf ranges are strictly increasing and
/// disjoint (`union_spec::valid_union_d`, upstream `IsValid()`).
pub fn any_valid_union<const N: usize>() -> (Vec<CellId>, [Decomposed; N]) {
    let (v, ds) = any_valid_seq::<N>();
    for i in 1..N {
        kani::assume(spec::range(ds[i - 1]).1 < spec::range(ds[i]).0);
    }
    (v, ds)
}

/// `NormUnion`: a `ValidUnion` with no four consecutive siblings (upstream `IsNormalized()`).
/// For `N ≤ 3` it is the same domain as [`any_valid_union`].
pub fn any_norm_union<const N: usize>() -> (Vec<CellId>, [Decomposed; N]) {
    let (v, ds) = any_valid_union::<N>();
    for w in ds.windows(4) {
        kani::assume(!union_spec::sibling_group(w[0], w[1], w[2], w[3]));
    }
    (v, ds)
}

/// `LevelMod`: `1..=3`, by the upstream `DCHECK` and the library's `debug_assert!`.
pub fn any_level_mod() -> u8 {
    let k: u8 = kani::any();
    kani::assume((1..=3).contains(&k));
    k
}

/// `Bytes`: every byte string of exactly `L` bytes. No `unsafe` is needed: the decoder takes
/// bytes.
pub fn with_bytes<const L: usize>(f: impl FnOnce(&[u8])) {
    let bytes: [u8; L] = kani::any();
    f(&bytes[..]);
}

/// A symbolic leaf cell and its leaf index. A property asserted for it holds for every leaf,
/// which is how set equalities are stated without enumerating leaves (research R3).
pub fn any_leaf() -> (CellId, u64) {
    let (c, d) = any_valid_at(30);
    (c, spec::leaf_index(d))
}

/// Whether an implementation output of at most `MAX` cells covers `leaf`, read through
/// `range_min` and `range_max`, which 001 proved equal to the model's `range` for every valid
/// cell (P-RANGE-01; data-model §7). Outputs of the code under test are read only through the
/// functions in this section.
///
/// An output's length is symbolic to CBMC, so every reader loops over the concrete `MAX` with
/// a length guard: a loop over the symbolic length, or a slice comparison, made the decoding
/// and set-operation proofs run out of memory.
pub fn out_covers<const MAX: usize>(ids: &[CellId], leaf: CellId) -> bool {
    assert!(ids.len() <= MAX);
    let mut covered = false;
    for i in 0..MAX {
        if i < ids.len() && ids[i].range_min() <= leaf && leaf <= ids[i].range_max() {
            covered = true;
        }
    }
    covered
}

/// The decompositions of an implementation output of at most `MAX` cells, or `None` if any
/// id is invalid. Used only to check the model's validity and normalisation predicates on
/// outputs (research R3).
///
/// Each id is read through `CellId::is_valid`, `face`, `level` and `pos`, which 001 proved
/// equal to the model for every id (P-VAL-03) and every valid id (P-VAL-06), instead of the
/// model's 31-level `spec::decompose`. That adds no loop to the normalising harnesses, whose
/// unwind must stay small (data-model §7).
pub fn out_decomposed<const MAX: usize>(ids: &[CellId]) -> Option<Vec<Decomposed>> {
    assert!(ids.len() <= MAX);
    let mut out = Vec::with_capacity(MAX);
    for i in 0..MAX {
        if i < ids.len() {
            let c = ids[i];
            if !c.is_valid() {
                return None;
            }
            let level = c.level().as_u8();
            out.push(Decomposed {
                face: c.face().as_u8(),
                level,
                path: c.pos() >> (61 - 2 * u32::from(level)),
            });
        }
    }
    Some(out)
}

/// Whether an implementation output of at most `MAX` cells is a valid union: every id valid,
/// and each cell's last leaf before the next cell's first. Read through `is_valid`,
/// `range_min` and `range_max` (001's P-VAL-03 and P-RANGE-01), so this is the model's
/// `valid_union` on the output, with no search and a loop over the concrete `MAX`.
pub fn out_valid<const MAX: usize>(ids: &[CellId]) -> bool {
    assert!(ids.len() <= MAX);
    let mut valid = true;
    for i in 0..MAX {
        if i < ids.len() {
            if !ids[i].is_valid() {
                valid = false;
            } else if i > 0 && ids[i - 1].is_valid() && ids[i - 1].range_max() >= ids[i].range_min()
            {
                valid = false;
            }
        }
    }
    valid
}

/// Whether an implementation output of at most `MAX` cells is a normalised union. With
/// `MAX < 4` no window of four exists, so this is [`out_valid`]; otherwise the sibling
/// check runs on [`out_decomposed`].
pub fn out_normalized<const MAX: usize>(ids: &[CellId]) -> bool {
    if MAX < 4 {
        out_valid::<MAX>(ids)
    } else {
        out_decomposed::<MAX>(ids).is_some_and(|d| union_spec::normalized_union_d(&d))
    }
}

/// The validity and range of an id through the implementation's `is_valid`, `range_min` and
/// `range_max` (001's P-VAL-03 and P-RANGE-01), in leaf ids rather than leaf indices: an
/// order-preserving renumbering, which `union_spec::decode_with` accepts.
pub fn impl_cell_range(x: u64) -> Option<(u64, u64)> {
    let c = CellId(x);
    c.is_valid().then(|| (c.range_min().0, c.range_max().0))
}

/// Whether an implementation output of at most `MAX` cells has exactly the ids `want`.
pub fn out_ids_equal<const MAX: usize>(ids: &[CellId], want: &[u64]) -> bool {
    assert!(ids.len() <= MAX && want.len() <= MAX);
    let mut equal = ids.len() == want.len();
    for i in 0..MAX {
        if i < ids.len() && i < want.len() && ids[i].0 != want[i] {
            equal = false;
        }
    }
    equal
}
