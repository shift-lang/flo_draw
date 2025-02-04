/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use flo_curves::bezier;
use flo_curves::bezier::path::*;
use flo_curves::*;
use flo_draw::canvas::*;
use flo_draw::*;

use std::f64;
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    with_2d_graphics(|| {
        let canvas = create_canvas_window("Curve and path distortion demonstration");

        // Simple rectangle to use as a source path
        let source_path = BezierPathBuilder::<SimpleBezierPath>::start(Coord2(300.0, 500.0))
            .line_to(Coord2(700.0, 500.0))
            .line_to(Coord2(700.0, 900.0))
            .line_to(Coord2(300.0, 900.0))
            .line_to(Coord2(300.0, 500.0))
            .build();

        // Line to use as a source curve
        let source_curve = bezier::Curve::from_points(
            Coord2(300.0, 200.0),
            (Coord2(300.0, 300.0), Coord2(700.0, 100.0)),
            Coord2(700.0, 200.0),
        );

        // We'll change the amount of distortion over time
        let start_time = Instant::now();

        loop {
            // Wait for the next frame
            thread::sleep(Duration::from_nanos(1_000_000_000 / 60));

            // Generate a distortion of the source path
            let since_start = Instant::now().duration_since(start_time);
            let since_start = since_start.as_nanos() as f64;
            let amplitude = (since_start / (f64::consts::PI * 500_000_000.0)).sin() * 50.0;

            let distorted_path = bezier::distort_path::<_, _, SimpleBezierPath>(
                &source_path,
                |point, _curve, _t| {
                    let distance = point.magnitude();
                    let ripple = (since_start / (f64::consts::PI * 500_000_000.0)) * 10.0;

                    let offset_x =
                        (distance / (f64::consts::PI * 5.0) + ripple).sin() * amplitude * 0.5;
                    let offset_y =
                        (distance / (f64::consts::PI * 4.0) + ripple).cos() * amplitude * 0.5;

                    Coord2(point.x() + offset_x, point.y() + offset_y)
                },
                1.0,
                0.1,
            )
            .unwrap();

            let distorted_curve = bezier::distort_curve::<_, _, bezier::Curve<Coord2>>(
                &source_curve,
                |point, _t| {
                    let offset_x = (point.x() / (f64::consts::PI * 25.0) * (amplitude / 50.0))
                        .sin()
                        * amplitude
                        * 2.0;
                    let offset_y = (point.x() / (f64::consts::PI * 12.0)).cos() * amplitude * 2.0;

                    Coord2(point.x() + offset_x, point.y() + offset_y)
                },
                1.0,
                0.1,
            )
            .unwrap();

            canvas.draw(|gc| {
                gc.clear_canvas(Color::Rgba(1.0, 1.0, 1.0, 1.0));

                gc.canvas_height(1000.0);
                gc.center_region(0.0, 0.0, 1000.0, 1000.0);

                gc.line_width(2.0);

                // Render the distorted curve and the original
                gc.stroke_color(Color::Rgba(0.8, 0.6, 0.0, 1.0));
                gc.new_path();
                gc.move_to(
                    source_curve.start_point().x() as _,
                    source_curve.start_point.y() as _,
                );
                gc.bezier_curve(&source_curve);
                gc.stroke();

                gc.stroke_color(Color::Rgba(0.0, 0.6, 0.8, 1.0));
                gc.new_path();
                gc.move_to(
                    distorted_curve[0].start_point().x() as _,
                    distorted_curve[0].start_point.y() as _,
                );
                for curve in distorted_curve.iter() {
                    gc.bezier_curve(curve);
                }
                gc.stroke();

                // Render the distorted path and the original
                gc.stroke_color(Color::Rgba(0.8, 0.6, 0.0, 1.0));
                gc.new_path();
                gc.bezier_path(&source_path);
                gc.stroke();

                gc.stroke_color(Color::Rgba(0.0, 0.6, 0.8, 1.0));
                gc.new_path();
                gc.bezier_path(&distorted_path);
                gc.stroke();

                if (since_start % 10_000_000_000.0) > 5_000_000_000.0 {
                    // Render the path points
                    gc.line_width(1.0);

                    for (_, _, point) in distorted_path.1.iter() {
                        gc.new_path();
                        gc.circle(point.x() as _, point.y() as _, 5.0);
                        gc.stroke();
                    }
                }
            });
        }
    });
}
