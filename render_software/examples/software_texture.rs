/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use flo_render_software::canvas::*;
use flo_render_software::render::*;

use std::io;

///
/// Draws FlowBetween's mascot as vector graphics in a window
///
pub fn main() {
    // Load a png file
    let flo_bytes: &[u8] = include_bytes!["flo_drawing_on_window.png"];

    // Create drawing instructions for the png
    let mut canvas = vec![];

    // Clear the canvas and set up the coordinates
    canvas.clear_canvas(Color::Rgba(1.0, 1.0, 1.0, 1.0));
    canvas.canvas_height(1000.0);
    canvas.center_region(0.0, 0.0, 1000.0, 1000.0);

    // Set up the texture
    let (flo_w, flo_h) = canvas
        .load_texture(TextureId(0), io::Cursor::new(flo_bytes))
        .unwrap();

    let ratio = (flo_w as f32) / (flo_h as f32);
    let height = 1000.0 / ratio;
    let y_pos = (1000.0 - height) / 2.0;

    // Draw a rectangle...
    canvas.new_path();
    canvas.rect(0.0, y_pos, 1000.0, y_pos + height);

    // Fill with the texture we just loaded
    canvas.fill_texture(TextureId(0), 0.0, y_pos + height as f32, 1000.0, y_pos);
    canvas.fill();

    // Render to the terminal window
    render_drawing(
        &mut TerminalRenderTarget::new(1920, 1080),
        canvas.iter().cloned(),
    );
}
