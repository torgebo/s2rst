// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use wasm_bindgen::prelude::*;

use crate::angle::Angle;

/// A vertex snap function for builder-based operations (`simplified`, buffering).
///
/// `wasm-bindgen` cannot export the `dyn SnapFunction` trait object directly, so
/// this is an opaque handle that records which concrete snap function to build;
/// consumers call [`SnapFunction::build`] internally to obtain a fresh
/// `Box<dyn SnapFunction>`.
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct SnapFunction(Kind);

#[derive(Clone, Debug)]
enum Kind {
    /// Snap radius in radians.
    Identity(f64),
    /// S2 cell level (0–30).
    CellId(u8),
    /// Lat/lng grid exponent.
    IntLatLng(i32),
}

#[wasm_bindgen]
impl SnapFunction {
    /// Identity snapping: vertices closer than `snapRadius` collapse together,
    /// but vertices are otherwise left where they are.
    pub fn identity(snap_radius: &Angle) -> SnapFunction {
        SnapFunction(Kind::Identity(snap_radius.0.radians()))
    }

    /// Snap vertices to S2 cell centers at the given level (0–30). Higher levels
    /// snap more finely.
    #[wasm_bindgen(js_name = "cellId")]
    pub fn cell_id(level: u8) -> SnapFunction {
        SnapFunction(Kind::CellId(level))
    }

    /// Snap vertices to a lat/lng grid with `10^(-exponent)` degree spacing
    /// (e.g. `exponent = 6` for E6 micro-degree precision).
    #[wasm_bindgen(js_name = "intLatLng")]
    pub fn int_lat_lng(exponent: i32) -> SnapFunction {
        SnapFunction(Kind::IntLatLng(exponent))
    }
}

impl SnapFunction {
    /// Build a fresh boxed core snap function. Internal: each builder operation
    /// needs its own owned `Box<dyn SnapFunction>`.
    pub(crate) fn build(&self) -> Box<dyn s2rst::s2::builder::snap::SnapFunction> {
        use s2rst::s2::builder::snap::{
            IdentitySnapFunction, IntLatLngSnapFunction, S2CellIdSnapFunction,
        };
        match self.0 {
            Kind::Identity(radians) => Box::new(IdentitySnapFunction::new(
                s2rst::s1::Angle::from_radians(radians),
            )),
            Kind::CellId(level) => Box::new(S2CellIdSnapFunction::new(level)),
            Kind::IntLatLng(exponent) => Box::new(IntLatLngSnapFunction::new(exponent)),
        }
    }
}
