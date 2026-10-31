// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Unit tests of [`crate::region_spec`] (spec 011 data-model §1), and the drift guards that pin
//! the library's text of upstream's constructions (P-UBND-04, P-UEXP-01): each guard compares a
//! function's body, one trimmed line per element with comments and blank lines dropped, with the
//! listing below it, whose comments name upstream's lines. A change to one of these functions
//! fails its guard until the listing (and the claim that it is upstream's) is reviewed.

use super::*;
use s2rst::r3::Vector;

fn raw(x: f64, y: f64, z: f64) -> Point {
    Point(Vector { x, y, z })
}

#[test]
fn pr_contains_is_numeric_equality() {
    assert!(pr_contains(raw(0.0, 0.0, 1.0), raw(-0.0, -0.0, 1.0)));
    assert!(!pr_contains(raw(0.0, 0.0, 1.0), raw(0.0, 1e-300, 1.0)));
    let nan = raw(f64::NAN, 0.0, 1.0);
    assert!(!pr_contains(nan, nan));
}

#[test]
fn upstream_average_area_at_hand_values() {
    assert_eq!(upstream_average_area(0), 4.0 * PI / 6.0);
    assert_eq!(upstream_average_area(1), PI / 6.0);
    assert_eq!(upstream_average_area(30), 4.0 * PI / 6.0 / 2f64.powi(60));
}

/// The body of the function whose signature line is `header`, ending at the closing brace at
/// `indent`: one trimmed line per element, comments and blank lines dropped.
fn body(src: &str, header: &str, indent: &str) -> Vec<String> {
    let start = src
        .find(header)
        .unwrap_or_else(|| panic!("{header} not found"));
    let rest = &src[start + header.len()..];
    let end = rest
        .find(&format!("\n{indent}}}\n"))
        .expect("the function's closing brace");
    rest[..end]
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("//"))
        .map(String::from)
        .collect()
}

const CELL_UNION: &str = include_str!("../../core/src/s2/cell_union.rs");
const POINT_REGION: &str = include_str!("../../core/src/s2/point_region.rs");

/// `S2CellUnion::GetCapBound` (`s2cell_union.cc:235–259`): the empty cap for no ids; the
/// centroid `Σ AverageArea(level) · ToPoint()` (`area * id.ToPoint()`: IEEE products commute);
/// `(1, 0, 0)` when it is zero, else normalised; then `AddCap` of each cell's `GetCapBound()`.
#[test]
fn cap_bound_is_upstreams() {
    assert_eq!(
        body(CELL_UNION, "    fn cap_bound(&self) -> Cap {", "    "),
        [
            "if self.0.is_empty() {",
            "return Cap::empty();",
            "}",
            "let mut centroid = crate::r3::Vector::default();",
            "for &id in &self.0 {",
            "let area = Cell::from_cell_id(id).average_area();",
            "centroid = centroid + id.to_point().vector() * area;",
            "}",
            "let mut cap = if centroid == crate::r3::Vector::default() {",
            "Cap::from_point(Point::from_coords(1.0, 0.0, 0.0))",
            "} else {",
            "Cap::from_point(Point(centroid.normalize()))",
            "};",
            "for &id in &self.0 {",
            "cap = cap.add_cap(Cell::from_cell_id(id).cap_bound());",
            "}",
            "cap",
        ]
    );
}

/// `S2CellUnion::GetRectBound` (`s2cell_union.cc:261–267`): `Union` of each cell's bound from
/// `S2LatLngRect::Empty()`.
#[test]
fn rect_bound_is_upstreams() {
    assert_eq!(
        body(CELL_UNION, "    fn rect_bound(&self) -> Rect {", "    "),
        [
            "let mut bound = Rect::empty();",
            "for &id in &self.0 {",
            "bound = bound.union(Cell::from_cell_id(id).rect_bound());",
            "}",
            "bound",
        ]
    );
}

/// `S2CellUnion::GetCellUnionBound` (`s2cell_union.cc:269–273`) and
/// `S2PointRegion::GetCellUnionBound` (`s2point_region.cc`): the cap bound's covering.
#[test]
fn cell_union_bounds_are_the_caps() {
    for src in [CELL_UNION, POINT_REGION] {
        assert_eq!(
            body(
                src,
                "    fn cell_union_bound(&self) -> Vec<CellId> {",
                "    "
            ),
            ["self.cap_bound().cell_union_bound()"]
        );
    }
}

/// `ApproxArea` and `ExactArea` (`s2cell_union.cc:499–513`): `double area = 0`, then
/// `area += S2Cell(id).…Area()` in order (spec 011 FR-018).
#[test]
fn areas_sum_from_zero() {
    for m in ["approx_area", "exact_area"] {
        assert_eq!(
            body(
                CELL_UNION,
                &format!("    pub fn {m}(&self) -> f64 {{"),
                "    "
            ),
            [
                "self.0",
                ".iter()",
                &format!(".fold(0.0, |area, &id| area + Cell::from_cell_id(id).{m}())"),
            ]
        );
    }
}

/// `S2CellUnion::Expand(S1Angle, int)` (`s2cell_union.cc:471–484`): the coarsest level; the
/// metric's level for the radius (`GetLevelForMinValue`, which 008's P-CPBND-02 proves the
/// library's `MIN_WIDTH.max_level` computes); a first expansion at level 0 when the radius is
/// above a face's width; then `Expand(min(min_level + max_level_diff, radius_level))`, which the
/// `u8` difference reaches by saturating and clamping to 30 (every level an `int` sum above 30
/// gives the same `min`). Upstream's coarsest level of the empty union is 30, the library's 0:
/// expanding the empty union is empty either way.
#[test]
fn expand_level_is_upstreams() {
    assert_eq!(
        body(
            CELL_UNION,
            "    pub fn expand_by_radius(&mut self, min_radius: Angle, max_level_diff: u8) {",
            "    "
        ),
        [
            "let min_level = self",
            ".0",
            ".iter()",
            ".map(|id| id.level())",
            ".min()",
            ".unwrap_or(Level::MIN);",
            "let radius_level = MIN_WIDTH.max_level(min_radius.radians());",
            "if radius_level == Level::MIN && min_radius.radians() > MIN_WIDTH.value(Level::MIN) {",
            "self.expand_at_level(Level::MIN);",
            "}",
            "let level =",
            "Level::new((min_level.as_u8().saturating_add(max_level_diff)).min(Level::MAX.as_u8()))",
            ".min(radius_level);",
            "self.expand_at_level(level);",
        ]
    );
}

const UNION: &str = include_str!("../../core/src/s2/region_union.rs");
const INTERSECTION: &str = include_str!("../../core/src/s2/region_intersection.rs");

/// `S2RegionUnion` and `S2RegionIntersection` (`s2region_union.cc`, `s2region_intersection.cc`):
/// `GetCapBound` is `GetRectBound().GetCapBound()`, `GetCellUnionBound` the cap's (P-ADBND-02,
/// spec 012).
#[test]
fn adaptors_are_upstreams() {
    for src in [UNION, INTERSECTION] {
        assert_eq!(
            body(src, "    fn cap_bound(&self) -> Cap {", "    "),
            ["self.rect_bound().cap_bound()"]
        );
        assert_eq!(
            body(
                src,
                "    fn cell_union_bound(&self) -> Vec<CellId> {",
                "    "
            ),
            ["self.cap_bound().cell_union_bound()"]
        );
    }
}
