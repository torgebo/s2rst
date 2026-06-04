// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).
//
// Targeted tests for the exact-arithmetic / symbolic-perturbation fallback
// paths of the robust predicates — the deepest, rarely-hit branches that random
// inputs almost never exercise. Each test crafts an exactly-degenerate input so
// the fast and exact stages tie and the Simulation-of-Simplicity fallback must
// decide.

use super::*;

/// Three distinct points that are exactly collinear (all on the `z = 0` great
/// circle) make the exact orientation determinant exactly zero, forcing
/// `robust_sign` into `symbolically_perturbed_sign`. The result must be a
/// definite, antisymmetric direction — never `Indeterminate` for distinct
/// points.
#[test]
fn robust_sign_symbolic_on_exact_collinear() {
    let a = Point::from_coords(1.0, 0.0, 0.0);
    let b = Point::from_coords(0.0, 1.0, 0.0);
    let c = Point::from_coords(1.0, 1.0, 0.0); // distinct, also on z = 0
    let abc = robust_sign(a, b, c);
    assert_ne!(
        abc,
        Direction::Indeterminate,
        "symbolic perturbation must break the exact-collinear tie"
    );
    // Swapping the first two arguments flips the orientation.
    assert_eq!(robust_sign(b, a, c), -abc);
}

/// A point exactly equidistant (90°) from two distinct others drives
/// `compare_distances` past its triage and exact stages into
/// `symbolic_compare_distances`, which must return a definite, antisymmetric
/// ordering — never `0` for distinct points.
#[test]
fn compare_distances_symbolic_tiebreak() {
    let x = Point::from_coords(1.0, 0.0, 0.0);
    let a = Point::from_coords(0.0, 1.0, 0.0);
    let b = Point::from_coords(0.0, 0.0, 1.0);
    // a and b are both exactly orthogonal to x, so the exact comparison ties.
    let ab = compare_distances(x, a, b);
    assert_ne!(
        ab, 0,
        "symbolic tie-break must not return 0 for distinct points"
    );
    assert_eq!(ab, -compare_distances(x, b, a), "must be antisymmetric");
}
