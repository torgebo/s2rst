#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
# Written for this crate (not ported from upstream S2).
"""Writes spec 014's mutant patches (verify/mutants/M-993-* … M-998-*) from exact-string edits."""
import difflib, os, sys
repo = '/home/t/sources/s2rst_multi/3'
outdir = os.path.join(repo, 'verify/mutants')
ENC = 'core/src/s2/encoding.rs'
M = []
def m(num, slug, path, old, new, targets, expect, defect, extra=()):
    M.append((num, slug, path, old, new, targets, expect, defect, extra))

m(993, 'point-region-version-0-only', ENC, 'if version > ENCODING_VERSION {\n            return Err(io::Error::new(\n                io::ErrorKind::InvalidData,\n                "unsupported point region', 'if version > 0 {\n            return Err(io::Error::new(\n                io::ErrorKind::InvalidData,\n                "unsupported point region',
  'P-FX-13', 'cd_fx::p_fx_13__b25', "`PointRegion`'s decoder rejects version 1, its own")
m(994, 'point-region-no-unit-check', ENC, '        if !point.is_unit() {', '        if false && !point.is_unit() {',
  'P-FX-13', 'cd_fx::p_fx_13__b25', "`PointRegion`'s decoder accepts a point off unit length")
m(995, 'point-region-unit-check-inverted', ENC, '        if !point.is_unit() {', '        if point.is_unit() {',
  'P-FX-11', 'cd_fx::p_fx_11', "`PointRegion`'s decoder rejects exactly the unit points")
m(996, 'point-region-encodes-version-0', ENC, '        w.write_all(&[ENCODING_VERSION])?;\n        self.point().encode(w)', '        w.write_all(&[0])?;\n        self.point().encode(w)',
  'P-FX-11', 'cd_fx::p_fx_11_enc', "`PointRegion`'s encoder writes version 0 (upstream reads it, but writes 1)")
m(997, 'latlng-encode-swapped', ENC, '        write_f64(w, self.lat.radians())?;\n        write_f64(w, self.lng.radians())', '        write_f64(w, self.lng.radians())?;\n        write_f64(w, self.lat.radians())',
  'P-FX-12', 'cd_fx::p_fx_12_enc', "`LatLng`'s encoder writes the longitude first")
m(998, 'latlng-decode-lng-twice', ENC, '        Ok(LatLng::from_radians(lat, lng))', '        Ok(LatLng::from_radians(lng, lng))',
  'P-FX-12', 'cd_fx::p_fx_12', "`LatLng`'s decoder puts the longitude in both fields")

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
