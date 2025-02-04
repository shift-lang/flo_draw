/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::super::super::geo::*;
use super::super::curve::*;
use super::path::*;
use super::to_curves::*;

///
/// Finds the bounds of a path
///
pub fn path_bounding_box<P: BezierPath, Bounds: BoundingBox<Point = P::Point>>(path: &P) -> Bounds {
    path_to_curves(path)
        .map(|curve: Curve<P::Point>| curve.bounding_box())
        .reduce(|first: Bounds, second| first.union_bounds(second))
        .unwrap_or_else(|| Bounds::from_min_max(P::Point::origin(), P::Point::origin()))
}

///
/// Finds the bounds of a path using the looser 'fast' algorithm
///
pub fn path_fast_bounding_box<P: BezierPath, Bounds: BoundingBox<Point = P::Point>>(
    path: &P,
) -> Bounds {
    path_to_curves(path)
        .map(|curve: Curve<P::Point>| curve.fast_bounding_box())
        .reduce(|first: Bounds, second| first.union_bounds(second))
        .unwrap_or_else(|| Bounds::from_min_max(P::Point::origin(), P::Point::origin()))
}
