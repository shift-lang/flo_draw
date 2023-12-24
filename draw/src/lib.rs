//!
//! `flo_draw` provides a simple API for rendering 2D graphics into a window
//!
//! It's part of a set of companion crates that provide a very flexible 2D graphics rendering system.
//!
//! * `flo_canvas` provides a way to describe 2D rendering operations without requiring any particular implementation
//! * `flo_render` describes how to render to modern graphics hardware via the OpenGL and Metal APIs
//! * `flo_render_canvas` translates between instructions for `flo_canvas` and `flo_render`, using `lyon` for tessellation.
//!    It also provides facilities for offscreen rendering
//! * `flo_draw` is this crate, and it provides an easy way to render 2D vector graphics to screen using `glutin` and OpenGL.
//!
//! # Why use these crates?
//!
//! The main reason to use `flo_draw` or the offscreen renderer in `flo_render_canvas` is that they provide a very straightforward API: the
//! setup needed to start drawing graphics to a window or a byte buffer is almost nonexistent. In spite of this they are also very flexible,
//! capable of being used to create fully interactive applications which can run on any system supported by glutin and OpenGL 3.3.
//!
//! The rendering system is very flexible and easily ported to a different target, so if you outgrow the glutin-based windowing system and
//! want to integrate your algorithms into another application, the architecture supplied by `flo_canvas` and `flo_render` makes it easy to
//! intercept the underlying rendering operations and integrate them into any other system. Additional renderers are already available in
//! FlowBetween to render `flo_canvas` instructions to HTML canvases, OS X Quartz render contexts and to Cairo. `flo_render` has native support
//! for both OpenGL 3.3 and Metal.
//!
//! The 2D graphics model used here has a few interesting features that are not present in many other rendering libraries. In particular,
//! there is a layer system which is very useful for simplifying the design of interactive graphics applications by reducing the amount of
//! work involved in a redraw, and it's possible to both draw and erase shapes. With the hardware renderers in `flo_render`, the number of
//! layers is effectively unlimited. There's also a 'sprite' system, which makes it possible to easily re-render complicated shapes.
//!
//! # Getting started
//!
//! Start your application by calling `with_2d_graphics(|| {})` with a function to perform whatever drawing operations you want.
//! In that function, `let canvas = create_drawing_window("Canvas window");` will create a window with a 2D graphics canvas that
//! you can draw on using `canvas.draw(|gc| { });`. Finally, `create_drawing_window_with_events()` is a way to create a graphics
//! window that supplies events allowing interactivity.
//!
//! The documentation for [flo_canvas](canvas) shows what can be done in a drawing routine.
//!
//! See the `canvas_window.rs` example for the basic setup for an application that renders 2D graphics and `follow_mouse.rs` for
//! a basic example with event handling.
//!
//!
//! # Feature flags
//!
//! There are a couple of feature flags that can be used to choose rendering engines or enable or disable features:
//!
//! * `render-opengl` - the default renderer, using the OpenGL API. Right now this provides the highest performance on the
//!   widest range of possible hardware.
//! * `render-wgpu` - uses WGPU instead of OpenGL. WGPU can use a wide range of backends but is currently quite low performing
//!   on several systems
//! * `profile` - output some performance metrics to the console on each frame
//! * `wgpu-profiler` - add in the WGPU profiler (somewhat unreliable)
//!

pub use flo_binding as binding;
pub use flo_scene as scene;

pub use flo_canvas as canvas;
pub use flo_canvas_events as events;
pub use flo_render::initialize_offscreen_rendering;
pub use flo_render_canvas as render_canvas;
pub use flo_render_canvas::render_canvas_offscreen;

pub use self::drawing_window::*;
pub use self::events::*;
#[cfg(all(feature = "render-opengl", not(feature = "render-wgpu")))]
pub use self::glutin::with_2d_graphics;
pub use self::render_window::*;
#[cfg(all(feature = "render-wgpu"))]
pub use self::wgpu::with_2d_graphics;
pub use self::window_properties::*;

mod drawing_window;
mod render_window;
mod window_properties;

/// The 'glutin' module provides an OpenGL implementation of the canvas using glutin for window management
#[cfg(feature = "render-opengl")]
pub mod glutin;

/// The 'wgpu' module provides a winit-based wgpu implementation of a renderer
#[cfg(feature = "render-wgpu")]
pub mod wgpu;

/// The 'Scene' API provides a framework for building more complex software out of message-passing components
pub mod draw_scene;

