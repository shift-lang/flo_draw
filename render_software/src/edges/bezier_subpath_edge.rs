use super::polyline_edge::*;
use super::flattened_bezier_subpath_edge::*;
use crate::edgeplan::*;

use flo_canvas as canvas;
use flo_canvas::curves::bezier::path::*;
use flo_canvas::curves::geo::*;
use flo_canvas::curves::bezier::*;

use smallvec::*;

use std::ops::{Range};
use std::sync::*;
use std::vec;

// These values are good for 4k rendering when flattening curves
const DETAIL: f64 = 2.0 / 4000.0;
const FLATNESS: f64 = 2.0 / 4000.0;

///
/// Bezier subpath that uses the 'non-zero' algorithm to decide whether a point is inside or outside the shape
///
#[derive(Clone)]
pub struct BezierSubpathNonZeroEdge {
    /// The ID of the shape that's inside this subpath
    shape_id: ShapeId,

    /// The subpath definition
    subpath: BezierSubpath,
}

///
/// Bezier subpath that uses the 'even-odd' algorithm to decide whether a point is inside or outside the shape
///
#[derive(Clone)]
pub struct BezierSubpathEvenOddEdge {
    /// The ID of the shape that's inside this subpath
    shape_id: ShapeId,

    /// The subpath definition
    subpath: BezierSubpath,
}

///
/// Represents a closed bezier subpath
///
/// To become an edge, this needs to be combined with a winding rule style and a 
///
#[derive(Clone)]
pub struct BezierSubpath {
    /// The curves within this subpath
    curves: Vec<SubpathCurve>,

    /// Lookup table for finding which curves are where (or None if this has not been calculated yet)
    space: Option<Space1D<usize>>,

    /// The bounding box (x coordinates)
    x_bounds: Range<f64>,

    /// The bounding box (y coordinates)
    y_bounds: Range<f64>,
}

#[derive(Clone, Debug)]
struct SubpathCurve {
    /// The y bounding box for this curve
    y_bounds: Range<f64>,

    /// x control points (w1, w2, w3, w4)
    wx: (f64, f64, f64, f64),

    /// y control points (w1, w2, w3, w4)
    wy: (f64, f64, f64, f64),

    /// The y-derivative control points (w1, w2, w3)
    wdy: (f64, f64, f64),
}

///
/// An intercept on a bezier subpath
///
#[derive(Clone, Copy, Debug)]
pub struct BezierSubpathIntercept {
    /// The x position of this intercept
    pub x_pos: f64,

    /// The curve that the intercept belongs to
    pub curve_idx: usize,

    /// The t-value of this intercept
    pub t: f64,
}

impl Geo for BezierSubpath {
    type Point = Coord2;
}

impl SubpathCurve {
    /// Converts this to a 'normal' curve
    fn as_curve(&self) -> Curve<Coord2> {
        Curve::from_points(Coord2(self.wx.0, self.wy.0), (Coord2(self.wx.1, self.wy.1), Coord2(self.wx.2, self.wy.2)), Coord2(self.wx.3, self.wy.3))
    }

    ///
    /// Returns a transformed version of this edge descriptor
    ///
    fn transform(&self, transform: &canvas::Transform2D) -> SubpathCurve {
        // Transform the points in this curve
        let (wx, wy) = (&self.wx, &self.wy);

        let w1 = transform.transform_point(wx.0 as _, wy.0 as _);
        let w2 = transform.transform_point(wx.1 as _, wy.1 as _);
        let w3 = transform.transform_point(wx.2 as _, wy.2 as _);
        let w4 = transform.transform_point(wx.3 as _, wy.3 as _);

        let w1 = (w1.0 as f64, w1.1 as f64);
        let w2 = (w2.0 as f64, w2.1 as f64);
        let w3 = (w3.0 as f64, w3.1 as f64);
        let w4 = (w4.0 as f64, w4.1 as f64);

        // Recalculate the bounds
        let y_bounds = bounding_box4::<_, Bounds<f64>>(w1.1, w2.1, w3.1, w4.1);

        // Calculate the derivative
        let wdy = derivative4(w1.1, w2.1, w3.1, w4.1);

        SubpathCurve {
            y_bounds: y_bounds.min()..y_bounds.max(),
            wx: (w1.0, w2.0, w3.0, w4.0),
            wy: (w1.1, w2.1, w3.1, w4.1),
            wdy: wdy,
        }
    }
}

impl BezierPath for BezierSubpath {
    type PointIter = vec::IntoIter<(Coord2, Coord2, Coord2)>;

    #[inline]
    fn start_point(&self) -> Self::Point {
        Coord2(self.curves[0].wx.0, self.curves[0].wy.0)
    }

    fn points(&self) -> Self::PointIter {
        self.curves.iter()
            .map(|curve| (Coord2(curve.wx.1, curve.wy.1), Coord2(curve.wx.2, curve.wy.2), Coord2(curve.wx.3, curve.wy.3)))
            .collect::<Vec<_>>()
            .into_iter()
    }
}

///
/// A bezier subpath can be used as the target of a bezier path factory
///
impl BezierPathFactory for BezierSubpath {
    fn from_points<FromIter: IntoIterator<Item=(Coord2, Coord2, Coord2)>>(start_point: Coord2, points: FromIter) -> Self {
        // This should be much smaller than a pixel: we exclude very short curves whose control polygon is smaller than this
        const MIN_DISTANCE: f64 = 1e-6;

        let mut curves = vec![];
        let mut last_point = start_point;

        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        for (cp1, cp2, end_point) in points {
            if last_point.is_near_to(&end_point, MIN_DISTANCE) && control_polygon_length(&Curve::from_points(last_point, (cp1, cp2), end_point)) <= MIN_DISTANCE {
                // This curve is very short, so we exclude it from the path
                continue;
            }

            // Fetch the w values, and calculate the derivative and bounding box
            let wx = (last_point.x(), cp1.x(), cp2.x(), end_point.x());
            let wy = (last_point.y(), cp1.y(), cp2.y(), end_point.y());
            let wdy = derivative4(wy.0, wy.1, wy.2, wy.3);
            let x_bounds = bounding_box4::<_, Bounds<f64>>(wy.0, wy.1, wy.2, wy.3);
            let y_bounds = bounding_box4::<_, Bounds<f64>>(wy.0, wy.1, wy.2, wy.3);

            // Update the min, max coordinates
            min_x = min_x.min(x_bounds.min());
            min_y = min_y.min(y_bounds.min());
            max_x = max_x.max(x_bounds.max());
            max_y = max_y.max(y_bounds.max());

            // Add a new curve
            curves.push(SubpathCurve {
                y_bounds: y_bounds.min()..y_bounds.max(),
                wx: wx,
                wy: wy,
                wdy: wdy,
            });

            // Update the last point to match the end point of the previous curve section
            last_point = end_point;
        }

        // Move the last point if it's 'close enough' (probably we removed a final short curve rather than the path being left open)
        if curves.len() > 0 && start_point != last_point && last_point.is_near_to(&start_point, MIN_DISTANCE) {
            let last_curve = curves.last_mut().unwrap();

            last_curve.wx.3 = start_point.x();
            last_curve.wy.3 = start_point.y();

            last_curve.wdy = derivative4(last_curve.wy.0, last_curve.wy.1, last_curve.wy.2, last_curve.wy.3);
        } else {
            // If a subpath isn't closed, then rays might 'escape'
            debug_assert!(start_point == last_point, "Bezier subpaths must be closed ({}, {} != {}, {})", start_point.x(), start_point.y(), last_point.x(), last_point.y());
        }

        if curves.len() == 0 {
            panic!("Bezier subpaths must have at least one curve in them");
        }

        BezierSubpath {
            curves: curves,
            space: None,
            x_bounds: min_x..max_x,
            y_bounds: min_y..max_y,
        }
    }
}

impl BezierSubpath {
    ///
    /// Fills in the 'space' structure in preparation to retrieve intercepts using `intercepts_on_line()`
    ///
    #[inline]
    pub fn prepare_to_render(&mut self) {
        let space = Space1D::from_data(self.curves.iter().enumerate()
            .map(|(idx, curve)| (curve.y_bounds.clone(), idx)));
        self.space = Some(space);
    }

    ///
    /// True if two curve indexes indicates that two curves are joined together
    ///
    #[inline]
    fn curves_are_neighbors(&self, idx1: usize, idx2: usize) -> bool {
        if idx1 == 0 && idx2 == self.curves.len() - 1 {
            true
        } else if idx2 == 0 && idx1 == self.curves.len() - 1 {
            true
        } else {
            ((idx1 as isize) - (idx2 as isize)).abs() == 1
        }
    }

    ///
    /// Finds the intercepts on a line of this subpath
    ///
    /// `prepare_to_render()` must be called before this can be used
    ///
    pub fn intercepts_on_line(&self, y_pos: f64) -> impl Iterator<Item=BezierSubpathIntercept> {
        // How close two intercepts have to be to invoke the 'double intercept' algorithm. This really depends on the precision of `solve_basis_for_t'
        const VERY_CLOSE_X: f64 = 1e-6;

        // How short the control polygon needs to be between two points to consider them as the same
        const MIN_CONTROL_POLYGON_LENGTH: f64 = 1e-6;

        // Compute the raw intercepts. These can have double intercepts where two curves meet
        let mut intercepts = if self.y_bounds.contains(&y_pos) {
            self.space
                .as_ref()
                .unwrap()
                .data_at_point(y_pos)
                .map(|idx| (*idx, &self.curves[*idx]))
                .flat_map(|(idx, curve)| solve_basis_for_t(curve.wy.0, curve.wy.1, curve.wy.2, curve.wy.3, y_pos).into_iter()
                    .filter(|t| *t >= 0.0 && *t <= 1.0)
                    .map(move |t| BezierSubpathIntercept { x_pos: de_casteljau4(t, curve.wx.0, curve.wx.1, curve.wx.2, curve.wx.3), curve_idx: idx, t: t }))
                .collect::<SmallVec<[_; 4]>>()
        } else {
            smallvec![]
        };

        // Sort the intercepts by x position
        intercepts.sort_unstable_by(|a, b| a.x_pos.total_cmp(&b.x_pos));

        if intercepts.len() > 1 {
            // Detect double intercepts
            // We use numerical methods to solve the intercept points, which is combined with the inherent imprecision of floating point numbers, so double intercepts will
            // not always appear at the same place. So the approach is this: if two intercepts have very close x values, are for the end and start of neighboring curves, and
            // are in the same direction, then count that intercept as just one. It's probably possible to fool this algorithm with a suitably constructed self-intersection shape.
            let mut intercept_idx = 0;
            while intercept_idx < intercepts.len() - 1 {
                // Fetch the two intercepts that we want to check for doubling up
                let mut overlap_idx = intercept_idx + 1;

                while overlap_idx < intercepts.len() && (intercepts[intercept_idx].x_pos - intercepts[overlap_idx].x_pos).abs() <= VERY_CLOSE_X {
                    let prev = &intercepts[intercept_idx];
                    let next = &intercepts[overlap_idx];

                    if self.curves_are_neighbors(prev.curve_idx, next.curve_idx) {
                        // Two points are very close together
                        let prev_curve = &self.curves[prev.curve_idx];
                        let next_curve = &self.curves[next.curve_idx];

                        let prev_tangent_y = de_casteljau3(prev.t, prev_curve.wdy.0, prev_curve.wdy.1, prev_curve.wdy.2);
                        let prev_normal_x = -prev_tangent_y;
                        let prev_side = prev_normal_x.signum();

                        let next_tangent_y = de_casteljau3(next.t, next_curve.wdy.0, next_curve.wdy.1, next_curve.wdy.2);
                        let next_normal_x = -next_tangent_y;
                        let next_side = next_normal_x.signum();

                        // Remove one of the intercepts if these two very close points are crossing the subpath in the same direction
                        if prev_side == next_side {
                            // Two intercepts are on the same side of the curve, on subsequent sections: they are (very probably) the same if the 'control polygon' distance between them is small enough
                            if prev.curve_idx < next.curve_idx {
                                let prev_as_curve = prev_curve.as_curve();
                                let next_as_curve = next_curve.as_curve();
                                let prev_section = prev_as_curve.section(prev.t, 1.0);
                                let next_section = next_as_curve.section(0.0, next.t);
                                let length = control_polygon_length(&prev_section) + control_polygon_length(&next_section);

                                if length < MIN_CONTROL_POLYGON_LENGTH || (prev.t >= 1.0 && next.t <= 0.0) {
                                    // Points are very close in terms of curve arc length
                                    intercepts.remove(overlap_idx);
                                } else {
                                    overlap_idx += 1;
                                }
                            } else {
                                let prev_as_curve = prev_curve.as_curve();
                                let next_as_curve = next_curve.as_curve();
                                let prev_section = prev_as_curve.section(0.0, prev.t);
                                let next_section = next_as_curve.section(next.t, 1.0);
                                let length = control_polygon_length(&prev_section) + control_polygon_length(&next_section);

                                if length < MIN_CONTROL_POLYGON_LENGTH || (prev.t <= 0.0 && next.t >= 1.0) {
                                    // Points are very close in terms of curve arc length
                                    intercepts.remove(overlap_idx);
                                } else {
                                    overlap_idx += 1;
                                }
                            }
                        } else {
                            overlap_idx += 1;
                        }
                    } else {
                        // Only test neighboring edges
                        overlap_idx += 1;
                    }
                }

                // Try the next intercept
                intercept_idx += 1;
            }
        }

        debug_assert!(intercepts.len() % 2 == 0, "\n\nIntercepts should be even, but found {} intercepts - {:?} - on line {:?} for path:\n'{}'\n\n", intercepts.len(), intercepts, y_pos, flo_canvas::curves::debug::bezier_path_to_rust_definition(self));

        // Iterate over the results
        intercepts.into_iter()
    }

    ///
    /// Returns a transformed version of this subpath
    ///
    pub fn transform(&self, transform: &canvas::Transform2D) -> BezierSubpath {
        // Transform the curves
        let curves = self.curves.iter().map(|curve| curve.transform(&transform)).collect::<Vec<_>>();

        // Calculate the bounding box
        let x_bounds = curves.iter()
            .fold(f64::MAX..f64::MIN, |x_bounds, curve| {
                let curve_x_bounds = bounding_box4::<_, Bounds<f64>>(curve.wx.0, curve.wx.1, curve.wx.2, curve.wx.3);

                x_bounds.start.min(curve_x_bounds.min())..x_bounds.end.max(curve_x_bounds.max())
            });
        let y_bounds = curves.iter()
            .fold(f64::MAX..f64::MIN, |y_bounds, curve| {
                y_bounds.start.min(curve.y_bounds.start)..y_bounds.end.max(curve.y_bounds.end)
            });

        BezierSubpath {
            curves: curves,
            space: None,
            x_bounds: x_bounds,
            y_bounds: y_bounds,
        }
    }

    ///
    /// Creates a non-zero edge from this subpath
    ///
    pub fn to_non_zero_edge(self, shape_id: ShapeId) -> BezierSubpathNonZeroEdge {
        BezierSubpathNonZeroEdge {
            shape_id: shape_id,
            subpath: self,
        }
    }

    ///
    /// Creates a non-zero edge from this subpath
    ///
    pub fn to_even_odd_edge(self, shape_id: ShapeId) -> BezierSubpathEvenOddEdge {
        BezierSubpathEvenOddEdge {
            shape_id: shape_id,
            subpath: self,
        }
    }

    ///
    /// Creates a non-zero edge from this subpath, which will be flattened to a polyline before rendering
    ///
    pub fn to_flattened_non_zero_edge(self, shape_id: ShapeId) -> FlattenedBezierNonZeroEdge {
        FlattenedBezierNonZeroEdge {
            shape_id: shape_id,
            path: FlattenedBezierSubpath::from_subpath(self, DETAIL, FLATNESS),
        }
    }

    ///
    /// Creates a non-zero edge from this subpath, which will be flattened to a polyline before rendering
    ///
    pub fn to_flattened_even_odd_edge(self, shape_id: ShapeId) -> FlattenedBezierEvenOddEdge {
        FlattenedBezierEvenOddEdge {
            shape_id: shape_id,
            path: FlattenedBezierSubpath::from_subpath(self, DETAIL, FLATNESS),
        }
    }

    ///
    /// Creates a polyline from this path
    ///
    pub fn flatten_to_polyline(self, min_length: f64, flatness: f64) -> Polyline {
        use std::iter;

        // TODO: this just creates the most basic polygon possible
        let start_point = Coord2(self.curves[0].wx.0, self.curves[0].wy.0);
        Polyline::new(iter::once(start_point)
            .chain(self.curves.into_iter()
                .flat_map(|curve| flatten_curve(&curve, min_length, flatness))))
    }
}

///
/// Flattens a curve into a set of points for a polyline
///
fn flatten_curve(curve: &SubpathCurve, min_length: f64, flatness: f64) -> Vec<Coord2> {
    // Create a curve from the subpath curve
    let curve = Curve::from_points(
        Coord2(curve.wx.0, curve.wy.0),
        (Coord2(curve.wx.1, curve.wy.1), Coord2(curve.wx.2, curve.wy.2)),
        Coord2(curve.wx.3, curve.wy.3),
    );

    // Process curve sections by subdividing until they are small enough
    let mut to_process = vec![];
    let mut result = Vec::with_capacity(4);
    to_process.push(curve.section(0.0, 1.0));

    while let Some(section) = to_process.pop() {
        let sp = section.start_point();
        let ep = section.end_point();

        if section.flatness() < flatness || (sp.is_near_to(&ep, min_length) && sp.is_near_to(&section.point_at_pos(0.5), min_length)) {
            // Section is either very short or flat so can be added to the result
            result.push(section.end_point());
        } else {
            // Subdivide and try again
            let lhs = section.subsection(0.0, 0.5);
            let rhs = section.subsection(0.5, 1.0);

            // Process the lhs first so the points are generated in order
            to_process.push(rhs);
            to_process.push(lhs);
        }
    }

    result
}

impl EdgeDescriptor for BezierSubpathEvenOddEdge {
    fn clone_as_object(&self) -> Arc<dyn EdgeDescriptor> {
        Arc::new(self.clone())
    }

    fn prepare_to_render(&mut self) {
        self.subpath.prepare_to_render();
    }

    #[inline]
    fn shape(&self) -> ShapeId { self.shape_id }

    #[inline]
    fn bounding_box(&self) -> ((f64, f64), (f64, f64)) {
        ((self.subpath.x_bounds.start, self.subpath.y_bounds.start), (self.subpath.x_bounds.end, self.subpath.y_bounds.end))
    }

    fn transform(&self, transform: &canvas::Transform2D) -> Arc<dyn EdgeDescriptor> {
        let mut subpath = self.subpath.transform(transform);
        subpath.prepare_to_render();

        let new_edge = Self {
            shape_id: self.shape_id,
            subpath: subpath,
        };

        Arc::new(new_edge)
    }

    #[inline]
    fn intercepts(&self, y_positions: &[f64], output: &mut [SmallVec<[EdgeDescriptorIntercept; 2]>]) {
        let mut y_pos_iter = y_positions.iter();
        let mut output_iter = output.iter_mut();

        while let (Some(y_pos), Some(output)) = (y_pos_iter.next(), output_iter.next()) {
            let intercepts = self.subpath.intercepts_on_line(*y_pos);

            if self.subpath.y_bounds.contains(y_pos) {
                output.extend(intercepts.into_iter()
                    .map(|intercept| EdgeDescriptorIntercept { direction: EdgeInterceptDirection::Toggle, x_pos: intercept.x_pos, position: EdgePosition(0, intercept.curve_idx, intercept.t) }));
            }
        }
    }

    fn description(&self) -> String {
        format!("Even-odd bezier edge: {:?}", self.subpath.curves)
    }
}

impl BezierSubpathEvenOddEdge {
    pub fn transform_as_self(&self, transform: &canvas::Transform2D) -> Self {
        let mut subpath = self.subpath.transform(transform);
        subpath.prepare_to_render();

        let new_edge = Self {
            shape_id: self.shape_id,
            subpath: subpath,
        };

        new_edge
    }
}

impl BezierSubpathNonZeroEdge {
    pub fn transform_as_self(&self, transform: &canvas::Transform2D) -> Self {
        let mut subpath = self.subpath.transform(transform);
        subpath.prepare_to_render();

        let new_edge = Self {
            shape_id: self.shape_id,
            subpath: subpath,
        };

        new_edge
    }
}

impl EdgeDescriptor for BezierSubpathNonZeroEdge {
    fn clone_as_object(&self) -> Arc<dyn EdgeDescriptor> {
        Arc::new(self.clone())
    }

    fn prepare_to_render(&mut self) {
        self.subpath.prepare_to_render();
    }

    #[inline]
    fn shape(&self) -> ShapeId { self.shape_id }

    #[inline]
    fn bounding_box(&self) -> ((f64, f64), (f64, f64)) {
        ((self.subpath.x_bounds.start, self.subpath.y_bounds.start), (self.subpath.x_bounds.end, self.subpath.y_bounds.end))
    }

    fn transform(&self, transform: &canvas::Transform2D) -> Arc<dyn EdgeDescriptor> {
        let mut subpath = self.subpath.transform(transform);
        subpath.prepare_to_render();

        let new_edge = Self {
            shape_id: self.shape_id,
            subpath: subpath,
        };

        Arc::new(new_edge)
    }

    #[inline]
    fn intercepts(&self, y_positions: &[f64], output: &mut [SmallVec<[EdgeDescriptorIntercept; 2]>]) {
        let mut y_pos_iter = y_positions.iter();
        let mut output_iter = output.iter_mut();

        while let (Some(y_pos), Some(output)) = (y_pos_iter.next(), output_iter.next()) {
            let intercepts = self.subpath.intercepts_on_line(*y_pos);

            if self.subpath.y_bounds.contains(y_pos) {
                *output = intercepts.into_iter()
                    .map(|intercept| {
                        // Compute the direction that the ray is crossing the curve
                        let t = intercept.t;
                        let (d1, d2, d3) = self.subpath.curves[intercept.curve_idx].wdy;

                        let tangent_y = de_casteljau3(t, d1, d2, d3);
                        let normal_x = -tangent_y;
                        let side = normal_x.signum();

                        // The basic approach to the normal is to get the dot product like this, but we precalculate just what we need
                        //let normal  = self.curve.normal_at_pos(t);
                        //let side    = (normal.x() * 1.0 + normal.y() * 0.0).signum();  // Dot product with the 'ray' direction of the scanline

                        if side <= 0.0 {
                            EdgeDescriptorIntercept { direction: EdgeInterceptDirection::DirectionOut, x_pos: intercept.x_pos, position: EdgePosition(0, intercept.curve_idx, intercept.t) }
                        } else {
                            EdgeDescriptorIntercept { direction: EdgeInterceptDirection::DirectionIn, x_pos: intercept.x_pos, position: EdgePosition(0, intercept.curve_idx, intercept.t) }
                        }
                    }).collect();
            } else {
                *output = smallvec![];
            }
        }
    }

    fn description(&self) -> String {
        format!("Non-zero bezier edge: {:?}", self.subpath.curves)
    }
}
