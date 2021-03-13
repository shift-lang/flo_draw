use flo_canvas as canvas;
use flo_render as render;

///
/// The ways the next path can be filled
///
#[derive(Clone, PartialEq)]
pub enum FillState {
    ///
    /// Unknown fill state
    ///
    None,

    ///
    /// Fill with a solid colour
    ///
    Color(render::Rgba8),

    ///
    /// Fill with a particular texture
    ///
    Texture(canvas::TextureId, render::Matrix, bool)
}

impl FillState {
    ///
    /// Returns a variant of this fill state with all channels set as the alpha channel
    ///
    pub fn all_channel_alpha(&self) -> Self {
        match self {
            FillState::None             => FillState::None,
            FillState::Color(color)     => FillState::Color(render::Rgba8([color.0[3], color.0[3], color.0[3], color.0[3]])),
            FillState::Texture(_, _, _) => self.clone()
        }
    }

    ///
    /// Returns the flat colour to use for this fill state
    ///
    pub fn flat_color(&self) -> render::Rgba8 {
        match self {
            FillState::None             => render::Rgba8([0, 0, 0, 255]),
            FillState::Color(color)     => *color,
            FillState::Texture(_, _, _) => render::Rgba8([0, 0, 0, 255])
        }
    }

    ///
    /// Creates a texture fill 
    ///
    pub fn texture_fill(texture_id: canvas::TextureId, x1: f32, y1: f32, x2: f32, y2: f32) -> FillState {
        // Avoid division by zero
        let x2 = if x2 == x1 { x1 + 0.0000001 } else { x2 };
        let y2 = if y2 == y1 { y1 + 0.0000001 } else { y2 };

        // Generate a matrix that transforms x1, y1 to 0,0 and x2, y2 to 1,1
        let a       = 1.0/(x2-x1);
        let b       = 0.0;
        let c       = -x1;

        let d       = 1.0/(y2-y1);
        let e       = 0.0;
        let f       = -y1;

        let matrix  = render::Matrix([
            [a,   b,   0.0, c  ],
            [d,   e,   0.0, f  ],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0]
        ]);

        // Create the fill-state for this matrix
        FillState::Texture(texture_id, matrix, true)
    }
}