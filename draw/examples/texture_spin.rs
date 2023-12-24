use std::io;
use std::thread;
use std::time::Duration;

use flo_draw::*;
use flo_draw::canvas::*;

///
/// Simple example that displays a canvas window and renders a spinning image from a png file
///
pub fn main() {
    // 'with_2d_graphics' is used to support operating systems that can't run event loops anywhere other than the main thread
    with_2d_graphics(|| {
        // Load a png file
        let flo_bytes: &[u8] = include_bytes!["flo_drawing_on_window.png"];

        // Create a window
        let canvas = create_drawing_window("Flo trying not to get too dizzy");

        // Load the texture into it
        let mut flo_w = 0;
        let mut flo_h = 0;
        canvas.draw(|gc| {
            // Clear the canvas and set up the coordinates
            gc.clear_canvas(Color::Rgba(1.0, 1.0, 1.0, 1.0));
            gc.canvas_height(1000.0);
            gc.center_region(0.0, 0.0, 1000.0, 1000.0);

            // Set up the texture
            let (w, h) = gc.load_texture(TextureId(0), io::Cursor::new(flo_bytes)).unwrap();
            flo_w = w;
            flo_h = h;
        });

        let mut angle = 0.0;

        loop {
            // Render the png to the window
            canvas.draw(|gc| {
                // Redraw layer 0 rather than clearing the canvas (as clearing the canvas also clears out the textures)
                gc.layer(LayerId(0));
                gc.clear_layer();

                let ratio = (flo_w as f32) / (flo_h as f32);
                let height = 1000.0 / ratio;
                let y_pos = (1000.0 - height) / 2.0;

                let mid_x = 500.0;
                let mid_y = y_pos + (height / 2.0);

                // Draw a circle...
                gc.new_path();
                gc.circle(mid_x, mid_y, height / 2.0);

                // Fill with the texture we just loaded
                gc.fill_texture(TextureId(0), 0.0, y_pos + height as f32, 1000.0, y_pos);

                gc.fill_transform(Transform2D::translate(-mid_x, -mid_y));
                gc.fill_transform(Transform2D::rotate_degrees(angle));
                gc.fill_transform(Transform2D::scale(1.0 / 3.0, 1.0 / 3.0));
                gc.fill_transform(Transform2D::translate(mid_x, mid_y));
                gc.fill();

                // Draw another couple of circles to demonstrate that it's the texture that's spinning and not the whole canvas
                gc.fill_color(Color::Rgba(0.0, 0.0, 0.0, 1.0));

                gc.new_path();
                gc.circle(mid_x - height / 2.5, mid_y - height / 2.5, 32.0);
                gc.fill();

                gc.new_path();
                gc.circle(mid_x + height / 2.5, mid_y + height / 2.5, 32.0);
                gc.fill();
            });

            // Wait for the next frame
            thread::sleep(Duration::from_nanos(1_000_000_000 / 60));

            // Rotate the texture
            angle += 1.0;
        }
    });
}
