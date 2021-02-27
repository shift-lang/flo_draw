#[cfg(feature = "outline-fonts")] use allsorts;
#[cfg(feature = "outline-fonts")] use allsorts::font;
#[cfg(feature = "outline-fonts")] use allsorts::error::{ParseError};
#[cfg(feature = "outline-fonts")] use allsorts::tables::{FontTableProvider};
#[cfg(feature = "outline-fonts")] use ttf_parser;

use std::marker::{PhantomPinned};
use std::fmt;
use std::slice;
use std::pin::*;
use std::sync::*;
use std::borrow::{Cow};

/// allsorts table provider implementation based on a unsafe (based on lifetime) pointer to a TTF parser face
pub struct CanvasTableProvider<'a>(&'a ttf_parser::Face<'a>);

impl<'b> FontTableProvider for CanvasTableProvider<'b> {
    fn table_data<'a>(&'a self, tag: u32) -> Result<Option<Cow<'a, [u8]>>, ParseError> {
        let table_data = self.0.table_data(ttf_parser::Tag::from_bytes(&tag.to_le_bytes()));
        let table_data = table_data.map(|data| Cow::Borrowed(data));

        Ok(table_data)
    }

    fn has_table<'a>(&'a self, tag: u32) -> bool {
        let table_data = self.0.table_data(ttf_parser::Tag::from_bytes(&tag.to_le_bytes()));
        table_data.is_some()
    }
}

///
/// Representation of a font face
///
/// This class acquires more features if the `outline-fonts` feature is turned on for
/// this crate.
///
pub struct CanvasFontFace {
    /// Data for this font face
    data: Arc<Pin<Box<[u8]>>>,

    /// The font face for the data
    #[cfg(feature = "outline-fonts")] ttf_font: Option<Pin<Box<ttf_parser::Face<'static>>>>,

    /// The font face is pinned: Allsorts and ttf-parser both need to be able to refer to it
    _pinned: PhantomPinned
}

impl CanvasFontFace {
    ///
    /// Creates a new font by loading the fonts from a slice
    ///
    pub fn from_slice(bytes: &[u8]) -> Arc<CanvasFontFace> {
        Self::from_bytes(Vec::from(bytes))
    }

    ///
    /// Creates a new font by loading the fonts from a byte array
    ///
    pub fn from_bytes(bytes: Vec<u8>) -> Arc<CanvasFontFace> {
        // Pin the data for this font face
        let data = bytes.into_boxed_slice();
        Self::from_pinned(Arc::new(data.into()), 0)
    }

    #[cfg(not(feature = "outline-fonts"))]
    fn from_pinned(data: Arc<Pin<Box<[u8]>>>) -> Arc<CanvasFontFace> {
        // Generate the font face
        Arc::new(CanvasFontFace {
            data:       data,
            _pinned:    PhantomPinned
        })
    }

    #[cfg(feature = "outline-fonts")]
    fn from_pinned(data: Arc<Pin<Box<[u8]>>>, font_index: u32) -> Arc<CanvasFontFace> {
        // Create the data pointer
        let len             = data.len();
        let slice           = data.as_ptr();

        // Load into the TTF parser with scary unsafe self-referential data
        let mut font_face   = CanvasFontFace {
            data:           data,

            ttf_font:       None,
            _pinned:        PhantomPinned
        };

        // TODO: is there a better way? TTF-parser requries a reference to data which either means we need to do this
        // or reload the font every time we use it (which might be OK for large amounts of layout work but probably
        // isn't what we want for reading single glyphs)
        //
        // This 'should' be safe, I think. We've declared the TTF font as 'static but we've pinned it so that it can't
        // be moved away from this structure which manages the lifetime of its owning data. Later on, we force it to be
        // dropped ahead of the data so we're sure that the face no longer exists at the point we drop the data itself.
        //
        // (For allsorts, it seems we can probably implement `FontTableProvider` for an Arc<[u8]> quite easily, but for
        // ttf_parser, it's not really clear how to make a 'static version of FaceTables)
        let ttf_font        = ttf_parser::Face::from_slice(unsafe { slice::from_raw_parts(slice, len) }, font_index as _).unwrap();

        font_face.ttf_font  = Some(Box::pin(ttf_font));

        // Generate the font face
        Arc::new(font_face)
    }

    ///
    /// Retrieves the data bytes for this font
    ///
    pub fn font_data<'a>(&'a self) -> &'a [u8] {
        &**self.data
    }
}

impl PartialEq for CanvasFontFace {
    fn eq(&self, other: &CanvasFontFace) -> bool {
        self.data.eq(&other.data)
    }
}

impl fmt::Debug for CanvasFontFace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CanvasFontFace")
         .field("data", &self.data)
         .finish()
    }
}

#[cfg(feature = "outline-fonts")]
impl Drop for CanvasFontFace {
    fn drop(&mut self) {
        // Ensure that the TTF font is dropped before we free the data it's using
        self.ttf_font       = None;

        // Now safe to drop data as nothing is using it
    }
}

#[cfg(feature = "outline-fonts")]
impl CanvasFontFace {
    ///
    /// Retrieves the TTF font face for this font
    ///
    pub fn ttf_font<'a>(&'a self) -> &'a ttf_parser::Face<'a> {
        &**self.ttf_font.as_ref().unwrap()
    }
}

///
/// See `allsorts` for what these functions do
///
#[cfg(feature = "outline-fonts")]
impl CanvasFontFace {
    ///
    /// Creates a TTF font face for this font
    ///
    pub fn allsorts_font<'a>(&'a self) -> allsorts::Font<CanvasTableProvider<'a>> {
        let face            = self.ttf_font();
        let table_provider  = CanvasTableProvider(face);

        allsorts::Font::new(table_provider)
            .expect("unable to load font tables")
            .expect("unable to find suitable cmap sub-table")
    }
}
