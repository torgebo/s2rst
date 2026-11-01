// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Spec 013 planning probe (research R0): does an order contract for `atan2` (and `sqrt`) prove
//! `Cell`'s bounding-rectangle law, and what do `Rect`'s bounding-cap law and P-PTDST-02 cost
//! under it? Included in the verification crate through a temporary `#[path]` module, as spec
//! 012's R0; run with `VERIFY_PLAYBACK=0 VERIFY_RUN_DIR=target/run-r013 ./run-verify.sh
//! --harness r0::r0_`.

use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI, SQRT_2};
use std::sync::Mutex;

use s2rst::s2::coords::face_uv_to_xyz;
use s2rst::s2::{Level, Point, Region};

use crate::stubs::{SLOTS, atan2_contract, sqrt_contract};
use crate::value_domain::any_cell;

/// A log of the calls a contract has answered: arguments and result bits.
struct Log {
    e: [[u64; 3]; SLOTS],
    used: usize,
}

impl Log {
    const fn new() -> Self {
        Log {
            e: [[0; 3]; SLOTS],
            used: 0,
        }
    }
}

static ATAN2_LOG: Mutex<Log> = Mutex::new(Log::new());
static SQRT_LOG: Mutex<Log> = Mutex::new(Log::new());

fn reset() {
    ATAN2_LOG.lock().unwrap().used = 0;
    SQRT_LOG.lock().unwrap().used = 0;
    crate::stubs::reset();
}

/// The closed quadrant of `(y, x)` by sign bits: bit 0 `x` negative, bit 1 `y` negative.
fn quadrant(y: f64, x: f64) -> u8 {
    u8::from(x.is_sign_negative()) | (u8::from(y.is_sign_negative()) << 1)
}

/// Whether `(y1, x1)`'s angle is at most `(y2, x2)`'s by dominance in quadrant `q` (both in it):
/// in the first quadrant the angle grows with `y` and shrinks with `x`, and the mirror images.
fn below(q: u8, y1: f64, x1: f64, y2: f64, x2: f64) -> bool {
    match q {
        0 => y1 <= y2 && x1 >= x2,
        1 => y1 >= y2 && x1 >= x2,
        2 => y1 <= y2 && x1 <= x2,
        _ => y1 >= y2 && x1 <= x2,
    }
}

/// The anchors: exact `atan2` values that bounds use, as virtual earlier calls.
fn anchors() -> [(f64, f64, f64); 6] {
    let p = POLE_ANCHOR;
    [
        (1.0, 1.0, FRAC_PI_4),
        (1.0, -1.0, 3.0 * FRAC_PI_4),
        (-1.0, 1.0, -FRAC_PI_4),
        (-1.0, -1.0, -3.0 * FRAC_PI_4),
        (1.0, SQRT_2, p),
        (-1.0, SQRT_2, -p),
    ]
}

/// `atan2(1, √2)`, checked natively (`r0_anchor_values` in the probe's tests).
const POLE_ANCHOR: f64 = 0.615_479_708_670_387_3;

/// `atan2`'s order contract: the range contract, one value per argument pair, the exact values
/// on the axes (C11 F.10.1.4), a result in the half-plane `|r| ≥ π/2` for a negative `x`, and
/// results ordered by dominance against every earlier call and anchor in the same quadrant.
pub fn atan2_order(y: f64, x: f64) -> f64 {
    if y.is_nan() || x.is_nan() {
        return f64::NAN;
    }
    let mut log = ATAN2_LOG.lock().unwrap();
    for i in 0..SLOTS {
        if i < log.used && log.e[i][0] == y.to_bits() && log.e[i][1] == x.to_bits() {
            return f64::from_bits(log.e[i][2]);
        }
    }
    let r = atan2_contract(y, x);
    if x.is_sign_negative() {
        kani::assume(r.abs() >= FRAC_PI_2);
    }
    if x == 0.0 && y != 0.0 {
        kani::assume(r == if y > 0.0 { FRAC_PI_2 } else { -FRAC_PI_2 });
    }
    if y == 0.0 {
        kani::assume(if x.is_sign_positive() {
            r == y
        } else if y.is_sign_positive() {
            r == PI
        } else {
            r == -PI
        });
    }
    let q = quadrant(y, x);
    for (ay, ax, ar) in anchors() {
        if quadrant(ay, ax) == q {
            if below(q, y, x, ay, ax) {
                kani::assume(r <= ar);
            }
            if below(q, ay, ax, y, x) {
                kani::assume(ar <= r);
            }
        }
    }
    for i in 0..SLOTS {
        if i < log.used {
            let (y2, x2, r2) = (
                f64::from_bits(log.e[i][0]),
                f64::from_bits(log.e[i][1]),
                f64::from_bits(log.e[i][2]),
            );
            if quadrant(y2, x2) == q {
                if below(q, y, x, y2, x2) {
                    kani::assume(r <= r2);
                }
                if below(q, y2, x2, y, x) {
                    kani::assume(r2 <= r);
                }
            }
        }
    }
    let n = log.used;
    if n < SLOTS {
        log.e[n] = [y.to_bits(), x.to_bits(), r.to_bits()];
        log.used = n + 1;
    }
    r
}

/// `sqrt`'s order contract: the range contract, one value per argument, results ordered as
/// their arguments (IEEE-754's `sqrt` is correctly rounded, so monotone), and `sqrt(2) = √2`.
pub fn sqrt_order(a: f64) -> f64 {
    let mut log = SQRT_LOG.lock().unwrap();
    for i in 0..SLOTS {
        if i < log.used && log.e[i][0] == a.to_bits() {
            return f64::from_bits(log.e[i][2]);
        }
    }
    let r = sqrt_contract(a);
    if !a.is_nan() && a >= 0.0 {
        if a <= 2.0 {
            kani::assume(r <= SQRT_2);
        }
        if a >= 2.0 {
            kani::assume(r >= SQRT_2);
        }
        for i in 0..SLOTS {
            if i < log.used {
                let (a2, r2) = (f64::from_bits(log.e[i][0]), f64::from_bits(log.e[i][2]));
                if a2 <= a {
                    kani::assume(r2 <= r);
                }
                if a <= a2 {
                    kani::assume(r <= r2);
                }
            }
        }
    }
    let n = log.used;
    if n < SLOTS {
        log.e[n] = [a.to_bits(), 0, r.to_bits()];
        log.used = n + 1;
    }
    r
}

/// `ij_level_to_bound_uv`, abstracted as spec 012's `any_uv_bound`, with the quadrant premise
/// (K2): below level 0 neither interval crosses 0, and no bound is `−0`; at level 0 the face.
pub fn quadrant_uv_bound(_i: i32, _j: i32, level: Level) -> s2rst::r2::Rect {
    let b: [f64; 4] = kani::any();
    if level.as_u8() == 0 {
        kani::assume(b[0] == -1.0 && b[1] == 1.0 && b[2] == -1.0 && b[3] == 1.0);
    } else {
        kani::assume(b[0] >= -1.0 && b[0] <= b[1] && b[1] <= 1.0);
        kani::assume(b[2] >= -1.0 && b[2] <= b[3] && b[3] <= 1.0);
        kani::assume(b[0] >= 0.0 || b[1] <= 0.0);
        kani::assume(b[2] >= 0.0 || b[3] <= 0.0);
        for v in b {
            kani::assume(!(v == 0.0 && v.is_sign_negative()));
        }
    }
    s2rst::r2::Rect {
        x: s2rst::r1::Interval { lo: b[0], hi: b[1] },
        y: s2rst::r1::Interval { lo: b[2], hi: b[3] },
    }
}

macro_rules! creg02 {
    ($name:ident, $lv:expr) => {
        /// P-CREG-02 for the cell's points as directions, levels `$lv`.
        #[kani::proof]
        #[kani::unwind(11)]
        #[kani::stub(s2rst::s2::ij_level_to_bound_uv, quadrant_uv_bound)]
        #[kani::stub(f64::atan2, atan2_order)]
        #[kani::stub(f64::sqrt, sqrt_order)]
        fn $name() {
            reset();
            let (c, _) = any_cell();
            kani::assume(($lv)(c.level().as_u8()));
            let uv = c.bound_uv();
            let (u, v): (f64, f64) = (kani::any(), kani::any());
            kani::assume(uv.x.lo <= u && u <= uv.x.hi && uv.y.lo <= v && v <= uv.y.hi);
            let p = Point(face_uv_to_xyz(c.face(), u, v));
            assert!(c.rect_bound().contains_point(p));
        }
    };
}
creg02!(r0_creg02_dir_l0, |l: u8| l == 0);
creg02!(r0_creg02_dir, |l: u8| l >= 1);

/// P-CREG-02 for the normalised point.
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, quadrant_uv_bound)]
#[kani::stub(f64::atan2, atan2_order)]
#[kani::stub(f64::sqrt, sqrt_order)]
fn r0_creg02_norm() {
    reset();
    let (c, _) = any_cell();
    kani::assume(c.level().as_u8() >= 1);
    let uv = c.bound_uv();
    let (u, v): (f64, f64) = (kani::any(), kani::any());
    kani::assume(uv.x.lo <= u && u <= uv.x.hi && uv.y.lo <= v && v <= uv.y.hi);
    let p = Point(face_uv_to_xyz(c.face(), u, v).normalize());
    assert!(c.rect_bound().contains_point(p));
}

/// The quadrant premise of `quadrant_uv_bound`, on the real `ij_level_to_bound_uv`.
#[kani::proof]
fn r0_uv_quadrant() {
    let (i, j, l): (i32, i32, u8) = (kani::any(), kani::any(), kani::any());
    kani::assume((0..1 << 30).contains(&i) && (0..1 << 30).contains(&j) && (1..=30).contains(&l));
    let b = s2rst::s2::ij_level_to_bound_uv(i, j, Level::new(l));
    for (lo, hi) in [(b.x.lo, b.x.hi), (b.y.lo, b.y.hi)] {
        assert!(-1.0 <= lo && lo <= hi && hi <= 1.0);
        assert!(lo >= 0.0 || hi <= 0.0);
        assert!(!(lo == 0.0 && lo.is_sign_negative()) && !(hi == 0.0 && hi.is_sign_negative()));
    }
}

/// P-PTDST-02 under a pure `atan2`: the distance is symmetric bit for bit.
#[kani::proof]
#[kani::stub(f64::atan2, crate::stubs::pure_atan2)]
#[kani::stub(f64::sqrt, crate::stubs::pure_sqrt)]
fn r0_ptdst02() {
    crate::stubs::reset();
    let (p, q) = (
        crate::value_domain::any_box_point(),
        crate::value_domain::any_box_point(),
    );
    assert!(p.distance(q).radians().to_bits() == q.distance(p).radians().to_bits());
}

/// P-LREG-03 under the order contract and the `sin_cos` contract.
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(f64::atan2, atan2_order)]
#[kani::stub(f64::sqrt, sqrt_order)]
#[kani::stub(f64::sin_cos, crate::stubs::pure_sin_cos)]
#[kani::stub(f64::sin, crate::stubs::pure_sin)]
#[kani::stub(f64::cos, crate::stubs::pure_cos)]
#[kani::stub(f64::asin, crate::stubs::pure_asin)]
fn r0_lreg03() {
    reset();
    let r = crate::stubs::any_valid_rect();
    let p = crate::value_domain::any_plain_triple();
    if r.contains_point(p) {
        assert!(r.cap_bound().contains_point(p));
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn r0_anchor_values() {
        use std::f64::consts::{FRAC_PI_4, SQRT_2};
        assert_eq!(1.0f64.atan2(SQRT_2), super::POLE_ANCHOR);
        assert_eq!(1.0f64.atan2(1.0), FRAC_PI_4);
        assert_eq!(1.0f64.atan2(-1.0), 3.0 * FRAC_PI_4);
        assert_eq!((-1.0f64).atan2(-1.0), -3.0 * FRAC_PI_4);
        assert_eq!(2.0f64.sqrt(), SQRT_2);
    }
}

macro_rules! creg02_split {
    ($($name:ident = $face:literal, $lat:literal);* $(;)?) => { $(
        /// P-CREG-02 for directions below level 0, one face, latitude or longitude.
        #[kani::proof]
        #[kani::unwind(11)]
        #[kani::stub(s2rst::s2::ij_level_to_bound_uv, quadrant_uv_bound)]
        #[kani::stub(f64::atan2, atan2_order)]
        #[kani::stub(f64::sqrt, sqrt_order)]
        fn $name() {
            reset();
            let (c, _) = any_cell();
            kani::assume(c.level().as_u8() >= 1 && c.face().as_u8() == $face);
            let uv = c.bound_uv();
            let (u, v): (f64, f64) = (kani::any(), kani::any());
            kani::assume(uv.x.lo <= u && u <= uv.x.hi && uv.y.lo <= v && v <= uv.y.hi);
            let p = Point(face_uv_to_xyz(c.face(), u, v));
            let b = c.rect_bound();
            let ll = s2rst::s2::LatLng::from_point(p);
            if $lat {
                assert!(b.lat.contains(ll.lat.radians()));
            } else {
                assert!(b.lng.contains(ll.lng.radians()));
            }
        }
    )* };
}
creg02_split!(
    r0_creg02__f0_lat = 0, true; r0_creg02__f0_lng = 0, false;
    r0_creg02__f2_lat = 2, true; r0_creg02__f2_lng = 2, false;
);

// ---- R0b: latitude through a monotone sum of squares ------------------------------------------

static SUMSQ_LOG: Mutex<Log> = Mutex::new(Log::new());

/// `x·x + y·y` as the library computes it, abstracted: one value per pair, monotone in `|x|`
/// and `|y|` (lemma `r0_sumsq_monotone`), with its exact values at the anchors 0, 1, 2.
fn sumsq(x: f64, y: f64) -> f64 {
    let mut log = SUMSQ_LOG.lock().unwrap();
    for i in 0..SLOTS {
        if i < log.used && log.e[i][0] == x.to_bits() && log.e[i][1] == y.to_bits() {
            return f64::from_bits(log.e[i][2]);
        }
    }
    let s: f64 = kani::any();
    kani::assume(s >= 0.0 && s <= 2.0);
    let (ax, ay) = (x.abs(), y.abs());
    for (bx, by, bs) in [(0.0, 0.0, 0.0), (1.0, 0.0, 1.0), (0.0, 1.0, 1.0), (1.0, 1.0, 2.0)] {
        if ax <= bx && ay <= by {
            kani::assume(s <= bs);
        }
        if bx <= ax && by <= ay {
            kani::assume(bs <= s);
        }
    }
    for i in 0..SLOTS {
        if i < log.used {
            let (bx, by, bs) = (
                f64::from_bits(log.e[i][0]).abs(),
                f64::from_bits(log.e[i][1]).abs(),
                f64::from_bits(log.e[i][2]),
            );
            if ax <= bx && ay <= by {
                kani::assume(s <= bs);
            }
            if bx <= ax && by <= ay {
                kani::assume(bs <= s);
            }
        }
    }
    let n = log.used;
    if n < SLOTS {
        log.e[n] = [x.to_bits(), y.to_bits(), s.to_bits()];
        log.used = n + 1;
    }
    s
}

/// `LatLng::latitude`, restated with [`sumsq`] (the library: `(p.z + 0.0).atan2((p.x * p.x +
/// p.y * p.y).sqrt())`), for coordinates in [−1, 1].
pub fn latitude_lemma(p: Point) -> s2rst::s1::Angle {
    s2rst::s1::Angle::from_radians((p.0.z + 0.0).atan2(sumsq(p.0.x, p.0.y).sqrt()))
}

/// The lemma: the library's sum of squares is monotone in `|x|` and `|y|` on [−1, 1], and exact
/// at the anchors.
#[kani::proof]
fn r0_sumsq_monotone() {
    let (x1, y1, x2, y2): (f64, f64, f64, f64) = (kani::any(), kani::any(), kani::any(), kani::any());
    for v in [x1, y1, x2, y2] {
        kani::assume(v.abs() <= 1.0);
    }
    kani::assume(x1.abs() <= x2.abs() && y1.abs() <= y2.abs());
    assert!(x1 * x1 + y1 * y1 <= x2 * x2 + y2 * y2);
}

/// The lemma's square alone: `|a| ≤ |b| ⇒ a·a ≤ b·b`.
#[kani::proof]
fn r0_square_monotone() {
    let (a, b): (f64, f64) = (kani::any(), kani::any());
    kani::assume(a.abs() <= b.abs() && b.abs() <= 1.0);
    assert!(a * a <= b * b);
}

macro_rules! creg02_lat {
    ($($name:ident = $face:literal);* $(;)?) => { $(
        /// P-CREG-02's latitude below level 0, one face, through `latitude_lemma`.
        #[kani::proof]
        #[kani::unwind(11)]
        #[kani::stub(s2rst::s2::ij_level_to_bound_uv, quadrant_uv_bound)]
        #[kani::stub(s2rst::s2::LatLng::latitude, latitude_lemma)]
        #[kani::stub(f64::atan2, atan2_order)]
        #[kani::stub(f64::sqrt, sqrt_order)]
        fn $name() {
            reset();
            SUMSQ_LOG.lock().unwrap().used = 0;
            let (c, _) = any_cell();
            kani::assume(c.level().as_u8() >= 1 && c.face().as_u8() == $face);
            let uv = c.bound_uv();
            let (u, v): (f64, f64) = (kani::any(), kani::any());
            kani::assume(uv.x.lo <= u && u <= uv.x.hi && uv.y.lo <= v && v <= uv.y.hi);
            let p = Point(face_uv_to_xyz(c.face(), u, v));
            let b = c.rect_bound();
            assert!(b.lat.contains(s2rst::s2::LatLng::latitude(p).radians()));
        }
    )* };
}
creg02_lat!(r0_creg02l__f0 = 0; r0_creg02l__f2 = 2);

// ---- R0c: every harness through `latitude_lemma`; all faces at once ---------------------------

macro_rules! creg02_c {
    ($($name:ident = $faces:expr, $lat:literal);* $(;)?) => { $(
        /// P-CREG-02 below level 0 through `latitude_lemma`, the given faces, one coordinate.
        #[kani::proof]
        #[kani::unwind(11)]
        #[kani::stub(s2rst::s2::ij_level_to_bound_uv, quadrant_uv_bound)]
        #[kani::stub(s2rst::s2::LatLng::latitude, latitude_lemma)]
        #[kani::stub(f64::atan2, atan2_order)]
        #[kani::stub(f64::sqrt, sqrt_order)]
        fn $name() {
            reset();
            SUMSQ_LOG.lock().unwrap().used = 0;
            let (c, _) = any_cell();
            kani::assume(c.level().as_u8() >= 1 && ($faces)(c.face().as_u8()));
            let uv = c.bound_uv();
            let (u, v): (f64, f64) = (kani::any(), kani::any());
            kani::assume(uv.x.lo <= u && u <= uv.x.hi && uv.y.lo <= v && v <= uv.y.hi);
            let p = Point(face_uv_to_xyz(c.face(), u, v));
            let b = c.rect_bound();
            let ll = s2rst::s2::LatLng::from_point(p);
            if $lat {
                assert!(b.lat.contains(ll.lat.radians()));
            } else {
                assert!(b.lng.contains(ll.lng.radians()));
            }
        }
    )* };
}
creg02_c!(
    r0_creg02c__f0_lng = |f: u8| f == 0, false;
    r0_creg02c__f2_lng = |f: u8| f == 2, false;
    r0_creg02c__all_lat = |_f: u8| true, true;
    r0_creg02c__all_lng = |_f: u8| true, false;
);
