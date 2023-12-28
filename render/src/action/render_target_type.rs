/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

///
/// The types of render target that can be created by the render layer
///
#[derive(Clone, Copy, PartialEq, Debug, Hash)]
pub enum RenderTargetType {
    /// Standard off-screen render target (with a texture)
    Standard,

    /// Off-screen render target for reading back to the CPU
    StandardForReading,

    /// Multisampled render target
    Multisampled,

    /// Multisampled texture render target
    MultisampledTexture,

    /// Monochrome off-screen render target (only writes the red channel)
    Monochrome,

    /// Multisampled monochrome off-screen render target (only writes the red channel)
    MonochromeMultisampledTexture,
}
