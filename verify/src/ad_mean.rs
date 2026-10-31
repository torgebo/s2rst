// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The region adaptors' meaning (spec 012 data-model §4.1): a union contains what some member
//! contains, an intersection what every member contains; the rectangle bound is the union
//! (intersection) of the members'. The members are abstract regions (`any_region`), so each
//! statement holds for every implementor. **P-ADBND-02** (the cap bound is the rectangle's cap
//! bound, the covering the cap's) is upstream's text, pinned by
//! `region_spec_tests::adaptors_are_upstreams`.

use s2rst::s2::Region;
use s2rst::s2::region_intersection::RegionIntersection;
use s2rst::s2::region_union::RegionUnion;

use crate::any_region::{AnyRegion, members, reset};
use crate::value_domain::{any_cell, any_triple};

/// P-ADCON-01 (correct result): `from_regions` keeps its members in order, and `add` appends one
/// (`RegionUnion`); `region(i)` is the `i`-th member (`RegionIntersection`).
///
/// Exactly: for three abstract members, member `k`'s answers (read through `iter`, `region(k)`)
/// are those of `AnyRegion { id: k }` for a cell and a point; after `add`, the last member's are
/// the added region's.
///
/// Stubs: `any_uv_bound`.
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn p_adcon_01() {
    reset();
    let (c, _) = any_cell();
    let p = any_triple();
    let mut u = RegionUnion::from_regions(members::<2>());
    u.add(Box::new(AnyRegion { id: 2 }));
    for (k, m) in u.iter().enumerate() {
        let r = AnyRegion { id: k as u8 };
        assert!(m.contains_cell(&c) == r.contains_cell(&c));
        assert!(m.contains_point(&p) == r.contains_point(&p));
    }
    let a = RegionIntersection::from_regions(members::<3>());
    for k in 0..3 {
        let r = AnyRegion { id: k as u8 };
        assert!(a.region(k).intersects_cell(&c) == r.intersects_cell(&c));
    }
}

macro_rules! p_adtst_01 {
    ($($name:ident = $n:literal),* $(,)?) => { $(
        #[doc = concat!(
            "P-ADTST-01 (correct result): the union's cell and point tests are `any` of the ",
            "members', the intersection's `all`.\n\nExactly: for exactly ", stringify!($n),
            " abstract members, every valid cell and every triple.\n\nStubs: `any_uv_bound`."
        )]
        #[kani::proof]
        #[kani::unwind(11)]
        #[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
        fn $name() {
            reset();
            let (c, _) = any_cell();
            let p = any_triple();
            let (mut any_c, mut any_i, mut any_p) = (false, false, false);
            let (mut all_c, mut all_i, mut all_p) = (true, true, true);
            for k in 0..$n {
                let m = AnyRegion { id: k as u8 };
                let (x, y, z) = (m.contains_cell(&c), m.intersects_cell(&c), m.contains_point(&p));
                any_c |= x;
                any_i |= y;
                any_p |= z;
                all_c &= x;
                all_i &= y;
                all_p &= z;
            }
            let u = RegionUnion::from_regions(members::<$n>());
            assert!(u.contains_cell(&c) == any_c && u.intersects_cell(&c) == any_i);
            assert!(u.contains_point(&p) == any_p);
            let a = RegionIntersection::from_regions(members::<$n>());
            assert!(a.contains_cell(&c) == all_c && a.intersects_cell(&c) == all_i);
            assert!(a.contains_point(&p) == all_p);
        }
    )* };
}

p_adtst_01!(
    p_adtst_01__n0 = 0,
    p_adtst_01__n1 = 1,
    p_adtst_01__n2 = 2,
    p_adtst_01__n3 = 3,
);

macro_rules! p_adbnd_01 {
    ($($(#[$attr:meta])* $name:ident = $n:literal),* $(,)?) => { $(
        #[doc = concat!(
            "P-ADBND-01 (correct result): the union's rectangle bound contains each member's, and ",
            "the intersection's holds every point that all the members' hold.\n\nExactly: for ",
            "exactly ", stringify!($n), " abstract members (any valid rectangles) and every ",
            "latitude and longitude. Not that each member's contains the intersection's: ",
            "`s1::Interval::intersection` returns the shorter operand when the overlap is two ",
            "pieces, and a near-full inverted interval's length rounds to 2π, a tie the full ",
            "interval wins, as upstream's (spec 012 research R0c)."
        )]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(11)]
        fn $name() {
            reset();
            let u = RegionUnion::from_regions(members::<$n>()).rect_bound();
            let i = RegionIntersection::from_regions(members::<$n>()).rect_bound();
            let (lat, lng): (f64, f64) = (kani::any(), kani::any());
            let mut in_all = true;
            for k in 0..$n {
                let m = AnyRegion { id: k as u8 }.rect_bound();
                assert!(u.contains(m));
                in_all &= crate::rect_spec::member(m, lat, lng);
            }
            if in_all {
                assert!(crate::rect_spec::member(i, lat, lng));
            }
        }
    )* };
}

p_adbnd_01!(
    p_adbnd_01__n1 = 1,
    p_adbnd_01__n2 = 2,
    #[cfg(feature = "slow")]
    p_adbnd_01__n3 = 3,
);

/// P-ADBND-03 (correct result): the empty union's bounds are the empty rectangle and cap, the
/// empty intersection's rectangle the full one, as upstream's (`Empty()`, `Full()`). The full
/// rectangle's cap bound is the full cap natively, C++ alike; under the range contracts of `asin`
/// and `atan2` the checker cannot fix that cap's angle, so that half is the native test
/// `region_intersection::tests::test_empty_intersection` and the C++ rows (spec 012 research R0c).
///
/// Under contract: `atan2_contract`, `asin_contract` (`Rect::cap_bound`).
#[kani::proof]
#[kani::stub(f64::atan2, crate::stubs::atan2_contract)]
#[kani::stub(f64::asin, crate::stubs::asin_contract)]
fn p_adbnd_03() {
    let u = RegionUnion::new();
    assert!(u.rect_bound().is_empty() && u.cap_bound().is_empty());
    let i = RegionIntersection::new();
    assert!(i.rect_bound().is_full());
}
