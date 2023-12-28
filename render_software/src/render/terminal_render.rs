/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#[cfg(feature = "render_term")]
mod term_render {
    use super::super::image_render::*;
    use super::super::render_slice::*;
    use super::super::render_target_trait::*;
    use super::super::renderer::*;

    use crate::pixel::*;

    ///
    /// Render target that sends its results to the terminal
    ///
    /// (This version only supports the iterm escape sequence)
    ///
    pub struct TerminalRenderTarget {
        width: usize,
        height: usize,
    }

    impl TerminalRenderTarget {
        ///
        /// Creates a terminal rendering target
        ///
        pub fn new(width: usize, height: usize) -> Self {
            TerminalRenderTarget { width, height }
        }
    }

    impl<'a, TPixel> RenderTarget<TPixel> for TerminalRenderTarget
    where
        TPixel: 'static
            + Send
            + Copy
            + Default
            + AlphaBlend
            + ToGammaColorSpace<U8RgbaPremultipliedPixel>,
    {
        #[inline]
        fn width(&self) -> usize {
            self.width
        }

        #[inline]
        fn height(&self) -> usize {
            self.height
        }

        fn render<TRegionRenderer>(
            &mut self,
            region_renderer: TRegionRenderer,
            source_data: &TRegionRenderer::Source,
        ) where
            TRegionRenderer: Renderer<Region = RenderSlice, Dest = [TPixel]>,
        {
            use base64::engine::general_purpose;
            use base64::engine::Engine;

            // Create the png data
            let mut png_data: Vec<u8> = vec![];

            // Render as PNG data
            {
                let mut png_render =
                    PngRenderTarget::from_stream(&mut png_data, self.width, self.height, 2.2);
                png_render.render(region_renderer, source_data);
            }

            // TODO: check termial capabilities (we can fall back to an ASCII-art representation)

            // Convert to base64
            let base64 = general_purpose::STANDARD_NO_PAD.encode(&png_data);

            // Write out the iterm escape sequence
            print!("\x1b]1337;File=inline=1:{}\x07", base64);
        }
    }
}

#[cfg(feature = "render_term")]
pub use term_render::*;
