/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::super::super::super::geo::*;
use super::super::graph_path::*;
use super::super::path::*;
use super::ray_cast::*;

///
/// Adds multiple paths in a single operation
///
pub fn path_add_chain<POut>(
    paths: &Vec<Vec<impl BezierPath<Point = POut::Point>>>,
    accuracy: f64,
) -> Vec<POut>
where
    POut: BezierPathFactory,
    POut::Point: Coordinate + Coordinate2D,
{
    // Build up the graph path from the supplied list
    let mut merged_path = GraphPath::new();

    for (path_idx, path) in paths.iter().enumerate() {
        let path_idx = path_idx as u32;
        merged_path = merged_path.collide(
            GraphPath::from_merged_paths(path.iter().map(|path| (path, PathLabel(path_idx)))),
            accuracy,
        );
    }

    merged_path.round(accuracy);

    // Set the exterior edges using the 'add' algorithm (all edges are considered 'external' here)
    merged_path.set_edge_kinds_by_ray_casting(|path_crossings| {
        for count in path_crossings.iter() {
            if (count & 1) != 0 {
                return true;
            }
        }

        false
    });
    merged_path.heal_exterior_gaps();

    // Produce the final result
    merged_path.exterior_paths()
}
