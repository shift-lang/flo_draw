/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::context::*;
use crate::draw::*;
use crate::font::*;
use crate::font_face::*;
use crate::transform2d::*;

use flo_curves::geo::*;

use allsorts::font::MatchingPresentation;
use allsorts::gpos;
use allsorts::gsub;
use allsorts::tag;

use std::mem;
use std::sync::*;

/// Actions that can be performed in a layout
#[derive(Clone)]
enum LayoutAction {
    /// Render a glyph at a particular position
    Glyph(GlyphPosition),

    /// Render drawing instructions (eg, changing fill colour)
    Draw(Draw),
}

///
/// Performs layout of text along a line. The `outline_fonts` feature must be enabled to use this data type.
///
/// This includes optional drawing operations in between glyphs to allow for
///
#[derive(Clone)]
pub struct CanvasFontLineLayout {
    /// The font that this layout is for
    font: Arc<CanvasFontFace>,

    /// Metrics for the text we've laid out
    metrics: TextLayoutMetrics,

    /// Number of font units per em
    units_per_em: f32,

    /// X-offset
    x_off: f32,

    /// Y-offset
    y_off: f32,

    /// em-size
    em_size: f32,

    /// Characters still pending layout
    pending: String,

    /// Layout so far
    layout: Vec<LayoutAction>,
}

impl CanvasFontLineLayout {
    ///
    /// Creates a new line layout.
    ///
    pub fn new(font: &Arc<CanvasFontFace>, em_size: f32) -> CanvasFontLineLayout {
        // Gather font info
        let ttf_font = font.ttf_font();
        let units_per_em = ttf_font.units_per_em() as f32;

        // Generate the initial font metrics
        let scale_factor = (em_size / units_per_em) as f64;
        let ascent = ttf_font.ascender() as f64;
        let descent = ttf_font.descender() as f64;
        let inner_bounds = (
            Coord2(0.0, descent * scale_factor),
            Coord2(0.0, ascent * scale_factor),
        );

        let initial_metrics = TextLayoutMetrics {
            inner_bounds: inner_bounds,
            pos: Coord2(0.0, 0.0),
        };

        CanvasFontLineLayout {
            font: Arc::clone(font),
            units_per_em: units_per_em,
            metrics: initial_metrics,
            x_off: 0.0,
            y_off: 0.0,
            em_size: em_size,
            pending: String::new(),
            layout: vec![],
        }
    }

    ///
    /// The font that is currently being laid out
    ///
    pub fn font(&self) -> Arc<CanvasFontFace> {
        Arc::clone(&self.font)
    }

    ///
    /// Update the rendering between the glyphs
    ///
    pub fn draw<DrawIter: IntoIterator<Item = Draw>>(&mut self, drawing: DrawIter) {
        self.layout_pending();
        self.layout
            .extend(drawing.into_iter().map(|item| LayoutAction::Draw(item)));
    }

    ///
    /// Adds some text to be laid out at the current offset
    ///
    pub fn add_text(&mut self, text: &str) {
        self.pending.extend(text.chars())
    }

    ///
    /// Manually advance where the next glyph will be placed after the current position
    ///
    pub fn advance(&mut self, x_offset: f32, y_offset: f32) {
        self.layout_pending();
        self.x_off += x_offset;
        self.y_off += y_offset;
    }

    ///
    /// Measures the text that's been laid out so far
    ///
    /// (Note that this will perform a layout so it's usually best to call before converting to drawing instructions or glyphs)
    ///
    pub fn measure(&mut self) -> TextLayoutMetrics {
        self.layout_pending();

        self.metrics.pos = Coord2(self.x_off as _, self.y_off as _);
        self.metrics.clone()
    }

    ///
    /// Aligns the glyphs according to a text alignment around a specific position
    ///
    /// Doesn't adjust the metrics, if you continue layout after this call, new glyphs will be
    /// positioned at the current baseline position, so this is usually only useful before finishing
    /// the text layout.
    ///
    pub fn align(&mut self, x: f32, y: f32, align: TextAlignment) {
        // Finish laying out any text that hasn't yet been laid out
        self.layout_pending();

        // We want to apply a constant offset to all of the glyphs: we can calculate this based on the inner bounds of the text
        let (Coord2(min_x, _min_y), Coord2(max_x, _max_y)) = self.metrics.inner_bounds;
        let (min_x, max_x) = (min_x as f32, max_x as f32);

        let y_offset = y;
        let x_offset = match align {
            TextAlignment::Left => x,
            TextAlignment::Right => x - max_x,
            TextAlignment::Center => x - (max_x + min_x) / 2.0,
        };

        // Move all of the glyph positions
        self.layout.iter_mut().for_each(|action| {
            match action {
                LayoutAction::Glyph(pos) => {
                    pos.location.0 += x_offset;
                    pos.location.1 += y_offset;
                }

                LayoutAction::Draw(Draw::Font(_, FontOp::DrawGlyphs(glyphs))) => {
                    // Assume that these were generated during a 'continue' call and not added by 'draw'
                    // (or at least, if they were added by 'draw', assume they want to be aligned with everything else)
                    glyphs.iter_mut().for_each(|pos| {
                        pos.location.0 += x_offset;
                        pos.location.1 += y_offset;
                    })
                }

                _ => {}
            }
        });
    }

    ///
    /// Aligns the glyphs according to a text alignment around a specific position, using a canvas transform
    ///
    /// This is useful if the text has been annotated with other drawings as it makes it possible to draw using
    /// the values in the metrics returned by 'measure'
    ///
    /// Doesn't adjust the metrics, if you continue layout after this call, new glyphs will be
    /// positioned at the current baseline position, so this is usually only useful before finishing
    /// the text layout.
    ///
    pub fn align_transform(&mut self, x: f32, y: f32, align: TextAlignment) {
        // Finish laying out any text that hasn't yet been laid out
        self.layout_pending();

        // We want to apply a constant offset to all of the glyphs: we can calculate this based on the inner bounds of the text
        let (Coord2(min_x, _min_y), Coord2(max_x, _max_y)) = self.metrics.inner_bounds;
        let (min_x, max_x) = (min_x as f32, max_x as f32);

        let y_offset = y;
        let x_offset = match align {
            TextAlignment::Left => x,
            TextAlignment::Right => x - max_x,
            TextAlignment::Center => x - (max_x + min_x) / 2.0,
        };

        // Add transform instructions at the start of the drawing, then restore the previous state at the end
        self.layout.splice(
            0..0,
            vec![
                LayoutAction::Draw(Draw::PushState),
                LayoutAction::Draw(Draw::MultiplyTransform(Transform2D::translate(
                    x_offset, y_offset,
                ))),
            ],
        );
        self.layout.push(LayoutAction::Draw(Draw::PopState));
    }

    ///
    /// Finishes the layout and returns a list of glyph positions (any drawing instructions are discarded)
    ///
    pub fn to_glyphs(mut self) -> Vec<GlyphPosition> {
        // Finish the layout
        self.layout_pending();

        // Generate the glyphs
        self.layout
            .into_iter()
            .flat_map(|action| match action {
                LayoutAction::Glyph(glyph) => Some(glyph),
                _ => None,
            })
            .collect()
    }

    ///
    /// Finishes the layout and returns the drawing instructions
    ///
    pub fn to_drawing(mut self, font_id: FontId) -> Vec<Draw> {
        // Finish the layout
        self.layout_pending();

        let mut draw = vec![];
        let mut glyphs = vec![];

        for action in self.layout.into_iter() {
            match action {
                LayoutAction::Glyph(glyph) => glyphs.push(glyph),
                LayoutAction::Draw(drawing) => {
                    // Draw any glyphs that are pending
                    let draw_glyphs = mem::take(&mut glyphs);
                    if draw_glyphs.len() > 0 {
                        draw.push(Draw::Font(font_id, FontOp::DrawGlyphs(draw_glyphs)));
                    }

                    // Followed up by the drawing action
                    draw.push(drawing);
                }
            }
        }

        // Remaining glyphs
        if glyphs.len() > 0 {
            draw.push(Draw::Font(font_id, FontOp::DrawGlyphs(glyphs)));
        }

        draw
    }

    ///
    /// Continues the layout with a new font
    ///
    /// `last_font_id` should be the ID of the font that the glyphs that have been rendered so far should be rendered in
    ///
    pub fn continue_with_new_font(
        mut self,
        last_font_id: FontId,
        new_font: &Arc<CanvasFontFace>,
        new_em_size: f32,
    ) -> CanvasFontLineLayout {
        // Layout the pending text before continuing
        self.layout_pending();

        // Finish the current layout by generating the drawing actions, and remember the state
        let x_off = self.x_off;
        let y_off = self.y_off;
        let metrics = self.metrics.clone();
        let drawing = self.to_drawing(last_font_id);

        // Create a new layout with the new font
        let mut new_layout = CanvasFontLineLayout::new(new_font, new_em_size);

        // Set it up to continue where the existing layout left off
        new_layout.layout = drawing
            .into_iter()
            .map(|draw| LayoutAction::Draw(draw))
            .collect();
        new_layout.x_off = x_off;
        new_layout.y_off = y_off;

        new_layout.metrics.inner_bounds = new_layout
            .metrics
            .inner_bounds
            .union_bounds(metrics.inner_bounds);

        new_layout
    }

    ///
    /// Performs layout on the pending string
    ///
    fn layout_pending(&mut self) {
        // Nothing to do if nothing is pending
        if self.pending.len() == 0 {
            return;
        }

        // Take the pending characters to be processed
        let pending = mem::take(&mut self.pending);

        // Shape the pending text
        let ttf_font = self.font.ttf_font();
        let mut shaper = self.font.allsorts_font();
        let glyphs = shaper.map_glyphs(&pending, tag::LATN, MatchingPresentation::NotRequired);
        let shape = shaper
            .shape(
                glyphs,
                tag::LATN,
                Some(tag::DFLT),
                &gsub::Features::Mask(gsub::FeatureMask::default()),
                true,
            )
            .ok()
            .unwrap_or_else(|| vec![]);

        // The scale factor is used to convert between font units and screen units
        let scale_factor = self.em_size / self.units_per_em;

        // Generate the glyph positions
        for glyph in shape {
            // Fetch information about this glyph
            let glyph_index = ttf_parser::GlyphId(glyph.glyph.glyph_index as _);
            let advance_x = ttf_font.glyph_hor_advance(glyph_index);
            let advance_y = ttf_font.glyph_ver_advance(glyph_index);
            let advance_x = if let Some(advance) = advance_x {
                advance
            } else {
                0
            };
            let advance_y = if let Some(advance) = advance_y {
                advance
            } else {
                0
            };

            // Adjust by any requested offset
            let (off_x, off_y) = match glyph.placement {
                gpos::Placement::None => (0.0, 0.0),
                gpos::Placement::Distance(x, y) => (x as f32, y as f32),
                gpos::Placement::MarkAnchor(_, _, _) => (0.0, 0.0), // TODO
                gpos::Placement::CursiveAnchor(_, _, _, _) => (0.0, 0.0), // TODO: https://docs.microsoft.com/en-us/typography/opentype/spec/gpos#lookup-type-3-cursive-attachment-positioning-subtable
                gpos::Placement::MarkOverprint(_) => (0.0, 0.0),          // TODO
            };
            let off_x = off_x * scale_factor;
            let off_y = off_y * scale_factor;

            // Push this glyph
            let glyph_pos = GlyphPosition {
                id: GlyphId(glyph.glyph.glyph_index as _),
                location: (self.x_off + off_x, self.y_off + off_y),
                em_size: self.em_size,
            };
            self.layout.push(LayoutAction::Glyph(glyph_pos));

            // Move to the next position
            let advance_x = (advance_x as f32) + (glyph.kerning as f32);
            let advance_y = advance_y as f32;
            let advance_x = advance_x * scale_factor;
            let advance_y = advance_y * scale_factor;

            let last_x = self.x_off;
            let last_y = self.y_off;

            self.x_off += advance_x + off_x;
            self.y_off += advance_y + off_y;

            // The inner bounds just uses the x, y offsets to amend the bounding box
            self.metrics.inner_bounds = self.metrics.inner_bounds.union_bounds((
                Coord2(last_x as _, last_y as _),
                Coord2(self.x_off as _, self.y_off as _),
            ));
        }
    }
}

impl GraphicsContext for CanvasFontLineLayout {
    #[inline]
    fn draw(&mut self, drawing: Draw) {
        self.layout_pending();
        self.layout.push(LayoutAction::Draw(drawing));
    }
}
