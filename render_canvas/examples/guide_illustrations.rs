/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//!
//! Renders the illustrations used in the guide to `flo_draw`
//!

use std::fs::*;
use std::io::*;
use std::path::*;
use std::sync::*;

use futures::executor;
use futures::prelude::*;
use futures::stream;
use once_cell::sync::Lazy;
use png;

use flo_canvas::*;
use flo_render_canvas::{initialize_offscreen_rendering, render_canvas_offscreen};

/// Size of the badge icons in pixels
const BADGE_SIZE: usize = 100;

static LATO: Lazy<Arc<CanvasFontFace>> =
    Lazy::new(|| CanvasFontFace::from_slice(include_bytes!("Lato-Regular.ttf")));

///
/// Draws a section badge
///
pub fn section_badge<TDrawFn: FnOnce(&mut Vec<Draw>) -> ()>(
    filename: &str,
    background_color: Color,
    draw: TDrawFn,
) {
    let (h, s, l, a) = background_color.to_hsluv_components();

    let (h1, s1, l1, a1) = (h + 2.0, s + 5.0, l + 15.0, a);
    let (h2, s2, l2, a2) = (h - 2.0, s - 5.0, l - 15.0, a);

    // The preamble sets up the rendering area (clip mask and background colour)
    let mut preamble = vec![];

    preamble.clear_canvas(Color::Rgba(0.0, 0.0, 0.0, 0.0));
    preamble.canvas_height(100.0);
    preamble.layer(LayerId(0));

    preamble.new_path();
    preamble.circle(0.0, 0.0, 49.0);
    preamble.clip();

    preamble.create_gradient(GradientId(0), Color::Hsluv(h1, s1, l1, a1));
    preamble.gradient_stop(GradientId(0), 0.5, background_color);
    preamble.gradient_stop(GradientId(0), 1.0, Color::Hsluv(h2, s2, l2, a2));

    preamble.new_path();
    preamble.circle(0.0, 0.0, 50.0);
    preamble.fill_gradient(GradientId(0), -50.0, -30.0, 50.0, 30.0);
    preamble.fill();

    preamble.fill_color(Color::Rgba(0.0, 0.0, 0.0, 1.0));
    preamble.line_width_pixels(1.0);

    preamble.layer(LayerId(1));

    // The actual rendering instructions generated by the drawing function
    let mut rendering = vec![];
    draw(&mut rendering);

    // Render to a bitmap
    let image = executor::block_on(async move {
        let mut context = initialize_offscreen_rendering().unwrap();
        let canvas_stream = stream::iter(preamble).chain(stream::iter(rendering));
        let canvas_stream = drawing_with_laid_out_text(canvas_stream);
        let canvas_stream = drawing_with_text_as_paths(canvas_stream);

        render_canvas_offscreen(&mut context, BADGE_SIZE, BADGE_SIZE, 1.0, canvas_stream).await
    });

    // Save to a png file
    let path = Path::new(filename);
    let file = File::create(path).unwrap();
    let ref mut writer = BufWriter::new(file);

    let mut png_encoder = png::Encoder::new(writer, BADGE_SIZE as _, BADGE_SIZE as _);
    png_encoder.set_color(png::ColorType::Rgba);
    png_encoder.set_depth(png::BitDepth::Eight);
    let mut png_writer = png_encoder.write_header().unwrap();

    png_writer.write_image_data(&image).unwrap();
}

fn section_graphics_primitives() {
    section_badge(
        "draw/guide_images/s_graphics_primitives.png",
        Color::Rgba(0.3, 0.8, 0.5, 0.8),
        |gc| {
            // Triangle, square, circle
            gc.fill_color(Color::Rgba(0.2, 0.2, 0.2, 0.8));
            gc.stroke_color(Color::Rgba(0.2, 0.2, 0.2, 0.8));

            gc.line_width(3.0);

            // Triangle
            gc.new_path();
            gc.move_to(-32.0, 28.0);
            gc.line_to(-14.0, 0.0);
            gc.line_to(4.0, 28.0);
            gc.close_path();

            gc.stroke();

            // Circle
            gc.new_path();
            gc.circle(14.0, -16.0, 18.0);
            gc.fill();

            // Rectangle
            gc.stroke_color(Color::Rgba(0.4, 0.4, 0.4, 0.9));
            gc.new_path();
            gc.rect(-16.0, -16.0, 16.0, 16.0);
            gc.stroke();
        },
    );
}

fn section_transforms() {
    section_badge(
        "draw/guide_images/s_transforms.png",
        Color::Rgba(0.3, 0.8, 0.5, 0.8),
        |gc| {
            for t in 0..4 {
                let t = (t as f32) + 1.0;
                let scale = 0.6 + (t / 10.0);
                let angle_degrees = 15.0 * (t - 1.0);
                let alpha = 0.5 + (t / 8.0);
                let offset = (t / 5.0) * 80.0 - 40.0;

                gc.push_state();

                gc.new_path();
                gc.rect(-16.0, -16.0, 16.0, 16.0);

                let transform = Transform2D::rotate_degrees(angle_degrees);
                let transform = Transform2D::scale(scale, scale) * transform;
                let transform = Transform2D::translate(offset, 0.0) * transform;
                gc.transform(transform);

                gc.line_width(1.8);

                gc.fill_color(Color::Rgba(0.7, 0.6, 0.2, alpha));
                gc.stroke_color(Color::Rgba(0.4, 0.3, 0.1, 1.0));
                gc.fill();
                gc.stroke();

                gc.pop_state();
            }
        },
    );
}

fn section_layers() {
    section_badge(
        "draw/guide_images/s_layers.png",
        Color::Rgba(0.3, 0.8, 0.5, 0.8),
        |gc| {
            for layer in 1..=4 {
                let t = layer as f32;
                let offset = (t / 5.0) * 60.0 - 30.0;
                let alpha = 0.5 + ((5.0 - t) / 8.0);
                let scale = (t / 5.0) * 0.4 + 0.8;

                gc.layer(LayerId(layer));

                gc.push_state();
                gc.transform(Transform2D::translate(0.0, -offset));
                gc.transform(Transform2D::scale(scale, scale));

                gc.new_path();
                gc.move_to(0.0, -10.0);
                gc.line_to(-28.0, 0.0);
                gc.line_to(0.0, 10.0);
                gc.line_to(28.0, 0.0);
                gc.close_path();

                gc.line_width(2.0);
                gc.fill_color(Color::Rgba(0.2, 0.3, 0.6, alpha));
                gc.stroke_color(Color::Rgba(0.2, 0.4, 0.8, 1.0));
                gc.fill();
                gc.stroke();

                gc.pop_state();
            }
        },
    );
}

fn section_sprites() {
    section_badge(
        "draw/guide_images/s_sprites.png",
        Color::Rgba(0.1, 0.5, 1.0, 0.8),
        |gc| {
            gc.sprite(SpriteId(0));
            gc.clear_sprite();

            gc.fill_color(Color::Rgba(1.0, 1.0, 1.0, 0.9));
            gc.move_to(-8.0, 0.0);
            gc.line_to(-2.0, -2.0);
            gc.line_to(0.0, -8.0);
            gc.line_to(2.0, -2.0);
            gc.line_to(8.0, 0.0);
            gc.line_to(2.0, 2.0);
            gc.line_to(0.0, 8.0);
            gc.line_to(-2.0, 2.0);
            gc.close_path();

            gc.fill();

            gc.layer(LayerId(1));

            gc.sprite_transform(SpriteTransform::Identity);
            gc.draw_sprite(SpriteId(0));

            gc.sprite_transform(SpriteTransform::Identity);
            gc.sprite_transform(SpriteTransform::Translate(28.0, -23.0));
            gc.draw_sprite(SpriteId(0));

            gc.sprite_transform(SpriteTransform::Identity);
            gc.sprite_transform(SpriteTransform::Translate(8.0, 1.0));
            gc.draw_sprite(SpriteId(0));

            gc.sprite_transform(SpriteTransform::Identity);
            gc.sprite_transform(SpriteTransform::Translate(22.0, -3.0));
            gc.draw_sprite(SpriteId(0));

            gc.sprite_transform(SpriteTransform::Identity);
            gc.sprite_transform(SpriteTransform::Translate(24.0, 13.0));
            gc.draw_sprite(SpriteId(0));

            gc.sprite_transform(SpriteTransform::Identity);
            gc.sprite_transform(SpriteTransform::Translate(3.0, 22.0));
            gc.draw_sprite(SpriteId(0));

            gc.sprite_transform(SpriteTransform::Identity);
            gc.sprite_transform(SpriteTransform::Translate(-10.0, 27.0));
            gc.draw_sprite(SpriteId(0));

            gc.sprite_transform(SpriteTransform::Identity);
            gc.sprite_transform(SpriteTransform::Translate(-14.0, -26.0));
            gc.draw_sprite(SpriteId(0));

            gc.sprite_transform(SpriteTransform::Identity);
            gc.sprite_transform(SpriteTransform::Translate(-32.0, 5.0));
            gc.draw_sprite(SpriteId(0));
        },
    );
}

fn section_textures() {
    // Define a texture as a character array (1-bit bitmap)
    let (w, h) = (16, 16);
    let texture_defn = "\
        #.#.#.#.#.#.#.#.\
        .#.#.#.#.#.#.#.#\
        #.#.#.#.#.#.#.#.\
        .#.#.#.#.#.#.#.#\
        #.#.#.#.#.#.#.#.\
        .#.#.#.#.#.#.#.#\
        #.#.#.#.#.#.#.#.\
        .#.#.#.#.#.#.#.#\
        #.#.#.#.#.#.#.#.\
        .#.#.#.#.#.#.#.#\
        #.#.#.#.#.#.#.#.\
        .#.#.#.#.#.#.#.#\
        #.#.#.#.#.#.#.#.\
        .#.#.#.#.#.#.#.#\
        #.#.#.#.#.#.#.#.\
        .#.#.#.#.#.#.#.#\
        ";

    // Map to RGBA data
    let texture_data = texture_defn
        .chars()
        .map(|c| match c {
            '.' => Some([0u8, 0u8, 0u8, 0u8]),
            '#' => Some([255u8, 255u8, 200u8, 200u8]),
            _ => None,
        })
        .flatten()
        .flatten()
        .collect::<Vec<_>>();
    assert!(texture_data.len() == (w * h * 4));
    let texture_data = Arc::new(texture_data);

    section_badge(
        "draw/guide_images/s_textures.png",
        Color::Rgba(0.1, 0.5, 1.0, 0.8),
        |gc| {
            // Load the texture
            gc.create_texture(TextureId(0), w as _, h as _, TextureFormat::Rgba);
            gc.set_texture_bytes(
                TextureId(0),
                0,
                0,
                w as _,
                h as _,
                Arc::clone(&texture_data),
            );

            gc.new_path();
            gc.rect(-32.0, -32.0, 32.0, 32.0);
            gc.fill_texture(TextureId(0), -32.0, -32.0, 32.0, 32.0);
            gc.fill();
        },
    );
}

fn section_gradients() {
    section_badge(
        "draw/guide_images/s_gradients.png",
        Color::Rgba(0.1, 0.5, 1.0, 0.8),
        |gc| {
            gc.create_gradient(GradientId(0), Color::Rgba(0.9, 0.4, 0.1, 1.0));
            gc.gradient_stop(GradientId(0), 0.5, Color::Rgba(0.9, 0.9, 0.1, 1.0));
            gc.gradient_stop(GradientId(0), 1.0, Color::Rgba(0.4, 0.9, 0.4, 1.0));
            gc.gradient_stop(GradientId(0), 1.5, Color::Rgba(0.4, 0.9, 0.9, 1.0));
            gc.gradient_stop(GradientId(0), 1.5, Color::Rgba(0.1, 0.4, 0.9, 1.0));

            gc.new_path();
            gc.circle(0.0, 0.0, 40.0);
            gc.fill_gradient(GradientId(0), -32.0, -32.0, 32.0, 32.0);
            gc.fill();
        },
    );
}

fn section_text_rendering() {
    section_badge(
        "draw/guide_images/s_text_rendering.png",
        Color::Rgba(0.1, 0.5, 1.0, 0.8),
        |gc| {
            gc.transform(Transform2D::scale(1.0, -1.0));
            gc.define_font_data(FontId(0), Arc::clone(&LATO));

            gc.fill_color(Color::Rgba(0.7, 0.6, 0.2, 1.0));
            gc.set_font_size(FontId(0), 60.0);
            gc.begin_line_layout(0.0, -19.0, TextAlignment::Center);
            gc.layout_text(FontId(0), "Aa".to_string());
            gc.draw_text_layout();
        },
    );
}

fn section_text_layout() {
    section_badge(
        "draw/guide_images/s_text_layout.png",
        Color::Rgba(0.1, 0.5, 1.0, 0.8),
        |gc| {
            gc.transform(Transform2D::scale(1.0, -1.0));
            gc.define_font_data(FontId(0), Arc::clone(&LATO));

            let metrics = LATO.font_metrics(50.0).unwrap();
            let mut line_layout = CanvasFontLineLayout::new(&LATO, 50.0);

            let initial_point = line_layout.measure();

            line_layout.fill_color(Color::Rgba(0.8, 0.8, 0.8, 0.8));
            line_layout.add_text("A");
            let mid_point = line_layout.measure();
            line_layout.add_text("a");

            let end_point = line_layout.measure();

            line_layout.stroke_color(Color::Rgba(0.7, 0.6, 0.2, 1.0));

            line_layout.move_to(initial_point.pos.x() as _, initial_point.pos.y() as _);
            line_layout.line_to(end_point.pos.x() as _, end_point.pos.y() as _);
            line_layout.stroke();

            line_layout.move_to(
                initial_point.pos.x() as _,
                initial_point.pos.y() as f32 + metrics.descender,
            );
            line_layout.line_to(
                end_point.pos.x() as _,
                end_point.pos.y() as f32 + metrics.descender,
            );
            line_layout.stroke();

            line_layout.move_to(
                initial_point.pos.x() as _,
                initial_point.pos.y() as f32 + metrics.ascender,
            );
            line_layout.line_to(
                end_point.pos.x() as _,
                end_point.pos.y() as f32 + metrics.ascender,
            );
            line_layout.stroke();

            line_layout.move_to(
                initial_point.pos.x() as _,
                initial_point.pos.y() as f32 + metrics.capital_height.unwrap(),
            );
            line_layout.line_to(
                end_point.pos.x() as _,
                end_point.pos.y() as f32 + metrics.capital_height.unwrap(),
            );
            line_layout.stroke();

            line_layout.move_to(
                initial_point.pos.x() as _,
                initial_point.pos.y() as f32 + metrics.descender,
            );
            line_layout.line_to(
                initial_point.pos.x() as _,
                initial_point.pos.x() as f32 + metrics.ascender,
            );
            line_layout.stroke();

            line_layout.move_to(
                mid_point.pos.x() as _,
                initial_point.pos.y() as f32 + metrics.descender,
            );
            line_layout.line_to(
                mid_point.pos.x() as _,
                initial_point.pos.x() as f32 + metrics.ascender,
            );
            line_layout.stroke();

            line_layout.move_to(
                end_point.pos.x() as _,
                initial_point.pos.y() as f32 + metrics.descender,
            );
            line_layout.line_to(
                end_point.pos.x() as _,
                initial_point.pos.x() as f32 + metrics.ascender,
            );
            line_layout.stroke();

            line_layout.align_transform(0.0, -20.0, TextAlignment::Center);
            gc.draw_list(line_layout.to_drawing(FontId(0)));
        },
    );
}

fn section_animation() {
    section_badge(
        "draw/guide_images/s_animation.png",
        Color::Rgba(0.2, 0.2, 0.2, 0.8),
        |gc| {
            gc.new_path();
            gc.rect(-20.0, -20.0, 20.0, 20.0);
            gc.fill();
        },
    );
}

fn section_offscreen() {
    section_badge(
        "draw/guide_images/s_offscreen.png",
        Color::Rgba(0.2, 0.2, 0.2, 0.8),
        |gc| {
            gc.new_path();
            gc.rect(-20.0, -20.0, 20.0, 20.0);
            gc.fill();
        },
    );
}

fn section_event_handling() {
    section_badge(
        "draw/guide_images/s_event_handling.png",
        Color::Rgba(0.2, 0.2, 0.2, 0.8),
        |gc| {
            gc.new_path();
            gc.rect(-20.0, -20.0, 20.0, 20.0);
            gc.fill();
        },
    );
}

fn section_window_properties() {
    section_badge(
        "draw/guide_images/s_window_properties.png",
        Color::Rgba(0.2, 0.2, 0.2, 0.8),
        |gc| {
            gc.new_path();
            gc.rect(-20.0, -20.0, 20.0, 20.0);
            gc.fill();
        },
    );
}

fn section_draw_streaming() {
    section_badge(
        "draw/guide_images/s_draw_streaming.png",
        Color::Rgba(0.2, 0.2, 0.2, 0.8),
        |gc| {
            gc.new_path();
            gc.rect(-20.0, -20.0, 20.0, 20.0);
            gc.fill();
        },
    );
}

fn section_raw_render_streaming() {
    section_badge(
        "draw/guide_images/s_raw_render_streaming.png",
        Color::Rgba(0.2, 0.2, 0.2, 0.8),
        |gc| {
            gc.new_path();
            gc.rect(-20.0, -20.0, 20.0, 20.0);
            gc.fill();
        },
    );
}

fn section_encoding_decoding() {
    section_badge(
        "draw/guide_images/s_encoding_decoding.png",
        Color::Rgba(0.2, 0.2, 0.2, 0.8),
        |gc| {
            gc.new_path();
            gc.rect(-20.0, -20.0, 20.0, 20.0);
            gc.fill();
        },
    );
}

pub fn main() {
    section_graphics_primitives();
    section_transforms();
    section_layers();
    section_sprites();
    section_textures();
    section_gradients();
    section_text_rendering();
    section_text_layout();
    section_animation();
    section_offscreen();
    section_event_handling();
    section_window_properties();
    section_draw_streaming();
    section_raw_render_streaming();
    section_encoding_decoding();
}
