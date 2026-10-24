// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Reference model of a cell union: the set of leaf cells it covers (data-model §1).
//!
//! Written from upstream `s2cell_union.h` / `.cc` and the mathematical meaning of a region,
//! on top of the cell model in [`crate::spec`]. It never calls `s2rst::s2::CellUnion` or any
//! `CellId` method (FR-006), so a property stated against it compares two independent
//! descriptions. Like `spec`, it is ordinary Rust: `cargo test` checks it against the
//! committed C++ vectors (`tests/union_spec_vs_cpp.rs`), and the Kani harnesses run it on
//! symbolic inputs.
//!
//! A union is a sequence of cells. Functions on decompositions (`&[Decomposed]`) are for
//! inputs a harness generated from the model; functions on raw ids (`&[u64]`) decompose them
//! first, for inputs that may hold invalid ids.

#![allow(
    clippy::needless_range_loop,
    reason = "the Kani harnesses run this model; indexed loops give CBMC the exact trip count, \
              where iterator adapters over slices left loops it could not bound"
)]

use crate::spec::{self, Decomposed, MAX_LEVEL};

/// The largest cell count a decoder accepts (C++ `FLAGS_s2cell_union_decode_max_num_cells`,
/// default 1,000,000; `s2cell_union.cc:47`).
pub const MAX_DECODE_CELLS: u64 = 1_000_000;

/// The encoding version written and accepted (C++ `kCurrentLosslessEncodingVersionNumber`).
pub const ENCODING_VERSION: u8 = 1;

/// Whether leaf index `x` lies in some member's leaf range: the meaning of a union as a
/// region (`S2CellUnion` class comment).
#[must_use]
pub fn covers(ds: &[Decomposed], x: u64) -> bool {
    for i in 0..ds.len() {
        let (lo, hi) = spec::range(ds[i]);
        if lo <= x && x <= hi {
            return true;
        }
    }
    false
}

/// `IsValid()` on decomposed members: consecutive leaf ranges are strictly increasing and
/// disjoint (`s2cell_union.h:197–199`). Sorted, disjoint ranges imply strictly increasing ids,
/// because every cell id lies inside its own range.
#[must_use]
pub fn valid_union_d(ds: &[Decomposed]) -> bool {
    for i in 1..ds.len() {
        if spec::range(ds[i - 1]).1 >= spec::range(ds[i]).0 {
            return false;
        }
    }
    true
}

/// `IsValid()` on raw ids: every id is a valid cell, and the cells satisfy [`valid_union_d`].
///
/// Written as one indexed pass with no allocation, like every loop in this module: harnesses
/// run it with the unwind of 32 that `spec::decompose` needs, and CBMC could not bound an
/// iterator `collect` over a slice under that unwind (it unrolled the collect 32 times, each
/// with a 31-level search).
#[must_use]
pub fn valid_union(ids: &[u64]) -> bool {
    let mut prev_hi: Option<u64> = None;
    for i in 0..ids.len() {
        let Some(d) = spec::decompose(ids[i]) else {
            return false;
        };
        let (lo, hi) = spec::range(d);
        if prev_hi.is_some_and(|p| p >= lo) {
            return false;
        }
        prev_hi = Some(hi);
    }
    true
}

/// Upstream `AreSiblings` as `IsNormalized` uses it on a valid union: four cells at the same
/// level `ℓ ≥ 1` with the same parent, at child positions 0, 1, 2, 3 in that order.
#[must_use]
pub fn sibling_group(a: Decomposed, b: Decomposed, c: Decomposed, d: Decomposed) -> bool {
    let same = |x: Decomposed| x.face == a.face && x.level == a.level;
    a.level >= 1
        && same(b)
        && same(c)
        && same(d)
        && spec::parent(b) == spec::parent(a)
        && spec::parent(c) == spec::parent(a)
        && spec::parent(d) == spec::parent(a)
        && spec::child_position(a, a.level) == 0
        && spec::child_position(b, b.level) == 1
        && spec::child_position(c, c.level) == 2
        && spec::child_position(d, d.level) == 3
}

/// `IsNormalized()` on decomposed members: valid, and no window of four consecutive members
/// is a [`sibling_group`] (`s2cell_union.h:201–205`).
#[must_use]
pub fn normalized_union_d(ds: &[Decomposed]) -> bool {
    if !valid_union_d(ds) {
        return false;
    }
    for i in 3..ds.len() {
        if sibling_group(ds[i - 3], ds[i - 2], ds[i - 1], ds[i]) {
            return false;
        }
    }
    true
}

/// `IsNormalized()` on raw ids, in one indexed pass like [`valid_union`]: valid, and no
/// member completes a [`sibling_group`] with the three before it.
#[must_use]
pub fn normalized_union(ids: &[u64]) -> bool {
    let mut window: [Option<Decomposed>; 3] = [None; 3];
    let mut prev_hi: Option<u64> = None;
    for i in 0..ids.len() {
        let Some(d) = spec::decompose(ids[i]) else {
            return false;
        };
        let (lo, hi) = spec::range(d);
        if prev_hi.is_some_and(|p| p >= lo) {
            return false;
        }
        if let [Some(a), Some(b), Some(c)] = window
            && sibling_group(a, b, c, d)
        {
            return false;
        }
        window = [window[1], window[2], Some(d)];
        prev_hi = Some(hi);
    }
    true
}

/// The number of leaf cells the members cover, counted with multiplicity: `Σ span(level)`.
/// For a valid union the members are disjoint, so this is the size of the covered set, at
/// most `6·4^30` (`LeafCellsCovered()`, `s2cell_union.h:303–305`). It is a `u128` so that it
/// is defined for any sequence.
#[must_use]
pub fn leaf_count(ds: &[Decomposed]) -> u128 {
    let mut total = 0;
    for i in 0..ds.len() {
        total += u128::from(spec::span(ds[i].level));
    }
    total
}

/// `Contains(S2CellId)`: some member contains `c`. On a union that is not normalised, four
/// children do not contain their parent (the upstream caveat).
#[must_use]
pub fn contains_id(ds: &[Decomposed], c: Decomposed) -> bool {
    for i in 0..ds.len() {
        if spec::contains(ds[i], c) {
            return true;
        }
    }
    false
}

/// `Intersects(S2CellId)`: some member's leaf range overlaps `c`'s.
#[must_use]
pub fn intersects_id(ds: &[Decomposed], c: Decomposed) -> bool {
    let (clo, chi) = spec::range(c);
    for i in 0..ds.len() {
        let (lo, hi) = spec::range(ds[i]);
        if lo <= chi && clo <= hi {
            return true;
        }
    }
    false
}

/// The level `Denormalize` gives a cell at `level` (`s2cell_union.cc:215–223`): at least
/// `min_level`, rounded up so that `(new_level − min_level)` is a multiple of `level_mod`,
/// but never past 30. Written as C++ writes it, not in the Rust implementation's form.
#[must_use]
pub fn denorm_level(level: u8, min_level: u8, level_mod: u8) -> u8 {
    let mut new_level = min_level.max(level);
    if level_mod > 1 {
        new_level += (MAX_LEVEL - (new_level - min_level)) % level_mod;
        new_level = new_level.min(MAX_LEVEL);
    }
    new_level
}

/// `Denormalize(min_level, level_mod)`: each member in order, kept when its
/// [`denorm_level`] equals its own level, otherwise replaced by all of its descendants at
/// that level, in curve order.
#[must_use]
pub fn denormalize(ds: &[Decomposed], min_level: u8, level_mod: u8) -> Vec<u64> {
    let mut out = Vec::new();
    for i in 0..ds.len() {
        let d = ds[i];
        let new_level = denorm_level(d.level, min_level, level_mod);
        if new_level == d.level {
            out.push(spec::compose(d));
            continue;
        }
        let gap = 2 * u32::from(new_level - d.level);
        let first = d.path << gap;
        for p in 0..(1u64 << gap) {
            out.push(spec::compose(Decomposed {
                face: d.face,
                level: new_level,
                path: first + p,
            }));
        }
    }
    out
}

/// `Encode()`: the version byte, the cell count as 8 little-endian bytes, then each id as 8
/// little-endian bytes (`s2cell_union.cc`, `S2CellUnion::Encode`).
#[must_use]
pub fn encoding(ids: &[u64]) -> Vec<u8> {
    let mut out = Vec::with_capacity(9 + 8 * ids.len());
    out.push(ENCODING_VERSION);
    out.extend_from_slice(&(ids.len() as u64).to_le_bytes());
    for i in 0..ids.len() {
        out.extend_from_slice(&ids[i].to_le_bytes());
    }
    out
}

/// `Decode()` with the library's documented deviation (FR-017): `Some(ids)` exactly when the
/// input starts with a version `≤ 1`, a count `n ≤ 1,000,000`, and at least `8n` more bytes
/// whose `n` ids form a valid union. The ids are kept as encoded. Trailing bytes are
/// ignored, as in C++ (`s2cell_union.cc:544–562`), which validates nothing (U-003).
#[must_use]
pub fn decode(bytes: &[u8]) -> Option<Vec<u64>> {
    decode_with(bytes, cell_range)
}

/// The leaf-index range of `x` if it is a valid cell: the model's definition of a cell
/// (`spec::decompose`, a search over the 31 levels).
#[must_use]
pub fn cell_range(x: u64) -> Option<(u64, u64)> {
    spec::decompose(x).map(spec::range)
}

/// [`decode`], with the validity and leaf range of each id given by `range_of`. Two ids
/// overlap or are out of order exactly when the first range does not end before the second
/// starts, so any monotone renumbering of the leaves gives the same result.
///
/// `decode` passes [`cell_range`]. A harness may pass the implementation's `is_valid`,
/// `range_min` and `range_max` instead, which 001 proved equal to the model (P-VAL-03,
/// P-RANGE-01), to avoid the search's unwind of 32. The loop runs over the concrete byte
/// length rather than the decoded count, so CBMC sees its exact bound.
#[must_use]
pub fn decode_with(bytes: &[u8], range_of: impl Fn(u64) -> Option<(u64, u64)>) -> Option<Vec<u64>> {
    let (&version, rest) = bytes.split_first()?;
    if version > ENCODING_VERSION || rest.len() < 8 {
        return None;
    }
    let n = u64::from_le_bytes(rest[..8].try_into().ok()?);
    let body = &rest[8..];
    if n > MAX_DECODE_CELLS || n > (body.len() / 8) as u64 {
        return None;
    }
    let mut ids = Vec::new();
    let mut prev_hi: Option<u64> = None;
    for i in 0..body.len() / 8 {
        if i as u64 >= n {
            break;
        }
        let id = u64::from_le_bytes(body[8 * i..8 * i + 8].try_into().ok()?);
        let (lo, hi) = range_of(id)?;
        if prev_hi.is_some_and(|p| p >= lo) {
            return None;
        }
        prev_hi = Some(hi);
        ids.push(id);
    }
    Some(ids)
}

#[cfg(test)]
#[path = "union_spec_tests.rs"]
mod union_spec_tests;
