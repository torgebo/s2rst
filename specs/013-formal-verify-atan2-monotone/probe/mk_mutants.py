#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
# Written for this crate (not ported from upstream S2).
"""Writes spec 013's mutant patches (verify/mutants/M-988-* … M-992-*) from exact-string edits."""
import difflib, os, sys
repo = '/home/t/sources/s2rst_multi/3'
outdir = os.path.join(repo, 'verify/mutants')
CELL = 'core/src/s2/cell.rs'
M = []
def m(num, slug, path, old, new, targets, expect, defect, extra=()):
    M.append((num, slug, path, old, new, targets, expect, defect, extra))

m(988, 'cell-rect-u-corner-inverted', CELL, '                if u < 0.0 { 1 } else { 0 }', '                if u < 0.0 { 0 } else { 1 }',
  'P-CREG-02', 'cr_reg::p_creg_02__lat', '`Cell::rect_bound` picks the u-axis corner the wrong way round')
m(989, 'cell-rect-lat-same-v', CELL, '.add_point(self.latitude(1 - i, 1 - j));', '.add_point(self.latitude(1 - i, j));',
  'P-CREG-02', 'cr_reg::p_creg_02__lat', "`Cell::rect_bound`'s second latitude corner shares the first's v edge")
m(990, 'cell-rect-lng-same-v', CELL, '.add_point(self.longitude(i, 1 - j))', '.add_point(self.longitude(i, j))',
  'P-CREG-02', 'cr_reg::p_creg_02__lng_f2', "`Cell::rect_bound`'s first longitude corner moves to the other v edge (on a polar face, where the longitude depends on v)", ('Tier: slow',))
m(991, 'cell-rect-no-polar-closure', CELL, '.expanded(LatLng::from_radians(eps2, eps2))\n                .polar_closure();', '.expanded(LatLng::from_radians(eps2, eps2));',
  'P-CREG-02', 'cr_reg::p_creg_02__lng_f2', '`Cell::rect_bound` drops `polar_closure`, so a cell with a pole corner gets two corners\' longitudes only', ('Tier: slow',))
m(992, 'cell-rect-face1-lng-from-half-pi', CELL, '                crate::s1::Interval::new(FRAC_PI_4, 3.0 * FRAC_PI_4),', '                crate::s1::Interval::new(FRAC_PI_2, 3.0 * FRAC_PI_4),',
  'P-CREG-02', 'cr_reg::p_creg_02__l0', "face 1's fixed longitude bound starts at π/2")

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
