// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Börresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Crash freedom of `RegionUnion` and `RegionIntersection` (spec 012 US3, FR-004; data-model §5;
//! research R0). The members are abstract regions (`any_region`), whose methods never panic, so a
//! crash proof here is about the adaptor's own code: any members whose methods do not panic on the
//! arguments give an adaptor whose methods do not. Member counts are concrete (`__n<k>`); the
//! bounds go through `Rect::union`, `intersection` and `cap_bound` (and `atan2`, `asin` there),
//! which cost minutes from one member up, so those members run weekly. `cell_union_bound` is the
//! cap bound's `Cap::cell_union_bound`, spec 008's tool limit, inherited.

use s2rst::s2::Region;
use s2rst::s2::region_intersection::RegionIntersection;
use s2rst::s2::region_union::RegionUnion;

use crate::any_region::{AnyRegion, members, reset};
use crate::value_domain::{any_cell, any_triple};

macro_rules! c_api {
    ($($name_u:ident, $name_i:ident = $n:literal),* $(,)?) => { $(
        #[doc = concat!(
            "C-RegionUnion::new, from_regions, add, len, is_empty, iter, both IntoIterators, ",
            "Default: complete for exactly ", stringify!($n), " members."
        )]
        #[kani::proof]
        #[kani::unwind(6)]
        fn $name_u() {
            reset();
            let mut u = RegionUnion::from_regions(members::<$n>());
            assert!(u.len() == $n && u.is_empty() == ($n == 0));
            assert!(u.iter().count() == $n && (&u).into_iter().count() == $n);
            u.add(Box::new(AnyRegion { id: 3 }));
            assert!(u.len() == $n + 1);
            assert!(u.into_iter().count() == $n + 1);
            assert!(RegionUnion::default().is_empty() && RegionUnion::new().is_empty());
        }

        #[doc = concat!(
            "C-RegionIntersection::new, from_regions, num_regions, region (in range), Default: ",
            "complete for exactly ", stringify!($n), " members."
        )]
        #[kani::proof]
        #[kani::unwind(6)]
        fn $name_i() {
            reset();
            let a = RegionIntersection::from_regions(members::<$n>());
            assert!(a.num_regions() == $n);
            for k in 0..$n {
                let _ = a.region(k);
            }
            assert!(RegionIntersection::default().num_regions() == 0);
            assert!(RegionIntersection::new().num_regions() == 0);
        }
    )* };
}

c_api!(
    c_union_api__n0,
    c_intersection_api__n0 = 0,
    c_union_api__n1,
    c_intersection_api__n1 = 1,
    c_union_api__n2,
    c_intersection_api__n2 = 2,
    c_union_api__n3,
    c_intersection_api__n3 = 3,
);

/// N-RegionIntersection::region: panics for every index past the end, in every build (spec 012
/// K2, FR-017).
#[kani::proof]
#[kani::should_panic]
#[kani::unwind(5)]
fn n_region_past_end() {
    reset();
    let a = RegionIntersection::from_regions(members::<2>());
    let i: usize = kani::any();
    kani::assume(i >= 2);
    let _ = a.region(i);
    kani::cover!(true, "returns");
}

macro_rules! c_tests {
    ($($name_u:ident, $name_i:ident = $n:literal),* $(,)?) => { $(
        #[doc = concat!(
            "C-Region::contains_cell, intersects_cell, contains_point for `RegionUnion` and ",
            "`RegionIntersection`: complete for exactly ", stringify!($n), " members, every ",
            "valid cell and every triple.\n\nStubs: `any_uv_bound`."
        )]
        #[kani::proof]
        #[kani::unwind(11)]
        #[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
        fn $name_u() {
            reset();
            let a = RegionUnion::from_regions(members::<$n>());
            let (c, _) = any_cell();
            let p = any_triple();
            let x = (a.contains_cell(&c), a.intersects_cell(&c), a.contains_point(&p));
            kani::cover!(x.2 || $n == 0, "a contained point");
        }

        #[doc = concat!("As the union's, for `RegionIntersection` of ", stringify!($n), " members.")]
        #[kani::proof]
        #[kani::unwind(11)]
        #[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
        fn $name_i() {
            reset();
            let a = RegionIntersection::from_regions(members::<$n>());
            let (c, _) = any_cell();
            let p = any_triple();
            let x = (a.contains_cell(&c), a.intersects_cell(&c), a.contains_point(&p));
            kani::cover!(!x.2 || $n == 0, "a point outside");
        }
    )* };
}

c_tests!(
    c_tests__ru__n0,
    c_tests__ri__n0 = 0,
    c_tests__ru__n1,
    c_tests__ri__n1 = 1,
    c_tests__ru__n2,
    c_tests__ri__n2 = 2,
    c_tests__ru__n3,
    c_tests__ri__n3 = 3,
);

macro_rules! c_bounds {
    ($($(#[$attr:meta])* $name_u:ident, $name_i:ident = $n:literal),* $(,)?) => { $(
        #[doc = concat!(
            "C-Region::rect_bound, cap_bound for `RegionUnion` and `RegionIntersection`: ",
            "complete for exactly ", stringify!($n), " members.\n\nUnder contract: ",
            "`atan2_contract`, `asin_contract` (`Rect::cap_bound`)."
        )]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(11)]
        #[kani::stub(f64::atan2, crate::stubs::atan2_contract)]
        #[kani::stub(f64::asin, crate::stubs::asin_contract)]
        fn $name_u() {
            reset();
            let a = RegionUnion::from_regions(members::<$n>());
            let r = a.rect_bound();
            let c = a.cap_bound();
            kani::cover!(r.is_empty() == ($n == 0) && c.is_empty() == ($n == 0), "the bounds");
        }

        #[doc = concat!("As the union's, for `RegionIntersection` of ", stringify!($n), " members.")]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(11)]
        #[kani::stub(f64::atan2, crate::stubs::atan2_contract)]
        #[kani::stub(f64::asin, crate::stubs::asin_contract)]
        fn $name_i() {
            reset();
            let a = RegionIntersection::from_regions(members::<$n>());
            let r = a.rect_bound();
            let c = a.cap_bound();
            kani::cover!(r.is_full() == ($n == 0) && c.is_full() == ($n == 0), "the bounds");
        }
    )* };
}

c_bounds!(
    c_bounds__ru__n0,
    c_bounds__ri__n0 = 0,
    // R0: 259.5 s (one member), 308.2 s (two), 342.6 and 579.5 s (three): weekly.
    #[cfg(feature = "slow")]
    c_bounds__ru__n1,
    c_bounds__ri__n1 = 1,
    #[cfg(feature = "slow")]
    c_bounds__ru__n2,
    c_bounds__ri__n2 = 2,
);
