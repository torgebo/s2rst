# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
# Written for this crate (not ported from upstream S2).
"""Compare the spec 010 survey outputs line by line, per section.

Usage: python3 -I compare.py rust/survey_rust.txt cpp/survey_cpp.txt
"""

import math
import struct
import sys
from collections import Counter, defaultdict


def f(h):
    return struct.unpack(">d", bytes.fromhex(h))[0]


def main(rust_path, cpp_path):
    with open(rust_path, encoding="utf-8") as fr, open(cpp_path, encoding="utf-8") as fc:
        rust = fr.read().splitlines()
        cpp = fc.read().splitlines()
    assert len(rust) == len(cpp), (len(rust), len(cpp))
    total = Counter()
    diff = defaultdict(list)
    for r, c in zip(rust, cpp):
        tag = r.split(" ", 1)[0]
        assert tag == c.split(" ", 1)[0]
        total[tag] += 1
        if r != c:
            diff[tag].append((r, c))
    for tag in total:
        print(f"{tag}: {len(diff[tag])} of {total[tag]} differ")
    # Normalized(): classify the differences.
    invalid = 0
    sign_only = 0
    pi_flip = 0
    inexact = 0
    for r, c in diff["N"]:
        lr, lc = f(r.split()[3]), f(c.split()[3])
        if abs(lr) > math.pi:
            invalid += 1
        if lr == lc:
            sign_only += 1
        elif abs(lr) == math.pi and lr == -lc:
            pi_flip += 1
        else:
            inexact += 1
    print(
        f"N detail: {invalid} library results outside [-pi, pi]; {pi_flip} +pi/-pi flips; "
        f"{sign_only} differ only in the sign of zero; {inexact} other values"
    )
    for r, c in diff["N"][:12]:
        print("  lib", r, "| c++", c)
    for tag in ("NL", "P", "PS", "L", "LS", "D", "DS", "S", "R", "G", "V"):
        for r, c in diff[tag][:5]:
            print("  lib", r, "| c++", c)


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
