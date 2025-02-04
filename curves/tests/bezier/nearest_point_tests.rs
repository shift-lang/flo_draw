/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use flo_curves::bezier::*;
use flo_curves::line::*;
use flo_curves::*;

use std::ops::Range;

fn nearest_t_value_iteration<C>(curve: &C, point: &C::Point) -> f64
where
    C: BezierCurve,
{
    let mut min_distance = f64::MAX;
    let mut min_t = 0.0;

    // Walk the curve in increments of .1 pixel
    for t in walk_curve_evenly(curve, 0.1, 0.01).map(|section| section.t_for_t(0.5)) {
        let distance = curve.point_at_pos(t).distance_to(point);

        if distance < min_distance {
            min_distance = distance;
            min_t = t;
        }
    }

    min_t
}

fn test_point_grid<C>(curve: &C, x_range: Range<f64>, y_range: Range<f64>, step: f64)
where
    C: BezierCurve<Point = Coord2>,
{
    // Iterate through the sample points
    let mut y = y_range.start;
    while y < y_range.end {
        let mut x = x_range.start;
        while x < x_range.end {
            // Sample at this point
            let nearest = curve.nearest_point(&Coord2(x, y));

            // Also search by iterating over the curve to get an 'accurate' value
            let iter_nearest = curve.point_at_pos(nearest_t_value_iteration(curve, &Coord2(x, y)));

            let nearest_to_point = Coord2(x, y).distance_to(&nearest);
            let iter_to_point = Coord2(x, y).distance_to(&iter_nearest);
            let distance_diff = nearest.distance_to(&iter_nearest);

            assert!(
                nearest_to_point <= iter_to_point,
                "Found {:?} but iteration found {:?} (distance between points {})",
                nearest,
                iter_nearest,
                distance_diff
            );

            x += step;
        }

        y += step;
    }
}

fn test_far_away_points<C>(curve: &C, nearest_point: impl Fn(&C, &C::Point) -> C::Point)
where
    C: BezierCurve<Point = Coord2>,
{
    // Generate the derivative coordinates
    let start = curve.start_point();
    let end = curve.end_point();
    let (cp1, cp2) = curve.control_points();

    // Generate control vertices
    let qn1 = (cp1 - start) * 3.0;
    let qn2 = (cp2 - cp1) * 3.0;
    let qn3 = (end - cp2) * 3.0;

    // Construct a test line to compute closest points along
    let baseline = (curve.start_point(), curve.end_point());
    let offset = Coord2(100.0, 0.0);
    let test_line = (baseline.0 + offset, baseline.1 + offset);

    // Check that the iterative algorithm finds similar points to all of the points on the test line
    for t in 0..=100 {
        let t = (t as f64) / 100.0;
        let test_point = test_line.point_at_pos(t);
        let nearest = nearest_point(curve, &test_point);
        let iter_nearest_t = nearest_t_value_iteration(curve, &test_point);
        let iter_nearest = curve.point_at_pos(iter_nearest_t);

        let nearest_t = curve.nearest_t(&test_point);
        let tangent = de_casteljau3(nearest_t, qn1, qn2, qn3);

        // Log some information if there's a discrepency between the nearest point found by iteration and the nearest point found by the 'nearest point' algorithm
        if iter_nearest.distance_to(&nearest) >= 0.1 {
            // Log the t position and the distance between the curve, plus the distance between the point we found and the curve
            println!("t={:?} distance={:?} ({:?} {:?}) to_curve={:?} to_curve_iter={:?} nearest_t={:?} (should be {:?})", t, iter_nearest.distance_to(&nearest), nearest, iter_nearest, test_point.distance_to(&nearest), test_point.distance_to(&iter_nearest), nearest_t, iter_nearest_t);
            println!(
                "  t={:?} distance={:?} ({:?} {:?})",
                t - 0.01,
                iter_nearest.distance_to(&nearest_point(curve, &test_line.point_at_pos(t - 0.01))),
                nearest_point(curve, &test_line.point_at_pos(t - 0.01)),
                iter_nearest
            );
            println!(
                "  t={:?} distance={:?} ({:?} {:?})",
                t + 0.01,
                iter_nearest.distance_to(&nearest_point(curve, &test_line.point_at_pos(t + 0.01))),
                nearest_point(curve, &test_line.point_at_pos(t + 0.01)),
                iter_nearest
            );

            let tangent = tangent.to_unit_vector();
            let offset = (test_point - nearest).to_unit_vector();
            println!(
                "  tangent={:?} tangent_dot_offset={:?}",
                tangent,
                tangent.dot(&offset)
            );

            // With the exceptions of cusps, we should have found a point perpendicular to the curve
            assert!(tangent.dot(&offset).abs() < 0.001 || nearest_t <= 0.0 || nearest_t >= 1.0, "Not perpendicular (t={:?}, tangent dot offset={:?}, found distance={:?}, iteration distance={:?})", nearest_t, tangent.dot(&offset), nearest.distance_to(&test_point), iter_nearest.distance_to(&test_point));

            // The nearest point is prone to sudden discontinuities: if the iterative value and the the 'nearest' point value are of similar distances from the curve, it's likely that the problem is the iterative algorithm missed a discontinuity
            let nearest_distance = nearest.distance_to(&test_point);
            let iter_distance = iter_nearest.distance_to(&test_point);

            // Find any point as good as or better than the iteration algorithm is a pass
            assert!(nearest_distance < iter_distance + 0.001);
        }
    }
}

#[test]
fn nearest_point_on_straight_line_newton_raphson() {
    // Create a curve from a line
    let line = (Coord2(0.0, 0.0), Coord2(10.0, 7.0));
    let curve = line_to_bezier::<bezier::Curve<_>>(&line);

    let line_near = line.nearest_point(&Coord2(1.0, 5.0));
    let curve_near_t = nearest_point_on_curve_newton_raphson(&curve, &Coord2(1.0, 5.0));
    let curve_near = curve.point_at_pos(curve_near_t);

    let iterate_t = nearest_t_value_iteration(&curve, &Coord2(1.0, 5.0));
    let iterate_point = curve.point_at_pos(iterate_t);

    assert!(iterate_point.distance_to(&curve_near) < 0.1);
    assert!(line_near.distance_to(&curve_near) < 0.1);
}

#[test]
fn nearest_point_on_curve_newton_raphson_1() {
    let curve = bezier::Curve::from_points(
        Coord2(10.0, 100.0),
        (Coord2(90.0, 30.0), Coord2(40.0, 140.0)),
        Coord2(220.0, 220.0),
    );
    let point = Coord2(100.0, 130.0);

    let curve_near_t = nearest_point_on_curve_newton_raphson(&curve, &point);
    let curve_near = curve.point_at_pos(curve_near_t);

    let iterate_t = nearest_t_value_iteration(&curve, &point);
    let iterate_point = curve.point_at_pos(iterate_t);

    assert!(iterate_point.distance_to(&curve_near) < 0.1);
}

#[test]
fn nearest_point_on_curve_newton_raphson_2() {
    // Point nearest to the start of the curve
    let curve = bezier::Curve::from_points(
        Coord2(10.0, 100.0),
        (Coord2(90.0, 30.0), Coord2(40.0, 140.0)),
        Coord2(220.0, 220.0),
    );
    let point = Coord2(-10.0, 100.0);

    let curve_near_t = nearest_point_on_curve_newton_raphson(&curve, &point);
    let curve_near = curve.point_at_pos(curve_near_t);

    let iterate_t = nearest_t_value_iteration(&curve, &point);
    let iterate_point = curve.point_at_pos(iterate_t);

    assert!(iterate_point.distance_to(&curve_near) < 0.1);
}

#[test]
fn nearest_point_on_curve_newton_raphson_3() {
    // Point nearest to the end of the curve
    let curve = bezier::Curve::from_points(
        Coord2(10.0, 100.0),
        (Coord2(90.0, 30.0), Coord2(40.0, 140.0)),
        Coord2(220.0, 220.0),
    );
    let point = Coord2(240.0, 220.0);

    let curve_near_t = nearest_point_on_curve_newton_raphson(&curve, &point);
    let curve_near = curve.point_at_pos(curve_near_t);

    let iterate_t = nearest_t_value_iteration(&curve, &point);
    let iterate_point = curve.point_at_pos(iterate_t);

    assert!(iterate_point.distance_to(&curve_near) < 0.1);
}

#[test]
fn nearest_point_on_curve_newton_raphson_4() {
    let curve = bezier::Curve::from_points(
        Coord2(10.0, 100.0),
        (Coord2(90.0, 30.0), Coord2(40.0, 140.0)),
        Coord2(220.0, 220.0),
    );

    test_far_away_points(&curve, |c, p| {
        c.point_at_pos(nearest_point_on_curve_newton_raphson(c, p))
    });
}

#[test]
fn nearest_point_on_curve_newton_raphson_5() {
    let curve = bezier::Curve::from_points(
        Coord2(259.0, 322.0),
        (Coord2(272.0, 329.0), Coord2(297.0, 341.0)),
        Coord2(350.0, 397.0),
    );

    test_far_away_points(&curve, |c, p| {
        c.point_at_pos(nearest_point_on_curve_newton_raphson(c, p))
    });
}

#[test]
fn nearest_point_on_curve_newton_raphson_6() {
    let curve = bezier::Curve::from_points(
        Coord2(259.0, 322.0),
        (Coord2(272.0, 329.0), Coord2(297.0, 341.0)),
        Coord2(350.0, 397.0),
    );
    let point = Coord2(240.0, 220.0);

    let curve_near_t = nearest_point_on_curve_newton_raphson(&curve, &point);
    let curve_near = curve.point_at_pos(curve_near_t);

    let iterate_t = nearest_t_value_iteration(&curve, &point);
    let iterate_point = curve.point_at_pos(iterate_t);

    assert!(iterate_point.distance_to(&curve_near) < 0.1);
}

/* -- fails with the newton-raphson algorithm
#[test]
fn nearest_point_on_curve_newton_raphson_7() {
    let curve = bezier::Curve::from_points(Coord2(269.1, 317.7), (Coord2(280.1, 332.7), Coord2(316.4, 414.1)), Coord2(322.4, 415.0));
    let point = Coord2(296.0, 367.0);

    let curve_near_t    = nearest_point_on_curve_newton_raphson(&curve, &point);
    let curve_near      = curve.point_at_pos(curve_near_t);

    let iterate_t       = nearest_t_value_iteration(&curve, &point);
    let iterate_point   = curve.point_at_pos(iterate_t);

    assert!(iterate_point.distance_to(&curve_near) < 0.1, "Searched for: {:?}, but found: {:?} (t should be {:?} but was {:?})", iterate_point, curve_near, iterate_t, curve_near_t);
}
*/

#[test]
fn nearest_point_on_straight_line() {
    // Create a curve from a line
    let line = (Coord2(0.0, 0.0), Coord2(10.0, 7.0));
    let curve = line_to_bezier::<bezier::Curve<_>>(&line);

    let line_near = line.nearest_point(&Coord2(1.0, 5.0));
    let curve_near_t = nearest_point_on_curve(&curve, &Coord2(1.0, 5.0));
    let curve_near = curve.point_at_pos(curve_near_t);

    let iterate_t = nearest_t_value_iteration(&curve, &Coord2(1.0, 5.0));
    let iterate_point = curve.point_at_pos(iterate_t);

    assert!(
        iterate_point.distance_to(&curve_near) < 0.1,
        "Searched for: {:?}, but found: {:?} (t should be {:?} but was {:?})",
        iterate_point,
        curve_near,
        iterate_t,
        curve_near_t
    );
    assert!(line_near.distance_to(&curve_near) < 0.1);
}

#[test]
fn nearest_point_on_curve_1() {
    let curve = bezier::Curve::from_points(
        Coord2(10.0, 100.0),
        (Coord2(90.0, 30.0), Coord2(40.0, 140.0)),
        Coord2(220.0, 220.0),
    );
    let point = Coord2(100.0, 130.0);

    let curve_near_t = nearest_point_on_curve(&curve, &point);
    let curve_near = curve.point_at_pos(curve_near_t);

    let iterate_t = nearest_t_value_iteration(&curve, &point);
    let iterate_point = curve.point_at_pos(iterate_t);

    assert!(
        iterate_point.distance_to(&curve_near) < 0.1,
        "Searched for: {:?}, but found: {:?} (t should be {:?} but was {:?})",
        iterate_point,
        curve_near,
        iterate_t,
        curve_near_t
    );
}

#[test]
fn nearest_point_on_curve_2() {
    // Point nearest to the start of the curve
    let curve = bezier::Curve::from_points(
        Coord2(10.0, 100.0),
        (Coord2(90.0, 30.0), Coord2(40.0, 140.0)),
        Coord2(220.0, 220.0),
    );
    let point = Coord2(-10.0, 100.0);

    let curve_near_t = nearest_point_on_curve(&curve, &point);
    let curve_near = curve.point_at_pos(curve_near_t);

    let iterate_t = nearest_t_value_iteration(&curve, &point);
    let iterate_point = curve.point_at_pos(iterate_t);

    assert!(
        iterate_point.distance_to(&curve_near) < 0.1,
        "Searched for: {:?}, but found: {:?} (t should be {:?} but was {:?})",
        iterate_point,
        curve_near,
        iterate_t,
        curve_near_t
    );
}

#[test]
fn nearest_point_on_curve_3() {
    // Point nearest to the end of the curve
    let curve = bezier::Curve::from_points(
        Coord2(10.0, 100.0),
        (Coord2(90.0, 30.0), Coord2(40.0, 140.0)),
        Coord2(220.0, 220.0),
    );
    let point = Coord2(240.0, 220.0);

    let curve_near_t = nearest_point_on_curve(&curve, &point);
    let curve_near = curve.point_at_pos(curve_near_t);

    let iterate_t = nearest_t_value_iteration(&curve, &point);
    let iterate_point = curve.point_at_pos(iterate_t);

    assert!(
        iterate_point.distance_to(&curve_near) < 0.1,
        "Searched for: {:?}, but found: {:?} (t should be {:?} but was {:?})",
        iterate_point,
        curve_near,
        iterate_t,
        curve_near_t
    );
}

#[test]
fn nearest_point_on_curve_4() {
    let curve = bezier::Curve::from_points(
        Coord2(10.0, 100.0),
        (Coord2(90.0, 30.0), Coord2(40.0, 140.0)),
        Coord2(220.0, 220.0),
    );

    test_far_away_points(&curve, |c, p| c.nearest_point(p));
}

#[test]
fn nearest_point_on_curve_5() {
    let curve = bezier::Curve::from_points(
        Coord2(259.0, 322.0),
        (Coord2(272.0, 329.0), Coord2(297.0, 341.0)),
        Coord2(350.0, 397.0),
    );

    test_far_away_points(&curve, |c, p| c.nearest_point(p));
}

#[test]
fn nearest_point_on_curve_6() {
    let curve = bezier::Curve::from_points(
        Coord2(259.0, 322.0),
        (Coord2(272.0, 329.0), Coord2(297.0, 341.0)),
        Coord2(350.0, 397.0),
    );
    let point = Coord2(240.0, 220.0);

    let curve_near_t = nearest_point_on_curve(&curve, &point);
    let curve_near = curve.point_at_pos(curve_near_t);

    let iterate_t = nearest_t_value_iteration(&curve, &point);
    let iterate_point = curve.point_at_pos(iterate_t);

    assert!(
        iterate_point.distance_to(&curve_near) < 0.1,
        "Searched for: {:?}, but found: {:?} (t should be {:?} but was {:?})",
        iterate_point,
        curve_near,
        iterate_t,
        curve_near_t
    );
}

#[test]
fn nearest_point_on_curve_7() {
    let curve = bezier::Curve::from_points(
        Coord2(269.1, 317.7),
        (Coord2(280.1, 332.7), Coord2(316.4, 414.1)),
        Coord2(322.4, 415.0),
    );
    let point = Coord2(296.0, 367.0);

    let curve_near_t = nearest_point_on_curve(&curve, &point);
    let curve_near = curve.point_at_pos(curve_near_t);

    let iterate_t = nearest_t_value_iteration(&curve, &point);
    let iterate_point = curve.point_at_pos(iterate_t);

    assert!(
        iterate_point.distance_to(&curve_near) < 0.1,
        "Searched for: {:?}, but found: {:?} (t should be {:?} but was {:?})",
        iterate_point,
        curve_near,
        iterate_t,
        curve_near_t
    );
}

#[test]
fn nearest_point_on_curve_8() {
    let curve = bezier::Curve::from_points(
        Coord2(269.1, 317.7),
        (Coord2(280.1, 332.7), Coord2(316.4, 414.1)),
        Coord2(322.4, 415.0),
    );

    test_far_away_points(&curve, |c, p| c.nearest_point(p));
}

#[test]
fn nearest_point_on_curve_9() {
    let curve = bezier::Curve {
        start_point: Coord2(269.1, 317.7),
        end_point: Coord2(322.4, 415.0),
        control_points: (Coord2(280.1, 332.7), Coord2(316.4, 414.1)),
    };
    let point = Coord2(371.76500000000004, 322.565);

    let curve_near_t = nearest_point_on_curve(&curve, &point);
    let curve_near = curve.point_at_pos(curve_near_t);

    let iterate_t = nearest_t_value_iteration(&curve, &point);
    let iterate_point = curve.point_at_pos(iterate_t);

    assert!(
        iterate_point.distance_to(&curve_near) < 0.1,
        "Searched for: {:?}, but found: {:?} (t should be {:?} but was {:?})",
        iterate_point,
        curve_near,
        iterate_t,
        curve_near_t
    );
}

#[test]
fn nearest_point_on_curve_10() {
    let curve = Curve {
        start_point: Coord2(613.1741629120686, 691.31977047597),
        end_point: Coord2(684.1381074976954, 728.253746282104),
        control_points: (
            Coord2(666.1741629120686, 709.31977047597),
            Coord2(678.1381074976954, 703.253746282104),
        ),
    };
    let point = Coord2(644.0, 767.0);

    let curve_near_t = nearest_point_on_curve(&curve, &point);
    let curve_near = curve.point_at_pos(curve_near_t);

    let iterate_t = nearest_t_value_iteration(&curve, &point);
    let iterate_point = curve.point_at_pos(iterate_t);

    assert!(
        iterate_point.distance_to(&curve_near) < 0.1,
        "Searched for: {:?}, but found: {:?} (t should be {:?} but was {:?})",
        iterate_point,
        curve_near,
        iterate_t,
        curve_near_t
    );
}

#[test]
fn nearest_point_on_curve_11() {
    let curve = Curve {
        start_point: Coord2(613.1741629120686, 691.31977047597),
        end_point: Coord2(684.1381074976954, 728.253746282104),
        control_points: (
            Coord2(666.1741629120686, 709.31977047597),
            Coord2(678.1381074976954, 703.253746282104),
        ),
    };

    test_point_grid(&curve, 0.0..1000.0, 0.0..1000.0, 10.0);
}
