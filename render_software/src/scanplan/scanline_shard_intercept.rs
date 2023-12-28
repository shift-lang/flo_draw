/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::scanline_transform::*;

use crate::edgeplan::*;

use std::{ops::Range, process::ExitCode};

///
/// Describes the location of a shard intercept
///
#[derive(Clone, Copy, Debug)]
pub struct ShardInterceptLocation {
    pub shape: ShapeId,
    pub direction: EdgeInterceptDirection,
    pub lower_x: f64,
    pub upper_x: f64,
    pub lower_x_floor: f64,
    pub upper_x_ceil: f64,
}

///
/// Ways that a scanline fragment can be bn
///
#[derive(Clone, Debug)]
pub enum InterceptBlend {
    /// Only the shape's blending should be used
    Solid,

    /// This should be alpha-blended using source-over with a linear fade
    Fade {
        x_range: Range<f64>,
        alpha_range: Range<f64>,
    },

    /// Nest the second blend inside the first blend
    NestedFade {
        x_range: Range<f64>,
        alpha_range: Range<f64>,
        nested: Box<InterceptBlend>,
    },
}

///
/// Represents an active intercept on a scanline
///
#[derive(Debug)]
pub struct ScanlineShardIntercept<'a> {
    /// The number of times an edge for this shape has been crossed
    count: isize,

    /// The x-position where the shape was first intercepted
    start_x: f64,

    /// How this intercept should be blended with those behind it
    blend: InterceptBlend,

    /// The shape that is being drawn by this scanline
    shape_id: ShapeId,

    /// The shape descriptor
    descriptor: &'a ShapeDescriptor,
}

///
/// Used to keep track of which shapes are being rendered when tracing a scanline
///
#[derive(Debug)]
pub struct ScanlineShardInterceptState<'a> {
    /// The currently active shapes, with the most recent one
    active_shapes: Vec<ScanlineShardIntercept<'a>>,

    /// The current z-floor
    z_floor: i64,
}

impl ShardInterceptLocation {
    #[inline]
    pub fn from(
        intercept: &EdgePlanShardIntercept,
        transform: &ScanlineTransform,
    ) -> ShardInterceptLocation {
        let lower_x = transform.source_x_to_pixels(intercept.lower_x);
        let upper_x = transform.source_x_to_pixels(intercept.upper_x);

        ShardInterceptLocation {
            shape: intercept.shape,
            direction: intercept.direction,
            lower_x_floor: lower_x.floor(),
            upper_x_ceil: upper_x.ceil(),
            lower_x: lower_x,
            upper_x: upper_x,
        }
    }
}

impl<'a> ScanlineShardIntercept<'a> {
    ///
    /// Returns the point at which this intercept started
    ///
    #[inline]
    pub fn start_x(&self) -> f64 {
        self.start_x
    }

    ///
    /// Returns the shape ID used for this intercept
    ///
    #[inline]
    pub fn shape_id(&self) -> ShapeId {
        self.shape_id
    }

    ///
    /// Returns the z-index of this shape (higher is in front of lower)
    ///
    #[inline]
    pub fn z_index(&self) -> i64 {
        self.descriptor.z_index
    }

    ///
    /// Returns true if this intercept is opaque
    ///
    #[inline]
    pub fn is_opaque(&self) -> bool {
        match self.blend {
            InterceptBlend::Solid => self.descriptor.is_opaque,
            InterceptBlend::Fade { .. } => false,
            InterceptBlend::NestedFade { .. } => false,
        }
    }

    ///
    /// Returns the shape descriptor for this intercept
    ///
    #[inline]
    pub fn shape_descriptor(&self) -> &ShapeDescriptor {
        self.descriptor
    }

    ///
    /// The blending mode to use for this intercept
    ///
    #[inline]
    pub fn blend(&self) -> &InterceptBlend {
        &self.blend
    }
}

///
/// Removes any intercepts that end at or before a position in a blend
///
/// The result is `InterceptBlend::Solid` if all of the intercepts are removed
///
fn clear_finished_intercepts(blend: &InterceptBlend, xpos: f64) -> InterceptBlend {
    match blend {
        // Solid intercepts have nothing to clear
        InterceptBlend::Solid => InterceptBlend::Solid,

        // Fades clear if the x position exceeds the x position
        InterceptBlend::Fade {
            x_range,
            alpha_range,
        } => {
            if x_range.end <= xpos {
                InterceptBlend::Solid
            } else {
                InterceptBlend::Fade {
                    x_range: x_range.clone(),
                    alpha_range: alpha_range.clone(),
                }
            }
        }

        // Nested fades work like normal fades, except they process their contents recursively
        InterceptBlend::NestedFade {
            x_range,
            alpha_range,
            nested,
        } => {
            // Recursively remove any finished intercepts from the nested version
            let nested_cleared = clear_finished_intercepts(&*nested, xpos);

            if x_range.end <= xpos {
                // If this blend has finished, then just use the nested version
                nested_cleared
            } else {
                if let InterceptBlend::Solid = &nested_cleared {
                    // Changes to a normal fade if the nested intercept is entirely cleared
                    InterceptBlend::Fade {
                        x_range: x_range.clone(),
                        alpha_range: alpha_range.clone(),
                    }
                } else {
                    // Stays nested
                    InterceptBlend::NestedFade {
                        x_range: x_range.clone(),
                        alpha_range: alpha_range.clone(),
                        nested: Box::new(nested_cleared),
                    }
                }
            }
        }
    }
}

impl<'a> ScanlineShardInterceptState<'a> {
    ///
    /// Creates a new intercept state
    ///
    #[inline]
    pub fn new() -> ScanlineShardInterceptState<'a> {
        ScanlineShardInterceptState {
            active_shapes: vec![],
            z_floor: i64::MIN,
        }
    }

    ///
    /// The z-index of the lowest opaque item in this state (or `i64::MIN` if there's no floor)
    ///
    #[inline]
    pub fn z_floor(&self) -> i64 {
        self.z_floor
    }

    ///
    /// Finds the index of the intercept that's >= the z-index
    ///
    /// Returns Ok(index) if we find an exact match, or Err(index) if we don't
    ///
    #[inline]
    pub fn find(&self, z_index: i64, shape_id: ShapeId) -> Result<usize, usize> {
        // min is inclusive, max is exclusive
        let mut min = 0;
        let mut max = self.active_shapes.len();

        // Binary search until we find a nearby shape
        while min < max {
            let mid = (min + max) >> 1;
            let intercept = &self.active_shapes[mid];

            if intercept.z_index() < z_index {
                min = mid + 1;
            } else if intercept.z_index() > z_index {
                max = mid;
            } else if intercept.shape_id < shape_id {
                min = mid + 1;
            } else if intercept.shape_id > shape_id {
                max = mid;
            } else {
                return Ok(mid);
            }
        }

        /* (may be faster)
        // Linear search for the remaining items
        while min < max {
            let intercept = &self.active_shapes[min];

            if intercept.z_index > z_index {
                return Err(min);
            } else if intercept.shape_id > shape_id {
                return Err(min);
            } else if intercept.z_index == z_index && intercept.shape_id == shape_id {
                return Ok(min);
            }

            min += 1;
        }
        */

        // 'min' should be the first >= value once the binary search converges
        return Err(min);
    }

    ///
    /// The number of intercepts that are currently on the stack
    ///
    #[inline]
    pub fn len(&self) -> usize {
        self.active_shapes.len()
    }

    ///
    /// Retrieves the intercept at the specified position on the stack
    ///
    #[inline]
    pub fn get(&self, idx: usize) -> Option<&ScanlineShardIntercept> {
        self.active_shapes.get(idx)
    }

    ///
    /// Adds or removes from the active shapes after an intercept
    ///
    pub fn start_intercept(
        &mut self,
        intercept: &ShardInterceptLocation,
        transform: &ScanlineTransform,
        descriptor: Option<&'a ShapeDescriptor>,
    ) {
        if let Some(descriptor) = descriptor {
            let (z_index, is_opaque) = (descriptor.z_index, descriptor.is_opaque);

            match self.find(z_index, intercept.shape) {
                Ok(existing_idx) => {
                    // Update the existing shape depending on the direction of the intercept
                    let existing = &mut self.active_shapes[existing_idx];
                    let was_inside = existing.count != 0;
                    match intercept.direction {
                        EdgeInterceptDirection::Toggle => {
                            existing.count = if existing.count != 0 { 1 } else { 0 };
                        }

                        EdgeInterceptDirection::DirectionOut => {
                            existing.count += 1;
                        }

                        EdgeInterceptDirection::DirectionIn => {
                            existing.count -= 1;
                        }
                    };
                    let is_inside = existing.count != 0;

                    if !was_inside && is_inside {
                        // Need to merge with the existing blend
                        self.active_shapes[existing_idx].blend =
                            match &self.active_shapes[existing_idx].blend {
                                InterceptBlend::Solid => InterceptBlend::Fade {
                                    x_range: intercept.lower_x..intercept.upper_x,
                                    alpha_range: 0.0..1.0,
                                },

                                InterceptBlend::Fade { .. } | InterceptBlend::NestedFade { .. } => {
                                    let nested =
                                        Box::new(self.active_shapes[existing_idx].blend.clone());

                                    InterceptBlend::NestedFade {
                                        x_range: intercept.lower_x..intercept.upper_x,
                                        alpha_range: 0.0..1.0,
                                        nested: nested,
                                    }
                                }
                            };
                    } else if !is_inside {
                        // Change the shape to fade out
                        self.active_shapes[existing_idx].blend =
                            match &self.active_shapes[existing_idx].blend {
                                InterceptBlend::Solid => InterceptBlend::Fade {
                                    x_range: intercept.lower_x..intercept.upper_x,
                                    alpha_range: 1.0..0.0,
                                },

                                InterceptBlend::Fade { .. } | InterceptBlend::NestedFade { .. } => {
                                    let nested =
                                        Box::new(self.active_shapes[existing_idx].blend.clone());

                                    InterceptBlend::NestedFade {
                                        x_range: intercept.lower_x..intercept.upper_x,
                                        alpha_range: 1.0..0.0,
                                        nested: nested,
                                    }
                                }
                            };

                        // If the shape matches the current z-floor, update it
                        if is_opaque && z_index == self.z_floor {
                            self.z_floor = i64::MIN;

                            // TODO: if multiple shapes are on the same z-index, existing_idx might represent a shape below the 'true' z-floor (so this will set the floor too low)
                            for idx in (0..existing_idx).rev() {
                                if self.active_shapes[idx].is_opaque() {
                                    self.z_floor = self.active_shapes[idx].z_index();
                                    break;
                                }
                            }
                        }
                    }
                }

                Err(following_idx) => {
                    // There's no existing matching shape: just insert a new intercept
                    let count = match intercept.direction {
                        EdgeInterceptDirection::Toggle => 1,
                        EdgeInterceptDirection::DirectionOut => 1,
                        EdgeInterceptDirection::DirectionIn => -1,
                    };

                    self.active_shapes.insert(
                        following_idx,
                        ScanlineShardIntercept {
                            count: count,
                            start_x: intercept.lower_x,
                            blend: InterceptBlend::Fade {
                                x_range: intercept.lower_x..intercept.upper_x,
                                alpha_range: 0.0..1.0,
                            },
                            shape_id: intercept.shape,
                            descriptor: descriptor,
                        },
                    )
                }
            }
        }
    }

    ///
    /// A partial intercept has finished
    ///
    pub fn finish_intercept(
        &mut self,
        intercept: &ShardInterceptLocation,
        descriptor: Option<&'a ShapeDescriptor>,
    ) {
        if let Some(descriptor) = descriptor {
            if let Ok(existing_idx) = self.find(descriptor.z_index, intercept.shape) {
                let active_shape = &mut self.active_shapes[existing_idx];
                let new_blend = clear_finished_intercepts(&active_shape.blend, intercept.upper_x);

                if active_shape.count != 0 {
                    // If we're inside the shape, we always update to the new blend (which will be solid when all the blends are gone)
                    active_shape.blend = new_blend;
                } else if let InterceptBlend::Solid = &new_blend {
                    // If we're outside the shape and all blends are gone, then remove the shape from the state
                    self.active_shapes.remove(existing_idx);
                } else {
                    // If we're outside the shape but still blending, then leave the existing blends going
                    active_shape.blend = new_blend;
                }
            }
        }
    }

    ///
    /// Adjusts all the existing intercepts so that they have a specified start position (for clipping onto the left-hand side of the visible region)
    ///
    pub fn clip_start_x(&mut self, clip_x: f64) {
        for intercept in self.active_shapes.iter_mut() {
            intercept.start_x = clip_x;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use smallvec::*;

    #[test]
    fn start_intercept() {
        let mut intercepts = ScanlineShardInterceptState::new();
        let transform = &ScanlineTransform::for_region(&(-1.0..1.0), 1080);
        let descriptor = ShapeDescriptor {
            programs: smallvec![],
            is_opaque: true,
            z_index: 1,
        };

        // Start entering
        intercepts.start_intercept(
            &ShardInterceptLocation {
                shape: ShapeId(1),
                direction: EdgeInterceptDirection::DirectionIn,
                lower_x: 100.0,
                upper_x: 120.0,
                lower_x_floor: 100.0,
                upper_x_ceil: 120.0,
            },
            &transform,
            Some(&descriptor),
        );

        // Should be one intercept, that's a fading intercept
        assert!(intercepts.len() == 1, "{:?}", intercepts);

        let fade_in = intercepts.get(0).unwrap();
        assert!(
            if let InterceptBlend::Fade { .. } = &fade_in.blend {
                true
            } else {
                false
            },
            "Not fading: {:?}",
            intercepts
        );
    }

    #[test]
    fn inside_shape() {
        let mut intercepts = ScanlineShardInterceptState::new();
        let transform = &ScanlineTransform::for_region(&(-1.0..1.0), 1080);
        let descriptor = ShapeDescriptor {
            programs: smallvec![],
            is_opaque: true,
            z_index: 1,
        };

        // Enter the shape
        intercepts.start_intercept(
            &ShardInterceptLocation {
                shape: ShapeId(1),
                direction: EdgeInterceptDirection::DirectionIn,
                lower_x: 100.0,
                upper_x: 120.0,
                lower_x_floor: 100.0,
                upper_x_ceil: 120.0,
            },
            &transform,
            Some(&descriptor),
        );
        intercepts.finish_intercept(
            &ShardInterceptLocation {
                shape: ShapeId(1),
                direction: EdgeInterceptDirection::DirectionIn,
                lower_x: 100.0,
                upper_x: 120.0,
                lower_x_floor: 100.0,
                upper_x_ceil: 120.0,
            },
            Some(&descriptor),
        );

        // Should be one intercept which is solid
        assert!(intercepts.len() == 1, "{:?}", intercepts);

        let fade_in = intercepts.get(0).unwrap();
        assert!(
            if let InterceptBlend::Solid = &fade_in.blend {
                true
            } else {
                false
            },
            "Not fading: {:?}",
            intercepts
        );
    }

    #[test]
    fn leave_intercept() {
        let mut intercepts = ScanlineShardInterceptState::new();
        let transform = &ScanlineTransform::for_region(&(-1.0..1.0), 1080);
        let descriptor = ShapeDescriptor {
            programs: smallvec![],
            is_opaque: true,
            z_index: 1,
        };

        // Start entering
        intercepts.start_intercept(
            &ShardInterceptLocation {
                shape: ShapeId(1),
                direction: EdgeInterceptDirection::DirectionIn,
                lower_x: 100.0,
                upper_x: 101.0,
                lower_x_floor: 100.0,
                upper_x_ceil: 101.0,
            },
            &transform,
            Some(&descriptor),
        );
        intercepts.finish_intercept(
            &ShardInterceptLocation {
                shape: ShapeId(1),
                direction: EdgeInterceptDirection::DirectionIn,
                lower_x: 100.0,
                upper_x: 101.0,
                lower_x_floor: 100.0,
                upper_x_ceil: 101.0,
            },
            Some(&descriptor),
        );

        // Start leaving...
        intercepts.start_intercept(
            &ShardInterceptLocation {
                shape: ShapeId(1),
                direction: EdgeInterceptDirection::DirectionOut,
                lower_x: 110.0,
                upper_x: 150.0,
                lower_x_floor: 110.0,
                upper_x_ceil: 150.0,
            },
            &transform,
            Some(&descriptor),
        );

        // Should be one intercept, that's a fading intercept
        assert!(intercepts.len() == 1, "{:?}", intercepts);

        let fade_in = intercepts.get(0).unwrap();
        assert!(
            if let InterceptBlend::Fade { .. } = &fade_in.blend {
                true
            } else {
                false
            },
            "Not fading: {:?}",
            intercepts
        );
    }

    #[test]
    fn start_overlapping_intercept() {
        let mut intercepts = ScanlineShardInterceptState::new();
        let transform = &ScanlineTransform::for_region(&(-1.0..1.0), 1080);
        let descriptor = ShapeDescriptor {
            programs: smallvec![],
            is_opaque: true,
            z_index: 1,
        };

        // Enter
        intercepts.start_intercept(
            &ShardInterceptLocation {
                shape: ShapeId(1),
                direction: EdgeInterceptDirection::DirectionIn,
                lower_x: 100.0,
                upper_x: 101.0,
                lower_x_floor: 100.0,
                upper_x_ceil: 101.0,
            },
            &transform,
            Some(&descriptor),
        );
        intercepts.finish_intercept(
            &ShardInterceptLocation {
                shape: ShapeId(1),
                direction: EdgeInterceptDirection::DirectionIn,
                lower_x: 100.0,
                upper_x: 101.0,
                lower_x_floor: 100.0,
                upper_x_ceil: 101.0,
            },
            Some(&descriptor),
        );

        // Start leaving...
        intercepts.start_intercept(
            &ShardInterceptLocation {
                shape: ShapeId(1),
                direction: EdgeInterceptDirection::DirectionOut,
                lower_x: 110.0,
                upper_x: 150.0,
                lower_x_floor: 110.0,
                upper_x_ceil: 150.0,
            },
            &transform,
            Some(&descriptor),
        );

        assert!(
            intercepts.len() == 1,
            "Should only be one intercept {:?}",
            intercepts
        );

        // Start re-entering
        intercepts.start_intercept(
            &ShardInterceptLocation {
                shape: ShapeId(1),
                direction: EdgeInterceptDirection::DirectionIn,
                lower_x: 125.0,
                upper_x: 170.0,
                lower_x_floor: 125.0,
                upper_x_ceil: 170.0,
            },
            &transform,
            Some(&descriptor),
        );

        // Should be one intercept, which should be a nested intercept
        assert!(
            intercepts.len() == 1,
            "Should be one intercept {:?}",
            intercepts
        );

        let nested = intercepts.get(0).unwrap();
        assert!(
            if let InterceptBlend::NestedFade { .. } = &nested.blend {
                true
            } else {
                false
            },
            "Not nested: {:?}",
            intercepts
        );
    }
}
