// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library — a derivative work of the
// upstream Apache-2.0 implementations (Copyright Google Inc.):
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java
// See LICENSE.

//! A library for spherical geometry on the unit sphere (S²).
//!
//! This crate provides robust, flexible, and performant operations on spherical
//! geometry. All geometry lives on the surface of the unit sphere in
//! three-dimensional space, with points represented as unit-length vectors
//! rather than latitude-longitude pairs. This representation avoids
//! singularities at the poles, eliminates discontinuities at the antimeridian,
//! and enables efficient computation with geodesic edges (shortest paths on the
//! sphere).
//!
//! # Key properties
//!
//! - **Robustness**: Core predicates use conservative error bounds with exact
//!   arithmetic fallback, guaranteeing topologically correct results.
//! - **Geodesic edges**: All edges follow great circles, behaving consistently
//!   everywhere on the sphere without special-casing poles or meridians.
//! - **Hierarchical indexing**: The sphere is decomposed into a hierarchy of
//!   cells ([`s2::CellId`]) ordered along a space-filling Hilbert curve,
//!   enabling fast spatial queries through [`s2::shape_index::ShapeIndex`].
//! - **No unsafe code**: The entire crate is built with `#![forbid(unsafe_code)]`.
//!
//! # Module hierarchy
//!
//! The crate is organized into five modules corresponding to the mathematical
//! spaces they operate in:
//!
//! | Module | Space | Key types |
//! |--------|-------|-----------|
//! | [`r1`] | Real line (ℝ¹) | [`r1::Interval`] |
//! | [`r2`] | Euclidean plane (ℝ²) | [`r2::Point`], [`r2::Rect`] |
//! | [`r3`] | Euclidean 3-space (ℝ³) | [`r3::Vector`], [`r3::PreciseVector`], [`r3::ExactFloat`] |
//! | [`s1`] | Unit circle (S¹) | [`s1::Angle`], [`s1::ChordAngle`], [`s1::Interval`] |
//! | [`s2`] | Unit sphere (S²) | [`s2::Point`], [`s2::CellId`], [`s2::Cell`], [`s2::Loop`], [`s2::Polygon`], and many more |
//!
//! Most users will work primarily with the types in [`s2`].
//!
//! # Core workflow
//!
//! 1. **Represent** geometry using [`s2::Point`], [`s2::Loop`],
//!    [`s2::Polygon`], or [`s2::polyline::Polyline`].
//! 2. **Index** geometry in a [`ShapeIndex`](s2::shape_index::ShapeIndex) for
//!    fast spatial queries.
//! 3. **Query** using [`s2::closest_edge_query`], [`s2::contains_point_query`],
//!    [`s2::crossing_edge_query`], or [`s2::boolean_operation`].
//! 4. **Cover** regions with cells using [`s2::region_coverer::RegionCoverer`]
//!    for spatial indexing in databases.
//! 5. **Build** new geometry from raw edges with
//!    [`s2::builder::S2Builder`], which handles vertex snapping and
//!    topological assembly.
//!
//! # Quick example
//!
//! ```
//! use s2rst::s1::Angle;
//! use s2rst::s2::{Cap, LatLng, Point, Region};
//! use s2rst::s2::region_coverer::RegionCoverer;
//!
//! // Define a spherical cap (disc) centered on Paris.
//! let center = LatLng::from_degrees(48.8566, 2.3522).to_point();
//! let cap = Cap::from_center_angle(center, Angle::from_degrees(0.5));
//!
//! // Approximate the cap with a covering of S2 cells.
//! let coverer = RegionCoverer::new().max_level(14).max_cells(8);
//! let covering = coverer.covering(&cap);
//! assert!(!covering.is_empty());
//!
//! // Every point inside the cap is contained by the covering.
//! assert!(covering.contains_point(center));
//! ```

#![forbid(unsafe_code)]
// Clippy reports a phantom `large_stack_arrays` warning (dummy span at byte 0)
// from the recursive const fn `init_lookup_tables` in `cell_id.rs`, which passes
// `[u16; 1024]` by value through 4 recursion levels during compile-time static
// init. No runtime stack allocation occurs. Module-level `#[allow]` cannot
// suppress it — only a crate-root attribute works. Only fires under `cfg(test)`.
#![cfg_attr(
    test,
    allow(
        clippy::large_stack_arrays,
        reason = "phantom warning from compile-time static init in cell_id.rs — see comment above"
    )
)]
#![warn(clippy::cast_lossless)]
#![warn(clippy::cast_sign_loss)]
#![warn(clippy::cast_possible_truncation)]
#![warn(clippy::cast_possible_wrap)]
#![cfg_attr(not(test), warn(clippy::panic))]
#![cfg_attr(not(test), warn(clippy::expect_used))]
#![cfg_attr(not(test), warn(clippy::unwrap_used))]

pub mod r1;
pub mod r2;
pub mod r3;
pub mod s1;
pub mod s2;

#[cfg(feature = "geo-types")]
pub mod geo_types_interop;
