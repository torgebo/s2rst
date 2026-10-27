//! Survey probe for spec 008 (`Cap`), 2026-10-24. Library side only; the upstream
//! behaviour each case compares with is read from `s2cap.cc` (v0.14.0-40).

use s2rst::s1::{Angle, ChordAngle};
use s2rst::s2::metric::MIN_WIDTH;
use s2rst::s2::region_coverer::RegionCoverer;
use s2rst::s2::{Cap, Cell, CellEdge, CellId, LatLng, Point, Region};

/// Upstream `S2Cap::Intersects(cell, vertices)` (s2cap.cc:225), transcribed.
fn up_intersects(cap: Cap, cell: Cell, v: &[Point; 4]) -> bool {
    if cap.chord_radius() >= ChordAngle::RIGHT || cap.is_empty() {
        return false;
    }
    if cell.contains_point(cap.center()) {
        return true;
    }
    let sin2 = cap.chord_radius().sin2();
    let c = cap.center().vector();
    for k in 0..4 {
        let e = cell.edge_raw(CellEdge::ALL[k]).vector();
        let dot = c.dot(e);
        if dot > 0.0 {
            continue;
        }
        if dot * dot > sin2 * e.norm2() {
            return false;
        }
        let dir = e.cross(c);
        if dir.dot(v[k].vector()) < 0.0 && dir.dot(v[(k + 1) & 3].vector()) > 0.0 {
            return true;
        }
    }
    false
}

/// Upstream `S2Cap::Contains(const S2Cell&)` (s2cap.cc:276).
fn up_contains_cell(cap: Cap, cell: Cell) -> bool {
    let v = [cell.vertex(0), cell.vertex(1), cell.vertex(2), cell.vertex(3)];
    if v.iter().any(|&p| !cap.contains_point(p)) {
        return false;
    }
    !up_intersects(cap.complement(), cell, &v)
}

/// Upstream `S2Cap::MayIntersect(const S2Cell&)` (s2cap.cc:292).
fn up_may_intersect(cap: Cap, cell: Cell) -> bool {
    let v = [cell.vertex(0), cell.vertex(1), cell.vertex(2), cell.vertex(3)];
    if v.iter().any(|&p| cap.contains_point(p)) {
        return true;
    }
    up_intersects(cap, cell, &v)
}

fn ll(lat: f64, lng: f64) -> Point {
    LatLng::from_degrees(lat, lng).to_point()
}

fn main() {
    // K1: covering level. Library: floor(log2(d / r)) - 1; upstream: ilogb(d / r) - 1
    // (MIN_WIDTH.max_level). Search angle radii near d / 2^k.
    let d = 2.0 * std::f64::consts::SQRT_2 / 3.0;
    let mut shown = 0;
    let mut differ = 0;
    let center = ll(10.0, 20.0);
    for k in 1..=29 {
        let mut r = d / f64::from(1u32 << k);
        for _ in 0..64 {
            let cap = Cap::from_center_angle(center, Angle::from_radians(r));
            let ar = cap.angle_radius().radians();
            let lib = cap.cell_union_bound();
            let up_level = i32::from(u8::from(MIN_WIDTH.max_level(ar))) - 1;
            let lib_level = if lib.len() == 6 { -1 } else { i32::from(u8::from(lib[0].level())) };
            if lib_level != up_level {
                differ += 1;
                if shown < 3 {
                    shown += 1;
                    println!("K1 angle_radius={ar:e} library level={lib_level} upstream level={up_level}");
                }
            }
            r = r.next_up();
        }
    }
    println!("K1 radii whose covering level differs: {differ} of {}", 29 * 64);

    // K2: Region::contains_cell. Caps that contain every point of a cell, off the cell centre.
    let cell = Cell::from_cell_id(CellId::from_face(0));
    let mut k2 = 0;
    let mut k2_lib_true = 0;
    let mut k2_example = None;
    for dlat in 0..=12 {
        for rad in [56.0, 58.0, 60.0, 62.0, 64.0] {
            let cap = Cap::from_center_angle(ll(f64::from(dlat), 0.0), Angle::from_degrees(rad));
            let lib = cap.contains_cell(&cell);
            let up = up_contains_cell(cap, cell);
            k2_lib_true += i32::from(lib);
            if lib != up {
                k2 += 1;
                k2_example.get_or_insert((dlat, rad, lib, up));
            }
        }
    }
    println!("K2 contains_cell(face 0) true in {k2_lib_true} of 65 (library)");
    println!("K2 contains_cell(face 0) library != upstream: {k2} of 65; first {k2_example:?} (dlat°, radius°, library, upstream)");

    // K3: Region::intersects_cell. A cap in the gap between the face cell and its bounding cap.
    let cap = Cap::from_center_angle(ll(0.0, 50.0), Angle::from_degrees(2.0));
    println!(
        "K3 cap at (0°, 50°) radius 2° vs face 0: library intersects_cell={} upstream MayIntersect={}",
        cap.intersects_cell(&cell),
        up_may_intersect(cap, cell)
    );
    let coverer = RegionCoverer::new().max_cells(8);
    let cov = coverer.covering(&cap);
    let face0 = cov.iter().filter(|id| id.face().as_u8() == 0).count();
    println!("K3 its covering (max 8 cells): {} cells, {face0} on face 0 (the cap is on face 1 only)", cov.len());

    // K4: complement. Library STRAIGHT - r; upstream FromLength2(4 - r²). Bit for bit?
    let mut k4 = 0;
    let mut x: u64 = 0x9e37_79b9_7f4a_7c15;
    for _ in 0..1_000_000 {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        let l2 = (x >> 11) as f64 / (1u64 << 53) as f64 * 4.0;
        let r = ChordAngle::from_length2(l2);
        let lib = (ChordAngle::STRAIGHT - r).length2();
        let up = ChordAngle::from_length2(4.0 - r.length2()).length2();
        if lib.to_bits() != up.to_bits() {
            k4 += 1;
        }
    }
    println!("K4 complement radius differs from upstream: {k4} of 1000000");
    for l2 in [0.1, 0.5, 1.0, 1.7, 2.9, 3.3] {
        let c = Cap::from_center_chord_angle(ll(10.0, 20.0), ChordAngle::from_length2(l2));
        println!("K4 l2={l2:.17} complement l2={:.17}", c.complement().chord_radius().length2());
    }

    // K5: rect_bound's longitude reduction. Library (x + π).rem_euclid(2π) − π; upstream
    // remainder(x, 2π). A cap centred near longitude 0 with a tiny radius.
    let cap = Cap::from_center_angle(ll(0.0, 1e-14), Angle::from_radians(1e-18));
    let rb = cap.rect_bound();
    println!(
        "K5 cap at lng 1e-14°, radius 1e-18 rad: rect lng = [{:e}, {:e}], centre lng = {:e}, contains centre: {}",
        rb.lng.lo,
        rb.lng.hi,
        LatLng::from(cap.center()).lng.radians(),
        rb.contains_point(cap.center())
    );

    // K6: approx_eq with centres 5e-15 rad apart and equal radii (upstream: true at 1e-14).
    let a = ll(10.0, 20.0);
    let la = LatLng::from(a);
    let b = LatLng::from_radians(la.lat.radians() + 5e-15, la.lng.radians()).to_point();
    let (ca, cb) = (
        Cap::from_center_chord_angle(a, ChordAngle::from_length2(1.0)),
        Cap::from_center_chord_angle(b, ChordAngle::from_length2(1.0)),
    );
    println!("K6 centres {:.3e} rad apart: approx_eq={} approx_eq_with(1e-14)={}", a.distance(b).radians(), ca.approx_eq(cb), ca.approx_eq_with(cb, 1e-14));
}
