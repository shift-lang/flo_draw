/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

/// An identifier corresponding to a vertex buffer
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct VertexBufferId(pub usize);

/// An identifier corresponding to an index buffer
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct IndexBufferId(pub usize);

/// An identifier corresponding to a render target
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct RenderTargetId(pub usize);

/// An identifier corresponding to a texture
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct TextureId(pub usize);
