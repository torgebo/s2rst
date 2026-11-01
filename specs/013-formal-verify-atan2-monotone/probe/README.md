# Spec 013 survey (2026-11-01)

Two standalone crates (each its own `[workspace]`), run with `cargo run --release`:

- `mono/`: is `f64::atan2` monotone in the exact angle of its arguments? Near-tie pairs in the
  right half-plane, each argument within four ulps of the other's; the exact order of `y₁/x₁` and
  `y₂/x₂` decided in big integers. `cargo run --release -- 20000000`:
  `pairs 20000000 violations 0 worst 0 ulp` (K1).
- `survey/`: 400,000 cells at every face and level (six face cells), and for each its corners,
  edge midpoints, centre and four random interior points: is the point inside `Cell::rect_bound`,
  as the direction `face_uv_to_xyz(face, u, v)` and normalised? Does a cell at level 1 or deeper
  straddle `u = 0` or `v = 0`? Then 20,000,000 random pairs in the four quadrants, of which those
  ordered by dominance (first quadrant: `y₁ ≤ y₂` and `x₁ ≥ x₂`; the mirror images) are checked:
  `cells 400000 straddling (level>=1) 0 points 5200000 outside as directions 0 outside normalised 0`,
  `dominance pairs 8335328 violations 0` (K1, K2, K3).

Platform: x86-64 Linux, glibc's `atan2`, Rust 1.9x release build.
