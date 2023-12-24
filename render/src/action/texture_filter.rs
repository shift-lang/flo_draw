use super::identities::*;

use std::f32;

///
/// Filters that can be applied to a texture by the rendering engine
///
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum TextureFilter {
    /// Applies a horizontal gaussian blur with the specified sigma (standard deviation) and step value, using a 9-pixel kernel
    GaussianBlurHorizontal9(f32, f32),

    /// Applies a horizontal gaussian blur with the specified sigma (standard deviation) and step value, using a 29-pixel kernel
    GaussianBlurHorizontal29(f32, f32),

    /// Applies a horizontal gaussian blur with the specified sigma (standard deviation) and step value, using a 61-pixel kernel
    GaussianBlurHorizontal61(f32, f32),

    /// Applies a vertical gaussian blur with the specified sigma (standard deviation) and step value, using a 9-pixel kernel
    GaussianBlurVertical9(f32, f32),

    /// Applies a vertical gaussian blur with the specified sigma (standard deviation) and step value, using a 9-pixel kernel
    GaussianBlurVertical29(f32, f32),

    /// Applies a vertical gaussian blur with the specified sigma (standard deviation) and step value, using a 9-pixel kernel
    GaussianBlurVertical61(f32, f32),

    /// Applies a gaussian blur in the horizontal direction with the specified sigma, step and kernel size
    GaussianBlurHorizontal(f32, f32, usize),

    /// Applies a gaussian blur in the vertical direction with the specified sigma, step and kernel size
    GaussianBlurVertical(f32, f32, usize),

    /// Adjusts the transparency of a texture
    AlphaBlend(f32),

    /// Masks a texture according to the content of another texture
    Mask(TextureId),

    /// Performs a displacement map with the specified texture ID and scale factors (scale factors use the 0-1 coordinate scheme for the whole texture, so need to be transformed into that range)
    DisplacementMap(TextureId, f32, f32),
}

impl TextureFilter {
    ///
    /// If this filter uses a kernel, returns the size to generate for the shader.
    ///
    /// This is the value to pass in for `weights_for_gaussian_blur`, so it's the half the total size of the kernel,
    /// plus 1 for the central value.
    ///
    pub(crate) fn kernel_size(&self) -> usize {
        use TextureFilter::*;

        match self {
            GaussianBlurHorizontal9(_, _) => 5,
            GaussianBlurHorizontal29(_, _) => 15,
            GaussianBlurHorizontal61(_, _) => 31,
            GaussianBlurVertical9(_, _) => 5,
            GaussianBlurVertical29(_, _) => 15,
            GaussianBlurVertical61(_, _) => 31,
            GaussianBlurHorizontal(_, _, size) => (size - 1) / 2 + 1,
            GaussianBlurVertical(_, _, size) => (size - 1) / 2 + 1,

            AlphaBlend(_) => 0,
            Mask(_) => 0,
            DisplacementMap(_, _, _) => 0,
        }
    }

    ///
    /// Computes the 1D weights for a gaussian blur for a particular standard deviation
    ///
    pub(crate) fn weights_for_gaussian_blur(sigma: f32, step: f32, count: usize) -> Vec<f32> {
        // Short-circuit the case where count is 0
        if count == 0 { return vec![]; }

        let sigma_squared = sigma * sigma;

        // Compute the weight at each position
        let uncorrected = (0..count).into_iter()
            .map(|x| {
                let x = x as f32;
                let x = x * step;
                (1.0 / ((2.0 * f32::consts::PI * sigma_squared).sqrt())) * (f32::consts::E.powf(-(x * x) / (2.0 * sigma_squared)))
            })
            .collect::<Vec<_>>();

        // Correct the blur so that the weights all add up to 1
        let sum = uncorrected[0] + uncorrected.iter().skip(1).fold(0.0, |x, y| x + *y) * 2.0;
        let corrected = uncorrected.into_iter().map(|weight| weight / sum).collect();

        corrected
    }

    ///
    /// Transforms the weights for the gaussian blur to a set of offsets and weights that can be used
    /// with bilinear texture filtering
    ///
    /// See See <https://www.rastergrid.com/blog/2010/09/efficient-gaussian-blur-with-linear-sampling/> for a
    /// description of this algorithm
    ///
    pub(crate) fn weights_and_offsets_for_gaussian_blur(weights: Vec<f32>) -> (Vec<f32>, Vec<f32>) {
        let mut new_weights = vec![weights[0]];
        let mut new_offsets = vec![0.0];

        let mut idx = 1;
        while idx < weights.len() - 1 {
            let offset1 = idx as f32;
            let offset2 = (idx + 1) as f32;

            let new_weight = weights[idx] + weights[idx + 1];
            new_weights.push(new_weight);
            new_offsets.push((offset1 * weights[idx] + offset2 * weights[idx + 1]) / new_weight);

            idx += 2;
        }

        (new_weights, new_offsets)
    }
}
