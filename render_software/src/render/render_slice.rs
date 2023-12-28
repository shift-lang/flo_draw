/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

///
/// Specifies a portion of a frame to render
///
#[derive(Debug)]
pub struct RenderSlice {
    /// The width in pixels of a scanline
    pub width: usize,

    /// The y-positions that should be rendered to the buffer
    pub y_positions: Vec<f64>,
}
