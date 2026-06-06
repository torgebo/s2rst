// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Edge tessellation under a map projection: subdivide geodesic edges so the
//! projected polyline stays within a tolerance of the true geodesic (for
//! rendering great-circle arcs on a flat map), and the inverse.

use wasm_bindgen::prelude::*;

use s2rst::s2::projections::{MercatorProjection, PlateCarreeProjection, Projection};

use crate::angle::Angle;
use crate::point::Point;

#[derive(Clone, Copy, Debug)]
enum ProjKind {
    PlateCarree(f64),
    Mercator(f64),
}

/// Tessellates edges under a chosen map projection to a tolerance.
#[wasm_bindgen]
pub struct EdgeTessellator {
    kind: ProjKind,
    tolerance_radians: f64,
}

#[wasm_bindgen]
impl EdgeTessellator {
    /// Tessellator using an equirectangular (Plate Carrée) projection with the
    /// given x-scale.
    #[wasm_bindgen(js_name = "plateCarree")]
    pub fn plate_carree(x_scale: f64, tolerance: &Angle) -> EdgeTessellator {
        EdgeTessellator {
            kind: ProjKind::PlateCarree(x_scale),
            tolerance_radians: tolerance.0.radians(),
        }
    }

    /// Tessellator using a Mercator projection with the given max longitude
    /// (radians) mapped to the horizontal extent.
    pub fn mercator(max_lng: f64, tolerance: &Angle) -> EdgeTessellator {
        EdgeTessellator {
            kind: ProjKind::Mercator(max_lng),
            tolerance_radians: tolerance.0.radians(),
        }
    }

    /// Tessellate the geodesic edge `(a, b)` into projected 2D vertices,
    /// returned flat as `[x0, y0, x1, y1, ...]`.
    #[wasm_bindgen(js_name = "tessellateProjected")]
    pub fn tessellate_projected(&self, a: &Point, b: &Point) -> Vec<f64> {
        let tol = s2rst::s1::Angle::from_radians(self.tolerance_radians);
        let mut out: Vec<s2rst::r2::Point> = Vec::new();
        match self.kind {
            ProjKind::PlateCarree(xs) => {
                let t = s2rst::s2::edge_tessellator::EdgeTessellator::new(
                    PlateCarreeProjection::new(xs),
                    tol,
                );
                t.append_projected(a.0, b.0, &mut out);
            }
            ProjKind::Mercator(ml) => {
                let t = s2rst::s2::edge_tessellator::EdgeTessellator::new(
                    MercatorProjection::new(ml),
                    tol,
                );
                t.append_projected(a.0, b.0, &mut out);
            }
        }
        out.iter().flat_map(|p| [p.x, p.y]).collect()
    }

    /// Tessellate a straight projected edge `(ax,ay)->(bx,by)` into sphere points.
    #[wasm_bindgen(js_name = "tessellateUnprojected")]
    pub fn tessellate_unprojected(&self, ax: f64, ay: f64, bx: f64, by: f64) -> Vec<Point> {
        let tol = s2rst::s1::Angle::from_radians(self.tolerance_radians);
        let pa = s2rst::r2::Point::new(ax, ay);
        let pb = s2rst::r2::Point::new(bx, by);
        let mut out: Vec<s2rst::s2::Point> = Vec::new();
        match self.kind {
            ProjKind::PlateCarree(xs) => {
                let t = s2rst::s2::edge_tessellator::EdgeTessellator::new(
                    PlateCarreeProjection::new(xs),
                    tol,
                );
                t.append_unprojected(pa, pb, &mut out);
            }
            ProjKind::Mercator(ml) => {
                let t = s2rst::s2::edge_tessellator::EdgeTessellator::new(
                    MercatorProjection::new(ml),
                    tol,
                );
                t.append_unprojected(pa, pb, &mut out);
            }
        }
        out.iter().map(|p| Point(*p)).collect()
    }

    /// Project a sphere point to map coordinates `[x, y]`.
    pub fn project(&self, point: &Point) -> Vec<f64> {
        let r = match self.kind {
            ProjKind::PlateCarree(xs) => PlateCarreeProjection::new(xs).project(point.0),
            ProjKind::Mercator(ml) => MercatorProjection::new(ml).project(point.0),
        };
        vec![r.x, r.y]
    }

    /// Unproject map coordinates `(x, y)` to a sphere point.
    pub fn unproject(&self, x: f64, y: f64) -> Point {
        let p = s2rst::r2::Point::new(x, y);
        let r = match self.kind {
            ProjKind::PlateCarree(xs) => PlateCarreeProjection::new(xs).unproject(p),
            ProjKind::Mercator(ml) => MercatorProjection::new(ml).unproject(p),
        };
        Point(r)
    }
}
