use super::Pixel;
use super::f32_linear::*;
use super::rgba_texture::*;
use super::texture_reader::*;

use once_cell::sync::{Lazy};

///
/// Table that maps values with the 8 upper bits representing the alpha value and the 8 lower bits representing the colour value
/// to their premultiplied-alpha equivalents
///
static TO_PREMULTIPLIED_LINEAR_WITH_ALPHA: Lazy<[f32; 65536]> = Lazy::new(|| {
    let mut table = [0.0; 65536];

    for a in 0..256 {
        // Convert the alpha value to f64 (these are always linear)
        let alpha = (a as f64) / 255.0;

        for c in 0..256 {
            // Gamma correct the value and pre-multiply it
            let val = (c as f64) / 255.0;
            let val = val.powf(2.2);
            let val = val * alpha;

            // Store in the table
            let table_pos = (a << 8) | c;
            table[table_pos] = val as f32;
        }
    }

    table
});

impl TextureReader<RgbaTexture> for F32LinearPixel {
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
        let af = (a as f32) / 255.0;

        F32LinearPixel::from_components([rf, gf, bf, af])
    }
}
