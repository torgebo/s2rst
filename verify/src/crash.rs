// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Crash-freedom harnesses `C-<op>` for the integer-only in-scope operations (FR-003).
//!
//! Each harness draws its inputs from the operation's domain in `INVENTORY.md`, assumes
//! exactly the documented preconditions, and calls the public API. Kani checks every panic,
//! arithmetic overflow and undefined behaviour on the way; the covers show that the domain is
//! non-empty and that the interesting outcomes are reachable (FR-008). The text parsers
//! discharge their crash obligations in shared property harnesses (`enc`; data-model §3).
//! The neighbour operations have crash-only harnesses here, both in the weekly slow tier
//! (feature `slow`): the `c_edge_neighbors__f*` family and `c_vertex_neighbors`
//! (research R12, R15).
#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::s2::{CellId, Level, from_face_ij, lsb_for_level, size_ij};

use crate::domain::{any_face, any_face_t, any_level, any_u64, any_valid, any_valid_or_end};
use crate::spec;

// ---------------------------------------------------------------------------------------------
// Constructors, queries, free functions and conversions (T021)
// ---------------------------------------------------------------------------------------------

/// C-none: `CellId::none` completes without panic, arithmetic overflow or undefined behaviour.
#[kani::proof]
fn c_none() {
    let c = CellId::none();
    kani::cover!(c.0 == 0, "none is 0");
}

/// C-sentinel: `CellId::sentinel` completes without panic, arithmetic overflow or undefined
/// behaviour.
#[kani::proof]
fn c_sentinel() {
    let c = CellId::sentinel();
    kani::cover!(c.0 == u64::MAX, "sentinel is all ones");
}

/// C-from_face: `CellId::from_face` completes without panic, arithmetic overflow or undefined
/// behaviour for every input in Face.
#[kani::proof]
fn c_from_face() {
    let f = any_face();
    let c = CellId::from_face(f);
    kani::cover!(f == 0, "face 0");
    kani::cover!(f == 5, "face 5");
    kani::cover!(c.0 != 0, "non-zero id");
}

/// C-from_face_pos_level: `CellId::from_face_pos_level` completes without panic, arithmetic
/// overflow or undefined behaviour for every input in Face × U64 × Level, assuming
/// `pos < 2^61` (rustdoc "61-bit").
#[kani::proof]
fn c_from_face_pos_level() {
    let f = any_face();
    let pos: u64 = kani::any();
    kani::assume(pos < (1u64 << 61));
    let l = any_level();
    let c = CellId::from_face_pos_level(f, pos, l);
    kani::cover!(l.as_u8() == 0, "level 0");
    kani::cover!(l.as_u8() == 30, "level 30");
    kani::cover!(
        f == 5 && pos == (1u64 << 61) - 1,
        "largest face and position"
    );
    kani::cover!(c.0 != 0, "non-zero id");
}

/// C-id: `CellId::id` completes without panic, arithmetic overflow or undefined behaviour for
/// every input in Any.
#[kani::proof]
fn c_id() {
    let c = any_u64();
    let x = c.id();
    kani::cover!(x == u64::MAX, "all ones");
}

/// C-is_valid: `CellId::is_valid` completes without panic, arithmetic overflow or undefined
/// behaviour for every input in Any.
#[kani::proof]
fn c_is_valid() {
    let c = any_u64();
    let v = c.is_valid();
    kani::cover!(v, "valid");
    kani::cover!(!v, "invalid");
}

/// C-face: `CellId::face` completes without panic, arithmetic overflow or undefined behaviour
/// for every input in Valid.
#[kani::proof]
fn c_face() {
    let (c, d) = any_valid();
    let f = c.face();
    kani::cover!(d.face == 0, "face 0");
    kani::cover!(f.as_u8() == 5, "face 5");
}

/// C-pos: `CellId::pos` completes without panic, arithmetic overflow or undefined behaviour
/// for every input in ValidOrEnd.
#[kani::proof]
fn c_pos() {
    let c = any_valid_or_end();
    let p = c.pos();
    kani::cover!(c.0 == spec::SENTINEL, "sentinel");
    kani::cover!(p == 1, "first leaf of face 0");
}

/// C-level: `CellId::level` completes without panic, arithmetic overflow or undefined
/// behaviour for every input in ValidOrEnd (which contains no id with 62 or more trailing
/// zeros). Upstream finding U-001: C++ `level()` is undefined behaviour on `0` and returns −1
/// on ids with 62 or more trailing zeros; Rust panics on those ids (`INVENTORY.md` row 11).
#[kani::proof]
fn c_level() {
    let c = any_valid_or_end();
    let l = c.level();
    kani::cover!(l.as_u8() == 0, "face level");
    kani::cover!(l.as_u8() == 30, "leaf level");
    kani::cover!(c.0 == spec::end(0), "end(0)");
}

/// C-lsb: `CellId::lsb` completes without panic, arithmetic overflow or undefined behaviour
/// for every input in ValidOrEnd.
#[kani::proof]
fn c_lsb() {
    let c = any_valid_or_end();
    let b = c.lsb();
    kani::cover!(b == 1, "leaf lsb");
    kani::cover!(b == 1u64 << 60, "face lsb");
}

/// C-is_leaf: `CellId::is_leaf` completes without panic, arithmetic overflow or undefined
/// behaviour for every input in ValidOrEnd.
#[kani::proof]
fn c_is_leaf() {
    let c = any_valid_or_end();
    let leaf = c.is_leaf();
    kani::cover!(leaf, "leaf");
    kani::cover!(!leaf, "not a leaf");
}

/// C-is_face: `CellId::is_face` completes without panic, arithmetic overflow or undefined
/// behaviour for every input in ValidOrEnd.
#[kani::proof]
fn c_is_face() {
    let c = any_valid_or_end();
    let face = c.is_face();
    kani::cover!(face, "face cell");
    kani::cover!(!face, "not a face cell");
}

/// C-lsb_for_level: `s2::lsb_for_level` completes without panic, arithmetic overflow or
/// undefined behaviour for every input in Level.
#[kani::proof]
fn c_lsb_for_level() {
    let l = any_level();
    let b = lsb_for_level(l);
    kani::cover!(b == 1, "level 30");
    kani::cover!(b == 1u64 << 60, "level 0");
}

/// C-size_ij: `s2::size_ij` completes without panic, arithmetic overflow or undefined
/// behaviour for every input in Level.
#[kani::proof]
fn c_size_ij() {
    let l = any_level();
    let s = size_ij(l);
    kani::cover!(s == 1, "level 30");
    kani::cover!(s == 1 << 30, "level 0");
}

/// C-Default: `impl Default for CellId` completes without panic, arithmetic overflow or
/// undefined behaviour.
#[kani::proof]
fn c_default() {
    let c = CellId::default();
    kani::cover!(c.0 == 0, "default is none");
}

/// C-From<u64>: `impl From<u64> for CellId` completes without panic, arithmetic overflow or
/// undefined behaviour for every input in Any.
#[kani::proof]
fn c_from_u64() {
    let x: u64 = kani::any();
    let c = CellId::from(x);
    kani::cover!(c.0 == u64::MAX, "all ones");
}

/// C-From<CellId> for u64: `impl From<CellId> for u64` completes without panic, arithmetic
/// overflow or undefined behaviour for every input in Any.
#[kani::proof]
fn c_u64_from() {
    let c = any_u64();
    let x = u64::from(c);
    kani::cover!(x == 0, "zero");
}

// ---------------------------------------------------------------------------------------------
// Hierarchy (T022)
// ---------------------------------------------------------------------------------------------

/// C-child_position: `CellId::child_position` completes without panic, arithmetic overflow
/// or undefined behaviour for every input in Valid × Level, assuming `1 ≤ l ≤ level`
/// (`debug_assert!`, rustdoc).
#[kani::proof]
fn c_child_position() {
    let (c, d) = any_valid();
    let l = any_level();
    kani::assume(l.as_u8() >= 1 && l.as_u8() <= d.level);
    let p = c.child_position(l);
    kani::cover!(p == 3, "position 3");
    kani::cover!(d.level == 30 && l.as_u8() == 30, "leaf, finest level");
}

/// C-parent_at_level: `CellId::parent_at_level` completes without panic, arithmetic overflow
/// or undefined behaviour for every input in Valid × Level, assuming `l ≤ level` (rustdoc).
#[kani::proof]
fn c_parent_at_level() {
    let (c, d) = any_valid();
    let l = any_level();
    kani::assume(l.as_u8() <= d.level);
    let p = c.parent_at_level(l);
    kani::cover!(l.as_u8() == 0, "face ancestor");
    kani::cover!(l.as_u8() == d.level && p == c, "itself");
}

/// C-parent: `CellId::parent` completes without panic, arithmetic overflow or undefined
/// behaviour for every input in Valid, assuming the cell is not a face (rustdoc,
/// `debug_assert!`).
#[kani::proof]
fn c_parent() {
    let (c, d) = any_valid();
    kani::assume(d.level >= 1);
    let p = c.parent();
    kani::cover!(d.level == 1, "parent is a face");
    kani::cover!(d.level == 30 && p.0 != 0, "parent of a leaf");
}

/// C-children: `CellId::children` completes without panic, arithmetic overflow or undefined
/// behaviour for every input in Valid, assuming the cell is not a leaf (`debug_assert!`).
#[kani::proof]
fn c_children() {
    let (c, d) = any_valid();
    kani::assume(d.level < 30);
    let ch = c.children();
    kani::cover!(d.level == 0, "children of a face");
    kani::cover!(d.level == 29 && ch[3].is_leaf(), "leaf children");
}

/// C-child_begin: `CellId::child_begin` completes without panic, arithmetic overflow or
/// undefined behaviour for every input in Valid, assuming the cell is not a leaf
/// (`debug_assert!`).
#[kani::proof]
fn c_child_begin() {
    let (c, d) = any_valid();
    kani::assume(d.level < 30);
    let b = c.child_begin();
    kani::cover!(d.level == 0, "face");
    kani::cover!(b.is_leaf(), "leaf child");
}

/// C-child_end: `CellId::child_end` completes without panic, arithmetic overflow or undefined
/// behaviour for every input in Valid, assuming the cell is not a leaf (`debug_assert!`).
#[kani::proof]
fn c_child_end() {
    let (c, d) = any_valid();
    kani::assume(d.level < 30);
    let e = c.child_end();
    kani::cover!(
        d.face == 5 && d.path == spec::pow4(u32::from(d.level)) - 1,
        "last cell"
    );
    kani::cover!(!e.is_valid(), "past the end");
}

/// C-child_begin_at_level: `CellId::child_begin_at_level` completes without panic, arithmetic
/// overflow or undefined behaviour for every input in Valid × Level, assuming `l ≥ level`
/// (rustdoc).
#[kani::proof]
fn c_child_begin_at_level() {
    let (c, d) = any_valid();
    let l = any_level();
    kani::assume(l.as_u8() >= d.level);
    let b = c.child_begin_at_level(l);
    kani::cover!(l.as_u8() == d.level && b == c, "same level");
    kani::cover!(d.level == 0 && l.as_u8() == 30, "face to leaf");
}

/// C-child_end_at_level: `CellId::child_end_at_level` completes without panic, arithmetic
/// overflow or undefined behaviour for every input in Valid × Level, assuming `l ≥ level`
/// (rustdoc).
#[kani::proof]
fn c_child_end_at_level() {
    let (c, d) = any_valid();
    let l = any_level();
    kani::assume(l.as_u8() >= d.level);
    let e = c.child_end_at_level(l);
    kani::cover!(
        d.face == 5 && d.level == 0 && l.as_u8() == 30,
        "end of the last face"
    );
    kani::cover!(e.0 != 0, "non-zero");
}

/// C-begin: `CellId::begin` completes without panic, arithmetic overflow or undefined
/// behaviour for every input in Level.
#[kani::proof]
fn c_begin() {
    let l = any_level();
    let b = CellId::begin(l);
    kani::cover!(l.as_u8() == 0, "level 0");
    kani::cover!(b.is_leaf(), "level 30");
}

/// C-end: `CellId::end` completes without panic, arithmetic overflow or undefined behaviour
/// for every input in Level.
#[kani::proof]
fn c_end() {
    let l = any_level();
    let e = CellId::end(l);
    kani::cover!(l.as_u8() == 0, "level 0");
    kani::cover!(e.is_leaf(), "level 30");
}

// ---------------------------------------------------------------------------------------------
// Ranges, traversal and common ancestor (T023)
// ---------------------------------------------------------------------------------------------

/// C-range_min: `CellId::range_min` completes without panic, arithmetic overflow or undefined
/// behaviour for every input in ValidOrEnd.
#[kani::proof]
fn c_range_min() {
    let c = any_valid_or_end();
    let r = c.range_min();
    kani::cover!(c.0 == spec::SENTINEL, "sentinel");
    kani::cover!(r.is_leaf(), "leaf");
}

/// C-range_max: `CellId::range_max` completes without panic, arithmetic overflow or undefined
/// behaviour for every input in ValidOrEnd.
#[kani::proof]
fn c_range_max() {
    let c = any_valid_or_end();
    let r = c.range_max();
    kani::cover!(c.0 == spec::SENTINEL, "sentinel");
    kani::cover!(r.is_leaf(), "leaf");
}

/// C-contains: `CellId::contains` completes without panic, arithmetic overflow or undefined
/// behaviour for every input in Valid × Valid (upstream `DCHECK(is_valid())` on both).
#[kani::proof]
fn c_contains() {
    let (a, _) = any_valid();
    let (b, _) = any_valid();
    let r = a.contains(b);
    kani::cover!(r && a != b, "strict containment");
    kani::cover!(!r, "not contained");
}

/// C-intersects: `CellId::intersects` completes without panic, arithmetic overflow or
/// undefined behaviour for every input in Valid × Valid (upstream `DCHECK(is_valid())` on
/// both).
#[kani::proof]
fn c_intersects() {
    let (a, _) = any_valid();
    let (b, _) = any_valid();
    let r = a.intersects(b);
    kani::cover!(r, "intersect");
    kani::cover!(!r, "disjoint");
}

/// C-next: `CellId::next` completes without panic, arithmetic overflow or undefined behaviour
/// for every input in Valid.
#[kani::proof]
fn c_next() {
    let (c, d) = any_valid();
    let n = c.next();
    kani::cover!(n.0 == spec::end(d.level), "reaches end");
    kani::cover!(n.is_valid(), "valid successor");
}

/// C-prev: `CellId::prev` completes without panic, arithmetic overflow or undefined behaviour
/// for every input in ValidOrEnd.
#[kani::proof]
fn c_prev() {
    let c = any_valid_or_end();
    let p = c.prev();
    kani::cover!(c.0 == spec::end(30) && p.is_valid(), "last leaf from end");
    kani::cover!(c.0 == spec::SENTINEL, "sentinel");
}

/// C-next_wrap: `CellId::next_wrap` completes without panic, arithmetic overflow or undefined
/// behaviour for every input in Valid (`debug_assert!`).
#[kani::proof]
fn c_next_wrap() {
    let (c, d) = any_valid();
    let n = c.next_wrap();
    kani::cover!(d.face == 5 && n.face().as_u8() == 0, "wraps to face 0");
    kani::cover!(n.0 > c.0, "no wrap");
}

/// C-prev_wrap: `CellId::prev_wrap` completes without panic, arithmetic overflow or undefined
/// behaviour for every input in Valid (`debug_assert!`).
#[kani::proof]
fn c_prev_wrap() {
    let (c, d) = any_valid();
    let p = c.prev_wrap();
    kani::cover!(d.face == 0 && p.face().as_u8() == 5, "wraps to face 5");
    kani::cover!(p.0 < c.0, "no wrap");
}

/// C-advance: `CellId::advance` completes without panic, arithmetic overflow or undefined
/// behaviour for every input in Valid × I64 (upstream `DCHECK(is_valid())`).
#[kani::proof]
fn c_advance() {
    let (c, d) = any_valid();
    let steps: i64 = kani::any();
    let r = c.advance(steps);
    kani::cover!(steps == i64::MAX, "i64::MAX steps");
    kani::cover!(steps == i64::MIN, "i64::MIN steps");
    kani::cover!(r.0 == spec::end(d.level), "clamped to end");
    kani::cover!(
        steps < 0 && r.0 == spec::from_position(d.level, 0),
        "clamped to begin"
    );
}

/// C-advance_wrap: `CellId::advance_wrap` completes without panic, arithmetic overflow or
/// undefined behaviour for every input in Valid × I64 (upstream `DCHECK(is_valid())`).
#[kani::proof]
fn c_advance_wrap() {
    let (c, d) = any_valid();
    let steps: i64 = kani::any();
    let r = c.advance_wrap(steps);
    kani::cover!(steps == i64::MAX, "i64::MAX steps");
    kani::cover!(steps == i64::MIN, "i64::MIN steps");
    kani::cover!(steps > 0 && r.0 < c.0, "wraps forward");
    kani::cover!(steps < 0 && r.0 > c.0 && d.level > 0, "wraps backward");
}

/// C-distance_from_begin: `CellId::distance_from_begin` completes without panic, arithmetic
/// overflow or undefined behaviour for every input in ValidOrEnd.
#[kani::proof]
fn c_distance_from_begin() {
    let c = any_valid_or_end();
    let n = c.distance_from_begin();
    kani::cover!(n == 0, "begin");
    kani::cover!(c.0 == spec::end(30), "end of the leaves");
}

/// C-maximum_tile: `CellId::maximum_tile` completes without panic, arithmetic overflow or
/// undefined behaviour for every input in Valid × ValidOrEnd.
#[kani::proof]
#[kani::unwind(31)]
fn c_maximum_tile() {
    let (c, _) = any_valid();
    let limit = any_valid_or_end();
    let t = c.maximum_tile(limit);
    kani::cover!(t == limit, "returns the limit");
    kani::cover!(t != limit && t.is_face(), "grows to a face");
    kani::cover!(t != limit && t.0 != c.0 && t.contains(c), "grows");
    kani::cover!(t != limit && c.contains(t) && t != c, "shrinks");
}

/// C-common_ancestor_level: `CellId::common_ancestor_level` completes without panic,
/// arithmetic overflow or undefined behaviour for every input in Any × Any, assuming the pair
/// is not (`none`, `none`). Upstream finding U-002: C++ `GetCommonAncestorLevel` assumes that
/// pair away (`ABSL_ASSUME(bits != 0)`), Go returns level 30, Rust panics in `ilog2(0)`.
#[kani::proof]
fn c_common_ancestor_level() {
    let a = any_u64();
    let b = any_u64();
    kani::assume(!(a.0 == 0 && b.0 == 0));
    let r = a.common_ancestor_level(b);
    kani::cover!(r.is_none(), "different faces");
    kani::cover!(r == Some(Level::new(30)), "level 30");
    kani::cover!(r == Some(Level::new(0)), "level 0");
}

// ---------------------------------------------------------------------------------------------
// Face / (i, j) (T024)
// ---------------------------------------------------------------------------------------------

/// C-get_center_si_ti: `CellId::get_center_si_ti` completes without panic, arithmetic overflow
/// or undefined behaviour for every input in Valid.
#[kani::proof]
#[kani::unwind(9)]
fn c_get_center_si_ti() {
    let (c, d) = any_valid();
    let (_, si, ti) = c.get_center_si_ti();
    kani::cover!(d.level == 30, "leaf");
    kani::cover!(
        si == (1 << 31) - 1 || ti == (1 << 31) - 1,
        "largest coordinate"
    );
}

/// C-to_face_ij_orientation: `CellId::to_face_ij_orientation` completes without panic,
/// arithmetic overflow or undefined behaviour for every input in Valid.
#[kani::proof]
#[kani::unwind(9)]
fn c_to_face_ij_orientation() {
    let (c, d) = any_valid();
    let (_, i, j, o) = c.to_face_ij_orientation();
    kani::cover!(d.level == 0, "face");
    kani::cover!(i == (1 << 30) - 1 && j == 0, "corner");
    kani::cover!(o == 3, "swapped and inverted");
}

/// C-from_face_ij: `s2::from_face_ij` completes without panic, arithmetic overflow or
/// undefined behaviour for every input in Face × IJ × IJ, assuming `i, j ∈ [0, 2^30 − 1]`
/// (upstream `s2coords.h:44–45`). Over the whole `i32` range Rust panics (`n * 2` overflows
/// in debug builds, e.g. `i = −2147483641`), where C++ wraps silently: defect D-001
/// (`EVALUATION.md`), documented in the `# Panics` rustdoc (`INVENTORY.md` row 57).
#[kani::proof]
#[kani::unwind(9)]
fn c_from_face_ij() {
    let f = any_face_t();
    let i: i32 = kani::any();
    let j: i32 = kani::any();
    kani::assume((0..1 << 30).contains(&i) && (0..1 << 30).contains(&j));
    let c = from_face_ij(f, i, j);
    kani::cover!(i == 0 && j == 0, "origin");
    kani::cover!(i == (1 << 30) - 1 && j == (1 << 30) - 1, "far corner");
    kani::cover!(c.is_leaf(), "leaf result");
}

// ---------------------------------------------------------------------------------------------
// Neighbour operations (research R12): crash-only split families. The property harnesses in
// `nbr` are too expensive to also carry these obligations within the R15 budget.
// ---------------------------------------------------------------------------------------------

/// One face of C-edge_neighbors. The six faces together cover Valid. Weekly slow tier
/// (feature `slow`): the family takes about 22 CPU-minutes, and with it the per-change run
/// took 27 minutes on a 4-vCPU machine, over SC-005's 15 (research R15).
macro_rules! edge_crash_face {
    ($($name:ident = $face:literal),* $(,)?) => { $(
        /// C-edge_neighbors: `CellId::edge_neighbors` completes without panic, arithmetic
        /// overflow or undefined behaviour for every input in Valid (one face of the
        /// `c_edge_neighbors__f*` family).
        #[cfg(feature = "slow")]
        #[kani::proof]
        #[kani::unwind(10)]
        fn $name() {
            let (c, _) = crate::domain::any_valid_on_face($face);
            let n = c.edge_neighbors();
            kani::cover!(n[0].face() != c.face(), "wrap");
            kani::cover!(n[0].face() == c.face(), "same face");
        }
    )* };
}

edge_crash_face!(
    c_edge_neighbors__f0 = 0,
    c_edge_neighbors__f1 = 1,
    c_edge_neighbors__f2 = 2,
    c_edge_neighbors__f3 = 3,
    c_edge_neighbors__f4 = 4,
    c_edge_neighbors__f5 = 5,
);

/// C-vertex_neighbors: `CellId::vertex_neighbors` completes without panic, arithmetic overflow
/// or undefined behaviour for every input in Valid × Level, assuming `l < level` (upstream
/// `DCHECK_LT(level, this->level())`). Weekly slow tier (feature `slow`): about 10.6 minutes,
/// over the per-change budget in every split tried (research R12).
#[cfg(feature = "slow")]
#[kani::proof]
#[kani::unwind(10)]
fn c_vertex_neighbors() {
    let (c, d) = any_valid();
    let l: u8 = kani::any();
    kani::assume(l < d.level);
    let v = c.vertex_neighbors(Level::new(l));
    kani::cover!(v.len() == 3, "cube vertex");
    kani::cover!(v.len() == 4, "4 cells");
}

// ---------------------------------------------------------------------------------------------
// Outside the domain: the documented panics (code review of 074e923's coverage, gap 1)
// ---------------------------------------------------------------------------------------------
//
// The harnesses above prove that an operation does not panic inside its domain. These prove
// the other half of its `# Panics` section: for every input outside the domain it panics (`n_*`,
// `#[kani::should_panic]`), so the section is exact. Kani passes a `should_panic` harness when
// one input panics; each one therefore ends in a single cover after the call, which only an
// input that returns would satisfy, and `run-verify.sh` requires it unsatisfied (covers
// `never`). A panic that the section calls debug-only is proven in the debug build (`n_*`, under
// `cfg(debug_assertions)`), and the release build's behaviour on the same inputs separately
// (`r_*`, under `cfg(not(debug_assertions))`, `make verify-release`): no panic and no overflow.
// Where a `# Panics` names inputs outside the domain that return (an unspecified result, or one
// it states), an `o_*` harness proves that they do, in every build. `make verify-release` runs
// the `n_*` harnesses whose panic is in every build, the `r_*` and the `o_*` ones.

/// N-Level: `impl From<u8> for Level` panics for every raw level above 30, in every build, and
/// so does each operation that takes `impl Into<Level>` (its `# Panics`: "the conversion to
/// `Level` panics"), before it reads its other arguments.
///
/// The unwind bound is for the code after the panic: the checker does not simplify the failed
/// check's condition (`raw ≤ 30` under the assumption `raw > 30`) to false, so it explores that
/// code too, on paths no input takes, and the neighbour operations' loops timed out at 10
/// minutes without it. Unwinding checks stay on, so a reachable loop beyond the bound would
/// fail the proof.
#[kani::proof]
#[kani::should_panic]
#[kani::unwind(2)]
fn n_raw_level() {
    let raw: u8 = kani::any();
    kani::assume(raw > 30);
    let (c, _) = any_valid();
    let op: u8 = kani::any();
    match op {
        0 => {
            let _ = Level::from(raw);
        }
        1 => {
            let _ = CellId::from_face_pos_level(0u8, 0, raw);
        }
        2 => {
            let _ = c.child_position(raw);
        }
        3 => {
            let _ = c.parent_at_level(raw);
        }
        4 => {
            let _ = c.child_begin_at_level(raw);
        }
        5 => {
            let _ = c.child_end_at_level(raw);
        }
        6 => {
            let _ = CellId::begin(raw);
        }
        7 => {
            let _ = CellId::end(raw);
        }
        8 => {
            let _ = lsb_for_level(raw);
        }
        9 => {
            let _ = size_ij(raw);
        }
        10 => {
            let _ = CellId::size_st(raw);
        }
        11 => {
            let _ = c.vertex_neighbors(raw);
        }
        _ => {
            let _ = c.all_neighbors(raw);
        }
    }
    kani::cover!(true, "returns");
}

/// N-level: `CellId::level` panics for every id with 62 or more trailing zero bits (`0`,
/// `1 << 62`, `1 << 63`, `3 << 62`), in every build: the ids its `# Panics` names.
#[kani::proof]
#[kani::should_panic]
fn n_level() {
    let x: u64 = kani::any();
    kani::assume(x.trailing_zeros() >= 62);
    let _ = CellId(x).level();
    kani::cover!(true, "returns");
}

/// The complement of `child_position`'s domain: an invalid id with any level, or a valid cell
/// with a level outside `1..=level`.
fn outside_child_position() -> (CellId, Level, bool) {
    let x: u64 = kani::any();
    let l = any_level();
    let valid = spec::valid_by_marker(x);
    if valid {
        // The level whose marker bit is the lowest set bit.
        let level = (60 - x.trailing_zeros()) / 2;
        kani::assume(l.as_u8() < 1 || u32::from(l.as_u8()) > level);
    }
    (CellId(x), l, !valid)
}

/// N-child_position: in debug builds, `CellId::child_position` panics for every invalid id and
/// every valid cell with a level outside `1..=level` (its `debug_assert!`s).
#[cfg(debug_assertions)]
#[kani::proof]
#[kani::should_panic]
fn n_child_position() {
    let (c, l, _) = outside_child_position();
    let _ = c.child_position(l);
    kani::cover!(true, "returns");
}

/// R-child_position: in a build without debug assertions, `CellId::child_position` completes
/// without panic or overflow on the same inputs (its `# Panics` names debug builds only).
#[cfg(not(debug_assertions))]
#[kani::proof]
fn r_child_position() {
    let (c, l, invalid) = outside_child_position();
    let _ = c.child_position(l);
    kani::cover!(invalid, "an invalid id");
    kani::cover!(!invalid, "a level outside 1..=level");
}

/// N-parent: in debug builds, `CellId::parent` panics for every face cell.
#[cfg(debug_assertions)]
#[kani::proof]
#[kani::should_panic]
fn n_parent() {
    let (c, _) = crate::domain::any_valid_at(0);
    let _ = c.parent();
    kani::cover!(true, "returns");
}

/// R-parent: in a build without debug assertions, `CellId::parent` completes without panic or
/// overflow for every face cell.
#[cfg(not(debug_assertions))]
#[kani::proof]
fn r_parent() {
    let (c, d) = crate::domain::any_valid_at(0);
    let _ = c.parent();
    kani::cover!(d.face == 5, "face 5");
}

/// The complement of the child operations' domain: an invalid id or a leaf.
fn outside_children() -> (CellId, bool) {
    let leaf: bool = kani::any();
    if leaf {
        (crate::domain::any_valid_at(30).0, true)
    } else {
        (crate::domain::any_invalid(), false)
    }
}

/// N-children, N-child_begin, N-child_end: in debug builds, each of `CellId::children`,
/// `child_begin` and `child_end` panics for every invalid id and every leaf.
#[cfg(debug_assertions)]
#[kani::proof]
#[kani::should_panic]
fn n_children() {
    let (c, _) = outside_children();
    let op: u8 = kani::any();
    match op {
        0 => {
            let _ = c.children();
        }
        1 => {
            let _ = c.child_begin();
        }
        _ => {
            let _ = c.child_end();
        }
    }
    kani::cover!(true, "returns");
}

/// R-children, R-child_begin, R-child_end: in a build without debug assertions, each completes
/// without panic or overflow on the same inputs.
#[cfg(not(debug_assertions))]
#[kani::proof]
fn r_children() {
    let (c, leaf) = outside_children();
    let op: u8 = kani::any();
    match op {
        0 => {
            let _ = c.children();
        }
        1 => {
            let _ = c.child_begin();
        }
        _ => {
            // `child_end`'s sum overflows for the ids within 5/4 of their lowest set bit of
            // 2^64, `sentinel` among them (2026-10-26). A release build wraps there, without a
            // panic, but Kani checks overflow in every build, so those ids are outside this
            // proof.
            let lsb = c.0 & c.0.wrapping_neg();
            kani::assume(c.0.checked_add(lsb + (lsb >> 2)).is_some());
            let _ = c.child_end();
        }
    }
    kani::cover!(leaf, "a leaf");
    kani::cover!(!leaf, "an invalid id");
}

/// N-next_wrap, N-prev_wrap: in debug builds, `CellId::next_wrap` and `prev_wrap` panic for
/// every invalid id.
#[cfg(debug_assertions)]
#[kani::proof]
#[kani::should_panic]
fn n_wrap() {
    let c = crate::domain::any_invalid();
    if kani::any::<bool>() {
        let _ = c.next_wrap();
    } else {
        let _ = c.prev_wrap();
    }
    kani::cover!(true, "returns");
}

/// R-next_wrap, R-prev_wrap: in a build without debug assertions, both complete without panic
/// or overflow for every invalid id.
#[cfg(not(debug_assertions))]
#[kani::proof]
fn r_wrap() {
    let c = crate::domain::any_invalid();
    let (n, p) = (c.next_wrap(), c.prev_wrap());
    kani::cover!(c.0 == 0 && n.0 == 0 && p.0 == 0, "none stays none");
}

/// N-vertex_neighbors: `CellId::vertex_neighbors` panics for every valid cell at level 30, in
/// every build (its `# Panics`: `Level::new(31)`).
#[kani::proof]
#[kani::should_panic]
fn n_vertex_neighbors() {
    let (c, _) = any_valid();
    let _ = c.vertex_neighbors(Level::new(30));
    kani::cover!(true, "returns");
}

/// N-common_ancestor_level: `CellId::common_ancestor_level` panics for the pair (`none`,
/// `none`), in every build (`ilog2(0)`).
#[kani::proof]
#[kani::should_panic]
fn n_common_ancestor_level() {
    let _ = CellId::none().common_ancestor_level(CellId::none());
    kani::cover!(true, "returns");
}
