/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use flo_curves::bezier;
use flo_curves::*;

#[test]
fn calculate_tangent_for_straight_line() {
    let straight_line = bezier::Curve::from_points(
        Coord2(0.0, 1.0),
        (Coord2(0.5, 1.5), Coord2(1.5, 2.5)),
        Coord2(2.0, 3.0),
    );
    let tangent = bezier::Tangent::from(&straight_line);

    assert!(tangent.tangent(0.5) == Coord2(2.25, 2.25));

    assert!(tangent.tangent(0.0).x() == tangent.tangent(0.0).y());
    assert!(tangent.tangent(0.5).x() == tangent.tangent(0.5).y());
    assert!(tangent.tangent(0.7).x() == tangent.tangent(0.7).y());
    assert!(tangent.tangent(1.0).x() == tangent.tangent(1.0).y());
}
