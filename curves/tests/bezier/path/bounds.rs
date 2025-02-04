/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use flo_curves::arc::*;
use flo_curves::bezier::path::*;
use flo_curves::*;

#[test]
fn circle_path_bounds() {
    let center = Coord2(5.0, 5.0);
    let radius = 4.0;

    // Create a path from a circle
    let circle: SimpleBezierPath = Circle::new(center, radius).to_path();

    let bounds: (Coord2, Coord2) = circle.bounding_box();

    assert!(bounds.0.distance_to(&Coord2(1.0, 1.0)) < 0.1);
    assert!(bounds.1.distance_to(&Coord2(9.0, 9.0)) < 0.1);
}

#[test]
fn circle_path_fast_bounds() {
    let center = Coord2(5.0, 5.0);
    let radius = 4.0;

    // Create a path from a circle
    let circle: SimpleBezierPath = Circle::new(center, radius).to_path();

    let bounds: (Coord2, Coord2) = circle.fast_bounding_box();

    assert!(bounds.0.x() <= 1.0);
    assert!(bounds.0.y() <= 1.0);
    assert!(bounds.1.x() >= 9.0);
    assert!(bounds.1.y() >= 9.0);
}
