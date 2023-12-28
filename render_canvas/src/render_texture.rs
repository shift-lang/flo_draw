/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use flo_render as render;

///
/// Used to indicate the state of a texture
///
/// A 'loading' texture is one where we're still writing data, where a 'Ready' texture is one where we've
/// generated the mipmap and are using it somewhere in the core
///
#[derive(Clone, Copy, Debug)]
pub enum RenderTexture {
    Loading(render::TextureId),
    Ready(render::TextureId),
}

impl Into<render::TextureId> for RenderTexture {
    fn into(self) -> render::TextureId {
        match self {
            RenderTexture::Loading(texture_id) => texture_id,
            RenderTexture::Ready(texture_id) => texture_id,
        }
    }
}

impl Into<render::TextureId> for &RenderTexture {
    fn into(self) -> render::TextureId {
        match self {
            RenderTexture::Loading(texture_id) => *texture_id,
            RenderTexture::Ready(texture_id) => *texture_id,
        }
    }
}
