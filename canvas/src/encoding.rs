use super::draw::*;
use super::color::*;
use super::transform2d::*;

///
/// Trait implemented by objects that can be encoded into a canvas
///
pub trait CanvasEncoding<Buffer> {
    ///
    /// Encodes this item by appending it to the specified string
    ///
    fn encode_canvas(&self, append_to: &mut Buffer);
}

pub (crate) const ENCODING_CHAR_SET: [char; 64] = [
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '+', '/'
];

///
/// Encodes a u64 using a compact representation that is smaller for smaller values
///
#[inline]
fn encode_compact_u64(val: &u64, append_to: &mut String) {
    let mut val = *val;

    for _ in 0..13 {
        let five_bits = (val & 0x1f) as usize;
        let remaining = val >> 5;

        if remaining != 0 {
            let next_char = ENCODING_CHAR_SET[five_bits | 0x20];
            append_to.push(next_char);
        } else {
            let next_char = ENCODING_CHAR_SET[five_bits];
            append_to.push(next_char);
            break;
        }

        val = remaining;
    }
}

impl CanvasEncoding<String> for char {
    #[inline]
    fn encode_canvas(&self, append_to: &mut String) {
        append_to.push(*self)
    }
}

impl CanvasEncoding<String> for u32 {
    #[inline]
    fn encode_canvas(&self, append_to: &mut String) {
        // Base-64 wastes some bits but requires 2 less characters than hex for a 32-bit number
        let mut remaining = *self;

        for _ in 0..6 {
            let next_part = remaining & 0x3f;
            let next_char = ENCODING_CHAR_SET[next_part as usize];
            append_to.push(next_char);

            remaining >>= 6;
        }
    }
}

impl CanvasEncoding<String> for f32 {
    #[inline]
    fn encode_canvas(&self, append_to: &mut String) {
        let transmuted: u32 = f32::to_bits(*self);
        transmuted.encode_canvas(append_to)
    }
}

//
// Some convenience encodings for implementing the main canvas encoding
//


impl<A: CanvasEncoding<String>, B: CanvasEncoding<String>> CanvasEncoding<String> for (A, B) {
    fn encode_canvas(&self, append_to: &mut String) {
        self.0.encode_canvas(append_to);
        self.1.encode_canvas(append_to);
    }
}

impl<A: CanvasEncoding<String>, B: CanvasEncoding<String>, C: CanvasEncoding<String>> CanvasEncoding<String> for (A, B, C) {
    fn encode_canvas(&self, append_to: &mut String) {
        self.0.encode_canvas(append_to);
        self.1.encode_canvas(append_to);
        self.2.encode_canvas(append_to);
    }
}

impl<A: CanvasEncoding<String>, B: CanvasEncoding<String>, C: CanvasEncoding<String>, D: CanvasEncoding<String>> CanvasEncoding<String> for (A, B, C, D) {
    fn encode_canvas(&self, append_to: &mut String) {
        self.0.encode_canvas(append_to);
        self.1.encode_canvas(append_to);
        self.2.encode_canvas(append_to);
        self.3.encode_canvas(append_to);
    }
}

impl<A: CanvasEncoding<String>, B: CanvasEncoding<String>, C: CanvasEncoding<String>, D: CanvasEncoding<String>, E: CanvasEncoding<String>> CanvasEncoding<String> for (A, B, C, D, E) {
    fn encode_canvas(&self, append_to: &mut String) {
        self.0.encode_canvas(append_to);
        self.1.encode_canvas(append_to);
        self.2.encode_canvas(append_to);
        self.3.encode_canvas(append_to);
        self.4.encode_canvas(append_to);
    }
}


impl<A: CanvasEncoding<String>> CanvasEncoding<String> for [A] {
    fn encode_canvas(&self, append_to: &mut String) {
        for component in self.iter() {
            component.encode_canvas(append_to);
        }
    }
}

//
// Main canvas encoding
//

impl CanvasEncoding<String> for Color {
    fn encode_canvas(&self, append_to: &mut String) {
        match self {
            &Color::Rgba(r,g,b,a) => ('R', r, g, b, a),

            other => {
                let (r, g, b, a) = other.to_rgba_components();
                ('R', r, g, b, a)
            }
        }.encode_canvas(append_to)
    }
}

impl CanvasEncoding<String> for LineJoin {
    fn encode_canvas(&self, append_to: &mut String) {
        use self::LineJoin::*;

        match self {
            &Miter => 'M',
            &Round => 'R',
            &Bevel => 'B'
        }.encode_canvas(append_to)
    }
}

impl CanvasEncoding<String> for LineCap {
    fn encode_canvas(&self, append_to: &mut String) {
        use self::LineCap::*;

        match self {
            &Butt   => 'B',
            &Round  => 'R',
            &Square => 'S'
        }.encode_canvas(append_to)
    }
}

impl CanvasEncoding<String> for WindingRule {
    fn encode_canvas(&self, append_to: &mut String) {
        use self::WindingRule::*;

        match self {
            &NonZero => 'n',
            &EvenOdd => 'e'
        }.encode_canvas(append_to)
    }
}

impl CanvasEncoding<String> for BlendMode {
    fn encode_canvas(&self, append_to: &mut String) {
        use self::BlendMode::*;

        match self {
            &SourceOver         => ('S', 'V'),
            &SourceIn           => ('S', 'I'),
            &SourceOut          => ('S', 'O'),
            &DestinationOver    => ('D', 'V'),
            &DestinationIn      => ('D', 'I'),
            &DestinationOut     => ('D', 'O'),
            &SourceAtop         => ('S', 'A'),
            &DestinationAtop    => ('D', 'A'),

            &Multiply           => ('E', 'M'),
            &Screen             => ('E', 'S'),
            &Darken             => ('E', 'D'),
            &Lighten            => ('E', 'L')
        }.encode_canvas(append_to)
    }
}

impl CanvasEncoding<String> for Transform2D {
    #[inline]
    fn encode_canvas(&self, append_to: &mut String) {
        let Transform2D([a, b, c]) = *self;
        a.encode_canvas(append_to);
        b.encode_canvas(append_to);
        c.encode_canvas(append_to);
    }
}

impl CanvasEncoding<String> for LayerId {
    #[inline]
    fn encode_canvas(&self, append_to: &mut String) {
        let LayerId(layer_id) = self;
        encode_compact_u64(layer_id, append_to)
    }
}

impl CanvasEncoding<String> for SpriteId {
    #[inline]
    fn encode_canvas(&self, append_to: &mut String) {
        let SpriteId(sprite_id) = self;
        encode_compact_u64(sprite_id, append_to)
    }
}

impl CanvasEncoding<String> for TextureId {
    #[inline]
    fn encode_canvas(&self, append_to: &mut String) {
        let TextureId(texture_id) = self;
        encode_compact_u64(texture_id, append_to)
    }
}

impl CanvasEncoding<String> for FontId {
    #[inline]
    fn encode_canvas(&self, append_to: &mut String) {
        let FontId(font_id) = self;
        encode_compact_u64(font_id, append_to)
    }
}

impl CanvasEncoding<String> for SpriteTransform {
    fn encode_canvas(&self, append_to: &mut String) {
        use self::SpriteTransform::*;

        match self {
            Identity                => 'i'.encode_canvas(append_to),
            Translate(x, y)         => ('t', *x, *y).encode_canvas(append_to),
            Scale(x, y)             => ('s', *x, *y).encode_canvas(append_to),
            Rotate(degrees)         => ('r', *degrees).encode_canvas(append_to),
            Transform2D(transform)  => ('T', *transform).encode_canvas(append_to)
        }
    }
}

impl<'a> CanvasEncoding<String> for &'a TextureOp {
    fn encode_canvas(&self, append_to: &mut String) {
        use self::TextureOp::*;

        match self {
            _ => { }
        }
    }
}

impl<'a> CanvasEncoding<String> for &'a FontOp {
    fn encode_canvas(&self, append_to: &mut String) {
        use self::FontOp::*;

        match self {
            _ => { }
        }
    }
}

impl CanvasEncoding<String> for Draw {
    fn encode_canvas(&self, append_to: &mut String) {
        use self::Draw::*;

        match self {
            &NewPath                                 => ('N', 'p').encode_canvas(append_to),
            &Move(x, y)                              => ('m', x, y).encode_canvas(append_to),
            &Line(x, y)                              => ('l', x, y).encode_canvas(append_to),
            &BezierCurve(p1, p2, p3)                 => ('c', p1, p2, p3).encode_canvas(append_to),
            &ClosePath                               => ('.').encode_canvas(append_to),
            &Fill                                    => 'F'.encode_canvas(append_to),
            &Stroke                                  => 'S'.encode_canvas(append_to),
            &LineWidth(width)                        => ('L', 'w', width).encode_canvas(append_to),
            &LineWidthPixels(width)                  => ('L', 'p', width).encode_canvas(append_to),
            &LineJoin(join)                          => ('L', 'j', join).encode_canvas(append_to),
            &LineCap(cap)                            => ('L', 'c', cap).encode_canvas(append_to),
            &WindingRule(rule)                       => ('W', rule).encode_canvas(append_to),
            &NewDashPattern                          => ('D', 'n').encode_canvas(append_to),
            &DashLength(length)                      => ('D', 'l', length).encode_canvas(append_to),
            &DashOffset(offset)                      => ('D', 'o', offset).encode_canvas(append_to),
            &StrokeColor(col)                        => ('C', 's', col).encode_canvas(append_to),
            &FillColor(col)                          => ('C', 'f', col).encode_canvas(append_to),
            &BlendMode(mode)                         => ('M', mode).encode_canvas(append_to),
            &IdentityTransform                       => ('T', 'i').encode_canvas(append_to),
            &CanvasHeight(height)                    => ('T', 'h', height).encode_canvas(append_to),
            &CenterRegion(min, max)                  => ('T', 'c', min, max).encode_canvas(append_to),
            &MultiplyTransform(transform)            => ('T', 'm', transform).encode_canvas(append_to),
            &Unclip                                  => ('Z', 'n').encode_canvas(append_to),
            &Clip                                    => ('Z', 'c').encode_canvas(append_to),
            &Store                                   => ('Z', 's').encode_canvas(append_to),
            &Restore                                 => ('Z', 'r').encode_canvas(append_to),
            &FreeStoredBuffer                        => ('Z', 'f').encode_canvas(append_to),
            &PushState                               => 'P'.encode_canvas(append_to),
            &PopState                                => 'p'.encode_canvas(append_to),
            &ClearCanvas(color)                      => ('N', 'A', color).encode_canvas(append_to),
            &Layer(layer_id)                         => ('N', 'L', layer_id).encode_canvas(append_to),
            &LayerBlend(layer_id, blend_mode)        => ('N', 'B', layer_id, blend_mode).encode_canvas(append_to),
            &ClearLayer                              => ('N', 'C').encode_canvas(append_to),
            &Sprite(sprite_id)                       => ('N', 's', sprite_id).encode_canvas(append_to),
            &ClearSprite                             => ('s', 'C').encode_canvas(append_to),
            &SpriteTransform(sprite_transform)       => ('s', 'T', sprite_transform).encode_canvas(append_to),
            &DrawSprite(sprite_id)                   => ('s', 'D', sprite_id).encode_canvas(append_to),
            &Texture(texture_id, ref op)             => ('T', texture_id, op).encode_canvas(append_to),
            &Font(font_id, ref op)                   => ('F', font_id, op).encode_canvas(append_to)
        }
    }
}

impl CanvasEncoding<String> for Vec<Draw> {
    fn encode_canvas(&self, append_to: &mut String) {
        self.iter().for_each(|item| { item.encode_canvas(append_to); append_to.push('\n'); });
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn encode_u32() {
        let test_number: u32 = 0xabcd1234;

        let mut encoded = String::new();
        test_number.encode_canvas(&mut encoded);

        assert!(encoded == "0IRzrC".to_string());
    }

    #[test]
    fn encode_f32() {
        let test_number: f32 = 3.141;

        let mut encoded = String::new();
        test_number.encode_canvas(&mut encoded);

        assert!(encoded == "lYQSAB".to_string());
    }

    fn encode_draw(item: Draw) -> String {
        let mut result = String::new();
        item.encode_canvas(&mut result);
        result
    }

    #[test]
    fn encode_newpath() { assert!(&encode_draw(Draw::NewPath) == "Np") }
    #[test]
    fn encode_move() { assert!(&encode_draw(Draw::Move(20.0, 20.0)) == "mAAAoBBAAAoBB") }
    #[test]
    fn encode_line() { assert!(&encode_draw(Draw::Line(20.0, 20.0)) == "lAAAoBBAAAoBB") }
    #[test]
    fn encode_bezier() { assert!(&encode_draw(Draw::BezierCurve((20.0, 20.0), (20.0, 20.0), (20.0, 20.0))) == "cAAAoBBAAAoBBAAAoBBAAAoBBAAAoBBAAAoBB") }
    #[test]
    fn encode_close_path() { assert!(&encode_draw(Draw::ClosePath) == ".") }
    #[test]
    fn encode_fill() { assert!(&encode_draw(Draw::Fill) == "F") }
    #[test]
    fn encode_stroke() { assert!(&encode_draw(Draw::Stroke) == "S") }
    #[test]
    fn encode_linewidth() { assert!(&encode_draw(Draw::LineWidth(20.0)) == "LwAAAoBB") }
    #[test]
    fn encode_linewidthpixels() { assert!(&encode_draw(Draw::LineWidthPixels(20.0)) == "LpAAAoBB") }
    #[test]
    fn encode_linejoin() { assert!(&encode_draw(Draw::LineJoin(LineJoin::Bevel)) == "LjB") }
    #[test]
    fn encode_linecap() { assert!(&encode_draw(Draw::LineCap(LineCap::Butt)) == "LcB") }
    #[test]
    fn encode_newdashpattern() { assert!(&encode_draw(Draw::NewDashPattern) == "Dn") }
    #[test]
    fn encode_dashlength() { assert!(&encode_draw(Draw::DashLength(20.0)) == "DlAAAoBB") }
    #[test]
    fn encode_dashoffset() { assert!(&encode_draw(Draw::DashOffset(20.0)) == "DoAAAoBB") }
    #[test]
    fn encode_strokecolor() { assert!(&encode_draw(Draw::StrokeColor(Color::Rgba(1.0, 1.0, 1.0, 1.0))) == "CsRAAAg/AAAAg/AAAAg/AAAAg/A") }
    #[test]
    fn encode_fillcolor() { assert!(&encode_draw(Draw::FillColor(Color::Rgba(1.0, 1.0, 1.0, 1.0))) == "CfRAAAg/AAAAg/AAAAg/AAAAg/A") }
    #[test]
    fn encode_blendmode() { assert!(&encode_draw(Draw::BlendMode(BlendMode::SourceOver)) == "MSV") }
    #[test]
    fn encode_identity_transform() { assert!(&encode_draw(Draw::IdentityTransform) == "Ti") }
    #[test]
    fn encode_canvas_height() { assert!(&encode_draw(Draw::CanvasHeight(20.0)) == "ThAAAoBB") }
    #[test]
    fn encode_multiply_transform() { assert!(&encode_draw(Draw::MultiplyTransform(Transform2D([[1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 0.0]]))) == "TmAAAg/AAAAAAAAAAAAAAAAg/AAAAAAAAAAAAAAAAg/AAAAAAAAAAAAA") }
    #[test]
    fn encode_unclip() { assert!(&encode_draw(Draw::Unclip) == "Zn") }
    #[test]
    fn encode_clip() { assert!(&encode_draw(Draw::Clip) == "Zc") }
    #[test]
    fn encode_store() { assert!(&encode_draw(Draw::Store) == "Zs") }
    #[test]
    fn encode_restore() { assert!(&encode_draw(Draw::Restore) == "Zr") }
    #[test]
    fn encode_pushstate() { assert!(&encode_draw(Draw::PushState) == "P") }
    #[test]
    fn encode_popstate() { assert!(&encode_draw(Draw::PopState) == "p") }
    #[test]
    fn encode_clearcanvas() { assert!(&encode_draw(Draw::ClearCanvas(Color::Rgba(1.0, 1.0, 1.0, 1.0))) == "NARAAAg/AAAAg/AAAAg/AAAAg/A") }
    #[test]
    fn encode_layer() { assert!(&encode_draw(Draw::Layer(LayerId(2))) == "NLC") }
    #[test]
    fn encode_layer_blend() { assert!(&encode_draw(Draw::LayerBlend(LayerId(2), BlendMode::Screen)) == "NBCES") }
    #[test]
    fn encode_clearlayer() { assert!(&encode_draw(Draw::ClearLayer) == "NC") }
    #[test]
    fn encode_nonzero_winding_rule() { assert!(&encode_draw(Draw::WindingRule(WindingRule::NonZero)) == "Wn") }
    #[test]
    fn encode_evenodd_winding_rule() { assert!(&encode_draw(Draw::WindingRule(WindingRule::EvenOdd)) == "We") }
}
