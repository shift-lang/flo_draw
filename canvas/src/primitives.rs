use super::draw::*;
use super::context::*;
use super::texture::*;

use flo_curves::*;
use flo_curves::arc;
use flo_curves::bezier::{BezierCurve};
use flo_curves::bezier::path::{BezierPath};

use std::iter;

#[cfg(feature = "image-loading")] use image;
#[cfg(feature = "image-loading")] use image::io::Reader as ImageReader;
#[cfg(feature = "image-loading")] use std::io;
#[cfg(feature = "image-loading")] use std::sync::*;

///
/// GraphicsPrimitives adds new primitives that can be built directly from a graphics context
///
pub trait GraphicsPrimitives : GraphicsContext {
    ///
    /// Draws a rectangle between particular coordinates
    ///
    fn rect(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) {
        for d in draw_rect(x1, y1, x2, y2) {
            self.draw(d);
        }
    }

    ///
    /// Draws a circle at a particular point
    ///
    fn circle(&mut self, center_x: f32, center_y: f32, radius: f32) {
        for d in draw_circle(center_x, center_y, radius) {
            self.draw(d);
        }
    }

    ///
    /// Draws a bezier path
    ///
    fn bezier_path<TPath: BezierPath>(&mut self, path: &TPath)
    where TPath::Point: Coordinate2D {
        let start_point = path.start_point();

        self.move_to(start_point.x() as _, start_point.y() as _);
        for (cp1, cp2, end) in path.points() {
            self.bezier_curve_to(end.x() as _, end.y() as _, cp1.x() as _, cp1.y() as _, cp2.x() as _, cp2.y() as _);
        }
    }

    ///
    /// Draws a bezier curve (defined by the BezierCurve trait)
    ///
    fn bezier_curve<TCurve: BezierCurve>(&mut self, curve: &TCurve)
    where TCurve::Point: Coordinate2D {
        let (cp1, cp2)  = curve.control_points();
        let end         = curve.end_point();

        self.bezier_curve_to(end.x() as _, end.y() as _, cp1.x() as _, cp1.y() as _, cp2.x() as _, cp2.y() as _);
    }

    ///
    /// Draws a series of instructions
    ///
    fn draw_list<'a, DrawIter: 'a+IntoIterator<Item=Draw>>(&'a mut self, drawing: DrawIter) {
        for d in drawing.into_iter() {
            self.draw(d);
        }
    }

    ///
    /// Loads an image from an IO stream into a texture, returning the size (or None if the image can't be read for any reason)
    ///
    #[cfg(feature = "image-loading")]
    fn load_texture<TSrc: io::BufRead+io::Read+io::Seek>(&mut self, texture_id: TextureId, data: TSrc) -> Option<(usize, usize)> {
        // Load the image
        let img         = ImageReader::new(data).decode().ok()?;

        // Convert to 8-bit RGBA
        let img         = img.into_rgba8();
        let width       = img.width();
        let height      = img.height();

        // Load the texture
        let raw_pixels  = Arc::new(img.into_raw());
        self.create_texture(texture_id, width, height, TextureFormat::Rgba);
        self.set_texture_bytes(texture_id, 0, 0, width, height, raw_pixels);

        // Result is the image size
        Some((width as _, height as _))
    }
}

///
/// Returns the drawing commands for a rectangle
///
pub fn draw_rect(x1: f32, y1: f32, x2: f32, y2: f32) -> Vec<Draw> {
    use self::Draw::*;

    vec![
        Move(x1, y1),
        Line(x1, y2),
        Line(x2, y2),
        Line(x2, y1),
        Line(x1, y1),
        ClosePath
    ]
}

///
/// Returns the drawing commands for a circle
///
pub fn draw_circle(center_x: f32, center_y: f32, radius: f32) -> Vec<Draw> {
    use self::Draw::*;

    // Generate the circle and turn it into bezier curves
    let circle                          = arc::Circle::new(Coord2(center_x as f64, center_y as f64), radius as f64);
    let curves: Vec<bezier::Curve<_>>   = circle.to_curves();
    let start_point                     = curves[0].start_point();

    // Draw the curves
    let curves  = curves.into_iter().map(|curve| Draw::from(&curve));

    // Complete the path
    let path    = iter::once(Move(start_point.x() as f32, start_point.y() as f32))
        .chain(curves)
        .chain(iter::once(ClosePath));

    path.collect()
}

impl<'a, Curve: BezierCurve> From<&'a Curve> for Draw
where Curve::Point: Coordinate2D {
    fn from(curve: &'a Curve) -> Draw {
        let end         = curve.end_point();
        let (cp1, cp2)  = curve.control_points();

        Draw::BezierCurve(
            (end.x() as f32, end.y() as f32),
            (cp1.x() as f32, cp1.y() as f32),
            (cp2.x() as f32, cp2.y() as f32))
    }
}

///
/// All graphics contexts provide graphics primitives
///
impl<T> GraphicsPrimitives for T
where T: GraphicsContext {

}

///
/// The dynamic graphics context object also implements the graphics primitives
///
impl<'a> GraphicsPrimitives for dyn 'a+GraphicsContext {

}
