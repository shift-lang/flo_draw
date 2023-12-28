/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::VecDeque;

use flo_rope::*;
use futures::task::*;

///
/// The state of a stream that is reading from a rope binding core
///
pub(super) struct RopeStreamState<Cell, Attribute>
where
    Cell: Clone + PartialEq,
    Attribute: Clone + PartialEq + Default,
{
    /// The identifier for this stream
    pub(super) identifier: usize,

    /// The waker for the current stream
    pub(super) waker: Option<Waker>,

    /// The changes that are waiting to be sent to this stream
    pub(super) pending_changes: VecDeque<RopeAction<Cell, Attribute>>,

    /// True if the rope has indicated there are changes waiting to be pulled
    pub(super) needs_pull: bool,
}
