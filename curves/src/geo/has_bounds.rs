/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::bounding_box::*;
use super::geo::*;

///
/// Trait implemented by types that have a bounding box associated with them
///
pub trait HasBoundingBox: Geo {
    ///
    /// Returns the bounding box that encloses this item
    ///
    fn get_bounding_box<Bounds: BoundingBox<Point = Self::Point>>(&self) -> Bounds;
}
