# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
# Written for this crate (not ported from upstream S2).
"""Compare the spec 011 survey outputs line by line, per section.

Usage: python3 -I compare.py rust/survey_rust.txt cpp/survey_cpp.txt
"""

import sys
from collections import Counter, defaultdict


def main(rust_path, cpp_path):
    with open(rust_path, encoding="utf-8") as fr, open(cpp_path, encoding="utf-8") as fc:
        rust = fr.read().splitlines()
        cpp = fc.read().splitlines()
    assert len(rust) == len(cpp), (len(rust), len(cpp))
    total = Counter()
    diff = defaultdict(list)
    for r, c in zip(rust, cpp):
        tag = r.split(" ", 1)[0]
        assert tag == c.split(" ", 1)[0], (r[:40], c[:40])
        total[tag] += 1
        if r != c:
            diff[tag].append((r, c))
    for tag in total:
        print(f"{tag}: {len(diff[tag])} of {total[tag]} differ")
    for tag in total:
        for r, c in diff[tag][:5]:
            print("  lib", r[:200], "| c++", c[:200])


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
