/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::super::consts::*;
use super::super::geo::*;
use super::curve::*;

use roots::{find_roots_cubic, find_roots_quadratic, Roots};
use smallvec::*;

pub(crate) const CLOSE_ENOUGH: f64 = SMALL_DISTANCE * 50.0;

///
/// Solves for t in a single dimension for a bezier curve (finds the point(s) where the basis
/// function evaluates to p)
///
pub fn solve_basis_for_t(w1: f64, w2: f64, w3: f64, w4: f64, p: f64) -> SmallVec<[f64; 4]> {
    const TINY_T: f64 = 1e-6;

    // Compute the coefficients for the cubic bezier function
    let d = w1 - p;
    let c = 3.0 * (w2 - w1);
    let b = 3.0 * (w3 - w2) - c;
    let a = w4 - w1 - c - b;

    // Solve for p
    let roots = if a.abs() < 0.00000001 {
        find_roots_quadratic(b, c, d)
    } else {
        find_roots_cubic(a, b, c, d)
    };
    let mut roots = match roots {
        Roots::No(_) => smallvec![],
        Roots::One([a]) => smallvec![a],
        Roots::Two([a, b]) => smallvec![a, b],
        Roots::Three([a, b, c]) => smallvec![a, b, c],
        Roots::Four([a, b, c, d]) => smallvec![a, b, c, d],
    };

    // Remove any roots outside the range of the function
    roots.retain(|r| *r > 0.0 && *r < 1.0);

    // Add 0.0 and 1.0 if they are an exact match
    if w1 == p {
        roots.retain(|r| *r > TINY_T);
        roots.insert(0, 0.0);
    }

    if w4 == p {
        roots.retain(|r| *r < 1.0 - TINY_T);
        roots.push(1.0);
    }

    // Return the roots
    roots
}

///
/// Searches along the x or y axis for a point within `accuracy` units of the curve, returning the `t` value of that point
///
/// This is best used for points that are known to either be on the curve or which are very close to it. There are a couple of
/// other options for finding points on a curve: `nearest_point_on_curve()` will return the true closest point on a curve rather
/// than just the closest point along a particular axis, and the ray casting function `curve_intersects_ray()` can be used to
/// search for the first point encountered along any direction instead of just searching the x or y axes.
///
/// For interactive use, `curve_intersects_ray()` might be more useful than eitehr this function or the `nearest_point_on_curve()`
/// function as the 'true' nearest point may move in an odd manner as the point it's closest to changes.
///
pub fn solve_curve_for_t_along_axis<C: BezierCurve>(
    curve: &C,
    point: &C::Point,
    accuracy: f64,
) -> Option<f64> {
    let p1 = curve.start_point();
    let (p2, p3) = curve.control_points();
    let p4 = curve.end_point();

    // Solve the basis function for each of the point's dimensions and pick the first that appears close enough (and within the range 0-1)
    for dimension in 0..(C::Point::len()) {
        // Solve for this dimension
        let (w1, w2, w3, w4) = (
            p1.get(dimension),
            p2.get(dimension),
            p3.get(dimension),
            p4.get(dimension),
        );
        let possible_t_values = solve_basis_for_t(w1, w2, w3, w4, point.get(dimension));

        for possible_t in possible_t_values {
            // Ignore values outside the range of the curve
            if !(-0.001..=1.001).contains(&possible_t) {
                continue;
            }

            // If this is an accurate enough solution, return this as the t value
            let point_at_t = curve.point_at_pos(possible_t);
            if point_at_t.is_near_to(point, accuracy) {
                return Some(possible_t);
            }
        }
    }

    // No solution: result is None
    None
}
