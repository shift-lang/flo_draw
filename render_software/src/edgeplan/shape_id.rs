/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::sync::atomic::{AtomicUsize, Ordering};

///
/// Identifies a shape that an edge is a part of (ie, when an edge is crossed, we are entering or leaving this shape)
///
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShapeId(pub(crate) usize);

impl ShapeId {
    ///
    /// Creates a new shpae ID (unique within this process)
    ///
    pub fn new() -> ShapeId {
        static NEXT_VALUE: AtomicUsize = AtomicUsize::new(0);

        let next_value = NEXT_VALUE.fetch_add(1, Ordering::Relaxed);
        ShapeId(next_value)
    }
}
