// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Reference model of the S2 cell hierarchy (data-model §1, research R3).
//!
//! The model is written from the bit layout documented in upstream `s2cell_id.h` and the
//! Hilbert-curve tables of upstream `s2coords_internal.h`. It is deliberately independent of
//! `core/src/s2/cell_id.rs` (FR-006):
//!
//! - an id is decomposed into `(face, level, path)` by **searching** the 31 levels for the
//!   marker bit, never with `trailing_zeros` or `x & x.wrapping_neg()`;
//! - hierarchy, range and traversal relations are arithmetic on the path and leaf index;
//! - the Hilbert curve is walked one level at a time, not through a 4-level lookup table.
//!
//! It is plain Rust over `u64`, so `cargo test` checks it against the committed C++ oracle
//! vectors (`tests/spec_vs_cpp.rs`) and the Kani harnesses compare the implementation to it.

/// The finest level (leaf cells).
pub const MAX_LEVEL: u8 = 30;

/// The id of `CellId::none()`.
pub const NONE: u64 = 0;

/// The id of `CellId::sentinel()`.
pub const SENTINEL: u64 = u64::MAX;

// ---------------------------------------------------------------------------------------------
// Bit layout (T005)
// ---------------------------------------------------------------------------------------------

/// A cell identifier split into its face, level and Hilbert path (base-4 digits, most
/// significant first).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decomposed {
    /// Cube face, `0..=5`.
    pub face: u8,
    /// Subdivision level, `0..=30`.
    pub level: u8,
    /// The `level` child positions, as a base-4 number below `4^level`.
    pub path: u64,
}

impl Decomposed {
    /// `face ≤ 5`, `level ≤ 30` and `path < 4^level`.
    #[must_use]
    pub fn is_well_formed(self) -> bool {
        self.face <= 5 && self.level <= MAX_LEVEL && self.path < pow4(u32::from(self.level))
    }
}

/// `4^n`, for `n ≤ 31`.
#[must_use]
pub const fn pow4(n: u32) -> u64 {
    1u64 << (2 * n)
}

/// The id of a well-formed decomposition:
/// `face << 61 | path << (61 − 2·level) | 1 << (60 − 2·level)`.
#[must_use]
pub fn compose(d: Decomposed) -> u64 {
    let l = u32::from(d.level);
    (u64::from(d.face) << 61) | (d.path << (61 - 2 * l)) | (1u64 << (60 - 2 * l))
}

/// The inverse of [`compose`] on its image, found by searching the 31 levels for the marker
/// bit: the level whose marker bit `60 − 2·level` is set with every lower bit clear. Any value
/// outside the image (no such level, or a face above 5) is `None`: that is the definition of
/// an invalid id.
#[must_use]
pub fn decompose(x: u64) -> Option<Decomposed> {
    let mut level = 0u8;
    while level <= MAX_LEVEL {
        let marker = 60 - 2 * u32::from(level);
        let below = (1u64 << marker) - 1;
        if (x >> marker) & 1 == 1 && x & below == 0 {
            let face = (x >> 61) as u8;
            if face > 5 {
                return None;
            }
            let l = u32::from(level);
            let path = (x >> (61 - 2 * l)) & (pow4(l) - 1);
            return Some(Decomposed { face, level, path });
        }
        level += 1;
    }
    None
}

/// Whether `x` is a valid cell id.
#[must_use]
pub fn valid(x: u64) -> bool {
    decompose(x).is_some()
}

/// The marker bit of a cell at `level`: `1 << (60 − 2·level)`.
#[must_use]
pub fn lsb_for_level(level: u8) -> u64 {
    1u64 << (60 - 2 * u32::from(level))
}

/// The edge length of a cell at `level` in leaf `(i, j)` units: `1 << (30 − level)`.
#[must_use]
pub fn size_ij(level: u8) -> u32 {
    1u32 << (30 - u32::from(level))
}

/// One past the last cell of face 5 at `level`: `6 << 61 | 1 << (60 − 2·level)`.
#[must_use]
pub fn end(level: u8) -> u64 {
    (6u64 << 61) | lsb_for_level(level)
}

// ---------------------------------------------------------------------------------------------
// Hierarchy, ranges, positions, common ancestor (T006)
// ---------------------------------------------------------------------------------------------

/// The parent of `d`, for `level ≥ 1`.
#[must_use]
pub fn parent(d: Decomposed) -> Decomposed {
    Decomposed {
        face: d.face,
        level: d.level - 1,
        path: d.path / 4,
    }
}

/// Child `k` of `d`, for `level ≤ 29` and `k < 4`.
#[must_use]
pub fn child(d: Decomposed, k: u8) -> Decomposed {
    Decomposed {
        face: d.face,
        level: d.level + 1,
        path: 4 * d.path + u64::from(k),
    }
}

/// The ancestor of `d` at level `l ≤ d.level`: the path divided by `4^(level − l)`.
///
/// Multiplying or dividing by `pow4(k)` is written as a shift by `2k` throughout the model:
/// the same value, but CBMC encodes a shift by a symbolic amount far more cheaply than a
/// symbolic 64-bit divide or multiply (research R15).
#[must_use]
pub fn ancestor(d: Decomposed, l: u8) -> Decomposed {
    Decomposed {
        face: d.face,
        level: l,
        path: d.path >> (2 * u32::from(d.level - l)),
    }
}

/// Whether `a` contains `b`: same face, `a` no finer than `b`, and `a` is `b`'s ancestor at
/// `a`'s level (the prefix relation on paths).
#[must_use]
pub fn contains(a: Decomposed, b: Decomposed) -> bool {
    a.face == b.face && a.level <= b.level && ancestor(b, a.level) == a
}

/// The child position of `d`'s ancestor at level `l`, for `1 ≤ l ≤ level`.
#[must_use]
pub fn child_position(d: Decomposed, l: u8) -> u8 {
    ((d.path >> (2 * u32::from(d.level - l))) & 3) as u8
}

/// The index along the whole curve of the first leaf cell of `d`.
#[must_use]
pub fn leaf_index(d: Decomposed) -> u64 {
    (u64::from(d.face) << 60) + (d.path << (2 * u32::from(MAX_LEVEL - d.level)))
}

/// The number of leaf cells covered by a cell at `level`.
#[must_use]
pub fn span(level: u8) -> u64 {
    pow4(u32::from(MAX_LEVEL - level))
}

/// The inclusive leaf-index interval covered by `d`.
#[must_use]
pub fn range(d: Decomposed) -> (u64, u64) {
    let lo = leaf_index(d);
    (lo, lo + span(d.level) - 1)
}

/// The id of the leaf cell with leaf index `li` (`li < 6·4^30`).
#[must_use]
pub fn leaf_id(li: u64) -> u64 {
    compose(Decomposed {
        face: (li >> 60) as u8,
        level: MAX_LEVEL,
        path: li & (pow4(30) - 1),
    })
}

/// The position of `d` along the curve at its own level.
#[must_use]
pub fn position(d: Decomposed) -> u64 {
    (u64::from(d.face) << (2 * u32::from(d.level))) + d.path
}

/// The id at curve position `p` of `level`, for `p ≤ 6·4^level`; `p == 6·4^level` gives
/// [`end`]`(level)`.
#[must_use]
pub fn from_position(level: u8, p: u64) -> u64 {
    let n = pow4(u32::from(level));
    if p == 6 * n {
        return end(level);
    }
    compose(Decomposed {
        face: (p >> (2 * u32::from(level))) as u8, // p / 4^level
        level,
        path: p & (n - 1), // p % 4^level
    })
}

/// The finest level at which `a` and `b` share an ancestor; `None` on different faces.
#[must_use]
pub fn common_ancestor_level(a: Decomposed, b: Decomposed) -> Option<u8> {
    if a.face != b.face {
        return None;
    }
    let mut l = a.level.min(b.level);
    loop {
        if ancestor(a, l) == ancestor(b, l) {
            return Some(l);
        }
        // Level 0 always matches on the same face, so this never underflows.
        l -= 1;
    }
}

// ---------------------------------------------------------------------------------------------
// Hilbert curve and text encodings (T007)
// ---------------------------------------------------------------------------------------------

/// Orientation bit: the i and j axes are swapped.
pub const SWAP: u8 = 1;
/// Orientation bit: the curve direction is inverted.
pub const INVERT: u8 = 2;

/// `kPosToIJ[orientation][pos] -> ij` (upstream `s2coords_internal.h`), `ij = i << 1 | j`.
pub const POS_TO_IJ: [[u8; 4]; 4] = [[0, 1, 3, 2], [0, 2, 3, 1], [3, 2, 0, 1], [3, 1, 0, 2]];

/// `kIJtoPos[orientation][ij] -> pos` (upstream `s2coords_internal.h`).
pub const IJ_TO_POS: [[u8; 4]; 4] = [[0, 1, 3, 2], [0, 3, 1, 2], [2, 3, 1, 0], [2, 1, 3, 0]];

/// `kPosToOrientation[pos]`: XOR-ed into the parent orientation to get the child's
/// (upstream: `{kSwapMask, 0, 0, kInvertMask + kSwapMask}`).
pub const POS_TO_ORIENTATION: [u8; 4] = [SWAP, 0, 0, INVERT + SWAP];

/// The 30 digits that `ToFaceIJOrientation` walks: the cell's own digits, then for a
/// non-leaf cell the suffix digit 2 (the marker `10`) followed by zeros.
fn walk_digit(d: Decomposed, k: u8) -> u8 {
    if k <= d.level {
        child_position(d, k)
    } else if k == d.level + 1 {
        2
    } else {
        0
    }
}

/// Upstream `S2CellId::ToFaceIJOrientation`: the face, the `(i, j)` of the leaf cell reached
/// by walking all 30 digits (own digits, then `2`, then zeros), and the orientation reached
/// after the cell's own digits.
#[must_use]
pub fn face_ij_orientation(d: Decomposed) -> (u8, u32, u32, u8) {
    let mut o = d.face & SWAP;
    let mut own = o;
    let (mut i, mut j) = (0u32, 0u32);
    let mut k = 1u8;
    while k <= MAX_LEVEL {
        let digit = walk_digit(d, k);
        let ij = POS_TO_IJ[usize::from(o)][usize::from(digit)];
        i = (i << 1) | u32::from(ij >> 1);
        j = (j << 1) | u32::from(ij & 1);
        o ^= POS_TO_ORIENTATION[usize::from(digit)];
        if k == d.level {
            own = o;
        }
        k += 1;
    }
    (d.face, i, j, own)
}

/// The id of the leaf cell at `(i, j)` on `face`, for `i, j < 2^30` (upstream `FromFaceIJ`).
#[must_use]
pub fn leaf_from_face_ij(face: u8, i: u32, j: u32) -> u64 {
    let mut o = face & SWAP;
    let mut path = 0u64;
    let mut k = 30u32;
    while k > 0 {
        k -= 1;
        let ij = (((i >> k) & 1) << 1) | ((j >> k) & 1);
        let digit = IJ_TO_POS[usize::from(o)][ij as usize];
        path = (path << 2) | u64::from(digit);
        o ^= POS_TO_ORIENTATION[usize::from(digit)];
    }
    compose(Decomposed {
        face,
        level: MAX_LEVEL,
        path,
    })
}

/// Upstream `GetCenterSiTi`: the cell centre in `(si, ti)` units (twice the leaf `(i, j)`
/// resolution), computed from the cell's lower-left corner and size.
#[must_use]
pub fn center_si_ti(d: Decomposed) -> (u8, u32, u32) {
    let mut o = d.face & SWAP;
    let (mut i, mut j) = (0u32, 0u32);
    let mut k = 1u8;
    while k <= d.level {
        let digit = child_position(d, k);
        let ij = POS_TO_IJ[usize::from(o)][usize::from(digit)];
        i = (i << 1) | u32::from(ij >> 1);
        j = (j << 1) | u32::from(ij & 1);
        o ^= POS_TO_ORIENTATION[usize::from(digit)];
        k += 1;
    }
    let shift = u32::from(MAX_LEVEL - d.level);
    let size = 1u32 << shift;
    (d.face, 2 * (i << shift) + size, 2 * (j << shift) + size)
}

const HEX: &[u8; 16] = b"0123456789abcdef";

/// Upstream `ToToken`: `"X"` for 0, otherwise the lowercase hex digits of `x` with the
/// trailing `'0'`s stripped. Returns the bytes and their length.
#[must_use]
pub fn token(x: u64) -> ([u8; 16], usize) {
    let mut out = [0u8; 16];
    if x == 0 {
        out[0] = b'X';
        return (out, 1);
    }
    let mut n = 16usize;
    let mut v = x;
    while v & 0xf == 0 {
        v >>= 4;
        n -= 1;
    }
    let mut k = 0usize;
    while k < n {
        out[k] = HEX[((x >> (60 - 4 * k)) & 0xf) as usize];
        k += 1;
    }
    (out, n)
}

fn hex_digit(c: u8) -> Option<u64> {
    match c {
        b'0'..=b'9' => Some(u64::from(c - b'0')),
        b'a'..=b'f' => Some(u64::from(c - b'a' + 10)),
        b'A'..=b'F' => Some(u64::from(c - b'A' + 10)),
        _ => None,
    }
}

/// Upstream `FromToken`: 0 (`None`) if `s` is longer than 16 bytes or has any byte outside
/// `0-9a-fA-F`; otherwise the digits, left-aligned. The empty token gives 0.
#[must_use]
pub fn parse_token(s: &[u8]) -> u64 {
    if s.len() > 16 {
        return NONE;
    }
    let mut id = 0u64;
    let mut k = 0usize;
    while k < s.len() {
        match hex_digit(s[k]) {
            Some(d) => id |= d << (60 - 4 * k),
            None => return NONE,
        }
        k += 1;
    }
    id
}

/// Upstream `ToString`: for a valid id the face digit, `/`, then one digit per level;
/// otherwise `Invalid: ` and the id as 16 zero-padded lowercase hex digits (FR-016).
#[must_use]
pub fn debug_string(x: u64) -> ([u8; 32], usize) {
    let mut out = [0u8; 32];
    match decompose(x) {
        Some(d) => {
            out[0] = b'0' + d.face;
            out[1] = b'/';
            let mut k = 1u8;
            while k <= d.level {
                out[usize::from(k) + 1] = b'0' + child_position(d, k);
                k += 1;
            }
            (out, usize::from(d.level) + 2)
        }
        None => {
            let prefix = b"Invalid: ";
            out[..prefix.len()].copy_from_slice(prefix);
            let mut k = 0usize;
            while k < 16 {
                out[prefix.len() + k] = HEX[((x >> (60 - 4 * k)) & 0xf) as usize];
                k += 1;
            }
            (out, prefix.len() + 16)
        }
    }
}

/// Upstream `FromDebugString`: `level = len − 2` in `0..=30`, a face byte `'0'..='5'`, a
/// `/`, and every digit in `'0'..='3'`; otherwise `None`.
#[must_use]
pub fn parse_debug_string(s: &[u8]) -> Option<u64> {
    if s.len() < 2 || s.len() - 2 > usize::from(MAX_LEVEL) {
        return None;
    }
    let face = s[0].wrapping_sub(b'0');
    if face > 5 || s[1] != b'/' {
        return None;
    }
    let level = (s.len() - 2) as u8;
    let mut path = 0u64;
    let mut k = 2usize;
    while k < s.len() {
        let digit = s[k].wrapping_sub(b'0');
        if digit > 3 {
            return None;
        }
        path = (path << 2) | u64::from(digit);
        k += 1;
    }
    Some(compose(Decomposed { face, level, path }))
}
