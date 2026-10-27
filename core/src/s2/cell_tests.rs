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
        Cell::from_cell_id(CellId::from_face(0).child_begin_at_level(5).next().next())
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
                let mut id = CellId::from_face(face).child_begin_at_level(level);
                let end = CellId::from_face(face).child_end_at_level(level);
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
            check(Cell::from_cell_id(leaf.parent_at_level((s % 31) as u8)));
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
}
