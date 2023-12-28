/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::super::geo::*;
use super::curve::*;
use super::normal::*;
use super::offset_lms::*;

///
/// Computes a series of curves that approximate an offset curve from the specified origin curve.
///
pub fn offset<Curve>(curve: &Curve, initial_offset: f64, final_offset: f64) -> Vec<Curve>
where
    Curve: BezierCurveFactory + NormalCurve,
    Curve::Point: Normalize + Coordinate2D,
{
    offset_lms_sampling(
        curve,
        move |t| (final_offset - initial_offset) * t + initial_offset,
        |_| 0.0,
        32,
        0.1,
    )
    .unwrap_or_else(|| vec![])
}
