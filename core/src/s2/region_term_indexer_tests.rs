// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original tests for [`super::RegionTermIndexer`]'s options (specs/009-bounded-types). Written
//! for this crate, not ported from upstream S2.

use super::*;
use crate::s2::{Cap, LatLng, Level};

fn reversed() -> RegionTermIndexer {
    let mut indexer = RegionTermIndexer::new();
    indexer.options_mut().min_level = Level::new(10);
    indexer.options_mut().max_level = Level::new(5);
    indexer
}

/// B12 (spec 009): reversed levels fail at once, in every build, with a message naming both.
/// Before, a release build's `get_index_terms` did not finish (the Python extension, probed),
/// and the point terms underflowed `max_level - min_level`.
#[test]
#[should_panic(expected = "min_level (10) must not exceed max_level (5)")]
fn index_terms_reject_reversed_levels() {
    drop(reversed().get_index_terms(&Cap::full()));
}

#[test]
#[should_panic(expected = "min_level (10) must not exceed max_level (5)")]
fn point_terms_reject_reversed_levels() {
    drop(reversed().get_index_terms_for_point(LatLng::from_degrees(1.0, 2.0).to_point()));
}

#[test]
#[should_panic(expected = "min_level (10) must not exceed max_level (5)")]
fn query_terms_reject_reversed_levels() {
    drop(reversed().get_query_terms_for_point(LatLng::from_degrees(1.0, 2.0).to_point()));
}
