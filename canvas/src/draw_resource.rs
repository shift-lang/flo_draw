use crate::draw::*;
use crate::font::*;
use crate::texture::*;

use smallvec::*;

///
/// Describes a resource that a drawing instruction can be attached to
///
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub (crate) enum DrawResource {
    Frame,
    Canvas,

    Layer(LayerId),
    Sprite(SpriteId),

    Texture(TextureId),
    Font(FontId),
    
    StrokeLineWidth,
    StrokeLineCap,
    StrokeLineJoin,
    StrokeDash,
    StrokeColor,

    FillWindingRule,
    FillBlend,
    FillColor,
}

impl Draw {
    ///
    /// Returns the resource that this drawing instruction requires to operate
    ///
    /// The active resource is the sprite or the layer that is currently selected for drawing
    ///
    #[inline]
    pub (crate) fn source_resource(&self, active_resource: &DrawResource) -> SmallVec<[DrawResource; 6]> {
        use self::Draw::*;

        match self {
            // Things that overwrite/create a new value for a resource have no source
            Texture(_, TextureOp::Create(_, _, _))  => smallvec![],
            Font(_, FontOp::UseFontDefinition(_))   => smallvec![],

            LineWidth(_)                            |
            LineWidthPixels(_)                      |
            LineJoin(_)                             |
            LineCap(_)                              |
            NewDashPattern                          |
            StrokeColor(_)                          => smallvec![],

            WindingRule(_)                          |
            BlendMode(_)                            |
            FillColor(_)                            => smallvec![],

            // Dash pattern is defined by multiple steps
            DashLength(_)                           |
            DashOffset(_)                           => smallvec![DrawResource::StrokeDash],

            // The fill and stroke operations depend on multiple resources, so their resource is 'special'
            Fill                                    => smallvec![DrawResource::FillWindingRule, DrawResource::FillBlend, DrawResource::FillColor],
            Stroke                                  => smallvec![DrawResource::StrokeLineWidth, DrawResource::StrokeLineCap, DrawResource::StrokeLineJoin, DrawResource::StrokeDash, DrawResource::StrokeColor, DrawResource::FillBlend],

            // Texture and font operations generally alter the existing resource so they have a dependency
            Texture(texture_id, _)                  => smallvec![DrawResource::Texture(*texture_id)],
            Font(font_id, _)                        => smallvec![DrawResource::Font(*font_id)],

            // DrawText and FillTexture use the corresponding resource
            DrawText(font_id, _, _, _)              => smallvec![DrawResource::Font(*font_id)],
            FillTexture(texture_id, _, _)           => smallvec![DrawResource::Texture(*texture_id)],

            // Transforms use the 'canvas' resource (setting the height or the identity transform resets any previous transform)
            IdentityTransform                       |
            CanvasHeight(_)                         => smallvec![],

            CenterRegion(_, _)                      |
            MultiplyTransform(_)                    => smallvec![DrawResource::Canvas],

            // Most things just affect the active resource
            _                                       => smallvec![*active_resource]
        }
    }

    ///
    /// Returns the resource that this drawing instruction will change
    ///
    /// The active resource is the sprite or the layer that is currently selected for drawing
    ///
    #[inline]
    pub (crate) fn target_resource(&self, active_resource: &DrawResource) -> DrawResource {
        use self::Draw::*;

        match self {
            StartFrame                  |
            ShowFrame                   |
            ResetFrame                  => DrawResource::Frame,

            IdentityTransform           |
            CanvasHeight(_)             |
            CenterRegion(_, _)          |
            MultiplyTransform(_)        => DrawResource::Canvas,

            LineWidth(_)                |
            LineWidthPixels(_)          => DrawResource::StrokeLineWidth,
            LineJoin(_)                 => DrawResource::StrokeLineJoin,
            LineCap(_)                  => DrawResource::StrokeLineCap,
            NewDashPattern              |
            DashLength(_)               |
            DashOffset(_)               => DrawResource::StrokeDash,
            StrokeColor(_)              => DrawResource::StrokeColor,

            WindingRule(_)              => DrawResource::FillWindingRule,
            BlendMode(_)                => DrawResource::FillBlend,
            FillColor(_)                |
            FillTexture(_, _, _)        => DrawResource::FillColor,

            LayerBlend(layer_id, _)     => DrawResource::Layer(*layer_id),
            Font(font_id, _)            => DrawResource::Font(*font_id),
            Texture(texture_id, _)      => DrawResource::Texture(*texture_id),

            // By default, everything affects the active resource
            _                           => *active_resource
        }
    }
}