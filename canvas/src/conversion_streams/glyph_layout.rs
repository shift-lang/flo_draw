use crate::draw::*;
use crate::font::*;
use crate::font_line_layout::*;
use crate::namespace::*;

use flo_stream::*;

use futures::prelude::*;

use std::collections::HashMap;
use std::iter;
use std::mem;
use std::sync::*;

///
/// Given a stream with font instructions, replaces any layout instruction (eg, `Draw::DrawText()`) with glyph
/// rendering instructions
///
pub fn drawing_with_laid_out_text<InStream>(
    draw_stream: InStream,
) -> impl Send + Unpin + Stream<Item=Draw>
    where
        InStream: 'static + Send + Unpin + Stream<Item=Draw>,
{
    generator_stream(move |yield_value| async move {
        let mut draw_stream = draw_stream;

        // State of this stream
        let mut namespace_id = NamespaceId::default().local_id();
        let mut namespace_stack = vec![];
        let mut font_map = HashMap::new();
        let mut font_size = HashMap::new();
        let mut current_line = None;
        let mut current_font = None;
        let (mut x_pos, mut y_pos) = (0.0, 0.0);
        let mut alignment = TextAlignment::Left;

        // Read from the drawing stream
        while let Some(draw) = draw_stream.next().await {
            match draw {
                Draw::Namespace(new_namespace) => {
                    namespace_id = new_namespace.local_id();
                    yield_value(draw).await;
                }

                Draw::PushState => {
                    namespace_stack.push(namespace_id);
                    yield_value(draw).await;
                }

                Draw::PopState => {
                    if let Some(new_namespace) = namespace_stack.pop() {
                        namespace_id = new_namespace;
                    }
                    yield_value(draw).await;
                }

                Draw::Font(font_id, FontOp::UseFontDefinition(font_defn)) => {
                    // Defining new fonts interrupts any existing text layout
                    current_line = None;
                    current_font = None;

                    // Store this font definition
                    font_map.insert((namespace_id, font_id), Arc::clone(&font_defn));
                    font_size.insert(font_id, 12.0);

                    // Send the font to the next part of the stream
                    yield_value(Draw::Font(font_id, FontOp::UseFontDefinition(font_defn))).await;
                }

                Draw::Font(font_id, FontOp::FontSize(new_size)) => {
                    // If we're changing the size of the active font, restart the layout with the new size
                    if current_font == Some(font_id) {
                        current_line = current_line.map(|line: CanvasFontLineLayout| {
                            let font = line.font();
                            line.continue_with_new_font(font_id, &font, new_size)
                        });
                    }

                    // Set up the new size
                    font_size.insert(font_id, new_size);

                    yield_value(Draw::Font(font_id, FontOp::FontSize(new_size))).await;
                }

                Draw::BeginLineLayout(x, y, align) => {
                    // If we're laying out text already, this discards that layout
                    current_line = None;
                    current_font = None;

                    // Set up the layout for the next set of text
                    x_pos = x;
                    y_pos = y;
                    alignment = align;
                }

                Draw::Font(font_id, FontOp::LayoutText(text)) => {
                    // Update the current font
                    if current_font != Some(font_id) {
                        if let (Some(new_font), Some(font_size)) = (
                            font_map.get(&(namespace_id, font_id)),
                            font_size.get(&font_id),
                        ) {
                            let last_font = current_font.unwrap_or(FontId(0));
                            let new_font = Arc::clone(new_font);
                            let font_size = *font_size;

                            current_line = current_line
                                .map(|line: CanvasFontLineLayout| {
                                    line.continue_with_new_font(last_font, &new_font, font_size)
                                })
                                .or_else(|| Some(CanvasFontLineLayout::new(&new_font, font_size)));
                            current_font = Some(font_id);
                        }
                    }

                    // Lay out the text
                    current_line.as_mut().map(|line| line.add_text(&text));
                }

                Draw::DrawLaidOutText => {
                    if let Some(layout) = mem::take(&mut current_line) {
                        // Align the layout
                        let mut layout = layout;
                        layout.align(x_pos, y_pos, alignment);

                        if let Some(current_font) = mem::take(&mut current_font) {
                            // Convert to drawing actions, and send those
                            let drawing = layout.to_drawing(current_font);

                            for draw in drawing {
                                yield_value(draw).await;
                            }
                        }
                    }
                }

                Draw::FillColor(fill_color) => {
                    // This is added as a drawing instruction to the current layout
                    if let Some(current_line) = &mut current_line {
                        current_line.draw(iter::once(Draw::FillColor(fill_color.clone())));
                    }

                    yield_value(Draw::FillColor(fill_color)).await;
                }

                Draw::DrawText(font_id, text, x, y) => {
                    if let (Some(font), Some(font_size)) = (
                        font_map.get(&(namespace_id, font_id)),
                        font_size.get(&font_id),
                    ) {
                        // This is just a straightforward immediate layout of the text as glyphs
                        let mut layout = CanvasFontLineLayout::new(font, *font_size);

                        // Lay out the text
                        layout.add_text(&text);

                        // Align it at the requested position
                        layout.align(x, y, TextAlignment::Left);

                        // Convert to glyph drawing instructions, and send those on to the next stage
                        let drawing = layout.to_drawing(font_id);
                        for draw in drawing {
                            yield_value(draw).await;
                        }
                    }
                }

                Draw::Layer(_) => {
                    // These instructions interrupt text layout
                    current_line = None;
                    current_font = None;

                    yield_value(draw).await;
                }

                Draw::Sprite(_) => {
                    // These instructions interrupt text layout
                    current_line = None;
                    current_font = None;

                    yield_value(draw).await;
                }

                Draw::ClearLayer => {
                    // These instructions interrupt text layout
                    current_line = None;
                    current_font = None;

                    yield_value(draw).await;
                }

                Draw::ClearCanvas(_) => {
                    // Clear state
                    font_map = HashMap::new();
                    current_line = None;
                    current_font = None;
                    namespace_id = NamespaceId::default().local_id();

                    yield_value(draw).await;
                }

                // Default action is just to pass the drawing on
                _ => {
                    yield_value(draw).await;
                }
            }
        }
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::font_face::*;
    use futures::executor;
    use futures::stream;

    #[test]
    fn layout_hello_world() {
        executor::block_on(async {
            // Set up loading a font from a byte stream
            let lato =
                CanvasFontFace::from_slice(include_bytes!("../../test_data/Lato-Regular.ttf"));

            let instructions = vec![
                Draw::Font(FontId(1), FontOp::UseFontDefinition(lato)),
                Draw::Font(FontId(1), FontOp::FontSize(100.0)),
                Draw::BeginLineLayout(500.0, 500.0, TextAlignment::Left),
                Draw::Font(FontId(1), FontOp::LayoutText("Hello, world".to_string())),
                Draw::DrawLaidOutText,
            ];
            let instructions = stream::iter(instructions);
            let instructions = drawing_with_laid_out_text(instructions);

            let instructions = instructions.collect::<Vec<_>>().await;

            // Should get the font definition, font size and glyph layouts
            assert!(instructions.len() == 3);

            if let Draw::Font(FontId(1), FontOp::DrawGlyphs(glyphs)) = &instructions[2] {
                // Final instruction should be to draw the glyphs we just laid out
                println!("{:?}", instructions[2]);

                // 'Hello, world' has a simple shape so we should generate one glyph per character
                assert!(glyphs.len() == "Hello, world".len());

                // Glyph values and positions should be approximately these values
                fn dist((x1, y1): (f32, f32), (x2, y2): (f32, f32)) -> f32 {
                    let (x, y) = (x1 - x2, y1 - y2);
                    (x * x + y * y).sqrt()
                }

                assert!(glyphs[0].id == GlyphId(15));
                assert!(glyphs[0].em_size == 100.0);
                assert!(dist(glyphs[0].location, (500.0, 500.0)) < 1.0);

                assert!(glyphs[1].id == GlyphId(59));
                assert!(glyphs[2].id == GlyphId(1140));
                assert!(glyphs[3].id == GlyphId(1140));
                assert!(glyphs[4].id == GlyphId(111));
                assert!(glyphs[5].id == GlyphId(311));
                assert!(glyphs[6].id == GlyphId(2));
                assert!(glyphs[7].id == GlyphId(137));
                assert!(glyphs[8].id == GlyphId(111));
                assert!(glyphs[9].id == GlyphId(117));
                assert!(glyphs[10].id == GlyphId(1140));
                assert!(glyphs[11].id == GlyphId(55));

                assert!(dist(glyphs[1].location, (574.8, 500.0)) < 1.0);
                assert!(dist(glyphs[2].location, (627.6, 500.0)) < 1.0);
                assert!(dist(glyphs[3].location, (651.2, 500.0)) < 1.0);
                assert!(dist(glyphs[4].location, (674.8, 500.0)) < 1.0);
                assert!(dist(glyphs[5].location, (731.5, 500.0)) < 1.0);
                assert!(dist(glyphs[6].location, (754.2, 500.0)) < 1.0);
                assert!(dist(glyphs[7].location, (777.75, 500.0)) < 1.0);
                assert!(dist(glyphs[8].location, (855.6, 500.0)) < 1.0);
                assert!(dist(glyphs[9].location, (912.3, 500.0)) < 1.0);
                assert!(dist(glyphs[10].location, (948.7, 500.0)) < 1.0);
                assert!(dist(glyphs[11].location, (972.3, 500.0)) < 1.0);
            } else {
                // Not the expected layout instruction
                println!("{:?}", instructions[2]);
                assert!(false);
            }
        });
    }

    #[test]
    fn layout_hello_world_with_continue() {
        executor::block_on(async {
            // Set up loading a font from a byte stream
            let lato =
                CanvasFontFace::from_slice(include_bytes!("../../test_data/Lato-Regular.ttf"));

            let instructions = vec![
                Draw::Font(FontId(1), FontOp::UseFontDefinition(lato.clone())),
                Draw::Font(FontId(2), FontOp::UseFontDefinition(lato)),
                Draw::Font(FontId(1), FontOp::FontSize(100.0)),
                Draw::Font(FontId(2), FontOp::FontSize(100.0)),
                Draw::BeginLineLayout(500.0, 500.0, TextAlignment::Left),
                Draw::Font(FontId(1), FontOp::LayoutText("Hello, ".to_string())),
                Draw::Font(FontId(2), FontOp::LayoutText("world".to_string())),
                Draw::DrawLaidOutText,
            ];
            let instructions = stream::iter(instructions);
            let instructions = drawing_with_laid_out_text(instructions);

            let instructions = instructions.collect::<Vec<_>>().await;

            // Should get the font definition, font size and glyph layouts
            assert!(instructions.len() == 6);

            fn dist((x1, y1): (f32, f32), (x2, y2): (f32, f32)) -> f32 {
                let (x, y) = (x1 - x2, y1 - y2);
                (x * x + y * y).sqrt()
            }

            if let Draw::Font(FontId(1), FontOp::DrawGlyphs(glyphs)) = &instructions[4] {
                // Final instruction should be to draw the glyphs we just laid out
                println!("{:?}", instructions[4]);

                // 'Hello, world' has a simple shape so we should generate one glyph per character
                assert!(glyphs.len() == "Hello, ".len());

                // Glyph values and positions should be approximately these values
                assert!(glyphs[0].id == GlyphId(15));
                assert!(glyphs[0].em_size == 100.0);
                assert!(dist(glyphs[0].location, (500.0, 500.0)) < 1.0);

                assert!(glyphs[1].id == GlyphId(59));
                assert!(glyphs[2].id == GlyphId(1140));
                assert!(glyphs[3].id == GlyphId(1140));
                assert!(glyphs[4].id == GlyphId(111));
                assert!(glyphs[5].id == GlyphId(311));
                assert!(glyphs[6].id == GlyphId(2));

                assert!(dist(glyphs[1].location, (574.8, 500.0)) < 1.0);
                assert!(dist(glyphs[2].location, (627.6, 500.0)) < 1.0);
                assert!(dist(glyphs[3].location, (651.2, 500.0)) < 1.0);
                assert!(dist(glyphs[4].location, (674.8, 500.0)) < 1.0);
                assert!(dist(glyphs[5].location, (731.5, 500.0)) < 1.0);
                assert!(dist(glyphs[6].location, (754.2, 500.0)) < 1.0);
            } else {
                // Not the expected layout instruction
                println!("{:?}", instructions[4]);
                assert!(false);
            }

            if let Draw::Font(FontId(2), FontOp::DrawGlyphs(glyphs)) = &instructions[5] {
                println!("{:?}", instructions[5]);

                // 'world' is in the same font but as we changed font IDs we'll force the layout to restart
                assert!(glyphs.len() == "world".len());

                assert!(glyphs[0].id == GlyphId(137));
                assert!(glyphs[1].id == GlyphId(111));
                assert!(glyphs[2].id == GlyphId(117));
                assert!(glyphs[3].id == GlyphId(1140));
                assert!(glyphs[4].id == GlyphId(55));

                assert!(dist(glyphs[0].location, (779.8, 500.0)) < 1.0);
                assert!(dist(glyphs[1].location, (857.7, 500.0)) < 1.0);
                assert!(dist(glyphs[2].location, (914.4, 500.0)) < 1.0);
                assert!(dist(glyphs[3].location, (950.8, 500.0)) < 1.0);
                assert!(dist(glyphs[4].location, (974.4, 500.0)) < 1.0);
            } else {
                // Not the expected layout instruction
                println!("{:?}", instructions[5]);
                assert!(false);
            }
        });
    }
}
