// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! Coordinate system conversions for the S2 cell decomposition.
//!
//! Corresponds to C++ `s2coords.h`, Go `s2/stuv.go`.
//!
//! Defines conversions between the following coordinate systems:
//!
//! - **(face, i, j)**: Leaf-cell integer coordinates, `i` and `j` in `[0, 2^30)`.
//! - **(face, s, t)**: Cell-space coordinates in `[0, 1]`.
//! - **(face, si, ti)**: Discrete cell-space coordinates in `[0, 2^31]`.
//! - **(face, u, v)**: Cube-space coordinates in `[-1, 1]`.
//! - **(x, y, z)**: Direction vectors ([`crate::r3::Vector`]).
//!
//! The ST→UV transform uses the **quadratic projection** (default in C++ and Go).

#![expect(
    clippy::cast_sign_loss,
    reason = "IJ/ST coordinate conversions — values always non-negative at point of cast"
)]
#![expect(
    clippy::cast_possible_truncation,
    reason = "IJ/ST coordinate conversions — bounded by cell level constants"
)]
#![cfg_attr(
    test,
    expect(
        clippy::cast_possible_wrap,
        reason = "u32 -> i32 for IJ coordinate test roundtrips — bounded by LIMIT_IJ"
    )
)]
use std::fmt;

use crate::r3::{Axis, Vector};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum subdivision level for S2 cells.
pub const MAX_CELL_LEVEL: u8 = 30;

/// A cell subdivision level in the range `0..=30`.
///
/// This newtype ensures that cell levels are always in the valid range,
/// preventing out-of-range values from propagating through the codebase.
/// Use [`Level::new`] for a panicking constructor or [`Level::try_new`]
/// for a fallible one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Level(u8);

impl Level {
    /// The minimum cell level (face cells).
    pub const MIN: Level = Level(0);

    /// The maximum cell level (leaf cells, approximately 1 cm on Earth).
    pub const MAX: Level = Level(MAX_CELL_LEVEL);

    /// Creates a new level, panicking if `v > 30`.
    ///
    /// # Panics
    ///
    /// Panics if `v > 30`.
    #[track_caller]
    pub const fn new(v: u8) -> Self {
        assert!(v <= MAX_CELL_LEVEL, "level must be 0..=30");
        Level(v)
    }

    /// Tries to create a level from a `u8`.
    pub const fn try_new(v: u8) -> Option<Self> {
        if v <= MAX_CELL_LEVEL {
            Some(Level(v))
        } else {
            None
        }
    }

    /// Returns the raw `u8` value.
    pub const fn as_u8(self) -> u8 {
        self.0
    }

    /// Returns the level as a `usize`, suitable for array indexing.
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }

    /// Returns the level as a `u32`.
    pub const fn as_u32(self) -> u32 {
        self.0 as u32 // const context: From not available
    }

    /// Returns the level as an `i32`.
    pub const fn as_i32(self) -> i32 {
        self.0 as i32 // const context: From not available
    }
}

/// Checked addition: panics if result > 30.
impl std::ops::Add<u8> for Level {
    type Output = Level;
    #[track_caller]
    fn add(self, rhs: u8) -> Level {
        Level::new(self.0 + rhs)
    }
}

/// Checked subtraction: panics if result would underflow.
impl std::ops::Sub<u8> for Level {
    type Output = Level;
    #[track_caller]
    fn sub(self, rhs: u8) -> Level {
        Level::new(self.0 - rhs)
    }
}

/// Difference between two levels.
impl std::ops::Sub<Level> for Level {
    type Output = u8;
    fn sub(self, rhs: Level) -> u8 {
        self.0 - rhs.0
    }
}

impl PartialEq<u8> for Level {
    fn eq(&self, other: &u8) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<u8> for Level {
    fn partial_cmp(&self, other: &u8) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Level> for u8 {
    fn from(l: Level) -> u8 {
        l.0
    }
}

impl From<Level> for usize {
    fn from(l: Level) -> usize {
        l.0 as usize
    }
}

impl From<Level> for u32 {
    fn from(l: Level) -> u32 {
        u32::from(l.0)
    }
}

impl From<Level> for i32 {
    fn from(l: Level) -> i32 {
        i32::from(l.0)
    }
}

/// Converts a `u8` to a `Level`.
///
/// # Panics
///
/// Panics if `v > 30`.
impl From<u8> for Level {
    #[track_caller]
    fn from(v: u8) -> Self {
        Level::new(v)
    }
}

impl TryFrom<u16> for Level {
    type Error = &'static str;
    fn try_from(v: u16) -> Result<Self, Self::Error> {
        if v <= u16::from(MAX_CELL_LEVEL) {
            Ok(Level(v as u8))
        } else {
            Err("level must be 0..=30")
        }
    }
}

impl From<Level> for f64 {
    fn from(l: Level) -> f64 {
        f64::from(l.0)
    }
}

impl TryFrom<i32> for Level {
    type Error = &'static str;
    fn try_from(v: i32) -> Result<Self, Self::Error> {
        if v >= 0 && v <= i32::from(MAX_CELL_LEVEL) {
            Ok(Level(v as u8))
        } else {
            Err("level must be 0..=30")
        }
    }
}

impl TryFrom<usize> for Level {
    type Error = &'static str;
    fn try_from(v: usize) -> Result<Self, Self::Error> {
        if v <= MAX_CELL_LEVEL as usize {
            Ok(Level(v as u8))
        } else {
            Err("level must be 0..=30")
        }
    }
}

/// Number of cube faces.
pub const NUM_FACES: u8 = 6;

// ---------------------------------------------------------------------------
// Face enum
// ---------------------------------------------------------------------------

/// Identifies one of the six cube faces of the S2 cell decomposition.
///
/// The S2 library projects the sphere onto six cube faces. Faces 0–2 have
/// their positive normal along the +X, +Y, +Z axes respectively. Faces 3–5
/// have their positive normal along the −X, −Y, −Z axes.
///
/// ```text
///   Face 0: +X    Face 3: −X
///   Face 1: +Y    Face 4: −Y
///   Face 2: +Z    Face 5: −Z
/// ```
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum Face {
    /// Face 0: positive X axis.
    F0 = 0,
    /// Face 1: positive Y axis.
    F1 = 1,
    /// Face 2: positive Z axis.
    F2 = 2,
    /// Face 3: negative X axis.
    F3 = 3,
    /// Face 4: negative Y axis.
    F4 = 4,
    /// Face 5: negative Z axis.
    F5 = 5,
}

impl Face {
    /// All six faces in order.
    pub const ALL: [Face; 6] = [Face::F0, Face::F1, Face::F2, Face::F3, Face::F4, Face::F5];

    /// Converts a `u8` (0–5) to the corresponding face.
    ///
    /// # Panics
    /// Panics if `v >= 6`.
    #[inline]
    pub fn from_u8(v: u8) -> Face {
        Face::ALL[v as usize]
    }

    /// Returns the face number as a `u8` (0–5).
    #[inline]
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Returns an iterator over all six faces in order (F0..F5).
    pub fn iter() -> impl Iterator<Item = Face> {
        Face::ALL.iter().copied()
    }

    /// Returns the coordinate axis that is normal to this face.
    ///
    /// Faces 0 and 3 → X, faces 1 and 4 → Y, faces 2 and 5 → Z.
    #[inline]
    pub fn axis(self) -> Axis {
        Axis::from_index((self as u8 % 3) as usize)
    }

    /// Returns the opposite face (the face whose normal points in the
    /// opposite direction).
    #[inline]
    pub fn opposite(self) -> Face {
        Face::from_u8((self as u8 + 3) % 6)
    }

    /// Returns `true` if this is a positive face (0, 1, or 2), i.e. the
    /// face normal points along the positive axis direction.
    #[inline]
    pub fn is_positive(self) -> bool {
        (self as u8) < 3
    }
}

impl fmt::Display for Face {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_u8())
    }
}

impl From<Face> for u8 {
    #[inline]
    fn from(f: Face) -> u8 {
        f as u8
    }
}

impl TryFrom<u8> for Face {
    type Error = u8;
    /// Converts a `u8` to `Face`. Returns `Err(v)` if `v >= 6`.
    #[inline]
    fn try_from(v: u8) -> Result<Face, u8> {
        if v < 6 {
            Ok(Face::ALL[v as usize])
        } else {
            Err(v)
        }
    }
}

/// Maximum leaf-cell index + 1. Valid leaf-cell indices are `[0, LIMIT_IJ)`.
pub const LIMIT_IJ: i32 = 1 << MAX_CELL_LEVEL; // 2^30

/// Maximum value of an si- or ti-coordinate. Valid range is `[0, MAX_SI_TI]`.
pub const MAX_SI_TI: u32 = 1u32 << (MAX_CELL_LEVEL + 1); // 2^31

/// Maximum absolute error in UV coordinates when converting from XYZ.
pub const MAX_XYZ_TO_UV_ERROR: f64 = 0.5 * f64::EPSILON;

/// Machine epsilon for f64 (alias for `f64::EPSILON`).
pub const DBL_EPSILON: f64 = f64::EPSILON;

// ---------------------------------------------------------------------------
// ST ↔ UV conversions (quadratic projection)
// ---------------------------------------------------------------------------

/// Converts an s- or t-value in `[0, 1]` to the corresponding u- or v-value
/// in `[-1, 1]`. Uses the quadratic projection.
#[inline]
pub fn st_to_uv(s: f64) -> f64 {
    if s >= 0.5 {
        (1.0 / 3.0) * (4.0 * s * s - 1.0)
    } else {
        (1.0 / 3.0) * (1.0 - 4.0 * (1.0 - s) * (1.0 - s))
    }
}

/// Converts a u- or v-value in `[-1, 1]` to the corresponding s- or t-value
/// in `[0, 1]`. Inverse of [`st_to_uv`].
#[inline]
pub fn uv_to_st(u: f64) -> f64 {
    if u >= 0.0 {
        0.5 * (1.0 + 3.0 * u).sqrt()
    } else {
        1.0 - 0.5 * (1.0 - 3.0 * u).sqrt()
    }
}

// ---------------------------------------------------------------------------
// IJ ↔ ST conversions
// ---------------------------------------------------------------------------

/// Converts an i- or j-index of a leaf cell to the minimum s- or t-value
/// contained by that cell. The argument must be in `[0, LIMIT_IJ]`.
#[inline]
pub fn ij_to_st_min(i: i32) -> f64 {
    debug_assert!((0..=LIMIT_IJ).contains(&i));
    (1.0 / f64::from(LIMIT_IJ)) * f64::from(i)
}

/// Returns the i- or j-index of the leaf cell containing the given s- or
/// t-value. The result is clamped to `[0, LIMIT_IJ - 1]`.
#[inline]
pub fn st_to_ij(s: f64) -> i32 {
    debug_assert!(!s.is_nan());
    if s <= 0.0 || s.is_nan() {
        return 0;
    }
    (f64::from(LIMIT_IJ) * s).min(f64::from(LIMIT_IJ - 1)) as i32
}

// ---------------------------------------------------------------------------
// SiTi ↔ ST conversions
// ---------------------------------------------------------------------------

/// Converts an si- or ti-value to the corresponding s- or t-value.
#[inline]
pub fn si_ti_to_st(si: u32) -> f64 {
    debug_assert!(si <= MAX_SI_TI);
    (1.0 / f64::from(MAX_SI_TI)) * f64::from(si)
}

/// Returns the si- or ti-coordinate nearest to the given s- or t-value.
/// The result may be outside `[0, MAX_SI_TI]`.
#[inline]
pub fn st_to_si_ti(s: f64) -> u32 {
    (s * f64::from(MAX_SI_TI)).round() as u32
}

// ---------------------------------------------------------------------------
// Face / UV / XYZ conversions
// ---------------------------------------------------------------------------

/// Converts `(face, u, v)` coordinates to a direction vector (not necessarily
/// unit length).
#[inline]
pub fn face_uv_to_xyz(face: Face, u: f64, v: f64) -> Vector {
    match face {
        Face::F0 => Vector { x: 1.0, y: u, z: v },
        Face::F1 => Vector {
            x: -u,
            y: 1.0,
            z: v,
        },
        Face::F2 => Vector {
            x: -u,
            y: -v,
            z: 1.0,
        },
        Face::F3 => Vector {
            x: -1.0,
            y: -v,
            z: -u,
        },
        Face::F4 => Vector {
            x: v,
            y: -1.0,
            z: -u,
        },
        Face::F5 => Vector {
            x: v,
            y: u,
            z: -1.0,
        },
    }
}

/// Given a *valid* face for the given point (i.e. the dot product of `p` with
/// the face normal is positive), returns the corresponding `(u, v)` values.
#[inline]
pub fn valid_face_xyz_to_uv(face: Face, p: &Vector) -> (f64, f64) {
    debug_assert!(
        p.dot(get_norm(face)) > 0.0,
        "valid_face_xyz_to_uv: p does not belong to face {face}"
    );
    match face {
        Face::F0 => (p.y / p.x, p.z / p.x),
        Face::F1 => (-p.x / p.y, p.z / p.y),
        Face::F2 => (-p.x / p.z, -p.y / p.z),
        Face::F3 => (p.z / p.x, p.y / p.x),
        Face::F4 => (p.z / p.y, -p.x / p.y),
        Face::F5 => (-p.y / p.z, -p.x / p.z),
    }
}

/// If the dot product of `p` with the given face normal is positive, returns
/// `Some((u, v))`. Otherwise returns `None`.
#[inline]
pub fn face_xyz_to_uv(face: Face, p: &Vector) -> Option<(f64, f64)> {
    if face.is_positive() {
        if p[face.axis()] <= 0.0 {
            return None;
        }
    } else if p[face.axis()] >= 0.0 {
        return None;
    }
    Some(valid_face_xyz_to_uv(face, p))
}

/// Transforms the given point `p` to the `(u, v, w)` coordinate frame of the
/// given face (where the w-axis represents the face normal).
#[inline]
pub fn face_xyz_to_uvw(face: Face, p: &Vector) -> Vector {
    match face {
        Face::F0 => Vector {
            x: p.y,
            y: p.z,
            z: p.x,
        },
        Face::F1 => Vector {
            x: -p.x,
            y: p.z,
            z: p.y,
        },
        Face::F2 => Vector {
            x: -p.x,
            y: -p.y,
            z: p.z,
        },
        Face::F3 => Vector {
            x: -p.z,
            y: -p.y,
            z: -p.x,
        },
        Face::F4 => Vector {
            x: -p.z,
            y: p.x,
            z: -p.y,
        },
        Face::F5 => Vector {
            x: p.y,
            y: p.x,
            z: -p.z,
        },
    }
}

/// Returns the face (0–5) containing the given direction vector. For points on
/// the boundary between faces, the result is arbitrary but deterministic.
#[inline]
pub fn get_face(p: &Vector) -> Face {
    let axis = p.largest_abs_component();
    let positive = p[Axis::from_index(axis)] >= 0.0;
    Face::from_u8(if positive { axis as u8 } else { axis as u8 + 3 })
}

/// Converts a direction vector (not necessarily unit length) to
/// `(face, u, v)` coordinates.
#[inline]
pub fn xyz_to_face_uv(p: &Vector) -> (Face, f64, f64) {
    let face = get_face(p);
    let (u, v) = valid_face_xyz_to_uv(face, p);
    (face, u, v)
}

/// Converts a direction vector to `(face, si, ti)` coordinates and, if `p`
/// is exactly equal to the center of a cell, returns the level of that cell.
/// Otherwise returns `None`.
#[inline]
pub fn xyz_to_face_si_ti(p: &Vector) -> (Face, u32, u32, Option<Level>) {
    let (face, u, v) = xyz_to_face_uv(p);
    let si = st_to_si_ti(uv_to_st(u));
    let ti = st_to_si_ti(uv_to_st(v));

    // If the levels corresponding to si,ti are not equal, then p is not a cell
    // center. The si,ti values 0 and MAX_SI_TI need to be handled specially
    // because they do not correspond to cell centers at any valid level; they
    // are mapped to level > MAX_CELL_LEVEL by the code below.
    let trailing_si = (si | MAX_SI_TI).trailing_zeros();
    let trailing_ti = (ti | MAX_SI_TI).trailing_zeros();
    // Use wrapping subtraction: if trailing > MAX_CELL_LEVEL, the result
    // wraps to a large value that is > MAX_CELL_LEVEL, caught below.
    let level_si = u32::from(MAX_CELL_LEVEL).wrapping_sub(trailing_si);
    let level_ti = u32::from(MAX_CELL_LEVEL).wrapping_sub(trailing_ti);

    if level_si > u32::from(MAX_CELL_LEVEL) || level_si != level_ti {
        return (face, si, ti, None);
    }
    let level = Level::new(level_si as u8);

    // In infinite precision, this test could be ST == SiTi. However,
    // due to rounding errors, uv_to_st(xyz_to_face_uv(face_uv_to_xyz(st_to_uv(...)))) is
    // not idempotent. The center is computed exactly the same way p was
    // originally computed (if it is indeed the center of a cell), so the
    // comparison can be exact.
    let center = face_si_ti_to_xyz(face, si, ti).normalize();
    if *p == center {
        (face, si, ti, Some(level))
    } else {
        (face, si, ti, None)
    }
}

/// Converts `(face, si, ti)` coordinates to a direction vector (not
/// necessarily unit length).
#[inline]
pub fn face_si_ti_to_xyz(face: Face, si: u32, ti: u32) -> Vector {
    let u = st_to_uv(si_ti_to_st(si));
    let v = st_to_uv(si_ti_to_st(ti));
    face_uv_to_xyz(face, u, v)
}

// ---------------------------------------------------------------------------
// Edge normals and face axes
// ---------------------------------------------------------------------------

/// Returns the right-handed normal (not necessarily unit length) for an edge
/// in the direction of the positive v-axis at the given u-value on the given
/// face.
#[inline]
pub fn get_u_norm(face: Face, u: f64) -> Vector {
    match face {
        Face::F0 => Vector {
            x: u,
            y: -1.0,
            z: 0.0,
        },
        Face::F1 => Vector {
            x: 1.0,
            y: u,
            z: 0.0,
        },
        Face::F2 => Vector {
            x: 1.0,
            y: 0.0,
            z: u,
        },
        Face::F3 => Vector {
            x: -u,
            y: 0.0,
            z: 1.0,
        },
        Face::F4 => Vector {
            x: 0.0,
            y: -u,
            z: 1.0,
        },
        Face::F5 => Vector {
            x: 0.0,
            y: -1.0,
            z: -u,
        },
    }
}

/// Returns the right-handed normal (not necessarily unit length) for an edge
/// in the direction of the positive u-axis at the given v-value on the given
/// face.
#[inline]
pub fn get_v_norm(face: Face, v: f64) -> Vector {
    match face {
        Face::F0 => Vector {
            x: -v,
            y: 0.0,
            z: 1.0,
        },
        Face::F1 => Vector {
            x: 0.0,
            y: -v,
            z: 1.0,
        },
        Face::F2 => Vector {
            x: 0.0,
            y: -1.0,
            z: -v,
        },
        Face::F3 => Vector {
            x: v,
            y: -1.0,
            z: 0.0,
        },
        Face::F4 => Vector {
            x: 1.0,
            y: v,
            z: 0.0,
        },
        Face::F5 => Vector {
            x: 1.0,
            y: 0.0,
            z: v,
        },
    }
}

/// Returns the unit-length face normal for the given face.
#[inline]
pub fn get_norm(face: Face) -> Vector {
    get_uvw_axis(face, 2)
}

/// Returns the u-axis unit vector for the given face.
#[inline]
pub fn get_u_axis(face: Face) -> Vector {
    get_uvw_axis(face, 0)
}

/// Returns the v-axis unit vector for the given face.
#[inline]
pub fn get_v_axis(face: Face) -> Vector {
    get_uvw_axis(face, 1)
}

/// Returns the given axis of the given face (u=0, v=1, w=2).
#[inline]
pub fn get_uvw_axis(face: Face, axis: u8) -> Vector {
    let p = &FACE_UVW_AXES[face.as_u8() as usize][axis as usize];
    Vector {
        x: p[0],
        y: p[1],
        z: p[2],
    }
}

/// Returns the face that lies in the given direction (negative=0, positive=1)
/// of the given axis (u=0, v=1, w=2) relative to the given face.
#[inline]
pub fn get_uvw_face(face: Face, axis: u8, direction: u8) -> Face {
    debug_assert!(axis < 3 && direction < 2);
    Face::from_u8(FACE_UVW_FACES[face.as_u8() as usize][axis as usize][direction as usize])
}

// ---------------------------------------------------------------------------
// Hilbert curve constants
// ---------------------------------------------------------------------------

/// Flag for axis swapping in Hilbert curve orientations.
pub const SWAP_MASK: u8 = 0x01;

/// Flag for 180-degree rotation in Hilbert curve orientations.
pub const INVERT_MASK: u8 = 0x02;

/// Maps `(orientation, ij)` to Hilbert curve position `[0..3]`.
pub const IJ_TO_POS: [[u8; 4]; 4] = [
    [0, 1, 3, 2], // canonical order
    [0, 3, 1, 2], // axes swapped
    [2, 3, 1, 0], // bits inverted
    [2, 1, 3, 0], // swapped & inverted
];

/// Maps `(orientation, pos)` to `(i,j)` index. Inverse of [`IJ_TO_POS`].
pub const POS_TO_IJ: [[u8; 4]; 4] = [
    [0, 1, 3, 2], // canonical order:    (0,0), (0,1), (1,1), (1,0)
    [0, 2, 3, 1], // axes swapped:       (0,0), (1,0), (1,1), (0,1)
    [3, 2, 0, 1], // bits inverted:      (1,1), (1,0), (0,0), (0,1)
    [3, 1, 0, 2], // swapped & inverted: (1,1), (0,1), (0,0), (1,0)
];

/// Orientation modifiers for child cells, indexed by position `[0..3]`.
pub const POS_TO_ORIENTATION: [u8; 4] = [SWAP_MASK, 0, 0, INVERT_MASK + SWAP_MASK];

// ---------------------------------------------------------------------------
// Lookup tables
// ---------------------------------------------------------------------------

/// The U, V, W axes for each face.
const FACE_UVW_AXES: [[[f64; 3]; 3]; 6] = [
    [[0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]],
    [[-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]],
    [[-1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0]],
    [[0.0, 0.0, -1.0], [0.0, -1.0, 0.0], [-1.0, 0.0, 0.0]],
    [[0.0, 0.0, -1.0], [1.0, 0.0, 0.0], [0.0, -1.0, 0.0]],
    [[0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0]],
];

/// Precomputed neighbor faces: `[face][axis][direction]`.
const FACE_UVW_FACES: [[[u8; 2]; 3]; 6] = [
    [[4, 1], [5, 2], [3, 0]],
    [[0, 3], [5, 2], [4, 1]],
    [[0, 3], [1, 4], [5, 2]],
    [[2, 5], [1, 4], [0, 3]],
    [[2, 5], [3, 0], [1, 4]],
    [[4, 1], [3, 0], [2, 5]],
];

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn float64_eq(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-15
    }

    fn float64_near(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() <= eps
    }

    // -- Hilbert curve tables --

    fn swap_axes(ij: u8) -> u8 {
        ((ij >> 1) & 1) + ((ij & 1) << 1)
    }

    fn invert_bits(ij: u8) -> u8 {
        ij ^ 3
    }

    #[test]
    fn test_traversal_order() {
        for r in 0..4u8 {
            for i in 0..4u8 {
                // Check consistency with respect to swapping axes.
                assert_eq!(
                    IJ_TO_POS[r as usize][i as usize],
                    IJ_TO_POS[(r ^ SWAP_MASK) as usize][swap_axes(i) as usize],
                );
                assert_eq!(
                    POS_TO_IJ[r as usize][i as usize],
                    swap_axes(POS_TO_IJ[(r ^ SWAP_MASK) as usize][i as usize]),
                );

                // Check consistency with respect to reversing axis directions.
                assert_eq!(
                    IJ_TO_POS[r as usize][i as usize],
                    IJ_TO_POS[(r ^ INVERT_MASK) as usize][invert_bits(i) as usize],
                );
                assert_eq!(
                    POS_TO_IJ[r as usize][i as usize],
                    invert_bits(POS_TO_IJ[(r ^ INVERT_MASK) as usize][i as usize]),
                );

                // Check that the two tables are inverses of each other.
                assert_eq!(
                    IJ_TO_POS[r as usize][POS_TO_IJ[r as usize][i as usize] as usize],
                    i,
                );
                assert_eq!(
                    POS_TO_IJ[r as usize][IJ_TO_POS[r as usize][i as usize] as usize],
                    i,
                );
            }
        }
    }

    // -- ST ↔ UV --

    #[test]
    fn test_st_uv_conversions() {
        // Check boundary conditions.
        for &s in &[0.0, 0.5, 1.0] {
            let u = st_to_uv(s);
            let want = 2.0 * s - 1.0;
            assert!(float64_eq(u, want), "st_to_uv({s}) = {u}, want {want}",);
        }
        for &u in &[-1.0, 0.0, 1.0] {
            let s = uv_to_st(u);
            let want = 0.5 * (u + 1.0);
            assert!(float64_eq(s, want), "uv_to_st({u}) = {s}, want {want}",);
        }

        // Check that uv_to_st and st_to_uv are inverses.
        let mut x = 0.0;
        while x <= 1.0 {
            let got = uv_to_st(st_to_uv(x));
            assert!(
                float64_near(got, x, 1e-15),
                "uv_to_st(st_to_uv({x})) = {got}, want {x}",
            );
            let u = 2.0 * x - 1.0;
            let got2 = st_to_uv(uv_to_st(u));
            assert!(
                float64_near(got2, u, 1e-15),
                "st_to_uv(uv_to_st({u})) = {got2}, want {u}",
            );
            x += 0.0001;
        }
    }

    // -- IJ ↔ ST --

    #[test]
    fn test_st_to_ij_boundaries() {
        assert_eq!(st_to_ij(0.0), 0);
        assert_eq!(st_to_ij(1.0), LIMIT_IJ - 1);
    }

    #[test]
    fn test_st_to_ij_halfway() {
        let recip = 1.0 / f64::from(LIMIT_IJ);
        assert_eq!(st_to_ij(0.5 * recip), 0);
        assert_eq!(st_to_ij(1.0 * recip), 1);
        assert_eq!(st_to_ij(1.5 * recip), 1);
        assert_eq!(st_to_ij(2.0 * recip), 2);
        assert_eq!(st_to_ij((f64::from(LIMIT_IJ) - 0.5) * recip), LIMIT_IJ - 1);
    }

    #[test]
    fn test_si_ti_st_roundtrip() {
        // int -> float -> int direction
        // Test specific boundary values and a spread of values.
        for si in [0u32, 1, 2, MAX_SI_TI / 2, MAX_SI_TI - 1, MAX_SI_TI] {
            assert_eq!(
                st_to_si_ti(si_ti_to_st(si)),
                si,
                "roundtrip failed for si={si}",
            );
        }

        // float -> int -> float direction
        let mut s = 0.0;
        while s <= 1.0 {
            let got = si_ti_to_st(st_to_si_ti(s));
            assert!(
                float64_near(got, s, 1e-8),
                "si_ti_to_st(st_to_si_ti({s})) = {got}, want ≈{s}",
            );
            s += 0.001;
        }
    }

    // -- Face UV / XYZ conversions --

    #[test]
    fn test_face_uv_to_xyz() {
        let mut sum = Vector::default();
        for face in Face::iter() {
            let center = face_uv_to_xyz(face, 0.0, 0.0);
            assert!(
                center.aequal(get_norm(face), 1e-15),
                "face_uv_to_xyz({face}, 0, 0) = {center:?}, want ≈ {:?}",
                get_norm(face),
            );
            // The center should have exactly one component with abs value 1.
            let abs_center = center.abs();
            match center.largest_abs_component() {
                0 => assert_eq!(abs_center.x, 1.0),
                1 => assert_eq!(abs_center.y, 1.0),
                _ => assert_eq!(abs_center.z, 1.0),
            }
            sum = sum + center.abs();

            // Check right-handed coordinate system.
            assert_eq!(
                get_u_axis(face).cross(get_v_axis(face)).dot(get_norm(face)),
                1.0,
                "face {face} not right-handed",
            );

            // Check Hilbert curve continuity across faces.
            let sign: f64 = if face.as_u8() & SWAP_MASK == 1 {
                -1.0
            } else {
                1.0
            };
            let next_face = Face::from_u8((face.as_u8() + 1) % 6);
            assert_eq!(
                face_uv_to_xyz(face, sign, -sign),
                face_uv_to_xyz(next_face, -1.0, -1.0),
                "Hilbert curve discontinuity at face {face}",
            );
        }

        // Sum of absolute face normals should be (2,2,2).
        assert!(sum.aequal(
            Vector {
                x: 2.0,
                y: 2.0,
                z: 2.0
            },
            1e-15
        ));
    }

    #[test]
    fn test_face_xyz_to_uv() {
        let point = Vector {
            x: 1.1,
            y: 1.2,
            z: 1.3,
        };
        let point_neg = Vector {
            x: -1.1,
            y: -1.2,
            z: -1.3,
        };

        let cases: Vec<(Face, Vector, Option<(f64, f64)>)> = vec![
            (Face::F0, point, Some((1.0 + 1.0 / 11.0, 1.0 + 2.0 / 11.0))),
            (Face::F0, point_neg, None),
            (Face::F1, point, Some((-11.0 / 12.0, 1.0 + 1.0 / 12.0))),
            (Face::F1, point_neg, None),
            (Face::F2, point, Some((-11.0 / 13.0, -12.0 / 13.0))),
            (Face::F2, point_neg, None),
            (Face::F3, point, None),
            (
                Face::F3,
                point_neg,
                Some((1.0 + 2.0 / 11.0, 1.0 + 1.0 / 11.0)),
            ),
            (Face::F4, point, None),
            (
                Face::F4,
                point_neg,
                Some((1.0 + 1.0 / 12.0, -(11.0 / 12.0))),
            ),
            (Face::F5, point, None),
            (Face::F5, point_neg, Some((-12.0 / 13.0, -11.0 / 13.0))),
        ];

        for (face, p, expected) in &cases {
            let got = face_xyz_to_uv(*face, p);
            match (got, expected) {
                (None, None) => {}
                (Some((gu, gv)), Some((eu, ev))) => {
                    assert!(
                        float64_eq(gu, *eu) && float64_eq(gv, *ev),
                        "face_xyz_to_uv({face}, {p:?}) = ({gu}, {gv}), want ({eu}, {ev})",
                    );
                }
                _ => panic!("face_xyz_to_uv({face}, {p:?}) = {got:?}, want {expected:?}",),
            }
        }
    }

    #[test]
    fn test_face_xyz_to_uvw() {
        let origin = Vector::default();
        let pos_x = Vector {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        };
        let neg_x = Vector {
            x: -1.0,
            y: 0.0,
            z: 0.0,
        };
        let pos_y = Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        };
        let neg_y = Vector {
            x: 0.0,
            y: -1.0,
            z: 0.0,
        };
        let pos_z = Vector {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        };
        let neg_z = Vector {
            x: 0.0,
            y: 0.0,
            z: -1.0,
        };

        for face in Face::iter() {
            assert_eq!(face_xyz_to_uvw(face, &origin), origin);
            assert_eq!(face_xyz_to_uvw(face, &get_u_axis(face)), pos_x);
            assert_eq!(face_xyz_to_uvw(face, &(-get_u_axis(face))), neg_x);
            assert_eq!(face_xyz_to_uvw(face, &get_v_axis(face)), pos_y);
            assert_eq!(face_xyz_to_uvw(face, &(-get_v_axis(face))), neg_y);
            assert_eq!(face_xyz_to_uvw(face, &get_norm(face)), pos_z);
            assert_eq!(face_xyz_to_uvw(face, &(-get_norm(face))), neg_z);
        }
    }

    #[test]
    fn test_uvw_axis() {
        for face in Face::iter() {
            // Check that axes are consistent with face_uv_to_xyz.
            assert_eq!(
                face_uv_to_xyz(face, 1.0, 0.0) - face_uv_to_xyz(face, 0.0, 0.0),
                get_u_axis(face),
            );
            assert_eq!(
                face_uv_to_xyz(face, 0.0, 1.0) - face_uv_to_xyz(face, 0.0, 0.0),
                get_v_axis(face),
            );
            assert_eq!(face_uv_to_xyz(face, 0.0, 0.0), get_norm(face));

            // Check right-handed coordinate frame.
            assert_eq!(
                get_u_axis(face).cross(get_v_axis(face)).dot(get_norm(face)),
                1.0,
            );

            // Check consistency of getters.
            assert_eq!(get_u_axis(face), get_uvw_axis(face, 0));
            assert_eq!(get_v_axis(face), get_uvw_axis(face, 1));
            assert_eq!(get_norm(face), get_uvw_axis(face, 2));
        }
    }

    #[test]
    fn test_uv_norms() {
        let step = 1.0 / 1024.0;
        for face in Face::iter() {
            let mut x = -1.0;
            while x <= 1.0 {
                // UNorm should be orthogonal to the face.
                let u_angle = face_uv_to_xyz(face, x, -1.0)
                    .cross(face_uv_to_xyz(face, x, 1.0))
                    .angle(get_u_norm(face, x));
                assert!(
                    float64_eq(u_angle, 0.0),
                    "UNorm not orthogonal at face={face}, x={x}: angle={u_angle}",
                );

                // VNorm should be orthogonal to the face.
                let v_angle = face_uv_to_xyz(face, -1.0, x)
                    .cross(face_uv_to_xyz(face, 1.0, x))
                    .angle(get_v_norm(face, x));
                assert!(
                    float64_eq(v_angle, 0.0),
                    "VNorm not orthogonal at face={face}, x={x}: angle={v_angle}",
                );

                x += step;
            }
        }
    }

    #[test]
    fn test_uvw_face() {
        // Check that get_uvw_face is consistent with get_uvw_axis.
        for face in Face::iter() {
            for axis in 0..3u8 {
                assert_eq!(
                    get_face(&(-get_uvw_axis(face, axis))),
                    get_uvw_face(face, axis, 0),
                );
                assert_eq!(
                    get_face(&get_uvw_axis(face, axis)),
                    get_uvw_face(face, axis, 1),
                );
            }
        }
    }

    #[test]
    fn test_get_face() {
        // 27 test vectors across all octants (from Go TestSTUVFace).
        let cases: Vec<(Vector, u8)> = vec![
            (
                Vector {
                    x: -1.0,
                    y: -1.0,
                    z: -1.0,
                },
                5,
            ),
            (
                Vector {
                    x: -1.0,
                    y: -1.0,
                    z: 0.0,
                },
                4,
            ),
            (
                Vector {
                    x: -1.0,
                    y: -1.0,
                    z: 1.0,
                },
                2,
            ),
            (
                Vector {
                    x: -1.0,
                    y: 0.0,
                    z: -1.0,
                },
                5,
            ),
            (
                Vector {
                    x: -1.0,
                    y: 0.0,
                    z: 0.0,
                },
                3,
            ),
            (
                Vector {
                    x: -1.0,
                    y: 0.0,
                    z: 1.0,
                },
                2,
            ),
            (
                Vector {
                    x: -1.0,
                    y: 1.0,
                    z: -1.0,
                },
                5,
            ),
            (
                Vector {
                    x: -1.0,
                    y: 1.0,
                    z: 0.0,
                },
                1,
            ),
            (
                Vector {
                    x: -1.0,
                    y: 1.0,
                    z: 1.0,
                },
                2,
            ),
            (
                Vector {
                    x: 0.0,
                    y: -1.0,
                    z: -1.0,
                },
                5,
            ),
            (
                Vector {
                    x: 0.0,
                    y: -1.0,
                    z: 0.0,
                },
                4,
            ),
            (
                Vector {
                    x: 0.0,
                    y: -1.0,
                    z: 1.0,
                },
                2,
            ),
            (
                Vector {
                    x: 0.0,
                    y: 0.0,
                    z: -1.0,
                },
                5,
            ),
            (
                Vector {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                2,
            ), // tie-break: z >= 0
            (
                Vector {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                },
                2,
            ),
            (
                Vector {
                    x: 0.0,
                    y: 1.0,
                    z: -1.0,
                },
                5,
            ),
            (
                Vector {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
                1,
            ),
            (
                Vector {
                    x: 0.0,
                    y: 1.0,
                    z: 1.0,
                },
                2,
            ),
            (
                Vector {
                    x: 1.0,
                    y: -1.0,
                    z: -1.0,
                },
                5,
            ),
            (
                Vector {
                    x: 1.0,
                    y: -1.0,
                    z: 0.0,
                },
                4,
            ),
            (
                Vector {
                    x: 1.0,
                    y: -1.0,
                    z: 1.0,
                },
                2,
            ),
            (
                Vector {
                    x: 1.0,
                    y: 0.0,
                    z: -1.0,
                },
                5,
            ),
            (
                Vector {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                0,
            ),
            (
                Vector {
                    x: 1.0,
                    y: 0.0,
                    z: 1.0,
                },
                2,
            ),
            (
                Vector {
                    x: 1.0,
                    y: 1.0,
                    z: -1.0,
                },
                5,
            ),
            (
                Vector {
                    x: 1.0,
                    y: 1.0,
                    z: 0.0,
                },
                1,
            ),
            (
                Vector {
                    x: 1.0,
                    y: 1.0,
                    z: 1.0,
                },
                2,
            ),
        ];

        for (v, want) in &cases {
            assert_eq!(
                get_face(v),
                Face::from_u8(*want),
                "get_face({v:?}) = {}, want {want}",
                get_face(v),
            );
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_level_roundtrip() {
        for l in 0..=MAX_CELL_LEVEL {
            let level = Level::new(l);
            let json = serde_json::to_string(&level).unwrap();
            let back: Level = serde_json::from_str(&json).unwrap();
            assert_eq!(level, back);
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_face_roundtrip() {
        for f in Face::ALL {
            let json = serde_json::to_string(&f).unwrap();
            let back: Face = serde_json::from_str(&json).unwrap();
            assert_eq!(f, back);
        }
    }
}

#[cfg(test)]
mod quickcheck_tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    fn clamp_finite(v: f64) -> f64 {
        if v.is_finite() {
            v.clamp(-1e10, 1e10)
        } else {
            0.0
        }
    }

    #[quickcheck]
    fn prop_uv_st_roundtrip(s: f64) -> bool {
        // uv_to_st(st_to_uv(s)) ≈ s for s in [0, 1]
        let s = clamp_finite(s).clamp(0.0, 1.0);
        let got = uv_to_st(st_to_uv(s));
        (got - s).abs() < 1e-15
    }

    #[quickcheck]
    fn prop_st_uv_roundtrip(u: f64) -> bool {
        // st_to_uv(uv_to_st(u)) ≈ u for u in [-1, 1]
        let u = clamp_finite(u).clamp(-1.0, 1.0);
        let got = st_to_uv(uv_to_st(u));
        (got - u).abs() < 1e-15
    }

    #[quickcheck]
    fn prop_face_uv_xyz_roundtrip(x: f64, y: f64, z: f64) -> bool {
        // face_uv_to_xyz followed by xyz_to_face_uv roundtrips.
        let x = clamp_finite(x);
        let y = clamp_finite(y);
        let z = clamp_finite(z);
        // Need a non-zero vector.
        if x == 0.0 && y == 0.0 && z == 0.0 {
            return true;
        }
        let p = Vector { x, y, z };
        let (face, u, v) = xyz_to_face_uv(&p);
        let q = face_uv_to_xyz(face, u, v);
        // q should be a positive scalar multiple of p.
        let scale = if p.x.abs() > p.y.abs().max(p.z.abs()) {
            q.x / p.x
        } else if p.y.abs() > p.z.abs() {
            q.y / p.y
        } else {
            q.z / p.z
        };
        scale > 0.0
            && (q.x - scale * p.x).abs() < 1e-10
            && (q.y - scale * p.y).abs() < 1e-10
            && (q.z - scale * p.z).abs() < 1e-10
    }

    #[quickcheck]
    fn prop_all_faces_reachable(x: f64, y: f64, z: f64) -> bool {
        // get_face always returns a value in [0, 5].
        let x = clamp_finite(x);
        let y = clamp_finite(y);
        let z = clamp_finite(z);
        let p = Vector { x, y, z };
        get_face(&p).as_u8() < 6
    }

    #[quickcheck]
    fn prop_ij_st_roundtrip(i: u32) -> bool {
        // ij_to_st_min → st_to_ij roundtrips for valid indices.
        let i = (i % (LIMIT_IJ as u32)) as i32;
        let s = ij_to_st_min(i);
        let j = st_to_ij(s);
        // s is the minimum s for cell i, so st_to_ij(s) should give i
        // (or i-1 at boundaries due to floating-point, but ij_to_st_min(i) maps to exact i).
        (j - i).abs() <= 1
    }

    #[quickcheck]
    fn prop_si_ti_st_roundtrip(si: u32) -> bool {
        // si_ti_to_st → st_to_si_ti roundtrips for valid values.
        let si = si % (MAX_SI_TI + 1);
        let s = si_ti_to_st(si);
        let si2 = st_to_si_ti(s);
        si == si2
    }

    #[quickcheck]
    fn prop_st_to_uv_monotonic(a: f64, b: f64) -> bool {
        // st_to_uv preserves ordering within [0, 1].
        let a = clamp_finite(a).clamp(0.0, 1.0);
        let b = clamp_finite(b).clamp(0.0, 1.0);
        if a <= b {
            st_to_uv(a) <= st_to_uv(b)
        } else {
            st_to_uv(a) >= st_to_uv(b)
        }
    }

    #[quickcheck]
    fn prop_uv_to_st_range(u: f64) -> bool {
        // uv_to_st maps [-1, 1] → [0, 1]
        let u = clamp_finite(u).clamp(-1.0, 1.0);
        let s = uv_to_st(u);
        (0.0..=1.0).contains(&s)
    }

    #[quickcheck]
    fn prop_face_axes_orthogonal(face: u8) -> bool {
        let face = Face::from_u8(face % 6);
        let u = get_u_axis(face);
        let v = get_v_axis(face);
        let n = get_norm(face);
        // All three axes should be mutually orthogonal.
        u.dot(v).abs() < 1e-14 && u.dot(n).abs() < 1e-14 && v.dot(n).abs() < 1e-14
    }

    #[quickcheck]
    fn prop_face_normals_unit(face: u8) -> bool {
        let face = Face::from_u8(face % 6);
        let n = get_norm(face);
        (n.norm() - 1.0).abs() < 1e-14
    }

    // ─── C++ IJtoSTtoIJRoundtripRandom equivalent ───

    #[test]
    fn test_ij_to_st_to_ij_roundtrip() {
        // C++ IJtoSTtoIJRoundtripRandom: for each leaf cell i, a random s in
        // [s_min, s_max) should round-trip back to i.
        use rand::rngs::StdRng;
        use rand::{Rng, SeedableRng};
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..1000 {
            let i = (rng.r#gen::<u32>() % LIMIT_IJ as u32) as i32;
            let s_min = ij_to_st_min(i);
            let s_max = ij_to_st_min(i + 1);
            // Random s in [s_min, s_max).
            let t: f64 = rng.r#gen();
            let s = s_min + t * (s_max - s_min);
            let s = s.min(f64::from_bits(s_max.to_bits() - 1)); // < s_max
            assert_eq!(st_to_ij(s), i, "s={s}, i={i}");
            assert_eq!(st_to_ij(s_min), i, "s_min={s_min}, i={i}");
            // Just below s_max should also map to i.
            let before_s_max = f64::from_bits(s_max.to_bits() - 1);
            assert_eq!(
                st_to_ij(before_s_max),
                i,
                "before_s_max={before_s_max}, i={i}"
            );
        }
    }

    // ─── C++ XYZToFaceSiTi equivalent ───

    #[test]
    fn test_xyz_to_face_si_ti() {
        use crate::s2::testing::random_cell_id_at_level;
        use rand::SeedableRng;
        use rand::rngs::StdRng;
        let mut rng = StdRng::seed_from_u64(123);

        // Check the conversion of random cells to center points and back.
        for level_u8 in 0..=MAX_CELL_LEVEL {
            let level = Level::new(level_u8);
            for _ in 0..100 {
                let id = random_cell_id_at_level(&mut rng, level);
                let p = id.to_point();
                let (face, si, ti, actual_level) = xyz_to_face_si_ti(&p.0);
                assert_eq!(
                    actual_level,
                    Some(level),
                    "level mismatch for {id:?} at level {level}"
                );
                let actual_id =
                    crate::s2::cell_id::from_face_ij(face, (si / 2) as i32, (ti / 2) as i32)
                        .parent_at_level(level);
                assert_eq!(id, actual_id);

                // Test a point near the cell center but not equal to it.
                let p_moved = crate::s2::Point(p.0 + Vector::new(1e-13, 1e-13, 1e-13)).normalize();
                let (face_moved, si_moved, ti_moved, level_moved) = xyz_to_face_si_ti(&p_moved.0);
                assert_eq!(
                    level_moved, None,
                    "moved point should not match a cell center"
                );
                assert_eq!(face, face_moved);
                assert_eq!(si, si_moved);
                assert_eq!(ti, ti_moved);
            }
        }
    }
}
