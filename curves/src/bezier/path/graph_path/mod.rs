/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::path::*;
use super::PathDirection;
use crate::consts::*;
use crate::geo::*;

use smallvec::*;

use std::cell::*;
use std::collections::VecDeque;
use std::fmt;

mod edge;
mod edge_ref;
mod path_collision;
mod ray_collision;

#[cfg(test)]
pub(crate) mod test;

pub use self::edge::*;
pub use self::edge_ref::*;
pub use self::path_collision::*;
pub use self::ray_collision::*;

/// Maximum number of edges to traverse when 'healing' gaps found in an external path
const MAX_HEAL_DEPTH: usize = 3;

///
/// Kind of a graph path edge
///
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GraphPathEdgeKind {
    /// An edge that hasn't been categorised yet
    Uncategorised,

    /// An edge that is uncategorised but has been visited
    Visited,

    /// An exterior edge
    ///
    /// These edges represent a transition between the inside and the outside of the path
    Exterior,

    /// An interior edge
    ///
    /// These edges are on the inside of the path
    Interior,
}

///
/// Reference to a graph edge
///
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GraphEdgeRef {
    /// The index of the point this edge starts from
    pub(crate) start_idx: usize,

    /// The index of the edge within the point
    pub(crate) edge_idx: usize,

    /// True if this reference is for the reverse of this edge
    pub(crate) reverse: bool,
}

///
/// Enum representing an edge in a graph path
///
#[derive(Clone, Debug)]
struct GraphPathEdge<Point, Label> {
    /// The label attached to this edge
    label: Label,

    /// The ID of the edge following this one on the target point
    following_edge_idx: usize,

    /// The kind of this edge
    kind: GraphPathEdgeKind,

    /// Position of the first control point
    cp1: Point,

    /// Position of the second control point
    cp2: Point,

    /// The index of the target point
    end_idx: usize,

    /// The bounding box of this edge, if it has been calculated
    bbox: RefCell<Option<(Point, Point)>>,
}

///
/// Struct representing a point in a graph path
///
#[derive(Clone, Debug)]
struct GraphPathPoint<Point, Label> {
    /// The position of this point
    position: Point,

    /// The edges attached to this point
    forward_edges: SmallVec<[GraphPathEdge<Point, Label>; 2]>,

    /// The points with edges connecting to this point
    connected_from: SmallVec<[usize; 2]>,
}

impl<Point, Label> GraphPathPoint<Point, Label> {
    ///
    /// Creates a new graph path point
    ///
    fn new(
        position: Point,
        forward_edges: SmallVec<[GraphPathEdge<Point, Label>; 2]>,
        connected_from: SmallVec<[usize; 2]>,
    ) -> GraphPathPoint<Point, Label> {
        GraphPathPoint {
            position,
            forward_edges,
            connected_from,
        }
    }
}

///
/// A graph path is a path where each point can have more than one connected edge. Edges are categorized
/// into interior and exterior edges depending on if they are on the outside or the inside of the combined
/// shape.
///
#[derive(Clone)]
pub struct GraphPath<Point, Label> {
    /// The points in this graph and their edges. Each 'point' here consists of two control points and an end point
    points: Vec<GraphPathPoint<Point, Label>>,

    /// The index to assign to the next path added to this path
    next_path_index: usize,
}

///
/// Indicates the result of colliding two graph paths
///
#[derive(Clone)]
pub enum CollidedGraphPath<Point, Label> {
    /// Some of the edges had collisions in them
    Collided(GraphPath<Point, Label>),

    /// None of the edges has collisions in them
    Merged(GraphPath<Point, Label>),
}

impl<Point: Coordinate, Label> Geo for GraphPath<Point, Label> {
    type Point = Point;
}

impl<Point: Coordinate + Coordinate2D, Label: Copy> GraphPath<Point, Label> {
    ///
    /// Creates a new graph path with no points
    ///
    pub fn new() -> GraphPath<Point, Label> {
        GraphPath {
            points: vec![],
            next_path_index: 0,
        }
    }

    ///
    /// Creates a graph path from a bezier path
    ///
    pub fn from_path(
        path: &impl BezierPath<Point = Point>,
        label: Label,
    ) -> GraphPath<Point, Label> {
        // Use a reversed path if the direction is anti-clockwise
        let direction = PathDirection::from(path);

        match direction {
            PathDirection::Clockwise => Self::from_clockwise_path(path, label),
            PathDirection::Anticlockwise => Self::from_anticlockwise_path(path, label),
        }
    }

    ///
    /// Creates a graph path from a bezier path moving in an anti-clockwise direction
    ///
    fn from_anticlockwise_path(
        path: &impl BezierPath<Point = Point>,
        label: Label,
    ) -> GraphPath<Point, Label> {
        Self::from_clockwise_path(
            &path.reversed::<(Point, Vec<(Point, Point, Point)>)>(),
            label,
        )
    }

    ///
    /// Creates a graph path from a bezier path moving in the clockwise direction
    ///
    fn from_clockwise_path(
        path: &impl BezierPath<Point = Point>,
        label: Label,
    ) -> GraphPath<Point, Label> {
        // All edges are exterior for a single path
        let mut points = vec![];

        // Push the start point (with an open path)
        let start_point = path.start_point();
        points.push(GraphPathPoint::new(start_point, smallvec![], smallvec![]));

        // We'll add edges to the previous point
        let mut last_point_pos = start_point;
        let mut last_point_idx = 0;
        let mut next_point_idx = 1;

        // Iterate through the points in the path
        for (cp1, cp2, end_point) in path.points() {
            // Ignore points that are too close to the last point
            if end_point.is_near_to(&last_point_pos, CLOSE_DISTANCE) {
                if cp1.is_near_to(&last_point_pos, CLOSE_DISTANCE)
                    && cp2.is_near_to(&cp1, CLOSE_DISTANCE)
                {
                    continue;
                }
            }

            // Push the points
            points.push(GraphPathPoint::new(end_point, smallvec![], smallvec![]));

            // Add an edge from the last point to the next point
            points[last_point_idx]
                .forward_edges
                .push(GraphPathEdge::new(
                    GraphPathEdgeKind::Uncategorised,
                    (cp1, cp2),
                    next_point_idx,
                    label,
                    0,
                ));

            // Update the last/next pooints
            last_point_idx += 1;
            next_point_idx += 1;
            last_point_pos = end_point;
        }

        // Close the path
        if last_point_idx > 0 {
            // Graph actually has some edges
            if start_point.distance_to(&points[last_point_idx].position) < CLOSE_DISTANCE {
                // Remove the last point (we're replacing it with an edge back to the start)
                points.pop();
                last_point_idx -= 1;

                // Change the edge to point back to the start
                points[last_point_idx].forward_edges[0].end_idx = 0;
            } else {
                // Need to draw a line to the last point (as there is always a single following edge, the following edge index is always 0 here)
                let close_vector = points[last_point_idx].position - start_point;
                let cp1 = close_vector * 0.33 + start_point;
                let cp2 = close_vector * 0.66 + start_point;

                points[last_point_idx]
                    .forward_edges
                    .push(GraphPathEdge::new(
                        GraphPathEdgeKind::Uncategorised,
                        (cp1, cp2),
                        0,
                        label,
                        0,
                    ));
            }
        } else {
            // Just a start point and no edges: remove the start point as it doesn't really make sense
            points.pop();
        }

        // Create the graph path from the points
        let mut path = GraphPath {
            points: points,
            next_path_index: 1,
        };
        path.recalculate_reverse_connections();
        path
    }

    ///
    /// Creates a new graph path by merging (not colliding) a set of paths with their labels
    ///
    pub fn from_merged_paths<
        'a,
        P: 'a + BezierPath<Point = Point>,
        PathIter: IntoIterator<Item = (&'a P, Label)>,
    >(
        paths: PathIter,
    ) -> GraphPath<Point, Label> {
        // Create an empty path
        let mut merged_path = GraphPath::new();

        // Merge each path in turn
        for (path, label) in paths {
            let path = GraphPath::from_path(path, label);
            merged_path = merged_path.merge(path);
        }

        merged_path
    }

    ///
    /// Recomputes the list of items that have connections to each point
    ///
    fn recalculate_reverse_connections(&mut self) {
        // Reset the list of connections to be empty
        for point_idx in 0..(self.points.len()) {
            self.points[point_idx].connected_from.clear();
        }

        // Add a reverse connection for every edge
        for point_idx in 0..(self.points.len()) {
            for edge_idx in 0..(self.points[point_idx].forward_edges.len()) {
                let end_idx = self.points[point_idx].forward_edges[edge_idx].end_idx;
                self.points[end_idx].connected_from.push(point_idx);
            }
        }

        // Sort and deduplicate them
        for point_idx in 0..(self.points.len()) {
            self.points[point_idx].connected_from.sort_unstable();
            self.points[point_idx].connected_from.dedup();
        }
    }

    ///
    /// Returns the number of points in this graph. Points are numbered from 0 to this value.
    ///
    #[inline]
    pub fn num_points(&self) -> usize {
        self.points.len()
    }

    ///
    /// Returns an iterator of all edges in this graph
    ///
    #[inline]
    pub fn all_edges(&self) -> impl '_ + Iterator<Item = GraphEdge<'_, Point, Label>> {
        (0..(self.points.len()))
            .into_iter()
            .flat_map(move |point_num| self.edges_for_point(point_num))
    }

    ///
    /// Returns an iterator of all the edges in this graph, as references
    ///
    #[inline]
    pub fn all_edge_refs(&self) -> impl '_ + Iterator<Item = GraphEdgeRef> {
        (0..(self.points.len()))
            .into_iter()
            .flat_map(move |point_idx| {
                (0..(self.points[point_idx].forward_edges.len()))
                    .into_iter()
                    .map(move |edge_idx| GraphEdgeRef {
                        start_idx: point_idx,
                        edge_idx: edge_idx,
                        reverse: false,
                    })
            })
    }

    ///
    /// Returns an iterator of the edges that leave a particular point
    ///
    /// Edges are directional: this will provide the edges that leave the supplied point
    ///
    #[inline]
    pub fn edges_for_point(
        &self,
        point_num: usize,
    ) -> impl '_ + Iterator<Item = GraphEdge<'_, Point, Label>> {
        (0..(self.points[point_num].forward_edges.len()))
            .into_iter()
            .map(move |edge_idx| {
                GraphEdge::new(
                    self,
                    GraphEdgeRef {
                        start_idx: point_num,
                        edge_idx: edge_idx,
                        reverse: false,
                    },
                )
            })
    }

    ///
    /// Returns the edge refs for a particular point
    ///
    pub fn edge_refs_for_point(&self, point_num: usize) -> impl Iterator<Item = GraphEdgeRef> {
        (0..(self.points[point_num].forward_edges.len()))
            .into_iter()
            .map(move |edge_idx| GraphEdgeRef {
                start_idx: point_num,
                edge_idx: edge_idx,
                reverse: false,
            })
    }

    ///
    /// Returns the position of a particular point
    ///
    #[inline]
    pub fn point_position(&self, point_num: usize) -> Point {
        self.points[point_num].position
    }

    ///
    /// Returns an iterator of the edges that arrive at a particular point
    ///
    /// Edges are directional: this will provide the edges that connect to the supplied point
    ///
    pub fn reverse_edges_for_point(
        &self,
        point_num: usize,
    ) -> impl '_ + Iterator<Item = GraphEdge<'_, Point, Label>> {
        // Fetch the points that connect to this point
        self.points[point_num]
            .connected_from
            .iter()
            .flat_map(move |connected_from| {
                let connected_from = *connected_from;

                // Any edge that connects to the current point, in reverse
                (0..(self.points[connected_from].forward_edges.len()))
                    .into_iter()
                    .filter_map(move |edge_idx| {
                        if self.points[connected_from].forward_edges[edge_idx].end_idx == point_num
                        {
                            Some(GraphEdgeRef {
                                start_idx: connected_from,
                                edge_idx: edge_idx,
                                reverse: true,
                            })
                        } else {
                            None
                        }
                    })
            })
            .map(move |edge_ref| GraphEdge::new(self, edge_ref))
    }

    ///
    /// Merges in another path
    ///
    /// This adds the edges in the new path to this path without considering if they are internal or external
    ///
    pub fn merge(self, merge_path: GraphPath<Point, Label>) -> GraphPath<Point, Label> {
        // Copy the points from this graph
        let mut new_points = self.points;
        let next_path_idx = self.next_path_index;

        // Add in points from the merge path
        let offset = new_points.len();
        new_points.extend(merge_path.points.into_iter().map(|mut point| {
            // Update the offsets in the edges
            for edge in &mut point.forward_edges {
                edge.end_idx += offset;
            }

            for previous_point in &mut point.connected_from {
                *previous_point += offset;
            }

            // Generate the new edge
            point
        }));

        // Combined path
        GraphPath {
            points: new_points,
            next_path_index: next_path_idx + merge_path.next_path_index,
        }
    }

    ///
    /// Returns true if the specified edge is very short (starts and ends at the same point and does not cover a significant amount of ground)
    ///
    fn edge_is_very_short(&self, edge_ref: GraphEdgeRef) -> bool {
        let edge = &self.points[edge_ref.start_idx].forward_edges[edge_ref.edge_idx];

        if edge_ref.start_idx == edge.end_idx {
            // Find the points on this edge
            let start_point = &self.points[edge_ref.start_idx].position;
            let cp1 = &edge.cp1;
            let cp2 = &edge.cp2;
            let end_point = &self.points[edge.end_idx].position;

            // If all the points are close to each other, then this is a short edge
            start_point.is_near_to(end_point, CLOSE_DISTANCE)
                && start_point.is_near_to(cp1, CLOSE_DISTANCE)
                && cp1.is_near_to(cp2, CLOSE_DISTANCE)
                && cp2.is_near_to(end_point, CLOSE_DISTANCE)
        } else {
            false
        }
    }

    ///
    /// Removes an edge by updating the previous edge to point at its next edge
    ///
    /// Control points are not updated so the shape will be distorted if the removed edge is very long
    ///
    fn remove_edge(&mut self, edge_ref: GraphEdgeRef) {
        // Edge consistency is preserved provided that the edges are already consistent
        self.check_following_edge_consistency();

        // Find the next edge
        let next_point_idx =
            self.points[edge_ref.start_idx].forward_edges[edge_ref.edge_idx].end_idx;
        let next_edge_idx =
            self.points[edge_ref.start_idx].forward_edges[edge_ref.edge_idx].following_edge_idx;

        // Edge shouldn't just loop around to itself
        test_assert!(next_point_idx != edge_ref.start_idx || next_edge_idx != edge_ref.edge_idx);

        // ... and the preceding edge (by searching all of the connected points)
        let previous_edge_ref = self.points[edge_ref.start_idx]
            .connected_from
            .iter()
            .flat_map(|point_idx| {
                let point_idx = *point_idx;
                self.points[point_idx]
                    .forward_edges
                    .iter()
                    .enumerate()
                    .map(move |(edge_idx, edge)| (point_idx, edge_idx, edge))
            })
            .find_map(|(point_idx, edge_idx, edge)| {
                if edge.end_idx == edge_ref.start_idx
                    && edge.following_edge_idx == edge_ref.edge_idx
                {
                    Some(GraphEdgeRef {
                        start_idx: point_idx,
                        edge_idx: edge_idx,
                        reverse: false,
                    })
                } else {
                    None
                }
            });

        test_assert!(previous_edge_ref.is_some());

        if let Some(previous_edge_ref) = previous_edge_ref {
            test_assert!(
                self.points[previous_edge_ref.start_idx].forward_edges[previous_edge_ref.edge_idx]
                    .end_idx
                    == edge_ref.start_idx
            );
            test_assert!(
                self.points[previous_edge_ref.start_idx].forward_edges[previous_edge_ref.edge_idx]
                    .following_edge_idx
                    == edge_ref.edge_idx
            );

            // Reconnect the previous edge to the next edge
            self.points[previous_edge_ref.start_idx].forward_edges[previous_edge_ref.edge_idx]
                .end_idx = next_point_idx;
            self.points[previous_edge_ref.start_idx].forward_edges[previous_edge_ref.edge_idx]
                .following_edge_idx = next_edge_idx;

            // Remove the old edge from the list
            self.points[edge_ref.start_idx]
                .forward_edges
                .remove(edge_ref.edge_idx);

            // For all the connected points, update the following edge refs
            let mut still_connected = false;

            self.points[edge_ref.start_idx]
                .connected_from
                .sort_unstable();
            self.points[edge_ref.start_idx].connected_from.dedup();
            for connected_point_idx in self.points[edge_ref.start_idx].connected_from.clone() {
                for edge_idx in 0..(self.points[connected_point_idx].forward_edges.len()) {
                    let connected_edge =
                        &mut self.points[connected_point_idx].forward_edges[edge_idx];

                    // Only interested in edges on the point we just changed
                    if connected_edge.end_idx != edge_ref.start_idx {
                        continue;
                    }

                    // We should have eliminated the edge we're deleting when we updated the edge above
                    test_assert!(connected_edge.following_edge_idx != edge_ref.edge_idx);

                    // Update the following edge if it was affected by the deletion
                    if connected_edge.following_edge_idx > edge_ref.edge_idx {
                        connected_edge.following_edge_idx -= 1;
                    }

                    // If there's another edge ending at the original point, then we're still connected
                    if connected_edge.end_idx == edge_ref.start_idx {
                        still_connected = true;
                    }
                }
            }

            // If the two points are not still connected, remove the previous point from the connected list
            if !still_connected {
                self.points[edge_ref.start_idx]
                    .connected_from
                    .retain(|point_idx| *point_idx != edge_ref.start_idx);
            }

            // Edges should be consistent again
            self.check_following_edge_consistency();
        }
    }

    ///
    /// Removes any edges that appear to be 'very short' from this graph
    ///
    /// 'Very short' edges are edges that start and end at the same point and have control points very close to the start position
    ///
    fn remove_all_very_short_edges(&mut self) {
        for point_idx in 0..(self.points.len()) {
            let mut edge_idx = 0;
            while edge_idx < self.points[point_idx].forward_edges.len() {
                // Remove this edge if it's very short
                let edge_ref = GraphEdgeRef {
                    start_idx: point_idx,
                    edge_idx: edge_idx,
                    reverse: false,
                };
                if self.edge_is_very_short(edge_ref) {
                    self.remove_edge(edge_ref);
                } else {
                    // Next edge
                    edge_idx += 1;
                }
            }
        }
    }

    ///
    /// Collides this path against another, generating a merged path
    ///
    /// Anywhere this graph intersects the second graph, a point with two edges will be generated. All edges will be left as
    /// interior or exterior depending on how they're set on the graph they originate from.
    ///
    /// Working out the collision points is the first step to performing path arithmetic: the resulting graph can be altered
    /// to specify edge types - knowing if an edge is an interior or exterior edge makes it possible to tell the difference
    /// between a hole cut into a shape and an intersection.
    ///
    /// Unlike collide(), this will indicate if any collisions were detected or if the two paths merged without collisions
    ///
    pub fn collide_or_merge(
        mut self,
        collide_path: GraphPath<Point, Label>,
        accuracy: f64,
    ) -> CollidedGraphPath<Point, Label> {
        // Generate a merged path with all of the edges
        let collision_offset = self.points.len();
        self = self.merge(collide_path);

        // Search for collisions between our original path and the new one
        let total_points = self.points.len();
        if self.detect_collisions(
            0..collision_offset,
            collision_offset..total_points,
            accuracy,
        ) {
            CollidedGraphPath::Collided(self)
        } else {
            CollidedGraphPath::Merged(self)
        }
    }

    ///
    /// Collides this path against another, generating a merged path
    ///
    /// Anywhere this graph intersects the second graph, a point with two edges will be generated. All edges will be left as
    /// interior or exterior depending on how they're set on the graph they originate from.
    ///
    /// Working out the collision points is the first step to performing path arithmetic: the resulting graph can be altered
    /// to specify edge types - knowing if an edge is an interior or exterior edge makes it possible to tell the difference
    /// between a hole cut into a shape and an intersection.
    ///
    pub fn collide(
        mut self,
        collide_path: GraphPath<Point, Label>,
        accuracy: f64,
    ) -> GraphPath<Point, Label> {
        // Generate a merged path with all of the edges
        let collision_offset = self.points.len();
        self = self.merge(collide_path);

        // Search for collisions between our original path and the new one
        let total_points = self.points.len();
        self.detect_collisions(
            0..collision_offset,
            collision_offset..total_points,
            accuracy,
        );

        // Return the result
        self
    }

    ///
    /// Rounds all of the points in this path to a particular accuracy level
    ///
    pub fn round(&mut self, accuracy: f64) {
        for point_idx in 0..(self.num_points()) {
            self.points[point_idx].position.round(accuracy);

            for edge_idx in 0..(self.points[point_idx].forward_edges.len()) {
                self.points[point_idx].forward_edges[edge_idx]
                    .cp1
                    .round(accuracy);
                self.points[point_idx].forward_edges[edge_idx]
                    .cp2
                    .round(accuracy);
            }
        }
    }

    ///
    /// Finds any collisions between existing points in the graph path
    ///
    pub fn self_collide(&mut self, accuracy: f64) {
        let total_points = self.points.len();
        self.detect_collisions(0..total_points, 0..total_points, accuracy);
    }

    ///
    /// Returns the GraphEdge for an edgeref
    ///
    #[inline]
    pub fn get_edge(&self, edge: GraphEdgeRef) -> GraphEdge<'_, Point, Label> {
        GraphEdge::new(self, edge)
    }

    ///
    /// Sets the kind of a single edge
    ///
    #[inline]
    pub fn set_edge_kind(&mut self, edge: GraphEdgeRef, new_type: GraphPathEdgeKind) {
        self.points[edge.start_idx].forward_edges[edge.edge_idx].kind = new_type;
    }

    ///
    /// Sets the label of a single edge
    ///
    #[inline]
    pub fn set_edge_label(&mut self, edge: GraphEdgeRef, new_label: Label) {
        self.points[edge.start_idx].forward_edges[edge.edge_idx].label = new_label;
    }

    ///
    /// Returns the type of the edge pointed to by an edgeref
    ///
    #[inline]
    pub fn edge_kind(&self, edge: GraphEdgeRef) -> GraphPathEdgeKind {
        self.points[edge.start_idx].forward_edges[edge.edge_idx].kind
    }

    ///
    /// Returns the label of the edge pointed to by an edgeref
    ///
    #[inline]
    pub fn edge_label(&self, edge: GraphEdgeRef) -> Label {
        self.points[edge.start_idx].forward_edges[edge.edge_idx].label
    }

    ///
    /// Resets the edge kinds in this path by setting them all to uncategorised
    ///
    pub fn reset_edge_kinds(&mut self) {
        for point in self.points.iter_mut() {
            for edge in point.forward_edges.iter_mut() {
                edge.kind = GraphPathEdgeKind::Uncategorised;
            }
        }
    }

    ///
    /// Sets the kind of an edge and any connected edge where there are no intersections (only one edge)
    ///
    pub fn set_edge_kind_connected(&mut self, edge: GraphEdgeRef, kind: GraphPathEdgeKind) {
        let mut current_edge = edge;
        let mut visited = vec![false; self.points.len()];

        // Move forward
        loop {
            // Set the kind of the current edge
            self.set_edge_kind(current_edge, kind);
            visited[current_edge.start_idx] = true;

            // Pick the next edge
            let end_idx =
                self.points[current_edge.start_idx].forward_edges[current_edge.edge_idx].end_idx;
            let edges = &self.points[end_idx].forward_edges;

            if edges.len() != 1 {
                // At an intersection
                break;
            } else {
                // Move on
                current_edge = GraphEdgeRef {
                    start_idx: end_idx,
                    edge_idx: 0,
                    reverse: false,
                }
            }

            // Also stop if we've followed the loop all the way around
            if visited[current_edge.start_idx] {
                break;
            }
        }

        // Move backwards
        current_edge = edge;
        loop {
            // Mark this point as visited
            visited[current_edge.start_idx] = true;

            if self.points[current_edge.start_idx].connected_from.len() != 1 {
                // There is more than one incoming edge
                break;
            } else {
                // There's a single preceding point (but maybe more than one edge)
                let current_point_idx = current_edge.start_idx;
                let previous_point_idx = self.points[current_edge.start_idx].connected_from[0];

                // Find the index of the preceding edge
                let mut previous_edges = (0..(self.points[previous_point_idx].forward_edges.len()))
                    .into_iter()
                    .filter(|edge_idx| {
                        self.points[previous_point_idx].forward_edges[*edge_idx].end_idx
                            == current_point_idx
                    });

                let previous_edge_idx = previous_edges.next().expect("Previous edge");
                if previous_edges.next().is_some() {
                    // There is more than one edge connecting these two points
                    break;
                }

                // Move on to the next edge
                current_edge = GraphEdgeRef {
                    start_idx: previous_point_idx,
                    edge_idx: previous_edge_idx,
                    reverse: false,
                };

                // Change its kind
                self.set_edge_kind(current_edge, kind);
            }

            // Also stop if we've followed the loop all the way around
            if visited[current_edge.start_idx] {
                break;
            }
        }
    }

    ///
    /// Returns true if the specified edge has a gap (end point has no following exterior edge)
    ///
    fn edge_has_gap(&self, edge: GraphEdgeRef) -> bool {
        // Interior edges have no gaps
        if self.points[edge.start_idx].forward_edges[edge.edge_idx].kind
            != GraphPathEdgeKind::Exterior
        {
            false
        } else {
            // Get the end point index for this edge
            let (start_idx, end_idx) = if edge.reverse {
                (
                    self.points[edge.start_idx].forward_edges[edge.edge_idx].end_idx,
                    edge.start_idx,
                )
            } else {
                (
                    edge.start_idx,
                    self.points[edge.start_idx].forward_edges[edge.edge_idx].end_idx,
                )
            };

            // Result is true if there is no edge attached to the end point that is marked exterior (other than the edge leading back to the initial point)
            !self
                .edges_for_point(end_idx)
                .chain(self.reverse_edges_for_point(end_idx))
                .filter(|following_edge| following_edge.end_point_index() != start_idx)
                .any(|following_edge| following_edge.kind() == GraphPathEdgeKind::Exterior)
        }
    }

    ///
    /// Given an edge that ends in a gap, attempts to bridge the gap by finding a following edge that has no following exterior edges on
    /// its start point.
    ///
    fn heal_edge_with_gap(&mut self, point_idx: usize, edge_idx: usize, max_depth: usize) -> bool {
        // This is Dijsktra's algorithm again: we also use this for a similar purpose in exterior_paths
        let end_point_idx = self.points[point_idx].forward_edges[edge_idx].end_idx;

        // State of the algorithm
        let mut preceding_edge = vec![None; self.points.len()];
        let mut points_to_process = vec![(point_idx, end_point_idx)];
        let mut current_depth = 0;
        let mut target_point_idx = None;

        // Iterate until we hit the maximum depth
        while current_depth < max_depth && target_point_idx.is_none() {
            // Points found in this pass that need to be checked
            let mut next_points_to_process = vec![];

            // Process all the points found in the previous pass
            for (from_point_idx, next_point_idx) in points_to_process {
                // Stop once we find a point
                if target_point_idx.is_some() {
                    break;
                }

                // Process all edges connected to this point
                for next_edge in self.edges_for_point(next_point_idx)
                /*.chain(self.reverse_edges_for_point(next_point_idx)) */
                {
                    let edge_end_point_idx = next_edge.end_point_index();
                    let next_edge_ref = GraphEdgeRef::from(&next_edge);
                    let edge_start_idx = next_edge.start_point_index();

                    // Don't go back the way we came
                    if edge_end_point_idx == from_point_idx {
                        continue;
                    }

                    // Don't revisit points we already have a trail for
                    if preceding_edge[edge_end_point_idx].is_some() {
                        continue;
                    }

                    // Ignore exterior edges (except exterior edges where edge_has_gap is true, which indicate we've crossed our gap)
                    let mut reversed_edge_ref = next_edge_ref;
                    reversed_edge_ref.reverse = !reversed_edge_ref.reverse;
                    if next_edge.kind() == GraphPathEdgeKind::Exterior
                        && !self.edge_has_gap(reversed_edge_ref)
                    {
                        continue;
                    }

                    // Add this as a preceding edge
                    preceding_edge[edge_end_point_idx] = Some(next_edge_ref);

                    // We've found a path across the gap if we find an exterior edge
                    if next_edge.kind() == GraphPathEdgeKind::Exterior {
                        // Set this as the target point
                        target_point_idx = Some(edge_end_point_idx);
                        break;
                    }

                    // Continue searching from this point
                    next_points_to_process.push((edge_start_idx, edge_end_point_idx));
                }
            }

            // Process any points we found in the next pass
            points_to_process = next_points_to_process;

            // Moved down a level in the graph
            current_depth += 1;
        }

        if let Some(target_point_idx) = target_point_idx {
            // Target_point represents the final point in the
            let mut current_point_idx = target_point_idx;

            while current_point_idx != end_point_idx {
                let previous_edge_ref =
                    preceding_edge[current_point_idx].expect("Previous point during gap healing");

                // Mark this edge as exterior
                self.points[previous_edge_ref.start_idx].forward_edges
                    [previous_edge_ref.edge_idx]
                    .kind = GraphPathEdgeKind::Exterior;

                // Move to the previous point
                let previous_edge = self.get_edge(previous_edge_ref);
                current_point_idx = previous_edge.start_point_index();
            }

            true
        } else {
            // Failed to cross the gap
            false
        }
    }

    ///
    /// Finds any gaps in the edges marked as exterior and attempts to 'heal' them by finding a route to another
    /// part of the path with a missing edge
    ///
    /// Returns true if all the gaps that were found were 'healed'
    ///
    pub fn heal_exterior_gaps(&mut self) -> bool {
        let mut all_healed = true;

        // Iterate over all the edges in this graph
        for point_idx in 0..(self.points.len()) {
            for edge_idx in 0..(self.points[point_idx].forward_edges.len()) {
                // If this edge has a gap...
                if self.edge_has_gap(GraphEdgeRef {
                    start_idx: point_idx,
                    edge_idx: edge_idx,
                    reverse: false,
                }) {
                    // ... try to heal it
                    if !self.heal_edge_with_gap(point_idx, edge_idx, MAX_HEAL_DEPTH) {
                        all_healed = false;
                    }
                }
            }
        }

        all_healed
    }

    ///
    /// Generates a description of all the external connections of this graph
    ///
    #[inline]
    fn all_exterior_connections(&self) -> Vec<SmallVec<[(usize, GraphEdgeRef); 4]>> {
        // Create a copy of all the 'exterior' edge connections in this graph
        // Max of 4 connections per point is typical for 2 merged paths
        let mut connections = vec![smallvec![]; self.num_points()];

        for (point_idx, point) in self.points.iter().enumerate() {
            for (edge_idx, edge) in point.forward_edges.iter().enumerate() {
                // Only add exterior edges
                if edge.kind == GraphPathEdgeKind::Exterior {
                    // Add both the forward and backwards indexes, if two edges reach the same point we add both edges (including the case where an edge reconnects to the same point)
                    let edge_ref = GraphEdgeRef {
                        start_idx: point_idx,
                        edge_idx: edge_idx,
                        reverse: false,
                    };

                    connections[point_idx].push((edge.end_idx, edge_ref));
                    connections[edge.end_idx].push((point_idx, edge_ref));
                }
            }
        }

        connections
    }

    ///
    /// Given the index of a starting edge in a connections list, attempts to find the shortest loop of edges that returns
    /// back to the edge's start point
    ///
    /// If there's a choice, this will not follow any previously used edge (but will follow them if that's the way to make progress with a loop of edges)
    ///
    fn find_loop(
        &self,
        connections: &Vec<SmallVec<[(usize, GraphEdgeRef); 4]>>,
        start_point_idx: usize,
        edge: usize,
        used_edges: &Vec<u64>,
    ) -> Option<Vec<(usize, GraphEdgeRef)>> {
        // The algorithm here is a slight modification of Dijkstra's algorithm, we start knowing the path has to contain a particular edge
        let mut previous_point = vec![None; connections.len()];

        // Mark the previous point of the edge we're checking as visited
        let (next_point_idx, edge_ref) = connections[start_point_idx][edge];
        previous_point[next_point_idx] = Some((start_point_idx, edge_ref));

        // We keep a stack of points to visit next
        let mut points_to_check = VecDeque::new();
        points_to_check.push_front((next_point_idx, edge_ref));

        // Flags indicating which edges are visited for each point (allows up to 32 edges per point, will malfunction beyond that point)
        let mut visited_edges = vec![0u64; self.num_points()];

        // Visit connected points until we find a loop or run out of unvisited connections
        loop {
            let (next_point_idx, edge_ref) = if let Some(point_idx) = points_to_check.pop_back() {
                point_idx
            } else {
                // Ran out of points to check without finding a loop
                return None;
            };

            // We've found a loop if we've found a path arriving back at the start index
            if next_point_idx == start_point_idx {
                break;
            }

            // If this edge was already visited, don't visit it again
            if (visited_edges[edge_ref.start_idx] & (1 << (edge_ref.edge_idx))) != 0 {
                continue;
            }

            // Mark this edge as visited
            visited_edges[edge_ref.start_idx] |= 1 << (edge_ref.edge_idx);

            // Visit all the points reachable from this edge, ignoring any edges that we've already visited
            let following_connections = &connections[next_point_idx];

            // Check the 'already used' list only if there are no alternative edges from this point
            let avoid_already_used = following_connections.len() > 1;

            for (following_point_idx, following_edge) in following_connections.iter() {
                // Don't follow visited edges
                if visited_edges[following_edge.start_idx] & (1 << following_edge.edge_idx) != 0 {
                    continue;
                }

                // Also avoid edges used for previous shapes unless they are the only way to make progress
                if avoid_already_used
                    && used_edges[following_edge.start_idx] & (1 << following_edge.edge_idx) != 0
                {
                    continue;
                }

                // Update the previous point for this point
                if previous_point[*following_point_idx].is_none() {
                    previous_point[*following_point_idx] = Some((next_point_idx, *following_edge));
                }

                // Visit along this edge next
                points_to_check.push_front((*following_point_idx, *following_edge));
            }
        }

        // We should find a loop by repeatedly reading the previous point from the start index
        let mut loop_points = vec![];
        let mut pos = start_point_idx;

        loop {
            // Get the point preceding the current point
            let (previous_idx, previous_edge) = previous_point[pos].unwrap();

            // Add to the loop, using the 'forward' variant of the edge
            loop_points.push((pos, previous_edge));

            // Update to the earlier point
            pos = previous_idx;

            // Stop once we reach the start point again (ie, after we've followed the entire loop)
            if previous_idx == start_point_idx {
                break;
            }
        }

        return Some(loop_points);
    }

    ///
    /// Given a set of connected edges (can be connected in any direction), generates a suitable path
    ///
    #[inline]
    fn generate_path<POut>(&self, edges: Vec<(usize, GraphEdgeRef)>) -> POut
    where
        POut: BezierPathFactory<Point = Point>,
    {
        // Build up a list of path points
        let mut path_points = vec![];

        let num_edges = edges.len();

        // Add each edge to the path in turn
        for idx in 0..num_edges {
            let next_idx = idx + 1;
            let next_idx = if next_idx >= num_edges { 0 } else { next_idx };

            // We need the edge and the following edge (we want to represent the edge in the path as the edge that connects edge.start_idx to next_edge.start_idx)
            let (_point_idx, edge) = &edges[idx];
            let (next_point_idx, _next_edge) = &edges[next_idx];

            if self.points[edge.start_idx].forward_edges[edge.edge_idx].end_idx == *next_point_idx {
                // Edge is a forward edge
                let forward_edge = &self.points[edge.start_idx].forward_edges[edge.edge_idx];

                let end_point = self.points[forward_edge.end_idx].position;
                let cp1 = forward_edge.cp1;
                let cp2 = forward_edge.cp2;

                path_points.push((cp1, cp2, end_point));
            } else {
                // Edge is a backwards edge (from edge.start_idx to next_edge.start_idx)
                let backwards_edge = &self.points[edge.start_idx].forward_edges[edge.edge_idx];
                debug_assert!(backwards_edge.end_idx == *_point_idx);

                let end_point = self.points[edge.start_idx].position;
                let cp1 = backwards_edge.cp2;
                let cp2 = backwards_edge.cp1;

                path_points.push((cp1, cp2, end_point));
            }
        }

        // Start point matches end point
        let start_point = path_points.last().unwrap().2;
        POut::from_points(start_point, path_points)
    }

    ///
    /// Finds the connected loops of exterior edges and turns them into a series of paths
    ///
    pub fn exterior_paths<POut>(&self) -> Vec<POut>
    where
        POut: BezierPathFactory<Point = Point>,
    {
        let mut paths = vec![];

        // Get the graph of exterior connections for the graph
        let connections = self.all_exterior_connections();

        // Order points by x then y index (ie, generate paths by sweeping from left to right)
        // This is to try to ensure that paths are matched from outside in: when there are paths that share vertices, just finding loops
        // is insufficient to always build a valid result (it's possible to generate paths that share all of their edges, which will fill
        // or clear path sections incorrectly)
        //
        // We try to avoid re-using edges but will re-use an edge to generate a loop if that's the only way, which can cause this same
        // problem to show up. Ordering like this reduces the incidence of this issue by making it so we find paths by working inwards
        // instead of randomly (though a most of the time this issue does not occur, so this is wasted effort, though having outer paths
        // come before inner paths is a side-benefit)
        let mut points = (0..self.points.len()).into_iter().collect::<Vec<_>>();
        points.sort_by(|point_a, point_b| {
            use std::cmp::Ordering;

            let x_a = self.points[*point_a].position.x();
            let x_b = self.points[*point_b].position.x();

            if (x_a - x_b).abs() < 0.01 {
                let y_a = self.points[*point_a].position.y();
                let y_b = self.points[*point_b].position.y();

                y_a.partial_cmp(&y_b).unwrap_or(Ordering::Equal)
            } else if x_a < x_b {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        });

        // Store a list of edges that have been visited or are already in a path (these are flags: up to 32 edges per point are allowed by this algorithm)
        // Even a complex path very rarely has more than 2 edges per point
        let mut included_edges = vec![0u64; self.num_points()];

        // Each connection describes the exterior edges for a point
        for (point_idx, edge_list) in points
            .into_iter()
            .map(|point_idx| (point_idx, &connections[point_idx]))
        {
            for (edge_idx, (_end_point_idx, edge_ref)) in edge_list.iter().enumerate() {
                // Ignore visited/included edges
                if included_edges[edge_ref.start_idx] & (1 << edge_ref.edge_idx) != 0 {
                    continue;
                }

                // Mark the edge as included
                debug_assert!(edge_ref.edge_idx < 64);
                included_edges[edge_ref.start_idx] |= 1 << edge_ref.edge_idx;

                // Try to find a loop from this edge
                let loop_edges = if let Some(loop_edges) =
                    self.find_loop(&connections, point_idx, edge_idx, &included_edges)
                {
                    // Loop was found without any re-used edges
                    Some(loop_edges)
                } else {
                    // Loop was found with some re-used edges
                    // TODO: this can produce bad path results when it occurs: see comment above
                    self.find_loop(
                        &connections,
                        point_idx,
                        edge_idx,
                        &vec![0u64; self.num_points()],
                    )
                };

                if let Some(loop_edges) = loop_edges {
                    // Mark all the loop edges as visited
                    for (_, edge) in loop_edges.iter() {
                        included_edges[edge.start_idx] |= 1 << edge.edge_idx;
                    }

                    // Generate a loop and add it to the paths
                    paths.push(self.generate_path(loop_edges));
                }
            }
        }

        paths
    }
}

///
/// Represents an edge in a graph path
///
#[derive(Clone)]
pub struct GraphEdge<'a, Point: 'a, Label: 'a> {
    /// The graph that this point is for
    graph: &'a GraphPath<Point, Label>,

    /// A reference to the edge this point is for
    edge: GraphEdgeRef,
}

impl<Point: Coordinate2D + Coordinate + fmt::Debug, Label: Copy> fmt::Debug
    for GraphPath<Point, Label>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for point_idx in 0..(self.points.len()) {
            write!(f, "\nPoint {:?}:", point_idx)?;

            for edge in self.edges_for_point(point_idx) {
                write!(f, "\n  {:?}", edge)?;
            }
        }

        Ok(())
    }
}
