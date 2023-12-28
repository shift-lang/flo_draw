/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use flo_draw::canvas::*;
use flo_draw::*;

///
/// Demonstrates using a clipping path
///
pub fn main() {
    with_2d_graphics(|| {
        // Create a window
        let canvas = create_drawing_window("Clipping demonstration");

        // Clip a large path using a circular clipping path
        canvas.draw(|gc| {
            // Set up the canvas
            gc.clear_canvas(Color::Rgba(0.95, 1.0, 0.9, 1.0));
            gc.canvas_height(1000.0);
            gc.center_region(0.0, 0.0, 1000.0, 1000.0);

            gc.new_path();
            gc.rect(800.0, 800.0, 900.0, 900.0);
            gc.fill_color(Color::Rgba(0.6, 0.0, 0.0, 1.0));
            gc.fill();

            gc.new_path();
            gc.circle(500.0, 500.0, 150.0);
            gc.circle(500.0, 600.0, 150.0);
            gc.circle(600.0, 500.0, 200.0);
            // gc.circle(150.0, 850.0, 100.0);
            gc.clip();

            gc.winding_rule(WindingRule::NonZero);
            gc.new_path();
            gc.rect(0.0, 0.0, 1000.0, 1000.0);
            gc.fill_color(Color::Rgba(0.0, 0.0, 0.6, 1.0));
            gc.fill();

            gc.new_path();
            gc.move_to(0.0, 1000.0);
            gc.line_to(1000.0, 0.0);
            gc.stroke_color(Color::Rgba(0.0, 0.6, 0.0, 1.0));
            gc.line_width(16.0);
            gc.stroke();

            gc.unclip();
            // gc.unclip();

            gc.new_path();
            gc.rect(100.0, 100.0, 200.0, 200.0);
            gc.fill_color(Color::Rgba(0.6, 0.0, 0.0, 1.0));
            gc.fill();
        });
    });
}
