/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::render_slice::*;
use super::renderer::*;

///
/// Trait implemented by types that can act as a render target
///
/// The 'IntermediatePixel' type is used to perform the initial rendering and blending, before conversion to the final format
///
pub trait RenderTarget<IntermediatePixel: 'static> {
    ///
    /// Retrieves the width of the target in pixels
    ///
    fn width(&self) -> usize;

    ///
    /// Retrieves the height of the target in pixels
    ///
    fn height(&self) -> usize;

    ///
    /// Renders a frame to this render target
    ///
    /// The renderer that is passed in here is a region renderer, which takes a list of y-positions and generates the pixels for those rows in the results.
    ///
    fn render<TRegionRenderer>(
        &mut self,
        region_renderer: TRegionRenderer,
        source_data: &TRegionRenderer::Source,
    ) where
        TRegionRenderer: Renderer<Region = RenderSlice, Dest = [IntermediatePixel]>;
}
