// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Red/green tests for spec 009's bounded arguments, through the JavaScript glue. Original,
//! not ported from upstream S2.
//!
//! Each test wraps a Rust object as its JavaScript class instance and calls the method by name
//! with raw JavaScript values, so the arguments pass through wasm-bindgen's conversions as a
//! JavaScript caller's would. Before spec 009 a level parameter was a `u8`, which arrives
//! through ToInt32 and `as u8`: 286 became 30, −1 became 255, `NaN` became 0, and an
//! out-of-range level panicked, which aborts the instance. Afterwards every such value throws a
//! catchable `Error` and the instance stays usable. The tests are numbered as in
//! `specs/009-bounded-types/contracts/red-green-tests.md`.

use js_sys::{Array, BigInt, Function, Reflect};
use s2rst_wasm::*;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::*;

// ---------------------------------------------------------------------------
// Calling through the JavaScript glue
// ---------------------------------------------------------------------------

/// Calls `target[method](...args)` in JavaScript.
fn call(target: &JsValue, method: &str, args: &[JsValue]) -> Result<JsValue, JsValue> {
    let f: Function = Reflect::get(target, &JsValue::from_str(method))
        .unwrap_or_else(|_| panic!("no property {method}"))
        .dyn_into()
        .unwrap_or_else(|_| panic!("{method} is not a function"));
    let array = Array::new();
    for a in args {
        array.push(a);
    }
    f.apply(target, &array)
}

/// The JavaScript class of a wrapped instance, for its static methods.
fn class_of(instance: &JsValue) -> JsValue {
    Reflect::get(instance, &JsValue::from_str("constructor")).expect("constructor")
}

/// A description of a call's outcome, for the red/green record.
fn outcome(r: &Result<JsValue, JsValue>) -> String {
    match r {
        Ok(v) => match Reflect::get(v, &JsValue::from_str("level")) {
            Ok(l) if l.as_f64().is_some() => format!("returns (level {})", l.as_f64().unwrap()),
            _ => "returns".to_string(),
        },
        Err(e) => match e.dyn_ref::<js_sys::Error>() {
            Some(err) => format!(
                "throws {}: {}",
                String::from(err.name()),
                String::from(err.message())
            ),
            None => match e.as_string() {
                Some(text) => format!("throws the string {text:?}"),
                None => format!("throws {e:?}"),
            },
        },
    }
}

/// The out-of-range values for a level, as a JavaScript caller passes them.
fn bad_levels() -> Vec<(&'static str, JsValue)> {
    vec![
        ("31", JsValue::from_f64(31.0)),
        ("286", JsValue::from_f64(286.0)),
        ("-1", JsValue::from_f64(-1.0)),
        ("NaN", JsValue::from_f64(f64::NAN)),
        ("3.5", JsValue::from_f64(3.5)),
        ("Infinity", JsValue::from_f64(f64::INFINITY)),
        ("5n", BigInt::from(5).into()),
    ]
}

/// The out-of-range values for a face.
fn bad_faces() -> Vec<(&'static str, JsValue)> {
    vec![
        ("6", JsValue::from_f64(6.0)),
        ("7", JsValue::from_f64(7.0)),
        ("-1", JsValue::from_f64(-1.0)),
        ("NaN", JsValue::from_f64(f64::NAN)),
        ("3.5", JsValue::from_f64(3.5)),
        ("5n", BigInt::from(5).into()),
    ]
}

/// Asserts that every bad level throws an exception naming the level's range, and that `valid`
/// still succeeds afterwards (see [`check_bounded_method`]).
fn check_level_method(
    label: &str,
    attempt: impl FnMut(JsValue) -> Result<JsValue, JsValue>,
    valid: f64,
) {
    check_bounded_method(label, "level must be", bad_levels(), attempt, valid);
}

/// Asserts that every value of `bad` throws an exception whose message contains `needle`
/// (the crate's error funnel throws the message as a string), and that `valid` still succeeds
/// afterwards. A `BigInt` is rejected by the glue before any Rust runs, with an `Error` of its
/// own ("expected a number argument, found bigint" in a debug build), which is catchable, so
/// any `Error` counts for it. Reports every outcome on failure.
fn check_bounded_method(
    label: &str,
    needle: &str,
    bad: Vec<(&'static str, JsValue)>,
    mut attempt: impl FnMut(JsValue) -> Result<JsValue, JsValue>,
    valid: f64,
) {
    let mut wrong = Vec::new();
    for (name, value) in bad {
        let r = attempt(value);
        let ok = match &r {
            Err(e) => match (e.dyn_ref::<js_sys::Error>(), e.as_string()) {
                (Some(_), _) if name == "5n" => true,
                (Some(err), _) => String::from(err.message()).contains(needle),
                // The crate's error funnel throws the message as a string.
                (None, Some(text)) => text.contains(needle),
                (None, None) => false,
            },
            Ok(_) => false,
        };
        if !ok {
            wrong.push(format!("{label}({name}) {}", outcome(&r)));
        }
    }
    let after = attempt(JsValue::from_f64(valid));
    if after.is_err() {
        wrong.push(format!("{label}({valid}) afterwards {}", outcome(&after)));
    }
    assert!(wrong.is_empty(), "{}", wrong.join("; "));
}

// Fixtures built through Rust calls whose signatures spec 009 does not change.

fn leaf() -> JsValue {
    CellId::from_lat_lng(&LatLng::from_degrees(40.0, -74.0)).into()
}

fn face0() -> JsValue {
    CellId::from_token("1").into()
}

fn small_polygon() -> JsValue {
    let points = [(10.0, 10.0), (10.0, 11.0), (11.0, 11.0), (11.0, 10.0)]
        .map(|(lat, lng)| LatLng::from_degrees(lat, lng).to_point());
    Polygon::new(vec![Loop::new(points.to_vec()).expect("a loop")]).into()
}

// ---------------------------------------------------------------------------
// WA-01, WA-02: levels
// ---------------------------------------------------------------------------

#[wasm_bindgen_test]
fn wa01_cell_id_parent_at_level() {
    check_level_method(
        "parentAtLevel",
        |v| call(&leaf(), "parentAtLevel", &[v]),
        10.0,
    );
}

#[wasm_bindgen_test]
fn wa01_cell_id_child_position() {
    check_level_method(
        "childPosition",
        |v| call(&leaf(), "childPosition", &[v]),
        10.0,
    );
}

#[wasm_bindgen_test]
fn wa01_cell_id_child_begin_at_level() {
    check_level_method(
        "childBeginAtLevel",
        |v| call(&face0(), "childBeginAtLevel", &[v]),
        10.0,
    );
}

#[wasm_bindgen_test]
fn wa01_cell_id_child_end_at_level() {
    check_level_method(
        "childEndAtLevel",
        |v| call(&face0(), "childEndAtLevel", &[v]),
        10.0,
    );
}

#[wasm_bindgen_test]
fn wa01_cell_id_vertex_neighbors() {
    check_level_method(
        "vertexNeighbors",
        |v| call(&leaf(), "vertexNeighbors", &[v]),
        10.0,
    );
}

#[wasm_bindgen_test]
fn wa01_cell_id_all_neighbors() {
    check_level_method(
        "allNeighbors",
        |v| call(&leaf(), "allNeighbors", &[v]),
        30.0,
    );
}

#[wasm_bindgen_test]
fn wa01_cell_id_begin() {
    check_level_method(
        "CellId.begin",
        |v| call(&class_of(&face0()), "begin", &[v]),
        10.0,
    );
}

#[wasm_bindgen_test]
fn wa01_cell_id_end() {
    check_level_method(
        "CellId.end",
        |v| call(&class_of(&face0()), "end", &[v]),
        10.0,
    );
}

#[wasm_bindgen_test]
fn wa01_cell_id_from_face_pos_level() {
    check_level_method(
        "CellId.fromFacePosLevel",
        |v| {
            call(
                &class_of(&face0()),
                "fromFacePosLevel",
                &[JsValue::from_f64(0.0), BigInt::from(0).into(), v],
            )
        },
        10.0,
    );
}

#[wasm_bindgen_test]
fn wa01_cell_average_area_for_level() {
    let cell: JsValue = Cell::from_cell_id(&CellId::from_token("1")).into();
    check_level_method(
        "Cell.averageAreaForLevel",
        |v| call(&class_of(&cell), "averageAreaForLevel", &[v]),
        10.0,
    );
}

#[wasm_bindgen_test]
fn wa01_cell_union_expand_at_level() {
    check_level_method(
        "CellUnion.expandAtLevel",
        |v| {
            let cu: JsValue = CellUnion::from_cell_ids(vec![CellId::from_lat_lng(
                &LatLng::from_degrees(1.0, 2.0),
            )])
            .into();
            call(&cu, "expandAtLevel", &[v])
        },
        10.0,
    );
}

#[wasm_bindgen_test]
fn wa01_region_coverer_set_min_level() {
    check_level_method(
        "RegionCoverer.setMinLevel",
        |v| call(&RegionCoverer::new().into(), "setMinLevel", &[v]),
        10.0,
    );
}

#[wasm_bindgen_test]
fn wa01_region_coverer_set_max_level() {
    check_level_method(
        "RegionCoverer.setMaxLevel",
        |v| call(&RegionCoverer::new().into(), "setMaxLevel", &[v]),
        10.0,
    );
}

#[wasm_bindgen_test]
fn wa01_term_indexer_set_min_level() {
    check_level_method(
        "RegionTermIndexer.setMinLevel",
        |v| call(&RegionTermIndexer::new().into(), "setMinLevel", &[v]),
        10.0,
    );
}

#[wasm_bindgen_test]
fn wa01_term_indexer_set_max_level() {
    check_level_method(
        "RegionTermIndexer.setMaxLevel",
        |v| call(&RegionTermIndexer::new().into(), "setMaxLevel", &[v]),
        10.0,
    );
}

#[wasm_bindgen_test]
fn wa01_polygon_snapped() {
    check_level_method(
        "Polygon.snapped",
        |v| call(&small_polygon(), "snapped", &[v]),
        20.0,
    );
}

#[wasm_bindgen_test]
fn wa01_snap_function_cell_id() {
    let f: JsValue = SnapFunction::identity(&Angle::from_degrees(0.0)).into();
    check_level_method(
        "SnapFunction.cellId",
        |v| call(&class_of(&f), "cellId", &[v]),
        10.0,
    );
}

#[wasm_bindgen_test]
fn wa02_integral_floats_are_levels() {
    let r = call(&leaf(), "parentAtLevel", &[JsValue::from_f64(5.0)]).expect("level 5.0");
    assert_eq!(
        Reflect::get(&r, &JsValue::from_str("level"))
            .unwrap()
            .as_f64(),
        Some(5.0)
    );
    for valid in [0.0, 30.0] {
        let r = call(&leaf(), "parentAtLevel", &[JsValue::from_f64(valid)]).expect("a valid level");
        assert_eq!(
            Reflect::get(&r, &JsValue::from_str("level"))
                .unwrap()
                .as_f64(),
            Some(valid)
        );
    }
}

// ---------------------------------------------------------------------------
// WA-03: faces
// ---------------------------------------------------------------------------

/// Checks that the call's result is a valid cell id (so a face in range still works).
fn assert_valid_id(r: Result<JsValue, JsValue>) -> Result<JsValue, JsValue> {
    let v = r?;
    let valid = call(&v, "isValid", &[])?;
    if valid.as_bool() == Some(true) {
        Ok(v)
    } else {
        Err(JsValue::from_str("an invalid cell id"))
    }
}

#[wasm_bindgen_test]
fn wa03_cell_id_from_face() {
    check_bounded_method(
        "CellId.fromFace",
        "face must be",
        bad_faces(),
        |v| assert_valid_id(call(&class_of(&face0()), "fromFace", &[v])),
        5.0,
    );
}

#[wasm_bindgen_test]
fn wa03_cell_id_from_face_pos_level() {
    check_bounded_method(
        "CellId.fromFacePosLevel",
        "face must be",
        bad_faces(),
        |v| {
            assert_valid_id(call(
                &class_of(&face0()),
                "fromFacePosLevel",
                &[v, BigInt::from(0).into(), JsValue::from_f64(10.0)],
            ))
        },
        5.0,
    );
}

#[wasm_bindgen_test]
fn wa03_cell_from_face() {
    let cell: JsValue = Cell::from_cell_id(&CellId::from_token("1")).into();
    check_bounded_method(
        "Cell.fromFace",
        "face must be",
        bad_faces(),
        |v| call(&class_of(&cell), "fromFace", &[v]),
        5.0,
    );
}

// ---------------------------------------------------------------------------
// WA-04, WA-05: level moduli, and the coverer's levels
// ---------------------------------------------------------------------------

/// The out-of-range values for a level modulus.
fn bad_level_mods() -> Vec<(&'static str, JsValue)> {
    vec![
        ("0", JsValue::from_f64(0.0)),
        ("4", JsValue::from_f64(4.0)),
        ("NaN", JsValue::from_f64(f64::NAN)),
        ("2.5", JsValue::from_f64(2.5)),
        ("5n", BigInt::from(5).into()),
    ]
}

#[wasm_bindgen_test]
fn wa04_region_coverer_set_level_mod() {
    check_bounded_method(
        "RegionCoverer.setLevelMod",
        "level_mod must be",
        bad_level_mods(),
        |v| call(&RegionCoverer::new().into(), "setLevelMod", &[v]),
        2.0,
    );
}

#[wasm_bindgen_test]
fn wa05_coverings_reject_min_level_above_max_level() {
    let point = LatLng::from_degrees(10.0, 10.0).to_point();
    let cap: JsValue = Cap::from_center_angle(&point, &Angle::from_degrees(1.0)).into();
    let rect: JsValue = Rect::from_lat_lng(&LatLng::from_degrees(10.0, 10.0)).into();
    let points = [(10.0, 10.0), (10.0, 11.0), (11.0, 11.0), (11.0, 10.0)]
        .map(|(lat, lng)| LatLng::from_degrees(lat, lng).to_point());
    let loop_: JsValue = Loop::new(points.to_vec()).expect("a loop").into();
    let polygon = small_polygon();
    let cu: JsValue =
        CellUnion::from_cell_ids(vec![CellId::from_lat_lng(&LatLng::from_degrees(1.0, 2.0))])
            .into();
    let methods = [
        ("coveringCap", &cap),
        ("coveringRect", &rect),
        ("coveringLoop", &loop_),
        ("coveringPolygon", &polygon),
        ("coveringCellUnion", &cu),
        ("interiorCoveringCap", &cap),
        ("interiorCoveringRect", &rect),
        ("interiorCoveringPolygon", &polygon),
        ("interiorCoveringLoop", &loop_),
        ("fastCoveringCap", &cap),
        ("fastCoveringRect", &rect),
        ("fastCoveringPolygon", &polygon),
    ];
    let mut wrong = Vec::new();
    for (method, region) in methods {
        let r = (|| {
            let c = call(
                &RegionCoverer::new().into(),
                "setMinLevel",
                &[JsValue::from_f64(10.0)],
            )?;
            let c = call(&c, "setMaxLevel", &[JsValue::from_f64(5.0)])?;
            call(&c, method, &[region.clone()])
        })();
        let ok = matches!(&r, Err(e) if e.as_string().is_some_and(|t| t.contains("min_level (10) must not exceed max_level (5)")));
        if !ok {
            wrong.push(format!("{method} {}", outcome(&r)));
        }
        // A coverer whose levels are in order still covers.
        let c = call(
            &RegionCoverer::new().into(),
            "setMinLevel",
            &[JsValue::from_f64(5.0)],
        )
        .unwrap();
        let c = call(&c, "setMaxLevel", &[JsValue::from_f64(10.0)]).unwrap();
        if let Err(e) = call(&c, method, &[region.clone()]) {
            wrong.push(format!("{method} with levels in order throws {e:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("; "));
}

// ---------------------------------------------------------------------------
// WA-06: cell vertex and edge indices
// ---------------------------------------------------------------------------

/// The out-of-range values for a vertex or edge index.
fn bad_indices() -> Vec<(&'static str, JsValue)> {
    vec![
        ("4", JsValue::from_f64(4.0)),
        ("-1", JsValue::from_f64(-1.0)),
        ("NaN", JsValue::from_f64(f64::NAN)),
        ("3.5", JsValue::from_f64(3.5)),
        ("5n", BigInt::from(5).into()),
    ]
}

#[wasm_bindgen_test]
fn wa06_cell_vertex() {
    let cell: JsValue = Cell::from_cell_id(&CellId::from_token("1")).into();
    check_bounded_method(
        "Cell.vertex",
        "cell vertex index must be",
        bad_indices(),
        |v| call(&cell, "vertex", &[v]),
        3.0,
    );
}

#[wasm_bindgen_test]
fn wa06_cell_edge() {
    let cell: JsValue = Cell::from_cell_id(&CellId::from_token("1")).into();
    check_bounded_method(
        "Cell.edge",
        "cell edge index must be",
        bad_indices(),
        |v| call(&cell, "edge", &[v]),
        3.0,
    );
}
