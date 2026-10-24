// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Neighbour properties P-NBR-* (FR-005, research R12).
//!
//! The reference model does not define cross-face neighbours (that needs upstream's
//! floating-point face transforms), so these properties are relational. CBMC reasons about
//! the `f64` wrap path bit for bit. The crash obligations C-edge_neighbors and
//! C-vertex_neighbors have their own crash-only harnesses in `crash` (research R12).
//!
//! The whole module is in the weekly slow tier (feature `slow`, `make verify-slow`): every
//! harness here exceeds the per-change budget (research R12, R15).
//!
//! P-NBR-02 (edge-neighbour symmetry) has no harness: it calls `edge_neighbors` twice, and
//! CBMC ran out of memory at the 4 GiB ceiling within 35 s per face, and per face × neighbour
//! index (`not-proven (tool-limit)`, `PROPERTIES.md`).
#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::s2::{CellId, Level, size_ij};

use crate::domain::any_valid_on_face;

/// The P-NBR-03 check shared by the quadrant cases: `vertex_neighbors(l)` gives 3 or 4 valid,
/// pairwise distinct cells at level `l`, one of them `parent_at_level(l)`.
///
/// The entries are read by fixed index (the fourth only when present) rather than by looping
/// over the `Vec`, so the assertions add a constant-size formula (research R15).
fn check_vertex_neighbors(c: CellId, l: Level) {
    let v = c.vertex_neighbors(l);
    let n = v.len();
    assert!(n == 3 || n == 4);
    let (a, b, e) = (v[0], v[1], v[2]);
    let four = n == 4;
    let d = if four { v[3] } else { a };
    for x in [a, b, e, d] {
        assert!(x.is_valid());
        assert!(x.level() == l);
    }
    assert!(a != b && a != e && b != e);
    if four {
        assert!(d != a && d != b && d != e);
    }
    let p = c.parent_at_level(l);
    assert!(a == p || b == p || e == p || d == p);
    kani::cover!(!four, "3 cells (cube vertex)");
    kani::cover!(four, "4 cells");
}

/// One face × quadrant case of P-NBR-03. The 24 cases together cover Valid × Level
/// (`l < level`). The quadrant-only family ran past 30 minutes per case (research R12).
macro_rules! vertex_face_quadrant {
    ($($name:ident = ($face:literal, $ihi:expr, $jhi:expr)),* $(,)?) => { $(
        /// P-NBR-03 (validity): `vertex_neighbors(l)` gives 3 or 4 distinct valid cells at level
        /// `l`, one of them the cell's own ancestor.
        ///
        /// `vertex_neighbors(l)` gives 3 or 4 valid, pairwise distinct cells at level `l`, one of
        /// them `parent_at_level(l)`. One face × quadrant case of the `p_nbr_03__f*_q*` family.
        #[kani::proof]
        #[kani::unwind(10)]
        fn $name() {
            let (c, d) = any_valid_on_face($face);
            let l: u8 = kani::any();
            // Documented upstream precondition: `DCHECK_LT(level, this->level())`.
            kani::assume(l < d.level);
            let half = size_ij(Level::new(l + 1));
            let (_, i, j, _) = c.to_face_ij_orientation();
            kani::assume(((i & half) != 0) == $ihi && ((j & half) != 0) == $jhi);
            check_vertex_neighbors(c, Level::new(l));
        }
    )* };
}

vertex_face_quadrant!(
    p_nbr_03__f0_q0 = (0, false, false),
    p_nbr_03__f0_q1 = (0, false, true),
    p_nbr_03__f0_q2 = (0, true, false),
    p_nbr_03__f0_q3 = (0, true, true),
    p_nbr_03__f1_q0 = (1, false, false),
    p_nbr_03__f1_q1 = (1, false, true),
    p_nbr_03__f1_q2 = (1, true, false),
    p_nbr_03__f1_q3 = (1, true, true),
    p_nbr_03__f2_q0 = (2, false, false),
    p_nbr_03__f2_q1 = (2, false, true),
    p_nbr_03__f2_q2 = (2, true, false),
    p_nbr_03__f2_q3 = (2, true, true),
    p_nbr_03__f3_q0 = (3, false, false),
    p_nbr_03__f3_q1 = (3, false, true),
    p_nbr_03__f3_q2 = (3, true, false),
    p_nbr_03__f3_q3 = (3, true, true),
    p_nbr_03__f4_q0 = (4, false, false),
    p_nbr_03__f4_q1 = (4, false, true),
    p_nbr_03__f4_q2 = (4, true, false),
    p_nbr_03__f4_q3 = (4, true, true),
    p_nbr_03__f5_q0 = (5, false, false),
    p_nbr_03__f5_q1 = (5, false, true),
    p_nbr_03__f5_q2 = (5, true, false),
    p_nbr_03__f5_q3 = (5, true, true),
);

/// One face × neighbour case of P-NBR-01's per-neighbour conjuncts. A per-face harness with
/// all conjuncts took 21–27 minutes on two faces and timed out at 30 on the other four;
/// checking one neighbour or one pair at a time keeps each case near 10 minutes, because
/// CBMC slices away the wrap paths the assertions do not use (research R12, R15). Together
/// with `p_nbr_01__f*_p*`, the family covers every conjunct on every face.
macro_rules! edge_neighbor_k {
    ($($name:ident = ($face:literal, $k:literal)),* $(,)?) => { $(
        /// P-NBR-01 (validity): `edge_neighbors()` gives four distinct valid cells of the same
        /// level, none of them the cell itself; here, the conjuncts for one neighbour.
        ///
        /// `edge_neighbors()[k]` is a valid cell at `level()`, not equal to `c`. One case of the
        /// `p_nbr_01__f*_k*` family.
        #[kani::proof]
        #[kani::unwind(10)]
        fn $name() {
            let (c, d) = any_valid_on_face($face);
            let n = c.edge_neighbors()[$k];
            assert!(n.is_valid());
            assert!(n.level().as_u8() == d.level);
            assert!(n != c);
            kani::cover!(n.face() != c.face(), "wrap");
            kani::cover!(n.face() == c.face(), "same face");
        }
    )* };
}

edge_neighbor_k!(
    p_nbr_01__f0_k0 = (0, 0),
    p_nbr_01__f0_k1 = (0, 1),
    p_nbr_01__f0_k2 = (0, 2),
    p_nbr_01__f0_k3 = (0, 3),
    p_nbr_01__f1_k0 = (1, 0),
    p_nbr_01__f1_k1 = (1, 1),
    p_nbr_01__f1_k2 = (1, 2),
    p_nbr_01__f1_k3 = (1, 3),
    p_nbr_01__f2_k0 = (2, 0),
    p_nbr_01__f2_k1 = (2, 1),
    p_nbr_01__f2_k2 = (2, 2),
    p_nbr_01__f2_k3 = (2, 3),
    p_nbr_01__f3_k0 = (3, 0),
    p_nbr_01__f3_k1 = (3, 1),
    p_nbr_01__f3_k2 = (3, 2),
    p_nbr_01__f3_k3 = (3, 3),
    p_nbr_01__f4_k0 = (4, 0),
    p_nbr_01__f4_k1 = (4, 1),
    p_nbr_01__f4_k2 = (4, 2),
    p_nbr_01__f4_k3 = (4, 3),
    p_nbr_01__f5_k0 = (5, 0),
    p_nbr_01__f5_k1 = (5, 1),
    p_nbr_01__f5_k2 = (5, 2),
    p_nbr_01__f5_k3 = (5, 3),
);

/// One face × pair case of P-NBR-01's distinctness conjunct.
macro_rules! edge_neighbor_pair {
    ($($name:ident = ($face:literal, $a:literal, $b:literal)),* $(,)?) => { $(
        /// P-NBR-01 (validity): `edge_neighbors()` gives four distinct valid cells of the same
        /// level, none of them the cell itself; here, the distinctness of one pair.
        ///
        /// `edge_neighbors()[a] != edge_neighbors()[b]`. One case of the `p_nbr_01__f*_p*`
        /// family.
        #[kani::proof]
        #[kani::unwind(10)]
        fn $name() {
            let (c, _) = any_valid_on_face($face);
            let n = c.edge_neighbors();
            assert!(n[$a] != n[$b]);
            kani::cover!(n[$a].face() != n[$b].face(), "different faces");
            kani::cover!(n[$a].face() == n[$b].face(), "same face");
        }
    )* };
}

edge_neighbor_pair!(
    p_nbr_01__f0_p01 = (0, 0, 1),
    p_nbr_01__f0_p02 = (0, 0, 2),
    p_nbr_01__f0_p03 = (0, 0, 3),
    p_nbr_01__f0_p12 = (0, 1, 2),
    p_nbr_01__f0_p13 = (0, 1, 3),
    p_nbr_01__f0_p23 = (0, 2, 3),
    p_nbr_01__f1_p01 = (1, 0, 1),
    p_nbr_01__f1_p02 = (1, 0, 2),
    p_nbr_01__f1_p03 = (1, 0, 3),
    p_nbr_01__f1_p12 = (1, 1, 2),
    p_nbr_01__f1_p13 = (1, 1, 3),
    p_nbr_01__f1_p23 = (1, 2, 3),
    p_nbr_01__f2_p01 = (2, 0, 1),
    p_nbr_01__f2_p02 = (2, 0, 2),
    p_nbr_01__f2_p03 = (2, 0, 3),
    p_nbr_01__f2_p12 = (2, 1, 2),
    p_nbr_01__f2_p13 = (2, 1, 3),
    p_nbr_01__f2_p23 = (2, 2, 3),
    p_nbr_01__f3_p01 = (3, 0, 1),
    p_nbr_01__f3_p02 = (3, 0, 2),
    p_nbr_01__f3_p03 = (3, 0, 3),
    p_nbr_01__f3_p12 = (3, 1, 2),
    p_nbr_01__f3_p13 = (3, 1, 3),
    p_nbr_01__f3_p23 = (3, 2, 3),
    p_nbr_01__f4_p01 = (4, 0, 1),
    p_nbr_01__f4_p02 = (4, 0, 2),
    p_nbr_01__f4_p03 = (4, 0, 3),
    p_nbr_01__f4_p12 = (4, 1, 2),
    p_nbr_01__f4_p13 = (4, 1, 3),
    p_nbr_01__f4_p23 = (4, 2, 3),
    p_nbr_01__f5_p01 = (5, 0, 1),
    p_nbr_01__f5_p02 = (5, 0, 2),
    p_nbr_01__f5_p03 = (5, 0, 3),
    p_nbr_01__f5_p12 = (5, 1, 2),
    p_nbr_01__f5_p13 = (5, 1, 3),
    p_nbr_01__f5_p23 = (5, 2, 3),
);
