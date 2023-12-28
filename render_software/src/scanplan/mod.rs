/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//!
//! # ScanPlan
//!
//! The 'scan-plan' is a low-level model of a rasterized scene. It represents each line of the final result as a 'plan'
//! of programs to apply to ranges of pixels. This plan can be executed to generate the final scene, or combined with
//! other plans to create more complex renders.
//!
//! There are a few advantages of making this plan: notably it avoids overdraw (where individual pixels are rendered
//! multiple times) and it makes it easy to efficiently mix colours using f32 precision. This can make rendering faster
//! for complex scenes as work can be avoided rendering pixels that will be obscured later on, and it makes it easy to
//! parallize both the rendering and the generation tasks. Less complex scenes may render more slowly due to the extra
//! work involved, however.
//!

pub(crate) mod buffer_stack;
mod pixel_scan_planner;
mod scan_planner;
mod scanline_intercept;
mod scanline_plan;
mod scanline_shard_intercept;
mod scanline_transform;
mod scanspan;
mod shard;
mod shard_scan_planner;

pub use pixel_scan_planner::*;
pub use scan_planner::*;
pub use scanline_intercept::*;
pub use scanline_plan::*;
pub use scanline_shard_intercept::*;
pub use scanline_transform::*;
pub use scanspan::*;
pub use shard::*;
pub use shard_scan_planner::*;
