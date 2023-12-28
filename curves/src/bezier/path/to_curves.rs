/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::super::curve::*;
use super::path::*;

use itertools::*;

///
/// Converts a path to a series of bezier curves
///
pub fn path_to_curves<Path: BezierPath, Curve: BezierCurveFactory<Point = Path::Point>>(
    path: &Path,
) -> impl Iterator<Item = Curve> {
    let just_start_point =
        vec![(path.start_point(), path.start_point(), path.start_point())].into_iter();
    let points = path.points();

    just_start_point.chain(points).tuple_windows().map(
        |((_, _, start_point), (cp1, cp2, end_point))| {
            Curve::from_points(start_point, (cp1, cp2), end_point)
        },
    )
}
