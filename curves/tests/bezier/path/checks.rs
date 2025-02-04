/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use flo_curves::bezier::path::*;
use flo_curves::bezier::*;

///
/// Returns true if the end points in the path match the expected end points (in any sequence, forwards or backwards)
///
pub fn path_has_end_points_in_order<PIn>(
    path: PIn,
    expected_end_points: Vec<PIn::Point>,
    max_error: f64,
) -> bool
where
    PIn: BezierPath,
    PIn::Point: std::fmt::Debug,
{
    // Collect the points in the path
    let end_points = path.points().map(|pt| pt.2).collect::<Vec<_>>();

    if expected_end_points.len() != end_points.len() {
        assert!(
            false,
            "Number of end points differs ({} != {})",
            end_points.len(),
            expected_end_points.len()
        );
        return false;
    }

    // Path must be fully closed to use this function
    let start_distance = end_points.last().unwrap().distance_to(&path.start_point());
    assert!(
        start_distance < max_error,
        "Path is not fully closed, start point at distance {} from end point",
        start_distance
    );

    // Find the first point in the list
    let first_point = end_points
        .iter()
        .enumerate()
        .filter(|(_, pt)| pt.distance_to(&expected_end_points[0]) <= max_error)
        .map(|(idx, _)| idx)
        .next();

    let first_point_idx = if let Some(idx) = first_point {
        idx
    } else {
        assert!(
            false,
            "Could not find first point {:?}",
            expected_end_points[0]
        );
        return false;
    };

    // Done if there's only one expected point
    if end_points.len() == 1 {
        return true;
    }

    // Decide if we're looking forwards or backwards based on the following point
    let next_idx = if first_point_idx + 1 >= end_points.len() {
        0
    } else {
        first_point_idx + 1
    };
    let prev_idx = if first_point_idx == 0 {
        end_points.len() - 1
    } else {
        first_point_idx - 1
    };

    let next_dist = expected_end_points[1].distance_to(&end_points[next_idx]);
    let prev_dist = expected_end_points[1].distance_to(&end_points[prev_idx]);

    if next_dist < prev_dist {
        // Check forward
        for offset in 1..end_points.len() {
            let idx = first_point_idx + offset;
            let idx = if idx >= end_points.len() {
                idx - end_points.len()
            } else {
                idx
            };

            let dist = expected_end_points[offset].distance_to(&end_points[idx]);

            if dist > max_error {
                assert!(false, "Point {} does not match (distance {}, checking forward), expected {:?}, got {:?}", offset, dist, expected_end_points[offset], end_points[idx]);
                return false;
            }
        }
    } else {
        // Check backwards
        for offset in 1..end_points.len() {
            let idx = first_point_idx + end_points.len() - offset;
            let idx = if idx >= end_points.len() {
                idx - end_points.len()
            } else {
                idx
            };

            let dist = expected_end_points[offset].distance_to(&end_points[idx]);

            if dist > max_error {
                assert!(false, "Point {} does not match (distance {}, checking backwards), expected {:?}, got {:?}", offset, dist, expected_end_points[offset], end_points[offset]);
                return false;
            }
        }
    }

    true
}

///
/// Returns true if the points in the path match the expected end points (in any sequence, forwards or backwards)
///
pub fn path_has_points_in_order<PIn>(
    path: PIn,
    expected_points: Vec<(PIn::Point, PIn::Point, PIn::Point)>,
    max_error: f64,
) -> bool
where
    PIn: BezierPath,
    PIn::Point: std::fmt::Debug,
{
    // Collect the points in the path
    let end_points = path.points().collect::<Vec<_>>();

    if expected_points.len() != end_points.len() {
        assert!(
            false,
            "Number of end points differs ({} != {})",
            end_points.len(),
            expected_points.len()
        );
        return false;
    }

    // Path must be fully closed to use this function
    let start_distance = end_points
        .last()
        .unwrap()
        .2
        .distance_to(&path.start_point());
    assert!(
        start_distance < max_error,
        "Path is not fully closed, start point at distance {} from end point",
        start_distance
    );

    // Find the first point in the list
    let first_point = end_points
        .iter()
        .enumerate()
        .filter(|(_, pt)| pt.2.distance_to(&expected_points[0].2) <= max_error)
        .map(|(idx, _)| idx)
        .next();

    let first_point_idx = if let Some(idx) = first_point {
        idx
    } else {
        assert!(
            false,
            "Could not find first point {:?}",
            expected_points[0].2
        );
        return false;
    };

    // Done if there's only one expected point
    if end_points.len() == 1 {
        return true;
    }

    // Decide if we're looking forwards or backwards based on the following point
    let next_idx = if first_point_idx + 1 >= end_points.len() {
        0
    } else {
        first_point_idx + 1
    };
    let prev_idx = if first_point_idx == 0 {
        end_points.len() - 1
    } else {
        first_point_idx - 1
    };

    let next_dist = expected_points[1].2.distance_to(&end_points[next_idx].2);
    let prev_dist = expected_points[1].2.distance_to(&end_points[prev_idx].2);

    if next_dist < prev_dist {
        // Check forward
        for offset in 0..end_points.len() {
            let idx = first_point_idx + offset;
            let idx = if idx >= end_points.len() {
                idx - end_points.len()
            } else {
                idx
            };

            let dist_0 = expected_points[offset].0.distance_to(&end_points[idx].0);
            let dist_1 = expected_points[offset].1.distance_to(&end_points[idx].1);
            let dist_2 = expected_points[offset].2.distance_to(&end_points[idx].2);

            if dist_0 > max_error {
                assert!(false, "Point {} cp1 does not match (distance {}, checking forward), expected {:?}, got {:?}", offset, dist_0, expected_points[offset].0, end_points[idx].0);
                return false;
            }
            if dist_1 > max_error {
                assert!(false, "Point {} cp2 does not match (distance {}, checking forward), expected {:?}, got {:?}", offset, dist_1, expected_points[offset].1, end_points[idx].1);
                return false;
            }
            if dist_2 > max_error {
                assert!(false, "Point {} end point does not match (distance {}, checking forward), expected {:?}, got {:?}", offset, dist_2, expected_points[offset].2, end_points[idx].2);
                return false;
            }
        }
    } else {
        // Check backwards
        for offset in 0..end_points.len() {
            let idx = first_point_idx + end_points.len() - offset;
            let idx = if idx >= end_points.len() {
                idx - end_points.len()
            } else {
                idx
            };

            let next_offset = if offset + 1 < expected_points.len() {
                offset + 1
            } else {
                0
            };

            let dist_0 = expected_points[next_offset]
                .1
                .distance_to(&end_points[idx].0);
            let dist_1 = expected_points[next_offset]
                .0
                .distance_to(&end_points[idx].1);
            let dist_2 = expected_points[offset].2.distance_to(&end_points[idx].2);

            if dist_0 > max_error {
                assert!(false, "Point {} cp1 does not match (distance {}, checking forward), expected {:?}, got {:?}", offset, dist_0, expected_points[next_offset].1, end_points[idx].0);
                return false;
            }
            if dist_1 > max_error {
                assert!(false, "Point {} cp2 does not match (distance {}, checking forward), expected {:?}, got {:?}", offset, dist_1, expected_points[next_offset].0, end_points[idx].1);
                return false;
            }
            if dist_2 > max_error {
                assert!(false, "Point {} end point does not match (distance {}, checking forward), expected {:?}, got {:?}", offset, dist_2, expected_points[offset].2, end_points[idx].2);
                return false;
            }
        }
    }

    true
}

#[test]
fn check_end_points_forward_no_offset() {
    let rectangle1 = BezierPathBuilder::<SimpleBezierPath>::start(Coord2(1.0, 1.0))
        .line_to(Coord2(5.0, 1.0))
        .line_to(Coord2(5.0, 5.0))
        .line_to(Coord2(1.0, 5.0))
        .line_to(Coord2(1.0, 1.0))
        .build();

    assert!(path_has_end_points_in_order(
        rectangle1,
        vec![
            Coord2(1.0, 1.0),
            Coord2(5.0, 1.0),
            Coord2(5.0, 5.0),
            Coord2(1.0, 5.0)
        ],
        0.1
    ));
}

#[test]
fn check_end_points_backwards_no_offset() {
    let rectangle1 = BezierPathBuilder::<SimpleBezierPath>::start(Coord2(1.0, 1.0))
        .line_to(Coord2(5.0, 1.0))
        .line_to(Coord2(5.0, 5.0))
        .line_to(Coord2(1.0, 5.0))
        .line_to(Coord2(1.0, 1.0))
        .build();

    assert!(path_has_end_points_in_order(
        rectangle1,
        vec![
            Coord2(1.0, 1.0),
            Coord2(1.0, 5.0),
            Coord2(5.0, 5.0),
            Coord2(5.0, 1.0)
        ],
        0.1
    ));
}

#[test]
fn check_end_points_forward_rotated_path() {
    let rectangle1 = BezierPathBuilder::<SimpleBezierPath>::start(Coord2(5.0, 1.0))
        .line_to(Coord2(5.0, 5.0))
        .line_to(Coord2(1.0, 5.0))
        .line_to(Coord2(1.0, 1.0))
        .line_to(Coord2(5.0, 1.0))
        .build();

    assert!(path_has_end_points_in_order(
        rectangle1,
        vec![
            Coord2(1.0, 1.0),
            Coord2(5.0, 1.0),
            Coord2(5.0, 5.0),
            Coord2(1.0, 5.0)
        ],
        0.1
    ));
}

#[test]
#[should_panic]
fn check_end_points_forward_different_path() {
    let rectangle1 = BezierPathBuilder::<SimpleBezierPath>::start(Coord2(5.0, 1.0))
        .line_to(Coord2(5.0, 5.0))
        .line_to(Coord2(1.0, 5.0))
        .line_to(Coord2(1.0, 1.0))
        .line_to(Coord2(5.0, 1.0))
        .build();

    assert!(path_has_end_points_in_order(
        rectangle1,
        vec![
            Coord2(1.0, 1.0),
            Coord2(5.0, 1.0),
            Coord2(5.0, 1.0),
            Coord2(1.0, 5.0)
        ],
        0.1
    ));
}

#[test]
fn check_points_forward_no_offset() {
    let rectangle1 = BezierPathBuilder::<SimpleBezierPath>::start(Coord2(1.0, 1.0))
        .line_to(Coord2(5.0, 1.0))
        .line_to(Coord2(5.0, 5.0))
        .line_to(Coord2(1.0, 5.0))
        .line_to(Coord2(1.0, 1.0))
        .build();
    let rectangle2 = BezierPathBuilder::<SimpleBezierPath>::start(Coord2(1.0, 1.0))
        .line_to(Coord2(5.0, 1.0))
        .line_to(Coord2(5.0, 5.0))
        .line_to(Coord2(1.0, 5.0))
        .line_to(Coord2(1.0, 1.0))
        .build();

    assert!(path_has_points_in_order(
        rectangle1,
        rectangle2.points().collect(),
        0.1
    ));
}

#[test]
fn check_points_forward_with_offset() {
    let rectangle1 = BezierPathBuilder::<SimpleBezierPath>::start(Coord2(1.0, 1.0))
        .line_to(Coord2(5.0, 1.0))
        .line_to(Coord2(5.0, 5.0))
        .line_to(Coord2(1.0, 5.0))
        .line_to(Coord2(1.0, 1.0))
        .build();
    let rectangle2 = BezierPathBuilder::<SimpleBezierPath>::start(Coord2(5.0, 1.0))
        .line_to(Coord2(5.0, 5.0))
        .line_to(Coord2(1.0, 5.0))
        .line_to(Coord2(1.0, 1.0))
        .line_to(Coord2(5.0, 1.0))
        .build();

    assert!(path_has_points_in_order(
        rectangle1,
        rectangle2.points().collect(),
        0.1
    ));
}

#[test]
fn check_points_backwards_no_offset() {
    let rectangle1 = BezierPathBuilder::<SimpleBezierPath>::start(Coord2(1.0, 1.0))
        .line_to(Coord2(5.0, 1.0))
        .line_to(Coord2(5.0, 5.0))
        .line_to(Coord2(1.0, 5.0))
        .line_to(Coord2(1.0, 1.0))
        .build();
    let rectangle2 = BezierPathBuilder::<SimpleBezierPath>::start(Coord2(1.0, 1.0))
        .line_to(Coord2(1.0, 5.0))
        .line_to(Coord2(5.0, 5.0))
        .line_to(Coord2(5.0, 1.0))
        .line_to(Coord2(1.0, 1.0))
        .build();

    assert!(path_has_points_in_order(
        rectangle1,
        rectangle2.points().collect(),
        0.1
    ));
}

#[test]
fn check_points_backwards_offset() {
    let rectangle1 = BezierPathBuilder::<SimpleBezierPath>::start(Coord2(1.0, 1.0))
        .line_to(Coord2(5.0, 1.0))
        .line_to(Coord2(5.0, 5.0))
        .line_to(Coord2(1.0, 5.0))
        .line_to(Coord2(1.0, 1.0))
        .build();
    let rectangle2 = BezierPathBuilder::<SimpleBezierPath>::start(Coord2(1.0, 5.0))
        .line_to(Coord2(5.0, 5.0))
        .line_to(Coord2(5.0, 1.0))
        .line_to(Coord2(1.0, 1.0))
        .line_to(Coord2(1.0, 5.0))
        .build();

    assert!(path_has_points_in_order(
        rectangle1,
        rectangle2.points().collect(),
        0.1
    ));
}

#[test]
#[should_panic]
fn check_points_backwards_different() {
    let rectangle1 = BezierPathBuilder::<SimpleBezierPath>::start(Coord2(1.0, 1.0))
        .line_to(Coord2(5.0, 1.0))
        .line_to(Coord2(5.0, 5.0))
        .line_to(Coord2(1.0, 5.0))
        .line_to(Coord2(1.0, 1.0))
        .build();
    let rectangle2 = BezierPathBuilder::<SimpleBezierPath>::start(Coord2(1.0, 1.0))
        .line_to(Coord2(1.0, 5.0))
        .line_to(Coord2(5.0, 5.0))
        .line_to(Coord2(1.0, 5.0))
        .line_to(Coord2(1.0, 1.0))
        .build();

    assert!(path_has_points_in_order(
        rectangle1,
        rectangle2.points().collect(),
        0.1
    ));
}

#[test]
#[should_panic]
fn check_points_forward_cp_different() {
    let rectangle1 = BezierPathBuilder::<SimpleBezierPath>::start(Coord2(1.0, 1.0))
        .line_to(Coord2(5.0, 1.0))
        .line_to(Coord2(5.0, 5.0))
        .line_to(Coord2(1.0, 5.0))
        .line_to(Coord2(1.0, 1.0))
        .build();
    let rectangle2 = BezierPathBuilder::<SimpleBezierPath>::start(Coord2(1.0, 1.0))
        .line_to(Coord2(5.0, 1.0))
        .curve_to((Coord2(10.0, 10.0), Coord2(20.0, 20.0)), Coord2(5.0, 5.0))
        .line_to(Coord2(1.0, 5.0))
        .line_to(Coord2(1.0, 1.0))
        .build();

    assert!(path_has_points_in_order(
        rectangle1,
        rectangle2.points().collect(),
        0.1
    ));
}

#[test]
#[should_panic]
fn check_points_backwards_cp_different() {
    let rectangle1 = BezierPathBuilder::<SimpleBezierPath>::start(Coord2(1.0, 1.0))
        .line_to(Coord2(5.0, 1.0))
        .line_to(Coord2(5.0, 5.0))
        .line_to(Coord2(1.0, 5.0))
        .line_to(Coord2(1.0, 1.0))
        .build();
    let rectangle2 = BezierPathBuilder::<SimpleBezierPath>::start(Coord2(1.0, 1.0))
        .line_to(Coord2(1.0, 5.0))
        .curve_to((Coord2(10.0, 10.0), Coord2(20.0, 20.0)), Coord2(5.0, 5.0))
        .line_to(Coord2(5.0, 1.0))
        .line_to(Coord2(1.0, 1.0))
        .build();

    assert!(path_has_points_in_order(
        rectangle1,
        rectangle2.points().collect(),
        0.1
    ));
}
