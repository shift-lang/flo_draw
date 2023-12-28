/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use wgpu;

use std::sync::*;

///
/// Representation of a texture stored in the WGPU renderer
///
#[derive(Clone)]
pub(crate) struct WgpuTexture {
    /// The descriptor used to create the texture
    pub descriptor: wgpu::TextureDescriptor<'static>,

    /// The WGPU texture stored here
    pub texture: Arc<wgpu::Texture>,

    /// True if this texture has premultiplied alpha
    pub is_premultiplied: bool,
}
