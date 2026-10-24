// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Input-domain generators for the decoding harnesses (spec 003 data-model §2;
//! contracts/harness-conventions.md, "Generators").
//!
//! Every generator produces **exactly** `N` values or `L` bytes; the length is a const
//! generic, never a symbolic value (002 research R0). A domain "of at most `N`" is covered by
//! the family `__n0 … __nN` or `__b0 … __bL`. Vectors are built by pushing into a `Vec` with
//! capacity `N`, never with a symbolic-size allocation (spec 003 research R0, finding 2).
//! Variable-length strings grow one byte at a time behind a length guard, for the same reason.

use s2rst::s2::{Cap, CellId, Point, Rect};
use s2rst::{r1, s1};

use crate::domain::any_valid;
use crate::spec::Decomposed;
use crate::wire_spec;

/// `Bytes(L)`: every byte string of exactly `L` bytes. (The same as 002's
/// `union_domain::with_bytes`, which sits behind the `cellunion` feature.)
pub fn with_bytes<const L: usize>(f: impl FnOnce(&[u8])) {
    let bytes: [u8; L] = kani::any();
    f(&bytes[..]);
}

/// Ends a harness's use of a decoder's result without running `io::Error`'s destructor.
///
/// Kani 0.68 reports "misaligned pointer to reference cast" inside std's
/// `io::error::repr_bitpacked::decode_repr` when it drops some errors: the `SimpleMessage`
/// of a short `read_exact`, returned through `Point::decode` or `Cap::decode`. The same error
/// dropped straight after `read_exact`, or through `CellId::decode`, passes (T040's probes,
/// `decoding/EVALUATION.md`). The report depends on the calling context, not on the value,
/// so it is the checker's model. The library builds errors only through std's safe API, so
/// their destructor is std's code: trusted (`verify/TRUSTED_BASE.md`). A decoded value is
/// still dropped normally.
pub fn discard<T>(r: std::io::Result<T>) {
    if let Err(e) = r {
        core::mem::forget(e);
    }
}

/// `Ints64(N)`: exactly `N` values of any bit pattern.
pub fn any_u64s<const N: usize>() -> Vec<u64> {
    let raw: [u64; N] = kani::any();
    let mut v = Vec::with_capacity(N);
    for i in 0..N {
        v.push(raw[i]);
    }
    v
}

/// `Ints32(N)`: exactly `N` values of any bit pattern.
pub fn any_u32s<const N: usize>() -> Vec<u32> {
    let raw: [u32; N] = kani::any();
    let mut v = Vec::with_capacity(N);
    for i in 0..N {
        v.push(raw[i]);
    }
    v
}

/// `Ids(N)`: exactly `N` identifiers of any bits; the format carries invalid cells too.
pub fn any_ids<const N: usize>() -> Vec<CellId> {
    let raw: [u64; N] = kani::any();
    let mut v = Vec::with_capacity(N);
    for i in 0..N {
        v.push(CellId(raw[i]));
    }
    v
}

/// `Strings(N, M)`: exactly `N` byte strings, each of a symbolic length up to `M`.
pub fn any_strings<const N: usize, const M: usize>() -> Vec<Vec<u8>> {
    let mut v = Vec::with_capacity(N);
    for _ in 0..N {
        let raw: [u8; M] = kani::any();
        let len: usize = kani::any();
        kani::assume(len <= M);
        let mut s = Vec::with_capacity(M);
        for j in 0..M {
            if j < len {
                s.push(raw[j]);
            }
        }
        v.push(s);
    }
    v
}

/// One point of any `f64` bit patterns, NaN and infinities included.
pub fn any_point() -> Point {
    let b: wire_spec::PointBits = kani::any();
    Point(s2rst::r3::Vector {
        x: f64::from_bits(b[0]),
        y: f64::from_bits(b[1]),
        z: f64::from_bits(b[2]),
    })
}

/// `Points(N)`: exactly `N` points of any bit patterns.
pub fn any_points<const N: usize>() -> Vec<Point> {
    let mut v = Vec::with_capacity(N);
    for _ in 0..N {
        v.push(any_point());
    }
    v
}

/// `ValidId`: 001's `Valid` (built from the model, not from `CellId::is_valid`).
pub fn any_valid_cell_id() -> (CellId, Decomposed) {
    any_valid()
}

/// `AnyCap`: a cap from any centre and `length2` bit patterns, through
/// `Cap::from_center_chord_angle` (the decoder's own constructor).
pub fn any_cap() -> Cap {
    let len2: u64 = kani::any();
    Cap::from_center_chord_angle(
        any_point(),
        s2rst::s1::ChordAngle::from_length2(f64::from_bits(len2)),
    )
}

/// `AnyRect`: any four `f64` bounds.
pub fn any_rect() -> Rect {
    let b: wire_spec::RectBits = kani::any();
    Rect {
        lat: r1::Interval {
            lo: f64::from_bits(b[0]),
            hi: f64::from_bits(b[1]),
        },
        lng: s1::Interval {
            lo: f64::from_bits(b[2]),
            hi: f64::from_bits(b[3]),
        },
    }
}

/// `ValidRect`: `AnyRect` with [`wire_spec::rect_valid`].
pub fn any_valid_rect() -> Rect {
    let r = any_rect();
    kani::assume(wire_spec::rect_valid(rect_bits(&r)));
    r
}

/// The bit patterns of a point.
pub fn point_bits(p: &Point) -> wire_spec::PointBits {
    [p.0.x.to_bits(), p.0.y.to_bits(), p.0.z.to_bits()]
}

/// The bit patterns of a cap's centre and `length2`.
pub fn cap_bits(c: &Cap) -> wire_spec::CapBits {
    let p = point_bits(&c.center());
    [p[0], p[1], p[2], c.chord_radius().length2().to_bits()]
}

/// The bit patterns of a rectangle.
pub fn rect_bits(r: &Rect) -> wire_spec::RectBits {
    [
        r.lat.lo.to_bits(),
        r.lat.hi.to_bits(),
        r.lng.lo.to_bits(),
        r.lng.hi.to_bits(),
    ]
}
