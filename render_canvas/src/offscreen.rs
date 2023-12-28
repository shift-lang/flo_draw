/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use futures::prelude::*;

use flo_canvas::*;
use flo_render::*;

use super::canvas_renderer::*;

///
/// Renders a canvas in an offscreen context, returning the resulting bitmap
///
pub fn render_canvas_offscreen<'a, DrawStream, RenderContext>(
    context: &'a mut RenderContext,
    width: usize,
    height: usize,
    scale: f32,
    actions: DrawStream,
) -> impl 'a + Future<Output = Vec<u8>>
where
    DrawStream: 'a + Stream<Item = Draw>,
    RenderContext: 'a + OffscreenRenderContext,
{
    async move {
        // Perform as many drawing actions simultaneously as we can
        let actions = Box::pin(actions);
        let mut actions = actions.ready_chunks(10000);

        // Create the offscreen render target
        let mut render_target = context.create_render_target(width, height);

        // Create the canvas renderer
        let mut renderer = CanvasRenderer::new();

        // Prepare to render
        renderer.set_viewport(
            0.0..(width as f32),
            0.0..(height as f32),
            width as f32,
            height as f32,
            scale,
        );

        // Send the drawing instructions from the action stream
        while let Some(drawing) = actions.next().await {
            // Render the next set of actions
            let rendering = renderer.draw(drawing.into_iter());
            let rendering = rendering.collect::<Vec<_>>().await;

            // Commit them to the offscreen canvas
            render_target.render(rendering);
        }

        // Result is the realized rendering
        render_target.realize()
    }
}
