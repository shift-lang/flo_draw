/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use flo_curves::line::*;

#[test]
fn points_on_line_are_on_line_1() {
    let line = (Coord2(2.0, 3.0), Coord2(7.0, 6.0));
    let (a, b, c) = line_coefficients_2d(&line).into();

    for t in 0..=16 {
        let t = (t as f64) / 16.0;
        let point = line.point_at_pos(t);

        assert!((a * point.x() + b * point.y() + c).abs() < 0.001);
    }
}

#[test]
fn points_on_line_are_on_line_2() {
    let line = (Coord2(7.0, 6.0), Coord2(2.0, 3.0));
    let (a, b, c) = line_coefficients_2d(&line).into();

    for t in 0..=16 {
        let t = (t as f64) / 16.0;
        let point = line.point_at_pos(t);

        assert!((a * point.x() + b * point.y() + c).abs() < 0.001);
    }
}

#[test]
fn points_on_line_are_on_line_3() {
    let line = (Coord2(2.0, 3.0), Coord2(7.0, 3.0));
    let (a, b, c) = line_coefficients_2d(&line).into();

    for t in 0..=16 {
        let t = (t as f64) / 16.0;
        let point = line.point_at_pos(t);

        assert!((a * point.x() + b * point.y() + c).abs() < 0.001);
    }
}

#[test]
fn points_on_line_are_on_line_4() {
    let line = (Coord2(2.0, 3.0), Coord2(2.0, 6.0));
    let (a, b, c) = line_coefficients_2d(&line).into();

    for t in 0..=16 {
        let t = (t as f64) / 16.0;
        let point = line.point_at_pos(t);

        assert!((a * point.x() + b * point.y() + c).abs() < 0.001);
    }
}

#[test]
fn points_on_line_are_on_line_5() {
    let line = (Coord2(2.0, 3.0), Coord2(2.0, 6.0));
    let (a, b, c) = line.coefficients().into();

    for t in 0..=16 {
        let t = (t as f64) / 16.0;
        let point = line.point_at_pos(t);

        assert!((a * point.x() + b * point.y() + c).abs() < 0.001);
    }
}

#[test]
fn distance_from_horizontal_line() {
    let line = (Coord2(2.0, 3.0), Coord2(8.0, 3.0));

    assert!((line.distance_to(&Coord2(4.0, 3.0))).abs() < 0.001);
    assert!((line.distance_to(&Coord2(5.0, 4.0)) - 1.0).abs() < 0.001);
    assert!((line.distance_to(&Coord2(3.0, 0.0)) - -3.0).abs() < 0.001);
}

#[test]
fn distance_from_vertical_line() {
    let line = (Coord2(2.0, 3.0), Coord2(2.0, 9.0));

    assert!((line.distance_to(&Coord2(2.0, 5.0))).abs() < 0.001);
    assert!((line.distance_to(&Coord2(3.0, 4.0)) - 1.0).abs() < 0.001);
    assert!((line.distance_to(&Coord2(0.0, 0.0)) - -2.0).abs() < 0.001);
}

#[test]
fn distance_from_diagonal_line() {
    let line = (Coord2(2.0, 3.0), Coord2(5.0, 9.0));

    assert!((line.distance_to(&Coord2(3.5, 6.0))).abs() < 0.001);
    assert!((line.distance_to(&Coord2(3.0, 4.0)) - 0.4472).abs() < 0.001);
}

#[test]
fn pos_for_point_horizontal() {
    let line = (Coord2(2.0, 3.0), Coord2(6.0, 3.0));
    assert!((line.pos_for_point(&Coord2(4.0, 3.0)) - 0.5).abs() < 0.001);
}

#[test]
fn pos_for_point_vertical() {
    let line = (Coord2(3.0, 2.0), Coord2(3.0, 6.0));
    assert!((line.pos_for_point(&Coord2(3.0, 4.0)) - 0.5).abs() < 0.001);
}

#[test]
fn x_for_y_at_start() {
    let line = (Coord2(2.0, 3.0), Coord2(7.0, 6.0));
    let coeff = line.coefficients();

    let x_at_3 = coeff.x_for_y(3.0);
    assert!((x_at_3 - 2.0).abs() < 0.001);
}

#[test]
fn x_for_y_at_end() {
    let line = (Coord2(2.0, 3.0), Coord2(7.0, 6.0));
    let coeff = line.coefficients();

    let x_at_6 = coeff.x_for_y(6.0);
    assert!((x_at_6 - 7.0).abs() < 0.001);
}

#[test]
fn y_for_x_at_start() {
    let line = (Coord2(2.0, 3.0), Coord2(7.0, 6.0));
    let coeff = line.coefficients();

    let y_at_2 = coeff.y_for_x(2.0);
    assert!((y_at_2 - 3.0).abs() < 0.001);
}

#[test]
fn y_for_x_at_end() {
    let line = (Coord2(2.0, 3.0), Coord2(7.0, 6.0));
    let coeff = line.coefficients();

    let y_at_7 = coeff.y_for_x(7.0);
    assert!((y_at_7 - 6.0).abs() < 0.001);
}

#[test]
fn x_for_y_along_line() {
    let line = (Coord2(2.0, 3.0), Coord2(7.0, 6.0));
    let coeff = line.coefficients();

    for t in 0..=16 {
        let t = (t as f64) / 16.0;
        let point = line.point_at_pos(t);
        let point_x = coeff.x_for_y(point.y());

        assert!((point_x - point.x()).abs() < 0.001);
    }
}

#[test]
fn y_for_x_along_line() {
    let line = (Coord2(2.0, 3.0), Coord2(7.0, 6.0));
    let coeff = line.coefficients();

    for t in 0..=16 {
        let t = (t as f64) / 16.0;
        let point = line.point_at_pos(t);
        let point_y = coeff.y_for_x(point.x());

        assert!((point_y - point.y()).abs() < 0.001);
    }
}

#[test]
fn perpendicular_line() {
    let line = (Coord2(2.0, 3.0), Coord2(7.0, 6.0));
    let coeff = line.coefficients();
    let coeff_perpendicular =
        coeff.to_perpendicular_line(&(Coord2(2.0 - (6.0 - 3.0), 3.0 - (7.0 - 2.0))));

    assert!(coeff.distance_to(&Coord2(2.0, 3.0)) < 0.001);
    assert!(coeff.distance_to(&Coord2(7.0, 6.0)) < 0.001);

    assert!(coeff_perpendicular.distance_to(&Coord2(2.0, 3.0)) < 0.001);
    assert!(coeff_perpendicular.distance_to(&Coord2(2.0 - (6.0 - 3.0), 3.0 - (7.0 - 2.0))) < 0.001);
}
