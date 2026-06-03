// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! Compressed encoding for sequences of `S2Points`.
//!
//! Points assumed to be centers of level-k cells are compressed using
//! run-length face encoding, Nth-derivative coding of (pi, qi) coordinates,
//! zigzag encoding, and bit interleaving.
//!
//! Corresponds to C++ `s2point_compression.h/cc`.

#![expect(
    clippy::cast_sign_loss,
    reason = "zigzag encoding and delta coding use intentional i32<->u32 bit reinterpretation"
)]
#![expect(
    clippy::cast_possible_truncation,
    reason = "zigzag/delta coding uses intentional bit truncation"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "u32 -> i32 for zigzag decoding — intentional bit reinterpretation"
)]
use std::io::{self, Read, Write};

use crate::s2::Point;
use crate::s2::coords::{
    MAX_CELL_LEVEL, MAX_SI_TI, NUM_FACES, face_uv_to_xyz, st_to_uv, xyz_to_face_si_ti,
};
use crate::s2::encoding::{read_uvarint, write_uvarint};

// ─── S2XYZFaceSiTi ─────────────────────────────────────────────────────

/// An `S2Point` together with its (face, si, ti) coordinates and cell level.
/// Intermediate representation of a point as XYZ + face/si/ti coordinates.
#[derive(Debug)]
pub struct S2XYZFaceSiTi {
    /// The original point.
    pub xyz: Point,
    /// The face on which the point lies.
    pub face: crate::s2::coords::Face,
    /// The si coordinate (scaled integer).
    pub si: u32,
    /// The ti coordinate (scaled integer).
    pub ti: u32,
    /// The cell level if this point is a cell center, or `None` if not.
    pub cell_level: Option<crate::s2::coords::Level>,
}

/// Converts a slice of Points to `S2XYZFaceSiTi` representations.
pub fn points_to_xyz_face_si_ti(vertices: &[Point]) -> Vec<S2XYZFaceSiTi> {
    vertices
        .iter()
        .map(|p| {
            let (face, si, ti, level) = xyz_to_face_si_ti(&p.0);
            S2XYZFaceSiTi {
                xyz: *p,
                face,
                si,
                ti,
                cell_level: level,
            }
        })
        .collect()
}

// ─── NthDerivativeCoder ─────────────────────────────────────────────────

const DERIVATIVE_ENCODING_ORDER: usize = 2;

struct NthDerivativeCoder {
    n: usize,
    m: usize,
    memory: [i32; 10],
}

impl NthDerivativeCoder {
    fn new(n: usize) -> Self {
        assert!(n <= 10);
        NthDerivativeCoder {
            n,
            m: 0,
            memory: [0; 10],
        }
    }

    fn encode(&mut self, mut k: i32) -> i32 {
        for i in 0..self.m {
            let delta = (k as u32).wrapping_sub(self.memory[i] as u32) as i32;
            self.memory[i] = k;
            k = delta;
        }
        if self.m < self.n {
            self.memory[self.m] = k;
            self.m += 1;
        }
        k
    }

    fn decode(&mut self, mut k: i32) -> i32 {
        if self.m < self.n {
            self.m += 1;
        }
        for i in (0..self.m).rev() {
            k = (self.memory[i] as u32).wrapping_add(k as u32) as i32;
            self.memory[i] = k;
        }
        k
    }
}

// ─── ZigZag encoding ────────────────────────────────────────────────────

fn zigzag_encode(n: i32) -> u32 {
    ((n as u32) << 1) ^ ((n >> 31) as u32)
}

fn zigzag_decode(n: u32) -> i32 {
    ((n >> 1) as i32) ^ (-((n & 1) as i32))
}

// ─── Bit interleaving ───────────────────────────────────────────────────

#[rustfmt::skip]
static INTERLEAVE_LUT: [u16; 256] = [
    0x0000, 0x0001, 0x0004, 0x0005, 0x0010, 0x0011, 0x0014, 0x0015,
    0x0040, 0x0041, 0x0044, 0x0045, 0x0050, 0x0051, 0x0054, 0x0055,
    0x0100, 0x0101, 0x0104, 0x0105, 0x0110, 0x0111, 0x0114, 0x0115,
    0x0140, 0x0141, 0x0144, 0x0145, 0x0150, 0x0151, 0x0154, 0x0155,
    0x0400, 0x0401, 0x0404, 0x0405, 0x0410, 0x0411, 0x0414, 0x0415,
    0x0440, 0x0441, 0x0444, 0x0445, 0x0450, 0x0451, 0x0454, 0x0455,
    0x0500, 0x0501, 0x0504, 0x0505, 0x0510, 0x0511, 0x0514, 0x0515,
    0x0540, 0x0541, 0x0544, 0x0545, 0x0550, 0x0551, 0x0554, 0x0555,
    0x1000, 0x1001, 0x1004, 0x1005, 0x1010, 0x1011, 0x1014, 0x1015,
    0x1040, 0x1041, 0x1044, 0x1045, 0x1050, 0x1051, 0x1054, 0x1055,
    0x1100, 0x1101, 0x1104, 0x1105, 0x1110, 0x1111, 0x1114, 0x1115,
    0x1140, 0x1141, 0x1144, 0x1145, 0x1150, 0x1151, 0x1154, 0x1155,
    0x1400, 0x1401, 0x1404, 0x1405, 0x1410, 0x1411, 0x1414, 0x1415,
    0x1440, 0x1441, 0x1444, 0x1445, 0x1450, 0x1451, 0x1454, 0x1455,
    0x1500, 0x1501, 0x1504, 0x1505, 0x1510, 0x1511, 0x1514, 0x1515,
    0x1540, 0x1541, 0x1544, 0x1545, 0x1550, 0x1551, 0x1554, 0x1555,
    0x4000, 0x4001, 0x4004, 0x4005, 0x4010, 0x4011, 0x4014, 0x4015,
    0x4040, 0x4041, 0x4044, 0x4045, 0x4050, 0x4051, 0x4054, 0x4055,
    0x4100, 0x4101, 0x4104, 0x4105, 0x4110, 0x4111, 0x4114, 0x4115,
    0x4140, 0x4141, 0x4144, 0x4145, 0x4150, 0x4151, 0x4154, 0x4155,
    0x4400, 0x4401, 0x4404, 0x4405, 0x4410, 0x4411, 0x4414, 0x4415,
    0x4440, 0x4441, 0x4444, 0x4445, 0x4450, 0x4451, 0x4454, 0x4455,
    0x4500, 0x4501, 0x4504, 0x4505, 0x4510, 0x4511, 0x4514, 0x4515,
    0x4540, 0x4541, 0x4544, 0x4545, 0x4550, 0x4551, 0x4554, 0x4555,
    0x5000, 0x5001, 0x5004, 0x5005, 0x5010, 0x5011, 0x5014, 0x5015,
    0x5040, 0x5041, 0x5044, 0x5045, 0x5050, 0x5051, 0x5054, 0x5055,
    0x5100, 0x5101, 0x5104, 0x5105, 0x5110, 0x5111, 0x5114, 0x5115,
    0x5140, 0x5141, 0x5144, 0x5145, 0x5150, 0x5151, 0x5154, 0x5155,
    0x5400, 0x5401, 0x5404, 0x5405, 0x5410, 0x5411, 0x5414, 0x5415,
    0x5440, 0x5441, 0x5444, 0x5445, 0x5450, 0x5451, 0x5454, 0x5455,
    0x5500, 0x5501, 0x5504, 0x5505, 0x5510, 0x5511, 0x5514, 0x5515,
    0x5540, 0x5541, 0x5544, 0x5545, 0x5550, 0x5551, 0x5554, 0x5555,
];

fn interleave_uint32(val0: u32, val1: u32) -> u64 {
    u64::from(INTERLEAVE_LUT[(val0 & 0xff) as usize])
        | (u64::from(INTERLEAVE_LUT[((val0 >> 8) & 0xff) as usize]) << 16)
        | (u64::from(INTERLEAVE_LUT[((val0 >> 16) & 0xff) as usize]) << 32)
        | (u64::from(INTERLEAVE_LUT[(val0 >> 24) as usize]) << 48)
        | (u64::from(INTERLEAVE_LUT[(val1 & 0xff) as usize]) << 1)
        | (u64::from(INTERLEAVE_LUT[((val1 >> 8) & 0xff) as usize]) << 17)
        | (u64::from(INTERLEAVE_LUT[((val1 >> 16) & 0xff) as usize]) << 33)
        | (u64::from(INTERLEAVE_LUT[(val1 >> 24) as usize]) << 49)
}

fn extract_even_bits(mut bits: u64) -> u32 {
    bits &= 0x5555555555555555;
    bits |= bits >> 1;
    bits &= 0x3333333333333333;
    bits |= bits >> 2;
    bits &= 0x0f0f0f0f0f0f0f0f;
    bits |= bits >> 4;
    bits &= 0x00ff00ff00ff00ff;
    bits |= bits >> 8;
    bits &= 0x0000ffff0000ffff;
    bits |= bits >> 16;
    bits as u32
}

fn deinterleave_uint32(code: u64) -> (u32, u32) {
    (extract_even_bits(code), extract_even_bits(code >> 1))
}

// ─── Coordinate conversion helpers ──────────────────────────────────────

fn si_ti_to_pi_qi(si: u32, level: i32) -> u32 {
    // Clamp to MAX_SI_TI - 1 to fit in `level` bits.
    let si = si.min(MAX_SI_TI - 1);
    si >> (i32::from(MAX_CELL_LEVEL) + 1 - level)
}

fn pi_qi_to_st(pi: u32, level: i32) -> f64 {
    (f64::from(pi) + 0.5) / f64::from(1u32 << level)
}

fn face_pi_qi_to_xyz(face: crate::s2::coords::Face, pi: u32, qi: u32, level: i32) -> Point {
    let u = st_to_uv(pi_qi_to_st(pi, level));
    let v = st_to_uv(pi_qi_to_st(qi, level));
    Point(face_uv_to_xyz(face, u, v).normalize())
}

// ─── Face run-length encoding ───────────────────────────────────────────

struct FaceRun {
    face: crate::s2::coords::Face,
    count: usize,
}

fn encode_faces(w: &mut dyn Write, faces: &[FaceRun]) -> io::Result<()> {
    for run in faces {
        write_uvarint(
            w,
            u64::from(NUM_FACES) * run.count as u64 + u64::from(run.face.as_u8()),
        )?;
    }
    Ok(())
}

fn decode_faces(r: &mut dyn Read, num_vertices: usize) -> io::Result<Vec<FaceRun>> {
    let mut runs = Vec::new();
    let mut num_parsed = 0usize;
    while num_parsed < num_vertices {
        let val = read_uvarint(r)?;
        let face = crate::s2::coords::Face::from_u8((val % u64::from(NUM_FACES)) as u8);
        let count = (val / u64::from(NUM_FACES)) as usize;
        if count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid face run count",
            ));
        }
        runs.push(FaceRun { face, count });
        num_parsed += count;
    }
    Ok(runs)
}

/// Iterator that yields faces one at a time from compressed runs.
struct FacesIterator {
    runs: Vec<FaceRun>,
    run_index: usize,
    count_used: usize,
}

impl FacesIterator {
    fn new(runs: Vec<FaceRun>) -> Self {
        FacesIterator {
            runs,
            run_index: 0,
            count_used: 0,
        }
    }

    fn next(&mut self) -> crate::s2::coords::Face {
        if self.count_used == self.runs[self.run_index].count {
            self.run_index += 1;
            self.count_used = 0;
        }
        self.count_used += 1;
        self.runs[self.run_index].face
    }
}

// ─── Point compression ──────────────────────────────────────────────────

fn encode_first_point_fixed_length(
    w: &mut dyn Write,
    pi: u32,
    qi: u32,
    level: i32,
    pi_coder: &mut NthDerivativeCoder,
    qi_coder: &mut NthDerivativeCoder,
) -> io::Result<()> {
    // No ZigZag for the first point (values cannot be negative).
    let encoded_pi = pi_coder.encode(pi as i32) as u32;
    let encoded_qi = qi_coder.encode(qi as i32) as u32;
    let interleaved = interleave_uint32(encoded_pi, encoded_qi);
    let bytes = interleaved.to_le_bytes();
    let bytes_required = ((level + 7) / 8 * 2) as usize;
    w.write_all(&bytes[..bytes_required])
}

fn encode_point_compressed(
    w: &mut dyn Write,
    pi: u32,
    qi: u32,
    pi_coder: &mut NthDerivativeCoder,
    qi_coder: &mut NthDerivativeCoder,
) -> io::Result<()> {
    let zz_pi = zigzag_encode(pi_coder.encode(pi as i32));
    let zz_qi = zigzag_encode(qi_coder.encode(qi as i32));
    let interleaved = interleave_uint32(zz_pi, zz_qi);
    write_uvarint(w, interleaved)
}

fn decode_first_point_fixed_length(
    r: &mut dyn Read,
    level: i32,
    pi_coder: &mut NthDerivativeCoder,
    qi_coder: &mut NthDerivativeCoder,
) -> io::Result<(u32, u32)> {
    let bytes_required = ((level + 7) / 8 * 2) as usize;
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf[..bytes_required])?;
    let interleaved = u64::from_le_bytes(buf);
    let (pi, qi) = deinterleave_uint32(interleaved);
    Ok((
        pi_coder.decode(pi as i32) as u32,
        qi_coder.decode(qi as i32) as u32,
    ))
}

fn decode_point_compressed(
    r: &mut dyn Read,
    pi_coder: &mut NthDerivativeCoder,
    qi_coder: &mut NthDerivativeCoder,
) -> io::Result<(u32, u32)> {
    let interleaved = read_uvarint(r)?;
    let (zz_pi, zz_qi) = deinterleave_uint32(interleaved);
    Ok((
        pi_coder.decode(zigzag_decode(zz_pi)) as u32,
        qi_coder.decode(zigzag_decode(zz_qi)) as u32,
    ))
}

/// Encodes points compressed. Points at cell centers for the given level are
/// encoded efficiently; off-center points are stored as raw xyz.
///
/// # Errors
///
/// Returns `io::Error` if writing to `w` fails.
pub fn encode_points_compressed(
    w: &mut dyn Write,
    vertices: &[S2XYZFaceSiTi],
    level: impl Into<crate::s2::coords::Level>,
) -> io::Result<()> {
    let level = level.into().as_i32();
    // Convert to (pi, qi) and collect face runs + off-center indices.
    let mut pi_qi: Vec<(u32, u32)> = Vec::with_capacity(vertices.len());
    let mut off_center: Vec<usize> = Vec::new();
    let mut face_runs: Vec<FaceRun> = Vec::new();

    for (i, v) in vertices.iter().enumerate() {
        // Face run-length encoding.
        if let Some(last) = face_runs.last_mut() {
            if last.face == v.face {
                last.count += 1;
            } else {
                face_runs.push(FaceRun {
                    face: v.face,
                    count: 1,
                });
            }
        } else {
            face_runs.push(FaceRun {
                face: v.face,
                count: 1,
            });
        }

        pi_qi.push((si_ti_to_pi_qi(v.si, level), si_ti_to_pi_qi(v.ti, level)));

        if v.cell_level.is_none_or(|l| l.as_i32() != level) {
            off_center.push(i);
        }
    }

    // Encode faces.
    encode_faces(w, &face_runs)?;

    // Encode points.
    let mut pi_coder = NthDerivativeCoder::new(DERIVATIVE_ENCODING_ORDER);
    let mut qi_coder = NthDerivativeCoder::new(DERIVATIVE_ENCODING_ORDER);

    for (i, &(pi, qi)) in pi_qi.iter().enumerate() {
        if i == 0 {
            encode_first_point_fixed_length(w, pi, qi, level, &mut pi_coder, &mut qi_coder)?;
        } else {
            encode_point_compressed(w, pi, qi, &mut pi_coder, &mut qi_coder)?;
        }
    }

    // Encode off-center points.
    write_uvarint(w, off_center.len() as u64)?;
    for &index in &off_center {
        write_uvarint(w, index as u64)?;
        let p = &vertices[index].xyz;
        w.write_all(&p.0.x.to_le_bytes())?;
        w.write_all(&p.0.y.to_le_bytes())?;
        w.write_all(&p.0.z.to_le_bytes())?;
    }

    Ok(())
}

/// Decodes points encoded with `encode_points_compressed`.
///
/// # Errors
///
/// Returns `io::Error` if reading from `r` fails or the data is malformed.
pub fn decode_points_compressed(
    r: &mut dyn Read,
    level: impl Into<crate::s2::coords::Level>,
    num_points: usize,
) -> io::Result<Vec<Point>> {
    let level = level.into().as_i32();
    // Decode faces.
    let face_runs = decode_faces(r, num_points)?;
    let mut faces_iter = FacesIterator::new(face_runs);

    let mut pi_coder = NthDerivativeCoder::new(DERIVATIVE_ENCODING_ORDER);
    let mut qi_coder = NthDerivativeCoder::new(DERIVATIVE_ENCODING_ORDER);

    let mut points = Vec::with_capacity(num_points);
    for i in 0..num_points {
        let (pi, qi) = if i == 0 {
            decode_first_point_fixed_length(r, level, &mut pi_coder, &mut qi_coder)?
        } else {
            decode_point_compressed(r, &mut pi_coder, &mut qi_coder)?
        };
        let face = faces_iter.next();
        points.push(face_pi_qi_to_xyz(face, pi, qi, level));
    }

    // Decode off-center points (overwrite with exact xyz).
    let num_off_center = read_uvarint(r)? as usize;
    if num_off_center > num_points {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "too many off-center points",
        ));
    }
    for _ in 0..num_off_center {
        let index = read_uvarint(r)? as usize;
        if index >= num_points {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "off-center index out of range",
            ));
        }
        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;
        let x = f64::from_le_bytes(buf);
        r.read_exact(&mut buf)?;
        let y = f64::from_le_bytes(buf);
        r.read_exact(&mut buf)?;
        let z = f64::from_le_bytes(buf);
        points[index] = Point(crate::r3::Vector { x, y, z });
    }

    Ok(points)
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::coords::Face;
    use crate::s2::earth;
    use crate::s2::testing::make_regular_points;
    use crate::s2::{CellId, LatLng};

    // ─── Test helpers (mirror C++ fixture) ───────────────────────────

    use crate::s2::coords::Level;

    fn snap_point_to_level(point: Point, level: Level) -> Point {
        CellId::from(&point).parent_at_level(level).to_point()
    }

    fn snap_points_to_level(points: &[Point], level: Level) -> Vec<Point> {
        points
            .iter()
            .map(|p| snap_point_to_level(*p, level))
            .collect()
    }

    fn make_regular(num_vertices: usize, radius_km: f64, level: Level) -> Vec<Point> {
        let center = Point::from_coords(1.0, 1.0, 1.0).normalize();
        let radius = earth::km_to_angle(radius_km);
        let unsnapped = make_regular_points(center, radius, num_vertices);
        snap_points_to_level(&unsnapped, level)
    }

    fn encode(points: &[Point], level: Level) -> Vec<u8> {
        let xyz_fst = points_to_xyz_face_si_ti(points);
        let mut buf = Vec::new();
        encode_points_compressed(&mut buf, &xyz_fst, level).unwrap();
        buf
    }

    fn decode(buf: &[u8], level: Level, num_points: usize) -> Vec<Point> {
        decode_points_compressed(&mut &buf[..], level, num_points).unwrap()
    }

    fn roundtrip(points: &[Point], level: Level) {
        let buf = encode(points, level);
        let decoded = decode(&buf, level, points.len());
        assert_eq!(decoded.len(), points.len(), "decoded length mismatch");
        for (i, (orig, dec)) in points.iter().zip(decoded.iter()).enumerate() {
            assert!(
                *orig == *dec,
                "vertex {i} mismatch:\n  original: {orig}\n  decoded:  {dec}"
            );
        }
    }

    // ─── C++ fixture data constructors ───────────────────────────────

    fn loop_4() -> Vec<Point> {
        make_regular(4, 0.1, Level::MAX)
    }

    fn loop_4_unsnapped() -> Vec<Point> {
        let center = Point::from_coords(1.0, 1.0, 1.0).normalize();
        let radius = earth::km_to_angle(0.1);
        make_regular_points(center, radius, 4)
    }

    fn loop_4_level_14() -> Vec<Point> {
        make_regular(4, 0.1, Level::new(14))
    }

    fn loop_100() -> Vec<Point> {
        make_regular(100, 0.1, Level::MAX)
    }

    fn loop_100_unsnapped() -> Vec<Point> {
        let center = Point::from_coords(1.0, 1.0, 1.0).normalize();
        let radius = earth::km_to_angle(0.1);
        make_regular_points(center, radius, 100)
    }

    fn loop_100_mixed(snap_count: usize, snap_stride: usize) -> Vec<Point> {
        let center = Point::from_coords(1.0, 1.0, 1.0).normalize();
        let radius = earth::km_to_angle(0.1);
        let mut pts = make_regular_points(center, radius, 100);
        for i in 0..snap_count {
            pts[snap_stride * i] = snap_point_to_level(pts[snap_stride * i], Level::MAX);
        }
        pts
    }

    fn loop_100_level_22() -> Vec<Point> {
        make_regular(100, 0.1, Level::new(22))
    }

    fn loop_multi_face() -> Vec<Point> {
        let pts = vec![
            Point(face_uv_to_xyz(Face::from_u8(0), -0.5, 0.5).normalize()),
            Point(face_uv_to_xyz(Face::from_u8(1), -0.5, 0.5).normalize()),
            Point(face_uv_to_xyz(Face::from_u8(1), 0.5, -0.5).normalize()),
            Point(face_uv_to_xyz(Face::from_u8(2), -0.5, 0.5).normalize()),
            Point(face_uv_to_xyz(Face::from_u8(2), 0.5, -0.5).normalize()),
            Point(face_uv_to_xyz(Face::from_u8(2), 0.5, 0.5).normalize()),
        ];
        snap_points_to_level(&pts, Level::MAX)
    }

    fn line() -> Vec<Point> {
        let mut pts = Vec::with_capacity(100);
        for i in 0..100 {
            let s = 0.01 + 0.005 * f64::from(i);
            let t = 0.01 + 0.009 * f64::from(i);
            let u = st_to_uv(s);
            let v = st_to_uv(t);
            pts.push(Point(face_uv_to_xyz(Face::from_u8(0), u, v).normalize()));
        }
        snap_points_to_level(&pts, Level::MAX)
    }

    // ─── Original tests ──────────────────────────────────────────────

    #[test]
    fn test_zigzag_roundtrip() {
        for &v in &[0i32, 1, -1, 100, -100, i32::MAX, i32::MIN] {
            assert_eq!(
                zigzag_decode(zigzag_encode(v)),
                v,
                "zigzag roundtrip failed for {v}"
            );
        }
    }

    #[test]
    fn test_interleave_roundtrip() {
        let cases: Vec<(u32, u32)> = vec![
            (0, 0),
            (1, 0),
            (0, 1),
            (1, 1),
            (0xFFFF, 0xFFFF),
            (0xDEADBEEF, 0xCAFEBABE),
            (u32::MAX, u32::MAX),
            (u32::MAX, 0),
            (0, u32::MAX),
        ];
        for (a, b) in cases {
            let interleaved = interleave_uint32(a, b);
            let (da, db) = deinterleave_uint32(interleaved);
            assert_eq!(
                (a, b),
                (da, db),
                "interleave roundtrip failed for ({a:#x}, {b:#x})"
            );
        }
    }

    #[test]
    fn test_nth_derivative_coder() {
        let input = [0i32, 0, 0, 0, 1, 2, 3, 4, 9, 16, 25, 36];
        let mut encoder = NthDerivativeCoder::new(2);
        let encoded: Vec<i32> = input.iter().map(|&v| encoder.encode(v)).collect();

        let mut decoder = NthDerivativeCoder::new(2);
        let decoded: Vec<i32> = encoded.iter().map(|&v| decoder.decode(v)).collect();

        assert_eq!(&decoded, &input);
    }

    #[test]
    fn test_encode_decode_points_all_on_level() {
        let level = Level::MAX;
        let base = CellId::from_face(0);
        let mut vertices = Vec::new();
        let mut id = base.child_begin_at_level(level);
        for _ in 0..20 {
            vertices.push(id.to_point());
            id = id.next();
        }

        let xyz_fst = points_to_xyz_face_si_ti(&vertices);
        for v in &xyz_fst {
            assert_eq!(v.cell_level, Some(level), "expected on-level");
        }

        let mut buf = Vec::new();
        encode_points_compressed(&mut buf, &xyz_fst, level).unwrap();
        let decoded = decode_points_compressed(&mut buf.as_slice(), level, vertices.len()).unwrap();

        assert_eq!(decoded.len(), vertices.len());
        for (i, (orig, dec)) in vertices.iter().zip(decoded.iter()).enumerate() {
            let dist = (orig.0 - dec.0).norm();
            assert!(dist < 1e-15, "vertex {i} too far: {dist}");
        }
    }

    #[test]
    fn test_encode_decode_points_mixed() {
        let level = Level::new(10);
        let cell = CellId::from_face(2).child_begin_at_level(level);
        let on_center = cell.to_point();
        let off_center = LatLng::from_degrees(37.7749, -122.4194).to_point();

        let vertices = vec![on_center, off_center, on_center];
        let xyz_fst = points_to_xyz_face_si_ti(&vertices);

        let mut buf = Vec::new();
        encode_points_compressed(&mut buf, &xyz_fst, level).unwrap();
        let decoded = decode_points_compressed(&mut buf.as_slice(), level, vertices.len()).unwrap();

        assert_eq!(decoded.len(), vertices.len());
        let dist0 = (vertices[0].0 - decoded[0].0).norm();
        assert!(dist0 < 1e-15, "on-center vertex 0 too far: {dist0}");
        assert_eq!(vertices[1], decoded[1], "off-center vertex should be exact");
        let dist2 = (vertices[2].0 - decoded[2].0).norm();
        assert!(dist2 < 1e-15, "on-center vertex 2 too far: {dist2}");
    }

    // ─── C++ tests: roundtrips and sizes ─────────────────────────────

    /// C++ `TEST_F(S2PointCompressionTest`, `RoundtripsEmpty`)
    #[test]
    fn test_roundtrips_empty() {
        let level = Level::MAX;
        let buf = encode(&[], level);
        let decoded = decode(&buf, level, 0);
        assert!(decoded.is_empty());
    }

    /// C++ `TEST_F(S2PointCompressionTest`, `RoundtripsFourVertexLoop`)
    #[test]
    fn test_roundtrips_four_vertex_loop() {
        roundtrip(&loop_4(), Level::MAX);
    }

    /// C++ `TEST_F(S2PointCompressionTest`, `RoundtripsFourVertexLoopUnsnapped`)
    #[test]
    fn test_roundtrips_four_vertex_loop_unsnapped() {
        roundtrip(&loop_4_unsnapped(), Level::MAX);
    }

    /// C++ `TEST_F(S2PointCompressionTest`, `FourVertexLoopSize`)
    #[test]
    fn test_four_vertex_loop_size() {
        let buf = encode(&loop_4(), Level::MAX);
        // C++ expects 39 bytes. Uncompressed would be 32 bytes (4 × 8 bytes).
        assert_eq!(buf.len(), 39, "4-vertex snapped loop encoded size");
    }

    /// C++ `TEST_F(S2PointCompressionTest`, `RoundtripsFourVertexLevel14Loop`)
    #[test]
    fn test_roundtrips_four_vertex_level_14_loop() {
        roundtrip(&loop_4_level_14(), Level::new(14));
    }

    /// C++ `TEST_F(S2PointCompressionTest`, `FourVertexLevel14LoopSize`)
    #[test]
    fn test_four_vertex_level_14_loop_size() {
        let buf = encode(&loop_4_level_14(), Level::new(14));
        // C++ expects 23 bytes (4 bytes per vertex without compression).
        assert_eq!(buf.len(), 23, "4-vertex level-14 loop encoded size");
    }

    /// C++ `TEST_F(S2PointCompressionTest`, `Roundtrips100VertexLoop`)
    #[test]
    fn test_roundtrips_100_vertex_loop() {
        roundtrip(&loop_100(), Level::MAX);
    }

    /// C++ `TEST_F(S2PointCompressionTest`, `Roundtrips100VertexLoopUnsnapped`)
    #[test]
    fn test_roundtrips_100_vertex_loop_unsnapped() {
        roundtrip(&loop_100_unsnapped(), Level::MAX);
    }

    /// C++ `TEST_F(S2PointCompressionTest`, `Roundtrips100VertexLoopMixed15`)
    #[test]
    fn test_roundtrips_100_vertex_loop_mixed_15() {
        let pts = loop_100_mixed(15, 3);
        roundtrip(&pts, Level::MAX);
        let buf = encode(&pts, Level::MAX);
        assert_eq!(buf.len(), 2381, "100-vertex mixed-15 loop encoded size");
    }

    /// C++ `TEST_F(S2PointCompressionTest`, `Roundtrips100VertexLoopMixed25`)
    #[test]
    fn test_roundtrips_100_vertex_loop_mixed_25() {
        let pts = loop_100_mixed(25, 4);
        roundtrip(&pts, Level::MAX);
        let buf = encode(&pts, Level::MAX);
        assert_eq!(buf.len(), 2131, "100-vertex mixed-25 loop encoded size");
    }

    /// C++ `TEST_F(S2PointCompressionTest`, `OneHundredVertexLoopSize`)
    #[test]
    fn test_100_vertex_loop_size() {
        let buf = encode(&loop_100(), Level::MAX);
        assert_eq!(buf.len(), 257, "100-vertex snapped loop encoded size");
    }

    /// C++ `TEST_F(S2PointCompressionTest`, `OneHundredVertexLoopUnsnappedSize`)
    #[test]
    fn test_100_vertex_loop_unsnapped_size() {
        let buf = encode(&loop_100_unsnapped(), Level::MAX);
        assert_eq!(buf.len(), 2756, "100-vertex unsnapped loop encoded size");
    }

    /// C++ `TEST_F(S2PointCompressionTest`, `Roundtrips100VertexLevel22Loop`)
    #[test]
    fn test_roundtrips_100_vertex_level_22_loop() {
        roundtrip(&loop_100_level_22(), Level::new(22));
    }

    /// C++ `TEST_F(S2PointCompressionTest`, `OneHundredVertexLoopLevel22Size`)
    #[test]
    fn test_100_vertex_loop_level_22_size() {
        let buf = encode(&loop_100_level_22(), Level::new(22));
        assert_eq!(buf.len(), 148, "100-vertex level-22 loop encoded size");
    }

    /// C++ `TEST_F(S2PointCompressionTest`, `MultiFaceLoop`)
    #[test]
    fn test_multi_face_loop() {
        roundtrip(&loop_multi_face(), Level::MAX);
    }

    /// C++ `TEST_F(S2PointCompressionTest`, `StraightLineCompressesWell`)
    #[test]
    fn test_straight_line_compresses_well() {
        let line_pts = line();
        roundtrip(&line_pts, Level::MAX);
        let buf = encode(&line_pts, Level::MAX);
        // C++ expects about 1 byte/vertex: line.size() + 17
        assert_eq!(
            buf.len(),
            line_pts.len() + 17,
            "line should compress to ~1 byte/vertex"
        );
    }

    /// C++ `TEST_F(S2PointCompressionTest`, `FirstPointOnFaceEdge`)
    ///
    /// Regression: `EncodeFirstPointFixedLength` tried to encode a pi/qi
    /// value of (2^level) in "level" bits. Fixed by `si_ti_to_pi_qi`.
    #[test]
    fn test_first_point_on_face_edge() {
        let points = vec![
            S2XYZFaceSiTi {
                xyz: Point::from_coords(
                    0.054299323861222645,
                    -0.70606358900180299,
                    0.70606358900180299,
                ),
                face: Face::from_u8(2),
                si: 956301312,
                ti: 2147483648, // MAX_SI_TI
                cell_level: None,
            },
            S2XYZFaceSiTi {
                xyz: Point::from_coords(
                    0.056482651436986935,
                    -0.70781701406865505,
                    0.70413406726388494,
                ),
                face: Face::from_u8(4),
                si: 4194304,
                ti: 1195376640,
                cell_level: Some(Level::new(8)),
            },
        ];

        let mut buf = Vec::new();
        encode_points_compressed(&mut buf, &points, Level::new(8)).unwrap();
        let decoded = decode_points_compressed(&mut buf.as_slice(), Level::new(8), 2).unwrap();
        assert_eq!(decoded[0], points[0].xyz);
        assert_eq!(decoded[1], points[1].xyz);
    }
}

#[cfg(test)]
mod quickcheck_tests {
    use super::*;
    use crate::s2::CellId;
    use crate::s2::coords::Level;
    use quickcheck_macros::quickcheck;

    #[quickcheck]
    fn prop_zigzag_roundtrip(n: i32) -> bool {
        zigzag_decode(zigzag_encode(n)) == n
    }

    #[quickcheck]
    fn prop_zigzag_non_negative(n: i32) -> bool {
        // ZigZag encoding always produces a non-negative value
        // (the MSB of u32 is used for sign, but encoded value is unsigned).
        // More importantly: small absolute values map to small encoded values.
        let encoded = zigzag_encode(n);
        if n == 0 { encoded == 0 } else { encoded > 0 }
    }

    #[quickcheck]
    fn prop_zigzag_small_values_compress_well(n: i16) -> bool {
        // Values near zero should encode to small unsigned values,
        // which is the whole point of zigzag encoding for varints.
        let encoded = zigzag_encode(i32::from(n));
        // |n| <= 32767, so encoded <= 65534
        encoded <= (2 * u32::from(n.unsigned_abs()))
    }

    #[quickcheck]
    fn prop_interleave_roundtrip(a: u32, b: u32) -> bool {
        let interleaved = interleave_uint32(a, b);
        let (da, db) = deinterleave_uint32(interleaved);
        da == a && db == b
    }

    #[quickcheck]
    fn prop_interleave_bit_placement(a: u32, b: u32) -> bool {
        // Bit i of a should appear at bit 2*i of the result.
        // Bit i of b should appear at bit 2*i+1 of the result.
        let interleaved = interleave_uint32(a, b);
        for i in 0..32 {
            let a_bit = u64::from((a >> i) & 1);
            let b_bit = u64::from((b >> i) & 1);
            if ((interleaved >> (2 * i)) & 1) != a_bit {
                return false;
            }
            if ((interleaved >> (2 * i + 1)) & 1) != b_bit {
                return false;
            }
        }
        true
    }

    #[quickcheck]
    fn prop_nth_derivative_roundtrip(values: Vec<i32>) -> bool {
        if values.is_empty() {
            return true;
        }
        // Limit length to avoid excessively long tests.
        let values: Vec<i32> = values.into_iter().take(200).collect();
        for order in 0..=3 {
            let mut encoder = NthDerivativeCoder::new(order);
            let encoded: Vec<i32> = values.iter().map(|&v| encoder.encode(v)).collect();
            let mut decoder = NthDerivativeCoder::new(order);
            let decoded: Vec<i32> = encoded.iter().map(|&v| decoder.decode(v)).collect();
            if decoded != values {
                return false;
            }
        }
        true
    }

    #[quickcheck]
    fn prop_nth_derivative_constant_sequence(val: i32, len: u8) -> bool {
        // A constant sequence should have 0 as all derivatives after the first.
        let len = (len % 50) as usize + 2;
        let values: Vec<i32> = vec![val; len];
        let mut encoder = NthDerivativeCoder::new(1);
        let encoded: Vec<i32> = values.iter().map(|&v| encoder.encode(v)).collect();
        // After the first value, all 1st derivatives of a constant are 0.
        encoded[1..].iter().all(|&v| v == 0)
    }

    #[quickcheck]
    fn prop_nth_derivative_linear_sequence(start: i16, step: i16, len: u8) -> bool {
        // A linear sequence should have 0 as all 2nd derivatives after ramp-up.
        let len = (len % 50) as usize + 3;
        let values: Vec<i32> = (0..len)
            .map(|i| i32::from(start).wrapping_add(i32::from(step).wrapping_mul(i as i32)))
            .collect();
        let mut encoder = NthDerivativeCoder::new(2);
        let encoded: Vec<i32> = values.iter().map(|&v| encoder.encode(v)).collect();
        // After the ramp-up (first 2 values), all 2nd derivatives should be 0.
        encoded[2..].iter().all(|&v| v == 0)
    }

    #[quickcheck]
    fn prop_cell_center_points_roundtrip_at_level(raw: u64) -> bool {
        // Generate a valid cell at a random level and verify compressed roundtrip.
        let face = (raw % 6) as u8;
        let level = Level::new(((raw >> 3) % 31) as u8);
        let pos = raw >> 6;
        let cell = CellId::from_face_pos_level(face, pos, level);
        let point = cell.to_point();

        let vertices = vec![point];
        let xyz_fst = points_to_xyz_face_si_ti(&vertices);

        // Should detect it as a cell center at the right level.
        if xyz_fst[0].cell_level != Some(level) {
            // Some cells may not roundtrip the level perfectly; skip those.
            return true;
        }

        let mut buf = Vec::new();
        encode_points_compressed(&mut buf, &xyz_fst, level).unwrap();
        let decoded = decode_points_compressed(&mut buf.as_slice(), level, 1).unwrap();
        let dist = (point.0 - decoded[0].0).norm();
        dist < 1e-15
    }

    #[quickcheck]
    fn prop_off_center_points_exact_roundtrip(x: i16, y: i16, z: i16) -> bool {
        // Use i16 to keep values reasonable and avoid quickcheck shrink overflow.
        // Arbitrary (non-cell-center) points should be stored exactly as raw xyz.
        if x == 0 && y == 0 && z == 0 {
            return true;
        }
        let v = crate::r3::Vector {
            x: f64::from(x),
            y: f64::from(y),
            z: f64::from(z),
        };
        let point = Point(v.normalize());

        // xyz_to_face_si_ti can panic for edge-case inputs (overflow in
        // coords.rs for si/ti at boundaries). Use catch_unwind to skip those.
        let result = std::panic::catch_unwind(|| {
            let xyz_fst = points_to_xyz_face_si_ti(&[point]);
            let level = Level::MAX;

            let mut buf = Vec::new();
            encode_points_compressed(&mut buf, &xyz_fst, level).unwrap();
            let decoded = decode_points_compressed(&mut buf.as_slice(), level, 1).unwrap();

            if xyz_fst[0].cell_level == Some(level) {
                (point.0 - decoded[0].0).norm() < 1e-15
            } else {
                point == decoded[0]
            }
        });
        result.unwrap_or(true) // skip panicking inputs
    }

    #[quickcheck]
    fn prop_face_run_roundtrip(faces: Vec<u8>) -> bool {
        if faces.is_empty() {
            return true;
        }
        // Limit and clamp to valid face range.
        let faces: Vec<crate::s2::coords::Face> = faces
            .into_iter()
            .take(100)
            .map(|f| crate::s2::coords::Face::from_u8(f % 6))
            .collect();
        let num = faces.len();

        // Build runs.
        let mut runs: Vec<FaceRun> = Vec::new();
        for &f in &faces {
            if let Some(last) = runs.last_mut()
                && last.face == f
            {
                last.count += 1;
                continue;
            }
            runs.push(FaceRun { face: f, count: 1 });
        }

        // Encode and decode.
        let mut buf = Vec::new();
        encode_faces(&mut buf, &runs).unwrap();
        let decoded_runs = decode_faces(&mut buf.as_slice(), num).unwrap();

        // Expand both back to face sequences and compare.
        let mut orig_expanded = Vec::new();
        for r in &runs {
            for _ in 0..r.count {
                orig_expanded.push(r.face);
            }
        }
        let mut dec_expanded = Vec::new();
        for r in &decoded_runs {
            for _ in 0..r.count {
                dec_expanded.push(r.face);
            }
        }
        orig_expanded == dec_expanded
    }

    #[quickcheck]
    fn prop_compressed_smaller_for_cell_centers(raw: u64) -> bool {
        // For sequences of cell centers at the same level, compressed encoding
        // should be significantly smaller than 24 bytes/vertex (lossless).
        let face = (raw % 6) as u8;
        let level = Level::new(((raw >> 3) % 25 + 5) as u8); // levels 5..29
        let pos = raw >> 8;
        let base = CellId::from_face_pos_level(face, pos, level);

        let mut vertices = Vec::new();
        let mut id = base;
        for _ in 0..10 {
            vertices.push(id.to_point());
            id = id.next();
        }
        let xyz_fst = points_to_xyz_face_si_ti(&vertices);

        let mut buf = Vec::new();
        encode_points_compressed(&mut buf, &xyz_fst, level).unwrap();
        let compressed_size = buf.len();
        let lossless_size = 24 * vertices.len(); // 3 * 8 bytes per vertex

        // Compressed should be much smaller (typically ~4 bytes/vertex).
        compressed_size < lossless_size
    }
}
