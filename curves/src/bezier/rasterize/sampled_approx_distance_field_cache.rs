/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::bezier::vectorize::*;
use crate::geo::*;

use smallvec::*;

use std::collections::{HashMap, HashSet};
use std::ops::Range;

///
/// Produces an approximation of a distance field for a shape
///
/// This uses an algorithm that assumes that the closest point to a given pixel is also the closest point of one of its neighbours.
/// This makes it possible to construct the distance field very quickly from sampled points around the perimeter of the shape but
/// produces reduced accuracy away from the edge.
///
#[derive(Clone)]
pub struct SampledApproxDistanceFieldCache {
    /// The size of the distance field (we stop generating at x=0, y=0 and thes bounds)
    size: ContourSize,

    /// The ranges along each scanline that contain interior pixels
    interior_pixels: Vec<SmallVec<[Range<f64>; 4]>>,

    /// Known points that are at 0 distance from the shape
    zero_points: Vec<(f64, f64)>,

    /// Points with distances derviced from the zero points (hashmap maps positions to a distance and an index into the zero_points structure)
    /// Distances are stored as 'distance squared' and are negative for points that are considered to be inside the shape.
    cached_points: HashMap<ContourPosition, (f64, usize)>,

    /// Points that are waiting to be calculated (these have neighbours in the cached_points structure)
    waiting_points: HashSet<ContourPosition>,
}

impl SampledApproxDistanceFieldCache {
    ///
    /// Begins populating the distance field cache from a list of points on the perimeter of the shape and the ranges covered by the interior pixels
    /// of the shape.
    ///
    /// The samples are a list of contour positions and the closest point found from them. The other samples in the cache will be generated by referencing
    /// these points: in general these points are the points that are found along the perimeter of the shape.
    ///
    /// The interior pixels are returned as an enumerator from y = 0 and should be ordered by x index (ie, in the same format as returned by
    /// `SampledContour::intercepts_on_line()`)
    ///
    pub fn from_points<'a, TPoint>(
        perimeter_samples: impl 'a + IntoIterator<Item = (ContourPosition, TPoint)>,
        interior_pixels: impl 'a + IntoIterator<Item = SmallVec<[Range<f64>; 4]>>,
        size: ContourSize,
    ) -> Self
    where
        TPoint: Coordinate2D,
    {
        let interior_pixels = interior_pixels.into_iter().collect();

        // The zero points are used as reference points
        let zero_points = perimeter_samples
            .into_iter()
            .map(|(pos, point)| (pos, point.x(), point.y()))
            .collect::<Vec<_>>();

        // Cached points are known on the distance field, waiting points are points which have not distributed their distance to their
        // neighbours yet
        let mut cached_points = HashMap::<ContourPosition, (f64, usize)>::new();
        let mut waiting_points = HashSet::new();

        for idx in 0..zero_points.len() {
            let (nearest_point, sample_x, sample_y) = zero_points[idx];

            // Work out the distance to this point
            let (point_x, point_y) = (nearest_point.x() as f64, nearest_point.y() as f64);
            let (offset_x, offset_y) = (sample_x - point_x, sample_y - point_y);
            let distance_squared = offset_x * offset_x + offset_y * offset_y;

            // Add or update the point in the cache
            if let Some((existing_distance, existing_idx)) = cached_points.get_mut(&nearest_point) {
                // Replace the existing point with this one if it's nearer to an edge
                if distance_squared < *existing_distance {
                    *existing_distance = distance_squared;
                    *existing_idx = idx;
                }
            } else {
                // Add a new point
                cached_points.insert(nearest_point, (distance_squared, idx));
                waiting_points.insert(nearest_point);
            }
        }

        // Remove the 'nearest point' from the zero points
        let zero_points = zero_points.into_iter().map(|(_, x, y)| (x, y)).collect();

        SampledApproxDistanceFieldCache {
            size,
            interior_pixels,
            zero_points,
            cached_points,
            waiting_points,
        }
    }

    ///
    /// Returns true if the pixel at the given position is inside the circle
    ///
    #[inline]
    fn point_is_inside(&self, x: usize, y: usize) -> bool {
        if y < self.interior_pixels.len() {
            let x = x as f64;
            let line = &self.interior_pixels[y];

            for range in line.iter() {
                if range.start > x {
                    break;
                }

                if range.start <= x && range.end > x {
                    return true;
                }
            }

            false
        } else {
            false
        }
    }

    ///
    /// The dimensions of this cache
    ///
    #[inline]
    pub fn size(&self) -> ContourSize {
        self.size
    }

    ///
    /// Process the waiting points to grow the set of points with distances set
    ///
    pub fn grow_samples(&mut self) {
        use std::mem;

        let width = self.size.width() as f64;
        let height = self.size.height() as f64;

        // Take the current set of waiting points
        let mut waiting_points = HashSet::new();
        mem::swap(&mut self.waiting_points, &mut waiting_points);

        // Process the neighbours of each one to generate a new set of samples/waiting points
        for pos in waiting_points {
            // This point should already be cached: its nearest point is likely to be the nearest point of one of the neighbours
            let (dist, nearest_idx) = *self.cached_points.get(&pos).unwrap();
            let (sample_x, sample_y) = self.zero_points[nearest_idx];
            let is_inside = dist < 0.0;

            // Process the neighbours of this point: either refine the distance if our 'root' point is nearer, or add a new 'nearest' point if not
            let point_x = pos.x() as f64;
            let point_y = pos.y() as f64;

            for y_offset in [-1, 1] {
                // Use the offset point
                let point_y = point_y + (y_offset as f64);
                if point_y < 0.0 || point_y >= height {
                    continue;
                }

                for x_offset in [-1, 1] {
                    // Use the offset point (ignore samples outside of the size boundary)
                    let point_x = point_x + (x_offset as f64);
                    if point_x < 0.0 || point_x >= width {
                        continue;
                    }

                    // Sample this position, determine if it's inside or not
                    let pos = ContourPosition(point_x as usize, point_y as usize);

                    let offset_x = sample_x - point_x;
                    let offset_y = sample_y - point_y;
                    let distance = offset_x * offset_x + offset_y * offset_y;

                    // Update the cache at this point
                    if let Some((existing_distance, existing_idx)) =
                        self.cached_points.get_mut(&pos)
                    {
                        // Replace the existing point if this one is closer (we shouldn't grow 'inside' distances outside the shape as we shouldn't find closer points at the boundary)
                        if distance < existing_distance.abs() {
                            let distance = if is_inside { -distance } else { distance };

                            *existing_distance = distance;
                            *existing_idx = nearest_idx;

                            self.waiting_points.insert(pos);
                        }
                    } else {
                        // Haven't seen this point yet, so this is the closest perimeter point to it
                        let distance = if is_inside { -distance } else { distance };
                        self.cached_points.insert(pos, (distance, nearest_idx));

                        // As we haven't seen this point before, add to the waiting points
                        self.waiting_points.insert(pos);
                    }
                }
            }
        }
    }

    ///
    /// Retrieves the squared distance at this point
    ///
    /// This will return a positive value for outside points and a negative value for inside points. Take the square root of
    /// the absolute value to get the distance, then preserve the sign to generate a signed distance field.
    ///
    pub fn distance_squared_at_point(&mut self, pos: ContourPosition) -> f64 {
        if pos.0 >= self.size.width() {
            return f64::MAX;
        };
        if pos.0 >= self.size.height() {
            return f64::MAX;
        };

        let distance_squared = loop {
            if self.waiting_points.is_empty() {
                // Have run out of waiting points: always return a value
                if let Some((distance, _)) = self.cached_points.get(&pos) {
                    break *distance;
                } else {
                    break f64::MAX;
                }
            }

            // If we already have an estimate for the distance of this point, then use that
            if let Some((distance, _)) = self.cached_points.get(&pos) {
                break *distance;
            }

            // Grow the set of samples and try again
            self.grow_samples();
        };

        if self.point_is_inside(pos.0, pos.1) {
            -distance_squared
        } else {
            distance_squared
        }
    }
}