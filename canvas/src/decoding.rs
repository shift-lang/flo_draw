/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::{
    color::*, draw::*, font::*, font_face::*, gradient::*, namespace::*, path::*, sprite::*,
    texture::*, transform2d::*,
};

use futures::task::Poll;
use futures::*;

use itertools::*;
use uuid::*;

use std::mem;
use std::result::Result;
use std::str::*;
use std::sync::*;

///
/// Represents a partial or full decoding result
///
#[derive(Debug)]
enum PartialResult<T> {
    MatchMore(String),
    FullMatch(T),
}

impl<T> PartialResult<T> {
    pub fn new() -> PartialResult<T> {
        PartialResult::MatchMore(String::new())
    }

    pub fn match_more(self) -> Result<String, DecoderError> {
        match self {
            PartialResult::MatchMore(data) => Ok(data),
            PartialResult::FullMatch(_) => Err(DecoderError::UnexpectedlyComplete),
        }
    }

    pub fn map<TFn: FnOnce(T) -> S, S>(self, map_fn: TFn) -> PartialResult<S> {
        match self {
            PartialResult::FullMatch(result) => PartialResult::FullMatch(map_fn(result)),
            PartialResult::MatchMore(data) => PartialResult::MatchMore(data),
        }
    }
}

///
/// Represents the state of an operation decoding a string value
///
struct DecodeString {
    length: PartialResult<u64>,
    string_encoding: PartialResult<String>,
}

///
/// Represents the state of an operation decoding a list of glyph positions
///
struct DecodeGlyphPositions {
    length: PartialResult<u64>,
    glyphs: PartialResult<Vec<GlyphPosition>>,
}

///
/// Represents the state of an operation decoding a set of bytes
///
struct DecodeBytes {
    length: PartialResult<u64>,
    byte_encoding: PartialResult<Vec<u8>>,
}

type DecodeLayerId = PartialResult<LayerId>;
type DecodeFontId = PartialResult<FontId>;
type DecodeSpriteId = PartialResult<SpriteId>;
type DecodeTextureId = PartialResult<TextureId>;
type DecodeGradientId = PartialResult<GradientId>;

impl DecodeString {
    ///
    /// Creates a new string decoder that has matched 0 characters
    ///
    fn new() -> DecodeString {
        DecodeString {
            length: PartialResult::new(),
            string_encoding: PartialResult::new(),
        }
    }

    ///
    /// Indicates if this string decoder is ready or not
    ///
    #[inline]
    fn ready(&self) -> bool {
        match (&self.length, &self.string_encoding) {
            (PartialResult::FullMatch(_), PartialResult::FullMatch(_)) => true,
            _ => false,
        }
    }

    ///
    /// Returns the string matched by this decoder (once it's ready)
    ///
    #[inline]
    fn to_string(self) -> Result<String, DecoderError> {
        match self.string_encoding {
            PartialResult::FullMatch(string) => Ok(string),
            PartialResult::MatchMore(_) => Err(DecoderError::NotReady),
        }
    }

    ///
    /// Decodes a single character and returns the new state of the decoder
    ///
    fn decode(mut self, chr: char) -> Result<DecodeString, DecoderError> {
        // Decode or fetch the length of the string
        let length = match self.length {
            PartialResult::MatchMore(so_far) => {
                self.length = CanvasDecoder::decode_compact_id(chr, so_far)?;

                if let &PartialResult::FullMatch(0) = &self.length {
                    self.string_encoding = PartialResult::FullMatch(String::new());
                }
                return Ok(self);
            }

            PartialResult::FullMatch(length) => {
                self.length = PartialResult::FullMatch(length);
                length as usize
            }
        };

        // Try to decode the rest of the string
        match self.string_encoding {
            PartialResult::FullMatch(string) => {
                // Nothing to do
                self.string_encoding = PartialResult::FullMatch(string);
            }

            PartialResult::MatchMore(mut string) => {
                string.push(chr);

                if string.len() >= length {
                    self.string_encoding = PartialResult::FullMatch(string);
                } else {
                    self.string_encoding = PartialResult::MatchMore(string);
                }
            }
        }

        Ok(self)
    }
}

impl DecodeBytes {
    ///
    /// Creates a new byte string decoder
    ///
    fn new() -> DecodeBytes {
        DecodeBytes {
            length: PartialResult::new(),
            byte_encoding: PartialResult::new(),
        }
    }

    ///
    /// Indicates if this string decoder is ready or not
    ///
    #[inline]
    fn ready(&self) -> bool {
        match (&self.length, &self.byte_encoding) {
            (PartialResult::FullMatch(_), PartialResult::FullMatch(_)) => true,
            _ => false,
        }
    }

    ///
    /// Returns the string matched by this decoder (once it's ready)
    ///
    #[inline]
    fn to_bytes(self) -> Result<Vec<u8>, DecoderError> {
        match self.byte_encoding {
            PartialResult::FullMatch(bytes) => Ok(bytes),
            PartialResult::MatchMore(_) => Err(DecoderError::NotReady),
        }
    }

    ///
    /// Decodes a single character and returns the new state of the decoder
    ///
    fn decode(mut self, chr: char) -> Result<DecodeBytes, DecoderError> {
        use PartialResult::*;

        // Decode or fetch the length of the bytes
        let length = match self.length {
            MatchMore(so_far) => {
                self.length = CanvasDecoder::decode_compact_id(chr, so_far)?;

                if let &FullMatch(0) = &self.length {
                    self.byte_encoding = FullMatch(vec![]);
                }
                return Ok(self);
            }

            FullMatch(length) => {
                self.length = PartialResult::FullMatch(length);
                length as usize
            }
        };

        // Try to decode the rest of the string
        match self.byte_encoding {
            FullMatch(_) => {
                // Already finished matching
                return Err(DecoderError::NotReady);
            }

            MatchMore(mut encoded_bytes) => {
                encoded_bytes.push(chr);

                // Every 4 encoded bytes makes up 3 output bytes (and the encoding rounds up overall)
                let encoded_length = (encoded_bytes.len() / 4) * 3;

                if encoded_length >= length {
                    // Decode the bytes
                    let mut decoded_bytes = vec![];

                    // 4 characters decode into 3 bytes
                    for (a, b, c, d) in encoded_bytes.chars().tuples() {
                        // Decode to 6-bit values
                        let a = CanvasDecoder::decode_base64(a)?;
                        let b = CanvasDecoder::decode_base64(b)?;
                        let c = CanvasDecoder::decode_base64(c)?;
                        let d = CanvasDecoder::decode_base64(d)?;

                        // Decode to 8-bit values
                        let a = a | ((b << 6) & 0xff);
                        let b = (b >> 2) | ((c << 4) & 0xff);
                        let c = (c >> 4) | ((d << 2) & 0xff);

                        // Add to the result
                        decoded_bytes.push(a);
                        decoded_bytes.push(b);
                        decoded_bytes.push(c);
                    }

                    // Trim to the actual required length
                    decoded_bytes.truncate(length);

                    // Have decoded the bytes for this object
                    self.byte_encoding = FullMatch(decoded_bytes);
                } else {
                    // Not enough bytes to make up the full string yet
                    self.byte_encoding = MatchMore(encoded_bytes);
                }

                return Ok(self);
            }
        }
    }
}

impl DecodeGlyphPositions {
    ///
    /// Creates a new string decoder that has matched 0 characters
    ///
    fn new() -> DecodeGlyphPositions {
        DecodeGlyphPositions {
            length: PartialResult::new(),
            glyphs: PartialResult::new(),
        }
    }

    ///
    /// Indicates if this string decoder is ready or not
    ///
    #[inline]
    fn ready(&self) -> bool {
        match (&self.length, &self.glyphs) {
            (PartialResult::FullMatch(_), PartialResult::FullMatch(_)) => true,
            _ => false,
        }
    }

    ///
    /// Returns the string matched by this decoder (once it's ready)
    ///
    #[inline]
    fn to_glyphs(self) -> Result<Vec<GlyphPosition>, DecoderError> {
        match self.glyphs {
            PartialResult::FullMatch(glyphs) => Ok(glyphs),
            PartialResult::MatchMore(_) => Err(DecoderError::NotReady),
        }
    }

    ///
    /// Decodes a single character and returns the new state of the decoder
    ///
    fn decode(mut self, chr: char) -> Result<DecodeGlyphPositions, DecoderError> {
        // Decode or fetch the length of the string
        let length = match self.length {
            PartialResult::MatchMore(so_far) => {
                self.length = CanvasDecoder::decode_compact_id(chr, so_far)?;

                if let &PartialResult::FullMatch(0) = &self.length {
                    self.glyphs = PartialResult::FullMatch(vec![]);
                }
                return Ok(self);
            }

            PartialResult::FullMatch(length) => {
                self.length = PartialResult::FullMatch(length);
                length as usize
            }
        };

        // Try to decode the rest of the string
        match self.glyphs {
            PartialResult::FullMatch(glyphs) => {
                // Nothing to do
                self.glyphs = PartialResult::FullMatch(glyphs);
            }

            PartialResult::MatchMore(mut string) => {
                string.push(chr);

                if string.len() >= length * 24 {
                    let mut chrs = string.chars();
                    let mut glyphs = vec![];

                    // Each glyph consists of a glyph ID, an x and y coord and a em_size
                    for _ in 0..length {
                        let id = CanvasDecoder::decode_u32(&mut chrs)?;
                        let x = CanvasDecoder::decode_f32(&mut chrs)?;
                        let y = CanvasDecoder::decode_f32(&mut chrs)?;
                        let em_size = CanvasDecoder::decode_f32(&mut chrs)?;

                        glyphs.push(GlyphPosition {
                            id: GlyphId(id),
                            location: (x, y),
                            em_size: em_size,
                        });
                    }

                    self.glyphs = PartialResult::FullMatch(glyphs);
                } else {
                    self.glyphs = PartialResult::MatchMore(string);
                }
            }
        }

        Ok(self)
    }
}

///
/// The possible states for a decoder to be in after accepting some characters from the source
///
enum DecoderState {
    None,
    Error,

    New,
    // 'N'
    LineStyle,
    // 'L'
    Dash,
    // 'D'
    Color,
    // 'C'
    Sprite,
    // 's'
    Transform,
    // 'T'
    State, // 'Z'

    ClearCanvas(String), // 'NA' (r, g, b, a)

    Move(String),
    // m (x, y)
    Line(String),
    // l (x, y)
    BezierCurve(String), // c (x, y, x, y, x, y)

    LineStyleWidth(String),
    // 'Lw' (w)
    LineStyleWidthPixels(String),
    // 'Lp' (w)
    LineStyleJoin(String),
    // 'Lj' (j)
    LineStyleCap(String),
    // 'Lc' (c)
    WindingRule, // 'W' (r)

    DashLength(String),
    // 'Dl' (len)
    DashOffset(String), // 'Do' (offset)

    ColorStroke(String),
    // 'Cs' (r, g, b, a)
    ColorFill(String),
    // 'Cf' (r, g, b, a)
    ColorTexture(DecodeTextureId, String),
    // 'Ct' (texture_id, x1, y1, x2, y2)
    ColorGradient(DecodeGradientId, String),
    // 'Cg' (gradient_id, x1, y1, x2, y2)
    ColorTransform(String), // 'CT' (transform)

    BlendMode(String), // 'M' (mode)

    TransformHeight(String),
    // 'Th' (h)
    TransformCenter(String),
    // 'Tc' (min, max)
    TransformMultiply(String), // 'Tm' (transform)

    NewLayerU32(String),
    // 'Nl' (id)
    NewLayerBlendU32(String),
    // 'Nb' (id, mode)
    NewLayer(String),
    // 'NL' (id)
    NewLayerBlend(DecodeLayerId, String),
    // 'NB' (id, mode)
    NewLayerAlpha(DecodeLayerId, String),
    // 'Nt' (id, alpha)
    SwapLayers(Option<LayerId>, String), // 'NX' (layer1, layer2)

    NewSprite(String),
    // 'Ns' (id)
    SpriteDraw(String),
    // 'sD' (id)
    SpriteDrawWithFilters(String),
    // 'sF' (id) (len) (filters)
    SpriteDrawWithFiltersId(SpriteId, String),
    // 'sF' (id) (len) (filters)
    SpriteMoveFrom(String),
    // 'sm' (id)
    SpriteTransform,
    // 'sT' (transform)
    SpriteTransformTranslate(String),
    // 'sTt' (x, y)
    SpriteTransformScale(String),
    // 'sTs' (x, y)
    SpriteTransformRotate(String),
    // 'sTr' (degr ees)
    SpriteTransformTransform(String), // 'sTT' (transform)

    NewNamespace(String), // 'NN' (GUID as two u64s)

    FontDrawing,
    // 't'
    FontDrawText(DecodeFontId, DecodeString, String),
    // 'tT' (font_id, string, x, y)
    FontBeginLayout(String), // 'tl' (x, y, align)

    FontOp(DecodeFontId),
    // 'f' (id, op)
    FontOpSize(FontId, String),
    // 'f<id>S' (size)
    FontOpData(FontId),
    // 'f<id>d'
    FontOpTtf(FontId, DecodeBytes),
    // 'f<id>dT' (bytes)
    FontOpLayoutText(FontId, DecodeString),
    // 'f<id>L' (string)
    FontOpDrawGlyphs(FontId, DecodeGlyphPositions), // 'f<id>G' (glyph positions)

    TextureOp(DecodeTextureId),
    // 'B<id>' (id, op)
    TextureOpCreate(TextureId, String),
    // 'B<id>N' (w, h, format)
    TextureOpSetBytes(TextureId, String, DecodeBytes),
    // 'B<id>D' (x, y, w, h, bytes)
    TextureOpSetFromSprite(TextureId, DecodeSpriteId, String),
    // 'B<id>S' (sprite, x, y, w, h)
    TextureOpCreateDynamicSprite(TextureId, DecodeSpriteId, String),
    // 'B<id>s' (sprite, x, y, w1, h1, w2, h2)
    TextureOpFillTransparency(TextureId, String),
    // 'B<id>t' (alpha)
    TextureOpCopy(TextureId, DecodeTextureId),
    // 'B<id>C' (texture)
    TextureOpFilter(TextureId, String), // 'B<id>F' (filter)

    GradientOp(DecodeGradientId),
    // 'G' (id, op)
    GradientOpNew(GradientId, String),
    // 'G<id>N' (r, g, b, a)
    GradientOpAddStop(GradientId, String), // 'G<id>S' (pos, r, g, b, a)
}

///
/// Possible error from the decoder
///
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DecoderError {
    /// The character was not valid for the current state of the decoder
    InvalidCharacter(char),

    /// The decoder tried to decode something before it had accepted all characters (probably a bug)
    MissingCharacter,

    /// A number could not be parsed for some reason
    BadNumber,

    /// A color had an unknown type
    UnknownColorType,

    /// The decoder previously encountered an error and cannot continue
    IsInErrorState,

    /// The decoder was asked for a result when it was not ready (usually indicates an internal bug)
    NotReady,

    /// The decoder was expecting a state as a partial match but it was completed
    UnexpectedlyComplete,
}

///
/// Represents a (stateful) canvas decoder
///
pub struct CanvasDecoder {
    state: DecoderState,
}

impl CanvasDecoder {
    ///
    /// Creates a new canvas decoder
    ///
    pub fn new() -> CanvasDecoder {
        CanvasDecoder {
            state: DecoderState::None,
        }
    }

    ///
    /// Decodes a character, returning the next Draw operation if there is one
    ///
    pub fn decode(&mut self, next_chr: char) -> Result<Option<Draw>, DecoderError> {
        use self::DecoderState::*;

        // Next state depends on the character and the current state
        let mut state = DecoderState::Error;
        mem::swap(&mut self.state, &mut state);

        let (next_state, result) = match state {
            None => Self::decode_none(next_chr)?,
            Error => Err(DecoderError::IsInErrorState)?,

            New => Self::decode_new(next_chr)?,
            LineStyle => Self::decode_line_style(next_chr)?,
            Dash => Self::decode_dash(next_chr)?,
            Color => Self::decode_color(next_chr)?,
            Sprite => Self::decode_sprite(next_chr)?,
            Transform => Self::decode_transform(next_chr)?,
            State => Self::decode_state(next_chr)?,

            Move(param) => Self::decode_move(next_chr, param)?,
            Line(param) => Self::decode_line(next_chr, param)?,
            BezierCurve(param) => Self::decode_bezier_curve(next_chr, param)?,

            LineStyleWidth(param) => Self::decode_line_width(next_chr, param)?,
            LineStyleWidthPixels(param) => Self::decode_line_width_pixels(next_chr, param)?,
            LineStyleJoin(param) => Self::decode_line_style_join(next_chr, param)?,
            LineStyleCap(param) => Self::decode_line_style_cap(next_chr, param)?,
            WindingRule => Self::decode_winding_rule(next_chr)?,

            DashLength(param) => Self::decode_dash_length(next_chr, param)?,
            DashOffset(param) => Self::decode_dash_offset(next_chr, param)?,

            ClearCanvas(param) => Self::decode_clear_canvas(next_chr, param)?,

            ColorStroke(param) => Self::decode_color_stroke(next_chr, param)?,
            ColorFill(param) => Self::decode_color_fill(next_chr, param)?,
            ColorTexture(id, param) => Self::decode_color_texture(next_chr, id, param)?,
            ColorGradient(id, param) => Self::decode_color_gradient(next_chr, id, param)?,
            ColorTransform(param) => Self::decode_color_transform(next_chr, param)?,

            BlendMode(param) => Self::decode_blend_mode(next_chr, param)?,

            TransformHeight(param) => Self::decode_transform_height(next_chr, param)?,
            TransformCenter(param) => Self::decode_transform_center(next_chr, param)?,
            TransformMultiply(param) => Self::decode_transform_multiply(next_chr, param)?,

            NewLayerU32(param) => Self::decode_new_layer_u32(next_chr, param)?,
            NewLayerBlendU32(param) => Self::decode_new_layer_blend_u32(next_chr, param)?,
            NewLayer(param) => Self::decode_new_layer(next_chr, param)?,
            NewLayerBlend(layer, blend) => Self::decode_new_layer_blend(next_chr, layer, blend)?,
            NewLayerAlpha(layer, alpha) => Self::decode_new_layer_alpha(next_chr, layer, alpha)?,
            SwapLayers(layer1, param) => Self::decode_swap_layers(next_chr, layer1, param)?,

            NewSprite(param) => Self::decode_new_sprite(next_chr, param)?,
            SpriteDraw(param) => Self::decode_sprite_draw(next_chr, param)?,
            SpriteDrawWithFilters(param) => Self::decode_sprite_draw_with_filters(next_chr, param)?,
            SpriteDrawWithFiltersId(id, param) => {
                Self::decode_sprite_draw_with_filters_id(next_chr, id, param)?
            }
            SpriteMoveFrom(param) => Self::decode_sprite_move_from(next_chr, param)?,
            SpriteTransform => Self::decode_sprite_transform(next_chr)?,
            SpriteTransformTranslate(param) => {
                Self::decode_sprite_transform_translate(next_chr, param)?
            }
            SpriteTransformScale(param) => Self::decode_sprite_transform_scale(next_chr, param)?,
            SpriteTransformRotate(param) => Self::decode_sprite_transform_rotate(next_chr, param)?,
            SpriteTransformTransform(param) => {
                Self::decode_sprite_transform_transform(next_chr, param)?
            }

            NewNamespace(param) => Self::decode_namespace(next_chr, param)?,

            FontDrawing => Self::decode_font_drawing(next_chr)?,
            FontDrawText(font_id, string_decode, coords) => {
                Self::decode_font_draw_text(next_chr, font_id, string_decode, coords)?
            }
            FontBeginLayout(param) => Self::decode_font_begin_layout(next_chr, param)?,

            FontOp(font_id) => Self::decode_font_op(next_chr, font_id)?,
            FontOpSize(font_id, size) => Self::decode_font_op_size(next_chr, font_id, size)?,
            FontOpData(font_id) => Self::decode_font_op_data(next_chr, font_id)?,
            FontOpTtf(font_id, bytes) => Self::decode_font_data_ttf(next_chr, font_id, bytes)?,
            FontOpLayoutText(font_id, string) => {
                Self::decode_font_op_layout(next_chr, font_id, string)?
            }
            FontOpDrawGlyphs(font_id, glyphs) => {
                Self::decode_font_op_glyphs(next_chr, font_id, glyphs)?
            }

            TextureOp(texture_id) => Self::decode_texture_op(next_chr, texture_id)?,
            TextureOpCreate(texture_id, param) => {
                Self::decode_texture_create(next_chr, texture_id, param)?
            }
            TextureOpSetBytes(texture_id, param, bytes) => {
                Self::decode_texture_set_bytes(next_chr, texture_id, param, bytes)?
            }
            TextureOpSetFromSprite(texture_id, sprite, param) => {
                Self::decode_texture_set_from_sprite(next_chr, texture_id, sprite, param)?
            }
            TextureOpCreateDynamicSprite(texture_id, sprite, param) => {
                Self::decode_texture_create_dynamic_sprite(next_chr, texture_id, sprite, param)?
            }
            TextureOpFillTransparency(texture_id, param) => {
                Self::decode_texture_fill_transparency(next_chr, texture_id, param)?
            }
            TextureOpCopy(texture_id, param) => {
                Self::decode_texture_copy(next_chr, texture_id, param)?
            }
            TextureOpFilter(texture_id, param) => {
                Self::decode_texture_filter(next_chr, texture_id, param)?
            }

            GradientOp(gradient_id) => Self::decode_gradient_op(next_chr, gradient_id)?,
            GradientOpNew(gradient_id, param) => {
                Self::decode_gradient_new(next_chr, gradient_id, param)?
            }
            GradientOpAddStop(gradient_id, param) => {
                Self::decode_gradient_add_stop(next_chr, gradient_id, param)?
            }
        };

        self.state = next_state;
        Ok(result)
    }

    ///
    /// Matches the first character of a canvas item
    ///
    #[inline]
    fn decode_none(next_chr: char) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        match next_chr {
            // Whitespace ignored if we're not parsing a command
            '\n' | '\r' | ' ' => Ok((DecoderState::None, None)),

            // Multi-character commands
            'N' => Ok((DecoderState::New, None)),
            'L' => Ok((DecoderState::LineStyle, None)),
            'D' => Ok((DecoderState::Dash, None)),
            'C' => Ok((DecoderState::Color, None)),
            's' => Ok((DecoderState::Sprite, None)),
            'T' => Ok((DecoderState::Transform, None)),
            'Z' => Ok((DecoderState::State, None)),
            'W' => Ok((DecoderState::WindingRule, None)),

            // Single character commands
            '.' => Ok((DecoderState::None, Some(Draw::Path(PathOp::ClosePath)))),
            'F' => Ok((DecoderState::None, Some(Draw::Fill))),
            'S' => Ok((DecoderState::None, Some(Draw::Stroke))),
            'P' => Ok((DecoderState::None, Some(Draw::PushState))),
            'p' => Ok((DecoderState::None, Some(Draw::PopState))),

            // Single character commands with a parameter
            'm' => Ok((DecoderState::Move(String::new()), None)),
            'l' => Ok((DecoderState::Line(String::new()), None)),
            'c' => Ok((DecoderState::BezierCurve(String::new()), None)),
            'M' => Ok((DecoderState::BlendMode(String::new()), None)),

            't' => Ok((DecoderState::FontDrawing, None)),
            'f' => Ok((
                DecoderState::FontOp(PartialResult::MatchMore(String::new())),
                None,
            )),

            'B' => Ok((DecoderState::TextureOp(PartialResult::new()), None)),

            'G' => Ok((DecoderState::GradientOp(PartialResult::new()), None)),

            // Other characters are not accepted
            _ => Err(DecoderError::InvalidCharacter(next_chr)),
        }
    }

    #[inline]
    fn decode_new(next_chr: char) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        // Matched 'N' so far
        match next_chr {
            'p' => Ok((DecoderState::None, Some(Draw::Path(PathOp::NewPath)))),
            'A' => Ok((DecoderState::ClearCanvas(String::new()), None)),
            'a' => Ok((DecoderState::None, Some(Draw::ClearAllLayers))),
            'C' => Ok((DecoderState::None, Some(Draw::ClearLayer))),

            'l' => Ok((DecoderState::NewLayerU32(String::new()), None)),
            'b' => Ok((DecoderState::NewLayerBlendU32(String::new()), None)),
            'L' => Ok((DecoderState::NewLayer(String::new()), None)),
            'B' => Ok((
                DecoderState::NewLayerBlend(PartialResult::MatchMore(String::new()), String::new()),
                None,
            )),
            't' => Ok((
                DecoderState::NewLayerAlpha(PartialResult::MatchMore(String::new()), String::new()),
                None,
            )),
            'X' => Ok((DecoderState::SwapLayers(None, String::new()), None)),
            's' => Ok((DecoderState::NewSprite(String::new()), None)),
            'N' => Ok((DecoderState::NewNamespace(String::new()), None)),

            'F' => Ok((DecoderState::None, Some(Draw::StartFrame))),
            'f' => Ok((DecoderState::None, Some(Draw::ShowFrame))),
            'G' => Ok((DecoderState::None, Some(Draw::ResetFrame))),

            _ => Err(DecoderError::InvalidCharacter(next_chr)),
        }
    }

    #[inline]
    fn decode_line_style(next_chr: char) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        // Matched 'L' so far
        match next_chr {
            'w' => Ok((DecoderState::LineStyleWidth(String::new()), None)),
            'p' => Ok((DecoderState::LineStyleWidthPixels(String::new()), None)),
            'j' => Ok((DecoderState::LineStyleJoin(String::new()), None)),
            'c' => Ok((DecoderState::LineStyleCap(String::new()), None)),

            _ => Err(DecoderError::InvalidCharacter(next_chr)),
        }
    }

    #[inline]
    fn decode_dash(next_chr: char) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        // Matched 'D' so far
        match next_chr {
            'n' => Ok((DecoderState::None, Some(Draw::NewDashPattern))),

            'l' => Ok((DecoderState::DashLength(String::new()), None)),
            'o' => Ok((DecoderState::DashOffset(String::new()), None)),

            _ => Err(DecoderError::InvalidCharacter(next_chr)),
        }
    }

    #[inline]
    fn decode_color(next_chr: char) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        // Matched 'C' so far
        match next_chr {
            's' => Ok((DecoderState::ColorStroke(String::new()), None)),
            'f' => Ok((DecoderState::ColorFill(String::new()), None)),
            't' => Ok((
                DecoderState::ColorTexture(DecodeTextureId::new(), String::new()),
                None,
            )),
            'g' => Ok((
                DecoderState::ColorGradient(DecodeGradientId::new(), String::new()),
                None,
            )),
            'T' => Ok((DecoderState::ColorTransform(String::new()), None)),

            _ => Err(DecoderError::InvalidCharacter(next_chr)),
        }
    }

    #[inline]
    fn decode_sprite(next_chr: char) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        // Matched 's' so far
        match next_chr {
            'D' => Ok((DecoderState::SpriteDraw(String::new()), None)),
            'F' => Ok((DecoderState::SpriteDrawWithFilters(String::new()), None)),
            'C' => Ok((DecoderState::None, Some(Draw::ClearSprite))),
            'T' => Ok((DecoderState::SpriteTransform, None)),
            'm' => Ok((DecoderState::SpriteMoveFrom(String::new()), None)),

            _ => Err(DecoderError::InvalidCharacter(next_chr)),
        }
    }

    #[inline]
    fn decode_transform(next_chr: char) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        // Matched 'T' so far
        match next_chr {
            'i' => Ok((DecoderState::None, Some(Draw::IdentityTransform))),
            'h' => Ok((DecoderState::TransformHeight(String::new()), None)),
            'c' => Ok((DecoderState::TransformCenter(String::new()), None)),
            'm' => Ok((DecoderState::TransformMultiply(String::new()), None)),

            _ => Err(DecoderError::InvalidCharacter(next_chr)),
        }
    }

    #[inline]
    fn decode_state(next_chr: char) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        // Matched 'Z' so far
        match next_chr {
            'n' => Ok((DecoderState::None, Some(Draw::Unclip))),
            'c' => Ok((DecoderState::None, Some(Draw::Clip))),
            's' => Ok((DecoderState::None, Some(Draw::Store))),
            'r' => Ok((DecoderState::None, Some(Draw::Restore))),
            'f' => Ok((DecoderState::None, Some(Draw::FreeStoredBuffer))),

            _ => Err(DecoderError::InvalidCharacter(next_chr)),
        }
    }

    #[inline]
    fn decode_line_width_pixels(
        next_chr: char,
        mut param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 5 {
            param.push(next_chr);
            Ok((DecoderState::LineStyleWidthPixels(param), None))
        } else {
            param.push(next_chr);

            let mut param = param.chars();
            let width = Self::decode_f32(&mut param)?;

            Ok((DecoderState::None, Some(Draw::LineWidthPixels(width))))
        }
    }

    #[inline]
    fn decode_move(
        next_chr: char,
        mut param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 11 {
            param.push(next_chr);
            Ok((DecoderState::Move(param), None))
        } else {
            param.push(next_chr);

            let mut param = param.chars();
            let x = Self::decode_f32(&mut param)?;
            let y = Self::decode_f32(&mut param)?;

            Ok((DecoderState::None, Some(Draw::Path(PathOp::Move(x, y)))))
        }
    }

    #[inline]
    fn decode_line(
        next_chr: char,
        mut param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 11 {
            param.push(next_chr);
            Ok((DecoderState::Line(param), None))
        } else {
            param.push(next_chr);

            let mut param = param.chars();
            let x = Self::decode_f32(&mut param)?;
            let y = Self::decode_f32(&mut param)?;

            Ok((DecoderState::None, Some(Draw::Path(PathOp::Line(x, y)))))
        }
    }

    #[inline]
    fn decode_bezier_curve(
        next_chr: char,
        mut param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 35 {
            param.push(next_chr);
            Ok((DecoderState::BezierCurve(param), None))
        } else {
            param.push(next_chr);

            let mut param = param.chars();
            let x1 = Self::decode_f32(&mut param)?;
            let y1 = Self::decode_f32(&mut param)?;
            let cp1x = Self::decode_f32(&mut param)?;
            let cp1y = Self::decode_f32(&mut param)?;
            let cp2x = Self::decode_f32(&mut param)?;
            let cp2y = Self::decode_f32(&mut param)?;

            Ok((
                DecoderState::None,
                Some(Draw::Path(PathOp::BezierCurve(
                    ((cp1x, cp1y), (cp2x, cp2y)),
                    (x1, y1),
                ))),
            ))
        }
    }

    #[inline]
    fn decode_line_width(
        next_chr: char,
        mut param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 5 {
            param.push(next_chr);
            Ok((DecoderState::LineStyleWidth(param), None))
        } else {
            param.push(next_chr);

            let mut param = param.chars();
            let width = Self::decode_f32(&mut param)?;

            Ok((DecoderState::None, Some(Draw::LineWidth(width))))
        }
    }

    #[inline]
    fn decode_line_style_join(
        next_chr: char,
        _param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        match next_chr {
            'M' => Ok((DecoderState::None, Some(Draw::LineJoin(LineJoin::Miter)))),
            'R' => Ok((DecoderState::None, Some(Draw::LineJoin(LineJoin::Round)))),
            'B' => Ok((DecoderState::None, Some(Draw::LineJoin(LineJoin::Bevel)))),

            _ => Err(DecoderError::InvalidCharacter(next_chr)),
        }
    }

    #[inline]
    fn decode_line_style_cap(
        next_chr: char,
        _param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        match next_chr {
            'B' => Ok((DecoderState::None, Some(Draw::LineCap(LineCap::Butt)))),
            'R' => Ok((DecoderState::None, Some(Draw::LineCap(LineCap::Round)))),
            'S' => Ok((DecoderState::None, Some(Draw::LineCap(LineCap::Square)))),

            _ => Err(DecoderError::InvalidCharacter(next_chr)),
        }
    }

    #[inline]
    fn decode_dash_length(
        next_chr: char,
        mut param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 5 {
            param.push(next_chr);
            Ok((DecoderState::DashLength(param), None))
        } else {
            param.push(next_chr);
            let mut param = param.chars();
            Ok((
                DecoderState::None,
                Some(Draw::DashLength(Self::decode_f32(&mut param)?)),
            ))
        }
    }

    #[inline]
    fn decode_dash_offset(
        next_chr: char,
        mut param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 5 {
            param.push(next_chr);
            Ok((DecoderState::DashOffset(param), None))
        } else {
            param.push(next_chr);
            let mut param = param.chars();
            Ok((
                DecoderState::None,
                Some(Draw::DashOffset(Self::decode_f32(&mut param)?)),
            ))
        }
    }

    #[inline]
    fn decode_clear_canvas(
        next_chr: char,
        mut param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 24 {
            param.push(next_chr);
            Ok((DecoderState::ClearCanvas(param), None))
        } else {
            param.push(next_chr);

            let mut param = param.chars();
            let col_type = param.next();
            let r = Self::decode_f32(&mut param)?;
            let g = Self::decode_f32(&mut param)?;
            let b = Self::decode_f32(&mut param)?;
            let a = Self::decode_f32(&mut param)?;

            if col_type != Some('R') {
                Err(DecoderError::UnknownColorType)?;
            }

            Ok((
                DecoderState::None,
                Some(Draw::ClearCanvas(Color::Rgba(r, g, b, a))),
            ))
        }
    }

    #[inline]
    fn decode_color_stroke(
        next_chr: char,
        mut param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 24 {
            param.push(next_chr);
            Ok((DecoderState::ColorStroke(param), None))
        } else {
            param.push(next_chr);

            let mut param = param.chars();
            let col_type = param.next();
            let r = Self::decode_f32(&mut param)?;
            let g = Self::decode_f32(&mut param)?;
            let b = Self::decode_f32(&mut param)?;
            let a = Self::decode_f32(&mut param)?;

            if col_type != Some('R') {
                Err(DecoderError::UnknownColorType)?;
            }

            Ok((
                DecoderState::None,
                Some(Draw::StrokeColor(Color::Rgba(r, g, b, a))),
            ))
        }
    }

    #[inline]
    fn decode_color_fill(
        next_chr: char,
        mut param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 24 {
            param.push(next_chr);
            Ok((DecoderState::ColorFill(param), None))
        } else {
            param.push(next_chr);

            let mut param = param.chars();
            let col_type = param.next();
            let r = Self::decode_f32(&mut param)?;
            let g = Self::decode_f32(&mut param)?;
            let b = Self::decode_f32(&mut param)?;
            let a = Self::decode_f32(&mut param)?;

            if col_type != Some('R') {
                Err(DecoderError::UnknownColorType)?;
            }

            Ok((
                DecoderState::None,
                Some(Draw::FillColor(Color::Rgba(r, g, b, a))),
            ))
        }
    }

    #[inline]
    fn decode_color_texture(
        next_chr: char,
        texture_id: DecodeTextureId,
        mut param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        use self::PartialResult::*;

        // Decode the texture ID first
        let texture_id = match texture_id {
            MatchMore(texture_id) => {
                let texture_id = Self::decode_texture_id(next_chr, texture_id)?;
                return Ok((DecoderState::ColorTexture(texture_id, param), None));
            }

            FullMatch(texture_id) => texture_id,
        };

        // There are 4 coordinates following the texture ID (at 6 bytes each)
        param.push(next_chr);

        if param.len() < 24 {
            // More characters required
            Ok((
                DecoderState::ColorTexture(FullMatch(texture_id), param),
                None,
            ))
        } else {
            // Decode the coordinates
            let mut param = param.chars();
            let x1 = Self::decode_f32(&mut param)?;
            let y1 = Self::decode_f32(&mut param)?;
            let x2 = Self::decode_f32(&mut param)?;
            let y2 = Self::decode_f32(&mut param)?;

            Ok((
                DecoderState::None,
                Some(Draw::FillTexture(texture_id, (x1, y1), (x2, y2))),
            ))
        }
    }

    #[inline]
    fn decode_color_gradient(
        next_chr: char,
        gradient_id: DecodeGradientId,
        mut param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        use self::PartialResult::*;

        // Decode the texture ID first
        let gradient_id = match gradient_id {
            MatchMore(gradient_id) => {
                let gradient_id = Self::decode_gradient_id(next_chr, gradient_id)?;
                return Ok((DecoderState::ColorGradient(gradient_id, param), None));
            }

            FullMatch(gradient_id) => gradient_id,
        };

        // There are 4 coordinates following the texture ID (at 6 bytes each)
        param.push(next_chr);

        if param.len() < 24 {
            // More characters required
            Ok((
                DecoderState::ColorGradient(FullMatch(gradient_id), param),
                None,
            ))
        } else {
            // Decode the coordinates
            let mut param = param.chars();
            let x1 = Self::decode_f32(&mut param)?;
            let y1 = Self::decode_f32(&mut param)?;
            let x2 = Self::decode_f32(&mut param)?;
            let y2 = Self::decode_f32(&mut param)?;

            Ok((
                DecoderState::None,
                Some(Draw::FillGradient(gradient_id, (x1, y1), (x2, y2))),
            ))
        }
    }

    #[inline]
    fn decode_color_transform(
        next_chr: char,
        mut param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 53 {
            param.push(next_chr);
            Ok((DecoderState::ColorTransform(param), None))
        } else {
            param.push(next_chr);
            let mut param = param.chars();

            let mut matrix = [0.0; 9];
            for entry in 0..9 {
                matrix[entry] = Self::decode_f32(&mut param)?;
            }

            let transform = Transform2D([
                [matrix[0], matrix[1], matrix[2]],
                [matrix[3], matrix[4], matrix[5]],
                [matrix[6], matrix[7], matrix[8]],
            ]);

            Ok((DecoderState::None, Some(Draw::FillTransform(transform))))
        }
    }

    #[inline]
    fn decode_blend_mode(
        next_chr: char,
        mut param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 1 {
            param.push(next_chr);
            Ok((DecoderState::BlendMode(param), None))
        } else {
            param.push(next_chr);
            let mut param = param.chars();
            Ok((
                DecoderState::None,
                Some(Draw::BlendMode(Self::decode_blend_mode_only(&mut param)?)),
            ))
        }
    }

    #[inline]
    fn decode_transform_height(
        next_chr: char,
        mut param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 5 {
            param.push(next_chr);
            Ok((DecoderState::TransformHeight(param), None))
        } else {
            param.push(next_chr);
            let mut param = param.chars();
            Ok((
                DecoderState::None,
                Some(Draw::CanvasHeight(Self::decode_f32(&mut param)?)),
            ))
        }
    }

    #[inline]
    fn decode_transform_center(
        next_chr: char,
        mut param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 23 {
            param.push(next_chr);
            Ok((DecoderState::TransformCenter(param), None))
        } else {
            param.push(next_chr);

            let mut param = param.chars();
            let min_x = Self::decode_f32(&mut param)?;
            let min_y = Self::decode_f32(&mut param)?;
            let max_x = Self::decode_f32(&mut param)?;
            let max_y = Self::decode_f32(&mut param)?;

            Ok((
                DecoderState::None,
                Some(Draw::CenterRegion((min_x, min_y), (max_x, max_y))),
            ))
        }
    }

    #[inline]
    fn decode_transform_multiply(
        next_chr: char,
        mut param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 53 {
            param.push(next_chr);
            Ok((DecoderState::TransformMultiply(param), None))
        } else {
            param.push(next_chr);
            let mut param = param.chars();

            let mut matrix = [0.0; 9];
            for entry in 0..9 {
                matrix[entry] = Self::decode_f32(&mut param)?;
            }

            let transform = Transform2D([
                [matrix[0], matrix[1], matrix[2]],
                [matrix[3], matrix[4], matrix[5]],
                [matrix[6], matrix[7], matrix[8]],
            ]);

            Ok((DecoderState::None, Some(Draw::MultiplyTransform(transform))))
        }
    }

    #[inline]
    fn decode_new_layer_u32(
        next_chr: char,
        mut param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 5 {
            param.push(next_chr);
            Ok((DecoderState::NewLayerU32(param), None))
        } else {
            param.push(next_chr);
            let mut param = param.chars();
            Ok((
                DecoderState::None,
                Some(Draw::Layer(LayerId(Self::decode_u32(&mut param)? as _))),
            ))
        }
    }

    #[inline]
    fn decode_new_layer_blend_u32(
        next_chr: char,
        mut param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 7 {
            param.push(next_chr);
            Ok((DecoderState::NewLayerBlendU32(param), None))
        } else {
            param.push(next_chr);

            let mut param = param.chars();
            let layer_id = Self::decode_u32(&mut param)?;
            let blend_mode = Self::decode_blend_mode_only(&mut param)?;

            Ok((
                DecoderState::None,
                Some(Draw::LayerBlend(LayerId(layer_id as _), blend_mode)),
            ))
        }
    }

    #[inline]
    fn decode_new_layer(
        next_chr: char,
        param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        match Self::decode_layer_id(next_chr, param)? {
            PartialResult::FullMatch(layer_id) => {
                Ok((DecoderState::None, Some(Draw::Layer(layer_id))))
            }
            PartialResult::MatchMore(param) => Ok((DecoderState::NewLayer(param), None)),
        }
    }

    #[inline]
    fn decode_swap_layers(
        next_chr: char,
        layer1: Option<LayerId>,
        param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        match (layer1, Self::decode_layer_id(next_chr, param)?) {
            (None, PartialResult::FullMatch(layer_id)) => Ok((
                DecoderState::SwapLayers(Some(layer_id), String::new()),
                None,
            )),
            (Some(layer1), PartialResult::FullMatch(layer2)) => {
                Ok((DecoderState::None, Some(Draw::SwapLayers(layer1, layer2))))
            }
            (layer1, PartialResult::MatchMore(param)) => {
                Ok((DecoderState::SwapLayers(layer1, param), None))
            }
        }
    }

    #[inline]
    fn decode_new_layer_blend(
        next_chr: char,
        layer_param: PartialResult<LayerId>,
        mut blend_mode: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        match (layer_param, blend_mode.len()) {
            (PartialResult::MatchMore(layer_param), _) => Ok((
                DecoderState::NewLayerBlend(
                    Self::decode_layer_id(next_chr, layer_param)?,
                    blend_mode,
                ),
                None,
            )),
            (PartialResult::FullMatch(layer_id), 0) => {
                blend_mode.push(next_chr);
                Ok((
                    DecoderState::NewLayerBlend(PartialResult::FullMatch(layer_id), blend_mode),
                    None,
                ))
            }
            (PartialResult::FullMatch(layer_id), _) => {
                blend_mode.push(next_chr);
                Ok((
                    DecoderState::None,
                    Some(Draw::LayerBlend(
                        layer_id,
                        Self::decode_blend_mode_only(&mut blend_mode.chars())?,
                    )),
                ))
            }
        }
    }

    #[inline]
    fn decode_new_layer_alpha(
        next_chr: char,
        layer_param: PartialResult<LayerId>,
        mut alpha: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        match layer_param {
            PartialResult::MatchMore(layer_param) => Ok((
                DecoderState::NewLayerAlpha(Self::decode_layer_id(next_chr, layer_param)?, alpha),
                None,
            )),
            PartialResult::FullMatch(layer_id) => {
                alpha.push(next_chr);

                if alpha.len() < 6 {
                    Ok((
                        DecoderState::NewLayerAlpha(PartialResult::FullMatch(layer_id), alpha),
                        None,
                    ))
                } else {
                    Ok((
                        DecoderState::None,
                        Some(Draw::LayerAlpha(
                            layer_id,
                            Self::decode_f32(&mut alpha.chars())?,
                        )),
                    ))
                }
            }
        }
    }

    #[inline]
    fn decode_new_sprite(
        next_chr: char,
        param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        match Self::decode_sprite_id(next_chr, param)? {
            PartialResult::FullMatch(sprite_id) => {
                Ok((DecoderState::None, Some(Draw::Sprite(sprite_id))))
            }
            PartialResult::MatchMore(param) => Ok((DecoderState::NewSprite(param), None)),
        }
    }

    #[inline]
    fn decode_sprite_draw(
        next_chr: char,
        param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        match Self::decode_sprite_id(next_chr, param)? {
            PartialResult::FullMatch(sprite_id) => {
                Ok((DecoderState::None, Some(Draw::DrawSprite(sprite_id))))
            }
            PartialResult::MatchMore(param) => Ok((DecoderState::SpriteDraw(param), None)),
        }
    }

    #[inline]
    fn decode_sprite_move_from(
        next_chr: char,
        param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        match Self::decode_sprite_id(next_chr, param)? {
            PartialResult::FullMatch(sprite_id) => {
                Ok((DecoderState::None, Some(Draw::MoveSpriteFrom(sprite_id))))
            }
            PartialResult::MatchMore(param) => Ok((DecoderState::SpriteMoveFrom(param), None)),
        }
    }

    #[inline]
    fn decode_sprite_draw_with_filters(
        next_chr: char,
        param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        match Self::decode_sprite_id(next_chr, param)? {
            PartialResult::FullMatch(sprite_id) => Ok((
                DecoderState::SpriteDrawWithFiltersId(sprite_id, String::new()),
                None,
            )),
            PartialResult::MatchMore(param) => {
                Ok((DecoderState::SpriteDrawWithFilters(param), None))
            }
        }
    }

    fn decode_sprite_draw_with_filters_id(
        next_chr: char,
        sprite_id: SpriteId,
        param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        // Add the character to the parameter
        let mut param = param;
        param.push(next_chr);

        // Try decoding
        let mut chars = param.chars();

        // Decode the length
        let length = match Self::try_decode_compact_u64(&mut chars)? {
            Some(length) => length,
            None => {
                return Ok((
                    DecoderState::SpriteDrawWithFiltersId(sprite_id, param),
                    None,
                ));
            }
        };

        // Decode the filters
        let mut filters = vec![];

        for _ in 0..length {
            match Self::try_decode_texture_filter(&mut chars)? {
                Some(filter) => {
                    filters.push(filter);
                }
                None => {
                    return Ok((
                        DecoderState::SpriteDrawWithFiltersId(sprite_id, param),
                        None,
                    ));
                }
            }
        }

        return Ok((
            DecoderState::None,
            Some(Draw::DrawSpriteWithFilters(sprite_id, filters)),
        ));
    }

    #[inline]
    fn decode_sprite_transform(
        next_chr: char,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        match next_chr {
            'i' => Ok((
                DecoderState::None,
                Some(Draw::SpriteTransform(SpriteTransform::Identity)),
            )),
            't' => Ok((DecoderState::SpriteTransformTranslate(String::new()), None)),
            's' => Ok((DecoderState::SpriteTransformScale(String::new()), None)),
            'r' => Ok((DecoderState::SpriteTransformRotate(String::new()), None)),
            'T' => Ok((DecoderState::SpriteTransformTransform(String::new()), None)),

            _ => Err(DecoderError::InvalidCharacter(next_chr)),
        }
    }

    #[inline]
    fn decode_sprite_transform_translate(
        next_chr: char,
        mut param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 11 {
            param.push(next_chr);
            Ok((DecoderState::SpriteTransformTranslate(param), None))
        } else {
            param.push(next_chr);

            let mut param = param.chars();
            let x = Self::decode_f32(&mut param)?;
            let y = Self::decode_f32(&mut param)?;

            Ok((
                DecoderState::None,
                Some(Draw::SpriteTransform(SpriteTransform::Translate(x, y))),
            ))
        }
    }

    #[inline]
    fn decode_sprite_transform_scale(
        next_chr: char,
        mut param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 11 {
            param.push(next_chr);
            Ok((DecoderState::SpriteTransformScale(param), None))
        } else {
            param.push(next_chr);

            let mut param = param.chars();
            let x = Self::decode_f32(&mut param)?;
            let y = Self::decode_f32(&mut param)?;

            Ok((
                DecoderState::None,
                Some(Draw::SpriteTransform(SpriteTransform::Scale(x, y))),
            ))
        }
    }

    #[inline]
    fn decode_sprite_transform_rotate(
        next_chr: char,
        mut param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 5 {
            param.push(next_chr);
            Ok((DecoderState::SpriteTransformRotate(param), None))
        } else {
            param.push(next_chr);

            let mut param = param.chars();
            let degrees = Self::decode_f32(&mut param)?;

            Ok((
                DecoderState::None,
                Some(Draw::SpriteTransform(SpriteTransform::Rotate(degrees))),
            ))
        }
    }

    #[inline]
    fn decode_sprite_transform_transform(
        next_chr: char,
        mut param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 53 {
            param.push(next_chr);
            Ok((DecoderState::SpriteTransformTransform(param), None))
        } else {
            param.push(next_chr);
            let mut param = param.chars();

            let mut matrix = [0.0; 9];
            for entry in 0..9 {
                matrix[entry] = Self::decode_f32(&mut param)?;
            }

            let transform = Transform2D([
                [matrix[0], matrix[1], matrix[2]],
                [matrix[3], matrix[4], matrix[5]],
                [matrix[6], matrix[7], matrix[8]],
            ]);

            Ok((
                DecoderState::None,
                Some(Draw::SpriteTransform(SpriteTransform::Transform2D(
                    transform,
                ))),
            ))
        }
    }

    #[inline]
    fn decode_winding_rule(next_chr: char) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        match next_chr {
            'n' => Ok((
                DecoderState::None,
                Some(Draw::WindingRule(WindingRule::NonZero)),
            )),
            'e' => Ok((
                DecoderState::None,
                Some(Draw::WindingRule(WindingRule::EvenOdd)),
            )),
            _ => Err(DecoderError::InvalidCharacter(next_chr)),
        }
    }

    ///
    /// Consumes 2 characters to decode a blend mode
    ///
    fn decode_blend_mode_only(param: &mut Chars) -> Result<BlendMode, DecoderError> {
        let (a, b) = (param.next(), param.next());
        let a = a.ok_or(DecoderError::MissingCharacter)?;
        let b = b.ok_or(DecoderError::MissingCharacter)?;

        match (a, b) {
            ('S', 'V') => Ok(BlendMode::SourceOver),
            ('S', 'I') => Ok(BlendMode::SourceIn),
            ('S', 'O') => Ok(BlendMode::SourceOut),
            ('S', 'A') => Ok(BlendMode::SourceAtop),

            ('D', 'V') => Ok(BlendMode::DestinationOver),
            ('D', 'I') => Ok(BlendMode::DestinationIn),
            ('D', 'O') => Ok(BlendMode::DestinationOut),
            ('D', 'A') => Ok(BlendMode::DestinationAtop),

            ('E', 'M') => Ok(BlendMode::Multiply),
            ('E', 'S') => Ok(BlendMode::Screen),
            ('E', 'D') => Ok(BlendMode::Darken),
            ('E', 'L') => Ok(BlendMode::Lighten),

            _ => Err(DecoderError::InvalidCharacter(a)),
        }
    }

    ///
    /// Consumes characters until we have a u64 ID
    ///
    fn try_decode_compact_u64(chars: &mut Chars) -> Result<Option<u64>, DecoderError> {
        let mut result = 0u64;
        let mut shift = 0;

        // Decode to a u64 if the 0x20 bit is not set
        while let Some(next_chr) = chars.next() {
            if shift >= 64 {
                return Err(DecoderError::BadNumber);
            }

            let decoded = Self::decode_base64(next_chr)?;
            result |= ((decoded & !0x20) as u64) << shift;

            if (decoded & 0x20) == 0 {
                return Ok(Some(result));
            }

            shift += 5;
        }

        Ok(None)
    }

    ///
    /// Consumes characters until we have a u64 ID
    ///
    fn decode_compact_id(
        next_chr: char,
        mut param: String,
    ) -> Result<PartialResult<u64>, DecoderError> {
        param.push(next_chr);

        match Self::try_decode_compact_u64(&mut param.chars())? {
            Some(num) => Ok(PartialResult::FullMatch(num)),
            None => Ok(PartialResult::MatchMore(param)),
        }
    }

    ///
    /// Consumes characters until we have a sprite ID
    ///
    fn decode_sprite_id(
        next_chr: char,
        param: String,
    ) -> Result<PartialResult<SpriteId>, DecoderError> {
        Self::decode_compact_id(next_chr, param).map(|id| id.map(|id| SpriteId(id)))
    }

    ///
    /// Consumes characters until we have a layer ID
    ///
    fn decode_layer_id(
        next_chr: char,
        param: String,
    ) -> Result<PartialResult<LayerId>, DecoderError> {
        Self::decode_compact_id(next_chr, param).map(|id| id.map(|id| LayerId(id)))
    }

    ///
    /// Consumes characters until we have a font ID
    ///
    fn decode_font_id(
        next_chr: char,
        param: String,
    ) -> Result<PartialResult<FontId>, DecoderError> {
        Self::decode_compact_id(next_chr, param).map(|id| id.map(|id| FontId(id)))
    }

    ///
    /// Consumes characters until we have a texture ID
    ///
    fn decode_texture_id(
        next_chr: char,
        param: String,
    ) -> Result<PartialResult<TextureId>, DecoderError> {
        Self::decode_compact_id(next_chr, param).map(|id| id.map(|id| TextureId(id)))
    }

    ///
    /// Tries to decode a texture ID from a list of characters
    ///
    fn try_decode_texture_id(chars: &mut Chars) -> Result<Option<TextureId>, DecoderError> {
        let mut texture_id = PartialResult::new();

        while let Some(next_chr) = chars.next() {
            // Add the next character to the result
            match texture_id {
                PartialResult::MatchMore(param) => {
                    texture_id = Self::decode_compact_id(next_chr, param)?;
                }
                _ => {
                    panic!()
                }
            }

            // Return the texture ID if we have a full match
            if let PartialResult::FullMatch(texture_id) = texture_id {
                return Ok(Some(TextureId(texture_id)));
            }
        }

        // Did not decode a full texture ID before running out of characters
        Ok(None)
    }

    ///
    /// Consumes characters until we have a gradient ID
    ///
    fn decode_gradient_id(
        next_chr: char,
        param: String,
    ) -> Result<PartialResult<GradientId>, DecoderError> {
        Self::decode_compact_id(next_chr, param).map(|id| id.map(|id| GradientId(id)))
    }

    ///
    /// Decodes a font drawing command
    ///
    #[inline]
    fn decode_font_drawing(chr: char) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        match chr {
            'T' => Ok((
                DecoderState::FontDrawText(
                    PartialResult::new(),
                    DecodeString::new(),
                    String::new(),
                ),
                None,
            )),
            'R' => Ok((DecoderState::None, Some(Draw::DrawLaidOutText))),
            'l' => Ok((DecoderState::FontBeginLayout(String::new()), None)),
            _ => Err(DecoderError::InvalidCharacter(chr)),
        }
    }

    ///
    /// Decodes the DrawText command (which is one of the more complicated ones, with a font ID, a string and a set of coordinates
    /// to deal with)
    ///
    fn decode_font_draw_text(
        chr: char,
        font_id: DecodeFontId,
        string: DecodeString,
        mut coords: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        use PartialResult::*;

        match (font_id, string.ready(), coords.len()) {
            (MatchMore(font_id), _, _) => Ok((
                DecoderState::FontDrawText(Self::decode_font_id(chr, font_id)?, string, coords),
                None,
            )),
            (FullMatch(font_id), false, _) => Ok((
                DecoderState::FontDrawText(FullMatch(font_id), string.decode(chr)?, coords),
                None,
            )),

            (FullMatch(font_id), true, 0)
            | (FullMatch(font_id), true, 1)
            | (FullMatch(font_id), true, 2)
            | (FullMatch(font_id), true, 3)
            | (FullMatch(font_id), true, 4)
            | (FullMatch(font_id), true, 5)
            | (FullMatch(font_id), true, 6)
            | (FullMatch(font_id), true, 7)
            | (FullMatch(font_id), true, 8)
            | (FullMatch(font_id), true, 9)
            | (FullMatch(font_id), true, 10) => {
                coords.push(chr);
                Ok((
                    DecoderState::FontDrawText(FullMatch(font_id), string, coords),
                    None,
                ))
            }

            (FullMatch(font_id), true, _) => {
                coords.push(chr);

                let mut coords = coords.chars();
                let x = Self::decode_f32(&mut coords)?;
                let y = Self::decode_f32(&mut coords)?;

                Ok((
                    DecoderState::None,
                    Some(Draw::DrawText(font_id, string.to_string()?, x, y)),
                ))
            }
        }
    }

    ///
    /// Decodes the 'begin layout' instruction
    ///
    fn decode_font_begin_layout(
        chr: char,
        param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        // Push the character
        let mut param = param;
        param.push(chr);

        // 2x f32 + 1 character
        if param.len() < 13 {
            return Ok((DecoderState::FontBeginLayout(param), None));
        }

        // Decode
        let mut chrs = param.chars();

        let x = Self::decode_f32(&mut chrs)?;
        let y = Self::decode_f32(&mut chrs)?;

        let align = match chrs.next() {
            Some('l') => Ok(TextAlignment::Left),
            Some('r') => Ok(TextAlignment::Right),
            Some('c') => Ok(TextAlignment::Center),
            Some(other) => Err(DecoderError::InvalidCharacter(other)),
            None => Err(DecoderError::NotReady),
        }?;

        Ok((DecoderState::None, Some(Draw::BeginLineLayout(x, y, align))))
    }

    ///
    /// Decodes a FontOp command
    ///
    /// These all start with a font ID, followed by an operation identifier which determines how the
    /// rest of the decoding should go
    ///
    fn decode_font_op(
        chr: char,
        font_id: DecodeFontId,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        use PartialResult::*;

        // Decode the font ID first
        let font_id = match font_id {
            MatchMore(font_id) => {
                let font_id = Self::decode_font_id(chr, font_id)?;
                return Ok((DecoderState::FontOp(font_id), None));
            }

            FullMatch(font_id) => font_id,
        };

        // The character following the font ID determines what state we move on to
        match chr {
            'd' => Ok((DecoderState::FontOpData(font_id), None)),
            'S' => Ok((DecoderState::FontOpSize(font_id, String::new()), None)),
            'L' => Ok((
                DecoderState::FontOpLayoutText(font_id, DecodeString::new()),
                None,
            )),
            'G' => Ok((
                DecoderState::FontOpDrawGlyphs(font_id, DecodeGlyphPositions::new()),
                None,
            )),

            _ => Err(DecoderError::InvalidCharacter(chr)),
        }
    }

    ///
    /// Decodes a FontSize fontop
    ///
    fn decode_font_op_size(
        chr: char,
        font_id: FontId,
        size: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        // Add the character to the size
        let mut size = size;
        size.push(chr);

        // Can decode once we have 6 characters
        if size.len() >= 6 {
            Ok((
                DecoderState::None,
                Some(Draw::Font(
                    font_id,
                    FontOp::FontSize(Self::decode_f32(&mut size.chars())?),
                )),
            ))
        } else {
            // Haven't got enough characters yet
            Ok((DecoderState::FontOpSize(font_id, size), None))
        }
    }

    ///
    /// Decodes a font data item
    ///
    fn decode_font_op_data(
        chr: char,
        font_id: FontId,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        match chr {
            'T' => Ok((DecoderState::FontOpTtf(font_id, DecodeBytes::new()), None)),
            _ => Err(DecoderError::InvalidCharacter(chr)),
        }
    }

    ///
    /// Decodes TTF font data
    ///
    fn decode_font_data_ttf(
        chr: char,
        font_id: FontId,
        bytes: DecodeBytes,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        // Decode the next byte
        let bytes = bytes.decode(chr)?;

        // Generate the result once finished
        if bytes.ready() {
            let bytes = bytes.to_bytes()?;
            let font = CanvasFontFace::from_bytes(bytes);
            Ok((
                DecoderState::None,
                Some(Draw::Font(font_id, FontOp::UseFontDefinition(font))),
            ))
        } else {
            Ok((DecoderState::FontOpTtf(font_id, bytes), None))
        }
    }

    ///
    /// Decides a text layout instruction
    ///
    fn decode_font_op_layout(
        chr: char,
        font_id: FontId,
        string: DecodeString,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        let string = string.decode(chr)?;

        if string.ready() {
            let string = string.to_string()?;
            Ok((
                DecoderState::None,
                Some(Draw::Font(font_id, FontOp::LayoutText(string))),
            ))
        } else {
            Ok((DecoderState::FontOpLayoutText(font_id, string), None))
        }
    }

    ///
    /// Decides a text layout instruction
    ///
    fn decode_font_op_glyphs(
        chr: char,
        font_id: FontId,
        glyphs: DecodeGlyphPositions,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        let glyphs = glyphs.decode(chr)?;

        if glyphs.ready() {
            let glyphs = glyphs.to_glyphs()?;
            Ok((
                DecoderState::None,
                Some(Draw::Font(font_id, FontOp::DrawGlyphs(glyphs))),
            ))
        } else {
            Ok((DecoderState::FontOpDrawGlyphs(font_id, glyphs), None))
        }
    }

    ///
    /// Decodes a texture operation
    ///
    fn decode_texture_op(
        chr: char,
        texture_id: DecodeTextureId,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        use PartialResult::*;

        // Decode the texture ID first
        let texture_id = match texture_id {
            MatchMore(texture_id) => {
                let texture_id = Self::decode_texture_id(chr, texture_id)?;
                return Ok((DecoderState::TextureOp(texture_id), None));
            }

            FullMatch(texture_id) => texture_id,
        };

        // The character following the texture ID determines what state we move on to
        match chr {
            'N' => Ok((
                DecoderState::TextureOpCreate(texture_id, String::new()),
                None,
            )),
            'X' => Ok((
                DecoderState::None,
                Some(Draw::Texture(texture_id, TextureOp::Free)),
            )),
            'D' => Ok((
                DecoderState::TextureOpSetBytes(texture_id, String::new(), DecodeBytes::new()),
                None,
            )),
            'S' => Ok((
                DecoderState::TextureOpSetFromSprite(
                    texture_id,
                    DecodeSpriteId::new(),
                    String::new(),
                ),
                None,
            )),
            's' => Ok((
                DecoderState::TextureOpCreateDynamicSprite(
                    texture_id,
                    DecodeSpriteId::new(),
                    String::new(),
                ),
                None,
            )),
            't' => Ok((
                DecoderState::TextureOpFillTransparency(texture_id, String::new()),
                None,
            )),
            'C' => Ok((
                DecoderState::TextureOpCopy(texture_id, DecodeTextureId::new()),
                None,
            )),
            'F' => Ok((
                DecoderState::TextureOpFilter(texture_id, String::new()),
                None,
            )),

            _ => Err(DecoderError::InvalidCharacter(chr)),
        }
    }

    ///
    /// Decodes a texture create operation
    ///
    fn decode_texture_create(
        chr: char,
        texture_id: TextureId,
        param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        // Follow-up is 2 u32s and 1 format character for 13 characters total
        let mut param = param;
        param.push(chr);

        if param.len() < 13 {
            return Ok((DecoderState::TextureOpCreate(texture_id, param), None));
        }

        // Decode the texture
        let mut chars = param.chars();
        let w = Self::decode_u32(&mut chars)?;
        let h = Self::decode_u32(&mut chars)?;

        let format = match chars.next() {
            Some('r') => TextureFormat::Rgba,
            Some(c) => {
                return Err(DecoderError::InvalidCharacter(c));
            }
            None => {
                return Err(DecoderError::NotReady);
            }
        };

        Ok((
            DecoderState::None,
            Some(Draw::Texture(
                texture_id,
                TextureOp::Create(TextureSize(w, h), format),
            )),
        ))
    }

    ///
    /// Decodes a texture 'set bytes' operation
    ///
    fn decode_texture_set_bytes(
        chr: char,
        texture_id: TextureId,
        param: String,
        bytes: DecodeBytes,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        // 4 u32s and some data
        if param.len() < 24 {
            let mut param = param;
            param.push(chr);
            return Ok((
                DecoderState::TextureOpSetBytes(texture_id, param, bytes),
                None,
            ));
        }

        let bytes = bytes.decode(chr)?;

        if !bytes.ready() {
            return Ok((
                DecoderState::TextureOpSetBytes(texture_id, param, bytes),
                None,
            ));
        }

        // Decode the data
        let mut chars = param.chars();
        let x = Self::decode_u32(&mut chars)?;
        let y = Self::decode_u32(&mut chars)?;
        let w = Self::decode_u32(&mut chars)?;
        let h = Self::decode_u32(&mut chars)?;

        Ok((
            DecoderState::None,
            Some(Draw::Texture(
                texture_id,
                TextureOp::SetBytes(
                    TexturePosition(x, y),
                    TextureSize(w, h),
                    Arc::new(bytes.to_bytes()?),
                ),
            )),
        ))
    }

    ///
    /// Decodes a texture 'set from sprite' operation
    ///
    fn decode_texture_set_from_sprite(
        chr: char,
        texture_id: TextureId,
        sprite_id: DecodeSpriteId,
        param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        // Decode the sprite ID first
        let sprite_id = match sprite_id {
            PartialResult::MatchMore(sprite_id) => {
                let sprite_id = Self::decode_sprite_id(chr, sprite_id)?;
                return Ok((
                    DecoderState::TextureOpSetFromSprite(texture_id, sprite_id, param),
                    None,
                ));
            }

            PartialResult::FullMatch(sprite_id) => sprite_id,
        };

        // The parameter is 4x f32
        let mut param = param;
        param.push(chr);

        if param.len() < 4 * 6 {
            return Ok((
                DecoderState::TextureOpSetFromSprite(
                    texture_id,
                    PartialResult::FullMatch(sprite_id),
                    param,
                ),
                None,
            ));
        }

        // Decode the parameters
        let mut param = param.chars();

        let x = Self::decode_f32(&mut param)?;
        let y = Self::decode_f32(&mut param)?;
        let w = Self::decode_f32(&mut param)?;
        let h = Self::decode_f32(&mut param)?;

        Ok((
            DecoderState::None,
            Some(Draw::Texture(
                texture_id,
                TextureOp::SetFromSprite(
                    sprite_id,
                    SpriteBounds(SpritePosition(x, y), SpriteSize(w, h)),
                ),
            )),
        ))
    }

    ///
    /// Decodes a texture 'create dynamic sprite' operation
    ///
    fn decode_texture_create_dynamic_sprite(
        chr: char,
        texture_id: TextureId,
        sprite_id: DecodeSpriteId,
        param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        // Decode the sprite ID first
        let sprite_id = match sprite_id {
            PartialResult::MatchMore(sprite_id) => {
                let sprite_id = Self::decode_sprite_id(chr, sprite_id)?;
                return Ok((
                    DecoderState::TextureOpCreateDynamicSprite(texture_id, sprite_id, param),
                    None,
                ));
            }

            PartialResult::FullMatch(sprite_id) => sprite_id,
        };

        // The parameter is 6x f32
        let mut param = param;
        param.push(chr);

        if param.len() < 6 * 6 {
            return Ok((
                DecoderState::TextureOpCreateDynamicSprite(
                    texture_id,
                    PartialResult::FullMatch(sprite_id),
                    param,
                ),
                None,
            ));
        }

        // Decode the parameters
        let mut param = param.chars();

        let x = Self::decode_f32(&mut param)?;
        let y = Self::decode_f32(&mut param)?;
        let sprite_w = Self::decode_f32(&mut param)?;
        let sprite_h = Self::decode_f32(&mut param)?;
        let canvas_w = Self::decode_f32(&mut param)?;
        let canvas_h = Self::decode_f32(&mut param)?;

        Ok((
            DecoderState::None,
            Some(Draw::Texture(
                texture_id,
                TextureOp::CreateDynamicSprite(
                    sprite_id,
                    SpriteBounds(SpritePosition(x, y), SpriteSize(sprite_w, sprite_h)),
                    CanvasSize(canvas_w, canvas_h),
                ),
            )),
        ))
    }

    ///
    /// Decodes a texture 'set fill transparency'
    ///
    fn decode_texture_fill_transparency(
        chr: char,
        texture_id: TextureId,
        param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        let mut param = param;
        param.push(chr);

        // 1 f32 for the alpha
        if param.len() < 6 {
            return Ok((
                DecoderState::TextureOpFillTransparency(texture_id, param),
                None,
            ));
        }

        // Decode
        let mut chars = param.chars();
        let alpha = Self::decode_f32(&mut chars)?;

        Ok((
            DecoderState::None,
            Some(Draw::Texture(
                texture_id,
                TextureOp::FillTransparency(alpha),
            )),
        ))
    }

    ///
    /// Decodes a texture copy
    ///
    fn decode_texture_copy(
        chr: char,
        texture_id: TextureId,
        param: DecodeTextureId,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        // Decode the target texture ID
        let target_texture_id = match Self::decode_texture_id(chr, param.match_more()?)? {
            PartialResult::MatchMore(param) => {
                return Ok((
                    DecoderState::TextureOpCopy(texture_id, PartialResult::MatchMore(param)),
                    None,
                ));
            }
            PartialResult::FullMatch(target_texture_id) => target_texture_id,
        };

        Ok((
            DecoderState::None,
            Some(Draw::Texture(
                texture_id,
                TextureOp::Copy(target_texture_id),
            )),
        ))
    }

    ///
    /// Given a texture filter string, attempts to decode the corresponding filter
    ///
    /// Returns the texture filter if the parameter matches one, 'None' if more characters are required, or an error if there's a problem
    ///
    fn try_decode_texture_filter(chars: &mut Chars) -> Result<Option<TextureFilter>, DecoderError> {
        match chars.next() {
            Some('B') => Self::try_decode_texture_filter_gaussian_blur(chars),
            Some('A') => Self::try_decode_texture_filter_alpha_blend(chars),
            Some('M') => Self::try_decode_texture_filter_mask(chars),
            Some('D') => Self::try_decode_texture_filter_displacement_map(chars),
            Some(other) => Err(DecoderError::InvalidCharacter(other)),
            None => Ok(None),
        }
    }

    ///
    /// Decodes the parameters for a gaussian blur texture filter
    ///
    fn try_decode_texture_filter_gaussian_blur(
        chars: &mut Chars,
    ) -> Result<Option<TextureFilter>, DecoderError> {
        let radius = Self::try_decode_f32(chars)?;

        if let Some(radius) = radius {
            Ok(Some(TextureFilter::GaussianBlur(radius)))
        } else {
            Ok(None)
        }
    }

    ///
    /// Decodes the parameters for a gaussian blur texture filter
    ///
    fn try_decode_texture_filter_alpha_blend(
        chars: &mut Chars,
    ) -> Result<Option<TextureFilter>, DecoderError> {
        let blend = Self::try_decode_f32(chars)?;

        if let Some(blend) = blend {
            Ok(Some(TextureFilter::AlphaBlend(blend)))
        } else {
            Ok(None)
        }
    }

    ///
    /// Decodes the parameters for a gaussian blur texture filter
    ///
    fn try_decode_texture_filter_mask(
        chars: &mut Chars,
    ) -> Result<Option<TextureFilter>, DecoderError> {
        let texture_id = Self::try_decode_texture_id(chars)?;

        if let Some(texture_id) = texture_id {
            Ok(Some(TextureFilter::Mask(texture_id)))
        } else {
            Ok(None)
        }
    }

    ///
    /// Decodes the parameters for a gaussian blur texture filter
    ///
    fn try_decode_texture_filter_displacement_map(
        chars: &mut Chars,
    ) -> Result<Option<TextureFilter>, DecoderError> {
        let texture_id = Self::try_decode_texture_id(chars)?;
        let texture_id = if let Some(texture_id) = texture_id {
            texture_id
        } else {
            return Ok(None);
        };
        let x_radius = Self::try_decode_f32(chars)?;
        let x_radius = if let Some(x_radius) = x_radius {
            x_radius
        } else {
            return Ok(None);
        };
        let y_radius = Self::try_decode_f32(chars)?;
        let y_radius = if let Some(y_radius) = y_radius {
            y_radius
        } else {
            return Ok(None);
        };

        Ok(Some(TextureFilter::DisplacementMap(
            texture_id, x_radius, y_radius,
        )))
    }

    ///
    /// Decodes a texture filter op
    ///
    fn decode_texture_filter(
        chr: char,
        texture_id: TextureId,
        param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        let mut param = param;

        param.push(chr);

        if let Some(filter) = Self::try_decode_texture_filter(&mut param.chars())? {
            Ok((
                DecoderState::None,
                Some(Draw::Texture(texture_id, TextureOp::Filter(filter))),
            ))
        } else {
            Ok((DecoderState::TextureOpFilter(texture_id, param), None))
        }
    }

    ///
    /// Decodes a gradient ID and determines which gradient operation is being performed on it
    ///
    fn decode_gradient_op(
        chr: char,
        gradient_id: DecodeGradientId,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        use PartialResult::*;

        // Decode the texture ID first
        let gradient_id = match gradient_id {
            MatchMore(gradient_id) => {
                let gradient_id = Self::decode_gradient_id(chr, gradient_id)?;
                return Ok((DecoderState::GradientOp(gradient_id), None));
            }

            FullMatch(gradient_id) => gradient_id,
        };

        // The gradient op is indicated by the next character
        match chr {
            'N' => Ok((
                DecoderState::GradientOpNew(gradient_id, String::new()),
                None,
            )),
            'S' => Ok((
                DecoderState::GradientOpAddStop(gradient_id, String::new()),
                None,
            )),

            _ => Err(DecoderError::InvalidCharacter(chr)),
        }
    }

    ///
    /// Decodes the GradientOp::Create instruction
    ///
    fn decode_gradient_new(
        next_chr: char,
        gradient_id: GradientId,
        param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        // Parameter is the initial colour
        let mut param = param;

        if param.len() < 24 {
            param.push(next_chr);
            Ok((DecoderState::GradientOpNew(gradient_id, param), None))
        } else {
            param.push(next_chr);

            let mut param = param.chars();
            let col_type = param.next();
            let r = Self::decode_f32(&mut param)?;
            let g = Self::decode_f32(&mut param)?;
            let b = Self::decode_f32(&mut param)?;
            let a = Self::decode_f32(&mut param)?;

            if col_type != Some('R') {
                Err(DecoderError::UnknownColorType)?;
            }

            Ok((
                DecoderState::None,
                Some(Draw::Gradient(
                    gradient_id,
                    GradientOp::Create(Color::Rgba(r, g, b, a)),
                )),
            ))
        }
    }

    ///
    /// Decodes the GradientOp::AddStop instruction
    ///
    fn decode_gradient_add_stop(
        next_chr: char,
        gradient_id: GradientId,
        param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        // Parameter is a position followed by a colour
        let mut param = param;

        if param.len() < 30 {
            param.push(next_chr);
            Ok((DecoderState::GradientOpAddStop(gradient_id, param), None))
        } else {
            param.push(next_chr);

            let mut param = param.chars();
            let pos = Self::decode_f32(&mut param)?;
            let col_type = param.next();
            let r = Self::decode_f32(&mut param)?;
            let g = Self::decode_f32(&mut param)?;
            let b = Self::decode_f32(&mut param)?;
            let a = Self::decode_f32(&mut param)?;

            if col_type != Some('R') {
                Err(DecoderError::UnknownColorType)?;
            }

            Ok((
                DecoderState::None,
                Some(Draw::Gradient(
                    gradient_id,
                    GradientOp::AddStop(pos, Color::Rgba(r, g, b, a)),
                )),
            ))
        }
    }

    ///
    /// Decodes the Namespace instruction
    ///
    fn decode_namespace(
        next_chr: char,
        param: String,
    ) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        let mut param = param;

        if param.len() < 21 {
            param.push(next_chr);
            Ok((DecoderState::NewNamespace(param), None))
        } else {
            param.push(next_chr);

            let mut param = param.chars();
            let id_a = Self::decode_u64(&mut param)?;
            let id_b = Self::decode_u64(&mut param)?;

            let global_id = Uuid::from_u64_pair(id_a, id_b);

            Ok((
                DecoderState::None,
                Some(Draw::Namespace(NamespaceId::with_id(global_id))),
            ))
        }
    }

    ///
    /// Consumes 6 characters to decode a f32
    ///
    fn try_decode_f32(chrs: &mut Chars) -> Result<Option<f32>, DecoderError> {
        let as_u32 = Self::try_decode_u32(chrs)?;
        if let Some(as_u32) = as_u32 {
            let as_f32 = f32::from_bits(as_u32);

            Ok(Some(as_f32))
        } else {
            Ok(None)
        }
    }

    ///
    /// Consumes 6 characters to decode a f32
    ///
    fn decode_f32(chrs: &mut Chars) -> Result<f32, DecoderError> {
        let as_u32 = Self::decode_u32(chrs)?;
        let as_f32 = f32::from_bits(as_u32);

        Ok(as_f32)
    }

    ///
    /// Consumes 6 characters to decode a u32
    ///
    fn decode_u32(chrs: &mut Chars) -> Result<u32, DecoderError> {
        let mut result = 0;
        let mut shift = 0;

        for _ in 0..6 {
            let next_chr = chrs.next().ok_or(DecoderError::BadNumber)?;
            result |= (Self::decode_base64(next_chr)? as u32) << shift;
            shift += 6;
        }

        Ok(result)
    }

    ///
    /// Consumes 6 characters to decode a u32
    ///
    fn try_decode_u32(chrs: &mut Chars) -> Result<Option<u32>, DecoderError> {
        let mut result = 0;
        let mut shift = 0;

        for _ in 0..6 {
            let next_chr = chrs.next();
            let next_chr = if let Some(next_chr) = next_chr {
                next_chr
            } else {
                return Ok(None);
            };
            result |= (Self::decode_base64(next_chr)? as u32) << shift;
            shift += 6;
        }

        Ok(Some(result))
    }

    ///
    /// Consumes 11 characters to decode a u64
    ///
    fn decode_u64(chrs: &mut Chars) -> Result<u64, DecoderError> {
        let mut result = 0;
        let mut shift = 0;

        for _ in 0..11 {
            let next_chr = chrs.next().ok_or(DecoderError::BadNumber)?;
            result |= (Self::decode_base64(next_chr)? as u64) << shift;
            shift += 6;
        }

        Ok(result)
    }

    ///
    /// Decodes a base64 character to a number (in the range 0x00 -> 0x3f)
    ///
    #[inline]
    fn decode_base64(chr: char) -> Result<u8, DecoderError> {
        if chr >= 'A' && chr <= 'Z' {
            Ok((chr as u8) - ('A' as u8))
        } else if chr >= 'a' && chr <= 'z' {
            Ok((chr as u8) - ('a' as u8) + 26)
        } else if chr >= '0' && chr <= '9' {
            Ok((chr as u8) - ('0' as u8) + 52)
        } else if chr == '+' {
            Ok(62)
        } else if chr == '/' {
            Ok(63)
        } else {
            Err(DecoderError::BadNumber)
        }
    }
}

///
/// Decodes a canvas drawing represented as an iterator of characters. If there's an error in the stream, it will
/// be the last item decoded.
///
pub fn decode_drawing<In: IntoIterator<Item = char>>(
    source: In,
) -> impl Iterator<Item = Result<Draw, DecoderError>> {
    // The decoder represents the state machine used for decoding this item
    let mut decoder = CanvasDecoder::new();
    let mut seen_error = false;

    // Map the source characters into draw actions via the decoder
    source.into_iter().filter_map(move |chr| {
        match decoder.decode(chr) {
            Ok(Some(draw)) => Some(Ok(draw)),
            Ok(None) => None,
            Err(err) => {
                // The decoder will just return errors once it hits a failure: only return the initial error
                if !seen_error {
                    seen_error = true;
                    Some(Err(err))
                } else {
                    None
                }
            }
        }
    })
}

///
/// Error from either a decoder or the stream that's feeding it
///
#[derive(Clone, Debug, PartialEq)]
pub enum StreamDecoderError<E> {
    /// Error from the decoder
    Decoder(DecoderError),

    /// Error from the stream
    Stream(E),
}

///
/// Decodes a canvas drawing represented as a stream of characters.
///
pub fn decode_drawing_stream<In: Unpin + Stream<Item = Result<char, E>>, E>(
    source: In,
) -> impl Unpin + Stream<Item = Result<Draw, StreamDecoderError<E>>> {
    let mut source = source;
    let mut decoder = CanvasDecoder::new();
    let mut seen_error = false;

    stream::poll_fn(move |context| {
        if seen_error {
            // Only allow one error from the decoder (it remains in an error state after this)
            Poll::Ready(None)
        } else {
            loop {
                match source.poll_next_unpin(context) {
                    Poll::Ready(None) => {
                        return Poll::Ready(None);
                    }
                    Poll::Pending => {
                        return Poll::Pending;
                    }
                    Poll::Ready(Some(Ok(c))) => match decoder.decode(c) {
                        Ok(None) => {
                            continue;
                        }
                        Ok(Some(draw)) => {
                            return Poll::Ready(Some(Ok(draw)));
                        }
                        Err(err) => {
                            seen_error = true;
                            return Poll::Ready(Some(Err(StreamDecoderError::Decoder(err))));
                        }
                    },

                    Poll::Ready(Some(Err(err))) => {
                        return Poll::Ready(Some(Err(StreamDecoderError::Stream(err))));
                    }
                }
            }
        }
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::encoding::*;

    use futures::executor;

    ///
    /// Checks if a particular drawing operation can be both encoded and decoded
    ///
    fn check_round_trip_single(instruction: Draw) {
        check_round_trip(vec![instruction])
    }

    ///
    /// Checks if a particular string of drawing operations can be both encoded and decoded
    ///
    fn check_round_trip(instructions: Vec<Draw>) {
        // Encode the instruction
        let mut encoded = String::new();
        for instruction in instructions.iter() {
            instruction.encode_canvas(&mut encoded);
        }

        println!("{:?} {:?}", instructions, encoded);

        // Try decoding it
        let decoded = decode_drawing(encoded.chars()).collect::<Vec<_>>();

        println!("  -> {:?}", decoded);

        // Should decode OK
        assert!(decoded.len() == instructions.len());

        // Should be the same as the original instruction
        assert!(
            decoded
                == instructions
                    .into_iter()
                    .map(|draw| Ok(draw))
                    .collect::<Vec<_>>()
        );
    }

    #[test]
    fn decode_start_frame() {
        check_round_trip_single(Draw::StartFrame);
    }

    #[test]
    fn decode_show_frame() {
        check_round_trip_single(Draw::ShowFrame);
    }

    #[test]
    fn decode_reset_frame() {
        check_round_trip_single(Draw::ResetFrame);
    }

    #[test]
    fn decode_new_path() {
        check_round_trip_single(Draw::Path(PathOp::NewPath));
    }

    #[test]
    fn decode_move() {
        check_round_trip_single(Draw::Path(PathOp::Move(10.0, 15.0)));
    }

    #[test]
    fn decode_line() {
        check_round_trip_single(Draw::Path(PathOp::Line(20.0, 42.0)));
    }

    #[test]
    fn decode_bezier_curve() {
        check_round_trip_single(Draw::Path(PathOp::BezierCurve(
            ((1.0, 2.0), (3.0, 4.0)),
            (5.0, 6.0),
        )));
    }

    #[test]
    fn decode_close_path() {
        check_round_trip_single(Draw::Path(PathOp::ClosePath));
    }

    #[test]
    fn decode_fill() {
        check_round_trip_single(Draw::Fill);
    }

    #[test]
    fn decode_stroke() {
        check_round_trip_single(Draw::Stroke);
    }

    #[test]
    fn decode_line_width() {
        check_round_trip_single(Draw::LineWidth(23.0));
    }

    #[test]
    fn decode_line_width_pixels() {
        check_round_trip_single(Draw::LineWidthPixels(43.0));
    }

    #[test]
    fn decode_line_join() {
        check_round_trip_single(Draw::LineJoin(LineJoin::Bevel));
    }

    #[test]
    fn decode_line_cap() {
        check_round_trip_single(Draw::LineCap(LineCap::Round));
    }

    #[test]
    fn decode_new_dash_pattern() {
        check_round_trip_single(Draw::NewDashPattern);
    }

    #[test]
    fn decode_dash_length() {
        check_round_trip_single(Draw::DashLength(56.0));
    }

    #[test]
    fn decode_dash_offset() {
        check_round_trip_single(Draw::DashOffset(13.0));
    }

    #[test]
    fn decode_stroke_color() {
        check_round_trip_single(Draw::StrokeColor(Color::Rgba(0.1, 0.2, 0.3, 0.4)));
    }

    #[test]
    fn decode_fill_color() {
        check_round_trip_single(Draw::FillColor(Color::Rgba(0.2, 0.3, 0.4, 0.5)));
    }

    #[test]
    fn decode_fill_texture() {
        check_round_trip_single(Draw::FillTexture(TextureId(42), (1.0, 2.0), (3.0, 4.0)));
    }

    #[test]
    fn decode_blend_mode() {
        check_round_trip_single(Draw::BlendMode(BlendMode::Lighten));
    }

    #[test]
    fn decode_identity_transform() {
        check_round_trip_single(Draw::IdentityTransform);
    }

    #[test]
    fn decode_canvas_height() {
        check_round_trip_single(Draw::CanvasHeight(81.0));
    }

    #[test]
    fn decode_center_region() {
        check_round_trip_single(Draw::CenterRegion((6.0, 7.0), (8.0, 9.0)));
    }

    #[test]
    fn decode_multiply_transform() {
        check_round_trip_single(Draw::MultiplyTransform(Transform2D([
            [1.0, 2.0, 3.0],
            [4.0, 5.0, 6.0],
            [7.0, 8.0, 9.0],
        ])));
    }

    #[test]
    fn decode_unclip() {
        check_round_trip_single(Draw::Unclip);
    }

    #[test]
    fn decode_clip() {
        check_round_trip_single(Draw::Clip)
    }

    #[test]
    fn decode_store() {
        check_round_trip_single(Draw::Store);
    }

    #[test]
    fn decode_restore() {
        check_round_trip_single(Draw::Restore);
    }

    #[test]
    fn decode_free_stored_buffer() {
        check_round_trip_single(Draw::FreeStoredBuffer);
    }

    #[test]
    fn decode_push_state() {
        check_round_trip_single(Draw::PushState);
    }

    #[test]
    fn decode_pop_state() {
        check_round_trip_single(Draw::PopState);
    }

    #[test]
    fn decode_clear_canvas() {
        check_round_trip_single(Draw::ClearCanvas(Color::Rgba(0.1, 0.2, 0.3, 0.4)));
    }

    #[test]
    fn decode_layer() {
        check_round_trip_single(Draw::Layer(LayerId(21)));
    }

    #[test]
    fn decode_layer_blend() {
        check_round_trip_single(Draw::LayerBlend(LayerId(76), BlendMode::Lighten))
    }

    #[test]
    fn decode_layer_alpha() {
        check_round_trip_single(Draw::LayerAlpha(LayerId(75), 0.25));
    }

    #[test]
    fn decode_clear_layer() {
        check_round_trip_single(Draw::ClearLayer);
    }

    #[test]
    fn decode_clear_all_layers() {
        check_round_trip_single(Draw::ClearAllLayers);
    }

    #[test]
    fn decode_swap_layers() {
        check_round_trip_single(Draw::SwapLayers(LayerId(1), LayerId(2)));
    }

    #[test]
    fn decode_sprite() {
        check_round_trip_single(Draw::Sprite(SpriteId(0)));
        check_round_trip_single(Draw::Sprite(SpriteId(10)));
        check_round_trip_single(Draw::Sprite(SpriteId(1300)));
        check_round_trip_single(Draw::Sprite(SpriteId(1000000000)));
    }

    #[test]
    fn decode_clear_sprite() {
        check_round_trip_single(Draw::ClearSprite);
    }

    #[test]
    fn decode_transform_sprite_translate() {
        check_round_trip_single(Draw::SpriteTransform(SpriteTransform::Translate(4.0, 5.0)));
    }

    #[test]
    fn decode_transform_sprite_scale() {
        check_round_trip_single(Draw::SpriteTransform(SpriteTransform::Scale(6.0, 7.0)));
    }

    #[test]
    fn decode_transform_sprite_rotate() {
        check_round_trip_single(Draw::SpriteTransform(SpriteTransform::Rotate(42.0)));
    }

    #[test]
    fn decode_transform_sprite_transform() {
        check_round_trip_single(Draw::SpriteTransform(SpriteTransform::Transform2D(
            Transform2D::scale(3.0, 4.0),
        )));
    }

    #[test]
    fn decode_winding_rule() {
        check_round_trip_single(Draw::WindingRule(WindingRule::NonZero));
        check_round_trip_single(Draw::WindingRule(WindingRule::EvenOdd));
    }

    #[test]
    fn decode_draw_sprite() {
        check_round_trip_single(Draw::DrawSprite(SpriteId(0)));
        check_round_trip_single(Draw::DrawSprite(SpriteId(10)));
        check_round_trip_single(Draw::DrawSprite(SpriteId(1300)));
        check_round_trip_single(Draw::DrawSprite(SpriteId(1000000000)));
    }

    #[test]
    fn decode_draw_sprite_filtered() {
        check_round_trip_single(Draw::DrawSpriteWithFilters(SpriteId(10), vec![]));
        check_round_trip_single(Draw::DrawSpriteWithFilters(
            SpriteId(10),
            vec![TextureFilter::GaussianBlur(4.0)],
        ));
        check_round_trip_single(Draw::DrawSpriteWithFilters(
            SpriteId(10),
            vec![
                TextureFilter::GaussianBlur(4.0),
                TextureFilter::GaussianBlur(8.0),
            ],
        ));
    }

    #[test]
    fn will_accept_newlines() {
        let mut decoder = CanvasDecoder::new();
        assert!(decoder.decode('\n') == Ok(None));
        assert!(decoder.decode('\n') == Ok(None));
        assert!(decoder.decode('\n') == Ok(None));
        assert!(decoder.decode('N') == Ok(None));
        assert!(decoder.decode('p') == Ok(Some(Draw::Path(PathOp::NewPath))));
    }

    #[test]
    fn error_on_bad_char() {
        let mut decoder = CanvasDecoder::new();
        assert!(decoder.decode('N') == Ok(None));
        assert!(decoder.decode('x') == Err(DecoderError::InvalidCharacter('x')));
    }

    #[test]
    fn decode_font_data() {
        let font = CanvasFontFace::from_slice(include_bytes!("../test_data/Lato-Regular.ttf"));

        check_round_trip_single(Draw::Font(FontId(42), FontOp::UseFontDefinition(font)));
    }

    #[test]
    fn decode_font_size() {
        check_round_trip_single(Draw::Font(FontId(42), FontOp::FontSize(32.0)));
    }

    #[test]
    fn decode_begin_line_layout() {
        check_round_trip_single(Draw::BeginLineLayout(1.0, 2.0, TextAlignment::Center));
    }

    #[test]
    fn decode_perform_layout() {
        check_round_trip_single(Draw::DrawLaidOutText);
    }

    #[test]
    fn decode_layout_text() {
        check_round_trip_single(Draw::Font(
            FontId(42),
            FontOp::LayoutText("Test".to_string()),
        ));
    }

    #[test]
    fn decode_draw_glyphs() {
        check_round_trip_single(Draw::Font(
            FontId(42),
            FontOp::DrawGlyphs(vec![
                GlyphPosition {
                    id: GlyphId(20),
                    location: (2.0, 3.0),
                    em_size: 18.0,
                },
                GlyphPosition {
                    id: GlyphId(25),
                    location: (5.0, 3.0),
                    em_size: 18.0,
                },
                GlyphPosition {
                    id: GlyphId(700),
                    location: (9.0, 3.0),
                    em_size: 18.0,
                },
            ]),
        ));
    }

    #[test]
    fn decode_draw_text() {
        check_round_trip_single(Draw::DrawText(
            FontId(42),
            "Hello, world".to_string(),
            100.0,
            200.0,
        ));
    }

    #[test]
    fn decode_create_texture() {
        check_round_trip_single(Draw::Texture(
            TextureId(42),
            TextureOp::Create(TextureSize(100, 200), TextureFormat::Rgba),
        ));
    }

    #[test]
    fn decode_free_texture() {
        check_round_trip_single(Draw::Texture(TextureId(43), TextureOp::Free));
    }

    #[test]
    fn decode_texture_set_bytes() {
        check_round_trip_single(Draw::Texture(
            TextureId(44),
            TextureOp::SetBytes(
                TexturePosition(100, 200),
                TextureSize(300, 400),
                Arc::new(vec![240, 230, 220, 210, 200, 190]),
            ),
        ));
    }

    #[test]
    fn decode_texture_set_from_sprite() {
        check_round_trip_single(Draw::Texture(
            TextureId(44),
            TextureOp::SetFromSprite(
                SpriteId(42),
                SpriteBounds(SpritePosition(20.0, 30.0), SpriteSize(40.0, 50.0)),
            ),
        ));
    }

    #[test]
    fn decode_texture_create_dynamic_from_sprite() {
        check_round_trip_single(Draw::Texture(
            TextureId(44),
            TextureOp::CreateDynamicSprite(
                SpriteId(42),
                SpriteBounds(SpritePosition(20.0, 30.0), SpriteSize(40.0, 50.0)),
                CanvasSize(60.0, 70.0),
            ),
        ));
    }

    #[test]
    fn decode_fill_transparency() {
        check_round_trip_single(Draw::Texture(
            TextureId(45),
            TextureOp::FillTransparency(0.75),
        ));
    }

    #[test]
    fn decode_gradient_new() {
        check_round_trip_single(Draw::Gradient(
            GradientId(42),
            GradientOp::Create(Color::Rgba(0.1, 0.2, 0.3, 0.4)),
        ));
    }

    #[test]
    fn decode_gradient_add_stop() {
        check_round_trip_single(Draw::Gradient(
            GradientId(44),
            GradientOp::AddStop(0.5, Color::Rgba(0.1, 0.2, 0.3, 0.4)),
        ));
    }

    #[test]
    fn decode_gradient_fill() {
        check_round_trip_single(Draw::FillGradient(
            GradientId(24),
            (42.0, 43.0),
            (44.0, 45.0),
        ));
    }

    #[test]
    fn decode_fill_transform() {
        check_round_trip_single(Draw::FillTransform(Transform2D::identity()));
    }

    #[test]
    fn decode_texture_copy() {
        check_round_trip_single(Draw::Texture(TextureId(46), TextureOp::Copy(TextureId(47))));
    }

    #[test]
    fn decode_texture_filter_gaussian_blur() {
        check_round_trip_single(Draw::Texture(
            TextureId(47),
            TextureOp::Filter(TextureFilter::GaussianBlur(23.0)),
        ));
    }

    #[test]
    fn decode_texture_filter_alpha_blend() {
        check_round_trip_single(Draw::Texture(
            TextureId(47),
            TextureOp::Filter(TextureFilter::AlphaBlend(0.6)),
        ));
    }

    #[test]
    fn decode_texture_filter_mask() {
        check_round_trip_single(Draw::Texture(
            TextureId(47),
            TextureOp::Filter(TextureFilter::Mask(TextureId(48))),
        ));
    }

    #[test]
    fn decode_texture_filter_displacement_map() {
        check_round_trip_single(Draw::Texture(
            TextureId(47),
            TextureOp::Filter(TextureFilter::DisplacementMap(TextureId(48), 1.0, 2.0)),
        ));
    }

    #[test]
    fn decode_move_sprite_from() {
        check_round_trip_single(Draw::MoveSpriteFrom(SpriteId(48)));
    }

    #[test]
    fn decode_namespace() {
        check_round_trip_single(Draw::Namespace(NamespaceId::default()));
    }

    #[test]
    fn decode_all_iter() {
        check_round_trip(vec![
            Draw::Path(PathOp::NewPath),
            Draw::Path(PathOp::Move(10.0, 15.0)),
            Draw::Path(PathOp::Line(20.0, 42.0)),
            Draw::Path(PathOp::BezierCurve(((1.0, 2.0), (3.0, 4.0)), (5.0, 6.0))),
            Draw::Path(PathOp::ClosePath),
            Draw::Fill,
            Draw::Stroke,
            Draw::LineWidth(23.0),
            Draw::LineWidthPixels(43.0),
            Draw::LineJoin(LineJoin::Bevel),
            Draw::LineCap(LineCap::Round),
            Draw::WindingRule(WindingRule::NonZero),
            Draw::NewDashPattern,
            Draw::DashLength(56.0),
            Draw::DashOffset(13.0),
            Draw::StrokeColor(Color::Rgba(0.1, 0.2, 0.3, 0.4)),
            Draw::FillColor(Color::Rgba(0.2, 0.3, 0.4, 0.5)),
            Draw::FillTexture(TextureId(23), (42.0, 43.0), (44.0, 45.0)),
            Draw::FillGradient(GradientId(24), (42.0, 43.0), (44.0, 45.0)),
            Draw::FillTransform(Transform2D::identity()),
            Draw::BlendMode(BlendMode::Lighten),
            Draw::IdentityTransform,
            Draw::CanvasHeight(81.0),
            Draw::CenterRegion((6.0, 7.0), (8.0, 9.0)),
            Draw::MultiplyTransform(Transform2D([
                [1.0, 2.0, 3.0],
                [4.0, 5.0, 6.0],
                [7.0, 8.0, 9.0],
            ])),
            Draw::Unclip,
            Draw::Store,
            Draw::Restore,
            Draw::FreeStoredBuffer,
            Draw::PushState,
            Draw::PopState,
            Draw::ClearCanvas(Color::Rgba(0.1, 0.2, 0.3, 0.4)),
            Draw::Namespace(NamespaceId::default()),
            Draw::Layer(LayerId(21)),
            Draw::ClearLayer,
            Draw::ClearAllLayers,
            Draw::SwapLayers(LayerId(1), LayerId(2)),
            Draw::Path(PathOp::NewPath),
            Draw::Sprite(SpriteId(1000)),
            Draw::ClearSprite,
            Draw::SpriteTransform(SpriteTransform::Translate(4.0, 5.0)),
            Draw::SpriteTransform(SpriteTransform::Transform2D(Transform2D::scale(3.0, 4.0))),
            Draw::MoveSpriteFrom(SpriteId(48)),
            Draw::DrawSprite(SpriteId(1300)),
            Draw::DrawSpriteWithFilters(SpriteId(10), vec![]),
            Draw::DrawSpriteWithFilters(SpriteId(10), vec![TextureFilter::GaussianBlur(4.0)]),
            Draw::DrawSpriteWithFilters(
                SpriteId(10),
                vec![
                    TextureFilter::GaussianBlur(4.0),
                    TextureFilter::GaussianBlur(8.0),
                ],
            ),
            Draw::Texture(
                TextureId(42),
                TextureOp::Create(TextureSize(1024, 768), TextureFormat::Rgba),
            ),
            Draw::Texture(TextureId(43), TextureOp::Free),
            Draw::Texture(
                TextureId(44),
                TextureOp::SetBytes(
                    TexturePosition(2, 3),
                    TextureSize(4, 5),
                    Arc::new(vec![1, 2, 3, 4, 5]),
                ),
            ),
            Draw::Texture(
                TextureId(44),
                TextureOp::SetFromSprite(
                    SpriteId(42),
                    SpriteBounds(SpritePosition(20.0, 30.0), SpriteSize(40.0, 50.0)),
                ),
            ),
            Draw::Texture(
                TextureId(44),
                TextureOp::CreateDynamicSprite(
                    SpriteId(42),
                    SpriteBounds(SpritePosition(20.0, 30.0), SpriteSize(40.0, 50.0)),
                    CanvasSize(60.0, 70.0),
                ),
            ),
            Draw::Texture(TextureId(45), TextureOp::FillTransparency(0.5)),
            Draw::Texture(TextureId(46), TextureOp::Copy(TextureId(47))),
            Draw::Texture(
                TextureId(47),
                TextureOp::Filter(TextureFilter::GaussianBlur(23.0)),
            ),
            Draw::Texture(
                TextureId(47),
                TextureOp::Filter(TextureFilter::AlphaBlend(0.6)),
            ),
            Draw::Texture(
                TextureId(47),
                TextureOp::Filter(TextureFilter::Mask(TextureId(48))),
            ),
            Draw::Texture(
                TextureId(47),
                TextureOp::Filter(TextureFilter::DisplacementMap(TextureId(48), 1.0, 2.0)),
            ),
            Draw::Gradient(
                GradientId(42),
                GradientOp::Create(Color::Rgba(0.1, 0.2, 0.3, 0.4)),
            ),
            Draw::Gradient(
                GradientId(44),
                GradientOp::AddStop(0.5, Color::Rgba(0.1, 0.2, 0.3, 0.4)),
            ),
        ]);
    }

    #[test]
    fn decode_all_stream() {
        let all = vec![
            Draw::Path(PathOp::NewPath),
            Draw::Path(PathOp::Move(10.0, 15.0)),
            Draw::Path(PathOp::Line(20.0, 42.0)),
            Draw::Path(PathOp::BezierCurve(((1.0, 2.0), (3.0, 4.0)), (5.0, 6.0))),
            Draw::Path(PathOp::ClosePath),
            Draw::Fill,
            Draw::FillTexture(TextureId(42), (1.0, 2.0), (3.0, 4.0)),
            Draw::Stroke,
            Draw::LineWidth(23.0),
            Draw::LineWidthPixels(43.0),
            Draw::LineJoin(LineJoin::Bevel),
            Draw::LineCap(LineCap::Round),
            Draw::WindingRule(WindingRule::EvenOdd),
            Draw::NewDashPattern,
            Draw::DashLength(56.0),
            Draw::DashOffset(13.0),
            Draw::StrokeColor(Color::Rgba(0.1, 0.2, 0.3, 0.4)),
            Draw::FillColor(Color::Rgba(0.2, 0.3, 0.4, 0.5)),
            Draw::FillTexture(TextureId(23), (42.0, 43.0), (44.0, 45.0)),
            Draw::FillGradient(GradientId(24), (42.0, 43.0), (44.0, 45.0)),
            Draw::FillTransform(Transform2D::identity()),
            Draw::BlendMode(BlendMode::Lighten),
            Draw::IdentityTransform,
            Draw::CanvasHeight(81.0),
            Draw::CenterRegion((6.0, 7.0), (8.0, 9.0)),
            Draw::MultiplyTransform(Transform2D([
                [1.0, 2.0, 3.0],
                [4.0, 5.0, 6.0],
                [7.0, 8.0, 9.0],
            ])),
            Draw::Unclip,
            Draw::Store,
            Draw::Restore,
            Draw::FreeStoredBuffer,
            Draw::PushState,
            Draw::PopState,
            Draw::ClearCanvas(Color::Rgba(0.1, 0.2, 0.3, 0.4)),
            Draw::Namespace(NamespaceId::default()),
            Draw::Layer(LayerId(21)),
            Draw::LayerBlend(LayerId(22), BlendMode::DestinationOut),
            Draw::LayerAlpha(LayerId(23), 0.4),
            Draw::ClearLayer,
            Draw::ClearAllLayers,
            Draw::SwapLayers(LayerId(1), LayerId(2)),
            Draw::Path(PathOp::NewPath),
            Draw::Sprite(SpriteId(1000)),
            Draw::ClearSprite,
            Draw::SpriteTransform(SpriteTransform::Translate(4.0, 5.0)),
            Draw::SpriteTransform(SpriteTransform::Transform2D(Transform2D::scale(3.0, 4.0))),
            Draw::MoveSpriteFrom(SpriteId(48)),
            Draw::DrawSprite(SpriteId(1300)),
            Draw::DrawSpriteWithFilters(SpriteId(10), vec![]),
            Draw::DrawSpriteWithFilters(SpriteId(10), vec![TextureFilter::GaussianBlur(4.0)]),
            Draw::DrawSpriteWithFilters(
                SpriteId(10),
                vec![
                    TextureFilter::GaussianBlur(4.0),
                    TextureFilter::GaussianBlur(8.0),
                ],
            ),
            Draw::Texture(
                TextureId(42),
                TextureOp::Create(TextureSize(1024, 768), TextureFormat::Rgba),
            ),
            Draw::Texture(TextureId(43), TextureOp::Free),
            Draw::Texture(
                TextureId(44),
                TextureOp::SetBytes(
                    TexturePosition(2, 3),
                    TextureSize(4, 5),
                    Arc::new(vec![1, 2, 3, 4, 5]),
                ),
            ),
            Draw::Texture(
                TextureId(44),
                TextureOp::SetFromSprite(
                    SpriteId(42),
                    SpriteBounds(SpritePosition(20.0, 30.0), SpriteSize(40.0, 50.0)),
                ),
            ),
            Draw::Texture(
                TextureId(44),
                TextureOp::CreateDynamicSprite(
                    SpriteId(42),
                    SpriteBounds(SpritePosition(20.0, 30.0), SpriteSize(40.0, 50.0)),
                    CanvasSize(60.0, 70.0),
                ),
            ),
            Draw::Texture(TextureId(45), TextureOp::FillTransparency(0.5)),
            Draw::Texture(TextureId(46), TextureOp::Copy(TextureId(47))),
            Draw::Texture(
                TextureId(47),
                TextureOp::Filter(TextureFilter::GaussianBlur(23.0)),
            ),
            Draw::Texture(
                TextureId(47),
                TextureOp::Filter(TextureFilter::AlphaBlend(0.6)),
            ),
            Draw::Texture(
                TextureId(47),
                TextureOp::Filter(TextureFilter::Mask(TextureId(48))),
            ),
            Draw::Texture(
                TextureId(47),
                TextureOp::Filter(TextureFilter::DisplacementMap(TextureId(48), 1.0, 2.0)),
            ),
            Draw::Gradient(
                GradientId(42),
                GradientOp::Create(Color::Rgba(0.1, 0.2, 0.3, 0.4)),
            ),
            Draw::Gradient(
                GradientId(44),
                GradientOp::AddStop(0.5, Color::Rgba(0.1, 0.2, 0.3, 0.4)),
            ),
        ];
        let mut encoded = String::new();
        all.encode_canvas(&mut encoded);

        println!("{:?}", encoded);

        let all_stream = stream::iter(
            encoded
                .chars()
                .into_iter()
                .map(|c| -> Result<_, ()> { Ok(c) }),
        );
        let decoder = decode_drawing_stream(all_stream);
        let mut decoder = decoder;

        executor::block_on(async {
            let mut decoded = vec![];
            while let Some(next) = decoder.next().await {
                decoded.push(next);
            }

            println!(" -> {:?}", decoded);

            let all = all.into_iter().map(|item| Ok(item)).collect::<Vec<_>>();
            assert!(all == decoded);
        });
    }
}
