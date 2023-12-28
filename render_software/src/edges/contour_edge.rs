/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::sync::*;

use smallvec::*;

use flo_canvas as canvas;
use flo_canvas::curves::bezier::vectorize::*;

use crate::edgeplan::*;

///
/// A contour edge provides an implementation of an edge for any type that implements the `SampledContour` trait
///
pub struct ContourEdge<TContour>
where
    TContour: 'static + Clone + SampledContour,
{
    /// The offset of the corner of where the contour should appear in space
    corner_offset: (f64, f64),

    /// The ID of the shape that this contour surrounds
    shape_id: ShapeId,

    /// The contour itself
    contour: TContour,
}

impl<TContour> ContourEdge<TContour>
where
    TContour: 'static + Clone + SampledContour,
{
    ///
    /// Creates a new edge description from a sampled contour
    ///
    pub fn new(corner_offset: (f64, f64), shape_id: ShapeId, contour: TContour) -> Self {
        ContourEdge {
            corner_offset,
            shape_id,
            contour,
        }
    }
}

impl<TContour> Clone for ContourEdge<TContour>
where
    TContour: 'static + Clone + SampledContour,
{
    #[inline]
    fn clone(&self) -> Self {
        // Not sure why, but #[derive(Clone)] causes the type to become private, so we declare Clone the old-fashioned way
        ContourEdge {
            corner_offset: self.corner_offset,
            shape_id: self.shape_id,
            contour: self.contour.clone(),
        }
    }
}

impl<TContour> EdgeDescriptor for ContourEdge<TContour>
where
    TContour: 'static + Clone + Send + Sync + SampledContour,
{
    #[inline]
    fn clone_as_object(&self) -> Arc<dyn EdgeDescriptor> {
        Arc::new(self.clone())
    }

    #[inline]
    fn prepare_to_render(&mut self) {}

    #[inline]
    fn shape(&self) -> ShapeId {
        self.shape_id
    }

    fn transform(&self, transform: &canvas::Transform2D) -> Arc<dyn EdgeDescriptor> {
        todo!()
    }

    fn bounding_box(&self) -> ((f64, f64), (f64, f64)) {
        let ContourSize(w, h) = self.contour.contour_size();
        let (w, h) = (w as f64, h as f64);
        let (x, y) = self.corner_offset;

        (self.corner_offset, (x + w, y + h))
    }

    fn intercepts(
        &self,
        y_positions: &[f64],
        output: &mut [SmallVec<[EdgeDescriptorIntercept; 2]>],
    ) {
        for idx in 0..y_positions.len() {
            let y_pos = y_positions[idx];

            let ContourSize(_, h) = self.contour.contour_size();
            let h = h as f64;
            let (x, y) = self.corner_offset;

            // TODO: need to be able to get more information about the intercept here, this won't work to generate shards for concave shapes
            let y_pos = y_pos - y;
            if !(y_pos < 0.0 || y_pos >= h) {
                output[idx].extend(self.contour.intercepts_on_line(y_pos).into_iter().flat_map(
                    |intercept| {
                        [
                            EdgeDescriptorIntercept {
                                direction: EdgeInterceptDirection::Toggle,
                                x_pos: intercept.start + x,
                                position: EdgePosition(0, 0, 0.0),
                            },
                            EdgeDescriptorIntercept {
                                direction: EdgeInterceptDirection::Toggle,
                                x_pos: intercept.end + x,
                                position: EdgePosition(0, 0, 0.0),
                            },
                        ]
                    },
                ));
            }
        }
    }
}
