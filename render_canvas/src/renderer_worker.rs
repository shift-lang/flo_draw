/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use lyon::path;
use lyon::tessellation;
use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillRule, FillVertex, Side, StrokeOptions, StrokeVertex,
    VertexBuffers,
};

use flo_canvas as canvas;
use flo_render as render;

use super::fill_state::*;
use super::layer_handle::*;
use super::render_entity::*;
use super::render_entity_details::*;
use super::stroke_settings::*;

/// The minimum tolerance to use when rendering fills/strokes
const MIN_TOLERANCE: f32 = 0.0001;

/// The maximum tolerance to use when rendering fills/strokes
const MAX_TOLERANCE: f32 = 1000.0;

///
/// References an entity in a layer
///
#[derive(Clone, Copy)]
pub struct LayerEntityRef {
    pub layer_id: LayerHandle,
    pub entity_index: usize,
    pub entity_id: usize,
}

///
/// Describes a job for a canvas worker
///
pub enum CanvasJob {
    ///
    /// Tessellates a path by filling it, generating a 'Fill' instruction that covers the path's interior
    ///
    Fill {
        path: path::Path,
        color: FillState,
        fill_rule: FillRule,
        scale_factor: f64,
        transform: canvas::Transform2D,
        entity: LayerEntityRef,
    },

    ///
    /// Tesselates a path by filling its edge, generating a a 'Fill' instruction that draws the edges of the path as a stroke
    ///
    Stroke {
        path: path::Path,
        stroke_options: StrokeSettings,
        scale_factor: f64,
        transform: canvas::Transform2D,
        entity: LayerEntityRef,
    },

    ///
    /// Tessellates a path by filling it, generating a 'EnableClipping' instruction that covers the path's interior
    ///
    Clip {
        path: path::Path,
        color: render::Rgba8,
        fill_rule: FillRule,
        scale_factor: f64,
        transform: canvas::Transform2D,
        entity: LayerEntityRef,
    },
}

///
/// State of a canvas worker
///
pub struct CanvasWorker {}

impl CanvasWorker {
    ///
    /// Creates a new canvas worker
    ///
    pub fn new() -> CanvasWorker {
        CanvasWorker {}
    }

    ///
    /// Processes a single tessellation job (returning a vertex buffer entity)
    ///
    pub fn process_job(
        &mut self,
        job: CanvasJob,
    ) -> (LayerEntityRef, RenderEntity, RenderEntityDetails) {
        use self::CanvasJob::*;

        match job {
            Fill {
                path,
                fill_rule,
                color,
                scale_factor,
                transform,
                entity,
            } => self.fill(
                path,
                fill_rule,
                color.flat_color(),
                scale_factor,
                transform,
                entity,
            ),
            Clip {
                path,
                fill_rule,
                color,
                scale_factor,
                transform,
                entity,
            } => self.clip(path, fill_rule, color, scale_factor, transform, entity),
            Stroke {
                path,
                stroke_options,
                scale_factor,
                transform,
                entity,
            } => self.stroke(path, stroke_options, scale_factor, transform, entity),
        }
    }

    ///
    /// Fills a path and returns the resulting render geometry
    ///
    fn fill_geometry(
        &mut self,
        path: path::Path,
        fill_rule: FillRule,
        render::Rgba8(color): render::Rgba8,
        scale_factor: f64,
    ) -> VertexBuffers<render::Vertex2D, u16> {
        // Create the tessellator and geometry
        let mut tessellator = tessellation::FillTessellator::new();
        let mut geometry = VertexBuffers::new();

        // Set up the fill options
        let mut fill_options = FillOptions::default();
        fill_options.fill_rule = fill_rule;
        fill_options.tolerance = FillOptions::DEFAULT_TOLERANCE * (scale_factor as f32);
        fill_options.tolerance = f32::min(MAX_TOLERANCE, fill_options.tolerance);
        fill_options.tolerance = f32::max(MIN_TOLERANCE, fill_options.tolerance);

        // Tessellate the current path
        tessellator
            .tessellate_path(
                &path,
                &fill_options,
                &mut BuffersBuilder::new(&mut geometry, move |vertex: FillVertex| {
                    render::Vertex2D {
                        pos: vertex.position().to_array(),
                        tex_coord: [0.0, 0.0],
                        color: color,
                    }
                }),
            )
            .unwrap();

        geometry
    }

    ///
    /// Fills the current path and returns the resulting render entity
    ///
    fn fill(
        &mut self,
        path: path::Path,
        fill_rule: FillRule,
        render::Rgba8(color): render::Rgba8,
        scale_factor: f64,
        transform: canvas::Transform2D,
        entity: LayerEntityRef,
    ) -> (LayerEntityRef, RenderEntity, RenderEntityDetails) {
        let geometry = self.fill_geometry(path, fill_rule, render::Rgba8(color), scale_factor);
        let details = RenderEntityDetails::from_vertices(&geometry.vertices, &transform);

        (
            entity,
            RenderEntity::VertexBuffer(geometry, VertexBufferIntent::Draw),
            details,
        )
    }

    ///
    /// Fills the current path and returns the resulting render entity
    ///
    fn clip(
        &mut self,
        path: path::Path,
        fill_rule: FillRule,
        render::Rgba8(color): render::Rgba8,
        scale_factor: f64,
        transform: canvas::Transform2D,
        entity: LayerEntityRef,
    ) -> (LayerEntityRef, RenderEntity, RenderEntityDetails) {
        let geometry = self.fill_geometry(path, fill_rule, render::Rgba8(color), scale_factor);
        let details = RenderEntityDetails::from_vertices(&geometry.vertices, &transform);

        (
            entity,
            RenderEntity::VertexBuffer(geometry, VertexBufferIntent::Clip),
            details,
        )
    }

    ///
    /// Converts some stroke settings to Lyon stroke options
    ///
    fn convert_stroke_settings(stroke_settings: StrokeSettings) -> StrokeOptions {
        let mut stroke_options = StrokeOptions::default();

        stroke_options.line_width = stroke_settings.line_width;
        stroke_options.end_cap = match stroke_settings.cap {
            canvas::LineCap::Butt => tessellation::LineCap::Butt,
            canvas::LineCap::Square => tessellation::LineCap::Square,
            canvas::LineCap::Round => tessellation::LineCap::Round,
        };
        stroke_options.start_cap = stroke_options.end_cap;
        stroke_options.line_join = match stroke_settings.join {
            canvas::LineJoin::Miter => tessellation::LineJoin::Miter,
            canvas::LineJoin::Bevel => tessellation::LineJoin::Bevel,
            canvas::LineJoin::Round => tessellation::LineJoin::Round,
        };

        stroke_options
    }

    ///
    /// Generates the geometry for a stroke
    ///
    fn stroke_geometry(
        &mut self,
        path: path::Path,
        stroke_options: StrokeSettings,
        scale_factor: f64,
    ) -> VertexBuffers<render::Vertex2D, u16> {
        // Create the tessellator and geometry
        let mut tessellator = tessellation::StrokeTessellator::new();
        let mut geometry = VertexBuffers::new();

        // Set up the stroke options
        let render::Rgba8(color) = stroke_options.stroke_color;
        let mut stroke_options = Self::convert_stroke_settings(stroke_options);
        stroke_options.tolerance = StrokeOptions::DEFAULT_TOLERANCE * (scale_factor as f32);
        stroke_options.tolerance = f32::min(MAX_TOLERANCE, stroke_options.tolerance);
        stroke_options.tolerance = f32::max(MIN_TOLERANCE, stroke_options.tolerance);

        // Stroke the path
        // TODO: 'TooManyVertices'
        tessellator
            .tessellate_path(
                &path,
                &stroke_options,
                &mut BuffersBuilder::new(&mut geometry, move |point: StrokeVertex| {
                    let advancement = point.advancement();
                    let side = match point.side() {
                        Side::Negative => 0.0,
                        Side::Positive => 1.0,
                    };

                    render::Vertex2D {
                        pos: point.position().to_array(),
                        tex_coord: [advancement, side],
                        color: color,
                    }
                }),
            )
            .unwrap();

        geometry
    }

    ///
    /// Strokes a path and returns the resulting render entity
    ///
    fn stroke(
        &mut self,
        path: path::Path,
        stroke_options: StrokeSettings,
        scale_factor: f64,
        transform: canvas::Transform2D,
        entity: LayerEntityRef,
    ) -> (LayerEntityRef, RenderEntity, RenderEntityDetails) {
        let geometry = self.stroke_geometry(path, stroke_options, scale_factor);
        let details = RenderEntityDetails::from_vertices(&geometry.vertices, &transform);

        (
            entity,
            RenderEntity::VertexBuffer(geometry, VertexBufferIntent::Draw),
            details,
        )
    }
}
