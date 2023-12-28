/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::sync::*;
use std::thread;
use std::time::{Duration, Instant};

use futures::executor;
use futures::prelude::*;
use futures::stream;
use num_complex::*;
use rayon::iter::*;

use flo_draw::binding::*;
use flo_draw::canvas::*;
use flo_draw::*;

///
/// Renders the mandelbrot set, demonstrates how to render from multiple threads and communicate with bindings
///
/// See the `flo_binding` library for some details about how bindings work
///
pub fn main() {
    with_2d_graphics(|| {
        let mut window_properties = WindowProperties::from(&"Mandelbrot set");
        window_properties.mouse_pointer = BindRef::from(bind(MousePointer::None));
        let (canvas, events) = create_drawing_window_with_events(window_properties);

        let lato = CanvasFontFace::from_slice(include_bytes!("Lato-Regular.ttf"));
        let lato_bold = CanvasFontFace::from_slice(include_bytes!("Lato-Bold.ttf"));

        // Initialise the canvas
        canvas.draw(|gc| {
            gc.clear_canvas(Color::Rgba(0.9, 0.9, 1.0, 1.0));
            gc.define_font_data(FontId(1), Arc::clone(&lato));
            gc.define_font_data(FontId(2), Arc::clone(&lato_bold));
        });

        // Create some bindings that represent our state
        let width = bind(1024u32);
        let height = bind(768u32);
        let crossfade = bind(0.0);
        let bounds = bind((Complex::new(-2.5, -1.0), Complex::new(1.0, 1.0)));

        // The update number is used to synchronise other updates and interrupt drawing the mandelbrot
        let update_num = bind(0u64);

        // Run some threads to display some different layers. We can write to layers independently on different threads
        show_title(&canvas, LayerId(100), crossfade.clone());
        show_stats(
            &canvas,
            LayerId(99),
            BindRef::from(&bounds),
            BindRef::from(&crossfade),
        );
        show_mandelbrot(
            &canvas,
            LayerId(0),
            TextureId(100),
            BindRef::from(&width),
            BindRef::from(&height),
            BindRef::from(&bounds),
            BindRef::from(&crossfade),
            BindRef::from(&update_num),
        );

        // Loop while there are events
        executor::block_on(async move {
            let mut events = events;
            while let Some(evt) = events.next().await {
                match evt {
                    DrawEvent::Resized(size) => {
                        if width.get() != size.width as _ || height.get() != size.height as _ {
                            width.set(size.width as _);
                            height.set(size.height as _);
                            update_num.set(update_num.get() + 1);
                        }
                    }

                    DrawEvent::CursorMoved { state } => {
                        // Draw a rectangle around where the image will zoom in
                        canvas.draw(|gc| {
                            gc.layer(LayerId(1));
                            gc.clear_layer();

                            gc.canvas_height(height.get() as _);
                            gc.center_region(0.0, 0.0, width.get() as _, height.get() as _);

                            let (x, y) = state.location_in_window;
                            let (x, y) = (x as f32, y as f32);
                            let (w, h) = (width.get() as f32, height.get() as f32);
                            let y = h - y;

                            gc.new_path();
                            gc.rect(
                                x - (w / 4.0) + 2.0,
                                y - (h / 4.0) - 2.0,
                                x + (w / 4.0) + 2.0,
                                y + h / 4.0 - 2.0,
                            );
                            gc.stroke_color(Color::Rgba(0.0, 0.0, 0.0, 0.6));
                            gc.line_width(4.0);
                            gc.stroke();

                            gc.new_path();
                            gc.rect(x - (w / 4.0), y - (h / 4.0), x + (w / 4.0), y + h / 4.0);
                            gc.stroke_color(Color::Rgba(0.0, 0.6, 0.0, 0.9));
                            gc.line_width(4.0);
                            gc.stroke();
                        });
                    }

                    DrawEvent::Pointer(PointerAction::Leave, _, _) => {
                        // Remove the highlight when the cursor leaves the window
                        canvas.draw(|gc| {
                            gc.layer(LayerId(1));
                            gc.clear_layer();
                        });
                    }

                    DrawEvent::Pointer(PointerAction::ButtonDown, _, state) => {
                        // Zoom in at the point the user clicked
                        let (x, y) = state.location_in_window;
                        let (x, y) = (x as f64, y as f64);
                        let (w, h) = (width.get() as f64, height.get() as f64);
                        let y = h - y;

                        // x and y as proportions within the min/max bounds
                        let (x, y) = (x / w, y / h);

                        // x and y as coordinates within the space of the mandelbrot
                        let (min, max) = bounds.get();
                        let x = (max.re - min.re) * x + min.re;
                        let y = (max.im - min.im) * y + min.im;
                        let off_x = (max.re - min.re) / 4.0;
                        let off_y = (max.im - min.im) / 4.0;

                        // Update the bounds
                        bounds.set((
                            Complex::new(x - off_x, y - off_y),
                            Complex::new(x + off_x, y + off_y),
                        ));
                        update_num.set(update_num.get() + 1);
                    }

                    _ => {}
                }
            }
        })
    })
}

///
/// Runs a thread that shows the title
///
fn show_title(canvas: &DrawingTarget, layer: LayerId, crossfade: Binding<f32>) {
    let canvas = canvas.clone();

    thread::Builder::new()
        .name("Title thread".into())
        .spawn(move || {
            // Draw the title with a cross-fade
            for fade in 0..=180 {
                // Update the crossfade factor for the other threads. Fade goes from 0.0 to 2.0
                let fade = (fade as f32) / 90.0;
                crossfade.set(fade);

                // Draw the title, with a cross fade to show the mandelbrot set
                canvas.draw(|gc| {
                    gc.layer(layer);
                    gc.clear_layer();

                    gc.canvas_height(1000.0);
                    gc.center_region(0.0, 0.0, 1000.0, 1000.0);

                    let title_fade = (2.0 - fade) - 0.5;
                    let title_fade = f32::min(f32::max(title_fade, 0.0), 1.0);

                    // Title card
                    gc.fill_color(Color::Rgba(0.0, 0.0, 0.6, title_fade));
                    gc.set_font_size(FontId(2), 36.0);
                    gc.begin_line_layout(500.0, 482.0 + (title_fade * 4.0), TextAlignment::Center);
                    gc.layout_text(FontId(2), "Mandelbrot set".into());
                    gc.draw_text_layout();

                    gc.set_font_size(FontId(1), 16.0);
                    gc.begin_line_layout(500.0, 430.0 - (title_fade * 4.0), TextAlignment::Center);
                    gc.layout_text(FontId(1), "A flo_draw demonstration".into());
                    gc.draw_text_layout();

                    gc.begin_line_layout(500.0, 400.0, TextAlignment::Center);
                    gc.layout_text(FontId(1), "Written by Andrew Hunter".into());
                    gc.draw_text_layout();
                });

                // Fade at 60fps
                thread::sleep(Duration::from_nanos(1_000_000_000 / 60));
            }

            // Blank the layer once done
            canvas.draw(|gc| {
                gc.layer(layer);
                gc.clear_layer();
            });
        })
        .unwrap();
}

///
/// Runs a thread that displays some statistics for the current rendering
///
fn show_stats(
    canvas: &DrawingTarget,
    layer: LayerId,
    bounds: BindRef<(Complex<f64>, Complex<f64>)>,
    crossfade: BindRef<f32>,
) {
    let canvas = canvas.clone();

    thread::Builder::new()
        .name("Stats thread".into())
        .spawn(move || {
            // Compute the value to display on the LHS of the display
            let left_stats = computed(move || {
                let bounds = bounds.get();
                let scale_factor_x = 3.5 / (bounds.1.re - bounds.0.re).abs();
                let scale_factor_y = 2.0 / (bounds.1.im - bounds.0.im).abs();
                let scale_factor = f64::max(scale_factor_x, scale_factor_y);
                let scale_factor = scale_factor.round();

                format!("Zoom: {}x", scale_factor)
            });

            // Run a loop to update and diplay the statistics for the mandelbrot set
            executor::block_on(async move {
                // Follow the stats as they change
                let alpha = computed(move || f32::max(f32::min(crossfade.get(), 1.0), 0.0));
                let mut stats = follow(computed(move || (left_stats.get(), alpha.get())));

                while let Some((left_stats, alpha)) = stats.next().await {
                    // Redraw the stats on the layer
                    canvas.draw(|gc| {
                        gc.layer(layer);
                        gc.clear_layer();

                        gc.canvas_height(1000.0);
                        gc.center_region(0.0, 0.0, 1000.0, 1000.0);

                        gc.fill_color(Color::Rgba(0.0, 0.0, 0.0, alpha * 0.7));
                        gc.set_font_size(FontId(1), 24.0);

                        gc.begin_line_layout(21.0, 899.0, TextAlignment::Left);
                        gc.layout_text(FontId(1), format!("{}", left_stats));
                        gc.draw_text_layout();

                        gc.fill_color(Color::Rgba(0.0, 0.6, 0.9, alpha * 0.9));
                        gc.set_font_size(FontId(1), 24.0);

                        gc.begin_line_layout(20.0, 900.0, TextAlignment::Left);
                        gc.layout_text(FontId(1), format!("{}", left_stats));
                        gc.draw_text_layout();
                    });
                }
            });
        })
        .unwrap();
}

///
/// Runs a thread that renders the mandelbrot set whenever the bindings change
///
fn show_mandelbrot(
    canvas: &DrawingTarget,
    layer: LayerId,
    texture: TextureId,
    width: BindRef<u32>,
    height: BindRef<u32>,
    bounds: BindRef<(Complex<f64>, Complex<f64>)>,
    crossfade: BindRef<f32>,
    update_num: BindRef<u64>,
) {
    let canvas = canvas.clone();

    thread::Builder::new()
        .name("Mandelbrot thread".into())
        .spawn(move || {
            enum Event {
                RenderBounds((u32, u32, (Complex<f64>, Complex<f64>))),
                CrossFade(f32),
            }

            let alpha = computed(move || f32::min(f32::max(crossfade.get() - 1.0, 0.0), 1.0));
            let alpha = BindRef::from(alpha);
            let mut texture_w = width.get();
            let mut texture_h = height.get();

            // The render bounds are used to determine when we start to re-render the mandelbrot set
            let render_bounds = computed(move || (width.get(), height.get(), bounds.get()));

            // Events either start rendering a new frame or changing the crossfade
            let render_bounds = follow(render_bounds)
                .map(|bounds| Event::RenderBounds(bounds))
                .boxed();
            let crossfade = follow(alpha.clone())
                .map(|xfade| Event::CrossFade(xfade))
                .boxed();

            let events = stream::select_all(vec![render_bounds, crossfade]);

            // Wait for events and render the mandelbrot set as they arrive
            executor::block_on(async move {
                let mut events = events;

                while let Some(evt) = events.next().await {
                    match evt {
                        Event::RenderBounds((new_width, new_height, new_bounds)) => {
                            texture_w = new_width;
                            texture_h = new_height;

                            // Create the texture for this width and height
                            canvas.draw(|gc| {
                                gc.layer(layer);
                                gc.create_texture(
                                    texture,
                                    texture_w,
                                    texture_h,
                                    TextureFormat::Rgba,
                                );
                                gc.set_texture_fill_alpha(texture, alpha.get());
                            });

                            // Fill it in with the current bounds
                            draw_mandelbrot(
                                &canvas,
                                layer,
                                texture,
                                new_bounds,
                                texture_w,
                                texture_h,
                                &alpha,
                                &update_num,
                            );
                        }

                        Event::CrossFade(new_alpha) => {
                            // Redraw the texture with the new alpha
                            canvas.draw(|gc| {
                                gc.layer(layer);
                                gc.clear_layer();
                                gc.set_texture_fill_alpha(texture, new_alpha);

                                gc.canvas_height(texture_h as _);
                                gc.center_region(0.0, 0.0, texture_w as _, texture_h as _);

                                gc.new_path();
                                gc.rect(0.0, 0.0, texture_w as _, texture_h as _);
                                gc.fill_texture(texture, 0.0, 0.0, texture_w as _, texture_h as _);
                                gc.fill();
                            });
                        }
                    }
                }
            });
        })
        .unwrap();
}

///
/// Draws the mandelbrot set within a specified set of bounds
///
fn draw_mandelbrot(
    canvas: &DrawingTarget,
    layer: LayerId,
    texture: TextureId,
    (min, max): (Complex<f64>, Complex<f64>),
    width: u32,
    height: u32,
    alpha: &BindRef<f32>,
    update_num: &BindRef<u64>,
) {
    // Create a vector for the pixels in the mandelbrot set
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    let mut pos = 0;
    let update = update_num.get();

    let mut start_time = Instant::now();

    // Work out how many iterations to perform
    let scale_factor_x = 3.5 / (max.re - min.re).abs();
    let scale_factor_y = 2.0 / (max.im - min.im).abs();
    let scale_factor = f64::max(scale_factor_x, scale_factor_y);
    let scale_factor = scale_factor.round();

    let num_cycles = if scale_factor < 64.0 {
        256
    } else if scale_factor < 2048.0 {
        1024
    } else if scale_factor < 8192.0 {
        2048
    } else {
        4096
    };

    // Render each pixel in turn
    for y in 0..height {
        let y = y as f64;
        let y = y / (height as f64);
        let y = (max.im - min.im) * y + min.im;

        let line = (0..width)
            .into_par_iter()
            .map(|x| {
                let x = x as f64;
                let x = x / (width as f64);
                let x = (max.re - min.re) * x + min.re;

                let c = Complex::new(x, y);
                let cycles = count_cycles(c, num_cycles);
                let (r, g, b, a) = color_for_cycles(cycles);

                (r, g, b, a)
            })
            .collect::<Vec<_>>();

        for (r, g, b, a) in line {
            pixels[pos + 0] = r;
            pixels[pos + 1] = g;
            pixels[pos + 2] = b;
            pixels[pos + 3] = a;

            pos += 4;
        }

        // Stop if there's an update to the state we're rendering
        if update_num.get() != update {
            return;
        }

        // Draw the story so far every 50ms
        if Instant::now().duration_since(start_time) > Duration::from_millis(50) {
            let intermediate_pixels = Arc::new(pixels.clone());
            canvas.draw(move |gc| {
                gc.layer(layer);
                gc.clear_layer();
                gc.create_texture(texture, width, height, TextureFormat::Rgba);
                gc.set_texture_bytes(texture, 0, 0, width, height, intermediate_pixels);
                gc.set_texture_fill_alpha(texture, alpha.get());

                gc.canvas_height(height as _);
                gc.center_region(0.0, 0.0, width as _, height as _);

                gc.new_path();
                gc.rect(0.0, 0.0, width as _, height as _);
                gc.fill_texture(texture, 0.0, 0.0, width as _, height as _);
                gc.fill();
            });

            start_time = Instant::now();
        }
    }

    // Draw to the texture
    canvas.draw(move |gc| {
        gc.create_texture(texture, width, height, TextureFormat::Rgba);
        gc.set_texture_bytes(texture, 0, 0, width, height, Arc::new(pixels));

        gc.layer(layer);
        gc.clear_layer();
        gc.set_texture_fill_alpha(texture, alpha.get());

        gc.canvas_height(height as _);
        gc.center_region(0.0, 0.0, width as _, height as _);

        gc.new_path();
        gc.rect(0.0, 0.0, width as _, height as _);
        gc.fill_texture(texture, 0.0, 0.0, width as _, height as _);
        gc.fill();
    });
}

///
/// Counts the number of cycles (up to a maximum count) at a particular pixel
///
#[inline]
fn count_cycles(c: Complex<f64>, max_count: usize) -> usize {
    let mut z = Complex::new(0.0, 0.0);
    let mut count = 0;

    while count < max_count && (z.re * z.re + z.im * z.im) < 2.0 * 2.0 {
        z = z * z + c;
        count = count + 1;
    }

    if count < max_count {
        count
    } else {
        0
    }
}

///
/// Returns the colour to use for a particular number of cycles
///
#[inline]
fn color_for_cycles(num_cycles: usize) -> (u8, u8, u8, u8) {
    let col_val = num_cycles % 64;
    let hue = (num_cycles / 64) % 8;

    let col_val = if col_val > 32 { 64 - col_val } else { col_val };
    let col_val = col_val * 8;

    let hue = [
        (255, 255, 255),
        (196, 0, 0),
        (128, 96, 0),
        (48, 128, 0),
        (0, 196, 255),
        (0, 0, 255),
        (128, 0, 196),
        (0, 196, 120),
    ][hue];

    let (hue_r, hue_g, hue_b) = hue;
    let (r, g, b) = (
        (col_val * hue_r) >> 8,
        (col_val * hue_g) >> 8,
        (col_val * hue_b) >> 8,
    );

    (r as u8, g as u8, b as u8, 255)
}
