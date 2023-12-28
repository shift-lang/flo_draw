/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use flo_render::*;

pub(crate) const MAIN_RENDER_TARGET: RenderTargetId = RenderTargetId(0);
pub(crate) const CLIP_RENDER_TARGET: RenderTargetId = RenderTargetId(1);
pub(crate) const RESOLVE_RENDER_TARGET: RenderTargetId = RenderTargetId(2);

pub(crate) const MAIN_RENDER_TEXTURE: TextureId = TextureId(0);
pub(crate) const CLIP_RENDER_TEXTURE: TextureId = TextureId(1);
pub(crate) const DASH_TEXTURE: TextureId = TextureId(2);
