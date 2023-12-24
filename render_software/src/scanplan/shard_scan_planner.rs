use super::scanline_shard_intercept::*;
use super::scanline_transform::*;
use super::scanline_plan::*;
use super::scan_planner::*;

use crate::edgeplan::*;
use crate::pixel::*;

use std::marker::{PhantomData};
use std::ops::{Range};
use std::sync::*;

///
/// The shard scan planner uses edge 'shards' to partially cover pixels, acheiving a fairly fast anti-aliasing effect
///
pub struct ShardScanPlanner<TEdge> {
    edge: PhantomData<Mutex<TEdge>>,
}

impl<TEdge> ShardScanPlanner<TEdge>
    where
        TEdge: EdgeDescriptor,
{
    ///
    /// Plans out a scanline using the ShardScanPlanner (this scan planner does not perform any anti-aliasing)
    ///
    #[inline]
    pub fn plan(edge_plan: &EdgePlan<TEdge>, transform: &ScanlineTransform, y_positions: &[f64], x_range: Range<f64>) -> Vec<(f64, ScanlinePlan)> {
        // Create a planner and the result vec
        let planner = Self::default();
        let mut scanlines = vec![(0.0, ScanlinePlan::default()); y_positions.len()];

        // Fill with scanlines
        planner.plan_scanlines(edge_plan, transform, y_positions, x_range, &mut scanlines);

        scanlines
    }
}

///
/// Represents an intercept against a shard. Every shard produces two intercepts: one where they start to fade in or out, and one where
/// they finish fading in or out.
///
#[derive(Clone, Copy, Debug)]
enum ShardIntercept {
    /// Start fading in the effects of an intercept
    Start(ShardInterceptLocation),

    /// Finish fading in the effects of an intercept
    Finish(ShardInterceptLocation),
}

struct ShardInterceptIterator<'a, TShardIterator>
    where
        TShardIterator: Iterator<Item=&'a EdgePlanShardIntercept>,
{
    /// The shards that remain in the iterator, set to None once the iterator is completed
    remaining_shards: Option<TShardIterator>,

    /// The next shard to start
    next_shard: Option<ShardInterceptLocation>,

    /// The shards that have been started: a stack, with the first to end at the top
    started_shards: Vec<ShardInterceptLocation>,

    /// The transform being used for the current scanline
    transform: &'a ScanlineTransform,
}

impl<'a, TShardIterator> ShardInterceptIterator<'a, TShardIterator>
    where
        TShardIterator: Iterator<Item=&'a EdgePlanShardIntercept>,
{
    ///
    /// Creates a new shard intercept iterator
    ///
    #[inline]
    pub fn from_intercepts(intercepts: TShardIterator, transform: &'a ScanlineTransform) -> Self {
        Self {
            remaining_shards: Some(intercepts),
            next_shard: None,
            started_shards: vec![],
            transform: transform,
        }
    }
}

impl<'a, TShardIterator> Iterator for ShardInterceptIterator<'a, TShardIterator>
    where
        TShardIterator: Iterator<Item=&'a EdgePlanShardIntercept>,
{
    type Item = ShardIntercept;

    #[inline]
    fn next(&mut self) -> Option<ShardIntercept> {
        // Retrieve/fill in the next shard
        let next_shard = if let Some(next_shard) = self.next_shard.take() {
            Some(next_shard)
        } else if let Some(remaining) = self.remaining_shards.as_mut() {
            let result = remaining.next().map(|intercept| ShardInterceptLocation::from(intercept, self.transform));

            if result.is_none() {
                self.remaining_shards = None;
            }

            result
        } else {
            None
        };

        if let Some(next_shard) = next_shard {
            // If there's a shard finishing before the next shard, return that one
            if let Some(started) = self.started_shards.pop() {
                if started.upper_x_ceil <= next_shard.lower_x {
                    // This shard is finishing before this new one starts
                    self.next_shard = Some(next_shard);

                    return Some(ShardIntercept::Finish(started));
                } else {
                    // Leave to process for later
                    self.started_shards.push(started);
                }
            }

            // Starting this shard: add to the list of started shards. The top of the list needs to be the shard that ends next
            let mut found_place = false;
            let next_upper_x = next_shard.upper_x_ceil;

            for idx in (0..self.started_shards.len()).rev() {
                if self.started_shards[idx].upper_x_ceil > next_upper_x {
                    self.started_shards.insert(idx + 1, next_shard);

                    found_place = true;
                    break;
                }
            }

            if !found_place {
                self.started_shards.insert(0, next_shard);
            }

            // Result is that this shard is starting
            Some(ShardIntercept::Start(next_shard))
        } else if let Some(started) = self.started_shards.pop() {
            // Finish this shard
            Some(ShardIntercept::Finish(started))
        } else {
            // No more shards remain
            None
        }
    }
}

impl ShardIntercept {
    ///
    /// Returns the intercept this is for
    ///
    #[inline]
    pub fn intercept(&self) -> &ShardInterceptLocation {
        match self {
            ShardIntercept::Start(intercept) => intercept,
            ShardIntercept::Finish(intercept) => intercept,
        }
    }

    ///
    /// Returns the shape ID that this intercept is against
    ///
    #[inline]
    pub fn shape(&self) -> ShapeId {
        self.intercept().shape
    }

    ///
    /// Returns the x position of this intercept
    ///
    #[inline]
    pub fn x_pos(&self) -> f64 {
        match self {
            ShardIntercept::Start(intercept) => intercept.lower_x_floor,
            ShardIntercept::Finish(intercept) => intercept.upper_x_ceil,
        }
    }
}

impl<TEdge> Default for ShardScanPlanner<TEdge>
    where
        TEdge: EdgeDescriptor,
{
    #[inline]
    fn default() -> Self {
        ShardScanPlanner { edge: PhantomData }
    }
}

///
/// Returns the coverage for a pixel. The two values passed in are the alpha value at the start of the
/// pixel (ie, where x=0) and the end (x=1).
///
fn alpha_coverage(pixel_start: f64, pixel_end: f64) -> f64 {
    // ax = 0, bx = 1
    let ay = pixel_start.min(pixel_end);
    let by = pixel_end.max(pixel_start);

    if ay <= 0.0 && by <= 0.0 {
        // Both outside
        0.0
    } else if ay >= 1.0 && by >= 1.0 {
        // Whole pixel covered
        1.0
    } else if ay < 0.0 {
        // Calculate intercept on the x-axis (y = 0)
        let cx = -ay / (by - ay);

        if by < 1.0 {
            // Crosses the x-axis then the y-axis
            0.5 * (1.0 - cx) * by
        } else {
            // Crosses the x-axis twice

            // Calculate intercept on x-axis for y=1
            let dx = (1.0 - ay) / (by - ay);

            0.5 * (dx - cx) + (1.0 - dx)
        }
    } else {
        // LHS intercepts the y-axis
        if by < 1.0 {
            // Both sides intercept the y-axis
            0.5 * (by - ay) + ay
        } else {
            // Calculate intercept on x-axis for y=1
            let dx = (1.0 - ay) / (by - ay);

            1.0 - (0.5 * dx * (1.0 - ay))
        }
    }
}

///
/// Calculates the range of alpha values to use in a rendering program.
///
/// The `render_x_range` is the range of values where the pixels will be rendered. The alpha_x_range is the full range of the fade,
/// and the alpha_range is the range of alpha values that will be covered.
///
/// This will the alpha values at the start and end of the pixels to calculate the initial and final coverage of the range (ie, if
/// the alpha value is 0 at the start of a pixel and non-zero at the end, that pixel will be partially covered rather than clear)
///
#[inline]
fn actual_fade_for_range(render_x_range: &Range<f64>, alpha_x_range: &Range<f64>, alpha_range: &Range<f64>) -> Range<f64> {
    if (alpha_x_range.end - alpha_x_range.start).abs() < 1e-6 {
        // Treat the intercept as a vertical line
        let offset = alpha_x_range.start - render_x_range.start;
        let offset = offset / (render_x_range.end - render_x_range.start);

        // Assuming that we're covering a single pixel, we can just use the offset here
        if alpha_range.start > 0.5 {
            offset..offset
        } else {
            (1.0 - offset)..(1.0 - offset)
        }
    } else {
        // Adjust the alpha range to the actual x range
        let pixel_x_start = render_x_range.start.floor();
        let pixel_x_end = render_x_range.end.ceil();
        let alpha_x_start = alpha_x_range.start;
        let alpha_x_end = alpha_x_range.end;

        let alpha_ratio = (alpha_range.end - alpha_range.start) / (alpha_x_end - alpha_x_start);

        let corrected_start_1 = (pixel_x_start - alpha_x_start) * alpha_ratio + alpha_range.start;
        let corrected_start_2 = ((pixel_x_start + 1.0) - alpha_x_start) * alpha_ratio + alpha_range.start;
        let corrected_end_1 = ((pixel_x_end - 1.0) - alpha_x_start) * alpha_ratio + alpha_range.start;
        let corrected_end_2 = (pixel_x_end - alpha_x_start) * alpha_ratio + alpha_range.start;

        let corrected_start = alpha_coverage(corrected_start_1, corrected_start_2);
        let corrected_end = alpha_coverage(corrected_end_1, corrected_end_2);

        let corrected_range = (corrected_start.max(0.0).min(1.0))..(corrected_end.max(0.0).min(1.0));

        corrected_range
    }
}

impl<TEdge> ScanPlanner for ShardScanPlanner<TEdge>
    where
        TEdge: EdgeDescriptor,
{
    type Edge = TEdge;

    fn plan_scanlines(&self, edge_plan: &EdgePlan<Self::Edge>, transform: &ScanlineTransform, y_positions: &[f64], x_range: Range<f64>, scanlines: &mut [(f64, ScanlinePlan)]) {
        // Must be enough scanlines supplied for filling the scanline array
        if scanlines.len() < y_positions.len() {
            panic!("The number of scanline suppled ({}) is less than the number of y positions to fill them ({})", scanlines.len(), y_positions.len());
        }

        // y-positions should be offset by half a pixel (shards are taken from a previous and a next line)
        let half_pixel = transform.pixel_range_to_x(&(0..1));
        let half_pixel = (half_pixel.end - half_pixel.start) / 2.0;

        let scan_positions = y_positions.iter()
            .map(|y| y - half_pixel)
            .chain(y_positions.last().map(|y| y + half_pixel))
            .collect::<Vec<_>>();

        // Map the x-range from the source coordinates to pixel coordinates
        let x_range = transform.source_x_to_pixels(x_range.start)..transform.source_x_to_pixels(x_range.end);

        // Ask the edge plan to compute the intercepts on the current scanline
        let mut ordered_intercepts = vec![vec![]; y_positions.len()];
        edge_plan.shards_on_scanlines(&scan_positions, &mut ordered_intercepts);

        'next_line: for y_idx in 0..y_positions.len() {
            // Fetch/clear the scanline that we'll be building
            let (scanline_pos, scanline) = &mut scanlines[y_idx];
            scanline.clear();
            *scanline_pos = y_positions[y_idx];

            // Iterate over the intercepts on this line
            let scanline_intercepts = &ordered_intercepts[y_idx];
            let mut scanline_intercepts = ShardInterceptIterator::from_intercepts(scanline_intercepts.into_iter(), transform);

            // Each shard has two intercepts: the lower is where we start fading into or out of the shape, and the upper is where we finish, either ending up fully inside
            // or outside the shape.

            // Initial program/position comes from the earliest intercept position
            let mut current_intercept = if let Some(intercept) = scanline_intercepts.next() { intercept } else { continue; };

            // Trace programs but don't generate fragments until we get an intercept
            let mut active_shapes = ScanlineShardInterceptState::new();

            while current_intercept.x_pos() < x_range.start {
                // Add or remove this intercept's programs to the active list
                let shape_descriptor = edge_plan.shape_descriptor(current_intercept.shape());

                match &current_intercept {
                    ShardIntercept::Start(intercept) => active_shapes.start_intercept(intercept, transform, shape_descriptor),
                    ShardIntercept::Finish(intercept) => active_shapes.finish_intercept(intercept, shape_descriptor),
                }

                // Move to the next intercept (or stop if no intercepts actually fall within the x-range)
                current_intercept = if let Some(intercept) = scanline_intercepts.next() { intercept } else { continue 'next_line; };
            }

            // Update all of the existing shapes to have a start position at the left-hand side of the screen
            active_shapes.clip_start_x(x_range.start as _);

            // Read intercepts until we reach the x_range end, and generate the program stacks for the scanline plan
            let mut last_x = x_range.start;
            let mut program_stack = vec![];
            let mut scanplan = vec![];
            let mut z_floor = active_shapes.z_floor();

            loop {
                // TODO: if a program range is < 1px, instead of just ignoring it, use a blend program (provides horizontal-only anti-aliasing)
                // TODO: if there are multiple intercepts on the same pixel, we should process them all simultaneously (otherwise we will occasionally start a set of programs one pixel too late)

                // Generate a stack for the current intercept
                let next_x = current_intercept.x_pos();

                // The end of the current range is the 'next_x' coordinate
                let next_x = if next_x > x_range.end { x_range.end } else { next_x };
                let stack_depth = active_shapes.len();

                // We use the z-index of the current shape to determine if it's in front of or behind the current line
                let shape_id = current_intercept.shape();
                let z_index = edge_plan.shape_z_index(shape_id);
                let shape_descriptor = edge_plan.shape_descriptor(shape_id);

                if z_index >= z_floor && next_x != last_x {
                    // Create a program stack between the ranges: all the programs until the first opaque layer
                    let x_range = last_x..next_x;
                    let mut is_opaque = false;

                    // We re-use program_stack so we don't have to keep re-allocating a vec as we go
                    program_stack.clear();
                    for shape in (0..stack_depth).rev() {
                        let intercept = active_shapes.get(shape).unwrap();
                        let shape_descriptor = intercept.shape_descriptor();
                        let mut blend = intercept.blend();
                        let mut num_blends = 0;

                        // Start the blends for the program
                        loop {
                            match blend {
                                InterceptBlend::Solid => {
                                    break;
                                }

                                InterceptBlend::Fade { x_range: alpha_x_range, alpha_range } => {
                                    // Adjust the alpha range to the actual x range
                                    let corrected_range = actual_fade_for_range(&x_range, &alpha_x_range, &alpha_range);

                                    // Run a linear blend using the corrected range
                                    program_stack.push(PixelProgramPlan::LinearSourceOver(corrected_range.start as _, corrected_range.end as _));
                                    num_blends += 1;
                                    break;
                                }

                                InterceptBlend::NestedFade { x_range: alpha_x_range, alpha_range, nested } => {
                                    // Adjust the alpha range to the actual x range
                                    let corrected_range = actual_fade_for_range(&x_range, &alpha_x_range, &alpha_range);

                                    // Run a linear blend using the corrected range
                                    program_stack.push(PixelProgramPlan::LinearSourceOver(corrected_range.start as _, corrected_range.end as _));

                                    // Apply the nested gradient as well
                                    blend = &*nested;
                                    num_blends += 1;
                                }
                            }
                        }

                        // Run the program for this range
                        program_stack.extend(shape_descriptor.programs.iter().map(|program| PixelProgramPlan::Run(*program)));

                        // Finish the blends
                        if num_blends > 0 {
                            program_stack.extend((0..num_blends).map(|_| PixelProgramPlan::StartBlend));
                        }

                        if intercept.is_opaque() {
                            is_opaque = true;
                            break;
                        }
                    }

                    if !program_stack.is_empty() {
                        // Create the stack for these programs
                        let stack = ScanSpanStack::with_reversed_programs(x_range, is_opaque, &program_stack);

                        // Add the stack to the scanplan
                        scanplan.push(stack);
                    }

                    // Next span will start after the end of this one
                    last_x = next_x;
                }

                // Update the state from the current intercept
                match &current_intercept {
                    ShardIntercept::Start(intercept) => active_shapes.start_intercept(intercept, transform, shape_descriptor),
                    ShardIntercept::Finish(intercept) => active_shapes.finish_intercept(intercept, shape_descriptor),
                }

                z_floor = active_shapes.z_floor();

                // Stop when the next_x value gets to the end of the range
                if next_x >= x_range.end {
                    break;
                }

                // Get ready to process the next intercept in the stack
                current_intercept = if let Some(next_intercept) = scanline_intercepts.next() { next_intercept } else { break; };
            }

            // Populate the scanline
            #[cfg(debug_assertions)]
            {
                scanline.fill_from_ordered_stacks(scanplan);
            }

            #[cfg(not(debug_assertions))]
            {
                unsafe { scanline.fill_from_ordered_stacks_prechecked(scanplan); }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn alpha_coverage_100_percent() {
        let coverage = alpha_coverage(1.5, 1.5);
        assert!((coverage - 1.0).abs() < 0.0001, "{:?}", coverage);
    }

    #[test]
    fn alpha_coverage_0_percent() {
        let coverage = alpha_coverage(-0.5, -0.5);
        assert!((coverage - 0.0).abs() < 0.0001, "{:?}", coverage);
    }

    #[test]
    fn alpha_coverage_50_percent() {
        let coverage = alpha_coverage(0.0, 1.0);
        assert!((coverage - 0.5).abs() < 0.0001, "{:?}", coverage);
    }

    #[test]
    fn alpha_coverage_lower_eighth() {
        let coverage = alpha_coverage(-0.5, 0.5);
        assert!((coverage - 0.125).abs() < 0.0001, "{:?}", coverage);
    }

    #[test]
    fn alpha_coverage_lower_quarter() {
        let coverage = alpha_coverage(0.0, 0.5);
        assert!((coverage - 0.25).abs() < 0.0001, "{:?}", coverage);
    }

    #[test]
    fn alpha_coverage_upper_quarter() {
        let coverage = alpha_coverage(0.5, 1.0);
        assert!((coverage - 0.75).abs() < 0.0001, "{:?}", coverage);
    }

    #[test]
    fn alpha_coverage_lower_quarter_reversed() {
        let coverage = alpha_coverage(0.5, 0.0);
        assert!((coverage - 0.25).abs() < 0.0001, "{:?}", coverage);
    }

    #[test]
    fn alpha_coverage_lower_partial() {
        let coverage = alpha_coverage(0.1, 0.6);
        assert!((coverage - 0.35).abs() < 0.0001, "{:?}", coverage);
    }

    #[test]
    fn alpha_coverage_middle_quarter() {
        let coverage = alpha_coverage(-0.5, 2.0);
        assert!((coverage - 0.6).abs() < 0.0001, "{:?}", coverage);
    }
}
