/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![allow(clippy::comparison_chain)] // I think the two instances in here are clearer written the way they are

use crate::bezier::curve::*;
use crate::bezier::normal::*;
use crate::bezier::path::graph_path::*;
use crate::bezier::path::is_clockwise::*;
use crate::bezier::path::path::*;
use crate::bezier::path::ray::*;
use crate::geo::*;

use smallvec::*;

///
/// Winding direction of a particular path
///
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum PathDirection {
    Clockwise,
    Anticlockwise,
}

impl<'a, P: BezierPath> From<&'a P> for PathDirection
where
    P::Point: Coordinate2D,
{
    #[inline]
    fn from(path: &'a P) -> PathDirection {
        if path.is_clockwise() {
            PathDirection::Clockwise
        } else {
            PathDirection::Anticlockwise
        }
    }
}

///
/// Label attached to a path used for arithmetic
///
/// The parameters are the path number (counting from 0) and the winding direction of the path
///
#[derive(Clone, Copy, Debug)]
pub struct PathLabel(pub u32);

impl<Point: Coordinate + Coordinate2D> GraphPath<Point, PathLabel> {
    ///
    /// Computes the collision count for a point along an edge in the graph
    ///
    /// The result is 'None' if the point described is at an intersection or if a ray cast at the specified point does not intersect the edge.
    ///
    /// The collision count is uncertain if it occurs on two overlapping edges, as any ordering of the edges is valid. `set_edge_kinds_by_ray_casting()`
    /// has rules for this so that crossing counts are consistent when entering and leaving a shape. This call uses simpler rules and might switch which
    /// shape is hit first when two shapes have edges is common.
    ///
    pub fn edge_collision_count(&self, target_edge: GraphEdgeRef, t: f64) -> Option<i64> {
        // TODO: the 'simpler rules' described above are defined in ray_collisions and probably should be changed to 'earlier edges are hit first', but this would be an API change and must be done at a major revision

        // Fetch the point that the ray is being 'fired' at
        let real_edge = self.get_edge(target_edge);
        let point = real_edge.point_at_pos(t);
        let normal = real_edge.normal_at_pos(t);

        // Work out what the ray collides with
        let ray = (point - normal, point);
        let ray_direction = ray.1 - ray.0;
        let collisions = self.ray_collisions(&ray);

        // Count collisions until we hit the point requested
        let mut count = 0;
        for (collision, curve_t, _line_t, _pos) in collisions {
            let edge = collision.edge();

            // The relative direction of the tangent to the ray indicates the direction we're crossing in
            let normal = self.get_edge(edge).normal_at_pos(curve_t);
            let side = ray_direction.dot(&normal).signum() as i32;

            // Add this collision to the count
            if side < 0 {
                count -= 1;
            } else if side > 0 {
                count += 1;
            }

            // Stop if we're in the approximate location of the requested target
            if edge == target_edge && (curve_t - t).abs() < 0.001 {
                if collision.is_intersection() {
                    // Intersections have uncertain counts as it's not clear which order the edge would be crossed by the ray (they're all crossed simultaneously)
                    return None;
                } else {
                    return Some(count);
                }
            }
        }

        // Did not intercept the target edge (or target edge was not included as a collision)
        None
    }

    ///
    /// Sets the edge kinds by performing ray casting
    ///
    /// The function passed in to this method takes two parameters: these are the number of times edges have been crossed in
    /// path 1 and path 2. It should return true if this number of crossings represents a point inside the final shape, or false
    /// if it represents a point outside of the shape.
    ///
    /// Path crossings are processed in the order they're hit by the ray, with some exceptions:
    ///
    /// If a ray hits an intersection or hits very close to an intersection, the order is arbitrary and the edge kinds are not
    /// updated for that intersection (but are updated later on).
    ///
    /// If a ray crosses a set of overlapping edges, the order that the edges are crossed in depends on whether or not the ray
    /// is considered to be entering or leaving the 'inner' of the two shapes. If it's found to be entering the shape, the ray
    /// will hit the edge belonging to the second shape first, and if it's leaving it will hit the edge belonging to the first
    /// shape first. This ensures that the behaviour is consistent when the ray's direction is reversed.
    ///
    pub fn set_edge_kinds_by_ray_casting<FnIsInside: Fn(&SmallVec<[i32; 8]>) -> bool>(
        &mut self,
        is_inside: FnIsInside,
    ) {
        for point_idx in 0..self.num_points() {
            for next_edge in self.edge_refs_for_point(point_idx) {
                // Only process edges that have not yet been categorised
                if self.edge_kind(next_edge) != GraphPathEdgeKind::Uncategorised {
                    continue;
                }

                // Cast a ray at this edge
                let real_edge = self.get_edge(next_edge);
                let next_point = real_edge.point_at_pos(0.5);
                let next_normal = real_edge.normal_at_pos(0.5);

                // Mark the next edge as visited (this prevents an infinite loop in the event the edge we're aiming at has a length of 0 and thus will always be an intersection)
                self.set_edge_kind(next_edge, GraphPathEdgeKind::Visited);

                // The 'total direction' indicates how often we've crossed an edge moving in a particular direction
                // We're inside the path when it's non-zero
                let mut path_crossings: SmallVec<[i32; 8]> = smallvec![0, 0];

                // Cast a ray at the target edge
                let ray = (next_point - next_normal, next_point);
                let ray_direction = ray.1 - ray.0;
                let collisions = self.ray_collisions(&ray);

                // Overlapping edges need special treatment
                let collisions = group_overlapped_collisions(self as &Self, collisions);

                // Work out which edges are interior or exterior for every edge the ray has crossed
                for overlapping_group in collisions {
                    // Re-order overlapping edges according to whether or not the ray is inside the shape or not
                    let overlapping_group = if overlapping_group.len() <= 1 {
                        // Usually the ray will not collide with any overlapping edges
                        overlapping_group
                    } else {
                        // Overlapping edges are processed in ascending order when entering the first shape, and descending order when leaving it
                        // (This has the effect that when the ray is considered 'outside' the first shape it will hit the second shape first, which is the correct
                        // ordering for the subtraction operation)
                        let mut overlapping_group = overlapping_group;

                        // We use the supplied function to determine if the ray should be considered 'inside' or not
                        let first_shape_crossings = smallvec![path_crossings[0], 0];

                        if !is_inside(&first_shape_crossings) {
                            // Later shapes are crossed before earlier shapes when the ray is outside the first shape
                            overlapping_group.sort_by(
                                |(collision_a, _, _, _), (collision_b, _, _, _)| {
                                    collision_b
                                        .edge()
                                        .edge_idx
                                        .cmp(&collision_a.edge().edge_idx)
                                },
                            )
                        } else {
                            // Earlier shapes are crossed before later shapes when the ray is inside the first shape
                            overlapping_group.sort_by(
                                |(collision_a, _, _, _), (collision_b, _, _, _)| {
                                    collision_a
                                        .edge()
                                        .edge_idx
                                        .cmp(&collision_b.edge().edge_idx)
                                },
                            )
                        }

                        overlapping_group
                    };

                    // Determine if the ray is already inside or outside of the path
                    let was_inside = is_inside(&path_crossings);

                    // Process the edges in the group
                    for (collision, curve_t, _line_t, _pos) in overlapping_group.iter() {
                        let edge = collision.edge();

                        let PathLabel(path_number) = self.edge_label(edge);

                        // The relative direction of the tangent to the ray indicates the direction we're crossing in
                        let normal = self.get_edge(edge).normal_at_pos(*curve_t);
                        let side = ray_direction.dot(&normal).signum() as i32;

                        // Extend the path_crossings vector to accomodate all of the paths included by this ray
                        while path_crossings.len() <= path_number as usize {
                            path_crossings.push(0);
                        }

                        if side < 0 {
                            path_crossings[path_number as usize] -= 1;
                        } else if side > 0 {
                            path_crossings[path_number as usize] += 1;
                        }
                    }

                    // Determine if the ray is now inside or outside of the path
                    let is_inside = is_inside(&path_crossings);

                    // Filter the edges to those that are not hit by a ray close to the end or at an intersection
                    let mut edges_to_set = overlapping_group
                        .into_iter()
                        .filter(|(collision, curve_t, _line_t, _pos)| {
                            // Rays passing close to intersections or the end of a curve are more likely to be out-of-order
                            let is_intersection = collision.is_intersection();
                            let is_near_end = *curve_t < 0.01 || *curve_t > 0.99;

                            !is_intersection && !is_near_end
                        })
                        .map(|(collision, _, _, _)| collision.edge());

                    if was_inside ^ is_inside {
                        // If the ray moved from outside to inside or vice-versa, set one of the edges as an exterior edge (doesn't matter which one)
                        if let Some(first_edge) = edges_to_set.next() {
                            self.set_edge_kind_connected(first_edge, GraphPathEdgeKind::Exterior);
                        }
                        edges_to_set.for_each(|edge| {
                            self.set_edge_kind_connected(edge, GraphPathEdgeKind::Interior)
                        });
                    } else {
                        // If the ray is either still inside or outside the result, set all the edges to interior
                        edges_to_set.for_each(|edge| {
                            test_assert!(self.edge_kind(edge) != GraphPathEdgeKind::Exterior);

                            self.set_edge_kind_connected(edge, GraphPathEdgeKind::Interior)
                        });
                    }
                }

                // The ray should exit and enter the path an even number of times
                test_assert!(path_crossings
                    .into_iter()
                    .all(|crossing_count| crossing_count == 0));
            }
        }
    }
}
