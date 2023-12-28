/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

///
/// The state a dynamic texture was in the last time it was rendered
///
#[derive(PartialEq)]
pub struct DynamicTextureState {
    /// The viewport size for the texture
    pub(super) viewport: (f32, f32),

    /// The number of times the sprite was modified
    pub(super) sprite_modification_count: usize,
}
