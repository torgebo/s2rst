#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
# Written for this crate (not ported from upstream S2).
"""Writes spec 012's mutant patches (verify/mutants/M-97N-*, M-98N-*) from exact-string edits."""
import difflib, os, sys
repo = '/home/t/sources/s2rst_multi/3'
outdir = os.path.join(repo, 'verify/mutants')
RU = 'core/src/s2/region_union.rs'
RI = 'core/src/s2/region_intersection.rs'
AR = 'verify/src/any_region.rs'
M = []
def m(num, slug, path, old, new, targets, expect, defect, extra=()):
    M.append((num, slug, path, old, new, targets, expect, defect, extra))

m(974, 'ru-contains-cell-all', RU, '            if region.contains_cell(cell) {\n                return true;', '            if !region.contains_cell(cell) {\n                return false;',
  'P-ADTST-01', 'ad_mean::p_adtst_01__n1', '`RegionUnion::contains_cell` is `all`-like: false at the first member that does not contain')
m(975, 'ru-intersects-is-contains', RU, '            if region.intersects_cell(cell) {', '            if region.contains_cell(cell) {',
  'P-ADTST-01', 'ad_mean::p_adtst_01__n1', '`RegionUnion::intersects_cell` asks `contains_cell`')
m(976, 'ru-contains-point-skips-first', RU, '        for region in &self.regions {\n            if region.contains_point(p) {', '        for region in self.regions.iter().skip(1) {\n            if region.contains_point(p) {',
  'P-ADTST-01', 'ad_mean::p_adtst_01__n1', '`RegionUnion::contains_point` skips the first member')
m(977, 'ru-rect-from-full', RU, '        let mut ret = Rect::empty();', '        let mut ret = Rect::full();',
  'P-ADBND-03', 'ad_mean::p_adbnd_03', '`RegionUnion::rect_bound` folds from the full rectangle')
m(978, 'ri-contains-point-any', RI, '        self.regions.iter().all(|r| r.contains_point(p))', '        self.regions.iter().any(|r| r.contains_point(p))',
  'P-ADTST-01', 'ad_mean::p_adtst_01__n2', '`RegionIntersection::contains_point` is `any`')
m(979, 'ri-rect-skips-first', RI, '        for region in &self.regions {\n            result = result.intersection(region.rect_bound());', '        for region in self.regions.iter().skip(1) {\n            result = result.intersection(region.rect_bound());',
  'P-ADBND-01', '', '`RegionIntersection::rect_bound` skips the first member (a larger rectangle still holds every common point, which P-ADBND-01 states since R0c; the C++ rows compare the bound bit for bit)', ('Test: core --all-features --test cpp_cell_bounds_diff',))
m(980, 'ri-region-no-panic', RI, '        self.regions[i].as_ref()', '        self.regions.get(i).unwrap_or(&self.regions[0]).as_ref()',
  'N-RegionIntersection::region', 'ad_crash::n_region_past_end', '`region(i)` returns the first member past the end', ('Test: core --lib s2::region_intersection::tests::region_panics_past_the_end',))
m(981, 'ru-from-regions-drops-last', RU, '    pub fn from_regions(regions: Vec<Box<dyn Region>>) -> Self {\n        RegionUnion { regions }', '    pub fn from_regions(regions: Vec<Box<dyn Region>>) -> Self {\n        let mut regions = regions;\n        regions.pop();\n        RegionUnion { regions }',
  'C-RegionUnion::from_regions', 'ad_crash::c_union_api__n1', '`RegionUnion::from_regions` drops the last member')
m(982, 'ru-add-at-front', RU, '        self.regions.push(region);', '        self.regions.insert(0, region);',
  'P-ADCON-01', 'ad_mean::p_adcon_01', '`RegionUnion::add` inserts at the front')
m(983, 'ru-intersects-negated', RU, '            if region.intersects_cell(cell) {', '            if !region.intersects_cell(cell) {',
  'P-ADREG-01', 'ad_reg::p_adreg_01__n1', '`RegionUnion::intersects_cell` asks the negation')
m(984, 'ri-intersects-any', RI, '        self.regions.iter().all(|r| r.intersects_cell(cell))', '        self.regions.iter().any(|r| r.intersects_cell(cell))',
  'P-ADTST-01', 'ad_mean::p_adtst_01__n2', '`RegionIntersection::intersects_cell` is `any`')
m(985, 'stub-any-region-point-not-pure', AR, '''        let b = bits3(*p);
        CONTAINS_POINT
            .lock()
            .unwrap()
            .get([u64::from(self.id), b[0], b[1], b[2]], any_bool)
            == 1''', '''        let _ = (bits3(*p), &CONTAINS_POINT);
        any_bool() == 1''',
  'P-ADTST-01', 'ad_mean::p_adtst_01__n1', 'stub mutant: the abstract region answers `contains_point` afresh on every call')
m(986, 'ru-contains-point-default-true', RU, '            if region.contains_point(p) {\n                return true;\n            }\n        }\n        false', '            if region.contains_point(p) {\n                return true;\n            }\n        }\n        true',
  'P-ADREG-02', 'ad_reg::p_adreg_02__n1', '`RegionUnion::contains_point` is true when no member contains the point')
m(987, 'ru-rect-skips-first', RU, '        for region in &self.regions {\n            ret = ret.union(region.rect_bound());', '        for region in self.regions.iter().skip(1) {\n            ret = ret.union(region.rect_bound());',
  'P-ADREG-03', 'ad_reg::p_adreg_03__n1', '`RegionUnion::rect_bound` skips the first member, so a point of that member can fall outside it')

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
