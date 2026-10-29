// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! `CellUnion::expand_by_radius` (spec 011 data-model §4.2, §5.2).
//!
//! **P-UEXP-01** (correct result), proven by composition: the level `expand_by_radius` expands at
//! is upstream's. The level arithmetic is upstream's text (`region_spec_tests::expand_level_is_upstreams`,
//! a drift guard), and the metric's level for the radius, `MIN_WIDTH.max_level(r)`, is
//! upstream's `GetLevelForMinValue(r)` for every finite radius (spec 008's P-CPBND-02); the C++
//! vectors (`CELLUNION_EXPAND`, 1,000 rows) check the results.
//!
//! **P-UEXP-02** (correct result), not proven: that the result contains the original union.
//! The expansion is `expand_at_level`, whose `all_neighbors` runs out of memory above 9.5 GB
//! (001 R12), so neither its crash freedom nor this statement is in reach; the vectors and the
//! unit tests check it.

use s2rst::s1::Angle;
use s2rst::s2::CellUnion;

use crate::union_domain::any_valid_union;

/// N-CellUnion::expand_by_radius: in debug builds, panics for every valid one-cell union and
/// every `max_level_diff` when the radius is NaN: the level lookup checks it before any
/// expansion (spec 011 K4; upstream's `GetLevelForMinValue` `DCHECK`s it).
#[cfg(debug_assertions)]
#[kani::proof]
#[kani::should_panic]
#[kani::unwind(5)]
fn n_expand_by_radius_nan() {
    let (v, _) = any_valid_union::<1>();
    let mut u = CellUnion::from_verbatim(v);
    let r: f64 = kani::any();
    kani::assume(r.is_nan());
    u.expand_by_radius(Angle::from_radians(r), kani::any());
    kani::cover!(true, "returns");
}
