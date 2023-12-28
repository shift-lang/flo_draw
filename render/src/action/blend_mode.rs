/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

///
/// The blending modes that the renderer must support (most of the Porter-Duff modes)
///
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlendMode {
    SourceOver,
    DestinationOver,
    SourceIn,
    DestinationIn,
    SourceOut,
    DestinationOut,
    SourceATop,
    DestinationATop,

    Screen,
    Multiply,

    AllChannelAlphaSourceOver,
    AllChannelAlphaDestinationOver,
}
