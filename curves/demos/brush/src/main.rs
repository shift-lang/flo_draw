/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use flo_curves::arc::*;
use flo_curves::bezier::path::*;
use flo_curves::bezier::rasterize::*;
use flo_curves::bezier::vectorize::*;
use flo_curves::bezier::*;
use flo_curves::line::*;
use flo_curves::*;
use flo_draw::canvas::*;
use flo_draw::*;

use flo_curves::geo::Coord2;

use std::f64;
use std::thread;
use std::time::{Duration, Instant};

///
/// Creates a slow but accurate signed distance field from a path
///
fn slow_distance_field_from_path(path: Vec<SimpleBezierPath>) -> F64SampledDistanceField {
    // Use PathContour to determine if a point is inside or not, and also to generate an offset for the path
    let (contour, offset) = PathContour::center_path(path.clone(), 8);

    // Create the distance field by slowly measuring the path at every point
    let size = contour.contour_size();

    let distance_field = create_distance_field(
        |x, y| {
            let is_inside = contour_point_is_inside(&contour, ContourPosition(x as _, y as _));
            let distance = path
                .iter()
                .map(|subpath| path_closest_point(subpath, &(Coord2(x, y) + offset)))
                .map(|(_, _, distance, _)| distance)
                .reduce(|a, b| if a < b { a } else { b })
                .unwrap()
                .abs();

            if distance.is_nan() {
                panic!("NaN distance");
            }

            if is_inside {
                -distance
            } else {
                distance
            }
        },
        size,
    );

    let width = distance_field.contour_size().width();
    let height = distance_field.contour_size().height();
    for y in 0..height {
        for x in 0..width {
            let distance = distance_field.distance_at_point(ContourPosition(x, y));

            if distance.is_nan() {
                print!("/");
            }

            if distance < 0.0 {
                print!("#");
            } else if distance < 1.0 {
                print!("*");
            } else if distance < 4.0 {
                print!("!");
            } else if distance < 8.0 {
                print!(".");
            } else {
                print!(" ");
            }
        }

        println!();
    }

    println!();

    for y in 0..height {
        let intercepts = distance_field
            .as_contour()
            .rounded_intercepts_on_line(y as _);
        let mut line = vec![false; width];

        for range in intercepts {
            for x in range {
                line[x] = true;
            }
        }

        for x in 0..width {
            if line[x] {
                if contour_point_is_inside(&contour, ContourPosition(x, y)) {
                    print!("#");
                } else {
                    print!("?");
                }
            } else {
                if !contour_point_is_inside(&contour, ContourPosition(x, y)) {
                    print!(" ");
                } else {
                    print!(".");
                }
            }
        }

        println!();
    }

    println!();

    distance_field
}

///
/// Creates a brush stroke path
///
fn brush_stroke(center_x: f64, length: f64, width: f64, wiggle: f64) -> SimpleBezierPath3 {
    // Limit the range of the length
    let length = length.max(0.0);
    let length = length.min(800.0);

    // Create some curves by fitting along the length
    let brush_stroke = (0..(length as isize)).map(|p| {
        // p gives us the y position
        let p = p as f64;
        let y_pos = p + 100.0;

        let p = p / 800.0;
        let p = p * f64::consts::PI;

        let x_pos = center_x + (p * wiggle).sin() * 32.0;
        let width = p.sin().abs() * (width - 2.0) + 2.0;

        Coord3(x_pos, y_pos, width)
    });
    let brush_stroke = fit_curve::<Curve<_>>(&brush_stroke.collect::<Vec<_>>(), 0.1).unwrap();
    let brush_stroke = SimpleBezierPath3::from_connected_curves(brush_stroke);

    brush_stroke
}

///
/// Draws the outline of a path
///
fn draw_path_outline(
    gc: &mut (impl GraphicsPrimitives + GraphicsContext),
    path: impl IntoIterator<Item = SimpleBezierPath>,
    col1: Color,
    col2: Color,
) {
    gc.new_path();

    for subpath in path {
        let sp = subpath.start_point();
        gc.move_to(sp.x() as _, sp.y() as _);

        for curve in subpath.to_curves::<Curve<_>>() {
            let (_, (cp1, cp2), ep) = curve.all_points();
            gc.bezier_curve_to(
                ep.x() as _,
                ep.y() as _,
                cp1.x() as _,
                cp1.y() as _,
                cp2.x() as _,
                cp2.y() as _,
            );
        }
    }

    // Filled center
    gc.fill_color(Color::Rgba(0.6, 0.9, 0.9, 0.8));
    gc.fill();

    // Thick 'outer' path
    gc.line_width(4.0);
    gc.stroke_color(col1);
    gc.stroke();

    // Thin 'inner' path
    gc.line_width(2.0);
    gc.stroke_color(col2);
    gc.stroke();
}

///
/// Draws the end points of a path
///
fn draw_end_points(
    gc: &mut (impl GraphicsPrimitives + GraphicsContext),
    path: impl IntoIterator<Item = SimpleBezierPath>,
) {
    gc.sprite(SpriteId(0));
    gc.fill_color(Color::Rgba(0.0, 0.6, 1.0, 0.1));
    gc.new_path();
    gc.circle(0.0, 0.0, 4.0);
    gc.fill();

    gc.layer(LayerId(0));

    for subpath in path {
        for (_, _, ep) in subpath.points() {
            gc.sprite_transform(SpriteTransform::Identity);
            gc.sprite_transform(SpriteTransform::Translate(ep.x() as f32, ep.y() as f32));
            gc.draw_sprite(SpriteId(0));
        }
    }
}

///
/// Draws the outline of a simple brush stroke by offsetting the path
///
fn draw_offset_brush_stroke(
    gc: &mut (impl GraphicsPrimitives + GraphicsContext),
    center_x: f64,
    length: f64,
) {
    let brush_stroke = brush_stroke(center_x, length, 20.0, 16.0);

    // Offset the curves in the brush stroke to generate the reuslt
    let offsets = brush_stroke
        .to_curves::<Curve<_>>()
        .into_iter()
        .map(|curve| {
            let (sp, (cp1, cp2), ep) = curve.all_points();
            let base_curve = Curve::from_points(
                Coord2(sp.x(), sp.y()),
                (Coord2(cp1.x(), cp1.y()), Coord2(cp2.x(), cp2.y())),
                Coord2(ep.x(), ep.y()),
            );
            let distance_curve = Curve::from_points(sp.z(), (cp1.z(), cp2.z()), ep.z());

            let outwards = offset_lms_sampling(
                &base_curve,
                |t| distance_curve.point_at_pos(t),
                |_| 0.0,
                400,
                0.1,
            )
            .unwrap();
            let inwards = offset_lms_sampling(
                &base_curve,
                |t| -distance_curve.point_at_pos(t),
                |_| 0.0,
                400,
                0.1,
            )
            .unwrap();

            (inwards, outwards)
        })
        .collect::<Vec<_>>();

    let mut curves = vec![];

    for (inward, _) in offsets.iter() {
        curves.extend(inward.clone());
    }

    let inward_end = offsets
        .last()
        .map(|(inward, _)| *inward.last().unwrap())
        .unwrap();
    let outward_end = offsets
        .last()
        .map(|(_, outward)| *outward.last().unwrap())
        .unwrap();
    let end_cap = line_to_bezier(&(inward_end.end_point(), outward_end.end_point()));
    curves.push(end_cap);

    for (_, outward) in offsets.iter().rev() {
        curves.extend(
            outward
                .iter()
                .rev()
                .map(|curve| curve.reverse::<Curve<_>>()),
        );
    }

    let brush_stroke_path = SimpleBezierPath::from_connected_curves(curves);

    // Draw it as a preview
    draw_path_outline(
        gc,
        vec![brush_stroke_path.clone()],
        Color::Rgba(1.0, 0.8, 0.8, 1.0),
        Color::Rgba(0.1, 0.1, 0.1, 1.0),
    );
    draw_end_points(gc, vec![brush_stroke_path]);
}

///
/// Draws the outline of a simple brush stroke using the 'circular' brush head
///
fn draw_circle_brush_stroke(
    gc: &mut (impl GraphicsPrimitives + GraphicsContext),
    center_x: f64,
    length: f64,
) {
    let brush_stroke = brush_stroke(center_x, length, 20.0, 16.0);

    // Use the circular brush
    let brush = CircularBrush;

    // Use the brush to create a brush stroke path
    let brush_stroke_path =
        brush_stroke_from_path::<SimpleBezierPath, _, _>(&brush, &brush_stroke, 0.5, 0.5);

    // Draw it as a preview
    draw_path_outline(
        gc,
        brush_stroke_path.iter().cloned(),
        Color::Rgba(1.0, 0.8, 0.8, 1.0),
        Color::Rgba(0.1, 0.1, 0.1, 1.0),
    );
    draw_end_points(gc, brush_stroke_path.iter().cloned());
}

///
/// Draws the outline of a simple brush stroke alongside an image of the brush head
///
fn draw_path_brush_stroke(
    gc: &mut (impl GraphicsPrimitives + GraphicsContext),
    center_x: f64,
    length: f64,
    brush_head: Vec<SimpleBezierPath>,
) {
    let bounds = brush_head
        .iter()
        .map(|subpath| subpath.bounding_box::<Bounds<_>>())
        .reduce(|a, b| a.union_bounds(b))
        .unwrap();

    // Create some curves by fitting along the length
    let brush_stroke = brush_stroke(center_x, length, 40.0, 7.0);

    // Draw the brush preview
    let offset = bounds.min();
    let size = bounds.max() - bounds.min();
    let scale = size.x().max(size.y());

    let preview = brush_head.iter().map(|subpath| {
        subpath.map_points::<SimpleBezierPath>(|point| {
            (point - offset - (size * 0.5)) * (1.0 / scale) * 32.0 + Coord2(center_x, 50.0)
        })
    });

    draw_path_outline(
        gc,
        preview,
        Color::Rgba(0.4, 0.85, 1.0, 1.0),
        Color::Rgba(0.1, 0.1, 0.1, 1.0),
    );

    // Create a brush from the path
    let (field, _) = PathDistanceField::center_path(brush_head, 4);
    let brush = ScaledBrush::from_distance_field(&field);
    let brush = &brush;

    // Use the brush to create a brush stroke path
    let brush_stroke_path = brush_stroke_from_path_intercepts::<SimpleBezierPath, _, _>(
        &brush,
        &brush_stroke,
        0.5,
        0.5,
    );

    // Draw it as a preview
    draw_path_outline(
        gc,
        brush_stroke_path.iter().cloned(),
        Color::Rgba(1.0, 0.8, 0.8, 1.0),
        Color::Rgba(0.1, 0.1, 0.1, 1.0),
    );
    draw_end_points(gc, brush_stroke_path.iter().cloned());
}

///
/// Draws the outline of a simple brush stroke alongside an image of the brush head
///
fn draw_field_brush_stroke(
    gc: &mut (impl GraphicsPrimitives + GraphicsContext),
    center_x: f64,
    length: f64,
    brush_head: &impl SampledSignedDistanceField,
) {
    // Create some curves by fitting along the length
    let brush_stroke = brush_stroke(center_x, length, 40.0, 7.0);

    // Draw a brush preview
    let preview = trace_paths_from_distance_field::<SimpleBezierPath>(brush_head, 0.1);

    let bounds = preview
        .iter()
        .map(|subpath| subpath.bounding_box::<Bounds<_>>())
        .reduce(|a, b| a.union_bounds(b))
        .unwrap();
    let offset = bounds.min();
    let size = bounds.max() - bounds.min();
    let scale = size.x().max(size.y());

    let preview = preview.iter().map(|subpath| {
        subpath.map_points::<SimpleBezierPath>(|point| {
            if point.x().is_nan() || point.y().is_nan() {
                panic!("Preview has a NaN point");
            }

            (point - offset - (size * 0.5)) * (1.0 / scale) * 32.0 + Coord2(center_x, 50.0)
        })
    });

    draw_path_outline(
        gc,
        preview,
        Color::Rgba(0.4, 0.85, 1.0, 1.0),
        Color::Rgba(0.1, 0.1, 0.1, 1.0),
    );

    // Create a brush from the path
    let brush = ScaledBrush::from_distance_field(brush_head);
    let brush = &brush;

    // Use the brush to create a brush stroke path
    let brush_stroke_path =
        brush_stroke_from_path::<SimpleBezierPath, _, _>(&brush, &brush_stroke, 0.5, 0.5);

    // Draw it as a preview
    draw_path_outline(
        gc,
        brush_stroke_path.iter().cloned(),
        Color::Rgba(1.0, 0.8, 0.8, 1.0),
        Color::Rgba(0.1, 0.1, 0.1, 1.0),
    );
    draw_end_points(gc, brush_stroke_path.iter().cloned());
}

fn main() {
    with_2d_graphics(|| {
        let canvas = create_canvas_window("Brush demo");
        let start_time = Instant::now();

        let chisel = BezierPathBuilder::<SimpleBezierPath>::start(Coord2(0.0, 0.0))
            .line_to(Coord2(12.0, 36.0))
            .line_to(Coord2(36.0, 48.0))
            .line_to(Coord2(24.0, 12.0))
            .line_to(Coord2(0.0, 0.0))
            .build();
        let chisel_field = slow_distance_field_from_path(vec![chisel.clone()]);

        let scale = 1.0 / 6.0;
        let angle = 2.0 * f64::consts::PI / 6.0;
        let oblique = Circle::new(Coord2(0.0, 0.0), 48.0)
            .to_path::<SimpleBezierPath>()
            .map_points::<SimpleBezierPath>(|p| Coord2(p.x() * scale, p.y()))
            .map_points::<SimpleBezierPath>(|p| {
                Coord2(
                    angle.sin() * p.x() + angle.cos() * p.y(),
                    angle.cos() * p.x() - angle.sin() * p.y(),
                )
            });

        let two_circles = vec![
            Circle::new(Coord2(0.0, 0.0), 8.0).to_path::<SimpleBezierPath>(),
            Circle::new(Coord2(24.0, 24.0), 8.0).to_path::<SimpleBezierPath>(),
        ];
        let two_circles_field = slow_distance_field_from_path(two_circles.clone());

        loop {
            thread::sleep(Duration::from_nanos(1_000_000_000 / 60));

            let since_start = Instant::now().duration_since(start_time);
            let since_start = since_start.as_nanos() as f64;
            let since_start = since_start / 1_000_000_000.0;

            let length = ((since_start / 5.0).sin() + 1.0) / 2.0;
            let length = length * 790.0;
            let length = length % 790.0;
            let length = length + 10.0;

            canvas.draw(|gc| {
                gc.clear_canvas(Color::Rgba(1.0, 1.0, 1.0, 1.0));

                gc.canvas_height(1000.0);
                gc.center_region(0.0, 0.0, 1000.0, 1000.0);

                gc.winding_rule(WindingRule::EvenOdd);

                draw_offset_brush_stroke(gc, 120.0, length);
                draw_circle_brush_stroke(gc, 240.0, length);
                draw_path_brush_stroke(
                    gc,
                    360.0,
                    length,
                    vec![Circle::new(Coord2(0.0, 0.0), 32.0).to_path::<SimpleBezierPath>()],
                );
                draw_path_brush_stroke(gc, 480.0, length, vec![chisel.clone()]);
                draw_field_brush_stroke(gc, 600.0, length, &chisel_field);
                draw_path_brush_stroke(gc, 720.0, length, vec![oblique.clone()]);
                draw_path_brush_stroke(gc, 840.0, length, two_circles.clone());
                draw_field_brush_stroke(gc, 960.0, length, &two_circles_field);
            });
        }
    });
}
