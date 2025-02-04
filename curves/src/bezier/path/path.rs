/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![allow(clippy::needless_collect)] // Required for lifetime and type inference reasons, I think. At least, clippy's suggestion does not compile here.

use super::super::super::geo::*;
use super::super::curve::*;
use super::bounds::*;
use super::to_curves::*;

use itertools::*;
use std::iter;
use std::vec;

///
/// Trait representing a path made out of bezier sections
///
pub trait BezierPath: Geo + Clone + Sized {
    /// Type of an iterator over the points in this curve. This tuple contains the points ordered as a hull: ie, two control points followed by a point on the curve
    type PointIter: Iterator<Item = (Self::Point, Self::Point, Self::Point)>;

    ///
    /// Retrieves the initial point of this path
    ///
    fn start_point(&self) -> Self::Point;

    ///
    /// Retrieves an iterator over the points in this path
    ///
    fn points(&self) -> Self::PointIter;

    ///
    /// Finds the bounds of this path
    ///
    #[inline]
    fn bounding_box<Bounds: BoundingBox<Point = Self::Point>>(&self) -> Bounds {
        path_bounding_box(self)
    }

    ///
    /// Finds a loose bounding box for this path (more quickly than bounding_box)
    ///
    /// This will contain the path but might not be tightly aligned to the curves
    ///
    fn fast_bounding_box<Bounds: BoundingBox<Point = Self::Point>>(&self) -> Bounds {
        path_fast_bounding_box(self)
    }

    ///
    /// Changes this path into a set of bezier curves
    ///
    #[inline]
    fn to_curves<Curve: BezierCurveFactory<Point = Self::Point>>(&self) -> Vec<Curve> {
        path_to_curves(self).collect()
    }

    ///
    /// Creates a reversed version of this path
    ///
    fn reversed<POut>(&self) -> POut
    where
        POut: BezierPathFactory<Point = Self::Point>,
    {
        // Add in the first point (control points don't matter)
        let fake_first_point = (
            Self::Point::origin(),
            Self::Point::origin(),
            self.start_point(),
        );
        let points = self.points();
        let points = iter::once(fake_first_point).chain(points);

        // Reverse the direction of the path
        let mut end_point = self.start_point();
        let mut reversed_points = vec![];

        for ((_, _, start_point), (cp1, cp2, last_point)) in points.tuple_windows() {
            end_point = last_point;

            reversed_points.push((cp2, cp1, start_point));
        }

        POut::from_points(end_point, reversed_points.into_iter().rev())
    }

    ///
    /// Creates a new path from this one by mapping the points using an arbitrary function
    ///
    #[inline]
    fn map_points<POut>(&self, map_fn: impl Fn(Self::Point) -> Self::Point) -> POut
    where
        POut: BezierPathFactory<Point = Self::Point>,
    {
        let start_point = map_fn(self.start_point());

        POut::from_points(
            start_point,
            self.points()
                .map(|(p1, p2, p3)| (map_fn(p1), map_fn(p2), map_fn(p3))),
        )
    }

    ///
    /// Creates a new path by adding a coordinate to all of the control points of this one
    ///
    #[inline]
    fn with_offset<POut>(&self, offset: Self::Point) -> POut
    where
        POut: BezierPathFactory<Point = Self::Point>,
    {
        self.map_points(|p| p + offset)
    }
}

///
/// Trait implemented by types that can construct new bezier paths
///
pub trait BezierPathFactory: BezierPath {
    ///
    /// Creates a new instance of this path from a set of points. The input here is a start point, followed by a set of points of the form
    /// (control_point_1, control_point_2, end_point)
    ///
    fn from_points<FromIter: IntoIterator<Item = (Self::Point, Self::Point, Self::Point)>>(
        start_point: Self::Point,
        points: FromIter,
    ) -> Self;

    ///
    /// Creates a new instance of this path from the points in another path
    ///
    fn from_path<FromPath: BezierPath<Point = Self::Point>>(path: &FromPath) -> Self {
        Self::from_points(path.start_point(), path.points())
    }

    ///
    /// Creates a new instance of this path from a list of curves
    ///
    /// Only the start point of the first curve will be used
    ///
    fn from_connected_curves<TCurve: BezierCurve<Point = Self::Point>>(
        curves: impl IntoIterator<Item = TCurve>,
    ) -> Self {
        let mut curves = curves.into_iter();

        // Peek at the first curve to get the start point
        let first_curve = curves.next();
        let start_point = first_curve
            .as_ref()
            .map(|c| c.start_point())
            .unwrap_or_else(|| Self::Point::origin());

        // Put the start curve back again and generate the path by reading the other points of the curves
        let points = first_curve.into_iter().chain(curves).map(|curve| {
            let (_, (cp1, cp2), ep) = curve.all_points();
            (cp1, cp2, ep)
        });

        Self::from_points(start_point, points)
    }
}

impl<Point: Clone + Coordinate> Geo for (Point, Vec<(Point, Point, Point)>) {
    type Point = Point;
}

///
/// The type (start_point, Vec<(Point, Point, Point)>) is the simplest bezier path type
///
impl<Point: Clone + Coordinate> BezierPath for (Point, Vec<(Point, Point, Point)>) {
    type PointIter = vec::IntoIter<(Point, Point, Point)>;

    ///
    /// Retrieves the initial point of this path
    ///
    fn start_point(&self) -> Self::Point {
        self.0
    }

    ///
    /// Retrieves an iterator over the points in this path
    ///
    fn points(&self) -> Self::PointIter {
        self.1.clone().into_iter()
    }
}

impl<Point: Clone + Coordinate> BezierPathFactory for (Point, Vec<(Point, Point, Point)>) {
    ///
    /// Creates a new instance of this path from a set of points
    ///
    fn from_points<FromIter: IntoIterator<Item = (Self::Point, Self::Point, Self::Point)>>(
        start_point: Self::Point,
        points: FromIter,
    ) -> Self {
        (start_point, points.into_iter().collect())
    }
}

/// Basic Bezier path type
pub type SimpleBezierPath = (Coord2, Vec<(Coord2, Coord2, Coord2)>);

/// Basic Bezier path type using 3D coordinates
pub type SimpleBezierPath3 = (Coord3, Vec<(Coord3, Coord3, Coord3)>);
