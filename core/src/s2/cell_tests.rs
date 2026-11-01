// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original regression tests for [`super::Cell`] fixes found during the formal verification
//! of the value types (specs/004-formal-verify-cell-interval). Written for this crate, not
//! ported from upstream S2.

#[cfg(feature = "serde")]
mod serde_tests {
    use super::super::*;
    use serde_json::Value;

    fn level5() -> Cell {
        Cell::from_cell_id(
            CellId::from_face(Face::F0)
                .child_begin_at_level(Level::new(5))
                .next()
                .next(),
        )
    }

    /// Every field of a cell, the bounds as bit patterns.
    fn fields(c: Cell) -> (CellId, Face, Level, u8, [u64; 4]) {
        let b = c.bound_uv();
        let bits = [b.x.lo, b.x.hi, b.y.lo, b.y.hi].map(f64::to_bits);
        (c.id(), c.face(), c.level(), c.orientation(), bits)
    }

    fn deserialize(v: Value) -> Result<Cell, serde_json::Error> {
        serde_json::from_value(v)
    }

    /// FR-018 (K2): a cell whose level was edited no longer deserialises. Before the fix, the
    /// level-5 cell with `"level": 12` came back reporting level 12 while its id has level 5,
    /// and still compared equal to the original.
    #[test]
    fn an_edited_level_is_rejected() {
        let json = serde_json::to_string(&level5()).unwrap();
        let edited = json.replacen("\"level\":5", "\"level\":12", 1);
        assert_ne!(json, edited);
        assert!(serde_json::from_str::<Cell>(&edited).is_err());
    }

    /// FR-018: a cell whose id is not a valid cell does not deserialise: the null id and an id
    /// with face 6 (on which `Cell::from_cell_id` panics) are rejected, not built.
    #[test]
    fn an_invalid_id_is_rejected() {
        for id in [0u64, (6 << 61) | (1 << 60)] {
            let mut v = serde_json::to_value(level5()).unwrap();
            v["id"] = Value::from(id);
            assert!(deserialize(v).is_err(), "id {id:#x}");
        }
    }

    /// FR-018: a face or an orientation that differs from the id's is rejected.
    #[test]
    fn an_inconsistent_face_or_orientation_is_rejected() {
        let mut v = serde_json::to_value(level5()).unwrap();
        v["face"] = Value::from("F4");
        assert!(deserialize(v).is_err());
        let c = level5();
        let mut v = serde_json::to_value(c).unwrap();
        v["orientation"] = Value::from((c.orientation() + 1) % 4);
        assert!(deserialize(v).is_err());
    }

    /// FR-018: a stored bound within one ulp of the id's is accepted and replaced by the exact
    /// value; two ulps away is rejected. (`Value` carries the `f64` exactly, with no text
    /// parsing in between.)
    #[test]
    fn a_bound_one_ulp_off_is_accepted_and_two_are_not() {
        let c = level5();
        let lo = c.bound_uv().x.lo;
        let off = |k: u64| {
            let mut v = serde_json::to_value(c).unwrap();
            v["uv"]["x"]["lo"] = Value::from(f64::from_bits(lo.to_bits() + k));
            deserialize(v)
        };
        let back = off(1).expect("one ulp off is accepted");
        assert_eq!(fields(back), fields(c));
        assert!(off(2).is_err());
    }

    /// Every cell to `max_level` on all faces, then `random` pseudo-random deeper cells, through
    /// JSON text and back: each comes back with exactly its own fields, bit for bit. Before
    /// FR-018, `serde_json`'s default float parsing moved a bound by one ulp in about a quarter
    /// of the cells (spec 004 research R6).
    fn round_trip(max_level: u8, random: usize) {
        let check = |c: Cell| {
            let back: Cell = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
            assert_eq!(fields(back), fields(c), "{:?}", c.id());
        };
        for face in 0..6u8 {
            for level in 0..=max_level {
                let mut id =
                    CellId::from_face(Face::from_u8(face)).child_begin_at_level(Level::new(level));
                let end =
                    CellId::from_face(Face::from_u8(face)).child_end_at_level(Level::new(level));
                while id != end {
                    check(Cell::from_cell_id(id));
                    id = id.next();
                }
            }
        }
        let mut s: u64 = 0x2545_F491_4F6C_DD1D;
        for _ in 0..random {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            let leaf = CellId::from((s % (6 << 61)) | 1);
            check(Cell::from_cell_id(
                leaf.parent_at_level(Level::new((s % 31) as u8)),
            ));
        }
    }

    /// SC-009, the default size for `make test`: levels 0–5 and 100,000 random cells.
    #[test]
    fn consistent_cells_round_trip_bit_for_bit() {
        round_trip(5, 100_000);
    }

    /// SC-009 in full: levels 0–7 and 2,000,000 random cells. Run it with
    /// `cargo test -p s2rst --features serde --release -- --ignored round_trip_in_full`.
    #[test]
    #[ignore = "SC-009's full size: 34 s in a debug build, 2 s in release"]
    fn consistent_cells_round_trip_in_full() {
        round_trip(7, 2_000_000);
    }

    /// The serialised form of face cell 0, before spec 009 (CO-04).
    const FACE0_JSON: &str = r#"{"face":"F0","level":0,"orientation":0,"id":1152921504606846976,"uv":{"x":{"lo":-1.0,"hi":1.0},"y":{"lo":-1.0,"hi":1.0}}}"#;

    /// CO-04 (spec 009): a cell keeps its serialised form, its level and face included, and
    /// reads back.
    #[test]
    fn face_cell_form_is_unchanged() {
        let c = Cell::from_cell_id(CellId::from_face(Face::F0));
        assert_eq!(serde_json::to_string(&c).unwrap(), FACE0_JSON);
        assert_eq!(
            fields(serde_json::from_str::<Cell>(FACE0_JSON).unwrap()),
            fields(c)
        );
    }
}

/// `CellVertex` and `CellEdge` as bounded types (spec 009).
mod bounded_index_tests {
    use super::super::*;
    use crate::s2::BoundedKind;

    #[test]
    fn cell_vertex_try_from_accepts_exactly_zero_to_three() {
        for k in -2i64..=6 {
            match CellVertex::try_from(k) {
                Ok(v) => {
                    assert!((0..=3).contains(&k));
                    assert_eq!(v.index() as i64, k);
                }
                Err(e) => {
                    assert!(!(0..=3).contains(&k));
                    assert_eq!(e.kind(), BoundedKind::CellVertex);
                    assert_eq!(e.value(), k);
                }
            }
        }
        assert!(CellVertex::try_from(usize::MAX).is_err());
        assert_eq!(CellVertex::try_from(2usize), Ok(CellVertex::UpperRight));
    }

    #[test]
    fn cell_vertex_next_and_prev_wrap() {
        for (k, v) in CellVertex::ALL.into_iter().enumerate() {
            assert_eq!(v.index(), k);
            assert_eq!(v.next(), CellVertex::ALL[(k + 1) % 4]);
            assert_eq!(v.prev(), CellVertex::ALL[(k + 3) % 4]);
        }
        assert_eq!(CellVertex::default(), CellVertex::LowerLeft);
    }

    /// The vertices are those `vertex_raw` gave for the indices 0..=3 before spec 009: the
    /// corners of the u-v rectangle, counter-clockwise from (u_lo, v_lo).
    #[test]
    fn cell_vertices_are_the_uv_corners_in_order() {
        let cell =
            Cell::from_cell_id(CellId::from_face(Face::F2).child_begin_at_level(Level::new(3)));
        let uv = cell.bound_uv().vertices();
        for v in CellVertex::ALL {
            let p = face_uv_to_xyz(cell.face(), uv[v.index()].x, uv[v.index()].y);
            assert_eq!(cell.vertex_raw(v).vector(), p);
        }
    }

    #[test]
    fn cell_edge_try_from_accepts_exactly_zero_to_three() {
        for k in -2i64..=6 {
            match CellEdge::try_from(k) {
                Ok(e) => assert_eq!(CellEdge::ALL[usize::try_from(k).unwrap()], e),
                Err(e) => {
                    assert!(!(0..=3).contains(&k));
                    assert_eq!(e.kind(), BoundedKind::CellEdge);
                }
            }
        }
    }
}

/// Spec 011 FR-017 (K1): the polar face cells' rectangle bounds are upstream's bit for bit.
/// Upstream computes the latitude of a polar face's corners, `asin(sqrt(1./3))`; the library
/// wrote Go's 15-digit literal, 2 ulps below it, so the bounds of faces 2 and 5 (and every
/// union's bound that contains them) were 2 ulps wider than upstream's.
mod pole_min_lat_tests {
    use super::super::*;

    #[test]
    fn pole_min_lat_is_upstreams_computed_value() {
        assert_eq!(
            POLE_MIN_LAT.to_bits(),
            (1.0f64 / 3.0).sqrt().asin().to_bits()
        );
    }

    #[test]
    fn polar_face_rect_bounds_are_upstreams() {
        // `S2Cell(S2CellId::FromFace(f)).GetRectBound()` on upstream v0.14.0-40 (spec 011
        // probe, section CF): the latitude bound nearest the equator.
        let north = Cell::from_cell_id(CellId::from_face(Face::F2)).rect_bound();
        let south = Cell::from_cell_id(CellId::from_face(Face::F5)).rect_bound();
        assert_eq!(north.lat.lo.to_bits(), 0x3fe3_b202_8082_e8d0);
        assert_eq!(south.lat.hi.to_bits(), 0xbfe3_b202_8082_e8d0);
        assert_eq!(north.lat.hi, FRAC_PI_2);
        assert_eq!(south.lat.lo, -FRAC_PI_2);
    }
}

/// Spec 011 K4 (FR-019): `contains_point` panics in debug builds exactly where the face
/// projection's check fails, as upstream's `ABSL_DCHECK`.
mod contains_point_panic_tests {
    use super::super::*;

    fn face0() -> Cell {
        Cell::from_cell_id(CellId::from_face(Face::F0))
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "does not belong to face")]
    fn nan_on_the_face_axis() {
        let _ = face0().contains_point(Point(crate::r3::Vector {
            x: f64::NAN,
            y: 0.0,
            z: 0.0,
        }));
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "does not belong to face")]
    fn infinity_off_the_axis_on_the_faces_side() {
        let _ = face0().contains_point(Point(crate::r3::Vector {
            x: 1.0,
            y: f64::INFINITY,
            z: 0.0,
        }));
    }

    #[test]
    fn infinity_off_the_axis_on_the_other_side_returns() {
        assert!(!face0().contains_point(Point(crate::r3::Vector {
            x: -1.0,
            y: f64::INFINITY,
            z: 0.0,
        })));
    }
}

/// Spec 012 FR-016 (U-017): `contains_point` accepts points up to `(5/3)·ε` outside the cell in
/// u-v, and the bounds cover the cell's own points only, as upstream's: a point of the margin can
/// lie outside them. These pin the survey's two inputs, so a change to either side is seen.
mod margin_tests {
    use super::super::*;
    use crate::r3::Vector;

    fn bits(x: u64, y: u64, z: u64) -> Point {
        Point(Vector {
            x: f64::from_bits(x),
            y: f64::from_bits(y),
            z: f64::from_bits(z),
        })
    }

    #[test]
    fn bounds_miss_the_margin() {
        // Face 0, level 1, u-v bound [−1, 0]²; the point is u = v = ε beyond the corner (0, 0).
        let c = Cell::from_cell_id(CellId(0x0400_0000_0000_0000));
        let p = Point(Vector {
            x: 1.0,
            y: f64::EPSILON,
            z: f64::EPSILON,
        });
        assert!(c.contains_point(p));
        assert!(!c.cap_bound().contains_point(p));
        // An edge midpoint moved out by ε.
        let c = Cell::from_cell_id(CellId(0x4340_0000_0000_0000));
        let p = bits(
            0xbcad_89d8_9d89_d89e,
            0x3fd8_9d89_d89d_89d5,
            0x3fed_89d8_9d89_d89e,
        );
        assert!(c.contains_point(p));
        assert!(!c.rect_bound().contains_point(p));
    }

    #[test]
    fn bounds_hold_the_cells_own_corners() {
        for k in CellVertex::ALL {
            let c = Cell::from_cell_id(CellId(0x0400_0000_0000_0000));
            let v = c.vertex(k);
            assert!(c.cap_bound().contains_point(v) && c.rect_bound().contains_point(v));
        }
    }
}
