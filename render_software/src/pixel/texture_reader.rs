/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

///
/// Trait implemented by types that can be read from a texture type
///
pub trait TextureReader<TTexture>: Send + Sync
where
    TTexture: Send + Sync,
{
    /// Reads the pixel at the specified position in the texture
    ///
    /// The coordinates are fractions of pixels
    fn read_pixel(texture: &TTexture, x: f64, y: f64) -> Self;
}
