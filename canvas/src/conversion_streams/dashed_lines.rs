use crate::draw::*;
use crate::path::*;

use flo_curves::bezier::path::*;
use flo_curves::bezier::*;
use flo_stream::*;
use futures::prelude::*;

use std::iter;

///
/// Converts a bezier path to a set of paths by a dash patter
///
pub fn path_to_dashed_lines<PathIn, PathOut, DashPattern>(
    path_in: &PathIn,
    dash_pattern: DashPattern,
    pattern_offset: f64,
) -> Vec<PathOut>
    where
        PathIn: BezierPath,
        PathOut: BezierPathFactory<Point=PathIn::Point>,
        DashPattern: Iterator<Item=f64>,
{
    // Create the resulting set of paths (most will have just a single curve in them)
    let mut output_paths = vec![];

    // Cycle the dash pattern
    let dash_pattern = dash_pattern.collect::<Vec<_>>();
    let dash_pattern = if dash_pattern.len() == 0 {
        vec![1.0]
    } else {
        dash_pattern
    };
    let mut dash_pos = 0;
    let max_dash_pos = dash_pattern.len() - 1;

    // Initial remaining length is that of the first dash in the pattern
    let mut remaining_length = dash_pattern[dash_pos];

    // We alternate between drawing and not drawing dashes
    let mut draw_dash = false;

    // Apply the dash pattern offset
    if pattern_offset > 0.0 {
        let mut remaining_offset = pattern_offset;

        while remaining_offset > 0.0 {
            let dash_length = remaining_length;

            if dash_length > remaining_offset {
                remaining_length -= remaining_offset;
                break;
            } else {
                remaining_offset -= dash_length;

                dash_pos = if dash_pos >= max_dash_pos {
                    0
                } else {
                    dash_pos + 1
                };
                remaining_length = dash_pattern[dash_pos];
                draw_dash = !draw_dash;
            }
        }
    }

    // Generate dashed lines for each path segment
    let mut start_point = path_in.start_point();
    let mut current_path_start = start_point;
    let mut current_path_points = vec![];

    for (cp1, cp2, end_point) in path_in.points() {
        // Create a curve for this section
        let curve = Curve::from_points(start_point, (cp1, cp2), end_point);

        if remaining_length <= 0.0 {
            dash_pos = if dash_pos >= max_dash_pos {
                0
            } else {
                dash_pos + 1
            };
            remaining_length = dash_pattern[dash_pos];
            draw_dash = !draw_dash;
        }

        // Walk it, starting with the remaining length and then moving on according to the dash pattern
        let mut next_length = remaining_length;
        let curve_dash_pattern =
            iter::once(next_length).chain(dash_pattern.iter().cycle().skip(dash_pos + 1).cloned());

        for section in walk_curve_evenly(&curve, 1.0, 0.05).vary_by(curve_dash_pattern) {
            // Toggle if we show the dash or not
            draw_dash = !draw_dash;

            // walk_curve_evenly uses chord lengths (TODO: arc lengths would be better)
            let section_length = chord_length(&section);

            // Update the remaining length
            remaining_length = next_length - section_length;

            // Add the dash to the current path
            let (section_cp1, section_cp2) = section.control_points();
            let section_end_point = section.end_point();
            current_path_points.push((section_cp1, section_cp2, section_end_point));

            // If there's enough space for the whole dash, invert the 'draw_dash' state and add the current path to the result
            if remaining_length < 0.01 {
                // Add this dash to the output
                if draw_dash {
                    output_paths.push(PathOut::from_points(
                        current_path_start,
                        current_path_points,
                    ));
                }

                // Clear the current path
                current_path_start = section_end_point;
                current_path_points = vec![];
            }

            // Fetch the next length from the dash pattern
            dash_pos = if dash_pos >= max_dash_pos {
                0
            } else {
                dash_pos + 1
            };
            next_length = dash_pattern[dash_pos];
        }

        // Walk back a dash position (remaining_length is the distance left in this dash)
        dash_pos = if dash_pos == 0 {
            max_dash_pos
        } else {
            dash_pos - 1
        };
        draw_dash = !draw_dash;

        // The start point of the next curve in this path is the end point of this one
        start_point = end_point;
    }

    // If there's any remaining parts of the current path, add them
    if current_path_points.len() > 0 && draw_dash {
        output_paths.push(PathOut::from_points(
            current_path_start,
            current_path_points,
        ));
    }

    output_paths
}

///
/// Converts dashed line stroke operations into separate lines
///
pub fn drawing_without_dashed_lines<InStream: 'static + Send + Unpin + Stream<Item=Draw>>(
    draw_stream: InStream,
) -> impl Send + Unpin + Stream<Item=Draw> {
    generator_stream(move |yield_value| async move {
        let mut draw_stream = draw_stream;

        // The current path that will be affected
        let mut current_path = vec![];
        let mut last_point = Coord2(0.0, 0.0);
        let mut start_point = Coord2(0.0, 0.0);

        // The dash pattern to apply to the current path
        let mut current_dash_pattern = None;
        let mut dash_pattern_offset = 0.0;

        // Stack of stored changes for the paths and dash patterns
        let mut path_stack = vec![];
        let mut dash_pattern_stack = vec![];

        while let Some(drawing) = draw_stream.next().await {
            use self::Draw::*;
            use self::PathOp::*;

            match drawing {
                ClearCanvas(colour) => {
                    current_path = vec![];
                    last_point = Coord2(0.0, 0.0);
                    start_point = Coord2(0.0, 0.0);
                    current_dash_pattern = None;
                    dash_pattern_offset = 0.0;
                    path_stack = vec![];
                    dash_pattern_stack = vec![];

                    yield_value(ClearCanvas(colour)).await;
                }

                Path(NewPath) => {
                    current_path = vec![];
                    last_point = Coord2(0.0, 0.0);
                    start_point = Coord2(0.0, 0.0);

                    yield_value(Path(NewPath)).await;
                }

                Path(Move(x, y)) => {
                    current_path.push((Coord2(x as _, y as _), vec![]));

                    last_point = Coord2(x as _, y as _);
                    start_point = Coord2(x as _, y as _);

                    yield_value(Path(Move(x, y))).await;
                }

                Path(Line(x, y)) => {
                    let end_point = Coord2(x as _, y as _);
                    let cp1 = (end_point - last_point) * (1.0 / 3.0) + last_point;
                    let cp2 = (end_point - last_point) * (2.0 / 3.0) + last_point;
                    let line = (cp1, cp2, end_point);

                    current_path.last_mut().map(|path| path.1.push(line));

                    last_point = Coord2(x as _, y as _);

                    yield_value(Path(Line(x, y))).await;
                }

                Path(BezierCurve(((cp1x, cp1y), (cp2x, cp2y)), (x, y))) => {
                    let curve = (
                        Coord2(cp1x as _, cp1y as _),
                        Coord2(cp2x as _, cp2y as _),
                        Coord2(x as _, y as _),
                    );
                    current_path.last_mut().map(|path| path.1.push(curve));

                    last_point = Coord2(x as _, y as _);

                    yield_value(Path(BezierCurve(((cp1x, cp1y), (cp2x, cp2y)), (x, y)))).await;
                }

                Path(ClosePath) => {
                    let end_point = start_point;
                    let cp1 = (end_point - last_point) * (1.0 / 3.0) + last_point;
                    let cp2 = (end_point - last_point) * (2.0 / 3.0) + last_point;
                    let line = (cp1, cp2, end_point);

                    current_path.last_mut().map(|path| path.1.push(line));

                    yield_value(Path(ClosePath)).await;
                }

                NewDashPattern => {
                    // Invalidate the dash pattern
                    current_dash_pattern = None;
                    dash_pattern_offset = 0.0;
                }

                DashLength(length) => {
                    // Update the dash pattern
                    current_dash_pattern
                        .get_or_insert_with(|| vec![])
                        .push(length)
                }

                DashOffset(offset) => {
                    dash_pattern_offset = offset;
                }

                PushState => {
                    // Store the current dash pattern and path on the stack
                    path_stack.push(current_path.clone());
                    dash_pattern_stack.push(current_dash_pattern.clone());

                    yield_value(PushState).await;
                }

                PopState => {
                    // Restore the previously stored dash pattern/path
                    current_path = path_stack.pop().unwrap_or_else(|| vec![]);
                    current_dash_pattern = dash_pattern_stack.pop().unwrap_or(None);

                    yield_value(PopState).await;
                }

                Stroke => {
                    if let Some(dash_pattern) = &current_dash_pattern {
                        // Create a dash path and pass it through as a new path
                        yield_value(Path(NewPath)).await;

                        for subpath in current_path.iter() {
                            for (start_point, curves) in
                            path_to_dashed_lines::<_, SimpleBezierPath, _>(
                                subpath,
                                dash_pattern.iter().map(|p| (*p) as f64),
                                dash_pattern_offset as _,
                            )
                            {
                                yield_value(Path(Move(start_point.x() as _, start_point.y() as _)))
                                    .await;
                                for (Coord2(cp1x, cp1y), Coord2(cp2x, cp2y), Coord2(x, y)) in curves
                                {
                                    yield_value(Path(BezierCurve(
                                        ((cp1x as _, cp1y as _), (cp2x as _, cp2y as _)),
                                        (x as _, y as _),
                                    )))
                                        .await;
                                }
                            }
                        }

                        // Stroke the dashed line
                        yield_value(Stroke).await;

                        // Restore the original path
                        yield_value(Path(NewPath)).await;

                        for (start_point, curves) in current_path.iter() {
                            yield_value(Path(Move(start_point.x() as _, start_point.y() as _)))
                                .await;
                            for (Coord2(cp1x, cp1y), Coord2(cp2x, cp2y), Coord2(x, y)) in curves {
                                yield_value(Path(BezierCurve(
                                    ((*cp1x as _, *cp1y as _), (*cp2x as _, *cp2y as _)),
                                    (*x as _, *y as _),
                                )))
                                    .await;
                            }
                        }
                    } else {
                        // If there's no dash pattern, let the path through untouched
                        yield_value(Stroke).await;
                    }
                }

                drawing => {
                    // Pass the drawing on
                    yield_value(drawing).await;
                }
            }
        }
    })
}

#[cfg(test)]
mod test {
    use super::*;

    use futures::executor;
    use futures::stream;

    #[test]
    fn pass_through_normal_path() {
        let input_drawing = vec![
            Draw::Path(PathOp::NewPath),
            Draw::Path(PathOp::Move(10.0, 10.0)),
            Draw::Path(PathOp::Line(10.0, 100.0)),
            Draw::Path(PathOp::Line(100.0, 100.0)),
            Draw::Path(PathOp::Line(100.0, 10.0)),
            Draw::Path(PathOp::ClosePath),
        ];

        executor::block_on(async move {
            let without_dashed_lines =
                drawing_without_dashed_lines(stream::iter(input_drawing.into_iter()));
            let output_drawing = without_dashed_lines.collect::<Vec<_>>().await;

            assert!(
                output_drawing
                    == vec![
                    Draw::Path(PathOp::NewPath),
                    Draw::Path(PathOp::Move(10.0, 10.0)),
                    Draw::Path(PathOp::Line(10.0, 100.0)),
                    Draw::Path(PathOp::Line(100.0, 100.0)),
                    Draw::Path(PathOp::Line(100.0, 10.0)),
                    Draw::Path(PathOp::ClosePath),
                ]
            );
        });
    }

    #[test]
    fn simple_dashed_line() {
        let input_drawing = vec![
            Draw::NewDashPattern,
            Draw::DashLength(5.0),
            Draw::DashLength(5.0),
            Draw::Path(PathOp::NewPath),
            Draw::Path(PathOp::Move(10.0, 10.0)),
            Draw::Path(PathOp::Line(10.0, 100.0)),
            Draw::Stroke,
        ];

        executor::block_on(async move {
            let without_dashed_lines =
                drawing_without_dashed_lines(stream::iter(input_drawing.into_iter()));
            let output_drawing = without_dashed_lines.collect::<Vec<_>>().await;

            assert!(
                output_drawing
                    == vec![
                    Draw::Path(PathOp::NewPath),
                    Draw::Path(PathOp::Move(10.0, 10.0)),
                    Draw::Path(PathOp::Line(10.0, 100.0)),
                    Draw::Path(PathOp::NewPath),
                    Draw::Path(PathOp::Move(10.0, 10.0)),
                    Draw::Path(PathOp::BezierCurve(
                        ((10.0, 11.666667), (10.0, 13.333333)),
                        (10.0, 15.0),
                    )),
                    Draw::Path(PathOp::Move(10.0, 20.0)),
                    Draw::Path(PathOp::BezierCurve(
                        ((10.0, 21.666666), (10.0, 23.333334)),
                        (10.0, 25.0),
                    )),
                    Draw::Path(PathOp::Move(10.0, 30.0)),
                    Draw::Path(PathOp::BezierCurve(
                        ((10.0, 31.666666), (10.0, 33.333332)),
                        (10.0, 35.0),
                    )),
                    Draw::Path(PathOp::Move(10.0, 40.0)),
                    Draw::Path(PathOp::BezierCurve(
                        ((10.0, 41.666668), (10.0, 43.333332)),
                        (10.0, 45.0),
                    )),
                    Draw::Path(PathOp::Move(10.0, 50.0)),
                    Draw::Path(PathOp::BezierCurve(
                        ((10.0, 51.666668), (10.0, 53.333332)),
                        (10.0, 55.0),
                    )),
                    Draw::Path(PathOp::Move(10.0, 60.0)),
                    Draw::Path(PathOp::BezierCurve(
                        ((10.0, 61.666668), (10.0, 63.333332)),
                        (10.0, 65.0),
                    )),
                    Draw::Path(PathOp::Move(10.0, 70.0)),
                    Draw::Path(PathOp::BezierCurve(
                        ((10.0, 71.666664), (10.0, 73.333336)),
                        (10.0, 75.0),
                    )),
                    Draw::Path(PathOp::Move(10.0, 80.0)),
                    Draw::Path(PathOp::BezierCurve(
                        ((10.0, 81.666664), (10.0, 83.333336)),
                        (10.0, 85.0),
                    )),
                    Draw::Path(PathOp::Move(10.0, 90.0)),
                    Draw::Path(PathOp::BezierCurve(
                        ((10.0, 91.666664), (10.0, 93.333336)),
                        (10.0, 95.0),
                    )),
                    Draw::Stroke,
                    Draw::Path(PathOp::NewPath),
                    Draw::Path(PathOp::Move(10.0, 10.0)),
                    Draw::Path(PathOp::BezierCurve(
                        ((10.0, 40.0), (10.0, 70.0)),
                        (10.0, 100.0),
                    )),
                ]
            );
        });
    }
}
