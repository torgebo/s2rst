// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! Text-based parsing and formatting for S2 types.
//!
//! Provides convenience functions for creating S2 types from compact text
//! representations. Primarily intended for testing and debugging.
//!
//! **Format**: Coordinates are `latitude:longitude` in degrees, separated
//! by commas. Loops in polygons are separated by semicolons.
//!
//! # Examples
//!
//! ```ignore
//! let p = parse_point("37.7749:-122.4194");
//! let loop_ = make_loop("0:0, 0:10, 10:0");
//! let polygon = make_polygon("0:0, 0:10, 10:0; 1:1, 1:5, 5:1");
//! ```
//!
//! Corresponds to Go `s2/textformat_test.go`.

#![expect(
    clippy::cast_sign_loss,
    reason = "decimal places (i32) cast to usize after max(0) — always non-negative"
)]
use crate::s2::{LatLng, Loop, Point, Polygon, Rect};

/// Parses a comma-separated list of `lat:lng` pairs into `LatLng` values.
///
/// Invalid entries are silently skipped.
pub fn parse_latlngs(s: &str) -> Vec<LatLng> {
    let s = s.trim();
    if s.is_empty() {
        return Vec::new();
    }

    s.split(',')
        .filter_map(|part| {
            let part = part.trim();
            if part.is_empty() {
                return None;
            }
            let mut iter = part.splitn(2, ':');
            let lat_str = iter.next()?.trim();
            let lng_str = iter.next()?.trim();
            let lat: f64 = lat_str.parse().ok()?;
            let lng: f64 = lng_str.parse().ok()?;
            Some(LatLng::from_degrees(lat, lng))
        })
        .collect()
}

/// Parses a comma-separated list of `lat:lng` pairs into `Point` values.
pub fn parse_points(s: &str) -> Vec<Point> {
    parse_latlngs(s).into_iter().map(LatLng::to_point).collect()
}

/// Parses a single `lat:lng` string into a `Point`.
///
/// Returns the origin point if parsing fails.
pub fn parse_point(s: &str) -> Point {
    let pts = parse_points(s);
    if pts.is_empty() {
        Point::default()
    } else {
        pts[0]
    }
}

/// Creates a `Rect` from a text representation.
///
/// The rect is the minimal bounding rectangle of the parsed lat/lng pairs.
pub fn make_rect(s: &str) -> Rect {
    let lls = parse_latlngs(s);
    let mut rect = Rect::empty();
    for ll in lls {
        rect = rect.add_point(ll);
    }
    rect
}

/// Creates a `Loop` from a text representation.
///
/// Supports `"empty"` and `"full"` as special values.
pub fn make_loop(s: &str) -> Loop {
    let s = s.trim();
    match s.to_lowercase().as_str() {
        "empty" => Loop::empty(),
        "full" => Loop::full(),
        _ => Loop::new(parse_points(s)),
    }
}

/// Creates a `Polygon` from a text representation.
///
/// Loops are separated by semicolons. Supports `"empty"` (or `""`) and
/// `"full"` as special values.
pub fn make_polygon(s: &str) -> Polygon {
    let s = s.trim();
    if s.is_empty() || s.eq_ignore_ascii_case("empty") {
        return Polygon::empty();
    }
    if s.eq_ignore_ascii_case("full") {
        return Polygon::full();
    }

    let loops: Vec<Loop> = s
        .split(';')
        .filter_map(|part| {
            let part = part.trim();
            if part.is_empty() {
                return None;
            }
            Some(make_loop(part))
        })
        .collect();

    Polygon::from_loops(loops)
}

/// Creates a `Polyline` from a text representation.
pub fn make_polyline(s: &str) -> crate::s2::polyline::Polyline {
    crate::s2::polyline::Polyline::new(parse_points(s))
}

/// Creates a `LaxPolygon` from a text representation.
///
/// Loops are separated by semicolons. Each loop is a comma-separated list
/// of `lat:lng` pairs. Supports `"empty"` (or `""`) and `"full"` as special values.
pub fn make_lax_polygon(s: &str) -> crate::s2::lax_polygon::LaxPolygon {
    let s = s.trim();
    if s.is_empty() || s.eq_ignore_ascii_case("empty") {
        return crate::s2::lax_polygon::LaxPolygon::empty();
    }
    if s.eq_ignore_ascii_case("full") {
        return crate::s2::lax_polygon::LaxPolygon::full();
    }

    let loops: Vec<Vec<Point>> = s
        .split(';')
        .filter_map(|part| {
            let part = part.trim();
            if part.is_empty() {
                return None;
            }
            Some(parse_points(part))
        })
        .collect();

    let loop_refs: Vec<&[Point]> = loops.iter().map(Vec::as_slice).collect();
    crate::s2::lax_polygon::LaxPolygon::from_loops(&loop_refs)
}

/// Formats a float like C's `%.15g`: 15 significant digits, no trailing zeros.
fn format_g15(v: f64) -> String {
    if v == 0.0 {
        return "0".to_string();
    }
    // Format with 14 decimal places in scientific notation = 15 significant digits
    let s = format!("{v:.14e}");
    let Some((mant, exp_str)) = s.split_once('e') else {
        return s;
    };
    let Ok(exp) = exp_str.parse::<i32>() else {
        return s;
    };

    if (-4..15).contains(&exp) {
        // Fixed notation: compute decimal places for 15 sig digits
        let decimal_places = (14 - exp).max(0) as usize;
        let s = format!("{v:.decimal_places$}");
        if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            s
        }
    } else {
        // Scientific notation with trimmed trailing zeros
        let mant_trimmed = mant.trim_end_matches('0').trim_end_matches('.');
        format!("{mant_trimmed}e{exp}")
    }
}

/// Formats a `Point` as `"lat:lng"` with full precision (matches C++ `%.15g`).
pub fn point_to_string(p: Point) -> String {
    let ll = LatLng::from_point(p);
    format!(
        "{}:{}",
        format_g15(ll.lat.degrees()),
        format_g15(ll.lng.degrees())
    )
}

/// Formats a slice of `Point`s as a comma-separated string.
pub fn points_to_string(points: &[Point]) -> String {
    points
        .iter()
        .map(|p| point_to_string(*p))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Formats a `Polyline` as a comma-separated string of `"lat:lng"` vertices.
pub fn polyline_to_string(polyline: &crate::s2::polyline::Polyline) -> String {
    let verts: Vec<Point> = (0..polyline.num_vertices())
        .map(|i| polyline.vertex(i))
        .collect();
    points_to_string(&verts)
}

/// Formats a `LaxPolyline` as a comma-separated string of `"lat:lng"` vertices.
pub fn lax_polyline_to_string(polyline: &crate::s2::lax_polyline::LaxPolyline) -> String {
    let verts: Vec<Point> = (0..polyline.num_vertices())
        .map(|i| polyline.vertex(i))
        .collect();
    points_to_string(&verts)
}

/// Formats a `LatLng` as `"lat:lng"` with full precision (matches C++ `%.15g`).
pub fn latlng_to_string(ll: LatLng) -> String {
    format!(
        "{}:{}",
        format_g15(ll.lat.degrees()),
        format_g15(ll.lng.degrees())
    )
}

/// Formats a `Loop` as a comma-separated string of `"lat:lng"` vertices.
pub fn loop_to_string(loop_: &Loop) -> String {
    if loop_.is_empty_loop() {
        return "empty".to_string();
    }
    if loop_.is_full_loop() {
        return "full".to_string();
    }
    let verts: Vec<Point> = (0..loop_.num_vertices()).map(|i| loop_.vertex(i)).collect();
    points_to_string(&verts)
}

/// Formats a `Polygon` as a semicolon-separated string of loops.
pub fn polygon_to_string(polygon: &Polygon) -> String {
    if polygon.is_empty_polygon() {
        return "empty".to_string();
    }
    if polygon.is_full_polygon() {
        return "full".to_string();
    }
    (0..polygon.num_loops())
        .map(|i| loop_to_string(polygon.loop_at(i)))
        .collect::<Vec<_>>()
        .join("; ")
}

/// Formats a `LaxPolygon` as a semicolon-separated string of loops.
pub fn lax_polygon_to_string(polygon: &crate::s2::lax_polygon::LaxPolygon) -> String {
    if polygon.num_loops() == 0 {
        return "empty".to_string();
    }
    (0..polygon.num_loops())
        .map(|i| {
            let n = polygon.num_loop_vertices(i);
            if n == 0 {
                return "full".to_string();
            }
            let verts: Vec<Point> = (0..n).map(|j| polygon.loop_vertex(i, j)).collect();
            points_to_string(&verts)
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// Creates a `LaxPolyline` from a text representation.
///
/// Each vertex is `lat:lng` separated by commas.
pub fn make_lax_polyline(s: &str) -> crate::s2::lax_polyline::LaxPolyline {
    crate::s2::lax_polyline::LaxPolyline::new(parse_points(s))
}

/// Creates a `ShapeIndex` from a multi-dimensional text representation.
///
/// Format: `"points # polylines # polygons"` where:
/// - Points are separated by `|`, each is `lat:lng`
/// - Polylines are separated by `|`, each is `lat1:lng1, lat2:lng2, ...`
/// - Polygons are separated by `|`, each has loops separated by `;`
///
/// Special polygon values: `"empty"`, `"full"`.
///
/// Corresponds to C++ `s2textformat::MakeIndexOrDie`.
///
/// # Panics
///
/// Panics if the string does not contain exactly two `#` separators or
/// if any component cannot be parsed.
pub fn make_index(s: &str) -> crate::s2::shape_index::ShapeIndex {
    use crate::s2::lax_polyline::LaxPolyline;
    use crate::s2::point_vector::PointVector;
    use crate::s2::shape_index::ShapeIndex;

    let parts: Vec<&str> = s.splitn(3, '#').collect();
    assert_eq!(
        parts.len(),
        3,
        "make_index: input must contain exactly two '#' characters, got: {s:?}"
    );

    let mut index = ShapeIndex::new();

    // Part 0: Points (separated by '|')
    let points_str = parts[0].trim();
    if !points_str.is_empty() {
        let points: Vec<Point> = points_str
            .split('|')
            .filter_map(|p| {
                let p = p.trim();
                if p.is_empty() {
                    return None;
                }
                Some(parse_point(p))
            })
            .collect();
        if !points.is_empty() {
            index.add(Box::new(PointVector::new(points)));
        }
    }

    // Part 1: Polylines (separated by '|')
    let polylines_str = parts[1].trim();
    if !polylines_str.is_empty() {
        for line_str in polylines_str.split('|') {
            let line_str = line_str.trim();
            if line_str.is_empty() {
                continue;
            }
            let vertices = parse_points(line_str);
            if !vertices.is_empty() {
                index.add(Box::new(LaxPolyline::new(vertices)));
            }
        }
    }

    // Part 2: Polygons (separated by '|')
    let polygons_str = parts[2].trim();
    if !polygons_str.is_empty() {
        for poly_str in polygons_str.split('|') {
            let poly_str = poly_str.trim();
            if poly_str.is_empty() {
                continue;
            }
            let polygon = make_lax_polygon(poly_str);
            index.add(Box::new(polygon));
        }
    }

    index.build();
    index
}

/// Formats a [`ShapeIndex`](crate::s2::shape_index::ShapeIndex) as a text string using the `"points # polylines # polygons"` format.
///
/// Corresponds to C++ `s2textformat::ToString(const S2ShapeIndex&)`.
pub fn index_to_string(index: &crate::s2::shape_index::ShapeIndex) -> String {
    use crate::s2::shape::Dimension;

    let mut points_parts: Vec<String> = Vec::new();
    let mut polyline_parts: Vec<String> = Vec::new();
    let mut polygon_parts: Vec<String> = Vec::new();

    for id in 0..index.num_shape_ids() {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_possible_wrap,
            reason = "shape IDs fit in i32"
        )]
        let Some(shape) = index.shape(id as i32) else {
            continue;
        };
        match shape.dimension() {
            Dimension::Point => {
                for i in 0..shape.num_edges() {
                    let e = shape.edge(i);
                    points_parts.push(point_to_string(e.v0));
                }
            }
            Dimension::Polyline => {
                for chain_id in 0..shape.num_chains() {
                    let chain = shape.chain(chain_id);
                    let mut verts = Vec::with_capacity(chain.length + 1);
                    for offset in 0..chain.length {
                        let e = shape.chain_edge(chain_id, offset);
                        if offset == 0 {
                            verts.push(e.v0);
                        }
                        verts.push(e.v1);
                    }
                    polyline_parts.push(points_to_string(&verts));
                }
            }
            Dimension::Polygon => {
                let lp = crate::s2::shape_util::shape_to_loop_vertices(shape);
                if lp.is_empty() || (lp.len() == 1 && lp[0].is_empty()) {
                    // Check if it's a "full" polygon
                    if shape.num_chains() == 1 && shape.num_edges() == 0 {
                        polygon_parts.push("full".to_string());
                    }
                } else {
                    let loops_str: Vec<String> = lp.iter().map(|l| points_to_string(l)).collect();
                    polygon_parts.push(loops_str.join("; "));
                }
            }
        }
    }

    let points_str = points_parts.join(" | ");
    let polylines_str = polyline_parts.join(" | ");
    let polygons_str = polygon_parts.join(" | ");

    // Format as "points # polylines # polygons", with each section
    // padded only if non-empty.
    let mut result = String::new();
    if !points_str.is_empty() {
        result.push_str(&points_str);
        result.push(' ');
    }
    result.push('#');
    if polylines_str.is_empty() {
        result.push(' ');
    } else {
        result.push(' ');
        result.push_str(&polylines_str);
        result.push(' ');
    }
    result.push('#');
    if !polygons_str.is_empty() {
        result.push(' ');
        result.push_str(&polygons_str);
    }
    result
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::region::Region;

    #[test]
    fn test_parse_empty() {
        assert!(parse_latlngs("").is_empty());
        assert!(parse_points("").is_empty());
    }

    #[test]
    fn test_parse_single_point() {
        let pts = parse_points("0:0");
        assert_eq!(pts.len(), 1);
    }

    #[test]
    fn test_parse_multiple_points() {
        let pts = parse_points("-20:150, 10:-120, 0.123:-170.652");
        assert_eq!(pts.len(), 3);
    }

    #[test]
    fn test_parse_latlngs() {
        let lls = parse_latlngs("45:90, -30:60");
        assert_eq!(lls.len(), 2);
        assert!((lls[0].lat.degrees() - 45.0).abs() < 1e-10);
        assert!((lls[0].lng.degrees() - 90.0).abs() < 1e-10);
    }

    #[test]
    fn test_parse_point_default() {
        let p = parse_point("");
        assert_eq!(p, Point::default());
    }

    #[test]
    fn test_make_rect() {
        let rect = make_rect("10:20, 30:40");
        assert!(!rect.is_empty());
        let ll = LatLng::from_degrees(20.0, 30.0);
        assert!(rect.contains_lat_lng(ll));
    }

    #[test]
    fn test_make_loop_empty() {
        let l = make_loop("empty");
        assert!(l.is_empty_loop());
    }

    #[test]
    fn test_make_loop_full() {
        let l = make_loop("full");
        assert!(l.is_full_loop());
    }

    #[test]
    fn test_make_loop_triangle() {
        let l = make_loop("0:0, 0:10, 10:0");
        assert_eq!(l.num_vertices(), 3);
        assert!(l.area() > 0.0);
    }

    #[test]
    fn test_make_polygon_empty() {
        let p = make_polygon("");
        assert!(p.is_empty_polygon());
        let p = make_polygon("empty");
        assert!(p.is_empty_polygon());
    }

    #[test]
    fn test_make_polygon_full() {
        let p = make_polygon("full");
        assert!(p.is_full_polygon());
    }

    #[test]
    fn test_make_polygon_single_loop() {
        let p = make_polygon("0:0, 0:10, 10:0");
        assert_eq!(p.num_loops(), 1);
    }

    #[test]
    fn test_make_polygon_two_loops() {
        let p = make_polygon("0:0, 0:20, 20:0; 1:1, 1:5, 5:1");
        assert_eq!(p.num_loops(), 2);
    }

    #[test]
    fn test_make_polyline() {
        let pl = make_polyline("0:0, 1:1, 2:0");
        assert_eq!(pl.len(), 3);
    }

    #[test]
    fn test_point_to_string() {
        let p = LatLng::from_degrees(45.0, 90.0).to_point();
        let s = point_to_string(p);
        // Should contain the latitude and longitude
        assert!(s.contains("45"));
        assert!(s.contains("90"));
    }

    #[test]
    fn test_points_to_string_roundtrip() {
        let original = "0:0, 10:20, -30:45";
        let pts = parse_points(original);
        let s = points_to_string(&pts);
        let back = parse_points(&s);
        assert_eq!(pts.len(), back.len());
        for (a, b) in pts.iter().zip(back.iter()) {
            let dist = (a.0 - b.0).norm();
            assert!(dist < 1e-10, "points don't match: dist={dist}");
        }
    }

    #[test]
    fn test_make_loop_contains_interior_point() {
        let l = make_loop("0:0, 0:10, 10:0");
        let inside = parse_point("2:2");
        assert!(
            l.contains_point(&inside),
            "loop should contain interior point"
        );
    }

    #[test]
    fn test_trailing_semicolon() {
        // Trailing semicolons should be handled gracefully.
        let p = make_polygon("0:0, 0:10, 10:0;");
        assert_eq!(p.num_loops(), 1);
    }

    #[test]
    fn test_polygon_to_string_roundtrip() {
        let original = "0:0, 0:10, 10:0";
        let poly = make_polygon(original);
        let s = polygon_to_string(&poly);
        let poly2 = make_polygon(&s);
        assert_eq!(poly.num_loops(), poly2.num_loops());
        assert_eq!(poly.num_vertices(), poly2.num_vertices());
    }

    #[test]
    fn test_polygon_to_string_empty() {
        let poly = make_polygon("empty");
        let s = polygon_to_string(&poly);
        let poly2 = make_polygon(&s);
        assert!(poly2.is_empty_polygon());
    }

    #[test]
    fn test_polygon_to_string_full() {
        let poly = make_polygon("full");
        let s = polygon_to_string(&poly);
        let poly2 = make_polygon(&s);
        assert!(poly2.is_full_polygon());
    }

    #[test]
    fn test_make_polygon_two_loops_roundtrip() {
        let original = "0:0, 0:20, 20:0; 1:1, 1:5, 5:1";
        let poly = make_polygon(original);
        assert_eq!(poly.num_loops(), 2);
        let s = polygon_to_string(&poly);
        let poly2 = make_polygon(&s);
        assert_eq!(poly2.num_loops(), 2);
    }

    #[test]
    fn test_parse_point_precision() {
        // Verify precise parsing of coordinates.
        let p = parse_point("47.1234567:-122.9876543");
        let ll = LatLng::from_point(p);
        assert!(
            (ll.lat.degrees() - 47.1234567).abs() < 1e-6,
            "lat = {}, expected 47.1234567",
            ll.lat.degrees()
        );
        assert!(
            (ll.lng.degrees() - (-122.9876543)).abs() < 1e-6,
            "lng = {}, expected -122.9876543",
            ll.lng.degrees()
        );
    }

    #[test]
    fn test_make_polyline_roundtrip() {
        let pl = make_polyline("0:0, 10:20, -30:45");
        assert_eq!(pl.len(), 3);
        // Verify first and last vertex coordinates.
        let ll0 = LatLng::from_point(pl.vertex(0));
        assert!((ll0.lat.degrees()).abs() < 1e-6);
        assert!((ll0.lng.degrees()).abs() < 1e-6);
        let ll2 = LatLng::from_point(pl.vertex(2));
        assert!((ll2.lat.degrees() - (-30.0)).abs() < 1e-6);
        assert!((ll2.lng.degrees() - 45.0).abs() < 1e-6);
    }

    #[test]
    fn test_make_rect_contains_center() {
        let rect = make_rect("10:20, 30:40");
        // Center should be contained.
        let center = LatLng::from_degrees(20.0, 30.0);
        assert!(rect.contains_lat_lng(center));
        // Outside point should not be contained.
        let outside = LatLng::from_degrees(50.0, 50.0);
        assert!(!rect.contains_lat_lng(outside));
    }

    // ─── C++ ToString SpecialCases / format_g15 equivalents ───────────

    #[test]
    fn test_format_g15_zero() {
        // C++: "0:0" for (0,0)
        assert_eq!(format_g15(0.0), "0");
    }

    #[test]
    fn test_format_g15_scientific_small() {
        // C++: "1e-20:1e-30"
        let s = format_g15(1e-20);
        // Should use scientific notation for very small values.
        assert!(
            s.contains("e-") || s.contains("E-") || s == "1e-20",
            "format_g15(1e-20) = '{s}', expected scientific notation"
        );
        // Roundtrip: parsing back should give the same value.
        let v: f64 = s.parse().unwrap();
        assert!(
            (v - 1e-20).abs() / 1e-20 < 1e-10,
            "roundtrip: '{s}' -> {v}, expected 1e-20"
        );
    }

    #[test]
    fn test_format_g15_scientific_very_small() {
        let s = format_g15(1e-30);
        let v: f64 = s.parse().unwrap();
        assert!(
            (v - 1e-30).abs() / 1e-30 < 1e-10,
            "roundtrip: '{s}' -> {v}, expected 1e-30"
        );
    }

    #[test]
    fn test_format_g15_pi() {
        // PI should roundtrip through format_g15 with 15 significant digits.
        let s = format_g15(std::f64::consts::PI);
        let v: f64 = s.parse().unwrap();
        assert!(
            (v - std::f64::consts::PI).abs() < 1e-14,
            "format_g15(PI) = '{s}', parsed = {v}"
        );
    }

    #[test]
    fn test_format_g15_negative() {
        let s = format_g15(-12.7);
        let v: f64 = s.parse().unwrap();
        assert_eq!(v, -12.7, "format_g15(-12.7) = '{s}'");
    }

    #[test]
    fn test_format_g15_integer() {
        // Integer values should not have trailing ".0".
        let s = format_g15(90.0);
        assert_eq!(s, "90");
    }

    #[test]
    fn test_point_to_string_origin() {
        // C++ TEST(ToString, SpecialCases): "0:0" for (0,0).
        let p = LatLng::from_degrees(0.0, 0.0).to_point();
        let s = point_to_string(p);
        assert_eq!(s, "0:0");
    }

    #[test]
    fn test_point_to_string_north_pole() {
        // C++ TEST(ToString, SpecialCases): "90:0" for north pole.
        let p = Point::from_coords(0.0, 0.0, 1.0);
        let s = point_to_string(p);
        assert_eq!(s, "90:0");
    }

    #[test]
    fn test_point_to_string_negative_zeros() {
        // C++ TEST(ToString, NegativeZeros): negative zero coords should
        // format the same as positive zeros.
        let p_neg_y = Point::from_coords(1.0, -0.0, 0.0);
        let p_pos_y = Point::from_coords(1.0, 0.0, 0.0);
        assert_eq!(point_to_string(p_neg_y), point_to_string(p_pos_y));

        let p_neg_z = Point::from_coords(1.0, 0.0, -0.0);
        assert_eq!(point_to_string(p_neg_z), point_to_string(p_pos_y));
    }

    #[test]
    fn test_point_to_string_small_values() {
        // C++ TEST(ToString, SpecialCases): "1e-20:1e-30".
        let p = LatLng::from_degrees(1e-20, 1e-30).to_point();
        let s = point_to_string(p);
        assert_eq!(s, "1e-20:1e-30");
    }

    #[test]
    fn test_point_to_string_negative_zeros_more() {
        // Additional C++ NegativeZeros cases.
        assert_eq!(
            point_to_string(Point::from_coords(-1.0, -0.0, 0.0)),
            "0:180"
        );
        assert_eq!(
            point_to_string(Point::from_coords(-1.0, 0.0, -0.0)),
            "0:180"
        );
        assert_eq!(point_to_string(Point::from_coords(-0.0, 0.0, 1.0)), "90:0");
    }

    #[test]
    fn test_loop_to_string_empty() {
        // C++ TEST(ToString, EmptyLoop).
        let l = Loop::empty();
        assert_eq!(loop_to_string(&l), "empty");
    }

    #[test]
    fn test_loop_to_string_full() {
        // C++ TEST(ToString, FullLoop).
        let l = Loop::full();
        assert_eq!(loop_to_string(&l), "full");
    }

    #[test]
    fn test_latlng_to_string_format() {
        // Should use %.15g format matching point_to_string.
        let ll = LatLng::from_degrees(45.0, 90.0);
        assert_eq!(latlng_to_string(ll), "45:90");
    }

    #[test]
    fn test_make_lax_polygon_empty() {
        // C++ TEST(MakeLaxPolygon, Empty).
        let p = make_lax_polygon("");
        assert_eq!(p.num_loops(), 0);
        let p = make_lax_polygon("empty");
        assert_eq!(p.num_loops(), 0);
    }

    #[test]
    fn test_make_lax_polygon_full() {
        // C++ TEST(MakeLaxPolygon, Full).
        let p = make_lax_polygon("full");
        assert_eq!(p.num_loops(), 1);
        assert_eq!(p.num_loop_vertices(0), 0);
    }

    #[test]
    fn test_make_lax_polygon_full_with_hole() {
        // C++ TEST(MakeLaxPolygon, FullWithHole).
        let p = make_lax_polygon("full; 0:0, 0:1, 1:0");
        assert_eq!(p.num_loops(), 2);
        assert_eq!(p.num_loop_vertices(0), 0); // full loop
        assert_eq!(p.num_loop_vertices(1), 3); // hole
    }

    #[test]
    fn test_make_index_basic() {
        // C++ TEST(ToString, S2ShapeIndex) cases.
        let index = make_index("# #");
        assert_eq!(index.num_shape_ids(), 0);

        let index = make_index("0:0 # #");
        assert_eq!(index.num_shape_ids(), 1); // one PointVector

        let index = make_index("# 0:0, 0:0 #");
        assert_eq!(index.num_shape_ids(), 1); // one polyline

        let index = make_index("# # 0:0, 0:1, 1:0");
        assert_eq!(index.num_shape_ids(), 1); // one polygon

        let index = make_index("5:5 # 6:6, 7:7 # 0:0, 0:1, 1:0");
        assert_eq!(index.num_shape_ids(), 3); // point + polyline + polygon
    }

    #[test]
    fn test_polyline_to_string_roundtrip() {
        let pl = make_polyline("0:0, 10:20, -30:45");
        let s = polyline_to_string(&pl);
        let pl2 = make_polyline(&s);
        assert_eq!(pl.len(), pl2.len());
    }

    #[test]
    fn test_lax_polyline_roundtrip() {
        let pl = make_lax_polyline("0:0, 10:20, -30:45");
        let s = lax_polyline_to_string(&pl);
        let pl2 = make_lax_polyline(&s);
        assert_eq!(pl.num_vertices(), pl2.num_vertices());
    }

    #[test]
    fn test_lax_polygon_to_string_roundtrip() {
        let p = make_lax_polygon("0:0, 0:10, 10:0; 1:1, 1:5, 5:1");
        let s = lax_polygon_to_string(&p);
        let p2 = make_lax_polygon(&s);
        assert_eq!(p.num_loops(), p2.num_loops());
    }

    // ═══════════════════════════════════════════════════════════════════
    // C++ s2text_format_test.cc ports
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn test_to_string_special_cases() {
        // C++ TEST(ToString, SpecialCases)
        assert_eq!("0:0", latlng_to_string(LatLng::from_degrees(0.0, 0.0)));
        assert_eq!("90:0", point_to_string(Point::from_coords(0.0, 0.0, 1.0)));
    }

    #[test]
    fn test_to_string_face_cell_id() {
        // C++ TEST(ToString, FaceCellId)
        use crate::s2::CellId;
        assert_eq!("2/", CellId::from_face(2).to_debug_string());
    }

    #[test]
    fn test_to_string_level3_cell_id() {
        // C++ TEST(ToString, Level3CellId)
        use crate::s2::CellId;
        let id = CellId::from_face(2).children()[0].children()[3].children()[3];
        assert_eq!("2/033", id.to_debug_string());
    }

    #[test]
    fn test_to_string_empty_polyline() {
        // C++ TEST(ToString, EmptyPolyline)
        let pl = crate::s2::polyline::Polyline::new(vec![]);
        assert_eq!("", polyline_to_string(&pl));
    }

    #[test]
    fn test_to_string_empty_point_vector() {
        // C++ TEST(ToString, EmptyPointVector)
        let pts: Vec<Point> = vec![];
        assert_eq!("", points_to_string(&pts));
    }

    #[test]
    fn test_to_string_s2_shape_index_roundtrips() {
        // C++ TEST(ToString, S2ShapeIndex) — verifies various index roundtrips
        fn test_index_str(s: &str) {
            let idx = make_index(s);
            let result = index_to_string(&idx);
            assert_eq!(s, result, "index roundtrip failed for: {s}");
        }
        test_index_str("# #");
        test_index_str("0:0 # #");
        test_index_str("0:0 | 1:0 # #");
        test_index_str("# 0:0, 0:0 #");
        test_index_str("# 0:0, 0:0 | 1:0, 2:0 #");
        test_index_str("# # 0:0, 0:1, 1:0");
    }

    #[test]
    fn test_to_string_point_shape_works() {
        // C++ TEST(ToString, PointShapeWorks)
        let idx = make_index("0:0 | 0:5 | 5:0 # #");
        let shape = idx.shape(0).unwrap();
        assert_eq!(shape.dimension(), crate::s2::shape::Dimension::Point);
    }

    #[test]
    fn test_to_string_lax_polygon_loop_separator() {
        // C++ TEST(ToString, LaxPolygonLoopSeparator)
        let loop1 = "0:0, 0:5, 5:0";
        let loop2 = "1:1, 4:1, 1:4";
        let p = make_lax_polygon(&format!("{loop1}; {loop2}"));
        assert_eq!(p.num_loops(), 2);
        let s = lax_polygon_to_string(&p);
        // Should contain both loops separated by "; "
        assert!(s.contains("0:0"), "missing loop1 in: {s}");
        assert!(s.contains("1:1"), "missing loop2 in: {s}");
    }

    #[test]
    fn test_to_string_s2_latlng_span() {
        // C++ TEST(ToString, S2LatLngSpan)
        let latlngs = parse_latlngs("-20:150, -20:151, -19:150");
        assert_eq!(3, latlngs.len());
        assert_eq!(latlngs[0], LatLng::from_degrees(-20.0, 150.0));
        assert_eq!(latlngs[1], LatLng::from_degrees(-20.0, 151.0));
        assert_eq!(latlngs[2], LatLng::from_degrees(-19.0, 150.0));
    }
}
