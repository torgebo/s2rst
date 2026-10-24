// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Traversal properties P-TRAV-* (FR-005): walking along the curve, stated against the reference
//! model. The model
//! arithmetic for `advance` and `advance_wrap` uses `i128`, so that steps of `i64::MIN` and
//! `i64::MAX` are exact.

#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::s2::CellId;

use crate::domain::{any_valid, any_valid_at};
use crate::spec;

/// Number of cells at `level` along the whole curve: `6·4^level`.
fn cells(level: u8) -> u64 {
    6 * spec::pow4(u32::from(level))
}

/// P-TRAV-01 (correct result): `next()` is the next cell of the same level along the curve; after
/// the last one comes `end(level)`.
///
/// `next() == from_position(level, pos(d) + 1)`; one past the last cell is `end(level)`.
#[kani::proof]
fn p_trav_01() {
    let (c, d) = any_valid();
    let n = c.next();
    assert_eq!(n.0, spec::from_position(d.level, spec::position(d) + 1));
    kani::cover!(n.0 == spec::end(d.level), "reaches end");
    kani::cover!(n.is_valid() && n.face() != c.face(), "crosses a face");
}

/// P-TRAV-02 (correct result): `prev()` is the previous cell; from `end(l)` it gives the last cell
/// of the level; `next().prev()` returns to the cell.
///
/// `prev() == from_position(level, pos(d) − 1)` for `pos(d) ≥ 1`, `end(l).prev()` is the last cell
/// at `l`, and `next().prev() == c`.
#[kani::proof]
fn p_trav_02() {
    let (c, d) = any_valid();
    let p = spec::position(d);
    if p >= 1 {
        assert_eq!(c.prev().0, spec::from_position(d.level, p - 1));
    }
    assert_eq!(c.next().prev(), c);
    let end = CellId(spec::end(d.level));
    assert_eq!(
        end.prev().0,
        spec::from_position(d.level, cells(d.level) - 1)
    );
    kani::cover!(p == 0, "first cell");
    kani::cover!(p >= 1, "has a predecessor");
}

/// P-TRAV-03 (correct result): `next_wrap()` and `prev_wrap()` step like `next()` and `prev()`, but
/// wrap from the last cell of the level to the first and back.
///
/// `next_wrap()` equals `next()` except that the last cell at a level wraps to `begin(level)`.
/// `prev_wrap()` equals `prev()` except that `begin(level)` wraps to the last cell.
#[kani::proof]
fn p_trav_03() {
    let (c, d) = any_valid();
    let n = cells(d.level);
    let p = spec::position(d);
    assert_eq!(c.next_wrap().0, spec::from_position(d.level, (p + 1) % n));
    assert_eq!(
        c.prev_wrap().0,
        spec::from_position(d.level, (p + n - 1) % n)
    );
    kani::cover!(p == n - 1, "last cell wraps");
    kani::cover!(p == 0, "first cell wraps");
}

/// P-TRAV-04 (correct result): `advance(n)` moves `n` cells along the curve, stopping at the first
/// cell or at `end(level)` rather than going past.
///
/// `advance(n) == from_position(level, clamp(pos(d) + n, 0, 6·4^level))` (i128).
#[kani::proof]
fn p_trav_04() {
    let (c, d) = any_valid();
    let steps: i64 = kani::any();
    let total = i128::from(cells(d.level));
    let target = (i128::from(spec::position(d)) + i128::from(steps)).clamp(0, total);
    assert_eq!(
        c.advance(steps).0,
        spec::from_position(d.level, target as u64)
    );
    kani::cover!(target == total, "clamped to end");
    kani::cover!(target == 0 && steps < 0, "clamped to begin");
    kani::cover!(steps == i64::MAX, "i64::MAX");
    kani::cover!(steps == i64::MIN, "i64::MIN");
}

/// One level of P-TRAV-05. The 31 levels together cover Valid × I64. With the level
/// concrete, the cell count `6·4^level` and the implementation's wrap modulus are constants,
/// which CBMC encodes far more cheaply than a symbolic 128-bit `rem_euclid` (the unsplit
/// harness took 7.8 minutes, over the 5-minute soft limit; research R15).
macro_rules! advance_wrap_level {
    ($($name:ident = $level:literal),* $(,)?) => { $(
        /// P-TRAV-05 (correct result): `advance_wrap(n)` moves `n` cells along the curve, wrapping
        /// around the level.
        ///
        /// `advance_wrap(n) == from_position(level, (pos(d) + n).rem_euclid(6·4^level))` (i128).
        /// One level of the `p_trav_05__l*` family.
        #[kani::proof]
        fn $name() {
            let (c, d) = any_valid_at($level);
            let steps: i64 = kani::any();
            let total = i128::from(cells($level));
            let target = (i128::from(spec::position(d)) + i128::from(steps)).rem_euclid(total);
            assert_eq!(
                c.advance_wrap(steps).0,
                spec::from_position($level, target as u64)
            );
            kani::cover!(steps == i64::MAX, "i64::MAX");
            kani::cover!(steps == i64::MIN, "i64::MIN");
            kani::cover!(
                steps > 0 && target < i128::from(spec::position(d)),
                "wraps forward"
            );
        }
    )* };
}

advance_wrap_level!(
    p_trav_05__l0 = 0,
    p_trav_05__l1 = 1,
    p_trav_05__l2 = 2,
    p_trav_05__l3 = 3,
    p_trav_05__l4 = 4,
    p_trav_05__l5 = 5,
    p_trav_05__l6 = 6,
    p_trav_05__l7 = 7,
    p_trav_05__l8 = 8,
    p_trav_05__l9 = 9,
    p_trav_05__l10 = 10,
    p_trav_05__l11 = 11,
    p_trav_05__l12 = 12,
    p_trav_05__l13 = 13,
    p_trav_05__l14 = 14,
    p_trav_05__l15 = 15,
    p_trav_05__l16 = 16,
    p_trav_05__l17 = 17,
    p_trav_05__l18 = 18,
    p_trav_05__l19 = 19,
    p_trav_05__l20 = 20,
    p_trav_05__l21 = 21,
    p_trav_05__l22 = 22,
    p_trav_05__l23 = 23,
    p_trav_05__l24 = 24,
    p_trav_05__l25 = 25,
    p_trav_05__l26 = 26,
    p_trav_05__l27 = 27,
    p_trav_05__l28 = 28,
    p_trav_05__l29 = 29,
    p_trav_05__l30 = 30,
);

/// P-TRAV-06 (correct result): `distance_from_begin()` is the cell's position along the curve at
/// its level.
///
/// `distance_from_begin() == pos(d)`.
#[kani::proof]
fn p_trav_06() {
    let (c, d) = any_valid();
    assert_eq!(c.distance_from_begin(), spec::position(d) as i64);
    kani::cover!(d.face == 5 && d.level == 30, "far leaf");
}

/// P-TRAV-07 (law): moving `n` steps at once lands where `n` single `next()` or `prev()` steps
/// would.
///
/// Stepping by n agrees with n single steps. For every `n` with `0 ≤ pos(d) + n < 6·4^level`:
/// `advance(n).next() == advance(n + 1)`. When also `pos(d) + n ≥ 1`:
/// `advance(n).prev() == advance(n − 1)`. With `advance(0) == c`, induction gives the FR-005
/// statement.
#[kani::proof]
fn p_trav_07() {
    let (c, d) = any_valid();
    let n: i64 = kani::any();
    let p = i128::from(spec::position(d));
    let at = p + i128::from(n);
    kani::assume(at >= 0 && at < i128::from(cells(d.level)));
    let a = c.advance(n);
    assert_eq!(a.next(), c.advance(n + 1));
    if at >= 1 {
        assert_eq!(a.prev(), c.advance(n - 1));
    }
    assert_eq!(c.advance(0), c);
    kani::cover!(n > 0 && at >= 1, "forward");
    kani::cover!(n < 0, "backward");
}
