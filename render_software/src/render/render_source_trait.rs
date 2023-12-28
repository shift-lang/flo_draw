/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::render_slice::*;
use super::renderer::*;

use crate::pixel::*;
use crate::scanplan::*;

///
/// A render source can create an edge region renderer to be used with a render target
///
pub trait RenderSource<TScanPlanner, TProgramRunner>
where
    TScanPlanner: ScanPlanner,
    TProgramRunner: PixelProgramRunner,
{
    /// The region renderer takes instances of this type and uses them to generate pixel values in a region
    type RegionRenderer: Renderer<
        Region = RenderSlice,
        Source = Self,
        Dest = [TProgramRunner::TPixel],
    >;

    ///
    /// Builds a region renderer that can read from this type and output pixels along rows
    ///
    fn create_region_renderer(
        planner: TScanPlanner,
        pixel_runner: TProgramRunner,
    ) -> Self::RegionRenderer;
}
