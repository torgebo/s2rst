// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Generic trait laws, each stated once over a type parameter and instantiated per type, as
//! spec 003's `Codec` contract is (spec 004 data-model §4.3, research R15; VERIFIED_TYPES §3).
//!
//! A law is a plain function that asserts. It is never a harness: an instance harness draws
//! the values with its own generators and calls the law. A law states only what every
//! implementor must satisfy; an instance's own meaning (for example, agreement with the cell
//! model) stays in that type's catalogue entries. The quickcheck twins of these laws are in
//! `core/tests/trait_property_tests.rs`, and `CORE_TRAITS.md` names both.

/// The integer laws of the `Region` trait (`CORE_TRAITS.md`, "Region").
pub mod region {
    use s2rst::s2::{Cell, Point, Region};

    /// Containing a cell implies intersecting it.
    ///
    /// Instances: P-CCNT-03 (`Cell`).
    pub fn contains_implies_intersects<R: Region>(r: &R, c: &Cell) {
        if r.contains_cell(c) {
            assert!(
                r.intersects_cell(c),
                "contains_cell without intersects_cell"
            );
        }
    }

    /// Every cell the region contains is covered by its covering bound: some identifier of
    /// `cell_union_bound()` contains the cell's. The bound has at most `MAX` identifiers, which
    /// the law asserts, so the search runs over a concrete length (collections are never
    /// compared with `==` in a harness).
    ///
    /// Instances: P-CCNT-03 (`Cell`, `MAX = 1`).
    pub fn covering_covers<R: Region, const MAX: usize>(r: &R, c: &Cell) {
        let ids = r.cell_union_bound();
        assert!(
            ids.len() <= MAX,
            "the covering bound has more ids than the instance allows"
        );
        if r.contains_cell(c) {
            let mut covered = false;
            for (k, id) in ids.iter().enumerate().take(MAX) {
                if k < ids.len() && id.contains(c.id()) {
                    covered = true;
                }
            }
            assert!(covered, "a contained cell outside the covering bound");
        }
    }

    /// Both `Region` laws.
    pub fn all<R: Region, const MAX: usize>(r: &R, c: &Cell) {
        contains_implies_intersects(r, c);
        covering_covers::<R, MAX>(r, c);
    }

    /// A cell the region does not intersect has no vertex the region contains (`CORE_TRAITS.md`,
    /// "Non-intersection implies non-containment").
    ///
    /// Not every implementor satisfies it: a region whose cell test is by id and whose point set
    /// is closed (`Cell`, as upstream's `S2Cell`) contains the shared vertices of a neighbour it
    /// does not intersect. It is a law of regions whose cell test checks the vertices first, as
    /// `Cap`'s (upstream's `MayIntersect`).
    ///
    /// The vertices are those of `c.vertex(k)`. An instance whose region computes them itself
    /// (`Cap`'s upstream cell tests) stubs `Cell::vertex` with a pure function, so the law reads
    /// the values the region's test read: a recomputed normalisation is not one value to the
    /// checker (spec 008 spec Edge Cases, "Two `sqrt` results"; research R7).
    ///
    /// Instances: P-CPREG-02 (`Cap`). Quickcheck twin: `region_no_intersect_no_vertex` in
    /// `core/tests/trait_property_tests.rs`.
    pub fn no_intersect_no_vertex<R: Region>(r: &R, c: &Cell) {
        if !r.intersects_cell(c) {
            for k in s2rst::s2::CellVertex::ALL {
                assert!(
                    !r.contains_point(&c.vertex(k)),
                    "a vertex of a cell the region does not intersect is contained"
                );
            }
        }
    }

    /// The bounding cap contains every point the region contains (`CORE_TRAITS.md`, "Bounding
    /// cap contains contained points").
    ///
    /// Instances: P-CPREG-03 (`Cap`). Quickcheck twin: `region_cap_bound_contains_point` in
    /// `core/tests/trait_property_tests.rs`.
    pub fn cap_bound_contains_points<R: Region>(r: &R, p: &Point) {
        if r.contains_point(p) {
            assert!(
                r.cap_bound().contains_point(*p),
                "a contained point outside the bounding cap"
            );
        }
    }
}

/// The laws of `PartialEq`: an equivalence relation on the instance's domain.
pub mod eq {
    /// `a == a`.
    ///
    /// Instances: P-CEQ-01 (`Cell`), P-IVAL-05 (`s1::Interval`), P-QEQ-01 (`r2::Rect`),
    /// P-PTVAL-03 (`s2::Point`).
    pub fn reflexive<T: PartialEq>(a: &T) {
        assert!(a == a, "equality is not reflexive");
    }

    /// `a == b` exactly when `b == a`.
    pub fn symmetric<T: PartialEq>(a: &T, b: &T) {
        assert_eq!(a == b, b == a, "equality is not symmetric");
    }

    /// `a == b` and `b == c` imply `a == c`.
    pub fn transitive<T: PartialEq>(a: &T, b: &T, c: &T) {
        if a == b && b == c {
            assert!(a == c, "equality is not transitive");
        }
    }

    /// The three laws.
    pub fn all<T: PartialEq>(a: &T, b: &T, c: &T) {
        reflexive(a);
        symmetric(a, b);
        transitive(a, b, c);
    }
}

/// The laws of a total order, stated through a three-way comparison so that they apply to a
/// method such as `Point::cmp_point` as well as to `Ord::cmp` (spec 006 data-model §4,
/// "Generic order laws"). The comparison returns [`Ordering`](std::cmp::Ordering) values,
/// which the laws compare with `==` only: Kani 0.68.0 mis-models `<` on symbolic booleans.
///
/// Together the laws make the comparison a total preorder whose ties are exactly `==`, so a
/// total order on the instance's domain. They are generic over the comparison so that
/// `ChordAngle` can reuse them (spec 006 FR-025).
///
/// Instances: P-PTORD-02 (`s2::Point`, `cmp_point` over triples without NaN).
pub mod ord {
    use std::cmp::Ordering;

    /// `cmp(a, a)` is `Equal`.
    pub fn reflexive<T, F: Fn(&T, &T) -> Ordering>(cmp: &F, a: &T) {
        assert!(cmp(a, a) == Ordering::Equal, "the order is not reflexive");
    }

    /// `cmp(b, a)` is `cmp(a, b)` reversed.
    pub fn dual<T, F: Fn(&T, &T) -> Ordering>(cmp: &F, a: &T, b: &T) {
        assert!(
            cmp(b, a) == cmp(a, b).reverse(),
            "the order is not antisymmetric"
        );
    }

    /// `a ≤ b` and `b ≤ c` imply `a ≤ c`, and the same for `<`.
    pub fn transitive<T, F: Fn(&T, &T) -> Ordering>(cmp: &F, a: &T, b: &T, c: &T) {
        let (ab, bc, ac) = (cmp(a, b), cmp(b, c), cmp(a, c));
        if ab != Ordering::Greater && bc != Ordering::Greater {
            assert!(ac != Ordering::Greater, "≤ is not transitive");
        }
        if ab == Ordering::Less && bc == Ordering::Less {
            assert!(ac == Ordering::Less, "< is not transitive");
        }
    }

    /// `cmp(a, b)` is `Equal` exactly when `a == b`.
    pub fn ties_are_eq<T: PartialEq, F: Fn(&T, &T) -> Ordering>(cmp: &F, a: &T, b: &T) {
        assert_eq!(
            cmp(a, b) == Ordering::Equal,
            a == b,
            "the order's ties are not equality"
        );
    }

    /// The four laws.
    pub fn all<T: PartialEq, F: Fn(&T, &T) -> Ordering>(cmp: &F, a: &T, b: &T, c: &T) {
        reflexive(cmp, a);
        dual(cmp, a, b);
        transitive(cmp, a, b, c);
        ties_are_eq(cmp, a, b);
    }
}
