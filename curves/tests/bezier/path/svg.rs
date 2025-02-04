/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use flo_curves::bezier::path::*;
use flo_curves::geo::*;

use std::fmt::Write;

pub fn svg_path_string<Path: BezierPath>(path: &Path) -> String
where
    Path::Point: Coordinate2D,
{
    let mut svg = String::new();

    write!(
        &mut svg,
        "M {} {}",
        path.start_point().x(),
        path.start_point().y()
    )
    .unwrap();
    for (cp1, cp2, end) in path.points() {
        write!(
            &mut svg,
            " C {} {}, {} {}, {} {}",
            cp1.x(),
            cp1.y(),
            cp2.x(),
            cp2.y(),
            end.x(),
            end.y()
        )
        .unwrap();
    }

    svg
}
