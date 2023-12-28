/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

///
/// Trait implemented by pixel types that can be converted to a gamma-corrected colour space
///
pub trait ToGammaColorSpace<TargetPixel>: Sized {
    /// Converts this pixel from its current colour space to a gamma corrected colour space
    fn to_gamma_colorspace(input_pixels: &[Self], output_pixels: &mut [TargetPixel], gamma: f64);
}
