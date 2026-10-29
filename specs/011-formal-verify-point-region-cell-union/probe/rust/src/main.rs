//! Survey probe for spec 011 (`PointRegion`, and `CellUnion`'s floating-point methods),
//! 2026-10-28. Library side; `../cpp/survey.cc` runs the same cases on upstream C++. Both print
//! one line per case (`<section> <index> <values as hex bits or 0/1>`), so `compare.py` can diff
//! them line by line. The inputs come from the same 64-bit LCG in both programs.

use s2rst::r3::Vector;
use s2rst::s1::Angle;
use s2rst::s2::point_region::PointRegion;
use s2rst::s2::{Cap, Cell, CellId, CellUnion, CellVertex, Face, Level, Point, Rect, Region};

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }
    /// A double in [0, 1).
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / 9_007_199_254_740_992.0
    }
    /// A double in [lo, hi).
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }
}

fn b(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

fn cap_s(c: Cap) -> String {
    let p = c.center();
    format!(
        "{} {} {} {}",
        b(p.x()),
        b(p.y()),
        b(p.z()),
        b(c.chord_radius().length2())
    )
}

fn rect_s(r: Rect) -> String {
    format!(
        "{} {} {} {}",
        b(r.lat.lo),
        b(r.lat.hi),
        b(r.lng.lo),
        b(r.lng.hi)
    )
}

fn ids_s(ids: &[CellId]) -> String {
    let mut s = ids.len().to_string();
    for id in ids {
        s += &format!(" {:016x}", id.id());
    }
    s
}

/// FNV-1a over the ids, for unions too long to print.
fn ids_hash(ids: &[CellId]) -> String {
    let mut h: u64 = 14_695_981_039_346_656_037;
    for id in ids {
        let v = id.id();
        for k in 0..8 {
            h ^= (v >> (8 * k)) & 0xff;
            h = h.wrapping_mul(1_099_511_628_211);
        }
    }
    format!("{} {h:016x}", ids.len())
}

fn raw(x: f64, y: f64, z: f64) -> Point {
    Point(Vector { x, y, z })
}

fn random_point(g: &mut Lcg) -> Point {
    loop {
        let (x, y, z) = (g.range(-1.0, 1.0), g.range(-1.0, 1.0), g.range(-1.0, 1.0));
        let n2 = x * x + y * y + z * z;
        if n2 > 1e-6 && n2 <= 1.0 {
            return Point(Vector { x, y, z }.normalize());
        }
    }
}

fn random_id(g: &mut Lcg, min_level: u64, max_level: u64) -> CellId {
    let face = (g.next() % 6) as usize;
    let pos = g.next() & ((1u64 << 61) - 1);
    let level = (min_level + g.next() % (max_level - min_level + 1)) as u8;
    CellId::from_face_pos_level(Face::ALL[face], pos, Level::new(level))
}

fn child(id: CellId, k: u64) -> CellId {
    id.children()[k as usize]
}

/// A random point of the cell: the centre of a random descendant at a random finer level.
fn point_in(g: &mut Lcg, id: CellId) -> Point {
    let level = u64::from(id.level().as_u8()) + g.next() % (31 - u64::from(id.level().as_u8()));
    let mut d = id;
    while u64::from(d.level().as_u8()) < level {
        d = child(d, g.next() % 4);
    }
    d.to_point()
}

/// A random union: either cells anywhere (levels 0..20), or a cluster of descendants of one cell.
fn random_union_ids(g: &mut Lcg) -> Vec<CellId> {
    let mut ids = Vec::new();
    let n = 1 + g.next() % 8;
    if g.next() % 2 == 0 {
        for _ in 0..n {
            ids.push(random_id(g, 0, 20));
        }
    } else {
        let base = random_id(g, 0, 18);
        for _ in 0..n {
            let extra = 1 + g.next() % 8;
            let mut d = base;
            let mut e = 0;
            while e < extra && d.level() < Level::MAX {
                d = child(d, g.next() % 4);
                e += 1;
            }
            ids.push(d);
        }
    }
    ids
}

#[allow(clippy::too_many_lines)]
fn main() {
    // PC, PR, PU: PointRegion's bounds over random unit points and special vectors.
    let mut g = Lcg(0x0110_0011_0011_0011);
    let pts: Vec<Point> = (0..20000).map(|_| random_point(&mut g)).collect();
    let specials = [
        raw(1.0, 0.0, 0.0),
        raw(-1.0, -0.0, 0.0),
        raw(-1.0, 0.0, 0.0),
        raw(0.0, 0.0, 1.0),
        raw(0.0, 0.0, -1.0),
        raw(0.0, 0.0, 0.0),
        raw(3.0, 4.0, 0.0),
        raw(1e-300, 0.0, 0.0),
        raw(f64::NAN, 0.0, 1.0),
        raw(f64::INFINITY, 0.0, 0.0),
        raw(-0.0, -0.0, -1.0),
    ];
    for (i, &p) in pts.iter().enumerate() {
        let r = PointRegion::new(p);
        println!("PC {i} {}", cap_s(r.cap_bound()));
        println!("PR {i} {}", rect_s(r.rect_bound()));
        if i < 2000 {
            let ids = r.cell_union_bound();
            println!("PU {i} {}", ids_s(&ids));
            println!(
                "PUC {i} {}",
                u8::from(CellUnion::from_cell_ids(ids).contains_point(p))
            );
        }
    }
    for (i, &p) in specials.iter().enumerate() {
        let r = PointRegion::new(p);
        println!("PSC {i} {}", cap_s(r.cap_bound()));
        println!("PSR {i} {}", rect_s(r.rect_bound()));
    }

    // PX: contains_cell and intersects_cell: the point's own cell at each level, and a random
    // cell.
    let mut g = Lcg(0x0120_0012_0012_0012);
    for i in 0..5000 {
        let p = random_point(&mut g);
        let r = PointRegion::new(p);
        let leaf = CellId::from_point(&p);
        let level = (g.next() % 31) as u8;
        let own = Cell::from(leaf.parent_at_level(Level::new(level)));
        let other = Cell::from(random_id(&mut g, 0, 30));
        println!(
            "PX {i} {} {} {} {}",
            u8::from(r.contains_cell(&own)),
            u8::from(r.intersects_cell(&own)),
            u8::from(r.contains_cell(&other)),
            u8::from(r.intersects_cell(&other))
        );
    }

    // PV: the vertex law for PointRegion: the region of a cell's vertex intersects the cell,
    // i.e. Cell::contains_point(vertex), for each of the four vertices.
    let mut g = Lcg(0x0130_0013_0013_0013);
    let mut pv_fail = 0;
    for i in 0..20000 {
        let c = Cell::from(random_id(&mut g, 0, 30));
        let mut s = String::new();
        for k in CellVertex::ALL {
            let inside = PointRegion::new(c.vertex(k)).intersects_cell(&c);
            if !inside {
                pv_fail += 1;
            }
            s += if inside { " 1" } else { " 0" };
        }
        println!("PV {i}{s}");
    }
    eprintln!("PV cells whose own vertex Cell::contains_point rejects: {pv_fail} of 80000");

    // UC, UR, UU, UA: CellUnion's bounds and areas over random unions.
    let mut g = Lcg(0x0140_0014_0014_0014);
    let (mut ug_contained, mut ug_cap_miss, mut ug_rect_miss, mut uu_miss) = (0, 0, 0, 0);
    for i in 0..10000 {
        let u = CellUnion::from_cell_ids(random_union_ids(&mut g));
        let cap = u.cap_bound();
        let rect = u.rect_bound();
        println!("UC {i} {}", cap_s(cap));
        println!("UR {i} {}", rect_s(rect));
        let bound = u.cell_union_bound();
        println!("UU {i} {}", ids_s(&bound));
        println!(
            "UA {i} {} {} {}",
            b(u.average_based_area()),
            b(u.approx_area()),
            b(u.exact_area())
        );
        // UG: the bounds contain every contained point: each cell's centre, its vertices and
        // random points of it.
        let (mut cap_miss, mut rect_miss, mut contained) = (0, 0, 0);
        for &id in u.cell_ids() {
            let c = Cell::from(id);
            let mut ps = vec![c.center()];
            for k in CellVertex::ALL {
                ps.push(c.vertex(k));
            }
            for _ in 0..4 {
                ps.push(point_in(&mut g, id));
            }
            for p in ps {
                if !u.contains_point(p) {
                    continue;
                }
                contained += 1;
                if !cap.contains_point(p) {
                    cap_miss += 1;
                }
                if !rect.contains_point(p) {
                    rect_miss += 1;
                }
            }
        }
        ug_contained += contained;
        ug_cap_miss += cap_miss;
        ug_rect_miss += rect_miss;
        println!("UG {i} {contained} {cap_miss} {rect_miss}");
        // UUC: the covering covers every cell of the union.
        let cover = CellUnion::from_cell_ids(bound);
        let miss = u
            .cell_ids()
            .iter()
            .filter(|&&id| !cover.contains_cell_id(id))
            .count();
        uu_miss += miss;
        println!("UUC {i} {miss}");
    }
    eprintln!(
        "UG contained points {ug_contained}, outside the cap bound {ug_cap_miss}, outside the \
         rect bound {ug_rect_miss}; union cells outside the covering {uu_miss}"
    );

    // US: special unions: empty, whole sphere, one leaf, opposite faces.
    let f = |k: usize| CellId::from_face(Face::ALL[k]);
    let sp: Vec<Vec<CellId>> = vec![
        vec![],
        (0..6).map(f).collect(),
        vec![CellId::from_point(&raw(1.0, 0.0, 0.0))],
        vec![f(0), f(3)],
        vec![f(2), f(5)],
        vec![f(1).children()[0], f(4).children()[3]],
    ];
    for (i, ids) in sp.into_iter().enumerate() {
        let u = CellUnion::from_cell_ids(ids);
        println!(
            "US {i} {} {} {} {} {} {}",
            cap_s(u.cap_bound()),
            rect_s(u.rect_bound()),
            ids_s(&u.cell_union_bound()),
            b(u.average_based_area()),
            b(u.approx_area()),
            b(u.exact_area())
        );
    }

    // CF: Cell::rect_bound() of the six face cells (UR's differences).
    for (k, face) in Face::ALL.into_iter().enumerate() {
        println!("CF {k} {}", rect_s(Cell::from(CellId::from_face(face)).rect_bound()));
    }

    // UV: the vertex law for CellUnion: the edge neighbours of a union's cells that the union
    // does not intersect, and how many of their vertices the union contains.
    let mut g = Lcg(0x0160_0016_0016_0016);
    let (mut uv_cells, mut uv_contained) = (0, 0);
    for i in 0..5000 {
        let u = CellUnion::from_cell_ids(random_union_ids(&mut g));
        let (mut cells, mut contained) = (0, 0);
        for &id in u.cell_ids() {
            for n in id.edge_neighbors() {
                let c = Cell::from(n);
                if u.intersects_cell(&c) {
                    continue;
                }
                cells += 1;
                for k in CellVertex::ALL {
                    contained += usize::from(u.contains_point(c.vertex(k)));
                }
            }
        }
        uv_cells += cells;
        uv_contained += contained;
        println!("UV {i} {cells} {contained}");
    }
    eprintln!(
        "UV non-intersected edge neighbours {uv_cells}, their vertices the union contains \
         {uv_contained}"
    );

    // UE: expand_by_radius(min_radius, max_level_diff) over small clustered unions.
    let mut g = Lcg(0x0150_0015_0015_0015);
    let radii = [
        0.0,
        -1.0,
        4.0,
        2.0,
        1e-300,
        f64::INFINITY,
        0.942_809_041_582_063_4,
        0.942_809_041_582_063_5,
        0.471_404_520_791_031_7,
    ];
    for i in 0..3000 {
        let mut ids = Vec::new();
        let base = random_id(&mut g, 4, 20);
        let n = 1 + g.next() % 3;
        for _ in 0..n {
            let mut d = base;
            let extra = g.next() % 4;
            let mut e = 0;
            while e < extra && d.level() < Level::MAX {
                d = child(d, g.next() % 4);
                e += 1;
            }
            ids.push(d);
        }
        let mut u = CellUnion::from_cell_ids(ids);
        let mut radius = radii[i % 9];
        if i >= 9 * 20 {
            let e = g.range(-9.0, 0.0);
            radius = 10f64.powf(e) * if g.next() % 2 == 1 { 1.0 } else { 0.5 };
        }
        let mut diff = (g.next() % 6) as u8;
        if i % 50 == 0 {
            diff = 40;
        }
        // Keep expansions small: a radius far above the cells' width expands by many cells.
        if radius > 1e-3 && base.level().as_u8() > 8 && i >= 9 * 20 {
            radius = 1e-4;
        }
        u.expand_by_radius(Angle::from_radians(radius), diff);
        println!("UE {i} {}", ids_hash(u.cell_ids()));
    }
}
