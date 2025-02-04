/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::column_sampled_contour::*;
use super::distance_field::*;
use super::sampled_contour::*;
use crate::geo::*;

use smallvec::*;

use std::ops::Range;

///
/// A distance field to a circle with a particular radius
///
#[derive(Clone, Copy, PartialEq)]
pub struct CircularDistanceField {
    radius: f64,
    center_x: f64,
    center_y: f64,
    diameter: usize,
}

impl CircularDistanceField {
    ///
    /// Creates a new sampled distance field for a circle with the specified radius
    ///
    #[inline]
    pub fn with_radius(radius: f64) -> Self {
        let radius = if radius < 0.0 { 0.0 } else { radius };
        let center = radius.ceil() + 1.0;
        let diameter = (center as usize) * 2 + 1;

        CircularDistanceField {
            radius: radius,
            center_x: center,
            center_y: center,
            diameter: diameter,
        }
    }

    ///
    /// Gives the circle a non-linear offset, from between 0.0 to 1.0
    ///
    #[inline]
    pub fn with_center_offset(self, x: f64, y: f64) -> Self {
        let center_x = self.center_x + x;
        let center_y = self.center_y + y;

        CircularDistanceField {
            radius: self.radius,
            center_x: center_x,
            center_y: center_y,
            diameter: ((center_x.max(center_y)).floor() as usize) * 2 + 1,
        }
    }

    ///
    /// Returns a circular distance field and an offset that will create a circle centered at the specified position
    ///
    /// All of the points within the resulting circle must be at positive coordinates (ie, `x-radius` and `y-radius` must
    /// be positive values). This is intended to be used as input to the `DaubBrushDistanceField` type to create brush
    /// strokes out of many circle.
    ///
    pub fn centered_at_position(
        pos: impl Coordinate + Coordinate2D,
        radius: f64,
    ) -> Option<(CircularDistanceField, ContourPosition)> {
        if radius <= 0.0 {
            return None;
        }

        let circle = CircularDistanceField::with_radius(radius);

        let x = pos.x() - circle.center_x - 1.0;
        let y = pos.y() - circle.center_y - 1.0;

        debug_assert!(
            x >= 0.0,
            "x {}-{}-1 < 0.0 ({})",
            pos.x(),
            circle.center_x,
            x
        );
        debug_assert!(
            y >= 0.0,
            "y {}-{}-1 < 0.0 ({})",
            pos.y(),
            circle.center_y,
            y
        );

        if x < 0.0 || y < 0.0 {
            return None;
        }

        let offset_x = x - x.floor();
        let offset_y = y - y.floor();

        let circle = circle.with_center_offset(offset_x, offset_y);
        let position = ContourPosition(x.floor() as usize, y.floor() as usize);

        Some((circle, position))
    }
}

impl SampledContour for CircularDistanceField {
    #[inline]
    fn contour_size(&self) -> ContourSize {
        ContourSize(self.diameter, self.diameter)
    }

    #[inline]
    fn intercepts_on_line(&self, ypos: f64) -> SmallVec<[Range<f64>; 4]> {
        let y = ypos - self.center_y;

        if y.abs() <= self.radius {
            let intercept = ((self.radius * self.radius) - (y * y)).sqrt();
            let min_x = self.center_x - intercept;
            let max_x = self.center_x + intercept;

            smallvec![min_x..max_x]
        } else {
            smallvec![]
        }
    }
}

impl ColumnSampledContour for CircularDistanceField {
    #[inline]
    fn intercepts_on_column(&self, xpos: f64) -> SmallVec<[Range<f64>; 4]> {
        let x = xpos - self.center_x;

        if x.abs() <= self.radius {
            let intercept = ((self.radius * self.radius) - (x * x)).sqrt();
            let min_y = self.center_y - intercept;
            let max_y = self.center_y + intercept;

            smallvec![min_y..max_y]
        } else {
            smallvec![]
        }
    }
}

impl SampledSignedDistanceField for CircularDistanceField {
    type Contour = CircularDistanceField;

    #[inline]
    fn field_size(&self) -> ContourSize {
        ContourSize(self.diameter, self.diameter)
    }

    fn distance_at_point(&self, pos: ContourPosition) -> f64 {
        let pos_x = pos.0 as f64;
        let pos_y = pos.1 as f64;
        let offset_x = pos_x - self.center_x;
        let offset_y = pos_y - self.center_y;

        (offset_x * offset_x + offset_y * offset_y).sqrt() - self.radius
    }

    #[inline]
    fn as_contour<'a>(&'a self) -> &'a Self::Contour {
        self
    }
}
