/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use once_cell::sync::Lazy;

use super::rgba_texture::*;
use super::texture_reader::*;
use super::u32_linear::*;
use super::Pixel;

///
/// Table that maps values with the 8 upper bits representing the alpha value and the 8 lower bits representing the colour value
/// to their premultiplied-alpha equivalents
///
static TO_PREMULTIPLIED_LINEAR_WITH_ALPHA: Lazy<[u16; 65536]> = Lazy::new(|| {
    let mut table = [0u16; 65536];

    for a in 0..256 {
        // Convert the alpha value to f64 (these are always linear)
        let alpha = (a as f64) / 255.0;

        for c in 0..256 {
            // Gamma correct the value and pre-multiply it
            let val = (c as f64) / 255.0;
            let val = val.powf(2.2);
            let val = val * alpha;
            let val = (val * 65535.0) as u16;

            // Store in the table
            let table_pos = (a << 8) | c;
            table[table_pos] = val;
        }
    }

    table
});

impl TextureReader<RgbaTexture> for U32LinearPixel {
    #[inline]
    fn read_pixel(texture: &RgbaTexture, x: f64, y: f64) -> Self {
        // Read the pixel at the floor of the supplied position
        let [r, g, b, a] = texture.read_pixel(x.floor() as _, y.floor() as _);

        // Use the 2.2 gamma conversion table to convert to a F32 pixel (we assume non-premultiplied RGBA pixels with a gamma of 2.2)
        let alpha = (a as usize) << 8;
        let ri = (r as usize) | alpha;
        let gi = (g as usize) | alpha;
        let bi = (b as usize) | alpha;

        let rf = unsafe { *(*TO_PREMULTIPLIED_LINEAR_WITH_ALPHA).get_unchecked(ri) };
        let gf = unsafe { *(*TO_PREMULTIPLIED_LINEAR_WITH_ALPHA).get_unchecked(gi) };
        let bf = unsafe { *(*TO_PREMULTIPLIED_LINEAR_WITH_ALPHA).get_unchecked(bi) };
        let af = (a as u16) << 8;

        U32LinearPixel::from_components([rf.into(), gf.into(), bf.into(), af.into()])
    }
}
