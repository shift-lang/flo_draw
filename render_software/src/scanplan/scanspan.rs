/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::pixel::*;

use std::fmt;
use std::ops::Range;

///
/// A ScanSpan indicates which program(s) to apply to a range along a scanline
///
#[derive(Clone, PartialEq)]
pub struct ScanSpan {
    /// The pixels to draw on the scanline
    pub(super) x_range: Range<f64>,

    /// The ID of the program data for the program to run over this range
    pub(super) program: PixelProgramDataId,

    /// True if this span is opaque (entirely obscures anything underneath it)
    pub(super) opaque: bool,
}

impl ScanSpan {
    ///
    /// Creates a scanspan that will run a single program that changes the pixels underneath it
    ///
    #[inline]
    pub fn transparent(range: Range<f64>, program: PixelProgramDataId) -> ScanSpan {
        ScanSpan {
            x_range: range,
            program: program,
            opaque: false,
        }
    }

    ///
    /// Creates a scanspan that will run a single program that replaces the pixels underneath it
    ///
    #[inline]
    pub fn opaque(range: Range<f64>, program: PixelProgramDataId) -> ScanSpan {
        ScanSpan {
            x_range: range,
            program: program,
            opaque: true,
        }
    }

    ///
    /// Splits this span at the specified position
    ///
    /// Returns the same span if the split would result in a 0-length span
    ///
    #[inline]
    pub fn split(self, pos: f64) -> Result<(ScanSpan, ScanSpan), ScanSpan> {
        if pos > self.x_range.start && pos < self.x_range.end {
            Ok((
                ScanSpan {
                    x_range: (self.x_range.start)..pos,
                    program: self.program,
                    opaque: self.opaque,
                },
                ScanSpan {
                    x_range: pos..(self.x_range.end),
                    program: self.program,
                    opaque: self.opaque,
                },
            ))
        } else {
            Err(self)
        }
    }
}

impl fmt::Debug for ScanSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.opaque {
            write!(f, "O({:?}: {:?})", self.x_range, self.program)
        } else {
            write!(f, "T({:?}: {:?})", self.x_range, self.program)
        }
    }
}
