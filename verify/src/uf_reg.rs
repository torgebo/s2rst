// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The generic `Region` laws for `CellUnion` (spec 011 US3, FR-012; data-model §4.2; research
//! R6). Only containment implies intersection is proven here, within size bounds. The others:
//!
//! - **P-UREG-02**, the covering covers every contained cell, and **P-UREG-06**, the covering
//!   contains every contained point: not proven. The covering is the cap bound's
//!   (`Cap::cell_union_bound`), whose level needs `asin`'s value, as spec 008's P-CPREG-04; and
//!   its crash freedom is spec 008's tool limit. Tested: the survey's `UUC` (every cell of 10,000
//!   unions covered, C++ and library alike) and the quickcheck twins.
//! - **P-UREG-03**, the vertex law: not a law of `CellUnion`, by design and in upstream too. Its
//!   cell test is by id and its point test by leaf, so a union can contain the shared vertex of a
//!   neighbour it does not intersect: the union of cell `14` contains a vertex of `1c`
//!   (`core/src/s2/cell_union_tests.rs`, `vertex_law_does_not_hold`; the survey's `UV` section,
//!   20,573 of the vertices of 39,636 non-intersected neighbours).
//! - **P-UREG-04**, the bounding cap contains every contained point, and **P-UREG-05**, the
//!   bounding rectangle does: not proven. A union's cap bound contains each cell's cap bound
//!   (P-UBND-02) and its rectangle bound each cell's (P-UBND-01); what remains is that a cell's
//!   own bounds contain the points whose leaf lies in the cell, `Cell`'s bounding laws, which
//!   need the projections' values (spec 004's float methods, a follow-up). Tested: the survey's
//!   `UG` (262,154 contained points, no miss, C++ and library alike) and the twins.

use s2rst::s2::CellUnion;

use crate::laws;
use crate::union_domain::any_valid_union;
use crate::value_domain::any_cell;

macro_rules! p_ureg_01 {
    ($($name:ident = $n:literal),* $(,)?) => { $(
        #[doc = concat!(
            "P-UREG-01 (law): containing a cell implies intersecting it ",
            "(`laws::region::contains_implies_intersects`).\n\n",
            "Exactly: for every valid union of exactly ", stringify!($n), " cells and every ",
            "valid cell, `contains_cell(c)` implies `intersects_cell(c)`.\n\nStubs: ",
            "`any_uv_bound` (the cell is read through its id only)."
        )]
        #[kani::proof]
        #[kani::unwind(11)]
        #[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
        fn $name() {
            let (v, _) = any_valid_union::<$n>();
            let u = CellUnion::from_verbatim(v);
            let (c, _) = any_cell();
            laws::region::contains_implies_intersects(&u, &c);
            kani::cover!(s2rst::s2::Region::contains_cell(&u, &c) || $n == 0, "a contained cell");
        }
    )* };
}

p_ureg_01!(
    p_ureg_01__n0 = 0,
    p_ureg_01__n1 = 1,
    p_ureg_01__n2 = 2,
    p_ureg_01__n3 = 3,
);
