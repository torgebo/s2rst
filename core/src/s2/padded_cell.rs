// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! Padded cell for efficient edge clipping against cell boundaries.
//!
//! A [`PaddedCell`] is a [`CellId`] whose (u,v)-range has been expanded on
//! all sides by a given amount of "padding". Its methods are optimised for
//! clipping edges against cell boundaries to find which cells are
//! intersected by a set of edges.
//!
//! Corresponds to C++ `s2padded_cell.h`, Go `s2/paddedcell.go`.

#![expect(
    clippy::cast_sign_loss,
    reason = "IJ coordinates (i32/i64) cast to u32 for SI/TI values — always non-negative in valid cells"
)]
#![expect(
    clippy::cast_possible_truncation,
    reason = "IJ coords (i64->u32) and level (i32->u8) — bounded by cell arithmetic"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "u32 -> i32 for IJ coordinate arithmetic — bounded by cell structure"
)]
use crate::r1;
use crate::r2;
use crate::s2::coords::{
    self, IJ_TO_POS, INVERT_MASK, Level, MAX_CELL_LEVEL, POS_TO_IJ, POS_TO_ORIENTATION, SWAP_MASK,
    face_si_ti_to_xyz, si_ti_to_st, st_to_ij, st_to_uv, uv_to_st,
};
use crate::s2::{CellId, Point, from_face_ij, ij_level_to_bound_uv, size_ij};

/// A [`CellId`] with an expanded (u,v)-bound for edge clipping.
#[derive(Clone, Debug, PartialEq)]
pub struct PaddedCell {
    id: CellId,
    padding: f64,
    bound: r2::Rect,
    middle: Option<r2::Rect>,
    i_lo: i32,
    j_lo: i32,
    orientation: u8,
    level: Level,
}

impl PaddedCell {
    /// Constructs a padded cell from a [`CellId`] with the given padding
    /// (in UV-space units).
    #[inline]
    pub fn from_cell_id(id: CellId, padding: f64) -> Self {
        // Fast path for face cells (the most common case).
        if id.is_face() {
            let limit = padding + 1.0;
            return PaddedCell {
                id,
                padding,
                bound: r2::Rect::new(
                    r1::Interval::new(-limit, limit),
                    r1::Interval::new(-limit, limit),
                ),
                middle: Some(r2::Rect::new(
                    r1::Interval::new(-padding, padding),
                    r1::Interval::new(-padding, padding),
                )),
                i_lo: 0,
                j_lo: 0,
                orientation: id.face().as_u8() & 1,
                level: Level::MIN,
            };
        }

        let (_, i, j, orient) = id.to_face_ij_orientation();
        let level = id.level();
        let bound = ij_level_to_bound_uv(i, j, level).expanded_by_margin(padding);
        let ij_size = size_ij(level);
        PaddedCell {
            id,
            padding,
            bound,
            middle: None,
            i_lo: i & -ij_size,
            j_lo: j & -ij_size,
            orientation: orient,
            level,
        }
    }

    /// Constructs the child of `parent` with the given `(i, j)` index
    /// (0 or 1 along each axis; increasing u and v respectively).
    #[inline]
    pub fn from_parent_ij(parent: &mut PaddedCell, i: i32, j: i32) -> Self {
        let pos = IJ_TO_POS[parent.orientation as usize][(2 * i + j) as usize];
        let child_id = parent.id.children()[pos as usize];
        let child_level = parent.level + 1;
        let ij_size = size_ij(child_level);

        let i_lo = parent.i_lo + i * ij_size;
        let j_lo = parent.j_lo + j * ij_size;

        let middle = parent.middle();
        let mut bound = parent.bound;
        if i == 1 {
            bound.x.lo = middle.x.lo;
        } else {
            bound.x.hi = middle.x.hi;
        }
        if j == 1 {
            bound.y.lo = middle.y.lo;
        } else {
            bound.y.hi = middle.y.hi;
        }

        PaddedCell {
            id: child_id,
            padding: parent.padding,
            bound,
            middle: None,
            i_lo,
            j_lo,
            orientation: parent.orientation ^ POS_TO_ORIENTATION[pos as usize],
            level: child_level,
        }
    }

    /// Returns the cell ID.
    #[inline]
    pub fn cell_id(&self) -> CellId {
        self.id
    }

    /// Returns the padding amount.
    #[inline]
    pub fn padding(&self) -> f64 {
        self.padding
    }

    /// Returns the level of this cell.
    #[inline]
    pub fn level(&self) -> Level {
        self.level
    }

    /// Returns the center of this cell as a Point.
    #[inline]
    pub fn center(&self) -> Point {
        let ij_size = size_ij(self.level);
        let si = (2 * self.i_lo + ij_size) as u32;
        let ti = (2 * self.j_lo + ij_size) as u32;
        Point(face_si_ti_to_xyz(self.id.face(), si, ti).normalize())
    }

    /// Returns the (u,v)-bound of this cell including padding.
    #[inline]
    pub fn bound(&self) -> r2::Rect {
        self.bound
    }

    /// Returns the rectangle in the middle of this cell that belongs to
    /// all four children in (u,v)-space. Computed lazily.
    pub fn middle(&mut self) -> r2::Rect {
        if let Some(m) = self.middle {
            return m;
        }
        let ij_size = size_ij(self.level);
        let u = st_to_uv(si_ti_to_st((2 * self.i_lo + ij_size) as u32));
        let v = st_to_uv(si_ti_to_st((2 * self.j_lo + ij_size) as u32));
        let m = r2::Rect::new(
            r1::Interval::new(u - self.padding, u + self.padding),
            r1::Interval::new(v - self.padding, v + self.padding),
        );
        self.middle = Some(m);
        m
    }

    /// Returns the `(i, j)` coordinates for the child cell at the given
    /// Hilbert curve traversal position (0–3).
    #[inline]
    pub fn child_ij(&self, pos: usize) -> (i32, i32) {
        let ij = POS_TO_IJ[self.orientation as usize][pos];
        (i32::from(ij >> 1), i32::from(ij & 1))
    }

    /// Returns the vertex where the space-filling curve enters this cell.
    pub fn entry_vertex(&self) -> Point {
        let mut i = i64::from(self.i_lo);
        let mut j = i64::from(self.j_lo);
        if self.orientation & INVERT_MASK != 0 {
            let ij_size = i64::from(size_ij(self.level));
            i += ij_size;
            j += ij_size;
        }
        Point(face_si_ti_to_xyz(self.id.face(), (2 * i) as u32, (2 * j) as u32).normalize())
    }

    /// Returns the vertex where the space-filling curve exits this cell.
    pub fn exit_vertex(&self) -> Point {
        let mut i = i64::from(self.i_lo);
        let mut j = i64::from(self.j_lo);
        let ij_size = i64::from(size_ij(self.level));
        if self.orientation == 0 || self.orientation == SWAP_MASK | INVERT_MASK {
            i += ij_size;
        } else {
            j += ij_size;
        }
        Point(face_si_ti_to_xyz(self.id.face(), (2 * i) as u32, (2 * j) as u32).normalize())
    }

    /// Returns the smallest [`CellId`] that contains all descendants of
    /// this padded cell whose bounds intersect `rect`.
    ///
    /// `rect` must intersect the bounds of this cell.
    pub fn shrink_to_fit(&mut self, rect: r2::Rect) -> CellId {
        debug_assert!(self.bound().intersects(rect));
        // Quick rejection: if rect contains the center along either axis,
        // no further shrinking is possible.
        if self.level == 0 && (rect.x.contains(0.0) || rect.y.contains(0.0)) {
            return self.id;
        }

        let ij_size = size_ij(self.level);
        if rect
            .x
            .contains(st_to_uv(si_ti_to_st((2 * self.i_lo + ij_size) as u32)))
            || rect
                .y
                .contains(st_to_uv(si_ti_to_st((2 * self.j_lo + ij_size) as u32)))
        {
            return self.id;
        }

        // Expand rect by padding and find the range of (i,j) it spans.
        let padded = rect.expanded_by_margin(self.padding + 1.5 * coords::DBL_EPSILON);

        let i_min = self.i_lo.max(st_to_ij(uv_to_st(padded.x.lo)));
        let i_max = (self.i_lo + ij_size - 1).min(st_to_ij(uv_to_st(padded.x.hi)));
        let i_xor = i_min ^ i_max;

        let j_min = self.j_lo.max(st_to_ij(uv_to_st(padded.y.lo)));
        let j_max = (self.j_lo + ij_size - 1).min(st_to_ij(uv_to_st(padded.y.hi)));
        let j_xor = j_min ^ j_max;

        // The highest bit position where the two endpoints differ tells us
        // the first level at which at least two children intersect rect.
        let level_msb = (((i_xor | j_xor) as u64) << 1) + 1;
        let level = i32::from(MAX_CELL_LEVEL) - (64 - level_msb.leading_zeros() as i32);
        if level <= i32::from(self.level) {
            return self.id;
        }

        from_face_ij(self.id.face(), i_min, j_min).parent_at_level(level as u8)
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_padded_cell_face() {
        let id = CellId::from_face(3);
        let pc = PaddedCell::from_cell_id(id, 0.5);
        assert_eq!(pc.cell_id(), id);
        assert_eq!(pc.level(), 0);
        assert_eq!(pc.padding(), 0.5);
        // Bound should be [-1.5, 1.5] × [-1.5, 1.5].
        assert!((pc.bound().x.lo - (-1.5)).abs() < 1e-15);
        assert!((pc.bound().x.hi - 1.5).abs() < 1e-15);
        assert!((pc.bound().y.lo - (-1.5)).abs() < 1e-15);
        assert!((pc.bound().y.hi - 1.5).abs() < 1e-15);
    }

    #[test]
    fn test_padded_cell_center() {
        let id = CellId::from_face(0);
        let pc = PaddedCell::from_cell_id(id, 0.0);
        let center = pc.center();
        // Face 0 center is (1, 0, 0).
        assert!(center.0.x > 0.99);
        assert!(center.0.y.abs() < 0.01);
        assert!(center.0.z.abs() < 0.01);
    }

    #[test]
    fn test_padded_cell_middle() {
        let id = CellId::from_face(0);
        let mut pc = PaddedCell::from_cell_id(id, 0.25);
        let mid = pc.middle();
        // For a face cell with padding 0.25, middle should be [-0.25, 0.25]^2.
        assert!((mid.x.lo - (-0.25)).abs() < 1e-15);
        assert!((mid.x.hi - 0.25).abs() < 1e-15);
    }

    #[test]
    fn test_padded_cell_from_parent() {
        let id = CellId::from_face(0);
        let mut parent = PaddedCell::from_cell_id(id, 0.0);
        let child = PaddedCell::from_parent_ij(&mut parent, 0, 0);
        assert_eq!(child.level(), 1);
        // Child bound should be a subset of parent bound.
        assert!(child.bound().x.lo >= parent.bound().x.lo - 1e-15);
        assert!(child.bound().x.hi <= parent.bound().x.hi + 1e-15);
    }

    #[test]
    fn test_padded_cell_child_ij() {
        let id = CellId::from_face(0);
        let pc = PaddedCell::from_cell_id(id, 0.0);
        // All 4 positions should give valid (i,j) in {0,1}.
        for pos in 0..4 {
            let (i, j) = pc.child_ij(pos);
            assert!(i == 0 || i == 1, "pos {pos}: i = {i}");
            assert!(j == 0 || j == 1, "pos {pos}: j = {j}");
        }
    }

    #[test]
    fn test_padded_cell_entry_exit_vertex() {
        let id = CellId::from_face(0);
        let pc = PaddedCell::from_cell_id(id, 0.0);
        let entry = pc.entry_vertex();
        let exit = pc.exit_vertex();
        // Entry and exit should be valid unit-length points.
        assert!(entry.is_unit());
        assert!(exit.is_unit());
        // They should not be the same.
        assert_ne!(entry, exit);
    }

    #[test]
    fn test_padded_cell_shrink_to_fit_full_rect() {
        let id = CellId::from_face(0);
        let mut pc = PaddedCell::from_cell_id(id, 0.0);
        // A rect that contains the center should return the cell itself.
        let rect = r2::Rect::new(r1::Interval::new(-0.5, 0.5), r1::Interval::new(-0.5, 0.5));
        assert_eq!(pc.shrink_to_fit(rect), id);
    }

    #[test]
    fn test_padded_cell_shrink_to_fit_corner() {
        let id = CellId::from_face(0);
        let mut pc = PaddedCell::from_cell_id(id, 0.0);
        // A rect in the corner should shrink to a child.
        let rect = r2::Rect::new(r1::Interval::new(0.5, 0.9), r1::Interval::new(0.5, 0.9));
        let shrunk = pc.shrink_to_fit(rect);
        assert!(shrunk.level() > 0, "should shrink to a deeper level");
        assert!(shrunk.level() <= MAX_CELL_LEVEL);
    }
}
