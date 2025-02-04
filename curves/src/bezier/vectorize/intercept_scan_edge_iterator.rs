/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::sampled_contour::*;

use smallvec::*;

use std::ops::Range;

///
/// An iterator that finds the edges of a contour by calling the `intercepts_on_line()` function
///
pub struct InterceptScanEdgeIterator<TLineIterator> {
    /// The contour that this is tracing the edges of
    line_iterator: TLineIterator,

    /// Set to true once there are no more lines in the line iterator
    finished: bool,

    /// The y pos of the current line
    ypos: f64,

    /// The preceding the current one
    previous_line: SmallVec<[Range<usize>; 4]>,

    /// The line following the current one
    current_line: SmallVec<[Range<usize>; 4]>,

    /// Index into the previous line of the current x position
    previous_pos: usize,

    /// Index into the current line of the current x position
    current_pos: usize,

    /// The next x position to return
    xpos: usize,
}

impl<TLineIterator> InterceptScanEdgeIterator<TLineIterator>
where
    TLineIterator: Iterator<Item = SmallVec<[Range<usize>; 4]>>,
{
    ///
    /// Creates a new edge iterator at the top-left corner of a contour
    ///
    /// The line iterator should return the rounded intercepts on each line in the contour
    ///
    pub fn from_iterator(line_iterator: TLineIterator) -> InterceptScanEdgeIterator<TLineIterator> {
        // Create an edge iterator in a neutral state
        let mut iterator = InterceptScanEdgeIterator {
            line_iterator: line_iterator,
            finished: false,
            ypos: 0.0,
            previous_line: smallvec![],
            current_line: smallvec![],
            previous_pos: 0,
            current_pos: 0,
            xpos: 0,
        };

        // Load the first line into the iterator
        iterator.load_line(0.0);

        iterator
    }

    ///
    /// Loads a line ahead of the current line into this contour
    ///
    fn load_line(&mut self, ypos: f64) {
        use std::mem;

        let mut ypos = ypos;

        loop {
            // Move the current line into the previous line
            mem::swap(&mut self.previous_line, &mut self.current_line);

            // Load the next line from the contour
            if let Some(line) = self.line_iterator.next() {
                self.current_line = line;
            } else {
                self.finished = true;
                self.current_line = smallvec![];
            }

            // Try to pick an x position to start at (one of the lines must be non-empty)
            let mut xpos = None;

            if self.previous_line.len() > 0 {
                xpos = Some(self.previous_line[0].start);
            }

            if self.current_line.len() > 0 {
                xpos = xpos.map_or(Some(self.current_line[0].start), |xpos| {
                    Some(xpos.min(self.current_line[0].start))
                });
            }

            if let Some(xpos) = xpos {
                // Found the next line
                self.previous_pos = 0;
                self.current_pos = 0;
                self.xpos = xpos;
                self.ypos = ypos;
                break;
            }

            // Try the next y position if we didn't find a match
            ypos += 1.0;

            if self.finished {
                // No more lines in this shape
                self.previous_pos = 0;
                self.current_pos = 0;
                self.xpos = 0;
                break;
            }
        }
    }
}

impl<TLineIterator> Iterator for InterceptScanEdgeIterator<TLineIterator>
where
    TLineIterator: Iterator<Item = SmallVec<[Range<usize>; 4]>>,
{
    type Item = (ContourPosition, ContourCell);

    fn next(&mut self) -> Option<Self::Item> {
        // Outer loop: move lines
        loop {
            if self.finished && self.previous_line.is_empty() {
                // Stop once the ypos leaves the end of the shape and there's no previous line
                return None;
            }

            // Inner loop: move within the current line
            loop {
                let xpos = self.xpos;

                // The previous line specifies whether or not the upper part of the current edge is filled, and the next line specifies whether or not the previous edge is filled
                let upper = self.previous_line.get(self.previous_pos);

                // Move the previous pos on if the x position has exceeded the current range of filled values
                if let Some(upper) = upper {
                    if xpos > upper.end {
                        self.previous_pos += 1;
                        continue;
                    }
                }

                let lower = self.current_line.get(self.current_pos);

                // Move the current pos on if the x position has exceeded the current range of filled values
                if let Some(lower) = lower {
                    if xpos > lower.end {
                        self.current_pos += 1;
                        continue;
                    }
                }

                // If both are beyond the end of the range, then we've finished the current edge
                if upper.is_none() && lower.is_none() {
                    // Leaving the inner loop will move to the next line
                    break;
                }

                // If both the upper and lower lines are empty, then move to the first filled spot
                if upper.map_or(true, |upper| xpos < upper.start)
                    && lower.map_or(true, |lower| xpos < lower.start)
                {
                    match (upper, lower) {
                        (Some(upper), Some(lower)) => {
                            self.xpos = upper.start.min(lower.start);
                        }
                        (Some(upper), None) => {
                            self.xpos = upper.start;
                        }
                        (None, Some(lower)) => {
                            self.xpos = lower.start;
                        }

                        (None, None) => {
                            unreachable!()
                        } // As this case is handled above
                    }

                    continue;
                }

                // If both the upper and lower lines are filled, then move to the earliest end point
                if upper.map_or(false, |upper| xpos > upper.start && xpos < upper.end)
                    && lower.map_or(false, |lower| xpos > lower.start && xpos < lower.end)
                {
                    match (upper, lower) {
                        (Some(upper), Some(lower)) => {
                            self.xpos = upper.end.min(lower.end);
                        }

                        _ => {
                            unreachable!()
                        } // Because we map to 'false' if either is None: hitting the end of the range is the same as the pixel being empty
                    }

                    continue;
                }

                // At least one of the upper or lower lines is transitioning between filled and unfilled at the current xpos
                let (tl, tr) = upper.map_or((false, false), |upper| {
                    (
                        xpos > upper.start && xpos <= upper.end,
                        xpos >= upper.start && xpos < upper.end,
                    )
                });
                let (bl, br) = lower.map_or((false, false), |lower| {
                    (
                        xpos > lower.start && xpos <= lower.end,
                        xpos >= lower.start && xpos < lower.end,
                    )
                });

                // Next iteration should look at the next cell along
                self.xpos += 1;

                // Found a cell to return to the caller
                return Some((
                    ContourPosition(xpos as _, self.ypos as _),
                    ContourCell::from_corners(tl, tr, bl, br),
                ));
            }

            // Read in the next line from the contour
            self.load_line(self.ypos + 1.0);
        }
    }
}

///
/// Iterator that returns the intercepts for each line of a contour
///
pub struct ContourInterceptsIterator<'a, TContour> {
    /// The contour to find the intercepts on
    contour: &'a TContour,

    /// The height of the contour
    height: f64,

    /// The current y position along the contour
    ypos: f64,
}

impl<'a, TContour> ContourInterceptsIterator<'a, TContour>
where
    TContour: SampledContour,
{
    ///
    /// Creates a new iterator that will return the intercepts in a contour
    ///
    #[inline]
    pub fn new(contour: &'a TContour) -> ContourInterceptsIterator<'a, TContour> {
        ContourInterceptsIterator {
            contour: contour,
            height: contour.contour_size().height() as _,
            ypos: 0.0,
        }
    }
}

impl<'a, TContour> Iterator for ContourInterceptsIterator<'a, TContour>
where
    TContour: SampledContour,
{
    type Item = SmallVec<[Range<usize>; 4]>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.ypos >= self.height {
            // Finished all the lines in the contour
            None
        } else {
            // Get the current line
            let current_line = self.contour.rounded_intercepts_on_line(self.ypos);

            // Move to the next line
            self.ypos += 1.0;

            Some(current_line)
        }
    }
}
