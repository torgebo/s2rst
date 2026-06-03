// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Benchmarks ported from C++: `s2builder_test.cc`, `s2builderutil_find_polygon_degeneracies_test.cc`
//! `BM_NearlyParallelCrossingEdges`, `BM_FindPolygonDegeneracies`
#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};

use s2rst::s2::LatLng;
use s2rst::s2::builder::polygon_layer::S2PolygonLayer;
use s2rst::s2::builder::snap::IdentitySnapFunction;
use s2rst::s2::builder::{Options as BuilderOptions, S2Builder};
use s2rst::s2::earth;
use s2rst::s2::edge_distances::point_on_line;

// C++: BM_NearlyParallelCrossingEdges
// Two clusters of vertices 100m apart, edges zigzag between them.
// Simplified: deterministic vertices near two points, no random sampling.
#[library_benchmark]
fn nearly_parallel_crossing_10_edges() {
    let a = LatLng::from_degrees(0.0, 0.0).to_point();
    let b = point_on_line(
        a,
        LatLng::from_degrees(0.0, 1.0).to_point(),
        earth::meters_to_angle(100.0),
    );
    let perturb = earth::meters_to_angle(0.0001); // 100 microns

    let mut vertices = Vec::new();
    for i in 0..=10 {
        let t = f64::from(i) / 10.0;
        // Tiny perturbation to create self-intersections.
        let offset_lat = (t * 137.0).sin() * perturb.radians().to_degrees();
        let offset_lng = (t * 251.0).cos() * perturb.radians().to_degrees();
        if i % 2 == 0 {
            vertices.push(LatLng::from_degrees(offset_lat, offset_lng).to_point());
        } else {
            let ll = LatLng::from_point(b);
            vertices.push(
                LatLng::from_degrees(ll.lat.degrees() + offset_lat, ll.lng.degrees() + offset_lng)
                    .to_point(),
            );
        }
    }

    let snap_radius = earth::meters_to_angle(0.000001); // 1 micron
    let snap_fn = IdentitySnapFunction::new(snap_radius);
    let options = BuilderOptions {
        snap_function: Box::new(snap_fn),
        split_crossing_edges: true,
        ..BuilderOptions::default()
    };
    let mut builder = S2Builder::new(options);
    let layer = S2PolygonLayer::new();
    builder.start_layer(Box::new(layer));
    builder.add_polyline_from_points(&vertices);
    drop(builder.build());
}

library_benchmark_group!(
    name = builder_advanced_benchmarks;
    benchmarks =
        nearly_parallel_crossing_10_edges
);

main!(library_benchmark_groups = builder_advanced_benchmarks);
