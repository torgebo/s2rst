// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The abstract region (spec 012 data-model §3; research R0, R5): a `Region` whose every answer
//! is any value, one per input and region, so that a statement proven over it holds for every
//! implementor. The region adaptors' proofs take their members from here: a preservation law
//! ("if every member satisfies law L, the adaptor does") assumes L of each member's answers, read
//! through the same methods the adaptor calls, and the tables make the two reads one value.
//!
//! Each region has an `id` (0 to 3); its answers are keyed by the id and the input's bits:
//! - `contains_cell`, `intersects_cell`: one boolean per cell id;
//! - `contains_point`: one boolean per point;
//! - `rect_bound`: one valid rectangle (`rect_spec::valid`) per region;
//! - `cap_bound`: one cap with a centre in the unit box per region;
//! - `cell_union_bound`: empty (the adaptors never read their members' coverings).
//!
//! The tables are `static Mutex` tables of ten slots, as `stubs`'; `reset` clears them.

use s2rst::s2::{Cap, Cell, CellId, Point, Rect, Region};
use std::sync::Mutex;

use crate::stubs::{SLOTS, Table, any_box_point, any_valid_rect, bits3};

static CONTAINS_CELL: Mutex<Table<2>> = Mutex::new(Table::new());
static INTERSECTS_CELL: Mutex<Table<2>> = Mutex::new(Table::new());
static CONTAINS_POINT: Mutex<Table<4>> = Mutex::new(Table::new());
static RECTS: Mutex<([[u64; 4]; 4], [bool; 4])> = Mutex::new(([[0; 4]; 4], [false; 4]));
static CAPS: Mutex<([[u64; 4]; 4], [bool; 4])> = Mutex::new(([[0; 4]; 4], [false; 4]));

/// Clears every table: a harness calls it first.
pub fn reset() {
    CONTAINS_CELL.lock().unwrap().clear();
    INTERSECTS_CELL.lock().unwrap().clear();
    CONTAINS_POINT.lock().unwrap().clear();
    *RECTS.lock().unwrap() = ([[0; 4]; 4], [false; 4]);
    *CAPS.lock().unwrap() = ([[0; 4]; 4], [false; 4]);
}

/// A region with any answers, one per input; `id` (0 to 3) tells regions apart.
#[derive(Clone, Copy, Debug)]
pub struct AnyRegion {
    pub id: u8,
}

fn any_bool() -> u64 {
    u64::from(kani::any::<bool>())
}

impl Region for AnyRegion {
    fn cap_bound(&self) -> Cap {
        let k = usize::from(self.id & 3);
        let mut t = CAPS.lock().unwrap();
        if !t.1[k] {
            let p = any_box_point();
            let l: f64 = kani::any();
            kani::assume((0.0..=4.0).contains(&l));
            t.0[k] = [
                p.0.x.to_bits(),
                p.0.y.to_bits(),
                p.0.z.to_bits(),
                l.to_bits(),
            ];
            t.1[k] = true;
        }
        let v = t.0[k];
        Cap::from_center_chord_angle(
            Point(s2rst::r3::Vector {
                x: f64::from_bits(v[0]),
                y: f64::from_bits(v[1]),
                z: f64::from_bits(v[2]),
            }),
            s2rst::s1::ChordAngle::from_length2(f64::from_bits(v[3])),
        )
    }

    fn rect_bound(&self) -> Rect {
        let k = usize::from(self.id & 3);
        let mut t = RECTS.lock().unwrap();
        if !t.1[k] {
            let r = any_valid_rect();
            t.0[k] = [
                r.lat.lo.to_bits(),
                r.lat.hi.to_bits(),
                r.lng.lo.to_bits(),
                r.lng.hi.to_bits(),
            ];
            t.1[k] = true;
        }
        let v = t.0[k];
        Rect {
            lat: s2rst::r1::Interval {
                lo: f64::from_bits(v[0]),
                hi: f64::from_bits(v[1]),
            },
            lng: s2rst::s1::Interval {
                lo: f64::from_bits(v[2]),
                hi: f64::from_bits(v[3]),
            },
        }
    }

    fn cell_union_bound(&self) -> Vec<CellId> {
        Vec::new()
    }

    fn contains_cell(&self, cell: &Cell) -> bool {
        CONTAINS_CELL
            .lock()
            .unwrap()
            .get([u64::from(self.id), cell.id().id()], any_bool)
            == 1
    }

    fn intersects_cell(&self, cell: &Cell) -> bool {
        INTERSECTS_CELL
            .lock()
            .unwrap()
            .get([u64::from(self.id), cell.id().id()], any_bool)
            == 1
    }

    fn contains_point(&self, p: &Point) -> bool {
        let b = bits3(*p);
        CONTAINS_POINT
            .lock()
            .unwrap()
            .get([u64::from(self.id), b[0], b[1], b[2]], any_bool)
            == 1
    }
}

/// `n` abstract regions, ids 0 to `n − 1`, boxed as an adaptor takes them.
pub fn members<const N: usize>() -> Vec<Box<dyn Region>> {
    let mut v: Vec<Box<dyn Region>> = Vec::new();
    for k in 0..N {
        v.push(Box::new(AnyRegion { id: k as u8 }));
    }
    v
}

const _: () = assert!(SLOTS >= 4);
