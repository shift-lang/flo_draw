/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use flo_render_software::edgeplan::*;
use flo_render_software::edges::*;
use flo_render_software::pixel::*;
use flo_render_software::render::*;
use flo_render_software::scanplan::*;

#[test]
fn render_rectangle() {
    // == Edge plan: draw two rectangles, one on top of the other in a foreground and background colour

    // The program data ID usually maps to the program cache (specifies what to do in a particular span)
    let program_data_id_1 = PixelProgramDataId(1);
    let program_data_id_2 = PixelProgramDataId(2);

    // One rectangle as a background, and one as a foreground
    let rectangle_shape_1 = ShapeId::new();
    let rectangle_shape_2 = ShapeId::new();
    let rectangle_edge_1 = RectangleEdge::new(rectangle_shape_1, 0.0..400.0, 0.0..300.0);
    let rectangle_edge_2 = RectangleEdge::new(rectangle_shape_2, 140.0..160.0, 140.0..160.0);
    let mut edge_plan = EdgePlan::new()
        .with_shape_description(
            rectangle_shape_1,
            ShapeDescriptor::opaque(program_data_id_1).with_z_index(0),
        )
        .with_shape_description(
            rectangle_shape_2,
            ShapeDescriptor::opaque(program_data_id_2).with_z_index(1),
        )
        .with_edge(rectangle_edge_1)
        .with_edge(rectangle_edge_2);

    edge_plan.prepare_to_render();

    // == Pixel programs: just render the requested colour

    // Create a program runner to fill in the pixels (white in the background, blue for the foreground)
    let background_col = F32LinearPixel::white();
    let foreground_col = F32LinearPixel::from_components([0.0, 0.0, 1.0, 1.0]);
    let program_runner = BasicPixelProgramRunner::from(
        |program, data: &mut [F32LinearPixel], range, _x_transform: &ScanlineTransform, _ypos| {
            let col = if program == program_data_id_2 {
                foreground_col
            } else {
                background_col
            };
            for x in range {
                data[x as usize] = col;
            }
        },
    );

    // == Render to a RGBA buffer using the basic PixelScanPlanner

    // Render with the basic scan planner
    let mut frame_data = vec![0u8; 400 * 300 * 4];
    render_frame_with_planner(
        PixelScanPlanner::default(),
        program_runner,
        &edge_plan,
        &mut RgbaFrame::from_bytes(400, 300, 2.2, &mut frame_data).unwrap(),
    );

    // == Assertions: check that the rectangles appear where they should in the frame we just rendered

    // Mid point should be inside the rectangle
    assert!(
        &frame_data[(150 * 4) + (150 * 400 * 4)..(151 * 4) + (150 * 400 * 4)] == &[0, 0, 255, 255],
        "Mid point is {:?}",
        &frame_data[(150 * 4) + (150 * 400 * 4)..(151 * 4) + (150 * 400 * 4)]
    );

    // Check the pixels
    for y in 0..300 {
        for x in 0..400 {
            let idx = (x * 4) + (y * 400 * 4);
            let pixel = &frame_data[idx..(idx + 4)];

            let expected_col = if x >= 140 && x < 160 && y >= 140 && y < 160 {
                [0, 0, 255, 255]
            } else {
                [255, 255, 255, 255]
            };

            assert!(
                pixel == &expected_col,
                "{:?} != {:?} (at {}, {})",
                pixel,
                expected_col,
                x,
                y
            );
        }
    }
}
