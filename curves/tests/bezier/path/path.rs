/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use flo_curves::bezier::path::*;
use flo_curves::*;

#[test]
fn reverse_rectangle() {
    let rectangle = BezierPathBuilder::<SimpleBezierPath>::start(Coord2(1.0, 1.0))
        .line_to(Coord2(1.0, 5.0))
        .line_to(Coord2(5.0, 5.0))
        .line_to(Coord2(5.0, 1.0))
        .line_to(Coord2(1.0, 1.0))
        .build();

    let reversed = rectangle.reversed::<SimpleBezierPath>();

    assert!(reversed.start_point() == Coord2(1.0, 1.0));

    let points = reversed.points().collect::<Vec<_>>();
    assert!(points.len() == 4);
    assert!(points[0].2 == Coord2(5.0, 1.0));
    assert!(points[1].2 == Coord2(5.0, 5.0));
    assert!(points[2].2 == Coord2(1.0, 5.0));
    assert!(points[3].2 == Coord2(1.0, 1.0));
}

#[test]
fn reverse_unclosed_rectangle() {
    let rectangle = BezierPathBuilder::<SimpleBezierPath>::start(Coord2(1.0, 1.0))
        .line_to(Coord2(1.0, 5.0))
        .line_to(Coord2(5.0, 5.0))
        .line_to(Coord2(5.0, 1.0))
        .build();

    let reversed = rectangle.reversed::<SimpleBezierPath>();

    assert!(reversed.start_point() == Coord2(5.0, 1.0));

    let points = reversed.points().collect::<Vec<_>>();
    assert!(points.len() == 3);
    assert!(points[0].2 == Coord2(5.0, 5.0));
    assert!(points[1].2 == Coord2(1.0, 5.0));
    assert!(points[2].2 == Coord2(1.0, 1.0));
}
