/// Kinds of edges that can be used in an edge plan
pub mod edges;

/// An edge plan divides a 2D spaces into regions using arbitrary edge definitions, and can be rendered down into a scan plan
pub mod edgeplan;

/// A scan plan describes the actions required to draw a single scanline (modelling a 1 dimensional space)
pub mod scanplan;

/// A pixel models a single colour sample (thematically it could be considered 0 dimensional, though really a pixel is better modelled as aggregation of the light passing through a particular region)
pub mod pixel;

/// Well-known pixel programs
pub mod pixel_programs;

/// Renderers convert from data represented by a series of instructions to a simpler form
pub mod render;

/// The 'draw' module converts from `flo_canvas::Draw` instructions to layered edge plans
pub mod draw;

pub use flo_canvas as canvas;
pub use flo_canvas::curves as curves;
