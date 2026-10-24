// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Hierarchy properties P-HIER-* (FR-005): parents, children and the levels, stated against
//! the reference model `spec`.

use s2rst::s2::{CellId, Level};

use crate::domain::{any_level, any_valid};
use crate::spec::{self, Decomposed};

/// P-HIER-01 (correct result): `children()` gives the cell's four children, in curve order.
///
/// `children()[k] == compose(child(d, k))` for `k < 4`.
#[kani::proof]
fn p_hier_01() {
    let (c, d) = any_valid();
    kani::assume(d.level < 30);
    let ch = c.children();
    for k in 0..4u8 {
        assert_eq!(ch[usize::from(k)].0, spec::compose(spec::child(d, k)));
    }
    kani::cover!(d.level == 0, "face");
    kani::cover!(d.level == 29, "parent of leaves");
}

/// P-HIER-02 (law): a cell's four children split its leaves into four consecutive runs in curve
/// order, with no gap and no overlap.
///
/// The four children partition the parent in curve order:
/// `children()[0].range_min() == range_min()`, `children()[3].range_max() == range_max()`,
/// `children()[k].range_max().id() + 2 == children()[k+1].range_min().id()`.
#[kani::proof]
fn p_hier_02() {
    let (c, d) = any_valid();
    kani::assume(d.level < 30);
    let ch = c.children();
    assert_eq!(ch[0].range_min(), c.range_min());
    assert_eq!(ch[3].range_max(), c.range_max());
    for k in 0..3 {
        assert_eq!(ch[k].range_max().id() + 2, ch[k + 1].range_min().id());
    }
    kani::cover!(d.level == 29, "leaf children");
}

/// P-HIER-03 (correct result): `parent()` is the cell one level up that contains it.
///
/// `parent() == compose(parent(d))`, for level ≥ 1.
#[kani::proof]
fn p_hier_03() {
    let (c, d) = any_valid();
    kani::assume(d.level >= 1);
    assert_eq!(c.parent().0, spec::compose(spec::parent(d)));
    kani::cover!(d.level == 1, "parent is a face");
    kani::cover!(d.level == 30, "parent of a leaf");
}

/// P-HIER-04 (correct result): `parent_at_level(l)` is the cell's ancestor at level `l`.
///
/// `parent_at_level(l) == compose(ancestor(d, l))`, for `l ≤ level`.
#[kani::proof]
fn p_hier_04() {
    let (c, d) = any_valid();
    let l = any_level();
    kani::assume(l.as_u8() <= d.level);
    assert_eq!(
        c.parent_at_level(l).0,
        spec::compose(spec::ancestor(d, l.as_u8()))
    );
    kani::cover!(l.as_u8() == 0 && d.level == 30, "leaf to face");
}

/// P-HIER-05 (correct result): `child_position(l)` tells which of its parent's four children the
/// cell's level-`l` ancestor is.
///
/// `child_position(l) == spec::child_position(d, l)`, for `1 ≤ l ≤ level`.
#[kani::proof]
fn p_hier_05() {
    let (c, d) = any_valid();
    let l = any_level();
    kani::assume(l.as_u8() >= 1 && l.as_u8() <= d.level);
    let p = c.child_position(l);
    assert_eq!(p, spec::child_position(d, l.as_u8()));
    kani::cover!(p == 3, "position 3");
}

/// P-HIER-06 (correct result): `child_begin()` is the first child and `child_end()` the position
/// just after the last.
///
/// `child_begin() == compose(child(d, 0))` and
/// `child_end() == from_position(level+1, pos(child(d, 3)) + 1)`, for level < 30.
#[kani::proof]
fn p_hier_06() {
    let (c, d) = any_valid();
    kani::assume(d.level < 30);
    assert_eq!(c.child_begin().0, spec::compose(spec::child(d, 0)));
    let last = spec::child(d, 3);
    assert_eq!(
        c.child_end().0,
        spec::from_position(d.level + 1, spec::position(last) + 1)
    );
    kani::cover!(
        c.child_end().0 == spec::end(d.level + 1),
        "child_end is end"
    );
}

/// P-HIER-07 (correct result): `child_begin_at_level(l)` and `child_end_at_level(l)` bracket the
/// cell's descendants at level `l`.
///
/// `child_begin_at_level(l) == compose({face, l, path·4^(l−level)})` and
/// `child_end_at_level(l) == from_position(l, pos(that) + 4^(l−level))`, for `l ≥ level`.
#[kani::proof]
fn p_hier_07() {
    let (c, d) = any_valid();
    let l = any_level();
    kani::assume(l.as_u8() >= d.level);
    let span = spec::pow4(u32::from(l.as_u8() - d.level));
    let first = Decomposed {
        face: d.face,
        level: l.as_u8(),
        path: d.path * span,
    };
    assert_eq!(c.child_begin_at_level(l).0, spec::compose(first));
    assert_eq!(
        c.child_end_at_level(l).0,
        spec::from_position(l.as_u8(), spec::position(first) + span)
    );
    kani::cover!(d.level == 0 && l.as_u8() == 30, "face to leaves");
}

/// P-HIER-08 (correct result): `begin(l)` is the first cell of level `l` and `end(l)` the position
/// just after the last.
///
/// `begin(l) == compose({0, l, 0})` and `end(l) == spec::end(l)`.
#[kani::proof]
fn p_hier_08() {
    let l: Level = any_level();
    assert_eq!(
        CellId::begin(l).0,
        spec::compose(Decomposed {
            face: 0,
            level: l.as_u8(),
            path: 0
        })
    );
    assert_eq!(CellId::end(l).0, spec::end(l.as_u8()));
    kani::cover!(l.as_u8() == 30, "leaves");
}
