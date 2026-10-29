#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
# Written for this crate (not ported from upstream S2).
"""Writes spec 011's mutant patches (verify/mutants/M-95N-*.patch) from exact-string edits."""
import difflib, os, sys
repo = '/home/t/sources/s2rst_multi/3'
outdir = os.path.join(repo, 'verify/mutants')
PR = 'core/src/s2/point_region.rs'
CU = 'core/src/s2/cell_union.rs'
CELL = 'core/src/s2/cell.rs'
CAP = 'core/src/s2/cap.rs'
MET = 'core/src/s2/metric.rs'
SPEC = 'verify/src/region_spec.rs'
STUBS = 'verify/src/stubs.rs'
M = []
def m(num, slug, path, old, new, targets, expect, defect, extra=()):
    M.append((num, slug, path, old, new, targets, expect, defect, extra))

m(950, 'pr-contains-point-ne', PR, '        self.point == *p', '        self.point != *p',
  'P-PRCEL-03', 'pr_cel::p_prcel_03', '`PointRegion::contains_point` is `!=`')
m(951, 'pr-contains-cell-true', PR, '    fn contains_cell(&self, _cell: &Cell) -> bool {\n        false', '    fn contains_cell(&self, _cell: &Cell) -> bool {\n        true',
  'P-PRCEL-01, P-PRREG-01', 'pr_cel::p_prcel_01,pr_reg::p_prreg_01', '`PointRegion::contains_cell` is always true')
m(952, 'pr-intersects-true', PR, '        Cell::contains_point(*cell, self.point)', '        true',
  'P-PRCEL-02', 'pr_cel::p_prcel_02', '`PointRegion::intersects_cell` is always true')
m(953, 'pr-rect-bound-default', PR, '        let ll = LatLng::from_point(self.point);', '        let ll = LatLng::default();',
  'P-PRBND-02, P-PRREG-05', 'pr_bnd::p_prbnd_02,pr_reg::p_prreg_05', '`PointRegion::rect_bound` is the bound of `(0, 0)`')
m(954, 'pr-cap-bound-fixed', PR, '        Cap::from_point(self.point)', '        Cap::from_point(Point::from_coords(1.0, 0.0, 0.0))',
  'P-PRBND-01, P-PRREG-04', 'pr_bnd::p_prbnd_01,pr_reg::p_prreg_04', '`PointRegion::cap_bound` is the cap of `(1, 0, 0)`')
m(955, 'pr-point-normalised', PR, '    pub fn point(&self) -> Point {\n        self.point', '    pub fn point(&self) -> Point {\n        self.point.normalize()',
  'P-PRCON-01', 'pr_con::p_prcon_01', '`PointRegion::point` returns a normalised copy')
m(956, 'pr-cell-union-bound-empty-cap', PR, '    fn cell_union_bound(&self) -> Vec<CellId> {\n        self.cap_bound().cell_union_bound()', '    fn cell_union_bound(&self) -> Vec<CellId> {\n        Cap::empty().cell_union_bound()',
  'P-PRBND-03, P-PRREG-06', 'pr_bnd::p_prbnd_03,pr_reg::p_prreg_06', '`PointRegion::cell_union_bound` is the empty cap\'s covering', ('Tier: slow',))
m(957, 'pr-eq-xy-only', PR,
  '#[derive(Clone, Copy, Debug, PartialEq)]\n#[cfg_attr(feature = "serde", derive(serde::Serialize))]\npub struct PointRegion {\n    point: Point,\n}\n',
  '#[derive(Clone, Copy, Debug)]\n#[cfg_attr(feature = "serde", derive(serde::Serialize))]\npub struct PointRegion {\n    point: Point,\n}\n\nimpl PartialEq for PointRegion {\n    fn eq(&self, other: &Self) -> bool {\n        self.point.0.x == other.point.0.x && self.point.0.y == other.point.0.y\n    }\n}\n',
  'P-PREQ-01', 'pr_eq::p_preq_01', 'a hand-written `PartialEq` comparing `x` and `y` only')
m(958, 'cu-contains-point-parent', CU, '        self.contains_cell_id(CellId::from_point(&p))', '        self.contains_cell_id(CellId::from_point(&p).parent())',
  'P-UPT-01', 'uf_pt::p_upt_01__n1', '`CellUnion::contains_point` tests the leaf\'s parent')
m(959, 'cu-rect-bound-skips-first', CU, '        for &id in &self.0 {\n            bound = bound.union(', '        for &id in self.0.iter().skip(1) {\n            bound = bound.union(',
  'P-UBND-01', 'uf_bnd::p_ubnd_01__n1', '`CellUnion::rect_bound` skips the first cell')
m(960, 'cu-cap-bound-centres', CU, '            cap = cap.add_cap(Cell::from_cell_id(id).cap_bound());', '            cap = cap.add_cap(Cap::from_point(id.to_point()));',
  'P-UBND-02', 'uf_bnd::p_ubnd_02__n1', '`CellUnion::cap_bound` adds each cell\'s centre, not its cap bound', ('Tier: slow', 'Test: core --test cpp_region_diff'))
m(961, 'cu-cap-bound-empty-full', CU, '            return Cap::empty();', '            return Cap::full();',
  'P-UBND-03', 'uf_bnd::p_ubnd_03', 'the empty union\'s cap bound is the full cap')
m(962, 'cu-average-area-level-29', CU, '        Cell::average_area_for_level(Level::MAX) * self.leaf_cells_covered() as f64', '        Cell::average_area_for_level(Level::new(29)) * self.leaf_cells_covered() as f64',
  'P-UAREA-01', 'uf_area::p_uarea_01__n1', '`average_based_area` uses the level-29 area')
m(963, 'cu-approx-area-skips-first', CU, '            .fold(0.0, |area, &id| area + Cell::from_cell_id(id).approx_area())', '            .skip(1)\n            .fold(0.0, |area, &id| area + Cell::from_cell_id(id).approx_area())',
  'P-UAREA-02', 'uf_area::p_uarea_02__n1', '`approx_area` skips the first cell')
m(964, 'fr018-reverted', CU, '            .fold(0.0, |area, &id| area + Cell::from_cell_id(id).approx_area())', '            .map(|&id| Cell::from_cell_id(id).approx_area())\n            .sum()',
  'P-UAREA-02', 'uf_area::p_uarea_02__n0', 'FR-018 reverted: `approx_area` sums from −0 (K2)', ('Test: core --lib s2::cell_union::cell_union_tests::empty_union_areas_are_positive_zero',))
m(965, 'cu-exact-area-is-approx', CU, '            .fold(0.0, |area, &id| area + Cell::from_cell_id(id).exact_area())', '            .fold(0.0, |area, &id| area + Cell::from_cell_id(id).approx_area())',
  'P-UAREA-03', '', '`exact_area` sums the approximate areas (killed by the C++ rows: P-UAREA-03\'s harness stubs only `exact_area`, so the mutant runs the real `approx_area`, which timed out)', ('Test: core --test cpp_region_diff',))
m(966, 'cu-intersects-negated', CU, '    fn intersects_cell(&self, cell: &Cell) -> bool {\n        self.intersects_cell_id(cell.id())', '    fn intersects_cell(&self, cell: &Cell) -> bool {\n        !self.intersects_cell_id(cell.id())',
  'P-UREG-01', 'uf_reg::p_ureg_01__n1', '`Region for CellUnion::intersects_cell` is negated')
m(967, 'fr017-reverted', CELL, 'const POLE_MIN_LAT: f64 = 0.615_479_708_670_387_3;', 'const POLE_MIN_LAT: f64 = 0.615_479_708_670_387;',
  'FR-017', '', 'FR-017 reverted: Go\'s 15-digit literal for the polar latitude (K1)', ('Test: core --lib s2::cell::cell_tests::pole_min_lat_tests', 'Test: core --test cpp_region_diff'))
m(968, 'fr019-reverted', PR, '        if !f.point.is_unit() {', '        if false {',
  'P-PRDES-01', '', 'FR-019 reverted: the deserialiser accepts any point (K3)', ('Test: core --features serde --lib s2::point_region::point_region_tests',))
m(969, 'spec-pr-contains-z-ne', SPEC, '    p.0.x == q.0.x && p.0.y == q.0.y && p.0.z == q.0.z', '    p.0.x == q.0.x && p.0.y == q.0.y && p.0.z != q.0.z',
  'P-PRCEL-03', 'pr_cel::p_prcel_03', 'spec mutant: `pr_contains` compares `z` with `!=`')
m(970, 'stub-pure-leaf-not-pure', STUBS,
  '    s2rst::s2::CellId(\n        LEAF.lock()\n            .unwrap()\n            .get(bits3(*p), || crate::domain::any_valid_at(30).0.id()),\n    )',
  '    let _ = bits3(*p);\n    crate::domain::any_valid_at(30).0',
  'P-UPT-01', 'uf_pt::p_upt_01__n1', 'stub mutant: `pure_leaf` returns a fresh leaf on every call')
m(971, 'cell-contains-no-margin', CELL, '        let margin = (5.0 / 3.0) * f64::EPSILON;', '        let margin = 0.0;',
  'P-PRREG-03', '', '`Cell::contains_point` drops the u-v margin, so a cell can reject its own vertex (P-PRREG-03 is a tool limit: its harnesses time out, so the vertex rows and the twin kill it)', ('Test: core --all-features --test cpp_region_diff', 'Test: core --all-features --test trait_property_tests region_point_region'))
m(972, 'cap-add-cap-shrinks', CAP, '        let new_rad = dist.plus_error((2.0 * f64::EPSILON + 2.02 * f64::EPSILON) * dist.length2());\n        if new_rad > self.radius {', '        let new_rad = dist.plus_error((2.0 * f64::EPSILON + 2.02 * f64::EPSILON) * dist.length2());\n        if new_rad < self.radius {',
  'L-PR-02', 'uf_bnd::l_pr_02', '`Cap::add_cap` takes the smaller radius')
m(973, 'max-level-no-nan-check', MET, '        debug_assert!(!val.is_nan(), "max_level: value must not be NaN");\n        if val <= 0.0 || val.is_nan() {\n            return Level::MAX;', '        if val <= 0.0 || val.is_nan() {\n            return Level::MAX;',
  'N-CellUnion::expand_by_radius', 'uf_exp::n_expand_by_radius_nan', 'the metric\'s level lookup drops its NaN check, so `expand_by_radius` returns on a NaN radius', ('Test: core --lib s2::cell_union::cell_union_tests::expand_by_radius_panics_on_a_nan_radius',))
for (num, slug, path, old, new, targets, expect, defect, extra) in M:
    src = open(os.path.join(repo, path)).read()
    n = src.count(old)
    if n != 1:
        sys.exit(f'M-{num}: {n} matches of the old string in {path}')
    dst = src.replace(old, new)
    diff = difflib.unified_diff(src.splitlines(True), dst.splitlines(True), f'a/{path}', f'b/{path}', n=3)
    body = ''.join(diff)
    hdr = f'Mutant: M-{num}\nTargets: {targets}\nExpect-Fail: {expect}\n'
    for e in extra:
        hdr += e + '\n'
    hdr += f'Defect: {defect}\n\n'
    with open(os.path.join(outdir, f'M-{num}-{slug}.patch'), 'w') as f:
        f.write(hdr + f'diff --git a/{path} b/{path}\n' + body)
print(len(M), 'patches')
