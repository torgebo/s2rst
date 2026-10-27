// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Differential tests for `s2::Cap` (spec 008 FR-024) against the C++ reference implementation
//! (google/s2geometry), Principle I.
//!
//! The vectors in `data/cpp_cap_vectors.csv` come from `tools/refgen-cpp/generate_cap`. Every
//! result is compared bit for bit, except where the library documents a deviation:
//! - `union`: the library adds the conversion errors to the radius; its radius is checked
//!   against C++'s within 10⁻¹², its centre within 10⁻¹⁵ (except for antipodal centres, where
//!   the direction is ill-conditioned). Neither union contains both caps on every row (K9, U-015): the
//!   counts are pinned.
//! - `RegionCoverer`'s covering of a cap differs from C++'s on 38 of 836 rows, though the cap's
//!   cell tests and `cell_union_bound` match: the coverer itself (K10, a follow-up); pinned.
//! - The constructors from a height or an area: spec 007's option B maps a negative length to
//!   `NEGATIVE` (−1), where upstream's `FromLength2` keeps it.
//!
//! `cell_union_bound` of the empty cap is upstream's four cells since spec 008's K8 (it was
//! none).

use s2rst::r3::Vector;
use s2rst::s1::{Angle, ChordAngle};
use s2rst::s2::region_coverer::RegionCoverer;
use s2rst::s2::{Cap, Cell, CellId, Point, Region};

const CSV: &str = include_str!("data/cpp_cap_vectors.csv");

fn section(name: &str) -> Vec<Vec<&'static str>> {
    let mut rows = Vec::new();
    let mut inside = false;
    for line in CSV.lines() {
        if let Some(h) = line.strip_prefix("# ") {
            inside = h == name;
            continue;
        }
        if inside && !line.is_empty() {
            rows.push(line.split(',').collect());
        }
    }
    assert!(!rows.is_empty(), "section {name} is empty or missing");
    rows
}

fn f(hex: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(hex, 16).unwrap())
}

fn d(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

fn b(x: bool) -> String {
    if x { "1" } else { "0" }.to_string()
}

fn pt(c: &[&str]) -> Point {
    Point(Vector {
        x: f(c[0]),
        y: f(c[1]),
        z: f(c[2]),
    })
}

fn ps(p: Point) -> [String; 3] {
    [d(p.0.x), d(p.0.y), d(p.0.z)]
}

/// The cap `S2Cap(c, FromLength2(l))`, or `Empty()` around `c` for `l = −1`.
fn cap(c: &[&str], l: &str) -> Cap {
    Cap::from_center_chord_angle(pt(c), ChordAngle::from_length2(f(l)))
}

/// The library's length² for upstream's `FromLength2(x)`: the same, except a negative length,
/// which spec 007's option B maps to −1.
fn option_b(x: f64) -> f64 {
    if x < 0.0 { -1.0 } else { x }
}

fn ids(s: &str) -> Vec<u64> {
    let mut v: Vec<u64> = s
        .split(' ')
        .filter(|t| !t.is_empty())
        .map(|t| u64::from_str_radix(t, 16).unwrap())
        .collect();
    v.sort_unstable();
    v
}

#[test]
fn cap_ops_match_cpp() {
    let mut n = 0;
    for name in ["CAP_OPS", "CAP_SPECIAL"] {
        for row in section(name) {
            // cx,cy,cz,l,px,py,pz,bx,by,bz,bl,d,h,a,ang,results,note
            let a = cap(&row[0..3], row[3]);
            let p = pt(&row[4..7]);
            let bc = cap(&row[7..10], row[10]);
            let (dist, h, area, ang) = (f(row[11]), f(row[12]), f(row[13]), f(row[14]));
            let r: Vec<&str> = row[15].split(';').collect();
            let ctx = row.join(",");
            let check = |i: usize, got: String, what: &str| {
                assert_eq!(got, r[i], "{what} (result {i}): {ctx}");
            };
            check(0, b(a.is_valid()), "is_valid");
            check(1, b(a.is_empty()), "is_empty");
            check(2, b(a.is_full()), "is_full");
            check(3, d(a.height()), "height");
            check(4, d(a.area()), "area");
            let c = ps(a.centroid());
            for k in 0..3 {
                check(5 + k, c[k].clone(), "centroid");
            }
            check(8, b(a.contains_point(p)), "contains_point");
            check(
                9,
                b(a.interior_contains_point(p)),
                "interior_contains_point",
            );
            check(10, b(a.contains(bc)), "contains");
            check(11, b(a.intersects(bc)), "intersects");
            check(12, b(a.interior_intersects(bc)), "interior_intersects");
            let k = a.complement();
            let kc = ps(k.center());
            for i in 0..3 {
                check(13 + i, kc[i].clone(), "complement centre");
            }
            check(16, d(k.chord_radius().length2()), "complement radius");
            check(
                17,
                d(a.expanded(Angle::from_radians(dist))
                    .chord_radius()
                    .length2()),
                "expanded",
            );
            // union: the documented margin; the radius within 1e-12 of C++'s. Containment is
            // counted in `union_containment_matches_the_record` (K9).
            let u = a.union(bc);
            let cpp_union_l2 = f(r[21]);
            if u.is_empty() {
                assert!(cpp_union_l2 < 0.0, "union of two empty caps: {ctx}");
            } else {
                // The centre within 1e-15 of C++'s (the larger radius moves it by rounding, 2.8e-16
                // at most). Antipodal centres are skipped: every great circle through both centres
                // serves, so the direction is ill-conditioned (up to 5.3e-3 apart).
                if row[16] != "antipodal-centres" {
                    let cpp_c = pt(&r[18..21]);
                    assert!(
                        (u.center().0 - cpp_c.0).norm() <= 1e-15,
                        "union centre {:?} against C++ {:?}: {ctx}",
                        u.center(),
                        cpp_c
                    );
                }
                assert!(
                    (u.chord_radius().length2() - cpp_union_l2).abs() <= 1e-12,
                    "union radius {} against C++ {cpp_union_l2}: {ctx}",
                    u.chord_radius().length2()
                );
            }
            let ap = a.add_point(p);
            check(22, d(ap.chord_radius().length2()), "add_point radius");
            let apc = ps(ap.center());
            for i in 0..3 {
                check(23 + i, apc[i].clone(), "add_point centre");
            }
            let ac = a.add_cap(bc);
            check(26, d(ac.chord_radius().length2()), "add_cap radius");
            let acc = ps(ac.center());
            for i in 0..3 {
                check(27 + i, acc[i].clone(), "add_cap centre");
            }
            check(30, b(a.approx_eq(bc)), "approx_eq");
            check(31, b(a.approx_eq_with(bc, 1e-10)), "approx_eq_with");
            check(32, b(a == bc), "==");
            let center = pt(&row[0..3]);
            assert_eq!(
                d(Cap::from_center_height(center, h).chord_radius().length2()),
                d(option_b(f(r[33]))),
                "from_center_height: {ctx}"
            );
            assert_eq!(
                d(Cap::from_center_area(center, area).chord_radius().length2()),
                d(option_b(f(r[34]))),
                "from_center_area: {ctx}"
            );
            check(
                35,
                d(Cap::from_center_angle(center, Angle::from_radians(ang))
                    .chord_radius()
                    .length2()),
                "from_center_angle",
            );
            check(36, d(a.angle_radius().radians()), "angle_radius");
            n += 1;
        }
    }
    assert_eq!(n, 1089);
}

#[test]
fn cap_bounds_match_cpp() {
    let coverer = RegionCoverer::new().max_cells(8);
    let (mut n, mut coverings_differing) = (0, 0);
    for row in section("CAP_BOUNDS") {
        // cx,cy,cz,l,results,note
        let a = cap(&row[0..3], row[3]);
        let r: Vec<&str> = row[4].split(';').collect();
        let ctx = row.join(",");
        let rect = a.rect_bound();
        let got = [
            d(rect.lat.lo),
            d(rect.lat.hi),
            d(rect.lng.lo),
            d(rect.lng.hi),
        ];
        for i in 0..4 {
            assert_eq!(got[i], r[i], "rect_bound (result {i}): {ctx}");
        }
        let mut bound: Vec<u64> = a.cell_union_bound().iter().map(|id| id.id()).collect();
        bound.sort_unstable();
        assert_eq!(bound, ids(r[4]), "cell_union_bound: {ctx}");
        let mut cov: Vec<u64> = coverer.covering(&a).iter().map(|id| id.id()).collect();
        cov.sort_unstable();
        if cov != ids(r[5]) {
            // K10: `RegionCoverer` itself differs from C++'s on some caps, though the cap's cell
            // tests and `cell_union_bound` match (outside spec 008's scope; a follow-up).
            assert!(cov.len() <= 8, "covering within max_cells: {ctx}");
            coverings_differing += 1;
        }
        n += 1;
    }
    assert_eq!(n, 836);
    assert_eq!(
        coverings_differing, 38,
        "K10's count changed: re-check RegionCoverer against C++"
    );
}

/// K9 and U-015: neither upstream's `Union` nor the library's contains both caps on every row,
/// though upstream's comment says "encloses"; the library's added margin narrows the gap. The
/// counts are pinned so that a change either way is noticed.
#[test]
fn union_containment_matches_the_record() {
    let (mut lib, mut cpp, mut n) = (0, 0, 0);
    for name in ["CAP_OPS", "CAP_SPECIAL"] {
        for row in section(name) {
            let (a, bc) = (cap(&row[0..3], row[3]), cap(&row[7..10], row[10]));
            let r: Vec<&str> = row[15].split(';').collect();
            let u = a.union(bc);
            lib += usize::from(!(u.contains(a) && u.contains(bc)));
            let cu =
                Cap::from_center_chord_angle(pt(&r[18..21]), ChordAngle::from_length2(f(r[21])));
            cpp += usize::from(!(cu.contains(a) && cu.contains(bc)));
            n += 1;
        }
    }
    assert_eq!(
        (lib, cpp, n),
        (7, 337, 1089),
        "union containment failures (library, C++, rows)"
    );
}

#[test]
fn cap_cell_tests_match_cpp() {
    let mut n = 0;
    for row in section("CAP_CELL") {
        // cx,cy,cz,l,cell,results,note
        let a = cap(&row[0..3], row[3]);
        let cell = Cell::from_cell_id(CellId::from(u64::from_str_radix(row[4], 16).unwrap()));
        let r: Vec<&str> = row[5].split(';').collect();
        let ctx = row.join(",");
        assert_eq!(b(a.contains_cell(&cell)), r[0], "contains_cell: {ctx}");
        assert_eq!(b(a.intersects_cell(&cell)), r[1], "intersects_cell: {ctx}");
        n += 1;
    }
    assert_eq!(n, 1066);
}
