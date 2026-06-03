// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use s2rst_wasm::*;
use wasm_bindgen_test::*;

// ===========================================================================
// Version
// ===========================================================================

#[wasm_bindgen_test]
fn test_version() {
    assert_eq!(version(), "0.1.0");
}

// ===========================================================================
// Angle
// ===========================================================================

#[wasm_bindgen_test]
fn test_angle_from_degrees() {
    let a = Angle::from_degrees(90.0);
    assert!((a.degrees() - 90.0).abs() < 1e-12);
    assert!((a.radians() - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
}

#[wasm_bindgen_test]
fn test_angle_from_radians() {
    let a = Angle::from_radians(std::f64::consts::PI);
    assert!((a.degrees() - 180.0).abs() < 1e-12);
}

#[wasm_bindgen_test]
fn test_angle_from_e5_e6_e7() {
    let a5 = Angle::from_e5(4500000); // 45 degrees
    assert!((a5.degrees() - 45.0).abs() < 1e-5);

    let a6 = Angle::from_e6(45000000);
    assert!((a6.degrees() - 45.0).abs() < 1e-6);

    let a7 = Angle::from_e7(450000000);
    assert!((a7.degrees() - 45.0).abs() < 1e-7);
}

#[wasm_bindgen_test]
fn test_angle_e5_e6_e7_getters() {
    let a = Angle::from_degrees(45.0);
    assert_eq!(a.e5(), 4500000);
    assert_eq!(a.e6(), 45000000);
    assert_eq!(a.e7(), 450000000);
}

#[wasm_bindgen_test]
fn test_angle_zero_infinity() {
    let z = Angle::zero();
    assert!((z.radians()).abs() < 1e-15);

    let inf = Angle::infinity_val();
    assert!(inf.is_infinite());

    assert!(!z.is_infinite());
}

#[wasm_bindgen_test]
fn test_angle_arithmetic() {
    let a = Angle::from_degrees(30.0);
    let b = Angle::from_degrees(60.0);

    let sum = a.add(&b);
    assert!((sum.degrees() - 90.0).abs() < 1e-12);

    let diff = b.sub(&a);
    assert!((diff.degrees() - 30.0).abs() < 1e-12);

    let scaled = a.mul(3.0);
    assert!((scaled.degrees() - 90.0).abs() < 1e-12);

    let halved = b.div(2.0);
    assert!((halved.degrees() - 30.0).abs() < 1e-12);

    let ratio = b.ratio(&a);
    assert!((ratio - 2.0).abs() < 1e-12);

    let neg = a.neg();
    assert!((neg.degrees() + 30.0).abs() < 1e-12);
}

#[wasm_bindgen_test]
fn test_angle_abs() {
    let a = Angle::from_degrees(-45.0);
    let abs = a.abs();
    assert!((abs.degrees() - 45.0).abs() < 1e-12);
}

#[wasm_bindgen_test]
fn test_angle_normalized() {
    let a = Angle::from_degrees(270.0);
    let n = a.normalized();
    assert!((n.degrees() - (-90.0)).abs() < 1e-10);
}

#[wasm_bindgen_test]
fn test_angle_trig() {
    let a = Angle::from_degrees(90.0);
    assert!((a.sin() - 1.0).abs() < 1e-12);
    assert!(a.cos().abs() < 1e-12);

    let b = Angle::from_degrees(45.0);
    assert!((b.tan() - 1.0).abs() < 1e-12);
}

#[wasm_bindgen_test]
fn test_angle_approx_eq() {
    let a = Angle::from_degrees(90.0);
    let b = Angle::from_degrees(90.0);
    assert!(a.approx_eq(&b));

    let c = Angle::from_degrees(91.0);
    assert!(!a.approx_eq(&c));
}

#[wasm_bindgen_test]
fn test_angle_to_string() {
    let a = Angle::from_degrees(45.0);
    let s = a.to_string_js();
    assert!(!s.is_empty());
}

// ===========================================================================
// ChordAngle
// ===========================================================================

#[wasm_bindgen_test]
fn test_chord_angle_constants() {
    assert!(ChordAngle::zero().is_zero());
    assert!(ChordAngle::infinity_val().is_infinity());
    assert!(ChordAngle::negative().is_negative());
    assert!(ChordAngle::right().is_valid());
    assert!(ChordAngle::straight().is_valid());
}

#[wasm_bindgen_test]
fn test_chord_angle_from_length2() {
    let ca = ChordAngle::from_length2(2.0);
    assert!((ca.length2() - 2.0).abs() < 1e-12);
}

#[wasm_bindgen_test]
fn test_chord_angle_from_angle() {
    let a = Angle::from_degrees(90.0);
    let ca = ChordAngle::from_angle(&a);
    assert!((ca.degrees() - 90.0).abs() < 1e-6);
}

#[wasm_bindgen_test]
fn test_chord_angle_from_radians_degrees() {
    let ca = ChordAngle::from_radians(std::f64::consts::FRAC_PI_2);
    assert!((ca.degrees() - 90.0).abs() < 1e-6);
    assert!((ca.radians() - std::f64::consts::FRAC_PI_2).abs() < 1e-6);

    let ca2 = ChordAngle::from_degrees(180.0);
    assert!((ca2.degrees() - 180.0).abs() < 1e-6);
}

#[wasm_bindgen_test]
fn test_chord_angle_to_angle() {
    let ca = ChordAngle::from_degrees(60.0);
    let a = ca.to_angle();
    assert!((a.degrees() - 60.0).abs() < 1e-6);
}

#[wasm_bindgen_test]
fn test_chord_angle_is_predicates() {
    let z = ChordAngle::zero();
    assert!(z.is_zero());
    assert!(!z.is_negative());
    assert!(!z.is_infinity());
    assert!(!z.is_special());
    assert!(z.is_valid());
}

#[wasm_bindgen_test]
fn test_chord_angle_successor_predecessor() {
    let z = ChordAngle::zero();
    let s = z.successor();
    assert!(!s.is_zero());
    let back = s.predecessor();
    assert!(back.is_zero());
}

#[wasm_bindgen_test]
fn test_chord_angle_trig() {
    let ca = ChordAngle::from_degrees(90.0);
    assert!((ca.sin() - 1.0).abs() < 1e-6);
    assert!((ca.sin2() - 1.0).abs() < 1e-6);
    assert!(ca.cos().abs() < 1e-6);

    let ca60 = ChordAngle::from_degrees(60.0);
    assert!(ca60.tan() > 0.0);
}

#[wasm_bindgen_test]
fn test_chord_angle_arithmetic() {
    let a = ChordAngle::from_degrees(30.0);
    let b = ChordAngle::from_degrees(20.0);
    let sum = a.add(&b);
    assert!(sum.degrees() > 49.0 && sum.degrees() < 51.0);

    let diff = a.sub(&b);
    assert!(diff.degrees() > 9.0 && diff.degrees() < 11.0);
}

#[wasm_bindgen_test]
fn test_chord_angle_to_string() {
    let ca = ChordAngle::from_degrees(45.0);
    let s = ca.to_string_js();
    assert!(!s.is_empty());
}

// ===========================================================================
// Point
// ===========================================================================

#[wasm_bindgen_test]
fn test_point_constructor() {
    let p = Point::new(1.0, 0.0, 0.0);
    assert!((p.x() - 1.0).abs() < 1e-15);
    assert!(p.y().abs() < 1e-15);
    assert!(p.z().abs() < 1e-15);
    assert!(p.is_unit());
}

#[wasm_bindgen_test]
fn test_point_origin() {
    let o = Point::origin();
    assert!(o.is_unit());
}

#[wasm_bindgen_test]
fn test_point_from_lat_lng() {
    let ll = LatLng::from_degrees(0.0, 0.0);
    let p = Point::from_lat_lng(&ll);
    assert!(p.is_unit());
    assert!((p.x() - 1.0).abs() < 1e-12);
}

#[wasm_bindgen_test]
fn test_point_normalize() {
    // Note: Point::new calls from_coords which auto-normalizes.
    // So Point::new(3,4,0) is already unit-length (0.6, 0.8, 0).
    let p = Point::new(3.0, 4.0, 0.0);
    assert!(p.is_unit());
    assert!((p.x() - 0.6).abs() < 1e-12);
    assert!((p.y() - 0.8).abs() < 1e-12);

    // normalize() on an already-unit point returns itself
    let n = p.normalize();
    assert!(n.is_unit());
    assert!(p.approx_eq(&n));
}

#[wasm_bindgen_test]
fn test_point_distance() {
    let p1 = Point::new(1.0, 0.0, 0.0);
    let p2 = Point::new(0.0, 1.0, 0.0);
    let d = p1.distance(&p2);
    assert!((d.degrees() - 90.0).abs() < 1e-10);
}

#[wasm_bindgen_test]
fn test_point_chord_angle() {
    let p1 = Point::new(1.0, 0.0, 0.0);
    let p2 = Point::new(0.0, 1.0, 0.0);
    let ca = p1.chord_angle(&p2);
    assert!((ca.degrees() - 90.0).abs() < 1e-6);
}

#[wasm_bindgen_test]
fn test_point_approx_eq() {
    let p1 = Point::new(1.0, 0.0, 0.0);
    let p2 = Point::new(1.0, 0.0, 0.0);
    assert!(p1.approx_eq(&p2));

    let p3 = Point::new(0.0, 1.0, 0.0);
    assert!(!p1.approx_eq(&p3));
}

#[wasm_bindgen_test]
fn test_point_approx_eq_with_angle() {
    let p1 = Point::new(1.0, 0.0, 0.0);
    let p2 = Point::new(0.0, 1.0, 0.0);
    assert!(!p1.approx_eq_with_angle(&p2, &Angle::from_degrees(1.0)));
    assert!(p1.approx_eq_with_angle(&p2, &Angle::from_degrees(91.0)));
}

#[wasm_bindgen_test]
fn test_point_cross() {
    let p1 = Point::new(1.0, 0.0, 0.0);
    let p2 = Point::new(0.0, 1.0, 0.0);
    let cross = p1.point_cross(&p2);
    // Should be approximately (0, 0, 1)
    assert!(cross.z().abs() > 0.5);
}

#[wasm_bindgen_test]
fn test_point_to_lat_lng() {
    let ll = LatLng::from_degrees(45.0, 90.0);
    let p = ll.to_point();
    let ll2 = p.to_lat_lng();
    assert!((ll2.lat_degrees() - 45.0).abs() < 1e-10);
    assert!((ll2.lng_degrees() - 90.0).abs() < 1e-10);
}

#[wasm_bindgen_test]
fn test_point_to_array() {
    // Point::new normalizes, so use a unit vector directly.
    let p = Point::new(1.0, 0.0, 0.0);
    let arr = p.to_array();
    assert_eq!(arr.len(), 3);
    assert!((arr[0] - 1.0).abs() < 1e-12);
    assert!(arr[1].abs() < 1e-12);
    assert!(arr[2].abs() < 1e-12);
}

#[wasm_bindgen_test]
fn test_point_to_string() {
    let p = Point::new(1.0, 0.0, 0.0);
    let s = p.to_string_js();
    assert!(!s.is_empty());
}

#[wasm_bindgen_test]
fn test_rotate_point() {
    let p = Point::new(1.0, 0.0, 0.0);
    let axis = Point::new(0.0, 0.0, 1.0);
    let rotated = rotate_point(&p, &axis, &Angle::from_degrees(90.0));
    // Should be approximately (0, 1, 0)
    assert!(rotated.x().abs() < 1e-10);
    assert!((rotated.y() - 1.0).abs() < 1e-10);
}

#[wasm_bindgen_test]
fn test_ortho_point() {
    let p = Point::new(1.0, 0.0, 0.0);
    let orth = ortho_point(&p);
    assert!(orth.is_unit());
    // Orthogonal: dot product should be ~0
    let dot = p.x() * orth.x() + p.y() * orth.y() + p.z() * orth.z();
    assert!(dot.abs() < 1e-12);
}

#[wasm_bindgen_test]
fn test_points_from_lat_lng_degrees() {
    let coords = vec![0.0, 0.0, 90.0, 0.0, 0.0, 90.0];
    let points = points_from_lat_lng_degrees(&coords).unwrap();
    assert_eq!(points.len(), 3);

    // Odd-length array should fail
    let bad = vec![0.0, 0.0, 1.0];
    assert!(points_from_lat_lng_degrees(&bad).is_err());
}

// ===========================================================================
// LatLng
// ===========================================================================

#[wasm_bindgen_test]
fn test_latlng_constructor() {
    let lat = Angle::from_degrees(48.0);
    let lng = Angle::from_degrees(2.0);
    let ll = LatLng::new(&lat, &lng);
    assert!((ll.lat_degrees() - 48.0).abs() < 1e-10);
    assert!((ll.lng_degrees() - 2.0).abs() < 1e-10);
}

#[wasm_bindgen_test]
fn test_latlng_from_degrees() {
    let ll = LatLng::from_degrees(48.8566, 2.3522);
    assert!(ll.is_valid());
    assert!((ll.lat_degrees() - 48.8566).abs() < 1e-10);
    assert!((ll.lng_degrees() - 2.3522).abs() < 1e-10);
}

#[wasm_bindgen_test]
fn test_latlng_from_radians() {
    let ll = LatLng::from_radians(0.0, 0.0);
    assert!(ll.is_valid());
    assert!(ll.lat_degrees().abs() < 1e-10);
}

#[wasm_bindgen_test]
fn test_latlng_from_point() {
    let p = Point::new(1.0, 0.0, 0.0);
    let ll = LatLng::from_point(&p);
    assert!(ll.lat_degrees().abs() < 1e-10);
    assert!(ll.lng_degrees().abs() < 1e-10);
}

#[wasm_bindgen_test]
fn test_latlng_from_e5_e6_e7() {
    let ll5 = LatLng::from_e5(4500000, 9000000);
    assert!((ll5.lat_degrees() - 45.0).abs() < 1e-5);
    assert!((ll5.lng_degrees() - 90.0).abs() < 1e-5);

    let ll6 = LatLng::from_e6(45000000, 90000000);
    assert!((ll6.lat_degrees() - 45.0).abs() < 1e-6);

    let ll7 = LatLng::from_e7(450000000, 900000000);
    assert!((ll7.lat_degrees() - 45.0).abs() < 1e-7);
}

#[wasm_bindgen_test]
fn test_latlng_invalid() {
    let inv = LatLng::invalid();
    assert!(!inv.is_valid());
}

#[wasm_bindgen_test]
fn test_latlng_lat_lng_getters() {
    let ll = LatLng::from_degrees(30.0, 60.0);
    let lat_angle = ll.lat();
    let lng_angle = ll.lng();
    assert!((lat_angle.degrees() - 30.0).abs() < 1e-10);
    assert!((lng_angle.degrees() - 60.0).abs() < 1e-10);
}

#[wasm_bindgen_test]
fn test_latlng_normalized() {
    let ll = LatLng::from_degrees(0.0, 270.0);
    let n = ll.normalized();
    assert!((n.lng_degrees() - (-90.0)).abs() < 1e-10);
}

#[wasm_bindgen_test]
fn test_latlng_to_point_roundtrip() {
    let ll = LatLng::from_degrees(48.8566, 2.3522);
    let p = ll.to_point();
    let ll2 = LatLng::from_point(&p);
    assert!((ll2.lat_degrees() - 48.8566).abs() < 1e-10);
    assert!((ll2.lng_degrees() - 2.3522).abs() < 1e-10);
}

#[wasm_bindgen_test]
fn test_latlng_get_distance() {
    let paris = LatLng::from_degrees(48.8566, 2.3522);
    let london = LatLng::from_degrees(51.5074, -0.1278);
    let d = paris.get_distance(&london);
    assert!(d.degrees() > 2.0 && d.degrees() < 5.0);
}

#[wasm_bindgen_test]
fn test_latlng_approx_eq() {
    let a = LatLng::from_degrees(1.0, 2.0);
    let b = LatLng::from_degrees(1.0, 2.0);
    assert!(a.approx_eq(&b));

    let c = LatLng::from_degrees(10.0, 20.0);
    assert!(!a.approx_eq(&c));
}

#[wasm_bindgen_test]
fn test_latlng_to_string() {
    let ll = LatLng::from_degrees(45.0, 90.0);
    let s = ll.to_string_in_degrees();
    assert!(!s.is_empty());
    let s2 = ll.to_string_js();
    assert!(!s2.is_empty());
}

// ===========================================================================
// CellId
// ===========================================================================

#[wasm_bindgen_test]
fn test_cell_id_none_sentinel() {
    let n = CellId::none();
    assert!(!n.is_valid());

    let s = CellId::sentinel();
    // Sentinel is valid structurally but represents a boundary value.
    let _ = s.to_token();
}

#[wasm_bindgen_test]
fn test_cell_id_from_face() {
    let id = CellId::from_face(3);
    assert!(id.is_valid());
    assert!(id.is_face());
    assert_eq!(id.face(), 3);
    assert_eq!(id.level(), 0);
}

#[wasm_bindgen_test]
fn test_cell_id_from_point() {
    let p = LatLng::from_degrees(48.8566, 2.3522).to_point();
    let id = CellId::from_point(&p);
    assert!(id.is_valid());
    assert_eq!(id.level(), 30);
    assert!(id.is_leaf());
}

#[wasm_bindgen_test]
fn test_cell_id_from_lat_lng() {
    let ll = LatLng::from_degrees(0.0, 0.0);
    let id = CellId::from_lat_lng(&ll);
    assert!(id.is_valid());
    assert_eq!(id.level(), 30);
}

#[wasm_bindgen_test]
fn test_cell_id_token_roundtrip() {
    let id = CellId::from_lat_lng(&LatLng::from_degrees(0.0, 0.0));
    let token = id.to_token();
    assert!(!token.is_empty());
    let id2 = CellId::from_token(&token);
    assert_eq!(id.to_token(), id2.to_token());
}

#[wasm_bindgen_test]
fn test_cell_id_id_string() {
    let id = CellId::from_face(0);
    let s = id.id_string();
    assert!(!s.is_empty());
    // Should be a decimal number string
    assert!(s.parse::<u64>().is_ok());
}

#[wasm_bindgen_test]
fn test_cell_id_id_parts() {
    let id = CellId::from_face(0);
    let parts = id.id_parts();
    assert_eq!(parts.len(), 2);
}

#[wasm_bindgen_test]
fn test_cell_id_face_level() {
    let id = CellId::from_lat_lng(&LatLng::from_degrees(45.0, 90.0));
    assert!(id.face() <= 5);
    assert_eq!(id.level(), 30);
    assert!(id.is_leaf());
    assert!(!id.is_face());
}

#[wasm_bindgen_test]
fn test_cell_id_parent() {
    let id = CellId::from_lat_lng(&LatLng::from_degrees(40.0, -74.0));
    let parent = id.parent();
    assert_eq!(parent.level(), 29);
    assert!(parent.contains(&id));
}

#[wasm_bindgen_test]
fn test_cell_id_parent_at_level() {
    let id = CellId::from_lat_lng(&LatLng::from_degrees(40.0, -74.0));
    let parent = id.parent_at_level(10);
    assert_eq!(parent.level(), 10);
    assert!(parent.contains(&id));
}

#[wasm_bindgen_test]
fn test_cell_id_children() {
    let id = CellId::from_lat_lng(&LatLng::from_degrees(0.0, 0.0)).parent_at_level(5);
    let children = id.children();
    assert_eq!(children.len(), 4);
    for c in &children {
        assert_eq!(c.level(), 6);
        assert!(id.contains(c));
    }
}

#[wasm_bindgen_test]
fn test_cell_id_range_min_max() {
    let id = CellId::from_lat_lng(&LatLng::from_degrees(0.0, 0.0)).parent_at_level(10);
    let rmin = id.range_min();
    let rmax = id.range_max();
    assert!(id.contains(&rmin));
    assert!(id.contains(&rmax));
}

#[wasm_bindgen_test]
fn test_cell_id_contains_intersects() {
    let parent = CellId::from_lat_lng(&LatLng::from_degrees(0.0, 0.0)).parent_at_level(5);
    let child = parent.children()[0];
    assert!(parent.contains(&child));
    assert!(parent.intersects(&child));
    assert!(!child.contains(&parent));
}

#[wasm_bindgen_test]
fn test_cell_id_next_prev() {
    let id = CellId::from_lat_lng(&LatLng::from_degrees(0.0, 0.0));
    let n = id.next();
    assert_ne!(n.to_token(), id.to_token());
    let p = n.prev();
    assert_eq!(p.to_token(), id.to_token());
}

#[wasm_bindgen_test]
fn test_cell_id_next_wrap_prev_wrap() {
    let id = CellId::from_lat_lng(&LatLng::from_degrees(0.0, 0.0));
    let nw = id.next_wrap();
    assert!(nw.is_valid());
    let pw = nw.prev_wrap();
    assert_eq!(pw.to_token(), id.to_token());
}

#[wasm_bindgen_test]
fn test_cell_id_to_point_to_lat_lng() {
    let id = CellId::from_lat_lng(&LatLng::from_degrees(45.0, 90.0));
    let p = id.to_point();
    assert!(p.is_unit());
    let ll = id.to_lat_lng();
    assert!((ll.lat_degrees() - 45.0).abs() < 0.01);
    assert!((ll.lng_degrees() - 90.0).abs() < 0.01);
}

#[wasm_bindgen_test]
fn test_cell_id_edge_neighbors() {
    let id = CellId::from_lat_lng(&LatLng::from_degrees(0.0, 0.0)).parent_at_level(10);
    let neighbors = id.edge_neighbors();
    assert_eq!(neighbors.len(), 4);
    for n in &neighbors {
        assert_eq!(n.level(), 10);
    }
}

#[wasm_bindgen_test]
fn test_cell_id_vertex_neighbors() {
    let id = CellId::from_lat_lng(&LatLng::from_degrees(0.0, 0.0)).parent_at_level(10);
    let neighbors = id.vertex_neighbors(10);
    assert!(neighbors.len() >= 3);
}

#[wasm_bindgen_test]
fn test_cell_id_all_neighbors() {
    let id = CellId::from_lat_lng(&LatLng::from_degrees(0.0, 0.0)).parent_at_level(10);
    let neighbors = id.all_neighbors(10);
    assert!(neighbors.is_some());
    assert!(neighbors.unwrap().len() >= 4);
}

#[wasm_bindgen_test]
fn test_cell_id_common_ancestor_level() {
    let a = CellId::from_lat_lng(&LatLng::from_degrees(0.0, 0.0));
    let b = CellId::from_lat_lng(&LatLng::from_degrees(0.001, 0.001));
    let lvl = a.common_ancestor_level(&b);
    assert!(lvl >= 0);
}

#[wasm_bindgen_test]
fn test_cell_id_debug_string_roundtrip() {
    let id = CellId::from_face(3).children()[1];
    let dbg = id.to_debug_string();
    assert!(!dbg.is_empty());
    let parsed = CellId::from_debug_string(&dbg);
    assert!(parsed.is_some());
    assert_eq!(parsed.unwrap().to_token(), id.to_token());
}

#[wasm_bindgen_test]
fn test_cell_id_to_string() {
    let id = CellId::from_face(0);
    let s = id.to_string_js();
    assert!(!s.is_empty());
}

// ===========================================================================
// Cell
// ===========================================================================

#[wasm_bindgen_test]
fn test_cell_from_cell_id() {
    let id = CellId::from_lat_lng(&LatLng::from_degrees(51.5074, -0.1278));
    let cell = Cell::from_cell_id(&id);
    assert_eq!(cell.level(), 30);
    assert_eq!(cell.face(), id.face());
    assert_eq!(cell.id().to_token(), id.to_token());
}

#[wasm_bindgen_test]
fn test_cell_from_point() {
    let p = LatLng::from_degrees(0.0, 0.0).to_point();
    let cell = Cell::from_point(&p);
    assert!(cell.is_leaf());
}

#[wasm_bindgen_test]
fn test_cell_from_lat_lng() {
    let ll = LatLng::from_degrees(45.0, 90.0);
    let cell = Cell::from_lat_lng(&ll);
    assert_eq!(cell.level(), 30);
}

#[wasm_bindgen_test]
fn test_cell_from_face() {
    let cell = Cell::from_face(2);
    assert_eq!(cell.face(), 2);
    assert_eq!(cell.level(), 0);
    assert!(!cell.is_leaf());
}

#[wasm_bindgen_test]
fn test_cell_vertex_edge() {
    let cell = Cell::from_face(0);
    for k in 0..4 {
        let v = cell.vertex(k);
        assert!(v.is_unit());
        let e = cell.edge(k as u8);
        let _ = e; // Just check it doesn't panic
    }
}

#[wasm_bindgen_test]
fn test_cell_center() {
    let cell = Cell::from_face(0);
    let c = cell.center();
    assert!(c.is_unit());
}

#[wasm_bindgen_test]
fn test_cell_area() {
    let cell = Cell::from_face(0);
    assert!(cell.exact_area() > 0.0);
    assert!(cell.approx_area() > 0.0);
    assert!(cell.average_area() > 0.0);
    let level_area = Cell::average_area_for_level(15);
    assert!(level_area > 0.0);
    assert!(level_area < cell.average_area());
}

#[wasm_bindgen_test]
fn test_cell_contains_point() {
    let ll = LatLng::from_degrees(0.0, 0.0);
    let cell = Cell::from_lat_lng(&ll);
    let p = ll.to_point();
    assert!(cell.contains_point(&p));
}

#[wasm_bindgen_test]
fn test_cell_cap_bound_rect_bound() {
    let cell = Cell::from_face(0);
    let cap = cell.cap_bound();
    assert!(cap.is_valid());
    assert!(!cap.is_empty());
    let rect = cell.rect_bound();
    assert!(rect.is_valid());
    assert!(!rect.is_empty());
}

#[wasm_bindgen_test]
fn test_cell_distance_to_point() {
    let cell = Cell::from_lat_lng(&LatLng::from_degrees(0.0, 0.0));
    let far = LatLng::from_degrees(45.0, 45.0).to_point();
    let d = cell.distance_to_point(&far);
    assert!(d.degrees() > 0.0);
    let md = cell.max_distance_to_point(&far);
    assert!(md.degrees() >= d.degrees());
}

#[wasm_bindgen_test]
fn test_cell_distance_to_cell() {
    let c1 = Cell::from_lat_lng(&LatLng::from_degrees(0.0, 0.0));
    let c2 = Cell::from_lat_lng(&LatLng::from_degrees(45.0, 45.0));
    let d = c1.distance_to_cell(&c2);
    assert!(d.degrees() > 0.0);
}

// ===========================================================================
// CellUnion
// ===========================================================================

#[wasm_bindgen_test]
fn test_cell_union_new_empty() {
    let cu = CellUnion::new();
    assert!(cu.is_empty());
    assert_eq!(cu.num_cells(), 0);
}

#[wasm_bindgen_test]
fn test_cell_union_from_cell_ids() {
    let id1 = CellId::from_lat_lng(&LatLng::from_degrees(0.0, 0.0));
    let id2 = CellId::from_lat_lng(&LatLng::from_degrees(1.0, 1.0));
    let cu = CellUnion::from_cell_ids(vec![id1, id2]);
    assert_eq!(cu.num_cells(), 2);
    assert!(!cu.is_empty());
    assert!(cu.is_valid());
    assert!(cu.is_normalized());
}

#[wasm_bindgen_test]
fn test_cell_union_from_tokens() {
    let id = CellId::from_lat_lng(&LatLng::from_degrees(0.0, 0.0));
    let token = id.to_token();
    let cu = CellUnion::from_tokens(vec![token]);
    assert_eq!(cu.num_cells(), 1);
}

#[wasm_bindgen_test]
fn test_cell_union_whole_sphere() {
    let ws = CellUnion::whole_sphere();
    assert!(!ws.is_empty());
    assert_eq!(ws.num_cells(), 6);
}

#[wasm_bindgen_test]
fn test_cell_union_cell_ids_and_tokens() {
    let id1 = CellId::from_lat_lng(&LatLng::from_degrees(0.0, 0.0));
    let id2 = CellId::from_lat_lng(&LatLng::from_degrees(10.0, 10.0));
    let cu = CellUnion::from_cell_ids(vec![id1, id2]);
    let ids = cu.cell_ids();
    assert_eq!(ids.len(), 2);
    let tokens = cu.tokens();
    assert_eq!(tokens.len(), 2);
}

#[wasm_bindgen_test]
fn test_cell_union_contains_intersects() {
    let id = CellId::from_lat_lng(&LatLng::from_degrees(0.0, 0.0));
    let cu = CellUnion::from_cell_ids(vec![id]);
    assert!(cu.contains_cell_id(&id));
    assert!(cu.intersects_cell_id(&id));

    let p = LatLng::from_degrees(0.0, 0.0).to_point();
    assert!(cu.contains_point(&p));
}

#[wasm_bindgen_test]
fn test_cell_union_contains_intersects_union() {
    let id1 = CellId::from_lat_lng(&LatLng::from_degrees(0.0, 0.0));
    let id2 = CellId::from_lat_lng(&LatLng::from_degrees(10.0, 10.0));
    let cu1 = CellUnion::from_cell_ids(vec![id1, id2]);
    let cu2 = CellUnion::from_cell_ids(vec![id1]);
    assert!(cu1.contains_union(&cu2));
    assert!(cu1.intersects_union(&cu2));
}

#[wasm_bindgen_test]
fn test_cell_union_set_operations() {
    let id1 = CellId::from_lat_lng(&LatLng::from_degrees(0.0, 0.0));
    let id2 = CellId::from_lat_lng(&LatLng::from_degrees(10.0, 10.0));
    let id3 = CellId::from_lat_lng(&LatLng::from_degrees(20.0, 20.0));
    let cu1 = CellUnion::from_cell_ids(vec![id1, id2]);
    let cu2 = CellUnion::from_cell_ids(vec![id2, id3]);

    let u = cu1.union_with(&cu2);
    assert_eq!(u.num_cells(), 3);

    let i = cu1.intersection_with(&cu2);
    assert_eq!(i.num_cells(), 1);

    let d = cu1.difference_with(&cu2);
    assert_eq!(d.num_cells(), 1);
}

#[wasm_bindgen_test]
fn test_cell_union_normalize() {
    let id = CellId::from_lat_lng(&LatLng::from_degrees(0.0, 0.0));
    let mut cu = CellUnion::from_cell_ids(vec![id]);
    cu.normalize();
    assert!(cu.is_normalized());
}

#[wasm_bindgen_test]
fn test_cell_union_expand() {
    let id = CellId::from_lat_lng(&LatLng::from_degrees(0.0, 0.0)).parent_at_level(10);
    let mut cu = CellUnion::from_cell_ids(vec![id]);
    let n_before = cu.num_cells();
    cu.expand_at_level(10);
    assert!(cu.num_cells() > n_before);
}

#[wasm_bindgen_test]
fn test_cell_union_expand_by_radius() {
    let id = CellId::from_lat_lng(&LatLng::from_degrees(0.0, 0.0)).parent_at_level(10);
    let mut cu = CellUnion::from_cell_ids(vec![id]);
    let n_before = cu.num_cells();
    cu.expand_by_radius(&Angle::from_degrees(1.0), 5);
    assert!(cu.num_cells() > n_before);
}

#[wasm_bindgen_test]
fn test_cell_union_area() {
    let ws = CellUnion::whole_sphere();
    let approx = ws.approx_area();
    let exact = ws.exact_area();
    // 4π ≈ 12.566
    assert!((approx - 4.0 * std::f64::consts::PI).abs() < 0.01);
    assert!((exact - 4.0 * std::f64::consts::PI).abs() < 1e-10);
}

// ===========================================================================
// Cap
// ===========================================================================

#[wasm_bindgen_test]
fn test_cap_from_center_angle() {
    let center = LatLng::from_degrees(0.0, 0.0).to_point();
    let cap = Cap::from_center_angle(&center, &Angle::from_degrees(10.0));
    assert!(cap.is_valid());
    assert!(!cap.is_empty());
    assert!(!cap.is_full());
}

#[wasm_bindgen_test]
fn test_cap_from_center_chord_angle() {
    let center = LatLng::from_degrees(0.0, 0.0).to_point();
    let cap = Cap::from_center_chord_angle(&center, &ChordAngle::from_degrees(10.0));
    assert!(cap.is_valid());
}

#[wasm_bindgen_test]
fn test_cap_from_point() {
    let p = LatLng::from_degrees(0.0, 0.0).to_point();
    let cap = Cap::from_point(&p);
    assert!(cap.is_valid());
    assert!(cap.contains_point(&p));
}

#[wasm_bindgen_test]
fn test_cap_from_center_area() {
    let center = LatLng::from_degrees(0.0, 0.0).to_point();
    let cap = Cap::from_center_area(&center, 1.0);
    assert!(cap.is_valid());
    assert!((cap.area() - 1.0).abs() < 0.01);
}

#[wasm_bindgen_test]
fn test_cap_empty_full() {
    let e = Cap::empty();
    assert!(e.is_empty());
    assert!(!e.is_full());

    let f = Cap::full();
    assert!(f.is_full());
    assert!(!f.is_empty());
}

#[wasm_bindgen_test]
fn test_cap_getters() {
    let center = LatLng::from_degrees(0.0, 0.0).to_point();
    let cap = Cap::from_center_angle(&center, &Angle::from_degrees(10.0));
    let c = cap.center();
    assert!(c.approx_eq(&center));
    assert!(cap.angle_radius().degrees() > 9.0);
    assert!(cap.chord_radius().degrees() > 9.0);
    assert!(cap.height() > 0.0);
    assert!(cap.area() > 0.0);
}

#[wasm_bindgen_test]
fn test_cap_contains_intersects() {
    let center = LatLng::from_degrees(0.0, 0.0).to_point();
    let big = Cap::from_center_angle(&center, &Angle::from_degrees(20.0));
    let small = Cap::from_center_angle(&center, &Angle::from_degrees(5.0));
    assert!(big.contains_cap(&small));
    assert!(big.intersects_cap(&small));
    assert!(!small.contains_cap(&big));
}

#[wasm_bindgen_test]
fn test_cap_contains_point() {
    let center = LatLng::from_degrees(0.0, 0.0).to_point();
    let cap = Cap::from_center_angle(&center, &Angle::from_degrees(10.0));
    assert!(cap.contains_point(&center));
    let far = LatLng::from_degrees(20.0, 20.0).to_point();
    assert!(!cap.contains_point(&far));
}

#[wasm_bindgen_test]
fn test_cap_complement() {
    let center = LatLng::from_degrees(0.0, 0.0).to_point();
    let cap = Cap::from_center_angle(&center, &Angle::from_degrees(10.0));
    let comp = cap.complement();
    assert!(!comp.is_empty());
    // Original + complement should cover the sphere
    let far = LatLng::from_degrees(90.0, 0.0).to_point();
    assert!(comp.contains_point(&far));
}

#[wasm_bindgen_test]
fn test_cap_expanded() {
    let center = LatLng::from_degrees(0.0, 0.0).to_point();
    let cap = Cap::from_center_angle(&center, &Angle::from_degrees(10.0));
    let expanded = cap.expanded(&Angle::from_degrees(5.0));
    assert!(expanded.area() > cap.area());
}

#[wasm_bindgen_test]
fn test_cap_union() {
    let c1 = Cap::from_center_angle(
        &LatLng::from_degrees(0.0, 0.0).to_point(),
        &Angle::from_degrees(5.0),
    );
    let c2 = Cap::from_center_angle(
        &LatLng::from_degrees(10.0, 10.0).to_point(),
        &Angle::from_degrees(5.0),
    );
    let u = c1.union_with(&c2);
    assert!(u.area() >= c1.area());
    assert!(u.area() >= c2.area());
}

#[wasm_bindgen_test]
fn test_cap_add_point() {
    let center = LatLng::from_degrees(0.0, 0.0).to_point();
    let cap = Cap::from_point(&center);
    let far = LatLng::from_degrees(10.0, 10.0).to_point();
    let bigger = cap.add_point(&far);
    assert!(bigger.contains_point(&far));
}

#[wasm_bindgen_test]
fn test_cap_bounds() {
    let center = LatLng::from_degrees(0.0, 0.0).to_point();
    let cap = Cap::from_center_angle(&center, &Angle::from_degrees(10.0));
    let cb = cap.cap_bound();
    assert!(cb.is_valid());
    let rb = cap.rect_bound();
    assert!(rb.is_valid());
    let cub = cap.cell_union_bound();
    assert!(!cub.is_empty());
}

#[wasm_bindgen_test]
fn test_cap_centroid() {
    let center = LatLng::from_degrees(0.0, 0.0).to_point();
    let cap = Cap::from_center_angle(&center, &Angle::from_degrees(10.0));
    let centroid = cap.centroid();
    // Centroid should be near the center direction
    assert!(centroid.x() > 0.0);
}

#[wasm_bindgen_test]
fn test_cap_approx_eq() {
    let center = LatLng::from_degrees(0.0, 0.0).to_point();
    let c1 = Cap::from_center_angle(&center, &Angle::from_degrees(10.0));
    let c2 = Cap::from_center_angle(&center, &Angle::from_degrees(10.0));
    assert!(c1.approx_eq(&c2));
}

// ===========================================================================
// Rect
// ===========================================================================

#[wasm_bindgen_test]
fn test_rect_empty_full() {
    let e = Rect::empty();
    assert!(e.is_empty());
    assert!(e.is_valid());

    let f = Rect::full();
    assert!(f.is_full());
    assert!(f.is_valid());
}

#[wasm_bindgen_test]
fn test_rect_from_lat_lng() {
    let ll = LatLng::from_degrees(10.0, 20.0);
    let r = Rect::from_lat_lng(&ll);
    assert!(r.is_point());
    assert!(r.is_valid());
}

#[wasm_bindgen_test]
fn test_rect_from_point_pair() {
    let r = Rect::from_point_pair(
        &LatLng::from_degrees(-10.0, -10.0),
        &LatLng::from_degrees(10.0, 10.0),
    );
    assert!(r.is_valid());
    assert!(!r.is_empty());
    assert!(!r.is_point());
}

#[wasm_bindgen_test]
fn test_rect_from_center_size() {
    let center = LatLng::from_degrees(0.0, 0.0);
    let size = LatLng::from_degrees(20.0, 20.0);
    let r = Rect::from_center_size(&center, &size);
    assert!(r.is_valid());
    assert!(!r.is_empty());
}

#[wasm_bindgen_test]
fn test_rect_lo_hi_center_size() {
    let r = Rect::from_point_pair(
        &LatLng::from_degrees(-10.0, -20.0),
        &LatLng::from_degrees(10.0, 20.0),
    );
    let lo = r.lo();
    let hi = r.hi();
    assert!((lo.lat_degrees() - (-10.0)).abs() < 1e-10);
    assert!((hi.lat_degrees() - 10.0).abs() < 1e-10);

    let center = r.center();
    assert!(center.lat_degrees().abs() < 1e-10);

    let sz = r.size();
    assert!((sz.lat_degrees() - 20.0).abs() < 1e-10);
}

#[wasm_bindgen_test]
fn test_rect_area() {
    let f = Rect::full();
    assert!((f.area() - 4.0 * std::f64::consts::PI).abs() < 0.01);
}

#[wasm_bindgen_test]
fn test_rect_contains() {
    let r = Rect::from_point_pair(
        &LatLng::from_degrees(-10.0, -10.0),
        &LatLng::from_degrees(10.0, 10.0),
    );
    assert!(r.contains_lat_lng(&LatLng::from_degrees(0.0, 0.0)));
    assert!(!r.contains_lat_lng(&LatLng::from_degrees(20.0, 0.0)));

    let p = LatLng::from_degrees(0.0, 0.0).to_point();
    assert!(r.contains_point(&p));

    let smaller = Rect::from_point_pair(
        &LatLng::from_degrees(-5.0, -5.0),
        &LatLng::from_degrees(5.0, 5.0),
    );
    assert!(r.contains_rect(&smaller));
    assert!(!smaller.contains_rect(&r));
}

#[wasm_bindgen_test]
fn test_rect_intersects() {
    let r1 = Rect::from_point_pair(
        &LatLng::from_degrees(0.0, 0.0),
        &LatLng::from_degrees(10.0, 10.0),
    );
    let r2 = Rect::from_point_pair(
        &LatLng::from_degrees(5.0, 5.0),
        &LatLng::from_degrees(15.0, 15.0),
    );
    assert!(r1.intersects_rect(&r2));
}

#[wasm_bindgen_test]
fn test_rect_add_point() {
    let r = Rect::from_lat_lng(&LatLng::from_degrees(0.0, 0.0));
    let r2 = r.add_point(&LatLng::from_degrees(10.0, 10.0));
    assert!(!r2.is_point());
    assert!(r2.contains_lat_lng(&LatLng::from_degrees(5.0, 5.0)));
}

#[wasm_bindgen_test]
fn test_rect_expanded() {
    let r = Rect::from_point_pair(
        &LatLng::from_degrees(-1.0, -1.0),
        &LatLng::from_degrees(1.0, 1.0),
    );
    let margin = LatLng::from_degrees(1.0, 1.0);
    let exp = r.expanded(&margin);
    assert!(exp.area() > r.area());
}

#[wasm_bindgen_test]
fn test_rect_expanded_by_distance() {
    let r = Rect::from_point_pair(
        &LatLng::from_degrees(-1.0, -1.0),
        &LatLng::from_degrees(1.0, 1.0),
    );
    let exp = r.expanded_by_distance(&Angle::from_degrees(1.0));
    assert!(exp.area() > r.area());
}

#[wasm_bindgen_test]
fn test_rect_union_intersection() {
    let r1 = Rect::from_point_pair(
        &LatLng::from_degrees(0.0, 0.0),
        &LatLng::from_degrees(10.0, 10.0),
    );
    let r2 = Rect::from_point_pair(
        &LatLng::from_degrees(5.0, 5.0),
        &LatLng::from_degrees(15.0, 15.0),
    );
    let u = r1.union_with(&r2);
    assert!(u.area() > r1.area());

    let i = r1.intersection_with(&r2);
    assert!(i.area() > 0.0);
    assert!(i.area() < r1.area());
}

#[wasm_bindgen_test]
fn test_rect_bounds() {
    let r = Rect::from_point_pair(
        &LatLng::from_degrees(-10.0, -10.0),
        &LatLng::from_degrees(10.0, 10.0),
    );
    let cb = r.cap_bound();
    assert!(cb.is_valid());
    let cub = r.cell_union_bound();
    assert!(!cub.is_empty());
}

#[wasm_bindgen_test]
fn test_rect_centroid() {
    let r = Rect::from_point_pair(
        &LatLng::from_degrees(-10.0, -10.0),
        &LatLng::from_degrees(10.0, 10.0),
    );
    let c = r.centroid();
    // Centroid is a weighted average, not necessarily unit-length, but nonzero.
    let len2 = c.x() * c.x() + c.y() * c.y() + c.z() * c.z();
    assert!(len2 > 0.0);
}

#[wasm_bindgen_test]
fn test_rect_distance() {
    let r1 = Rect::from_point_pair(
        &LatLng::from_degrees(0.0, 0.0),
        &LatLng::from_degrees(1.0, 1.0),
    );
    let r2 = Rect::from_point_pair(
        &LatLng::from_degrees(10.0, 10.0),
        &LatLng::from_degrees(11.0, 11.0),
    );
    let d = r1.get_distance(&r2);
    assert!(d.degrees() > 0.0);

    let p = LatLng::from_degrees(20.0, 20.0);
    let dp = r1.get_distance_to_latlng(&p);
    assert!(dp.degrees() > 0.0);

    let hd = r1.get_hausdorff_distance(&r2);
    assert!(hd.degrees() > 0.0);
}

#[wasm_bindgen_test]
fn test_rect_approx_eq() {
    let r1 = Rect::from_point_pair(
        &LatLng::from_degrees(0.0, 0.0),
        &LatLng::from_degrees(10.0, 10.0),
    );
    let r2 = Rect::from_point_pair(
        &LatLng::from_degrees(0.0, 0.0),
        &LatLng::from_degrees(10.0, 10.0),
    );
    assert!(r1.approx_eq(&r2));
}

#[wasm_bindgen_test]
fn test_rect_to_string() {
    let r = Rect::from_point_pair(
        &LatLng::from_degrees(0.0, 0.0),
        &LatLng::from_degrees(10.0, 10.0),
    );
    let s = r.to_string_js();
    assert!(!s.is_empty());
}

// ===========================================================================
// S2Loop
// ===========================================================================

#[wasm_bindgen_test]
fn test_loop_constructor() {
    let pts = parse_points("0:0, 0:10, 10:10, 10:0");
    let loop_ = Loop::new(pts);
    assert_eq!(loop_.num_vertices(), 4);
}

#[wasm_bindgen_test]
fn test_loop_empty_full() {
    let e = Loop::empty();
    assert!(e.is_empty_loop());
    assert!(!e.is_full_loop());
    assert!(e.is_empty_loop() || e.is_full_loop()); // is_empty_or_full equivalent

    let f = Loop::full();
    assert!(f.is_full_loop());
    assert!(!f.is_empty_loop());
}

#[wasm_bindgen_test]
fn test_loop_from_cell() {
    let cell = Cell::from_face(0);
    let loop_ = Loop::from_cell(&cell);
    assert_eq!(loop_.num_vertices(), 4);
}

#[wasm_bindgen_test]
fn test_loop_make_regular() {
    let center = LatLng::from_degrees(0.0, 0.0).to_point();
    let loop_ = Loop::make_regular(&center, &Angle::from_degrees(5.0), 32);
    assert_eq!(loop_.num_vertices(), 32);
}

#[wasm_bindgen_test]
fn test_loop_vertex_vertices() {
    let loop_ = make_loop("0:0, 0:10, 10:10, 10:0");
    let v0 = loop_.vertex(0);
    assert!(v0.is_unit());
    let all = loop_.vertices();
    assert_eq!(all.len(), 4);
}

#[wasm_bindgen_test]
fn test_loop_is_hole_sign() {
    let loop_ = make_loop("0:0, 0:10, 10:10, 10:0");
    // A CCW loop is not a hole, sign = +1
    assert!(!loop_.is_hole());
    assert_eq!(loop_.sign(), 1);
}

#[wasm_bindgen_test]
fn test_loop_is_normalized() {
    let loop_ = make_loop("0:0, 0:10, 10:10, 10:0");
    assert!(loop_.is_normalized());
}

#[wasm_bindgen_test]
fn test_loop_normalize_invert() {
    // Use a full loop — guaranteed not to be a hole.
    let mut loop_ = Loop::full();
    assert!(!loop_.is_hole());
    loop_.invert();
    // Inverting the full loop gives the empty loop.
    assert!(loop_.is_empty_loop());

    // Test normalize on a regular loop
    let mut loop2 = make_loop("0:0, 0:10, 10:10, 10:0");
    loop2.invert();
    loop2.normalize();
    assert!(loop2.is_normalized());
}

#[wasm_bindgen_test]
fn test_loop_area_centroid() {
    let loop_ = make_loop("0:0, 0:10, 10:10, 10:0");
    assert!(loop_.area() > 0.0);
    let c = loop_.centroid();
    let ll = LatLng::from_point(&c);
    // Centroid should be roughly in the center
    assert!(ll.lat_degrees() > 0.0 && ll.lat_degrees() < 10.0);
}

#[wasm_bindgen_test]
fn test_loop_turning_angle() {
    let loop_ = make_loop("0:0, 0:10, 10:10, 10:0");
    let ta = loop_.turning_angle();
    // For a convex loop, turning angle ≈ 2π
    assert!(ta > 0.0);
}

#[wasm_bindgen_test]
fn test_loop_validate() {
    let loop_ = make_loop("0:0, 0:10, 10:10, 10:0");
    assert!(loop_.validate().is_ok());
}

#[wasm_bindgen_test]
fn test_loop_equal() {
    let a = make_loop("0:0, 0:10, 10:10, 10:0");
    let b = make_loop("0:0, 0:10, 10:10, 10:0");
    assert!(a.equal(&b));
}

#[wasm_bindgen_test]
fn test_loop_bound_cap_bound() {
    let loop_ = make_loop("0:0, 0:10, 10:10, 10:0");
    let b = loop_.bound();
    assert!(b.is_valid());
    let cb = loop_.cap_bound();
    assert!(cb.is_valid());
}

#[wasm_bindgen_test]
fn test_loop_get_distance() {
    let loop_ = make_loop("0:0, 0:10, 10:10, 10:0");
    let far = LatLng::from_degrees(50.0, 50.0).to_point();
    let d = loop_.get_distance(&far);
    assert!(d.degrees() > 0.0);
}

#[wasm_bindgen_test]
fn test_loop_project_point() {
    let loop_ = make_loop("0:0, 0:10, 10:10, 10:0");
    let p = LatLng::from_degrees(5.0, 5.0).to_point();
    let proj = loop_.project_point(&p);
    assert!(proj.is_unit());
}

#[wasm_bindgen_test]
fn test_loop_contains_intersects_loop() {
    let big = make_loop("0:0, 0:20, 20:20, 20:0");
    let small = make_loop("5:5, 5:10, 10:10, 10:5");
    assert!(big.contains_loop(&small));
    assert!(big.intersects_loop(&small));
    assert!(!small.contains_loop(&big));
}

#[wasm_bindgen_test]
fn test_loop_boundary_approx_eq() {
    let a = make_loop("0:0, 0:10, 10:10, 10:0");
    let b = make_loop("0:0, 0:10, 10:10, 10:0");
    assert!(a.boundary_approx_eq(&b, &Angle::from_degrees(1e-10)));
}

#[wasm_bindgen_test]
fn test_loop_boundary_near() {
    let a = make_loop("0:0, 0:10, 10:10, 10:0");
    let b = make_loop("0:0, 0:10, 10:10, 10:0");
    assert!(a.boundary_near(&b, &Angle::from_degrees(1.0)));
}

#[wasm_bindgen_test]
fn test_loop_contains_origin() {
    let full = Loop::full();
    assert!(full.contains_origin());

    let empty = Loop::empty();
    assert!(!empty.contains_origin());
}

// ===========================================================================
// Polygon
// ===========================================================================

#[wasm_bindgen_test]
fn test_polygon_constructor() {
    let loop_ = make_loop("0:0, 0:10, 10:10, 10:0");
    let poly = Polygon::new(vec![loop_]);
    assert_eq!(poly.num_loops(), 1);
}

#[wasm_bindgen_test]
fn test_polygon_empty_full() {
    let e = Polygon::empty();
    assert!(e.is_empty_polygon());
    assert!(!e.is_full_polygon());

    let f = Polygon::full();
    assert!(f.is_full_polygon());
    assert!(!f.is_empty_polygon());
}

#[wasm_bindgen_test]
fn test_polygon_from_cell() {
    let cell = Cell::from_face(0);
    let poly = Polygon::from_cell(&cell);
    assert_eq!(poly.num_loops(), 1);
    assert!(poly.area() > 0.0);
}

#[wasm_bindgen_test]
fn test_polygon_num_loops_vertices() {
    let poly = make_polygon("0:0, 0:10, 10:10, 10:0");
    assert_eq!(poly.num_loops(), 1);
    assert_eq!(poly.num_vertices(), 4);
    assert!(!poly.has_holes());
}

#[wasm_bindgen_test]
fn test_polygon_loop_at() {
    let poly = make_polygon("0:0, 0:10, 10:10, 10:0");
    let loop_ = poly.loop_at(0);
    assert_eq!(loop_.num_vertices(), 4);
}

#[wasm_bindgen_test]
fn test_polygon_area_centroid() {
    let poly = make_polygon("0:0, 0:10, 10:10, 10:0");
    assert!(poly.area() > 0.0);
    let c = poly.centroid();
    let ll = LatLng::from_point(&c);
    assert!(ll.lat_degrees() > 0.0 && ll.lat_degrees() < 10.0);
}

#[wasm_bindgen_test]
fn test_polygon_bound_cap_bound() {
    let poly = make_polygon("0:0, 0:10, 10:10, 10:0");
    let b = poly.bound();
    assert!(b.is_valid());
    let cb = poly.cap_bound();
    assert!(cb.is_valid());
}

#[wasm_bindgen_test]
fn test_polygon_get_distance() {
    let poly = make_polygon("0:0, 0:10, 10:10, 10:0");
    let far = LatLng::from_degrees(50.0, 50.0).to_point();
    let d = poly.get_distance(&far);
    assert!(d.degrees() > 0.0);
}

#[wasm_bindgen_test]
fn test_polygon_project_point() {
    let poly = make_polygon("0:0, 0:10, 10:10, 10:0");
    let p = LatLng::from_degrees(5.0, 5.0).to_point();
    let proj = poly.project_point(&p);
    assert!(proj.is_unit());
}

#[wasm_bindgen_test]
fn test_polygon_project_to_boundary() {
    let poly = make_polygon("0:0, 0:10, 10:10, 10:0");
    let p = LatLng::from_degrees(5.0, 5.0).to_point();
    let proj = poly.project_to_boundary(&p);
    assert!(proj.is_unit());
}

#[wasm_bindgen_test]
fn test_polygon_validate() {
    let poly = make_polygon("0:0, 0:10, 10:10, 10:0");
    assert!(poly.validate().is_ok());
}

#[wasm_bindgen_test]
fn test_polygon_invert() {
    let mut poly = make_polygon("0:0, 0:10, 10:10, 10:0");
    let area_before = poly.area();
    poly.invert();
    let area_after = poly.area();
    // Inverted polygon covers the rest of the sphere
    assert!((area_before + area_after - 4.0 * std::f64::consts::PI).abs() < 0.01);
}

#[wasm_bindgen_test]
fn test_polygon_complement() {
    let poly = make_polygon("0:0, 0:10, 10:10, 10:0");
    let comp = Polygon::complement(&poly);
    assert!((poly.area() + comp.area() - 4.0 * std::f64::consts::PI).abs() < 0.01);
}

#[wasm_bindgen_test]
fn test_polygon_union() {
    let mut a = make_polygon("0:0, 0:10, 10:10, 10:0");
    let mut b = make_polygon("5:5, 5:15, 15:15, 15:5");
    let c = Polygon::union_op(&mut a, &mut b);
    assert!(c.area() > a.area());
    assert!(c.area() > b.area());
}

#[wasm_bindgen_test]
fn test_polygon_intersection() {
    let mut a = make_polygon("0:0, 0:10, 10:10, 10:0");
    let mut b = make_polygon("5:5, 5:15, 15:15, 15:5");
    let c = Polygon::intersection_op(&mut a, &mut b);
    assert!(c.area() > 0.0);
    assert!(c.area() < a.area());
}

#[wasm_bindgen_test]
fn test_polygon_difference() {
    let mut a = make_polygon("0:0, 0:10, 10:10, 10:0");
    let mut b = make_polygon("5:5, 5:15, 15:15, 15:5");
    let c = Polygon::difference_op(&mut a, &mut b);
    assert!(c.area() > 0.0);
    assert!(c.area() < a.area());
}

#[wasm_bindgen_test]
fn test_polygon_symmetric_difference() {
    let mut a = make_polygon("0:0, 0:10, 10:10, 10:0");
    let mut b = make_polygon("5:5, 5:15, 15:15, 15:5");
    let c = Polygon::symmetric_difference_op(&mut a, &mut b);
    assert!(c.area() > 0.0);
}

#[wasm_bindgen_test]
fn test_polygon_equal() {
    let a = make_polygon("0:0, 0:10, 10:10, 10:0");
    let b = make_polygon("0:0, 0:10, 10:10, 10:0");
    assert!(a.equal(&b));
}

#[wasm_bindgen_test]
fn test_polygon_contains_intersects() {
    let big = make_polygon("0:0, 0:20, 20:20, 20:0");
    let small = make_polygon("5:5, 5:10, 10:10, 10:5");
    assert!(big.contains_polygon(&small));
    assert!(big.intersects_polygon(&small));
    assert!(!small.contains_polygon(&big));
}

#[wasm_bindgen_test]
fn test_polygon_boundary_approx_eq() {
    let a = make_polygon("0:0, 0:10, 10:10, 10:0");
    let b = make_polygon("0:0, 0:10, 10:10, 10:0");
    assert!(a.boundary_approx_eq(&b, &Angle::from_degrees(1e-10)));
}

#[wasm_bindgen_test]
fn test_polygon_polyline_operations() {
    let mut poly = make_polygon("0:0, 0:10, 10:10, 10:0");
    let pl = make_polyline("-5:5, 15:5"); // crosses the polygon

    let inside_parts = poly.intersect_with_polyline(&pl);
    assert!(!inside_parts.is_empty());

    let outside_parts = poly.subtract_from_polyline(&pl);
    assert!(!outside_parts.is_empty());

    let contained_pl = make_polyline("2:2, 8:8");
    assert!(poly.contains_polyline(&contained_pl));
    assert!(poly.intersects_polyline(&contained_pl));
}

#[wasm_bindgen_test]
fn test_polygon_get_snap_level() {
    let poly = make_polygon("0:0, 0:10, 10:10, 10:0");
    let lvl = poly.get_snap_level();
    // Generic polygon doesn't have a snap level
    assert!(lvl == -1 || lvl >= 0);
}

// ===========================================================================
// Polyline
// ===========================================================================

#[wasm_bindgen_test]
fn test_polyline_constructor() {
    let pts = parse_points("0:0, 0:10, 10:10");
    let pl = Polyline::new(pts);
    assert_eq!(pl.num_vertices(), 3);
}

#[wasm_bindgen_test]
fn test_polyline_from_lat_lngs() {
    let lls = parse_latlngs("0:0, 0:10, 10:10");
    let pl = Polyline::from_lat_lngs(lls);
    assert_eq!(pl.num_vertices(), 3);
}

#[wasm_bindgen_test]
fn test_polyline_vertex_vertices() {
    let pl = make_polyline("0:0, 0:10, 10:10");
    let v0 = pl.vertex(0);
    assert!(v0.is_unit());
    let all = pl.vertices();
    assert_eq!(all.len(), 3);
}

#[wasm_bindgen_test]
fn test_polyline_reverse() {
    let mut pl = make_polyline("0:0, 0:10, 10:10");
    let first_before = pl.vertex(0);
    let last_before = pl.vertex(2);
    pl.reverse();
    let first_after = pl.vertex(0);
    let last_after = pl.vertex(2);
    assert!(first_before.approx_eq(&last_after));
    assert!(last_before.approx_eq(&first_after));
}

#[wasm_bindgen_test]
fn test_polyline_length() {
    let pl = make_polyline("0:0, 0:10, 10:10");
    let len = pl.length();
    assert!(len.degrees() > 0.0);
}

#[wasm_bindgen_test]
fn test_polyline_centroid() {
    let pl = make_polyline("0:0, 0:10, 10:10");
    let c = pl.centroid();
    let _ = c; // Just check it doesn't panic
}

#[wasm_bindgen_test]
fn test_polyline_validate() {
    let pl = make_polyline("0:0, 0:10, 10:10");
    assert!(pl.validate().is_ok());
}

#[wasm_bindgen_test]
fn test_polyline_project() {
    let pl = make_polyline("0:0, 0:10");
    let p = LatLng::from_degrees(0.0, 5.0).to_point();
    let result = pl.project(&p);
    assert_eq!(result.len(), 2);
}

#[wasm_bindgen_test]
fn test_polyline_interpolate() {
    let pl = make_polyline("0:0, 0:10");
    let result = pl.interpolate(0.5);
    assert_eq!(result.len(), 2);
}

#[wasm_bindgen_test]
fn test_polyline_equal() {
    let a = make_polyline("0:0, 0:10, 10:10");
    let b = make_polyline("0:0, 0:10, 10:10");
    assert!(a.equal(&b));
}

#[wasm_bindgen_test]
fn test_polyline_approx_eq_with() {
    let a = make_polyline("0:0, 0:10, 10:10");
    let b = make_polyline("0:0, 0:10, 10:10");
    assert!(a.approx_eq_with(&b, &Angle::from_degrees(1e-10)));
}

#[wasm_bindgen_test]
fn test_polyline_is_on_right() {
    let pl = make_polyline("0:0, 0:10");
    let right = LatLng::from_degrees(-1.0, 5.0).to_point();
    let left = LatLng::from_degrees(1.0, 5.0).to_point();
    // One side should be right, the other left
    assert!(pl.is_on_right(&right) != pl.is_on_right(&left));
}

#[wasm_bindgen_test]
fn test_polyline_intersects() {
    let a = make_polyline("0:0, 0:10");
    let b = make_polyline("-5:5, 5:5"); // crosses a
    assert!(a.intersects(&b));

    let c = make_polyline("20:20, 20:30"); // does not cross
    assert!(!a.intersects(&c));
}

#[wasm_bindgen_test]
fn test_polyline_subsample_vertices() {
    let pl = make_polyline("0:0, 0:1, 0:2, 0:3, 0:10");
    let indices = pl.subsample_vertices(&Angle::from_degrees(5.0));
    // Should subsample to fewer vertices
    assert!(indices.len() <= 5);
    assert!(indices.len() >= 2); // at least first and last
}

#[wasm_bindgen_test]
fn test_polyline_nearly_covers() {
    let a = make_polyline("0:0, 0:10");
    let b = make_polyline("0:1, 0:9"); // subset
    assert!(a.nearly_covers(&b, &Angle::from_degrees(2.0)));
}

// ===========================================================================
// RegionCoverer
// ===========================================================================

#[wasm_bindgen_test]
fn test_region_coverer_new() {
    let rc = RegionCoverer::new();
    let _ = rc; // constructor works
}

#[wasm_bindgen_test]
fn test_region_coverer_settings() {
    let rc = RegionCoverer::new()
        .set_min_level(5)
        .set_max_level(15)
        .set_level_mod(2)
        .set_max_cells(10);
    // Just verify chaining works and doesn't panic
    let _ = rc;
}

#[wasm_bindgen_test]
fn test_region_coverer_covering_cap() {
    let center = LatLng::from_degrees(48.8566, 2.3522).to_point();
    let cap = Cap::from_center_angle(&center, &Angle::from_degrees(0.5));
    let coverer = RegionCoverer::new().set_max_level(14).set_max_cells(8);
    let covering = coverer.covering_cap(&cap);
    assert!(covering.num_cells() > 0);
    assert!(covering.num_cells() <= 8);
}

#[wasm_bindgen_test]
fn test_region_coverer_covering_rect() {
    let rect = Rect::from_point_pair(
        &LatLng::from_degrees(0.0, 0.0),
        &LatLng::from_degrees(1.0, 1.0),
    );
    let coverer = RegionCoverer::new().set_max_cells(20);
    let covering = coverer.covering_rect(&rect);
    assert!(covering.num_cells() > 0);
}

#[wasm_bindgen_test]
fn test_region_coverer_covering_loop() {
    let loop_ = make_loop("0:0, 0:10, 10:10, 10:0");
    let coverer = RegionCoverer::new().set_max_cells(20);
    let covering = coverer.covering_loop(&loop_);
    assert!(covering.num_cells() > 0);
}

#[wasm_bindgen_test]
fn test_region_coverer_covering_polygon() {
    let poly = make_polygon("0:0, 0:10, 10:10, 10:0");
    let coverer = RegionCoverer::new().set_max_cells(20);
    let covering = coverer.covering_polygon(&poly);
    assert!(covering.num_cells() > 0);
}

#[wasm_bindgen_test]
fn test_region_coverer_interior_covering_cap() {
    let center = LatLng::from_degrees(0.0, 0.0).to_point();
    let cap = Cap::from_center_angle(&center, &Angle::from_degrees(5.0));
    let coverer = RegionCoverer::new().set_max_cells(50);
    let covering = coverer.interior_covering_cap(&cap);
    assert!(covering.num_cells() > 0);
}

#[wasm_bindgen_test]
fn test_region_coverer_interior_covering_rect() {
    let rect = Rect::from_point_pair(
        &LatLng::from_degrees(0.0, 0.0),
        &LatLng::from_degrees(10.0, 10.0),
    );
    let coverer = RegionCoverer::new().set_max_cells(50);
    let covering = coverer.interior_covering_rect(&rect);
    assert!(covering.num_cells() > 0);
}

#[wasm_bindgen_test]
fn test_region_coverer_interior_covering_polygon() {
    let poly = make_polygon("0:0, 0:10, 10:10, 10:0");
    let coverer = RegionCoverer::new().set_max_cells(50);
    let covering = coverer.interior_covering_polygon(&poly);
    assert!(covering.num_cells() > 0);
}

// ===========================================================================
// ShapeIndex
// ===========================================================================

#[wasm_bindgen_test]
fn test_shape_index_new() {
    let index = ShapeIndex::new();
    assert!(index.is_empty());
    assert_eq!(index.len(), 0);
    assert_eq!(index.num_shape_ids(), 0);
    assert_eq!(index.num_edges(), 0);
}

#[wasm_bindgen_test]
fn test_shape_index_add_polygon() {
    let poly = make_polygon("0:0, 0:10, 10:10, 10:0");
    let mut index = ShapeIndex::new();
    let id = index.add_polygon(poly);
    assert_eq!(id, 0);
    assert_eq!(index.num_shape_ids(), 1);
    assert!(!index.is_empty());
}

#[wasm_bindgen_test]
fn test_shape_index_add_polyline() {
    let pl = make_polyline("0:0, 0:10, 10:10");
    let mut index = ShapeIndex::new();
    let id = index.add_polyline(pl);
    assert_eq!(id, 0);
    assert_eq!(index.len(), 1);
}

#[wasm_bindgen_test]
fn test_shape_index_add_loop() {
    let loop_ = make_loop("0:0, 0:10, 10:10, 10:0");
    let mut index = ShapeIndex::new();
    let id = index.add_loop(loop_);
    assert_eq!(id, 0);
}

#[wasm_bindgen_test]
fn test_shape_index_build() {
    let poly = make_polygon("0:0, 0:10, 10:10, 10:0");
    let mut index = ShapeIndex::new();
    index.add_polygon(poly);
    index.build(); // should not panic
}

#[wasm_bindgen_test]
fn test_shape_index_num_edges() {
    let poly = make_polygon("0:0, 0:10, 10:10, 10:0");
    let mut index = ShapeIndex::new();
    index.add_polygon(poly);
    assert_eq!(index.num_edges(), 4);
}

#[wasm_bindgen_test]
fn test_shape_index_contains_point() {
    let poly = make_polygon("0:0, 0:10, 10:10, 10:0");
    let mut index = ShapeIndex::new();
    index.add_polygon(poly);
    index.build();

    let inside = LatLng::from_degrees(5.0, 5.0).to_point();
    let outside = LatLng::from_degrees(20.0, 20.0).to_point();
    assert!(index.contains_point(&inside));
    assert!(!index.contains_point(&outside));
}

#[wasm_bindgen_test]
fn test_shape_index_containing_shape_ids() {
    let poly = make_polygon("0:0, 0:10, 10:10, 10:0");
    let mut index = ShapeIndex::new();
    index.add_polygon(poly);
    index.build();

    let inside = LatLng::from_degrees(5.0, 5.0).to_point();
    let ids = index.containing_shape_ids(&inside);
    assert_eq!(ids.len(), 1);
    assert_eq!(ids[0], 0);

    let outside = LatLng::from_degrees(20.0, 20.0).to_point();
    let ids2 = index.containing_shape_ids(&outside);
    assert!(ids2.is_empty());
}

#[wasm_bindgen_test]
fn test_shape_index_get_distance_to_point() {
    let poly = make_polygon("0:0, 0:10, 10:10, 10:0");
    let mut index = ShapeIndex::new();
    index.add_polygon(poly);
    index.build();

    let outside = LatLng::from_degrees(20.0, 20.0).to_point();
    let dist = index.get_distance_to_point(&outside);
    assert!(dist.degrees() > 0.0);

    let inside = LatLng::from_degrees(5.0, 5.0).to_point();
    let dist_inside = index.get_distance_to_point(&inside);
    assert!(dist_inside.length2() < 1e-10);
}

#[wasm_bindgen_test]
fn test_shape_index_is_distance_less() {
    let poly = make_polygon("0:0, 0:10, 10:10, 10:0");
    let mut index = ShapeIndex::new();
    index.add_polygon(poly);
    index.build();

    let p = LatLng::from_degrees(11.0, 5.0).to_point();
    let close = ChordAngle::from_degrees(5.0);
    let far = ChordAngle::from_degrees(0.1);
    assert!(index.is_distance_less_to_point(&p, &close));
    assert!(!index.is_distance_less_to_point(&p, &far));
}

#[wasm_bindgen_test]
fn test_shape_index_locate_cell() {
    let poly = make_polygon("0:0, 0:10, 10:10, 10:0");
    let mut index = ShapeIndex::new();
    index.add_polygon(poly);
    index.build();

    let cell_id = CellId::from_lat_lng(&LatLng::from_degrees(5.0, 5.0)).parent_at_level(5);
    let rel = index.locate_cell(&cell_id);
    assert!(rel == "DISJOINT" || rel == "SUBDIVIDED" || rel == "INDEXED");
}

#[wasm_bindgen_test]
fn test_shape_index_locate_point() {
    let poly = make_polygon("0:0, 0:10, 10:10, 10:0");
    let mut index = ShapeIndex::new();
    index.add_polygon(poly);
    index.build();

    let inside = LatLng::from_degrees(5.0, 5.0).to_point();
    assert!(index.locate_point(&inside));
}

#[wasm_bindgen_test]
fn test_shape_index_multiple_shapes() {
    let poly = make_polygon("0:0, 0:10, 10:10, 10:0");
    let pl = make_polyline("20:20, 20:30");
    let mut index = ShapeIndex::new();
    let id0 = index.add_polygon(poly);
    let id1 = index.add_polyline(pl);
    assert_eq!(id0, 0);
    assert_eq!(id1, 1);
    assert_eq!(index.num_shape_ids(), 2);
    assert_eq!(index.len(), 2);
}

// ===========================================================================
// Boolean operations (free functions)
// ===========================================================================

#[wasm_bindgen_test]
fn test_boolean_contains() {
    let big = make_polygon("0:0, 0:20, 20:20, 20:0");
    let small = make_polygon("5:5, 5:10, 10:10, 10:5");
    let mut idx_big = ShapeIndex::new();
    idx_big.add_polygon(big);
    let mut idx_small = ShapeIndex::new();
    idx_small.add_polygon(small);
    assert!(boolean_contains(&mut idx_big, &mut idx_small));
    assert!(!boolean_contains(&mut idx_small, &mut idx_big));
}

#[wasm_bindgen_test]
fn test_boolean_intersects() {
    let a = make_polygon("0:0, 0:10, 10:10, 10:0");
    let b = make_polygon("5:5, 5:15, 15:15, 15:5");
    let mut idx_a = ShapeIndex::new();
    idx_a.add_polygon(a);
    let mut idx_b = ShapeIndex::new();
    idx_b.add_polygon(b);
    assert!(boolean_intersects(&mut idx_a, &mut idx_b));
}

#[wasm_bindgen_test]
fn test_boolean_equals() {
    let a = make_polygon("0:0, 0:10, 10:10, 10:0");
    let b = make_polygon("0:0, 0:10, 10:10, 10:0");
    let mut idx_a = ShapeIndex::new();
    idx_a.add_polygon(a);
    let mut idx_b = ShapeIndex::new();
    idx_b.add_polygon(b);
    assert!(boolean_equals(&mut idx_a, &mut idx_b));
}

// ===========================================================================
// ConvexHullQuery
// ===========================================================================

#[wasm_bindgen_test]
fn test_convex_hull_new() {
    let q = ConvexHullQuery::new();
    let _ = q;
}

#[wasm_bindgen_test]
fn test_convex_hull_add_point() {
    let mut q = ConvexHullQuery::new();
    q.add_point(&LatLng::from_degrees(0.0, 0.0).to_point());
    q.add_point(&LatLng::from_degrees(10.0, 0.0).to_point());
    q.add_point(&LatLng::from_degrees(0.0, 10.0).to_point());
    let hull = q.convex_hull();
    assert!(hull.num_vertices() >= 3);
}

#[wasm_bindgen_test]
fn test_convex_hull_add_points() {
    let mut q = ConvexHullQuery::new();
    let points = parse_points("0:0, 10:0, 0:10, 10:10, 5:5");
    q.add_points(points);
    let hull = q.convex_hull();
    assert!(hull.num_vertices() >= 4);
}

#[wasm_bindgen_test]
fn test_convex_hull_add_polyline() {
    let mut q = ConvexHullQuery::new();
    let pl = make_polyline("0:0, 10:0, 10:10, 0:10");
    q.add_polyline(&pl);
    let hull = q.convex_hull();
    assert!(hull.num_vertices() >= 4);
}

#[wasm_bindgen_test]
fn test_convex_hull_add_loop() {
    let mut q = ConvexHullQuery::new();
    let loop_ = make_loop("0:0, 0:10, 10:10, 10:0");
    q.add_loop(&loop_);
    let hull = q.convex_hull();
    assert!(hull.num_vertices() >= 4);
}

#[wasm_bindgen_test]
fn test_convex_hull_add_polygon() {
    let mut q = ConvexHullQuery::new();
    let poly = make_polygon("0:0, 0:10, 10:10, 10:0");
    q.add_polygon(&poly);
    let hull = q.convex_hull();
    assert!(hull.num_vertices() >= 4);
}

// ===========================================================================
// Earth utility functions
// ===========================================================================

#[wasm_bindgen_test]
fn test_meters_to_angle() {
    let a = meters_to_angle(1000.0);
    assert!(a.radians() > 0.0);
}

#[wasm_bindgen_test]
fn test_angle_to_meters_roundtrip() {
    let a = meters_to_angle(5000.0);
    let m = angle_to_meters(&a);
    assert!((m - 5000.0).abs() < 0.1);
}

#[wasm_bindgen_test]
fn test_km_to_angle() {
    let a = km_to_angle(100.0);
    assert!(a.radians() > 0.0);
}

#[wasm_bindgen_test]
fn test_angle_to_km_roundtrip() {
    let a = km_to_angle(100.0);
    let km = angle_to_km(&a);
    assert!((km - 100.0).abs() < 0.001);
}

#[wasm_bindgen_test]
fn test_get_distance_meters_points() {
    let a = LatLng::from_degrees(0.0, 0.0).to_point();
    let b = LatLng::from_degrees(0.0, 1.0).to_point();
    let d = get_distance_meters_points(&a, &b);
    // ~111 km = ~111000 m
    assert!(d > 100_000.0 && d < 120_000.0);
}

#[wasm_bindgen_test]
fn test_get_distance_meters_latlng() {
    let a = LatLng::from_degrees(0.0, 0.0);
    let b = LatLng::from_degrees(0.0, 1.0);
    let d = get_distance_meters_latlng(&a, &b);
    assert!(d > 100_000.0 && d < 120_000.0);
}

#[wasm_bindgen_test]
fn test_get_distance_km_points() {
    let a = LatLng::from_degrees(0.0, 0.0).to_point();
    let b = LatLng::from_degrees(0.0, 1.0).to_point();
    let d = get_distance_km_points(&a, &b);
    assert!(d > 100.0 && d < 120.0);
}

#[wasm_bindgen_test]
fn test_get_distance_km_latlng() {
    let paris = LatLng::from_degrees(48.8566, 2.3522);
    let london = LatLng::from_degrees(51.5074, -0.1278);
    let d = get_distance_km_latlng(&paris, &london);
    assert!(d > 300.0 && d < 400.0);
}

#[wasm_bindgen_test]
fn test_get_initial_bearing() {
    let a = LatLng::from_degrees(0.0, 0.0);
    let b = LatLng::from_degrees(0.0, 1.0); // due east
    let bearing = get_initial_bearing(&a, &b);
    // Bearing should be ~90 degrees (east)
    assert!((bearing.degrees() - 90.0).abs() < 1.0);
}

#[wasm_bindgen_test]
fn test_square_km_steradians_roundtrip() {
    let sr = square_km_to_steradians(1000.0);
    let km2 = steradians_to_square_km(sr);
    assert!((km2 - 1000.0).abs() < 0.001);
}

#[wasm_bindgen_test]
fn test_square_meters_steradians_roundtrip() {
    let sr = square_meters_to_steradians(1_000_000.0);
    let m2 = steradians_to_square_meters(sr);
    assert!((m2 - 1_000_000.0).abs() < 0.01);
}

// ===========================================================================
// Text format functions
// ===========================================================================

#[wasm_bindgen_test]
fn test_parse_points_fn() {
    let pts = parse_points("0:0, 10:20, 30:40");
    assert_eq!(pts.len(), 3);
}

#[wasm_bindgen_test]
fn test_parse_point_fn() {
    let p = parse_point("48.8566:2.3522");
    assert!(p.is_unit());
}

#[wasm_bindgen_test]
fn test_parse_latlngs_fn() {
    let lls = parse_latlngs("10:20, 30:40");
    assert_eq!(lls.len(), 2);
    assert!((lls[0].lat_degrees() - 10.0).abs() < 1e-10);
}

#[wasm_bindgen_test]
fn test_make_rect_fn() {
    let r = make_rect("0:0, 10:10");
    assert!(r.is_valid());
    assert!(!r.is_empty());
}

#[wasm_bindgen_test]
fn test_make_loop_fn() {
    let l = make_loop("0:0, 0:10, 10:10, 10:0");
    assert_eq!(l.num_vertices(), 4);
}

#[wasm_bindgen_test]
fn test_make_polygon_fn() {
    let p = make_polygon("0:0, 0:10, 10:10, 10:0");
    assert_eq!(p.num_loops(), 1);
}

#[wasm_bindgen_test]
fn test_make_polyline_fn() {
    let pl = make_polyline("0:0, 0:10, 10:10");
    assert_eq!(pl.num_vertices(), 3);
}

#[wasm_bindgen_test]
fn test_point_to_string_fn() {
    let p = parse_point("45:90");
    let s = point_to_string(&p);
    assert!(!s.is_empty());
    assert!(s.contains(":"));
}

#[wasm_bindgen_test]
fn test_points_to_string_fn() {
    let pts = parse_points("0:0, 10:20");
    let s = points_to_string(pts);
    assert!(!s.is_empty());
}

#[wasm_bindgen_test]
fn test_loop_to_string_fn() {
    let l = make_loop("0:0, 0:10, 10:10, 10:0");
    let s = loop_to_string(&l);
    assert!(!s.is_empty());
}

#[wasm_bindgen_test]
fn test_polygon_to_string_fn() {
    let p = make_polygon("0:0, 0:10, 10:10, 10:0");
    let s = polygon_to_string(&p);
    assert!(!s.is_empty());
}

#[wasm_bindgen_test]
fn test_polyline_to_string_fn() {
    let pl = make_polyline("0:0, 0:10, 10:10");
    let s = polyline_to_string(&pl);
    assert!(!s.is_empty());
}

#[wasm_bindgen_test]
fn test_latlng_to_string_fn() {
    let ll = LatLng::from_degrees(45.0, 90.0);
    let s = latlng_to_string(&ll);
    assert!(!s.is_empty());
}

#[wasm_bindgen_test]
fn test_text_format_polygon_roundtrip() {
    let orig = "0:0, 0:10, 10:10, 10:0";
    let poly = make_polygon(orig);
    let s = polygon_to_string(&poly);
    let poly2 = make_polygon(&s);
    assert!(poly.boundary_approx_eq(&poly2, &Angle::from_degrees(1e-6)));
}
