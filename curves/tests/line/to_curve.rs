/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use flo_curves::bezier::*;
use flo_curves::line::*;
use flo_curves::*;

#[test]
fn convert_line_to_bezier_curve() {
    let line = (Coord2(10.0, 20.0), Coord2(40.0, 30.0));
    let curve = line_to_bezier::<Curve<_>>(&line);

    assert!(curve.start_point == Coord2(10.0, 20.0));
    assert!(curve.end_point == Coord2(40.0, 30.0));
    assert!(curve.control_points.0.distance_to(&Coord2(20.0, 23.33)) < 0.1);
    assert!(curve.control_points.1.distance_to(&Coord2(30.0, 26.66)) < 0.1);
}
