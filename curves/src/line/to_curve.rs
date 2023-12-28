/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::super::bezier::*;
use super::line::*;

///
/// Changes a line to a bezier curve
///
pub fn line_to_bezier<Curve: BezierCurveFactory>(line: &impl Line<Point = Curve::Point>) -> Curve {
    let points = line.points();
    let point_distance = points.1 - points.0;
    let (cp1, cp2) = (
        points.0 + point_distance * 0.3333,
        points.0 + point_distance * 0.6666,
    );

    Curve::from_points(points.0, (cp1, cp2), points.1)
}
