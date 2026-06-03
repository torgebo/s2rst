// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry

//! Dynamic Time Warping (DTW) based vertex alignment between polylines.
//!
//! Computes optimal vertex pairings between two polylines, minimizing the
//! sum of chordal distances. Provides exact (O(n*m)) and approximate
//! (`FastDTW`, O(max(n,m))) algorithms.
//!
//! Also provides multi-sequence operations: medoid (representative polyline)
//! and consensus (weighted average polyline via DBA).
//!
//! Corresponds to C++ `s2polyline_alignment.h/cc`.

#![expect(
    clippy::cast_sign_loss,
    reason = "row/col indices (i64) used as Vec indices after bounds checking"
)]
#![expect(
    clippy::cast_possible_truncation,
    reason = "DTW row/col (i64->usize) after bounds checks, f64->usize for scaling"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "usize -> i64 for DTW matrix indices — always fits"
)]
use crate::s2::Point;
use crate::s2::polyline::Polyline;

/// A warp path entry: (index into polyline a, index into polyline b).
pub type WarpPath = Vec<(usize, usize)>;

/// Result of a vertex alignment between two polylines.
#[derive(Clone, Debug, PartialEq)]
pub struct VertexAlignment {
    /// Sum of chordal distances between paired vertices.
    pub alignment_cost: f64,
    /// Sequence of paired vertex indices.
    pub warp_path: WarpPath,
}

// ─── Internal: Window for windowed DTW ────────────────────────────────

/// A column stride (start..end range) for a single row in the search window.
#[derive(Clone, Copy, Debug)]
struct ColumnStride {
    start: usize,
    end: usize,
}

impl ColumnStride {
    fn in_range(&self, col: usize) -> bool {
        col >= self.start && col < self.end
    }
}

/// Sparse binary search window for windowed DTW.
struct Window {
    strides: Vec<ColumnStride>,
    rows: usize,
    cols: usize,
}

impl Window {
    /// Creates a window from column strides.
    fn from_strides(strides: Vec<ColumnStride>) -> Self {
        assert!(!strides.is_empty(), "Cannot construct empty window");
        assert_eq!(strides[0].start, 0, "First stride must start at 0");
        let rows = strides.len();
        let cols = strides[strides.len() - 1].end;
        let w = Window {
            strides,
            rows,
            cols,
        };
        debug_assert!(w.is_valid(), "Window constructor validity check fail");
        w
    }

    /// Creates a window from a warp path.
    fn from_warp_path(path: &WarpPath) -> Self {
        assert!(!path.is_empty(), "Cannot construct window from empty path");
        assert_eq!(path[0], (0, 0), "Warp path must start at (0,0)");
        let rows = path[path.len() - 1].0 + 1;
        let cols = path[path.len() - 1].1 + 1;
        let mut strides = vec![ColumnStride { start: 0, end: 0 }; rows];

        let mut prev_row = 0;
        let mut stride_start = 0;
        let mut stride_stop = 0;
        for &(r, c) in path {
            if r > prev_row {
                strides[prev_row] = ColumnStride {
                    start: stride_start,
                    end: stride_stop,
                };
                stride_start = c;
                prev_row = r;
            }
            stride_stop = c + 1;
        }
        strides[rows - 1] = ColumnStride {
            start: stride_start,
            end: stride_stop,
        };
        let w = Window {
            strides,
            rows,
            cols,
        };
        debug_assert!(w.is_valid(), "Window constructor validity check fail");
        w
    }

    /// Returns true if this window's data represents a valid window.
    fn is_valid(&self) -> bool {
        if self.rows == 0
            || self.cols == 0
            || self.strides[0].start != 0
            || self.strides[self.rows - 1].end != self.cols
        {
            return false;
        }
        let mut prev = ColumnStride {
            start: usize::MAX,
            end: usize::MAX,
        };
        // Use wrapping comparison: prev starts at MAX so first stride always passes.
        let mut first = true;
        for s in &self.strides {
            if s.end <= s.start {
                return false;
            }
            if !first && (s.start < prev.start || s.end < prev.end) {
                return false;
            }
            prev = *s;
            first = false;
        }
        true
    }

    fn stride(&self, row: usize) -> ColumnStride {
        self.strides[row]
    }

    fn checked_stride(&self, row: i64) -> ColumnStride {
        if row < 0 || row as usize >= self.rows {
            ColumnStride { start: 0, end: 0 }
        } else {
            self.strides[row as usize]
        }
    }

    /// Upsamples the window to new dimensions.
    fn upsample(&self, new_rows: usize, new_cols: usize) -> Window {
        assert!(new_rows >= self.rows);
        assert!(new_cols >= self.cols);
        let row_scale = new_rows as f64 / self.rows as f64;
        let col_scale = new_cols as f64 / self.cols as f64;
        let mut new_strides = vec![ColumnStride { start: 0, end: 0 }; new_rows];
        for (row, stride) in new_strides.iter_mut().enumerate() {
            let from = self.strides[((row as f64 + 0.5) / row_scale) as usize];
            *stride = ColumnStride {
                start: (col_scale * from.start as f64 + 0.5) as usize,
                end: (col_scale * from.end as f64 + 0.5) as usize,
            };
        }
        Window::from_strides(new_strides)
    }

    /// Returns a debug string showing the window as a grid.
    #[cfg(test)]
    fn debug_string(&self) -> String {
        let mut out = String::new();
        for row in 0..self.rows {
            let s = &self.strides[row];
            for col in 0..self.cols {
                if col > 0 {
                    out.push(' ');
                }
                if col >= s.start && col < s.end {
                    out.push('*');
                } else {
                    out.push('.');
                }
            }
            out.push('\n');
        }
        out
    }

    /// Dilates the window by the given radius.
    fn dilate(&self, radius: usize) -> Window {
        let mut new_strides = vec![ColumnStride { start: 0, end: 0 }; self.rows];
        for (row, stride) in new_strides.iter_mut().enumerate() {
            let prev_row = row.saturating_sub(radius);
            let next_row = std::cmp::min(row + radius, self.rows - 1);
            let start = self.strides[prev_row].start.saturating_sub(radius);
            let end = std::cmp::min(self.strides[next_row].end + radius, self.cols);
            *stride = ColumnStride { start, end };
        }
        Window::from_strides(new_strides)
    }
}

// ─── Internal: DTW cost table lookup ──────────────────────────────────

fn bounds_checked_cost(row: i64, col: i64, stride: &ColumnStride, table: &[Vec<f64>]) -> f64 {
    if row < 0 && col < 0 {
        0.0
    } else if row < 0 || col < 0 || !stride.in_range(col as usize) {
        f64::MAX
    } else {
        table[row as usize][col as usize]
    }
}

/// Chordal distance between two points (= Norm of difference vector).
fn chordal_distance(a: Point, b: Point) -> f64 {
    (a.0 - b.0).norm()
}

/// Core DTW routine: fills DP table within the window and recovers warp path.
fn dynamic_timewarp(a: &Polyline, b: &Polyline, w: &Window) -> VertexAlignment {
    let rows = a.num_vertices();
    let cols = b.num_vertices();
    let mut costs = vec![vec![0.0f64; cols]; rows];

    let mut prev = ColumnStride { start: 0, end: 0 };
    // Treat the "row before 0" as having an all-zero stride
    // (bounds_checked_cost handles negative indices).
    // For row 0, prev is effectively empty except that (-1,-1) returns 0.0.
    let sentinel = ColumnStride {
        start: 0,
        end: cols,
    };
    let _ = sentinel;

    for row in 0..rows {
        let curr = w.stride(row);
        for col in curr.start..curr.end {
            let d_cost = bounds_checked_cost(row as i64 - 1, col as i64 - 1, &prev, &costs);
            let u_cost = bounds_checked_cost(row as i64 - 1, col as i64, &prev, &costs);
            let l_cost = bounds_checked_cost(row as i64, col as i64 - 1, &curr, &costs);
            costs[row][col] =
                d_cost.min(u_cost).min(l_cost) + chordal_distance(a.vertex(row), b.vertex(col));
        }
        prev = curr;
    }

    // Walk back through the cost table to recover the warp path.
    let mut warp_path = Vec::with_capacity(std::cmp::max(rows, cols));
    let mut row = rows as i64 - 1;
    let mut col = cols as i64 - 1;
    let mut curr = w.checked_stride(row);
    let mut prev_s = w.checked_stride(row - 1);
    while row >= 0 && col >= 0 {
        warp_path.push((row as usize, col as usize));
        let d_cost = bounds_checked_cost(row - 1, col - 1, &prev_s, &costs);
        let u_cost = bounds_checked_cost(row - 1, col, &prev_s, &costs);
        let l_cost = bounds_checked_cost(row, col - 1, &curr, &costs);
        if d_cost <= u_cost && d_cost <= l_cost {
            row -= 1;
            col -= 1;
            curr = w.checked_stride(row);
            prev_s = w.checked_stride(row - 1);
        } else if u_cost <= l_cost {
            row -= 1;
            curr = w.checked_stride(row);
            prev_s = w.checked_stride(row - 1);
        } else {
            col -= 1;
        }
    }
    warp_path.reverse();
    let final_cost = costs[rows - 1][cols - 1];
    VertexAlignment {
        alignment_cost: final_cost,
        warp_path,
    }
}

/// Half-resolution of a polyline: takes every other vertex.
fn half_resolution(polyline: &Polyline) -> Polyline {
    let n = polyline.num_vertices();
    let mut vertices = Vec::with_capacity(n / 2 + 1);
    let mut i = 0;
    while i < n {
        vertices.push(polyline.vertex(i));
        i += 2;
    }
    Polyline::new(vertices)
}

// ─── Public API ───────────────────────────────────────────────────────

/// Computes the exact optimal vertex alignment between two non-empty polylines.
///
/// Time and space complexity: O(A * B) where A and B are the vertex counts.
///
/// # Panics
///
/// Panics if either polyline is empty.
pub fn get_exact_vertex_alignment(a: &Polyline, b: &Polyline) -> VertexAlignment {
    let a_n = a.num_vertices();
    let b_n = b.num_vertices();
    assert!(a_n > 0, "Polyline A is empty");
    assert!(b_n > 0, "Polyline B is empty");
    let strides: Vec<ColumnStride> = vec![ColumnStride { start: 0, end: b_n }; a_n];
    let w = Window::from_strides(strides);
    dynamic_timewarp(a, b, &w)
}

/// Computes only the cost of the optimal alignment (space-efficient O(max(A,B))).
///
/// # Panics
///
/// Panics if either polyline is empty.
pub fn get_exact_vertex_alignment_cost(a: &Polyline, b: &Polyline) -> f64 {
    let a_n = a.num_vertices();
    let b_n = b.num_vertices();
    assert!(a_n > 0, "Polyline A is empty");
    assert!(b_n > 0, "Polyline B is empty");
    let mut cost = vec![f64::MAX; b_n];
    let mut left_diag_min;
    for row in 0..a_n {
        left_diag_min = if row == 0 { 0.0 } else { f64::MAX };
        for (col, cell) in cost.iter_mut().enumerate() {
            let up_cost = *cell;
            *cell = left_diag_min.min(up_cost) + chordal_distance(a.vertex(row), b.vertex(col));
            left_diag_min = (*cell).min(up_cost);
        }
    }
    cost[b_n - 1]
}

/// Computes an approximate optimal alignment using the `FastDTW` algorithm.
///
/// The `radius` parameter controls the search window size.
/// Time and space complexity: O(max(A, B)).
///
/// # Panics
///
/// Panics if either polyline is empty.
pub fn get_approx_vertex_alignment(a: &Polyline, b: &Polyline, radius: usize) -> VertexAlignment {
    const SIZE_SWITCHOVER: usize = 32;
    const DENSITY_SWITCHOVER: f64 = 0.85;

    let a_n = a.num_vertices();
    let b_n = b.num_vertices();
    assert!(a_n > 0, "Polyline A is empty");
    assert!(b_n > 0, "Polyline B is empty");

    // If small enough, use exact algorithm.
    if a_n.saturating_sub(radius) < SIZE_SWITCHOVER || b_n.saturating_sub(radius) < SIZE_SWITCHOVER
    {
        return get_exact_vertex_alignment(a, b);
    }

    // If window would be nearly full, use exact algorithm.
    if a_n.max(b_n) * (2 * radius + 1) > ((a_n as f64 * b_n as f64 * DENSITY_SWITCHOVER) as usize) {
        return get_exact_vertex_alignment(a, b);
    }

    // Recursive half-resolution.
    let a_half = half_resolution(a);
    let b_half = half_resolution(b);
    let proj = get_approx_vertex_alignment(&a_half, &b_half, radius);
    let w = Window::from_warp_path(&proj.warp_path)
        .upsample(a_n, b_n)
        .dilate(radius);
    dynamic_timewarp(a, b, &w)
}

/// Computes approximate alignment with a default radius of max(A,B)^0.25.
pub fn get_approx_vertex_alignment_default(a: &Polyline, b: &Polyline) -> VertexAlignment {
    let max_len = a.num_vertices().max(b.num_vertices());
    let radius = (max_len as f64).powf(0.25) as usize;
    get_approx_vertex_alignment(a, b, radius)
}

/// Options for the medoid computation.
#[derive(Clone, Debug, PartialEq)]
pub struct MedoidOptions {
    /// If true, use approximate alignment; if false, use exact.
    pub approx: bool,
}

impl Default for MedoidOptions {
    fn default() -> Self {
        MedoidOptions { approx: true }
    }
}

/// Returns the index of the medoid polyline: the one that minimizes the
/// summed alignment cost to all other polylines.
///
/// # Panics
///
/// Panics if `polylines` is empty.
pub fn get_medoid_polyline(polylines: &[Polyline], options: &MedoidOptions) -> usize {
    let n = polylines.len();
    assert!(n > 0, "Empty polyline collection");

    let cost_fn = |a: &Polyline, b: &Polyline| -> f64 {
        if options.approx {
            get_approx_vertex_alignment_default(a, b).alignment_cost
        } else {
            get_exact_vertex_alignment_cost(a, b)
        }
    };

    let mut costs = vec![0.0f64; n];
    for i in 0..n {
        for j in (i + 1)..n {
            let cost = cost_fn(&polylines[i], &polylines[j]);
            costs[i] += cost;
            costs[j] += cost;
        }
    }

    costs
        .iter()
        .enumerate()
        .min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map_or(0, |(idx, _)| idx)
}

/// Options for the consensus computation.
#[derive(Clone, Debug, PartialEq)]
pub struct ConsensusOptions {
    /// If true, use approximate alignment.
    pub approx: bool,
    /// If true, seed with the medoid instead of the first polyline.
    pub seed_medoid: bool,
    /// Maximum number of DBA refining iterations.
    pub iteration_cap: usize,
}

impl Default for ConsensusOptions {
    fn default() -> Self {
        ConsensusOptions {
            approx: true,
            seed_medoid: false,
            iteration_cap: 5,
        }
    }
}

/// Returns a consensus polyline using Dynamic Time Warp Barycenter Averaging.
///
/// # Panics
///
/// Panics if `polylines` is empty.
pub fn get_consensus_polyline(polylines: &[Polyline], options: &ConsensusOptions) -> Polyline {
    let n = polylines.len();
    assert!(n > 0, "Empty polyline collection");

    let align_fn = |a: &Polyline, b: &Polyline| -> VertexAlignment {
        if options.approx {
            get_approx_vertex_alignment_default(a, b)
        } else {
            get_exact_vertex_alignment(a, b)
        }
    };

    // Seed the consensus.
    let seed_index = if options.seed_medoid {
        let medoid_opts = MedoidOptions {
            approx: options.approx,
        };
        get_medoid_polyline(polylines, &medoid_opts)
    } else {
        0
    };
    let mut consensus = polylines[seed_index].clone();
    let num_cv = consensus.num_vertices();
    debug_assert!(num_cv > 1);

    let mut converged = false;
    let mut iterations = 0;
    while !converged && iterations < options.iteration_cap {
        let mut points = vec![Point::default(); num_cv];
        for polyline in polylines {
            let alignment = align_fn(&consensus, polyline);
            for &(ci, pi) in &alignment.warp_path {
                points[ci] = Point(points[ci].0 + polyline.vertex(pi).0);
            }
        }
        // Normalize each accumulated point.
        for p in &mut points {
            *p = p.normalize();
        }

        iterations += 1;
        let new_consensus = Polyline::new(points);
        converged = new_consensus.approx_eq_with(&consensus, crate::s1::Angle::from_radians(1e-9));
        consensus = new_consensus;
    }
    consensus
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::text_format::make_polyline;

    fn verify_path(a: &Polyline, b: &Polyline, expected: &[(usize, usize)]) {
        let mut correct_cost = 0.0;
        for &(i, j) in expected {
            correct_cost += chordal_distance(a.vertex(i), b.vertex(j));
        }
        let exact_cost = get_exact_vertex_alignment_cost(a, b);
        let alignment = get_exact_vertex_alignment(a, b);
        assert!(
            (correct_cost - exact_cost).abs() < 1e-6,
            "Cost mismatch: expected {correct_cost}, got {exact_cost}"
        );
        assert!(
            (correct_cost - alignment.alignment_cost).abs() < 1e-6,
            "Alignment cost mismatch"
        );
        assert_eq!(
            alignment.warp_path.len(),
            expected.len(),
            "Path length mismatch"
        );
        for (i, (actual, exp)) in alignment.warp_path.iter().zip(expected.iter()).enumerate() {
            assert_eq!(actual, exp, "Path mismatch at {i}");
        }
    }

    fn verify_cost(a: &Polyline, b: &Polyline) {
        let exact_cost = get_exact_vertex_alignment_cost(a, b);
        let alignment = get_exact_vertex_alignment(a, b);
        assert!(
            (exact_cost - alignment.alignment_cost).abs() < 1e-6,
            "Cost-only ({exact_cost}) != alignment cost ({})",
            alignment.alignment_cost
        );
    }

    /// Brute-force DTW cost for verifying the DP solvers.
    fn brute_force_cost(table: &[Vec<f64>], i: usize, j: usize) -> f64 {
        if i == 0 && j == 0 {
            table[0][0]
        } else if i == 0 {
            brute_force_cost(table, i, j - 1) + table[i][j]
        } else if j == 0 {
            brute_force_cost(table, i - 1, j) + table[i][j]
        } else {
            let d = brute_force_cost(table, i - 1, j - 1);
            let u = brute_force_cost(table, i - 1, j);
            let l = brute_force_cost(table, i, j - 1);
            d.min(u).min(l) + table[i][j]
        }
    }

    fn distance_matrix(a: &Polyline, b: &Polyline) -> Vec<Vec<f64>> {
        let a_n = a.num_vertices();
        let b_n = b.num_vertices();
        (0..a_n)
            .map(|i| {
                (0..b_n)
                    .map(|j| chordal_distance(a.vertex(i), b.vertex(j)))
                    .collect()
            })
            .collect()
    }

    fn verify_cost_brute_force(a: &Polyline, b: &Polyline) {
        let table = distance_matrix(a, b);
        let a_n = a.num_vertices();
        let b_n = b.num_vertices();
        let brute_cost = brute_force_cost(&table, a_n - 1, b_n - 1);
        let exact_cost = get_exact_vertex_alignment_cost(a, b);
        let alignment = get_exact_vertex_alignment(a, b);
        assert!(
            (brute_cost - exact_cost).abs() < 1e-6,
            "Brute force cost ({brute_cost}) != exact cost ({exact_cost})"
        );
        assert!(
            (brute_cost - alignment.alignment_cost).abs() < 1e-6,
            "Brute force cost ({brute_cost}) != alignment cost ({})",
            alignment.alignment_cost
        );
    }

    // ─── Window: debug_string ─────────────────────────────────────────

    /// C++ TEST(S2PolylineAlignmentTest, `GeneratesWindowDebugString`)
    #[test]
    fn test_window_debug_string() {
        let strides = vec![
            ColumnStride { start: 0, end: 4 },
            ColumnStride { start: 0, end: 4 },
            ColumnStride { start: 0, end: 4 },
            ColumnStride { start: 0, end: 4 },
        ];
        let w = Window::from_strides(strides);
        let expected = "\
* * * *
* * * *
* * * *
* * * *
";
        assert_eq!(w.debug_string(), expected);
    }

    // ─── Window: upsample ─────────────────────────────────────────────

    /// C++ TEST(S2PolylineAlignmentTest, `UpsamplesWindowByFactorOfTwo`)
    #[test]
    fn test_upsample_by_factor_of_two() {
        let strides = vec![
            ColumnStride { start: 0, end: 3 },
            ColumnStride { start: 1, end: 4 },
            ColumnStride { start: 2, end: 4 },
            ColumnStride { start: 3, end: 6 },
            ColumnStride { start: 4, end: 6 },
        ];
        let w = Window::from_strides(strides);
        let w_up = w.upsample(10, 12);
        let expected = "\
* * * * * * . . . . . .
* * * * * * . . . . . .
. . * * * * * * . . . .
. . * * * * * * . . . .
. . . . * * * * . . . .
. . . . * * * * . . . .
. . . . . . * * * * * *
. . . . . . * * * * * *
. . . . . . . . * * * *
. . . . . . . . * * * *
";
        assert_eq!(w_up.debug_string(), expected);
    }

    /// C++ TEST(S2PolylineAlignmentTest, `UpsamplesWindowXAxisByFactorOfThree`)
    #[test]
    fn test_upsample_x_axis_by_factor_of_three() {
        let strides = vec![
            ColumnStride { start: 0, end: 3 },
            ColumnStride { start: 1, end: 4 },
            ColumnStride { start: 2, end: 4 },
            ColumnStride { start: 3, end: 6 },
            ColumnStride { start: 4, end: 6 },
        ];
        let w = Window::from_strides(strides);
        let w_up = w.upsample(5, 18);
        let expected = "\
* * * * * * * * * . . . . . . . . .
. . . * * * * * * * * * . . . . . .
. . . . . . * * * * * * . . . . . .
. . . . . . . . . * * * * * * * * *
. . . . . . . . . . . . * * * * * *
";
        assert_eq!(w_up.debug_string(), expected);
    }

    /// C++ TEST(S2PolylineAlignmentTest, `UpsamplesWindowYAxisByFactorOfThree`)
    #[test]
    fn test_upsample_y_axis_by_factor_of_three() {
        let strides = vec![
            ColumnStride { start: 0, end: 3 },
            ColumnStride { start: 1, end: 4 },
            ColumnStride { start: 2, end: 4 },
            ColumnStride { start: 3, end: 6 },
            ColumnStride { start: 4, end: 6 },
        ];
        let w = Window::from_strides(strides);
        let w_up = w.upsample(15, 6);
        let expected = "\
* * * . . .
* * * . . .
* * * . . .
. * * * . .
. * * * . .
. * * * . .
. . * * . .
. . * * . .
. . * * . .
. . . * * *
. . . * * *
. . . * * *
. . . . * *
. . . . * *
. . . . * *
";
        assert_eq!(w_up.debug_string(), expected);
    }

    /// C++ TEST(S2PolylineAlignmentTest, `UpsamplesWindowByNonInteger`)
    #[test]
    fn test_upsample_by_non_integer() {
        let strides = vec![
            ColumnStride { start: 0, end: 3 },
            ColumnStride { start: 1, end: 4 },
            ColumnStride { start: 2, end: 4 },
            ColumnStride { start: 3, end: 6 },
            ColumnStride { start: 4, end: 6 },
        ];
        let w = Window::from_strides(strides);
        let w_up = w.upsample(19, 23);
        let expected = "\
* * * * * * * * * * * * . . . . . . . . . . .
* * * * * * * * * * * * . . . . . . . . . . .
* * * * * * * * * * * * . . . . . . . . . . .
* * * * * * * * * * * * . . . . . . . . . . .
. . . . * * * * * * * * * * * . . . . . . . .
. . . . * * * * * * * * * * * . . . . . . . .
. . . . * * * * * * * * * * * . . . . . . . .
. . . . * * * * * * * * * * * . . . . . . . .
. . . . . . . . * * * * * * * . . . . . . . .
. . . . . . . . * * * * * * * . . . . . . . .
. . . . . . . . * * * * * * * . . . . . . . .
. . . . . . . . . . . . * * * * * * * * * * *
. . . . . . . . . . . . * * * * * * * * * * *
. . . . . . . . . . . . * * * * * * * * * * *
. . . . . . . . . . . . * * * * * * * * * * *
. . . . . . . . . . . . . . . * * * * * * * *
. . . . . . . . . . . . . . . * * * * * * * *
. . . . . . . . . . . . . . . * * * * * * * *
. . . . . . . . . . . . . . . * * * * * * * *
";
        assert_eq!(w_up.debug_string(), expected);
    }

    // ─── Window: dilate ───────────────────────────────────────────────

    /// C++ TEST(S2PolylineAlignmentTest, `DilatesWindowByRadiusZero`)
    #[test]
    fn test_dilate_by_radius_zero() {
        let strides = vec![
            ColumnStride { start: 0, end: 3 },
            ColumnStride { start: 2, end: 3 },
            ColumnStride { start: 2, end: 3 },
            ColumnStride { start: 2, end: 4 },
            ColumnStride { start: 3, end: 6 },
        ];
        let w = Window::from_strides(strides);
        let w_d = w.dilate(0);
        let expected = "\
* * * . . .
. . * . . .
. . * . . .
. . * * . .
. . . * * *
";
        assert_eq!(w_d.debug_string(), expected);
    }

    /// C++ TEST(S2PolylineAlignmentTest, `DilatesWindowByRadiusOne`)
    #[test]
    fn test_dilate_by_radius_one() {
        let strides = vec![
            ColumnStride { start: 0, end: 3 },
            ColumnStride { start: 2, end: 3 },
            ColumnStride { start: 2, end: 3 },
            ColumnStride { start: 2, end: 4 },
            ColumnStride { start: 3, end: 6 },
        ];
        let w = Window::from_strides(strides);
        let w_d = w.dilate(1);
        let expected = "\
* * * * . .
* * * * . .
. * * * * .
. * * * * *
. * * * * *
";
        assert_eq!(w_d.debug_string(), expected);
    }

    /// C++ TEST(S2PolylineAlignmentTest, `DilatesWindowByRadiusTwo`)
    #[test]
    fn test_dilate_by_radius_two() {
        let strides = vec![
            ColumnStride { start: 0, end: 3 },
            ColumnStride { start: 2, end: 3 },
            ColumnStride { start: 2, end: 3 },
            ColumnStride { start: 2, end: 4 },
            ColumnStride { start: 3, end: 6 },
        ];
        let w = Window::from_strides(strides);
        let w_d = w.dilate(2);
        let expected = "\
* * * * * .
* * * * * *
* * * * * *
* * * * * *
* * * * * *
";
        assert_eq!(w_d.debug_string(), expected);
    }

    /// C++ TEST(S2PolylineAlignmentTest, `DilatesWindowByVeryLargeRadius`)
    #[test]
    fn test_dilate_by_very_large_radius() {
        let strides = vec![
            ColumnStride { start: 0, end: 3 },
            ColumnStride { start: 2, end: 3 },
            ColumnStride { start: 2, end: 3 },
            ColumnStride { start: 2, end: 4 },
            ColumnStride { start: 3, end: 6 },
        ];
        let w = Window::from_strides(strides);
        let w_d = w.dilate(100);
        let expected = "\
* * * * * *
* * * * * *
* * * * * *
* * * * * *
* * * * * *
";
        assert_eq!(w_d.debug_string(), expected);
    }

    // ─── Half resolution ──────────────────────────────────────────────

    /// C++ TEST(S2PolylineAlignmentTest, `HalvesZeroLengthPolyline`)
    #[test]
    fn test_halves_zero_length_polyline() {
        let line = Polyline::new(vec![]);
        let halved = half_resolution(&line);
        assert_eq!(halved.num_vertices(), 0);
    }

    /// C++ TEST(S2PolylineAlignmentTest, `HalvesEvenLengthPolyline`)
    #[test]
    fn test_halves_even_length_polyline() {
        let line = make_polyline("0:0, 0:1, 0:2, 1:2");
        let halved = half_resolution(&line);
        let correct = make_polyline("0:0, 0:2");
        assert_eq!(halved.num_vertices(), correct.num_vertices());
        for i in 0..halved.num_vertices() {
            assert!(
                halved.vertex(i).approx_eq(correct.vertex(i)),
                "vertex {i} mismatch"
            );
        }
    }

    /// C++ TEST(S2PolylineAlignmentTest, `HalvesOddLengthPolyline`)
    #[test]
    fn test_halves_odd_length_polyline() {
        let line = make_polyline("0:0, 0:1, 0:2, 1:2, 3:5");
        let halved = half_resolution(&line);
        let correct = make_polyline("0:0, 0:2, 3:5");
        assert_eq!(halved.num_vertices(), correct.num_vertices());
        for i in 0..halved.num_vertices() {
            assert!(
                halved.vertex(i).approx_eq(correct.vertex(i)),
                "vertex {i} mismatch"
            );
        }
    }

    // ─── Panic tests (C++ death tests) ───────────────────────────────

    /// C++ TEST(S2PolylineAlignmentDeathTest, `ExactLengthZeroInputs`)
    #[test]
    #[should_panic(expected = "Polyline A is empty")]
    fn test_exact_length_zero_inputs() {
        let a = Polyline::new(vec![]);
        let b = Polyline::new(vec![]);
        drop(get_exact_vertex_alignment(&a, &b));
    }

    /// C++ TEST(S2PolylineAlignmentDeathTest, `ExactLengthZeroInputA`)
    #[test]
    #[should_panic(expected = "Polyline A is empty")]
    fn test_exact_length_zero_input_a() {
        let a = Polyline::new(vec![]);
        let b = make_polyline("0:0, 1:1, 2:2");
        drop(get_exact_vertex_alignment(&a, &b));
    }

    /// C++ TEST(S2PolylineAlignmentDeathTest, `ExactLengthZeroInputB`)
    #[test]
    #[should_panic(expected = "Polyline B is empty")]
    fn test_exact_length_zero_input_b() {
        let a = make_polyline("0:0, 1:1, 2:2");
        let b = Polyline::new(vec![]);
        drop(get_exact_vertex_alignment(&a, &b));
    }

    /// C++ TEST(S2PolylineAlignmentDeathTest, `MedoidPolylineNoPolylines`)
    #[test]
    #[should_panic(expected = "Empty polyline collection")]
    fn test_medoid_no_polylines() {
        let polylines: Vec<Polyline> = vec![];
        let _ = get_medoid_polyline(&polylines, &MedoidOptions::default());
    }

    /// C++ TEST(S2PolylineAlignmentDeathTest, `ConsensusPolylineNoPolylines`)
    #[test]
    #[should_panic(expected = "Empty polyline collection")]
    fn test_consensus_no_polylines() {
        let polylines: Vec<Polyline> = vec![];
        drop(get_consensus_polyline(
            &polylines,
            &ConsensusOptions::default(),
        ));
    }

    // ─── Brute-force fuzz verification ────────────────────────────────

    /// C++ TEST(S2PolylineAlignmentTest, `FuzzedWithBruteForce`)
    ///
    /// Uses deterministic pseudo-random polylines to verify that the DP
    /// cost matches a brute-force recursive solver.
    #[test]
    fn test_fuzzed_with_brute_force() {
        use crate::s2::LatLng;
        // Generate a set of short, correlated polylines.
        let num_polylines = 10;
        let num_vertices = 8;

        // Deterministic "random" points at various latitudes/longitudes.
        let mut polylines = Vec::with_capacity(num_polylines);
        for i in 0..num_polylines {
            let mut pts = Vec::with_capacity(num_vertices);
            for j in 0..num_vertices {
                let lat = (i as f64 * 3.7 + j as f64 * 1.3) % 10.0;
                let lng = (i as f64 * 5.1 + j as f64 * 2.9) % 20.0 - 10.0;
                pts.push(LatLng::from_degrees(lat, lng).to_point());
            }
            polylines.push(Polyline::new(pts));
        }

        for i in 0..num_polylines {
            for j in (i + 1)..num_polylines {
                verify_cost_brute_force(&polylines[i], &polylines[j]);
            }
        }
    }

    // ─── Original tests ───────────────────────────────────────────────

    #[test]
    fn test_exact_length_one_inputs() {
        let a = make_polyline("1:1");
        let b = make_polyline("2:2");
        verify_path(&a, &b, &[(0, 0)]);
        verify_cost(&a, &b);
    }

    #[test]
    fn test_exact_length_one_input_a() {
        let a = make_polyline("0:0");
        let b = make_polyline("0:0, 1:1, 2:2");
        verify_path(&a, &b, &[(0, 0), (0, 1), (0, 2)]);
        verify_cost(&a, &b);
    }

    #[test]
    fn test_exact_length_one_input_b() {
        let a = make_polyline("0:0, 1:1, 2:2");
        let b = make_polyline("0:0");
        verify_path(&a, &b, &[(0, 0), (1, 0), (2, 0)]);
        verify_cost(&a, &b);
    }

    #[test]
    fn test_exact_header_file_example() {
        let a = make_polyline("1:0, 5:0, 6:0, 9:0");
        let b = make_polyline("2:0, 7:0, 8:0");
        verify_path(&a, &b, &[(0, 0), (1, 1), (2, 1), (3, 2)]);
        verify_cost(&a, &b);
    }

    #[test]
    fn test_different_path_for_chordal_vs_squared() {
        let a = make_polyline("0.1:-0.1, 0.1:0, 0.1:0.1, -0.1:0.1");
        let b = make_polyline("0.1:-0.1, -0.1:-0.1, -0.1:0.1");
        verify_path(&a, &b, &[(0, 0), (1, 0), (2, 1), (3, 2)]);
        verify_cost(&a, &b);
    }

    #[test]
    fn test_approx_matches_exact_small() {
        let a = make_polyline("0:0, 1:0, 2:0, 3:0, 4:0");
        let b = make_polyline("0:1, 1:1, 2:1, 3:1");
        let exact = get_exact_vertex_alignment(&a, &b);
        let approx = get_approx_vertex_alignment_default(&a, &b);
        // For small polylines, approx should fall through to exact.
        assert!(
            (exact.alignment_cost - approx.alignment_cost).abs() < 1e-6,
            "Approx cost differs from exact"
        );
    }

    #[test]
    fn test_medoid_one_polyline() {
        let polylines = vec![make_polyline("5:0, 5:1, 5:2")];
        let medoid = get_medoid_polyline(&polylines, &MedoidOptions::default());
        assert_eq!(medoid, 0);
    }

    #[test]
    fn test_medoid_two_polylines() {
        let polylines = vec![
            make_polyline("5:0, 5:1, 5:2"),
            make_polyline("1:0, 1:1, 1:2"),
        ];
        let medoid = get_medoid_polyline(&polylines, &MedoidOptions::default());
        // Tie-breaking: smallest index.
        assert_eq!(medoid, 0);
    }

    #[test]
    fn test_medoid_few_small() {
        let polylines = vec![
            make_polyline("5:0, 5:1, 5:2"),
            make_polyline("3:0, 3:1, 3:2"),
            make_polyline("1:0, 1:1, 1:2"),
        ];
        let medoid = get_medoid_polyline(&polylines, &MedoidOptions::default());
        assert_eq!(medoid, 1);
    }

    #[test]
    fn test_medoid_overlapping() {
        let polylines = vec![
            make_polyline("1:0, 1:1, 1:2"),
            make_polyline("1:0, 1:1, 1:2"),
        ];
        let medoid = get_medoid_polyline(&polylines, &MedoidOptions::default());
        assert_eq!(medoid, 0);
    }

    #[test]
    fn test_medoid_different_lengths() {
        let polylines = vec![
            make_polyline("5:0, 5:1, 5:2"),
            make_polyline("3:0, 3:0.5, 3:1, 3:2"),
            make_polyline("1:0, 1:0.5, 1:1, 1:1.5, 1:2"),
        ];
        let medoid = get_medoid_polyline(&polylines, &MedoidOptions::default());
        assert_eq!(medoid, 1);
    }

    #[test]
    fn test_consensus_one_polyline() {
        let polylines = vec![make_polyline("3:0, 3:1, 3:2")];
        let result = get_consensus_polyline(&polylines, &ConsensusOptions::default());
        let expected = make_polyline("3:0, 3:1, 3:2");
        assert!(result.approx_eq_with(&expected, crate::s1::Angle::from_radians(1e-6)));
    }

    #[test]
    fn test_consensus_two_polylines() {
        let polylines = vec![
            make_polyline("3:0, 3:1, 3:2"),
            make_polyline("1:0, 1:1, 1:2"),
        ];
        let result = get_consensus_polyline(&polylines, &ConsensusOptions::default());
        let expected = make_polyline("2:0, 2:1, 2:2");
        assert!(
            result.approx_eq_with(&expected, crate::s1::Angle::from_radians(1e-2)),
            "Consensus should be approximately midway"
        );
    }

    #[test]
    fn test_consensus_overlapping() {
        let polylines = vec![
            make_polyline("1:0, 1:1, 1:2"),
            make_polyline("1:0, 1:1, 1:2"),
        ];
        let result = get_consensus_polyline(&polylines, &ConsensusOptions::default());
        let expected = make_polyline("1:0, 1:1, 1:2");
        assert!(result.approx_eq_with(&expected, crate::s1::Angle::from_radians(1e-6)));
    }

    // ─── C++ counterpart tests for refactored code paths ──────────────

    #[test]
    fn test_medoid_few_large_polylines() {
        let polylines = vec![
            make_polyline("1:0, 1:1, 1:2, 1:3, 1:4"),
            make_polyline("3:0, 3:1, 3:2, 3:3, 3:4"),
            make_polyline("5:0, 5:1, 5:2, 5:3, 5:4"),
            make_polyline("7:0, 7:1, 7:2, 7:3, 7:4"),
            make_polyline("9:0, 9:1, 9:2, 9:3, 9:4"),
            make_polyline("11:0, 11:1, 11:2, 11:3, 11:4"),
        ];
        let medoid = get_medoid_polyline(&polylines, &MedoidOptions::default());
        assert!(
            medoid == 2 || medoid == 3,
            "expected medoid in [2,3], got {medoid}"
        );
    }

    #[test]
    fn test_medoid_exact_mode() {
        let polylines = vec![
            make_polyline("5:0, 5:1, 5:2"),
            make_polyline("3:0, 3:1, 3:2"),
            make_polyline("1:0, 1:1, 1:2"),
        ];
        let exact_opts = MedoidOptions { approx: false };
        let medoid = get_medoid_polyline(&polylines, &exact_opts);
        assert_eq!(medoid, 1, "exact medoid should be the central polyline");
    }

    /// C++ TEST(S2PolylineAlignmentTest, `MedoidPolylineFewLargePolylines`)
    /// — verify that exact and approx medoid modes both compute correctly
    /// by comparing against manually computed cost sums.
    #[test]
    fn test_medoid_exact_vs_approx_costs() {
        let polylines = vec![
            make_polyline("5:0, 5:1, 5:2"),
            make_polyline("3:0, 3:1, 3:2"),
            make_polyline("1:0, 1:1, 1:2"),
        ];

        // Compute exact costs manually.
        let exact_costs: Vec<f64> = (0..3)
            .map(|i| {
                (0..3)
                    .filter(|&j| j != i)
                    .map(|j| get_exact_vertex_alignment_cost(&polylines[i], &polylines[j]))
                    .sum()
            })
            .collect();
        let exact_medoid_index = exact_costs
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0;

        let exact_opts = MedoidOptions { approx: false };
        let exact_medoid = get_medoid_polyline(&polylines, &exact_opts);
        assert_eq!(exact_medoid, exact_medoid_index);

        // Compute approx costs manually.
        let approx_costs: Vec<f64> = (0..3)
            .map(|i| {
                (0..3)
                    .filter(|&j| j != i)
                    .map(|j| {
                        get_approx_vertex_alignment_default(&polylines[i], &polylines[j])
                            .alignment_cost
                    })
                    .sum()
            })
            .collect();
        let approx_medoid_index = approx_costs
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0;

        let approx_opts = MedoidOptions { approx: true };
        let approx_medoid = get_medoid_polyline(&polylines, &approx_opts);
        assert_eq!(approx_medoid, approx_medoid_index);
    }

    #[test]
    fn test_window_from_warp_path_endpoints() {
        let path: WarpPath = vec![
            (0, 0),
            (1, 0),
            (1, 1),
            (2, 1),
            (3, 1),
            (3, 2),
            (3, 3),
            (4, 4),
            (4, 5),
        ];
        let w = Window::from_warp_path(&path);
        assert_eq!(w.rows, 5);
        assert_eq!(w.cols, 6);
        assert_eq!(w.strides[0].start, 0);
        assert_eq!(w.strides[0].end, 1);
        assert_eq!(w.strides[1].start, 0);
        assert_eq!(w.strides[1].end, 2);
        assert_eq!(w.strides[2].start, 1);
        assert_eq!(w.strides[2].end, 2);
        assert_eq!(w.strides[3].start, 1);
        assert_eq!(w.strides[3].end, 4);
        assert_eq!(w.strides[4].start, 4);
        assert_eq!(w.strides[4].end, 6);
    }

    #[test]
    fn test_window_from_strides_cols() {
        let strides = vec![
            ColumnStride { start: 0, end: 3 },
            ColumnStride { start: 1, end: 4 },
            ColumnStride { start: 2, end: 4 },
            ColumnStride { start: 3, end: 6 },
            ColumnStride { start: 4, end: 6 },
        ];
        let w = Window::from_strides(strides);
        assert_eq!(w.rows, 5);
        assert_eq!(w.cols, 6);
        assert_eq!(w.strides[0].start, 0);
        assert_eq!(w.strides[0].end, 3);
        assert_eq!(w.strides[4].start, 4);
        assert_eq!(w.strides[4].end, 6);
    }

    /// C++ TEST(S2PolylineAlignmentTest, `CreatesWindowFromWarpPath`)
    /// — verify the `debug_string` output matches the expected grid.
    #[test]
    fn test_window_from_warp_path_debug_string() {
        let path: WarpPath = vec![
            (0, 0),
            (1, 0),
            (1, 1),
            (2, 1),
            (3, 1),
            (3, 2),
            (3, 3),
            (4, 4),
            (4, 5),
        ];
        let w = Window::from_warp_path(&path);
        let expected = "\
* . . . . .
* * . . . .
. * . . . .
. * * * . .
. . . . * *
";
        assert_eq!(w.debug_string(), expected);
    }

    /// Verify that the cost-only function matches the alignment cost
    /// for the exact length-zero panic path (exercises the assert in
    /// `get_exact_vertex_alignment_cost`).
    #[test]
    #[should_panic(expected = "Polyline A is empty")]
    fn test_exact_cost_length_zero_inputs() {
        let a = Polyline::new(vec![]);
        let b = Polyline::new(vec![]);
        let _ = get_exact_vertex_alignment_cost(&a, &b);
    }

    /// C++ TEST(S2PolylineAlignmentDeathTest, `ExactLengthZeroInputA`) — cost variant
    #[test]
    #[should_panic(expected = "Polyline A is empty")]
    fn test_exact_cost_length_zero_input_a() {
        let a = Polyline::new(vec![]);
        let b = make_polyline("0:0, 1:1, 2:2");
        let _ = get_exact_vertex_alignment_cost(&a, &b);
    }

    /// C++ TEST(S2PolylineAlignmentDeathTest, `ExactLengthZeroInputB`) — cost variant
    #[test]
    #[should_panic(expected = "Polyline B is empty")]
    fn test_exact_cost_length_zero_input_b() {
        let a = make_polyline("0:0, 1:1, 2:2");
        let b = Polyline::new(vec![]);
        let _ = get_exact_vertex_alignment_cost(&a, &b);
    }

    /// Approx alignment also panics on empty polylines.
    #[test]
    #[should_panic(expected = "Polyline A is empty")]
    fn test_approx_length_zero_input_a() {
        let a = Polyline::new(vec![]);
        let b = make_polyline("0:0, 1:1, 2:2");
        drop(get_approx_vertex_alignment(&a, &b, 2));
    }

    /// Approx alignment also panics on empty polylines.
    #[test]
    #[should_panic(expected = "Polyline B is empty")]
    fn test_approx_length_zero_input_b() {
        let a = make_polyline("0:0, 1:1, 2:2");
        let b = Polyline::new(vec![]);
        drop(get_approx_vertex_alignment(&a, &b, 2));
    }
}
