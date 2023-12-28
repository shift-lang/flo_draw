/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

///
/// Layout for the TextureSettings uniform
///
#[derive(Clone, Copy, PartialEq, Debug, Default)]
#[repr(C, packed)]
pub struct TextureSettings {
    pub transform: [[f32; 4]; 4],
    pub alpha: f32,
    pub _padding: [u32; 3],
}
