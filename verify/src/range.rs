// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Range and containment properties P-RANGE-* (FR-005): which leaves a cell contains, stated
//! against the reference model.

use crate::domain::{any_valid, any_valid_or_end};
use crate::spec;

/// P-RANGE-01 (correct result): `range_min()` and `range_max()` are the first and the last leaf
/// inside the cell.
///
/// `range_min() == leaf_id(range(d).0)` and `range_max() == leaf_id(range(d).1)`.
#[kani::proof]
fn p_range_01() {
    let (c, d) = any_valid();
    let (lo, hi) = spec::range(d);
    assert_eq!(c.range_min().0, spec::leaf_id(lo));
    assert_eq!(c.range_max().0, spec::leaf_id(hi));
    kani::cover!(d.level == 0, "face");
    kani::cover!(lo == hi, "leaf");
}

/// P-RANGE-02 (correct result): `a.contains(b)` holds exactly when `b` is `a` or lies inside it.
///
/// `a.contains(b) == spec::contains(da, db)`.
#[kani::proof]
fn p_range_02() {
    let (a, da) = any_valid();
    let (b, db) = any_valid();
    let r = a.contains(b);
    assert_eq!(r, spec::contains(da, db));
    kani::cover!(r && da.level < db.level, "strict containment");
    kani::cover!(!r, "not contained");
}

/// P-RANGE-03 (correct result): `a.intersects(b)` holds exactly when one of the two contains the
/// other.
///
/// `a.intersects(b) == (contains(da, db) ∨ contains(db, da))`.
#[kani::proof]
fn p_range_03() {
    let (a, da) = any_valid();
    let (b, db) = any_valid();
    let r = a.intersects(b);
    assert_eq!(r, spec::contains(da, db) || spec::contains(db, da));
    kani::cover!(r && !spec::contains(da, db), "b contains a");
    kani::cover!(!r, "disjoint");
}

/// P-RANGE-04 (law): `a.contains(b)` holds exactly when `b`'s id lies between `a.range_min()` and
/// `a.range_max()`.
///
/// `a.contains(b) ⇔ a.range_min().id() ≤ b.id() ≤ a.range_max().id()`.
#[kani::proof]
fn p_range_04() {
    let (a, _) = any_valid();
    let b = any_valid_or_end();
    let r = a.contains(b);
    assert_eq!(
        r,
        a.range_min().id() <= b.id() && b.id() <= a.range_max().id()
    );
    kani::cover!(r, "contained");
    kani::cover!(!r && !b.is_valid(), "past the end");
}

/// P-RANGE-05 (law): two cells are either nested or disjoint; they never partly overlap.
///
/// Nested or disjoint: `a.contains(b) ∨ b.contains(a) ∨ ¬a.intersects(b)`.
#[kani::proof]
fn p_range_05() {
    let (a, _) = any_valid();
    let (b, _) = any_valid();
    assert!(a.contains(b) || b.contains(a) || !a.intersects(b));
    kani::cover!(a.contains(b) && a != b, "nested");
    kani::cover!(!a.intersects(b), "disjoint");
}

/// P-RANGE-06 (correct result): `maximum_tile(limit)` is the largest cell that starts where this
/// one starts and ends before `limit` begins, or `limit` itself if this cell starts at or after it.
///
/// `maximum_tile(limit)` is `limit` if `range_min() ≥ limit.range_min()`. Otherwise it is the
/// largest valid cell `r` with `r.range_min() == range_min()` and
/// `r.range_max() < limit.range_min()` (`r` is a face, or `r.parent()` breaks one of the two
/// conditions). Leaf indices are compared through the model (`leaf id = 2·index + 1`).
///
/// Compositional: the result is checked through `is_valid`, `range_min`, `range_max` and
/// `parent`, which P-VAL-03, P-RANGE-01 and P-HIER-03 prove equal to the model on every valid
/// cell. Checking it through the model's `decompose` search instead ran past the 8-minute
/// budget. It stays unsplit although it is over the 5-minute soft limit: a split by level
/// cut each member to 13–36 s but nearly doubled the total CPU time (research R15).
#[kani::proof]
#[kani::unwind(32)]
fn p_range_06() {
    let (c, d) = any_valid();
    let limit = any_valid_or_end();
    let start = spec::leaf_index(d);
    let limit_lo = limit.range_min().id() >> 1;
    let r = c.maximum_tile(limit);
    if start >= limit_lo {
        assert_eq!(r, limit);
    } else {
        assert!(r.is_valid());
        assert_eq!(r.range_min().id() >> 1, start);
        assert!(r.range_max().id() >> 1 < limit_lo);
        if !r.is_face() {
            let p = r.parent();
            assert!(p.range_min().id() >> 1 != start || p.range_max().id() >> 1 >= limit_lo);
        }
        kani::cover!(r.level().as_u8() < d.level, "grows");
        kani::cover!(r.level().as_u8() > d.level, "shrinks");
        kani::cover!(r.is_face(), "a whole face");
    }
    kani::cover!(start >= limit_lo, "returns the limit");
}
