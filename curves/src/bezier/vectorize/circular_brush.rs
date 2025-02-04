/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::brush_stroke::*;
use super::circular_distance_field::*;
use super::sampled_contour::*;
use crate::geo::*;

///
/// A brush distance field that can be used to create brush strokes made up of variable-radius circles
///
pub struct CircularBrush;

impl DaubBrush for CircularBrush {
    type DaubDistanceField = CircularDistanceField;

    #[inline]
    fn create_daub(
        &self,
        centered_at: impl Coordinate + Coordinate2D,
        radius: f64,
    ) -> Option<(CircularDistanceField, ContourPosition)> {
        CircularDistanceField::centered_at_position(centered_at, radius)
    }
}
