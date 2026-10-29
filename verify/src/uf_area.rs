// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! `CellUnion`'s areas (spec 011 data-model §4.2; research R0). **P-UAREA-04**, that
//! `Cell::average_area_for_level` is upstream's `AverageArea`, is proven by enumeration of the
//! 31 levels (`tests/area_enumeration.rs`): the checker treats `powi` as it treats `sqrt`
//! (KI-013), so a statement here that compares two of its results uses `pure_avg_area`.

use s2rst::s2::{Cell, CellUnion, Level};

use crate::union_domain::any_valid_union;

/// P-UAREA-01 (correct result): the average-based area is the leaf area times the number of
/// leaves covered, as upstream's `AverageBasedArea`.
///
/// Exactly: for every valid one-cell union, `average_based_area()` has the bits of
/// `Cell::average_area_for_level(30) * leaf_cells_covered() as f64`.
///
/// Stubs: `pure_avg_area` (one value per level). R0: two and three cells time out at 12 minutes.
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::Cell::average_area_for_level, crate::stubs::pure_avg_area)]
fn p_uarea_01__n1() {
    crate::stubs::reset();
    let (v, _) = any_valid_union::<1>();
    let u = CellUnion::from_verbatim(v);
    let e = Cell::average_area_for_level(Level::MAX) * u.leaf_cells_covered() as f64;
    assert!(u.average_based_area().to_bits() == e.to_bits());
}

macro_rules! p_uarea_sum {
    ($id:literal, $method:ident, $path:path, $($(#[$attr:meta])* $name:ident = $n:literal),* $(,)?) => { $(
        #[doc = concat!(
            $id, ": `", stringify!($method), "` is the sum of the cells' areas ",
            "in order from +0, as upstream's (`double area = 0`), +0 for the empty union (spec ",
            "011 FR-018).\n\n",
            "Exactly: for every valid union of exactly ", stringify!($n), " cells, `",
            stringify!($method), "()` has the bits of `0.0 + a₀ + … + aₙ₋₁`, `aₖ` the cells' ",
            "`", stringify!($method), "()`.\n\nStubs: `pure_cell_area` (one value per cell), ",
            "`any_uv_bound`."
        )]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(11)]
        #[kani::stub($path, crate::stubs::pure_cell_area)]
        #[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
        fn $name() {
            crate::stubs::reset();
            let (v, _) = any_valid_union::<$n>();
            let u = CellUnion::from_verbatim(v.clone());
            let mut s = 0.0;
            for k in 0..$n {
                s += Cell::from_cell_id(v[k]).$method();
            }
            assert!(u.$method().to_bits() == s.to_bits());
        }
    )* };
}

p_uarea_sum!(
    "P-UAREA-02 (correct result)",
    approx_area,
    s2rst::s2::Cell::approx_area,
    p_uarea_02__n0 = 0,
    p_uarea_02__n1 = 1,
    // Weekly, as `__n3`: over 100 s of solver time, and far more wall time per change (2026-10-29).
    #[cfg(feature = "slow")]
    p_uarea_02__n2 = 2,
    // R0b: 429.7 s.
    #[cfg(feature = "slow")]
    p_uarea_02__n3 = 3,
);

p_uarea_sum!(
    "P-UAREA-03 (correct result)",
    exact_area,
    s2rst::s2::Cell::exact_area,
    p_uarea_03__n0 = 0,
    p_uarea_03__n1 = 1,
    // Weekly: 88.4 s of solver time but about 16 minutes of wall time per change (2026-10-29).
    #[cfg(feature = "slow")]
    p_uarea_03__n2 = 2,
    #[cfg(feature = "slow")]
    p_uarea_03__n3 = 3,
);
