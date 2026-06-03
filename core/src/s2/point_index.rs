// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

// S2PointIndex: a spatial index for S2Points sorted by leaf S2CellId.
//
// Each point can optionally store auxiliary data. Points can be added or
// removed dynamically, and a seekable iterator provides efficient spatial
// traversal.
//
// C++ ref: s2point_index.h

use std::collections::BTreeMap;

use crate::s2::{CellId, Point};

/// An entry in the point index: a point with associated data.
#[derive(Clone, Debug)]
pub struct PointData<D: Clone> {
    /// The point on the sphere.
    pub point: Point,
    /// The associated data value.
    pub data: D,
}

impl<D: Clone + PartialEq> PartialEq for PointData<D> {
    fn eq(&self, other: &Self) -> bool {
        self.point == other.point && self.data == other.data
    }
}

/// A spatial index of `S2Points` sorted by leaf `S2CellId`.
///
/// Each point can store auxiliary data of type `D`. Use `()` if no extra
/// data is needed.
#[derive(Debug)]
pub struct S2PointIndex<D: Clone> {
    // BTreeMap from CellId to a list of point-data pairs at that cell.
    map: BTreeMap<CellId, Vec<PointData<D>>>,
    num_points: usize,
}

impl<D: Clone> Default for S2PointIndex<D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<D: Clone> S2PointIndex<D> {
    /// Creates an empty index.
    pub fn new() -> Self {
        S2PointIndex {
            map: BTreeMap::new(),
            num_points: 0,
        }
    }

    /// Returns the number of points in the index.
    pub fn num_points(&self) -> usize {
        self.num_points
    }

    /// Adds a point with associated data to the index.
    pub fn add(&mut self, point: Point, data: D) {
        let id = CellId::from_point(&point);
        self.map
            .entry(id)
            .or_default()
            .push(PointData { point, data });
        self.num_points += 1;
    }

    /// Removes the first matching (point, data) pair. Returns false if not found.
    pub fn remove(&mut self, point: Point, data: &D) -> bool
    where
        D: PartialEq,
    {
        let id = CellId::from_point(&point);
        if let Some(entries) = self.map.get_mut(&id)
            && let Some(pos) = entries
                .iter()
                .position(|e| e.point == point && e.data == *data)
        {
            entries.swap_remove(pos);
            if entries.is_empty() {
                self.map.remove(&id);
            }
            self.num_points -= 1;
            return true;
        }
        false
    }

    /// Removes all points from the index.
    pub fn clear(&mut self) {
        self.map.clear();
        self.num_points = 0;
    }

    /// Returns an iterator positioned at the first entry.
    pub fn iter(&self) -> PointIndexIterator<'_, D> {
        PointIndexIterator::new(self)
    }
}

impl<'a, D: Clone> IntoIterator for &'a S2PointIndex<D> {
    type Item = (CellId, &'a PointData<D>);
    type IntoIter = PointIndexIterator<'a, D>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Iterator over an `S2PointIndex`. Walks entries in `CellId` order.
///
/// The iterator provides seek-based traversal suitable for spatial queries.
#[derive(Debug)]
pub struct PointIndexIterator<'a, D: Clone> {
    // Flattened view: we iterate over the BTreeMap entries, and for each
    // CellId entry we iterate over its Vec of PointData.
    entries: Vec<(CellId, &'a PointData<D>)>,
    pos: usize,
}

impl<'a, D: Clone> PointIndexIterator<'a, D> {
    /// Creates a new iterator positioned at the first entry.
    pub fn new(index: &'a S2PointIndex<D>) -> Self {
        let mut entries = Vec::with_capacity(index.num_points);
        for (&cell_id, point_datas) in &index.map {
            for pd in point_datas {
                entries.push((cell_id, pd));
            }
        }
        PointIndexIterator { entries, pos: 0 }
    }

    /// Returns true if the iterator is past the last entry.
    pub fn done(&self) -> bool {
        self.pos >= self.entries.len()
    }

    /// Returns the `CellId` of the current entry.
    pub fn id(&self) -> CellId {
        self.entries[self.pos].0
    }

    /// Returns the point of the current entry.
    pub fn point(&self) -> Point {
        self.entries[self.pos].1.point
    }

    /// Returns the data of the current entry.
    pub fn data(&self) -> &D {
        &self.entries[self.pos].1.data
    }

    /// Returns the `PointData` of the current entry.
    pub fn point_data(&self) -> &'a PointData<D> {
        self.entries[self.pos].1
    }

    /// Positions the iterator at the first entry.
    pub fn begin(&mut self) {
        self.pos = 0;
    }

    /// Positions the iterator past the last entry.
    pub fn finish(&mut self) {
        self.pos = self.entries.len();
    }

    /// Advances to the next entry.
    pub fn next(&mut self) {
        self.pos += 1;
    }

    /// Moves to the previous entry. Returns false if already at beginning.
    pub fn prev(&mut self) -> bool {
        if self.pos == 0 {
            return false;
        }
        self.pos -= 1;
        true
    }

    /// Positions at the first entry with `id()` >= target.
    pub fn seek(&mut self, target: CellId) {
        self.pos = self.entries.partition_point(|&(id, _)| id < target);
    }
}

impl<'a, D: Clone> Iterator for PointIndexIterator<'a, D> {
    type Item = (CellId, &'a PointData<D>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.done() {
            return None;
        }
        let item = self.entries[self.pos];
        self.pos += 1;
        Some(item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::LatLng;

    #[test]
    fn test_empty_index() {
        let index = S2PointIndex::<i32>::new();
        assert_eq!(index.num_points(), 0);
        let iter = index.iter();
        assert!(iter.done());
    }

    #[test]
    fn test_add_and_iterate() {
        let mut index = S2PointIndex::new();
        let p1 = LatLng::from_degrees(0.0, 0.0).to_point();
        let p2 = LatLng::from_degrees(10.0, 20.0).to_point();
        let p3 = LatLng::from_degrees(-5.0, 50.0).to_point();
        index.add(p1, 1);
        index.add(p2, 2);
        index.add(p3, 3);
        assert_eq!(index.num_points(), 3);

        let mut iter = index.iter();
        let mut count = 0;
        while !iter.done() {
            count += 1;
            iter.next();
        }
        assert_eq!(count, 3);
    }

    #[test]
    fn test_remove() {
        let mut index = S2PointIndex::new();
        let p = LatLng::from_degrees(0.0, 0.0).to_point();
        index.add(p, 42);
        assert_eq!(index.num_points(), 1);
        assert!(index.remove(p, &42));
        assert_eq!(index.num_points(), 0);
        assert!(!index.remove(p, &42));
    }

    #[test]
    fn test_seek() {
        let mut index = S2PointIndex::new();
        // Add points at various locations.
        for i in 0..20 {
            let lat = -90.0 + f64::from(i) * 9.0;
            let p = LatLng::from_degrees(lat, 0.0).to_point();
            index.add(p, i);
        }
        let mut iter = index.iter();
        // Seek to the CellId of a point near the equator.
        let target = CellId::from_point(&LatLng::from_degrees(0.0, 0.0).to_point());
        iter.seek(target);
        assert!(!iter.done());
        assert!(iter.id() >= target);
    }

    #[test]
    fn test_duplicate_points() {
        let mut index = S2PointIndex::new();
        let p = Point::from_coords(1.0, 0.0, 0.0);
        for i in 0..100 {
            index.add(p, i);
        }
        assert_eq!(index.num_points(), 100);
        let mut iter = index.iter();
        let mut count = 0;
        while !iter.done() {
            count += 1;
            iter.next();
        }
        assert_eq!(count, 100);
    }

    #[test]
    fn test_prev() {
        let mut index = S2PointIndex::new();
        let p1 = LatLng::from_degrees(10.0, 0.0).to_point();
        let p2 = LatLng::from_degrees(20.0, 0.0).to_point();
        index.add(p1, 1);
        index.add(p2, 2);
        let mut iter = index.iter();
        assert!(!iter.prev()); // at beginning
        iter.finish();
        assert!(iter.prev()); // go to last
        assert!(!iter.done());
    }
}
