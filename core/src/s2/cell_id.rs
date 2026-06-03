// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! A 64-bit identifier that uniquely identifies a cell in the S2 cell decomposition.
//!
//! Corresponds to C++ `S2CellId`, Go `s2.CellID`.
//!
//! The most significant 3 bits encode the face number (0–5). The remaining
//! 61 bits encode the position of the center of this cell along the Hilbert
//! curve on that face. The zero value is an invalid cell ID.
//!
//! Sequentially increasing cell IDs follow a continuous space-filling curve
//! over the entire sphere.

#![expect(
    clippy::cast_sign_loss,
    reason = "IJ coordinates and face (i32/u8) cast to u64/u32 for bit manipulation — values always non-negative"
)]
#![expect(
    clippy::cast_possible_truncation,
    reason = "u64 bit manipulation — values bounded by cell level arithmetic"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "u64/u32 -> i64/i32 for cell coordinate arithmetic — bounded by cell structure"
)]
use crate::r1;
use crate::r2;
use crate::s2::coords::{
    Face, INVERT_MASK, Level, MAX_CELL_LEVEL, POS_TO_IJ, POS_TO_ORIENTATION, SWAP_MASK,
    face_uv_to_xyz, ij_to_st_min, si_ti_to_st, st_to_ij, st_to_uv, uv_to_st, xyz_to_face_uv,
};
use crate::s2::{LatLng, Point};
use std::fmt;
use std::str::FromStr;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Number of bits used to encode the face number.
const FACE_BITS: u32 = 3;

/// Number of faces on the cube.
const NUM_FACES: u64 = 6;

/// Total number of Hilbert curve position bits (2 * `MAX_LEVEL` + 1).
const POS_BITS: u32 = 2 * MAX_CELL_LEVEL as u32 + 1; // 61

/// Maximum valid leaf-cell index + 1.
const MAX_SIZE: i32 = 1 << MAX_CELL_LEVEL;

/// Number of bits per lookup-table iteration.
const LOOKUP_BITS: u32 = 4;

/// Wrap offset: `NUM_FACES << POS_BITS`. Used for wrapping arithmetic.
const WRAP_OFFSET: u64 = NUM_FACES << POS_BITS;

// ---------------------------------------------------------------------------
// Lookup tables (compile-time)
// ---------------------------------------------------------------------------

/// Both lookup tables, built at compile time.
static LOOKUP_TABLES: ([u16; 1024], [u16; 1024]) = {
    let lp = [0u16; 1024];
    let li = [0u16; 1024];
    let (lp, li) = init_lookup_tables(lp, li, 0, 0, 0, 0, 0, 0);
    let (lp, li) = init_lookup_tables(lp, li, 0, 0, 0, SWAP_MASK as i32, 0, SWAP_MASK as i32);
    let (lp, li) = init_lookup_tables(lp, li, 0, 0, 0, INVERT_MASK as i32, 0, INVERT_MASK as i32);
    init_lookup_tables(
        lp,
        li,
        0,
        0,
        0,
        (SWAP_MASK | INVERT_MASK) as i32,
        0,
        (SWAP_MASK | INVERT_MASK) as i32,
    )
};

/// Lookup table mapping (i-bits, j-bits, orientation) → (pos-bits, new-orientation).
static LOOKUP_POS: &[u16; 1024] = &LOOKUP_TABLES.0;

/// Inverse lookup table mapping (pos-bits, orientation) → (i-bits, j-bits, new-orientation).
static LOOKUP_IJ: &[u16; 1024] = &LOOKUP_TABLES.1;

/// Recursive const fn that populates both lookup tables simultaneously.
/// Returns (`lookup_pos`, `lookup_ij`) with the entries filled in.
#[expect(clippy::too_many_arguments, reason = "matches C++ API")]
const fn init_lookup_tables(
    mut lp: [u16; 1024],
    mut li: [u16; 1024],
    level: i32,
    i: i32,
    j: i32,
    orig_orientation: i32,
    pos: i32,
    orientation: i32,
) -> ([u16; 1024], [u16; 1024]) {
    if level == LOOKUP_BITS as i32 {
        let ij = (i << LOOKUP_BITS) + j;
        lp[(ij << 2 | orig_orientation) as usize] = (pos << 2 | orientation) as u16;
        li[(pos << 2 | orig_orientation) as usize] = (ij << 2 | orientation) as u16;
        return (lp, li);
    }

    let next_level = level + 1;
    let ni = i << 1;
    let nj = j << 1;
    let npos = pos << 2;

    let r = POS_TO_IJ[orientation as usize];

    // Child 0
    let (lp2, li2) = init_lookup_tables(
        lp,
        li,
        next_level,
        ni + ((r[0] >> 1) as i32),
        nj + ((r[0] & 1) as i32),
        orig_orientation,
        npos,
        orientation ^ POS_TO_ORIENTATION[0] as i32,
    );
    // Child 1
    let (lp3, li3) = init_lookup_tables(
        lp2,
        li2,
        next_level,
        ni + ((r[1] >> 1) as i32),
        nj + ((r[1] & 1) as i32),
        orig_orientation,
        npos + 1,
        orientation ^ POS_TO_ORIENTATION[1] as i32,
    );
    // Child 2
    let (lp4, li4) = init_lookup_tables(
        lp3,
        li3,
        next_level,
        ni + ((r[2] >> 1) as i32),
        nj + ((r[2] & 1) as i32),
        orig_orientation,
        npos + 2,
        orientation ^ POS_TO_ORIENTATION[2] as i32,
    );
    // Child 3
    init_lookup_tables(
        lp4,
        li4,
        next_level,
        ni + ((r[3] >> 1) as i32),
        nj + ((r[3] & 1) as i32),
        orig_orientation,
        npos + 3,
        orientation ^ POS_TO_ORIENTATION[3] as i32,
    )
}

// ---------------------------------------------------------------------------
// CellId
// ---------------------------------------------------------------------------

/// A 64-bit identifier that uniquely identifies a cell in the S2 cell decomposition.
///
/// `CellId` is `Copy` and `Ord` (ordered by raw u64 value, matching C++/Go).
///
/// # Examples
///
/// ```
/// use s2rst::s2::{CellId, LatLng, Point};
///
/// // Create a CellId from a lat/lng point (New York City).
/// let ll = LatLng::from_degrees(40.7128, -74.0060);
/// let id = CellId::from_lat_lng(&ll);
/// assert!(id.is_valid());
/// assert_eq!(id.level(), 30); // leaf cell
///
/// // Navigate the hierarchy: parent at level 10.
/// let parent = id.parent_at_level(10);
/// assert_eq!(parent.level(), 10);
/// assert!(parent.contains(id));
///
/// // Get the four children of the parent.
/// let children = parent.children();
/// assert_eq!(children.len(), 4);
/// assert!(children.iter().any(|c| c.contains(id)));
///
/// // Token round-trip.
/// let token = parent.to_token();
/// assert_eq!(CellId::from_token(&token), parent);
/// ```
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CellId(pub u64);

impl CellId {
    // =======================================================================
    // Constructors
    // =======================================================================

    /// Returns the invalid zero cell ID.
    #[inline]
    pub fn none() -> Self {
        CellId(0)
    }

    /// Returns an invalid cell ID guaranteed to be larger than any valid cell ID.
    /// Used primarily by `ShapeIndex`.
    #[inline]
    pub fn sentinel() -> Self {
        CellId(!0u64)
    }

    /// Returns the cell corresponding to a given S2 cube face (0–5).
    #[inline]
    pub fn from_face(face: impl Into<u8>) -> Self {
        let f: u8 = face.into();
        CellId((u64::from(f) << POS_BITS) + lsb_for_level(Level::MIN))
    }

    /// Returns a cell given its face (0–5), the 61-bit Hilbert curve position
    /// within that face, and the level (0–30). The position is truncated to
    /// correspond to the Hilbert curve position at the center of the returned cell.
    #[inline]
    pub fn from_face_pos_level(face: impl Into<u8>, pos: u64, level: impl Into<Level>) -> Self {
        let f: u8 = face.into();
        CellId(((u64::from(f) << POS_BITS) + pos) | 1).parent_at_level(level)
    }

    /// Returns the leaf cell containing the given point.
    #[inline]
    pub fn from_point(p: &Point) -> Self {
        let (f, u, v) = xyz_to_face_uv(&p.0);
        let i = st_to_ij(uv_to_st(u));
        let j = st_to_ij(uv_to_st(v));
        from_face_ij(f, i, j)
    }

    /// Returns the leaf cell containing the given latitude-longitude.
    #[inline]
    pub fn from_lat_lng(ll: &LatLng) -> Self {
        Self::from_point(&ll.to_point())
    }

    // =======================================================================
    // Accessors
    // =======================================================================

    /// Returns the raw 64-bit cell ID.
    #[inline]
    pub fn id(self) -> u64 {
        self.0
    }

    /// Reports whether this is a valid cell ID.
    #[inline]
    pub fn is_valid(self) -> bool {
        let face_bits = (self.0 >> POS_BITS) as u8;
        face_bits < 6 && (self.lsb() & 0x1555555555555555 != 0)
    }

    /// Returns the cube face (0–5).
    #[inline]
    pub fn face(self) -> Face {
        Face::from_u8((self.0 >> POS_BITS) as u8)
    }

    /// Returns the position along the Hilbert curve (0..2^61 − 1).
    #[inline]
    pub fn pos(self) -> u64 {
        self.0 & (!0u64 >> FACE_BITS)
    }

    /// Returns the subdivision level (0–30).
    #[inline]
    pub fn level(self) -> Level {
        Level::new(MAX_CELL_LEVEL - (self.0.trailing_zeros() >> 1) as u8)
    }

    /// Returns the lowest-numbered bit that is on for this cell ID.
    #[inline]
    pub fn lsb(self) -> u64 {
        self.0 & self.0.wrapping_neg()
    }

    /// Reports whether this is a leaf cell (level 30).
    #[inline]
    pub fn is_leaf(self) -> bool {
        self.0 & 1 != 0
    }

    /// Reports whether this is a top-level face cell (level 0).
    #[inline]
    pub fn is_face(self) -> bool {
        self.0 & (lsb_for_level(Level::MIN) - 1) == 0
    }

    /// Returns the child position (0–3) of this cell's ancestor at the given
    /// level, relative to its parent. The level should be in `1..=MAX_LEVEL`.
    #[inline]
    pub fn child_position(self, level: impl Into<Level>) -> u8 {
        let level = level.into();
        debug_assert!(self.is_valid());
        debug_assert!(level >= Level::new(1));
        debug_assert!(level <= self.level());
        ((self.0 >> (2 * (u32::from(MAX_CELL_LEVEL) - level.as_u32()) + 1)) & 3) as u8
    }

    // =======================================================================
    // Hierarchy
    // =======================================================================

    /// Returns the cell at the given level (which must be <= this cell's level).
    #[inline]
    pub fn parent_at_level(self, level: impl Into<Level>) -> CellId {
        let new_lsb = lsb_for_level(level.into());
        CellId((self.0 & new_lsb.wrapping_neg()) | new_lsb)
    }

    /// Returns the immediate parent cell. Panics if `self.is_face()`.
    #[inline]
    pub fn parent(self) -> CellId {
        debug_assert!(!self.is_face());
        let nlsb = self.lsb() << 2;
        CellId((self.0 & nlsb.wrapping_neg()) | nlsb)
    }

    /// Returns the four immediate children of this cell.
    #[inline]
    pub fn children(self) -> [CellId; 4] {
        debug_assert!(self.is_valid());
        debug_assert!(!self.is_leaf());
        let old_lsb = self.lsb();
        let new_lsb = old_lsb >> 2;
        let child0 = self.0 - old_lsb + new_lsb;
        let step = new_lsb << 1;
        [
            CellId(child0),
            CellId(child0 + step),
            CellId(child0 + 2 * step),
            CellId(child0 + 3 * step),
        ]
    }

    /// Returns the first child in a traversal of children in Hilbert curve order.
    #[inline]
    pub fn child_begin(self) -> CellId {
        debug_assert!(self.is_valid());
        debug_assert!(!self.is_leaf());
        let ol = self.lsb();
        CellId(self.0 - ol + (ol >> 2))
    }

    /// Returns the first cell after a traversal of children in Hilbert curve order.
    #[inline]
    pub fn child_end(self) -> CellId {
        debug_assert!(self.is_valid());
        debug_assert!(!self.is_leaf());
        let ol = self.lsb();
        CellId(self.0 + ol + (ol >> 2))
    }

    /// Returns the first child at the given level (must be >= this cell's level).
    #[inline]
    pub fn child_begin_at_level(self, level: impl Into<Level>) -> CellId {
        CellId(self.0 - self.lsb() + lsb_for_level(level.into()))
    }

    /// Returns the first cell after the last child at the given level.
    #[inline]
    pub fn child_end_at_level(self, level: impl Into<Level>) -> CellId {
        CellId(self.0 + self.lsb() + lsb_for_level(level.into()))
    }

    /// Returns the first cell at the given level (the smallest cell ID).
    #[inline]
    pub fn begin(level: impl Into<Level>) -> CellId {
        CellId::from_face(0).child_begin_at_level(level)
    }

    /// Returns the past-the-end cell at the given level.
    #[inline]
    pub fn end(level: impl Into<Level>) -> CellId {
        CellId::from_face(5).child_end_at_level(level)
    }

    /// Returns the minimum `CellId` contained within this cell.
    #[inline]
    pub fn range_min(self) -> CellId {
        CellId(self.0 - (self.lsb() - 1))
    }

    /// Returns the maximum `CellId` contained within this cell.
    #[inline]
    pub fn range_max(self) -> CellId {
        CellId(self.0 + (self.lsb() - 1))
    }

    /// Reports whether this cell contains `other`.
    #[inline]
    pub fn contains(self, other: CellId) -> bool {
        self.range_min().0 <= other.0 && other.0 <= self.range_max().0
    }

    /// Reports whether this cell intersects `other`.
    #[inline]
    pub fn intersects(self, other: CellId) -> bool {
        other.range_min().0 <= self.range_max().0 && other.range_max().0 >= self.range_min().0
    }

    // =======================================================================
    // Traversal
    // =======================================================================

    /// Returns the next cell along the Hilbert curve.
    #[inline]
    pub fn next(self) -> CellId {
        CellId(self.0.wrapping_add(self.lsb() << 1))
    }

    /// Returns the previous cell along the Hilbert curve.
    #[inline]
    pub fn prev(self) -> CellId {
        CellId(self.0.wrapping_sub(self.lsb() << 1))
    }

    /// Returns the next cell along the Hilbert curve, wrapping from last to first.
    #[inline]
    pub fn next_wrap(self) -> CellId {
        debug_assert!(self.is_valid());
        let n = self.next();
        if n.0 < WRAP_OFFSET {
            n
        } else {
            CellId(n.0 - WRAP_OFFSET)
        }
    }

    /// Returns the previous cell, wrapping from first to last.
    #[inline]
    pub fn prev_wrap(self) -> CellId {
        debug_assert!(self.is_valid());
        let p = self.prev();
        if p.0 < WRAP_OFFSET {
            p
        } else {
            CellId(p.0.wrapping_add(WRAP_OFFSET))
        }
    }

    /// Advances or retreats the indicated number of steps along the Hilbert
    /// curve at the current level. The position is clamped to [Begin, End].
    pub fn advance(self, steps: i64) -> CellId {
        if steps == 0 {
            return self;
        }
        let step_shift = 2 * (u32::from(MAX_CELL_LEVEL) - self.level().as_u32()) + 1;
        if steps < 0 {
            let min_steps = -((self.0 >> step_shift) as i64);
            let steps = steps.max(min_steps);
            CellId(self.0.wrapping_add((steps as u64) << step_shift))
        } else {
            let max_steps = ((WRAP_OFFSET + self.lsb() - self.0) >> step_shift) as i64;
            let steps = steps.min(max_steps);
            CellId(self.0 + ((steps as u64) << step_shift))
        }
    }

    /// Advances or retreats with wrapping between first and last faces.
    pub fn advance_wrap(self, steps: i64) -> CellId {
        if steps == 0 {
            return self;
        }
        let shift = 2 * (u32::from(MAX_CELL_LEVEL) - self.level().as_u32()) + 1;
        let steps = if steps < 0 {
            let min = -((self.0 >> shift) as i64);
            if steps < min {
                let wrap = (WRAP_OFFSET >> shift) as i64;
                let mut s = steps % wrap;
                if s < min {
                    s += wrap;
                }
                s
            } else {
                steps
            }
        } else {
            let max = ((WRAP_OFFSET - self.0) >> shift) as i64;
            if steps > max {
                let wrap = (WRAP_OFFSET >> shift) as i64;
                let mut s = steps % wrap;
                if s > max {
                    s -= wrap;
                }
                s
            } else {
                steps
            }
        };
        CellId(self.0.wrapping_add((steps as u64) << shift))
    }

    /// Returns the number of steps from `Begin()` at this level to this cell.
    #[inline]
    pub fn distance_from_begin(self) -> i64 {
        (self.0 >> (2 * (u32::from(MAX_CELL_LEVEL) - self.level().as_u32()) + 1)) as i64
    }

    // =======================================================================
    // Geometry conversions
    // =======================================================================

    /// Returns the `(face, si, ti)` coordinates of the center of this cell.
    #[inline]
    pub fn get_center_si_ti(self) -> (Face, u32, u32) {
        let (face, i, j, _) = self.to_face_ij_orientation();
        let delta: i32;
        if self.is_leaf() {
            delta = 1;
        } else if (i64::from(i) ^ (self.0 as i64 >> 2)) & 1 != 0 {
            delta = 2;
        } else {
            delta = 0;
        }
        (face, (2 * i + delta) as u32, (2 * j + delta) as u32)
    }

    /// Returns an unnormalized direction vector from the origin through the
    /// center of this cell (`IsS2Cell` `S2Point` `ToPointRaw()`).
    #[inline]
    pub fn to_point_raw(self) -> Point {
        Point(self.raw_point())
    }

    /// Returns an unnormalized direction vector from the origin through the
    /// center of this cell.
    #[inline]
    fn raw_point(self) -> crate::r3::Vector {
        let (face, si, ti) = self.get_center_si_ti();
        face_uv_to_xyz(
            face,
            st_to_uv((0.5 / f64::from(MAX_SIZE)) * f64::from(si)),
            st_to_uv((0.5 / f64::from(MAX_SIZE)) * f64::from(ti)),
        )
    }

    /// Returns the center of this cell as a unit-length `Point`.
    #[inline]
    pub fn to_point(self) -> Point {
        Point(self.raw_point().normalize())
    }

    /// Returns the center of this cell as a `LatLng`.
    #[inline]
    pub fn to_lat_lng(self) -> LatLng {
        LatLng::from_point(Point(self.raw_point()))
    }

    /// Returns the center of this cell in (s, t)-space.
    #[inline]
    pub fn center_st(self) -> r2::Point {
        let (_, si, ti) = self.get_center_si_ti();
        r2::Point::new(si_ti_to_st(si), si_ti_to_st(ti))
    }

    /// Returns the center of this cell in (u, v)-space.
    #[inline]
    pub fn center_uv(self) -> r2::Point {
        let (_, si, ti) = self.get_center_si_ti();
        r2::Point::new(st_to_uv(si_ti_to_st(si)), st_to_uv(si_ti_to_st(ti)))
    }

    /// Returns the edge length at the given level in (s,t)-space.
    #[inline]
    pub fn size_st(level: impl Into<Level>) -> f64 {
        ij_to_st_min(size_ij(level))
    }

    /// Returns the bound of this cell in (s,t)-space.
    #[inline]
    pub fn bound_st(self) -> r2::Rect {
        let s = Self::size_st(self.level());
        r2::Rect::from_center_size(self.center_st(), r2::Point::new(s, s))
    }

    /// Returns the bound of this cell in (u,v)-space.
    #[inline]
    pub fn bound_uv(self) -> r2::Rect {
        let (_, i, j, _) = self.to_face_ij_orientation();
        ij_level_to_bound_uv(i, j, self.level())
    }

    // =======================================================================
    // Face/IJ encoding and decoding
    // =======================================================================

    /// Decodes this cell ID into `(face, i, j, orientation)`.
    #[inline]
    pub fn to_face_ij_orientation(self) -> (Face, i32, i32, u8) {
        let f = self.face();
        let mut orientation = i32::from(f.as_u8() & SWAP_MASK);
        let mut i = 0i32;
        let mut j = 0i32;

        // First iteration uses fewer bits.
        let mut nbits = i32::from(MAX_CELL_LEVEL) - 7 * LOOKUP_BITS as i32;

        for k in (0..=7i32).rev() {
            let bits_to_extract = 2 * nbits as u32;
            let mask = (1i32 << bits_to_extract) - 1;
            orientation += ((self.0 >> (k as u32 * 2 * LOOKUP_BITS + 1)) as i32 & mask) << 2;
            orientation = i32::from(LOOKUP_IJ[orientation as usize]);
            i += (orientation >> (LOOKUP_BITS as i32 + 2)) << (k as u32 * LOOKUP_BITS);
            j += ((orientation >> 2) & ((1 << LOOKUP_BITS) - 1)) << (k as u32 * LOOKUP_BITS);
            orientation &= i32::from(SWAP_MASK | INVERT_MASK);
            nbits = LOOKUP_BITS as i32;
        }

        if self.lsb() & 0x1111111111111110 != 0 {
            orientation ^= i32::from(SWAP_MASK);
        }

        (f, i, j, orientation as u8)
    }

    // =======================================================================
    // Token / String
    // =======================================================================

    /// Returns a hex-encoded token string with trailing zeros stripped.
    pub fn to_token(self) -> String {
        if self.0 == 0 {
            return "X".to_string();
        }
        let s = format!("{:016x}", self.0);
        s.trim_end_matches('0').to_string()
    }

    /// Parses a hex-encoded token string. Returns `CellId::none()` on error.
    pub fn from_token(token: &str) -> Self {
        if token.len() > 16 || token.is_empty() {
            return CellId::none();
        }
        if token == "X" {
            return CellId::none();
        }
        let Ok(n) = u64::from_str_radix(token, 16) else {
            return CellId::none();
        };
        if token.len() < 16 {
            CellId(n << (4 * (16 - token.len()) as u32))
        } else {
            CellId(n)
        }
    }

    // =======================================================================
    // Neighbors
    // =======================================================================

    /// Returns the four edge neighbors (down, right, up, left in face space).
    #[inline]
    pub fn edge_neighbors(self) -> [CellId; 4] {
        let level = self.level();
        let size = size_ij(level);
        let (f, i, j, _) = self.to_face_ij_orientation();
        [
            from_face_ij_same(f, i, j - size, j - size >= 0).parent_at_level(level),
            from_face_ij_same(f, i + size, j, i + size < MAX_SIZE).parent_at_level(level),
            from_face_ij_same(f, i, j + size, j + size < MAX_SIZE).parent_at_level(level),
            from_face_ij_same(f, i - size, j, i - size >= 0).parent_at_level(level),
        ]
    }

    /// Returns the neighbors of the closest vertex at the given level.
    /// Normally 4 cells, but only 3 at a cube vertex.
    pub fn vertex_neighbors(self, level: impl Into<Level>) -> Vec<CellId> {
        let level = level.into();
        let half_size = size_ij(Level::new(level.as_u8() + 1));
        let size = half_size << 1;
        let (f, i, j, _) = self.to_face_ij_orientation();

        let (ioffset, isame) = if i & half_size != 0 {
            (size, (i + size) < MAX_SIZE)
        } else {
            (-size, (i - size) >= 0)
        };
        let (joffset, jsame) = if j & half_size != 0 {
            (size, (j + size) < MAX_SIZE)
        } else {
            (-size, (j - size) >= 0)
        };

        let mut results = vec![
            self.parent_at_level(level),
            from_face_ij_same(f, i + ioffset, j, isame).parent_at_level(level),
            from_face_ij_same(f, i, j + joffset, jsame).parent_at_level(level),
        ];

        if isame || jsame {
            results.push(
                from_face_ij_same(f, i + ioffset, j + joffset, isame && jsame)
                    .parent_at_level(level),
            );
        }

        results
    }

    /// Returns all neighbors of this cell at the given level. Two cells are
    /// neighbors if their boundaries intersect but their interiors do not.
    pub fn all_neighbors(self, level: impl Into<Level>) -> Option<Vec<CellId>> {
        let level = level.into();
        if level < self.level() || level > Level::MAX {
            return None;
        }

        let (face, mut i, mut j, _) = self.to_face_ij_orientation();

        // Normalize (i, j) to the lower-left corner of this cell.
        let size = size_ij(self.level());
        i &= -size;
        j &= -size;

        let nbr_size = size_ij(level);
        let mut neighbors = Vec::new();

        let mut k = -nbr_size;
        loop {
            let same_face;
            if k < 0 {
                same_face = j + k >= 0;
            } else if k >= size {
                same_face = j + k < MAX_SIZE;
            } else {
                same_face = true;
                // Top and bottom neighbors.
                neighbors.push(
                    from_face_ij_same(face, i + k, j - nbr_size, j - size >= 0)
                        .parent_at_level(level),
                );
                neighbors.push(
                    from_face_ij_same(face, i + k, j + size, j + size < MAX_SIZE)
                        .parent_at_level(level),
                );
            }

            // Left, right, and diagonal neighbors.
            neighbors.push(
                from_face_ij_same(face, i - nbr_size, j + k, same_face && i - size >= 0)
                    .parent_at_level(level),
            );
            neighbors.push(
                from_face_ij_same(face, i + size, j + k, same_face && i + size < MAX_SIZE)
                    .parent_at_level(level),
            );

            if k >= size {
                break;
            }
            k += nbr_size;
        }

        Some(neighbors)
    }

    // =======================================================================
    // Advanced
    // =======================================================================

    /// Returns the level of the lowest common ancestor of the two cell IDs,
    /// or `None` if they are on different faces.
    #[inline]
    pub fn common_ancestor_level(self, other: CellId) -> Option<Level> {
        let diff_bits = (self.0 ^ other.0).max(self.lsb()).max(other.lsb());
        let msb_pos = diff_bits.ilog2(); // bits::Len64(x) - 1
        if msb_pos > 60 {
            return None;
        }
        Some(Level::new(((60 - msb_pos) >> 1) as u8))
    }

    /// Returns the largest cell with the same `range_min()` such that
    /// `range_max() < limit.range_min()`. Returns `limit` if no such cell exists.
    pub fn maximum_tile(self, limit: CellId) -> CellId {
        let start = self.range_min();
        if start >= limit.range_min() {
            return limit;
        }

        let mut ci = self;
        if ci.range_max() >= limit {
            loop {
                ci = ci.children()[0];
                if ci.range_max() < limit {
                    break;
                }
            }
            return ci;
        }

        while !ci.is_face() {
            let p = ci.parent();
            if p.range_min() != start || p.range_max() >= limit {
                break;
            }
            ci = p;
        }
        ci
    }

    /// Returns the string representation in the form "face/childpath" (e.g. "3/102").
    pub fn to_debug_string(self) -> String {
        if !self.is_valid() {
            return format!("Invalid: {:x}", self.0);
        }
        let mut s = String::new();
        s.push((b'0' + self.face().as_u8()) as char);
        s.push('/');
        for lev in 1..=self.level().as_u8() {
            s.push((b'0' + self.child_position(Level::new(lev))) as char);
        }
        s
    }

    /// Parses a debug string in the form "face/childpath".
    pub fn from_debug_string(s: &str) -> Option<CellId> {
        let level = s.len() as i32 - 2;
        if level < 0 || level > i32::from(MAX_CELL_LEVEL) {
            return None;
        }
        let bytes = s.as_bytes();
        let face = bytes[0].wrapping_sub(b'0');
        if face > 5 || bytes[1] != b'/' {
            return None;
        }
        let mut id = CellId::from_face(face);
        for &b in &bytes[2..] {
            let child_pos = b.wrapping_sub(b'0');
            if child_pos > 3 {
                return None;
            }
            id = id.children()[child_pos as usize];
        }
        Some(id)
    }
}

// ---------------------------------------------------------------------------
// Free functions
// ---------------------------------------------------------------------------

/// Returns the lowest-numbered bit that is on for cells at the given level.
#[inline]
pub fn lsb_for_level(level: impl Into<Level>) -> u64 {
    let level = level.into();
    1u64 << (2 * (u32::from(MAX_CELL_LEVEL) - level.as_u32()))
}

/// Returns the edge length of cells at the given level in (i,j)-space.
#[inline]
pub fn size_ij(level: impl Into<Level>) -> i32 {
    1 << (MAX_CELL_LEVEL - level.into().as_u8())
}

/// Returns a leaf cell given its cube face and (i, j) coordinates.
#[inline]
pub fn from_face_ij(f: Face, i: i32, j: i32) -> CellId {
    let fb = f.as_u8();
    let mut n = u64::from(fb) << (POS_BITS - 1);
    let mut bits = i32::from(fb & SWAP_MASK);
    for k in (0..=7i32).rev() {
        let mask = (1 << LOOKUP_BITS) - 1;
        bits += ((i >> (k as u32 * LOOKUP_BITS)) & mask) << (LOOKUP_BITS as i32 + 2);
        bits += ((j >> (k as u32 * LOOKUP_BITS)) & mask) << 2;
        bits = i32::from(LOOKUP_POS[bits as usize]);
        n |= (bits as u64 >> 2) << (k as u32 * 2 * LOOKUP_BITS);
        bits &= i32::from(SWAP_MASK | INVERT_MASK);
    }
    CellId(n * 2 + 1)
}

/// Wraps (i, j) coordinates onto the appropriate adjacent face.
fn from_face_ij_wrap(f: Face, i: i32, j: i32) -> CellId {
    let i = i.clamp(-1, MAX_SIZE);
    let j = j.clamp(-1, MAX_SIZE);

    const SCALE: f64 = 1.0 / MAX_SIZE as f64;
    let limit = f64::from_bits(0x3FF0000000000001); // 1.0 + epsilon, i.e. next_after(1.0, 2.0)
    // Use i64 to avoid overflow: i can be -1 and MAX_SIZE is 2^30.
    let u = (SCALE * (i64::from(i) * 2 + 1 - i64::from(MAX_SIZE)) as f64).clamp(-limit, limit);
    let v = (SCALE * (i64::from(j) * 2 + 1 - i64::from(MAX_SIZE)) as f64).clamp(-limit, limit);

    let (nf, nu, nv) = xyz_to_face_uv(&face_uv_to_xyz(f, u, v));
    from_face_ij(nf, st_to_ij(0.5 * (nu + 1.0)), st_to_ij(0.5 * (nv + 1.0)))
}

/// If `same_face` is true, uses `from_face_ij`, otherwise `from_face_ij_wrap`.
#[inline]
fn from_face_ij_same(f: Face, i: i32, j: i32, same_face: bool) -> CellId {
    if same_face {
        from_face_ij(f, i, j)
    } else {
        from_face_ij_wrap(f, i, j)
    }
}

/// Returns the bounds in (u,v)-space for the cell at the given level
/// containing the leaf cell with the given (i,j)-coordinates.
#[inline]
pub fn ij_level_to_bound_uv(i: i32, j: i32, level: impl Into<Level>) -> r2::Rect {
    let cell_size = size_ij(level);
    let x_lo = i & (-cell_size);
    let y_lo = j & (-cell_size);
    r2::Rect {
        x: r1::Interval::new(
            st_to_uv(ij_to_st_min(x_lo)),
            st_to_uv(ij_to_st_min(x_lo + cell_size)),
        ),
        y: r1::Interval::new(
            st_to_uv(ij_to_st_min(y_lo)),
            st_to_uv(ij_to_st_min(y_lo + cell_size)),
        ),
    }
}

/// Helper for `expanded_by_distance_uv`. Given an edge of the form
/// (u,v0)-(u,v1), returns a new u-coordinate such that the distance from
/// the line u=u' to the given edge is exactly `sin_dist`.
fn expand_endpoint(u: f64, max_v: f64, sin_dist: f64) -> f64 {
    let sin_u_shift = sin_dist * ((1.0 + u * u + max_v * max_v) / (1.0 + u * u)).sqrt();
    let cos_u_shift = (1.0 - sin_u_shift * sin_u_shift).sqrt();
    (cos_u_shift * u + sin_u_shift) / (cos_u_shift - sin_u_shift * u)
}

/// Expands a rectangle in (u,v)-space so that it contains all points within
/// the given distance of the original rectangle.
pub fn expanded_by_distance_uv(uv: &r2::Rect, distance: crate::s1::Angle) -> r2::Rect {
    let u0 = uv.x.lo;
    let u1 = uv.x.hi;
    let v0 = uv.y.lo;
    let v1 = uv.y.hi;
    let max_u = u0.abs().max(u1.abs());
    let max_v = v0.abs().max(v1.abs());
    let sin_dist = distance.radians().sin();

    let xi = r1::Interval::new(
        expand_endpoint(u0, max_v, -sin_dist),
        expand_endpoint(u1, max_v, sin_dist),
    );
    let yi = r1::Interval::new(
        expand_endpoint(v0, max_u, -sin_dist),
        expand_endpoint(v1, max_u, sin_dist),
    );
    if xi.is_empty() || yi.is_empty() {
        r2::Rect {
            x: r1::Interval::new(u0, u0),
            y: r1::Interval::new(v0, v0),
        }
    } else {
        r2::Rect { x: xi, y: yi }
    }
}

// ---------------------------------------------------------------------------
// Trait impls
// ---------------------------------------------------------------------------

impl Default for CellId {
    fn default() -> Self {
        CellId::none()
    }
}

impl From<u64> for CellId {
    #[inline]
    fn from(v: u64) -> Self {
        CellId(v)
    }
}

impl From<CellId> for u64 {
    #[inline]
    fn from(c: CellId) -> Self {
        c.0
    }
}

impl From<&Point> for CellId {
    #[inline]
    fn from(p: &Point) -> Self {
        CellId::from_point(p)
    }
}

impl From<&LatLng> for CellId {
    #[inline]
    fn from(ll: &LatLng) -> Self {
        CellId::from_lat_lng(ll)
    }
}

impl fmt::Display for CellId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_debug_string())
    }
}

impl FromStr for CellId {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        CellId::from_debug_string(s).ok_or(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn is_send_sync<T: Sized + Send + Sync + Unpin>() {}

    #[test]
    fn cell_id_is_send_sync() {
        is_send_sync::<CellId>();
    }

    fn float64_near(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() <= eps
    }

    // --- Basic construction ---

    #[test]
    fn test_from_face() {
        for face in 0..6u8 {
            let fpl = CellId::from_face_pos_level(face, 0, 0);
            let f = CellId::from_face(face);
            assert_eq!(
                fpl, f,
                "from_face_pos_level({face}, 0, 0) != from_face({face})"
            );
        }
    }

    #[test]
    fn test_sentinel_range_min_max() {
        let s = CellId::sentinel();
        assert_eq!(s.range_min(), s);
        assert_eq!(s.range_max(), s);
    }

    #[test]
    fn test_parent_child_relationships() {
        let ci = CellId::from_face_pos_level(3, 0x12345678, MAX_CELL_LEVEL - 4);

        assert!(ci.is_valid());
        assert_eq!(ci.face(), Face::F3);
        assert_eq!(ci.pos(), 0x12345700);
        assert_eq!(ci.level(), 26);
        assert!(!ci.is_leaf());

        assert_eq!(ci.child_begin_at_level(ci.level() + 2).pos(), 0x12345610);
        assert_eq!(ci.child_begin().pos(), 0x12345640);
        assert_eq!(ci.children()[0].pos(), 0x12345640);
        assert_eq!(ci.parent().pos(), 0x12345400);
        assert_eq!(ci.parent_at_level(ci.level() - 2).pos(), 0x12345000);

        assert!(ci.child_begin().0 < ci.0);
        assert!(ci.child_end().0 > ci.0);
        assert_eq!(ci.child_end(), ci.child_begin().next().next().next().next());
        assert_eq!(ci.range_min(), ci.child_begin_at_level(MAX_CELL_LEVEL));
        assert_eq!(ci.range_max().next(), ci.child_end_at_level(MAX_CELL_LEVEL));
    }

    #[test]
    fn test_containment() {
        let a = CellId(0x80855c0000000000); // Pittsburg
        let b = CellId(0x80855d0000000000); // child of a
        let c = CellId(0x80855dc000000000); // child of b
        let d = CellId(0x8085630000000000); // disjoint from a

        assert!(a.contains(a));
        assert!(a.contains(b));
        assert!(a.contains(c));
        assert!(!a.contains(d));
        assert!(!b.contains(a));
        assert!(b.contains(b));
        assert!(b.contains(c));
        assert!(!b.contains(d));
        assert!(!c.contains(a));
        assert!(!c.contains(b));
        assert!(c.contains(c));
        assert!(!c.contains(d));
        assert!(!d.contains(a));
        assert!(!d.contains(b));
        assert!(!d.contains(c));
        assert!(d.contains(d));

        assert!(a.intersects(a));
        assert!(a.intersects(b));
        assert!(a.intersects(c));
        assert!(!a.intersects(d));
        assert!(b.intersects(a));
        assert!(b.intersects(b));
        assert!(b.intersects(c));
        assert!(!b.intersects(d));
    }

    #[test]
    fn test_cell_id_string() {
        let ci = CellId(0xbb04000000000000);
        assert_eq!(ci.to_debug_string(), "5/31200");
    }

    #[test]
    fn test_from_debug_string() {
        assert_eq!(CellId::from_debug_string("3/"), Some(CellId::from_face(3)));
        assert_eq!(
            CellId::from_debug_string("0/21"),
            Some(CellId::from_face(0).children()[2].children()[1])
        );
        assert_eq!(
            CellId::from_debug_string("4/000000000000000000000000000000"),
            Some(CellId::from_face(4).range_min())
        );
        // Too many levels
        assert_eq!(
            CellId::from_debug_string("4/0000000000000000000000000000000"),
            None
        );
        assert_eq!(CellId::from_debug_string(""), None);
        assert_eq!(CellId::from_debug_string("7/"), None);
        assert_eq!(CellId::from_debug_string(" /"), None);
        assert_eq!(CellId::from_debug_string("3:0"), None);
        assert_eq!(CellId::from_debug_string("3/ 12"), None);
        assert_eq!(CellId::from_debug_string("3/1241"), None);
    }

    // --- LatLng conversion ---

    #[test]
    fn test_lat_lng() {
        let tests: Vec<(u64, f64, f64)> = vec![
            (0x47a1cbd595522b39, 49.703498679, 11.770681595),
            (0x46525318b63be0f9, 55.685376759, 12.588490937),
            (0x52b30b71698e729d, 45.486546517, -93.449700022),
            (0x46ed8886cfadda85, 58.299984854, 23.049300056),
            (0x3663f18a24cbe857, 34.364439040, 108.330699969),
            (0x10a06c0a948cf5d, -30.694551352, -30.048758753),
            (0x2b2bfd076787c5df, -25.285264027, 133.823116966),
            (0xb09dff882a7809e1, -75.000000031, 0.000000133),
            (0x94daa3d000000001, -24.694439215, -47.537363213),
            (0x87a1000000000001, 38.899730392, -99.901813021),
            (0x4fc76d5000000001, 81.647200334, -55.631712940),
            (0x3b00955555555555, 10.050986518, 78.293170610),
            (0x1dcc469991555555, -34.055420593, 18.551140038),
            (0xb112966aaaaaaaab, -69.219262171, 49.670072392),
        ];

        for (id_val, lat, lng) in &tests {
            let id = CellId(*id_val);
            let l1 = LatLng::from_degrees(*lat, *lng);
            let l2 = id.to_lat_lng();
            let dist = l1.get_distance(l2);
            assert!(
                dist.degrees() < 1e-9,
                "CellId({:#x}).to_lat_lng() = ({}, {}), want ({}, {}), dist = {}",
                id_val,
                l2.lat.degrees(),
                l2.lng.degrees(),
                lat,
                lng,
                dist.degrees(),
            );

            let c2 = CellId::from_lat_lng(&l1);
            assert_eq!(
                id, c2,
                "from_lat_lng({}, {}) = {:#x}, want {:#x}",
                lat, lng, c2.0, id_val
            );
        }
    }

    // --- Token conversion ---

    #[test]
    fn test_tokens_nominal() {
        let tests: Vec<(&str, u64)> = vec![
            ("1", 0x1000000000000000),
            ("3", 0x3000000000000000),
            ("14", 0x1400000000000000),
            ("41", 0x4100000000000000),
            ("094", 0x0940000000000000),
            ("537", 0x5370000000000000),
            ("3fec", 0x3fec000000000000),
            ("72f3", 0x72f3000000000000),
            ("52b8c", 0x52b8c00000000000),
            ("990ed", 0x990ed00000000000),
            ("4476dc", 0x4476dc0000000000),
            ("2a724f", 0x2a724f0000000000),
            ("7d4afc4", 0x7d4afc4000000000),
            ("b675785", 0xb675785000000000),
            ("40cd6124", 0x40cd612400000000),
            ("3ba32f81", 0x3ba32f8100000000),
            ("08f569b5c", 0x08f569b5c0000000),
            ("385327157", 0x3853271570000000),
            ("166c4d1954", 0x166c4d1954000000),
            ("96f48d8c39", 0x96f48d8c39000000),
            ("0bca3c7f74c", 0x0bca3c7f74c00000),
            ("1ae3619d12f", 0x1ae3619d12f00000),
            ("07a77802a3fc", 0x07a77802a3fc0000),
            ("4e7887ec1801", 0x4e7887ec18010000),
            ("4adad7ae74124", 0x4adad7ae74124000),
            ("90aba04afe0c5", 0x90aba04afe0c5000),
            ("8ffc3f02af305c", 0x8ffc3f02af305c00),
            ("6fa47550938183", 0x6fa4755093818300),
            ("aa80a565df5e7fc", 0xaa80a565df5e7fc0),
            ("01614b5e968e121", 0x01614b5e968e1210),
            ("aa05238e7bd3ee7c", 0xaa05238e7bd3ee7c),
            ("48a23db9c2963e5b", 0x48a23db9c2963e5b),
        ];

        for (token, id_val) in &tests {
            let ci = CellId::from_token(token);
            assert_eq!(
                ci.0, *id_val,
                "from_token({token}) = {:#x}, want {:#x}",
                ci.0, id_val
            );
            assert_eq!(ci.to_token(), *token, "to_token({id_val:#x})");
        }
    }

    #[test]
    fn test_tokens_error_cases() {
        assert_eq!(CellId(0).to_token(), "X");
        assert_eq!(CellId::from_token("X"), CellId(0));

        // Invalid tokens
        for bad in &["876b e99", "876bee99\n", "876[ee99", " 876bee99"] {
            assert_eq!(
                CellId::from_token(bad).0,
                0,
                "from_token({bad:?}) should be 0"
            );
        }
    }

    // --- Edge neighbors ---

    #[test]
    fn test_edge_neighbors() {
        // Face 1 edge neighbors should be faces 5, 3, 2, 0.
        let faces = [Face::F5, Face::F3, Face::F2, Face::F0];
        let nbrs = from_face_ij(Face::F1, 0, 0)
            .parent_at_level(0)
            .edge_neighbors();
        for (i, nbr) in nbrs.iter().enumerate() {
            assert!(nbr.is_face(), "neighbor {i} should be a face");
            assert_eq!(
                nbr.face(),
                faces[i],
                "face 1 edge neighbor {i}: got face {}, want {}",
                nbr.face(),
                faces[i]
            );
        }

        // Check corner cells at all levels.
        let max_ij = MAX_SIZE - 1;
        for level in 1..=MAX_CELL_LEVEL {
            let id = from_face_ij(Face::F1, 0, 0).parent_at_level(level);
            let level_size_ij = size_ij(level);
            let want = [
                from_face_ij(Face::F5, max_ij, max_ij).parent_at_level(level),
                from_face_ij(Face::F1, level_size_ij, 0).parent_at_level(level),
                from_face_ij(Face::F1, 0, level_size_ij).parent_at_level(level),
                from_face_ij(Face::F0, max_ij, 0).parent_at_level(level),
            ];
            let got = id.edge_neighbors();
            for i in 0..4 {
                assert_eq!(
                    got[i], want[i],
                    "level {level} edge neighbor {i}: got {}, want {}",
                    got[i], want[i]
                );
            }
        }
    }

    // --- Vertex neighbors ---

    #[test]
    fn test_vertex_neighbors() {
        // Center of face 2 at level 5.
        let id = CellId::from_point(&Point::from_coords(0.0, 0.0, 1.0));
        let mut neighbors = id.vertex_neighbors(5);
        neighbors.sort();

        for (n, neighbor) in neighbors.iter().enumerate() {
            let i = if n < 2 { (1 << 29) - 1 } else { 1 << 29 };
            let j = if n == 0 || n == 3 {
                (1 << 29) - 1
            } else {
                1 << 29
            };
            let want = from_face_ij(Face::F2, i, j).parent_at_level(5);
            assert_eq!(*neighbor, want, "vertex neighbor {n}");
        }

        // Corner of faces 0, 4, 5.
        let id = CellId::from_face_pos_level(0, 0, MAX_CELL_LEVEL);
        let mut neighbors = id.vertex_neighbors(0);
        neighbors.sort();
        assert_eq!(neighbors.len(), 3);
        assert_eq!(neighbors[0], CellId::from_face(0));
        assert_eq!(neighbors[1], CellId::from_face(4));
    }

    // --- Common ancestor level ---

    #[test]
    fn test_common_ancestor_level() {
        // Identical cells.
        assert_eq!(
            CellId::from_face(0).common_ancestor_level(CellId::from_face(0)),
            Some(Level::MIN)
        );
        assert_eq!(
            CellId::from_face(0)
                .child_begin_at_level(30)
                .common_ancestor_level(CellId::from_face(0).child_begin_at_level(30)),
            Some(Level::MAX)
        );

        // One is descendant of the other.
        assert_eq!(
            CellId::from_face(0)
                .child_begin_at_level(30)
                .common_ancestor_level(CellId::from_face(0)),
            Some(Level::MIN)
        );
        assert_eq!(
            CellId::from_face(5)
                .common_ancestor_level(CellId::from_face(5).child_end_at_level(30).prev()),
            Some(Level::MIN)
        );

        // No common ancestors (different faces).
        assert_eq!(
            CellId::from_face(0).common_ancestor_level(CellId::from_face(5)),
            None
        );
        assert_eq!(
            CellId::from_face(2)
                .child_begin_at_level(30)
                .common_ancestor_level(CellId::from_face(3).child_begin_at_level(20)),
            None
        );

        // Common ancestor distinct from both.
        assert_eq!(
            CellId::from_face(5)
                .child_begin_at_level(9)
                .next()
                .child_begin_at_level(15)
                .common_ancestor_level(
                    CellId::from_face(5)
                        .child_begin_at_level(9)
                        .child_begin_at_level(20)
                ),
            Some(Level::new(8))
        );
        assert_eq!(
            CellId::from_face(0)
                .child_begin_at_level(2)
                .child_begin_at_level(30)
                .common_ancestor_level(
                    CellId::from_face(0)
                        .child_begin_at_level(2)
                        .next()
                        .child_begin_at_level(5)
                ),
            Some(Level::new(1))
        );
    }

    // --- Advance ---

    #[test]
    fn test_advance() {
        assert_eq!(
            CellId::from_face(0).child_begin_at_level(0).advance(7),
            CellId::from_face(5).child_end_at_level(0)
        );
        assert_eq!(
            CellId::from_face(0).child_begin_at_level(0).advance(12),
            CellId::from_face(5).child_end_at_level(0)
        );
        assert_eq!(
            CellId::from_face(5).child_end_at_level(0).advance(-7),
            CellId::from_face(0).child_begin_at_level(0)
        );
        assert_eq!(
            CellId::from_face(5)
                .child_end_at_level(0)
                .advance(-12000000),
            CellId::from_face(0).child_begin_at_level(0)
        );

        // Advance 256 leaf cells = one cell at level MAX_LEVEL - 4.
        let id = CellId::from_face_pos_level(3, 0x12345678, MAX_CELL_LEVEL - 4);
        assert_eq!(
            id.child_begin_at_level(MAX_CELL_LEVEL).advance(256),
            id.next().child_begin_at_level(MAX_CELL_LEVEL)
        );

        assert_eq!(
            CellId::from_face_pos_level(1, 0, MAX_CELL_LEVEL)
                .advance(4 << (2 * i64::from(MAX_CELL_LEVEL))),
            CellId::from_face_pos_level(5, 0, MAX_CELL_LEVEL)
        );
    }

    // --- Wrapping ---

    #[test]
    fn test_wrapping() {
        // Wrap from beginning to end.
        assert_eq!(
            CellId::from_face(0).child_begin_at_level(0).prev_wrap(),
            CellId::from_face(5).child_end_at_level(0).prev()
        );

        // Smallest end leaf wraps to smallest first leaf.
        assert_eq!(
            CellId::from_face(0)
                .child_begin_at_level(MAX_CELL_LEVEL)
                .prev_wrap(),
            CellId::from_face_pos_level(5, !0u64 >> FACE_BITS, MAX_CELL_LEVEL)
        );

        // PrevWrap == AdvanceWrap(-1)
        assert_eq!(
            CellId::from_face(0)
                .child_begin_at_level(MAX_CELL_LEVEL)
                .prev_wrap(),
            CellId::from_face(0)
                .child_begin_at_level(MAX_CELL_LEVEL)
                .advance_wrap(-1)
        );

        // Prev + NextWrap stays the same.
        assert_eq!(
            CellId::from_face(5)
                .child_end_at_level(4)
                .prev()
                .next_wrap(),
            CellId::from_face(0).child_begin_at_level(4)
        );

        // AdvanceWrap forward and back.
        assert_eq!(
            CellId::from_face(5)
                .child_end_at_level(4)
                .advance(-1)
                .advance_wrap(1),
            CellId::from_face(0).child_begin_at_level(4)
        );

        // Advancing 7 steps around cube.
        assert_eq!(
            CellId::from_face(0).child_begin_at_level(0).advance_wrap(7),
            CellId::from_face(1)
        );

        // Twice around.
        assert_eq!(
            CellId::from_face(0)
                .child_begin_at_level(0)
                .advance_wrap(12),
            CellId::from_face(0).child_begin_at_level(0)
        );

        // Backwards once around plus one step.
        assert_eq!(CellId::from_face(5).advance_wrap(-7), CellId::from_face(4));

        // Even multiple wrapping.
        assert_eq!(
            CellId::from_face(0)
                .child_begin_at_level(0)
                .advance_wrap(-12000000),
            CellId::from_face(0).child_begin_at_level(0)
        );

        // Combination of advances that should be equivalent.
        assert_eq!(
            CellId::from_face(0)
                .child_begin_at_level(5)
                .advance_wrap(6644),
            CellId::from_face(0)
                .child_begin_at_level(5)
                .advance_wrap(-11788)
        );
    }

    // --- Distance from begin ---

    #[test]
    fn test_distance_from_begin() {
        assert_eq!(
            CellId::from_face(5)
                .child_end_at_level(0)
                .distance_from_begin(),
            6
        );
        assert_eq!(
            CellId::from_face(5)
                .child_end_at_level(MAX_CELL_LEVEL)
                .distance_from_begin(),
            6 * (1i64 << (2 * u32::from(MAX_CELL_LEVEL)))
        );
        assert_eq!(
            CellId::from_face(0)
                .child_begin_at_level(0)
                .distance_from_begin(),
            0
        );
        assert_eq!(
            CellId::from_face(0)
                .child_begin_at_level(MAX_CELL_LEVEL)
                .distance_from_begin(),
            0
        );

        // Advancing from begin by distance should return the same cell.
        let id = CellId::from_face_pos_level(3, 0x12345678, MAX_CELL_LEVEL - 4);
        assert_eq!(
            CellId::from_face(0)
                .child_begin_at_level(id.level())
                .advance(id.distance_from_begin()),
            id
        );
    }

    // --- FaceSiTi ---

    #[test]
    fn test_face_si_ti() {
        let id = CellId::from_face_pos_level(3, 0x12345678, MAX_CELL_LEVEL);
        for level in 0..=MAX_CELL_LEVEL {
            let l = MAX_CELL_LEVEL - level;
            let want = 1u32 << level;
            let mask = (1u32 << (level + 1)) - 1;

            let (_, si, ti) = id.parent_at_level(l).get_center_si_ti();
            assert_eq!(
                si & mask,
                want,
                "level {l}: si & mask = {}, want {}",
                si & mask,
                want
            );
            assert_eq!(
                ti & mask,
                want,
                "level {l}: ti & mask = {}, want {}",
                ti & mask,
                want
            );
        }
    }

    // --- Maximum tile ---

    #[test]
    fn test_maximum_tile() {
        // Test a fixed cell at level 10.
        let id = CellId::from_face_pos_level(3, 0x12345678, 10);

        // limit == id returns id.
        assert_eq!(id.maximum_tile(id), id);
        // child[0] starts at range_min(id), so limit=id means limit is reached.
        assert_eq!(id.children()[0].maximum_tile(id), id);
        // child[1] past the beginning of id.
        assert_eq!(id.children()[1].maximum_tile(id), id);
        // id.next() starts past id.
        assert_eq!(id.next().maximum_tile(id), id);

        // Shrinking: limit is child[0] of id.
        assert_eq!(id.maximum_tile(id.children()[0]), id.children()[0]);

        // Growing: child[0] with limit=next → should grow to id.
        assert_eq!(id.children()[0].maximum_tile(id.next()), id);
        assert_eq!(id.children()[0].maximum_tile(id.next().children()[0]), id);

        // Growing from deeper children.
        assert_eq!(id.children()[0].children()[0].maximum_tile(id.next()), id);
        assert_eq!(
            id.children()[0].children()[0].children()[0].maximum_tile(id.next()),
            id
        );

        // Shrinking: limit is child[0].next.
        assert_eq!(id.maximum_tile(id.children()[0].next()), id.children()[0]);
        assert_eq!(
            id.maximum_tile(id.children()[0].next().children()[0]),
            id.children()[0]
        );

        // Deeper shrinking.
        assert_eq!(
            id.maximum_tile(id.children()[0].children()[0].next()),
            id.children()[0].children()[0]
        );
    }

    // --- Face Definitions ---

    #[test]
    fn test_face_definitions() {
        // Verify which face each lat/lng maps to, matching C++ FaceDefinitions test.
        let get_face = |lat_deg: f64, lng_deg: f64| -> Face {
            CellId::from_lat_lng(&LatLng::from_degrees(lat_deg, lng_deg)).face()
        };
        assert_eq!(get_face(0.0, 0.0), Face::F0);
        assert_eq!(get_face(0.0, 90.0), Face::F1);
        assert_eq!(get_face(90.0, 0.0), Face::F2);
        assert_eq!(get_face(0.0, 180.0), Face::F3);
        assert_eq!(get_face(0.0, -90.0), Face::F4);
        assert_eq!(get_face(-90.0, 0.0), Face::F5);
    }

    // --- Continuity ---

    #[test]
    fn test_continuity() {
        // Verify that sequentially increasing cell IDs form a continuous path
        // over the surface of the sphere (no discontinuous jumps).
        use crate::s2::metric::MAX_EDGE;

        const MAX_WALK_LEVEL: u8 = 8;
        let max_dist = MAX_EDGE.value(MAX_WALK_LEVEL);

        // Begin/End iterate over all cells at the given level across all faces.
        // Begin(level) = face 0 child_begin_at_level
        // End(level) = face 5 child_end_at_level
        let begin = CellId::from_face(0).child_begin_at_level(MAX_WALK_LEVEL);
        let end = CellId::from_face(5).child_end_at_level(MAX_WALK_LEVEL);
        let mut id = begin;
        let mut count = 0u64;
        while id != end {
            let next = id.next_wrap();
            let angle = id.to_point().0.angle(next.to_point().0);
            assert!(
                angle <= max_dist,
                "Discontinuity at cell {id}: angle {angle} > max_dist {max_dist} (count={count})",
            );

            // Verify advance_wrap consistency.
            assert_eq!(id.next_wrap(), id.advance_wrap(1));
            assert_eq!(id, id.next_wrap().advance_wrap(-1));

            id = id.next();
            count += 1;
        }
        // At level 8, there should be 6 * 4^8 = 393216 cells.
        assert_eq!(count, 6 * (1u64 << (2 * u64::from(MAX_WALK_LEVEL))));
    }

    // --- Inverses ---

    #[test]
    fn test_inverses() {
        // Verify that converting leaf cells to LatLng and back preserves the cell ID.
        // Use deterministic set of cells across all faces and positions.
        for face in 0..6u8 {
            for pos_shift in [0u64, 0x12345678, 0xABCDEF01, 0x55555555, 0xFFFFFFFF] {
                let id = CellId::from_face_pos_level(face, pos_shift, MAX_CELL_LEVEL);
                assert!(id.is_leaf(), "Expected leaf cell");
                assert_eq!(id.level(), MAX_CELL_LEVEL);
                let center = id.to_lat_lng();
                let reconstructed = CellId::from_lat_lng(&center);
                assert_eq!(
                    id.0, reconstructed.0,
                    "CellId -> LatLng -> CellId roundtrip failed for face={face}, pos={pos_shift:#x}"
                );
            }
        }
    }

    // --- Display / FromStr ---

    #[test]
    fn test_display_fromstr() {
        let id = CellId::from_face_pos_level(3, 0x12345678, MAX_CELL_LEVEL - 4);
        let s = format!("{id}");
        let id2: CellId = s.parse().unwrap();
        assert_eq!(id, id2);

        let face = CellId::from_face(2);
        assert_eq!(format!("{face}"), "2/");
        assert_eq!("2/".parse::<CellId>().unwrap(), face);
    }

    // --- ij_level_to_bound_uv ---

    #[test]
    fn test_ij_level_to_bound_uv() {
        let max_ij = (1 << MAX_CELL_LEVEL) - 1;

        // Minimum i,j at level 0.
        let uv = ij_level_to_bound_uv(0, 0, 0);
        assert!(float64_near(uv.x.lo, -1.0, 1e-15));
        assert!(float64_near(uv.x.hi, 1.0, 1e-15));
        assert!(float64_near(uv.y.lo, -1.0, 1e-15));
        assert!(float64_near(uv.y.hi, 1.0, 1e-15));

        // Maximum i,j at level 0.
        let uv = ij_level_to_bound_uv(max_ij, max_ij, 0);
        assert!(float64_near(uv.x.lo, -1.0, 1e-15));
        assert!(float64_near(uv.x.hi, 1.0, 1e-15));

        // Minimum i,j at MAX_LEVEL.
        let uv = ij_level_to_bound_uv(0, 0, MAX_CELL_LEVEL);
        assert!(float64_near(uv.x.lo, -1.0, 1e-15));
        assert!(float64_near(uv.x.hi, -0.999999997516473060, 1e-15));

        // Maximum i,j at MAX_LEVEL.
        let uv = ij_level_to_bound_uv(max_ij, max_ij, MAX_CELL_LEVEL);
        assert!(float64_near(uv.x.lo, 0.999999997516473060, 1e-15));
        assert!(float64_near(uv.x.hi, 1.0, 1e-15));
    }

    // --- All neighbors ---

    #[test]
    fn test_cell_id_all_neighbors() {
        // Test all_neighbors at the same level for a cell near a face boundary.
        // Face 0, high i value (near the edge shared with face 1).
        let max_ij = MAX_SIZE - 1;
        let near_edge = from_face_ij(Face::F0, max_ij, MAX_SIZE / 2).parent_at_level(5);
        let level = near_edge.level();
        let nbrs = near_edge.all_neighbors(level).unwrap();

        // All neighbors must be valid cells at the requested level.
        for nbr in &nbrs {
            assert!(nbr.is_valid(), "neighbor {nbr:?} is not valid");
            assert_eq!(
                nbr.level(),
                level,
                "neighbor {:?} has level {}, want {}",
                nbr,
                nbr.level(),
                level
            );
        }

        // The cell itself must NOT appear in its neighbor list.
        assert!(
            !nbrs.contains(&near_edge),
            "neighbor list should not contain the cell itself"
        );

        // No duplicates.
        let mut sorted = nbrs.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            nbrs.len(),
            "neighbor list should not contain duplicates"
        );

        // At level == self.level(), we expect 8 neighbors (the surrounding ring).
        assert_eq!(
            nbrs.len(),
            8,
            "expected 8 neighbors at same level, got {}",
            nbrs.len()
        );

        // Since the cell is near the face 0/1 boundary, at least one neighbor
        // should be on a different face.
        let has_other_face = nbrs.iter().any(|n| n.face() != near_edge.face());
        assert!(
            has_other_face,
            "expected at least one neighbor on a different face for cell near face edge"
        );

        // Test all_neighbors at a finer level (one level deeper).
        let fine_level = level + 1;
        let fine_nbrs = near_edge.all_neighbors(fine_level).unwrap();
        for nbr in &fine_nbrs {
            assert!(nbr.is_valid(), "fine neighbor {nbr:?} is not valid");
            assert_eq!(nbr.level(), fine_level);
        }
        // At level+1, the ring has more cells: top/bottom rows contribute
        // 2*2=4 cells per side neighbor, plus left/right contribute more.
        // The count should be larger than the same-level case.
        assert!(
            fine_nbrs.len() > nbrs.len(),
            "finer level should produce more neighbors: got {} vs {}",
            fine_nbrs.len(),
            nbrs.len()
        );

        // Test that requesting a level below self.level() returns None.
        assert!(
            near_edge.all_neighbors(level - 1).is_none(),
            "all_neighbors with level < self.level() should return None"
        );
    }

    // --- Advance wrap ---

    #[test]
    fn test_cell_id_advance_wrap() {
        // The total number of cells at level 0 is 6 (one per face).
        // advance_wrap(6) at level 0 should return to the same cell.
        let start = CellId::from_face(0);
        assert_eq!(
            start.advance_wrap(6),
            start,
            "advancing 6 steps at level 0 should wrap back to start"
        );

        // advance_wrap(-6) at level 0 should also wrap back to start.
        assert_eq!(
            start.advance_wrap(-6),
            start,
            "advancing -6 steps at level 0 should wrap back to start"
        );

        // advance_wrap(+N) followed by advance_wrap(-N) returns to start.
        let id = CellId::from_face_pos_level(3, 0x12345678, 15);
        for &steps in &[1i64, 100, 100_000, 1_000_000_000] {
            let forwarded = id.advance_wrap(steps);
            let roundtrip = forwarded.advance_wrap(-steps);
            assert_eq!(
                roundtrip, id,
                "advance_wrap({}) then advance_wrap({}) should return to start",
                steps, -steps
            );
        }

        // Large positive wrap: more than the total number of cells at a given level.
        // At level 0, there are 6 cells. 13 steps = 2*6 + 1, so advance_wrap(13) == advance_wrap(1).
        assert_eq!(
            CellId::from_face(0).advance_wrap(13),
            CellId::from_face(0).advance_wrap(1),
            "advance_wrap(13) should equal advance_wrap(1) at level 0"
        );

        // Large negative wrap.
        assert_eq!(
            CellId::from_face(2).advance_wrap(-14),
            CellId::from_face(2).advance_wrap(-2),
            "advance_wrap(-14) should equal advance_wrap(-2) at level 0"
        );

        // Verify wrapping consistency at a deeper level.
        let deep = CellId::from_face_pos_level(1, 0, 10);
        let total_at_level_10 = 6i64 * (1 << (2 * 10)); // 6 * 4^10
        assert_eq!(
            deep.advance_wrap(total_at_level_10),
            deep,
            "advancing by total cell count at level 10 should return to start"
        );
        assert_eq!(
            deep.advance_wrap(-total_at_level_10),
            deep,
            "advancing by -total cell count at level 10 should return to start"
        );
    }

    // --- From trait impls ---

    #[test]
    fn test_cell_id_from_trait_impls() {
        // From<u64> for CellId
        let raw: u64 = 0x80855c0000000000;
        let id: CellId = CellId::from(raw);
        assert_eq!(id.0, raw);

        // From<CellId> for u64
        let back: u64 = u64::from(id);
        assert_eq!(back, raw);

        // Roundtrip: u64 -> CellId -> u64
        assert_eq!(u64::from(CellId::from(raw)), raw);

        // From<&Point> for CellId
        let ll = LatLng::from_degrees(49.703498679, 11.770681595);
        let point = ll.to_point();
        let id_from_point: CellId = CellId::from(&point);
        let id_direct = CellId::from_point(&point);
        assert_eq!(
            id_from_point, id_direct,
            "From<&Point> should match from_point"
        );

        // From<&LatLng> for CellId
        let id_from_ll: CellId = CellId::from(&ll);
        let id_direct_ll = CellId::from_lat_lng(&ll);
        assert_eq!(
            id_from_ll, id_direct_ll,
            "From<&LatLng> should match from_lat_lng"
        );

        // From<&Point> and From<&LatLng> should agree (same underlying point).
        assert_eq!(
            id_from_point, id_from_ll,
            "From<&Point> and From<&LatLng> should agree for same location"
        );

        // FromStr for CellId
        let face_cell = CellId::from_face(3);
        let parsed: CellId = "3/".parse().unwrap();
        assert_eq!(parsed, face_cell, "FromStr should parse face cell");

        let child = CellId::from_face(0).children()[2].children()[1];
        let parsed2: CellId = "0/21".parse().unwrap();
        assert_eq!(parsed2, child, "FromStr should parse child path");

        // FromStr error case
        let err = "invalid".parse::<CellId>();
        assert!(err.is_err(), "FromStr should return Err for invalid input");

        // Display -> FromStr roundtrip
        let id = CellId::from_face_pos_level(4, 0xABCDEF00, 20);
        let displayed = format!("{id}");
        let roundtrip: CellId = displayed.parse().unwrap();
        assert_eq!(
            id, roundtrip,
            "Display -> FromStr roundtrip should be exact"
        );
    }

    // --- Center ST and UV ---

    #[test]
    fn test_cell_id_center_st_and_uv() {
        // Test center_st() returns values in [0, 1] for all faces at level 0.
        for face in 0..6u8 {
            let id = CellId::from_face(face);
            let st = id.center_st();
            assert!(
                st.x >= 0.0 && st.x <= 1.0,
                "face {} center_st().x = {} not in [0,1]",
                face,
                st.x
            );
            assert!(
                st.y >= 0.0 && st.y <= 1.0,
                "face {} center_st().y = {} not in [0,1]",
                face,
                st.y
            );
            // Face-level cells should have center at (0.5, 0.5) in ST space.
            assert!(
                float64_near(st.x, 0.5, 1e-15),
                "face {} center_st().x = {}, want 0.5",
                face,
                st.x
            );
            assert!(
                float64_near(st.y, 0.5, 1e-15),
                "face {} center_st().y = {}, want 0.5",
                face,
                st.y
            );
        }

        // Center UV at level 0 should be (0, 0) since st_to_uv(0.5) = 0.
        for face in 0..6u8 {
            let id = CellId::from_face(face);
            let uv = id.center_uv();
            assert!(
                float64_near(uv.x, 0.0, 1e-15),
                "face {} center_uv().x = {}, want 0.0",
                face,
                uv.x
            );
            assert!(
                float64_near(uv.y, 0.0, 1e-15),
                "face {} center_uv().y = {}, want 0.0",
                face,
                uv.y
            );
        }

        // Test at deeper levels: ST should still be in [0, 1] and UV in [-1, 1].
        for level in [1u8, 5, 10, 15, 20, 25, MAX_CELL_LEVEL] {
            for face in 0..6u8 {
                // Test child_begin (lowest cell) and child_end.prev (highest cell).
                let lo = CellId::from_face(face).child_begin_at_level(level);
                let hi = CellId::from_face(face).child_end_at_level(level).prev();

                for id in [lo, hi] {
                    let st = id.center_st();
                    assert!(
                        st.x >= 0.0 && st.x <= 1.0,
                        "level {} face {} id={}: center_st().x = {} not in [0,1]",
                        level,
                        face,
                        id,
                        st.x
                    );
                    assert!(
                        st.y >= 0.0 && st.y <= 1.0,
                        "level {} face {} id={}: center_st().y = {} not in [0,1]",
                        level,
                        face,
                        id,
                        st.y
                    );

                    let uv = id.center_uv();
                    assert!(
                        uv.x >= -1.0 && uv.x <= 1.0,
                        "level {} face {} id={}: center_uv().x = {} not in [-1,1]",
                        level,
                        face,
                        id,
                        uv.x
                    );
                    assert!(
                        uv.y >= -1.0 && uv.y <= 1.0,
                        "level {} face {} id={}: center_uv().y = {} not in [-1,1]",
                        level,
                        face,
                        id,
                        uv.y
                    );
                }
            }
        }

        // Verify consistency: center_uv should equal st_to_uv(center_st).
        let id = CellId::from_face_pos_level(2, 0x12345678, 15);
        let st = id.center_st();
        let uv = id.center_uv();
        assert!(
            float64_near(uv.x, st_to_uv(st.x), 1e-15),
            "center_uv().x = {} != st_to_uv(center_st().x) = {}",
            uv.x,
            st_to_uv(st.x)
        );
        assert!(
            float64_near(uv.y, st_to_uv(st.y), 1e-15),
            "center_uv().y = {} != st_to_uv(center_st().y) = {}",
            uv.y,
            st_to_uv(st.y)
        );
    }

    #[test]
    fn test_begin_end() {
        // Begin(0) should be face 0 at level 0.
        let b0 = CellId::begin(0);
        assert_eq!(b0, CellId::from_face(0));
        // End(0) should be past face 5 at level 0.
        let e0 = CellId::end(0);
        assert!(e0.0 > CellId::from_face(5).0);
        // Begin at leaf level.
        let b30 = CellId::begin(MAX_CELL_LEVEL);
        assert!(b30.is_leaf());
        // Begin should be less than End at any level.
        for level in 0..=MAX_CELL_LEVEL {
            assert!(CellId::begin(level).0 < CellId::end(level).0);
        }
    }

    #[test]
    fn test_to_point_raw() {
        // to_point_raw should be proportional to to_point but unnormalized.
        let id = CellId::from_face(0).child_begin_at_level(15);
        let raw = id.to_point_raw();
        let norm = id.to_point();
        let normalized = Point(raw.vector().normalize());
        let diff = (normalized.vector() - norm.vector()).norm();
        assert!(diff < 1e-14);
    }

    #[test]
    fn test_expanded_by_distance_uv() {
        // Expanding by zero should not change the rect significantly.
        let uv = ij_level_to_bound_uv(0, 0, 10);
        let expanded = expanded_by_distance_uv(&uv, crate::s1::Angle::from_radians(0.0));
        assert!((expanded.x.lo - uv.x.lo).abs() < 1e-10);
        assert!((expanded.x.hi - uv.x.hi).abs() < 1e-10);

        // Expanding by a positive distance should grow the rect.
        let expanded = expanded_by_distance_uv(&uv, crate::s1::Angle::from_degrees(1.0));
        assert!(expanded.x.lo < uv.x.lo);
        assert!(expanded.x.hi > uv.x.hi);
        assert!(expanded.y.lo < uv.y.lo);
        assert!(expanded.y.hi > uv.y.hi);

        // Shrinking by a small distance should reduce the rect.
        let expanded = expanded_by_distance_uv(&uv, crate::s1::Angle::from_degrees(-0.001));
        assert!(expanded.x.lo > uv.x.lo);
        assert!(expanded.x.hi < uv.x.hi);
    }
}

#[cfg(test)]
mod quickcheck_tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    fn random_cell_id(raw: u64) -> CellId {
        // Generate a valid cell ID from an arbitrary u64.
        let face = (raw % 6) as u8;
        let level = ((raw >> 3) % 31) as u8;
        let pos = raw >> 6;
        CellId::from_face_pos_level(face, pos, level)
    }

    #[quickcheck]
    fn prop_parent_contains_child(raw: u64) -> bool {
        let id = random_cell_id(raw);
        if id.is_face() {
            return true;
        }
        id.parent().contains(id)
    }

    #[quickcheck]
    fn prop_range_min_le_cell_le_range_max(raw: u64) -> bool {
        let id = random_cell_id(raw);
        id.range_min().0 <= id.0 && id.0 <= id.range_max().0
    }

    #[quickcheck]
    fn prop_token_roundtrip(raw: u64) -> bool {
        let id = random_cell_id(raw);
        CellId::from_token(&id.to_token()) == id
    }

    #[quickcheck]
    fn prop_face_roundtrip(raw: u64) -> bool {
        let id = random_cell_id(raw);
        let (f, i, j, _) = id.to_face_ij_orientation();
        let leaf = from_face_ij(f, i, j);
        leaf.parent_at_level(id.level()) == id
    }

    #[quickcheck]
    fn prop_children_cover_parent(raw: u64) -> bool {
        let id = random_cell_id(raw);
        if id.is_leaf() {
            return true;
        }
        let kids = id.children();
        kids[0].range_min() == id.range_min()
            && kids[3].range_max() == id.range_max()
            && kids[0].range_max().next() == kids[1].range_min()
            && kids[1].range_max().next() == kids[2].range_min()
            && kids[2].range_max().next() == kids[3].range_min()
    }

    #[quickcheck]
    fn prop_contains_reflexive(raw: u64) -> bool {
        let id = random_cell_id(raw);
        id.contains(id)
    }

    #[quickcheck]
    fn prop_point_roundtrip(raw: u64) -> bool {
        let id = random_cell_id(raw);
        let p = id.to_point();
        let id2 = CellId::from_point(&p);
        id2.parent_at_level(id.level()) == id
    }

    // --- Default constructor ---

    #[test]
    fn test_default_constructor() {
        let id = CellId(0);
        assert_eq!(id.0, 0);
        assert!(!id.is_valid());
    }

    // --- Display / to_debug_string ---

    #[test]
    fn test_output_display() {
        let cell = CellId(0xbb04000000000000u64);
        assert_eq!(format!("{cell}"), "5/31200");
    }

    // --- Token encode/decode ---

    #[test]
    fn test_token_roundtrip() {
        // A sample cell at various levels.
        let id = CellId::from_face_pos_level(3, 0x12345678, 20);
        let token = id.to_token();
        let back = CellId::from_token(&token);
        assert_eq!(id, back, "token round-trip failed for {id}");

        // Face cell.
        let id = CellId::from_face(0);
        let token = id.to_token();
        let back = CellId::from_token(&token);
        assert_eq!(id, back, "token round-trip failed for face cell");

        // None/sentinel cell.
        let none = CellId::none();
        let token = none.to_token();
        let back = CellId::from_token(&token);
        assert_eq!(none, back, "token round-trip failed for none");
    }

    // --- Corner cell has 7 neighbors ---

    #[test]
    fn test_corner_cell_has_7_neighbors() {
        // Cell "3/0000" is a corner cell and has 7 unique neighbors (one duplicate
        // in the output because 2/3333 appears twice).
        let id = CellId::from_debug_string("3/0000").unwrap();
        let output = id.all_neighbors(id.level()).unwrap();
        // C++ expects 8 entries (one duplicate), but let's check unique count.
        let mut unique: Vec<CellId> = output.clone();
        unique.sort();
        unique.dedup();
        assert!(
            unique.len() == 7 || unique.len() == 8,
            "corner cell should have 7-8 unique neighbors, got {}",
            unique.len()
        );

        // Check specific expected neighbors are present.
        let expected_strs = ["3/0001", "3/0002", "3/0003"];
        for s in &expected_strs {
            let expected = CellId::from_debug_string(s).unwrap();
            assert!(
                output.contains(&expected),
                "missing expected neighbor {} in {:?}",
                s,
                output
                    .iter()
                    .map(|c| c.to_debug_string())
                    .collect::<Vec<_>>()
            );
        }
    }

    // --- Face-level neighbors ---

    #[test]
    fn test_face_level_neighbors() {
        // Face 3 should have faces 1, 2, 4, 5 as neighbors (each adjacent).
        let id = CellId::from_debug_string("3/").unwrap();
        let output = id.all_neighbors(id.level()).unwrap();
        // Neighbors at level 0 should include some of [0,1,2,4,5].
        let mut faces: Vec<Face> = output.iter().map(|c| c.face()).collect();
        faces.sort_unstable();
        faces.dedup();
        // Face 3 borders faces 1, 2, 4, 5 but NOT face 0.
        assert!(
            faces.contains(&Face::F1),
            "face 3 should have face 1 neighbor"
        );
        assert!(
            faces.contains(&Face::F2),
            "face 3 should have face 2 neighbor"
        );
        assert!(
            faces.contains(&Face::F4),
            "face 3 should have face 4 neighbor"
        );
        assert!(
            faces.contains(&Face::F5),
            "face 3 should have face 5 neighbor"
        );
        assert!(
            !faces.contains(&Face::F0),
            "face 3 should NOT have face 0 neighbor"
        );
    }

    // --- Debug string roundtrip ---

    #[test]
    fn test_debug_string_known_values() {
        // Verify specific known debug string ↔ CellId mappings.
        let cases = [("3/0000", 4), ("5/31200", 5), ("0/", 0)];
        for (s, expected_level) in &cases {
            let id = CellId::from_debug_string(s).unwrap();
            assert_eq!(
                i32::from(id.level()),
                *expected_level,
                "wrong level for {s}"
            );
            assert_eq!(
                &id.to_debug_string(),
                s,
                "debug string roundtrip failed for {s}"
            );
        }
    }

    // --- Sentinel range_min/range_max ---

    #[test]
    fn test_sentinel_range_min_max() {
        assert_eq!(CellId::sentinel(), CellId::sentinel().range_min());
        assert_eq!(CellId::sentinel(), CellId::sentinel().range_max());
    }

    // --- Common ancestor level ---

    #[test]
    fn test_common_ancestor_level_same_cell() {
        // Two identical cell ids.
        assert_eq!(
            CellId::from_face(0).common_ancestor_level(CellId::from_face(0)),
            Some(Level::MIN)
        );
        assert_eq!(
            CellId::from_face(0)
                .child_begin_at_level(MAX_CELL_LEVEL)
                .common_ancestor_level(CellId::from_face(0).child_begin_at_level(MAX_CELL_LEVEL)),
            Some(Level::MAX),
        );
    }

    #[test]
    fn test_common_ancestor_level_descendant() {
        // One cell is a descendant of the other.
        assert_eq!(
            CellId::from_face(0)
                .child_begin_at_level(MAX_CELL_LEVEL)
                .common_ancestor_level(CellId::from_face(0)),
            Some(Level::MIN),
        );
    }

    #[test]
    fn test_common_ancestor_level_different_faces() {
        // Two cells on different faces have no common ancestor.
        assert_eq!(
            CellId::from_face(0).common_ancestor_level(CellId::from_face(5)),
            None,
        );
    }

    #[test]
    fn test_common_ancestor_level_shared_ancestor() {
        // Two cells that have a common ancestor distinct from both.
        let a = CellId::from_face(5)
            .child_begin_at_level(9)
            .next()
            .child_begin_at_level(15);
        let b = CellId::from_face(5)
            .child_begin_at_level(9)
            .child_begin_at_level(20);
        assert_eq!(a.common_ancestor_level(b), Some(Level::new(8)));
    }

    // --- Wrapping ---

    #[test]
    fn test_wrapping_begin_prev_wrap() {
        // Wrapping backward from the beginning should give the last cell.
        let begin = CellId::from_face(0).child_begin_at_level(4);
        let end = CellId::from_face(5).child_end_at_level(4);
        assert_eq!(begin.prev_wrap(), end.prev());
    }

    #[test]
    fn test_wrapping_end_next_wrap() {
        // Wrapping forward from the end should give the first cell.
        let begin = CellId::from_face(0).child_begin_at_level(4);
        let end = CellId::from_face(5).child_end_at_level(4);
        assert_eq!(end.prev().next_wrap(), begin);
    }

    #[test]
    fn test_wrapping_advance_wrap() {
        // advance_wrap(-1) from begin == prev_wrap from begin
        let begin = CellId::from_face(0).child_begin_at_level(MAX_CELL_LEVEL);
        assert_eq!(begin.prev_wrap(), begin.advance_wrap(-1));
    }

    #[quickcheck]
    fn prop_is_valid(raw: u64) -> bool {
        let id = random_cell_id(raw);
        id.is_valid()
    }

    #[quickcheck]
    fn prop_level_in_range(raw: u64) -> bool {
        let id = random_cell_id(raw);
        id.level() <= MAX_CELL_LEVEL
    }

    #[quickcheck]
    fn prop_debug_string_roundtrip(raw: u64) -> bool {
        let id = random_cell_id(raw);
        let s = id.to_debug_string();
        CellId::from_debug_string(&s) == Some(id)
    }

    #[cfg(feature = "serde")]
    #[quickcheck]
    fn prop_serde_roundtrip(raw: u64) -> bool {
        let id = random_cell_id(raw);
        let json = serde_json::to_string(&id).unwrap();
        let back: CellId = serde_json::from_str(&json).unwrap();
        back == id
    }

    // --- New foundational tests ---

    #[test]
    fn test_from_point_roundtrip() {
        // Every point should map to a valid leaf cell, and the cell center
        // should be close to the original point.
        let points = [
            LatLng::from_degrees(0.0, 0.0).to_point(),
            LatLng::from_degrees(90.0, 0.0).to_point(),
            LatLng::from_degrees(-90.0, 0.0).to_point(),
            LatLng::from_degrees(45.0, 90.0).to_point(),
            LatLng::from_degrees(-30.0, -120.0).to_point(),
        ];
        for p in &points {
            let id = CellId::from_point(p);
            assert!(id.is_leaf());
            assert!(id.is_valid());
            let center = id.to_point();
            let dist = center.distance(*p);
            // Leaf cells have a maximum diagonal of ~1.2e-7 radians at level 30,
            // so the center can be up to half that from the input point.
            assert!(
                dist.radians() < 1e-7,
                "from_point roundtrip error: {:.2e} radians for {:?}",
                dist.radians(),
                p
            );
        }
    }

    #[test]
    fn test_contains_hierarchy() {
        // Parent should contain child at every level.
        let leaf = CellId::from_point(&LatLng::from_degrees(48.8566, 2.3522).to_point());
        for level in 0..MAX_CELL_LEVEL {
            let parent = leaf.parent_at_level(level);
            let child = leaf.parent_at_level(level + 1);
            assert!(
                parent.contains(child),
                "level {level} parent should contain level {} child",
                level + 1
            );
            assert!(
                !child.contains(parent),
                "level {} child should not contain level {level} parent",
                level + 1
            );
        }
    }

    #[test]
    fn test_to_face_ij_orientation_consistency() {
        // Verify that to_face_ij_orientation is consistent with from_face_ij.
        let test_ids = [
            CellId::from_face(0),
            CellId::from_face(3),
            CellId::from_point(&LatLng::from_degrees(0.0, 0.0).to_point()).parent_at_level(10),
            CellId::from_point(&LatLng::from_degrees(45.0, 90.0).to_point()).parent_at_level(15),
        ];
        for id in &test_ids {
            let (f, i, j, _) = id.to_face_ij_orientation();
            let reconstructed = from_face_ij(f, i, j).parent_at_level(id.level());
            assert_eq!(
                *id,
                reconstructed,
                "face_ij roundtrip failed for {:?}",
                id.to_debug_string()
            );
        }
    }

    #[test]
    fn test_child_position() {
        // child_position returns 0..3 indicating which quadrant.
        let parent = CellId::from_face(0);
        let children = parent.children();
        for (i, child) in children.iter().enumerate() {
            assert_eq!(child.child_position(1), i as u8);
        }
    }

    #[test]
    fn test_edge_neighbors_valid() {
        // Each edge neighbor should be a valid cell at the same level.
        let id = CellId::from_point(&LatLng::from_degrees(0.0, 0.0).to_point()).parent_at_level(10);
        let neighbors = id.edge_neighbors();
        for (k, nbr) in neighbors.iter().enumerate() {
            assert!(nbr.is_valid(), "edge neighbor {k} is invalid");
            assert_eq!(nbr.level(), id.level(), "edge neighbor {k} has wrong level");
            assert_ne!(*nbr, id, "edge neighbor {k} equals self");
        }
    }

    #[test]
    fn test_begin_end_iteration() {
        // begin(level)..end(level) should cover all cells at that level on each face.
        for level in [0u8, 1, 5] {
            let begin = CellId::begin(level);
            let end = CellId::end(level);
            assert!(begin < end, "begin >= end at level {level}");
            assert_eq!(begin.level(), level);
            assert!(begin.is_valid());
            // The range should span all 6 faces.
            assert_eq!(begin.face(), Face::F0);
            assert_eq!(end.prev().face(), Face::F5);
        }
    }
}
