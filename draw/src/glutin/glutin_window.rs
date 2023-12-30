/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::ffi::CString;
use std::pin::*;

use flo_stream::*;
use futures::prelude::*;
use futures::task::{Context, Poll};
use gl;
use glutin::context::{
    NotCurrentContext, NotCurrentGlContextSurfaceAccessor, PossiblyCurrentGlContext,
};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::prelude::{GlConfig, GlSurface};
use glutin::surface::{Surface, SurfaceTypeTrait};
use glutin_winit::GlWindow;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::window::{Fullscreen, Theme, Window, WindowLevel};

use flo_binding::*;
use flo_render::*;

use crate::events::*;
use crate::window_properties::*;

///
/// Message indicating that the application has been suspended or resumed
///
#[derive(Clone, PartialEq, Debug)]
pub(crate) enum SuspendResume {
    Suspended,
    Resumed,
}

///
/// Manages the state of a Glutin window
///
pub struct GlutinWindow<TConfig>
where
    TConfig: GlConfig + GetGlDisplay,
{
    /// The context for this window
    context: Option<NotCurrentContext>,

    /// The configuration from when the context was create
    gl_config: TConfig,

    /// The surface for the window
    surface: Option<<TConfig::Target as GlDisplay>::WindowSurface>,

    /// The window the context is attached to
    window: Option<Window>,

    /// The renderer for this window (or none if there isn't one yet)
    renderer: Option<GlRenderer>,
}

impl<TConfig> GlutinWindow<TConfig>
where
    TConfig: GlConfig + GetGlDisplay,
{
    ///
    /// Creates a new glutin window
    ///
    pub fn new(
        context: NotCurrentContext,
        gl_config: TConfig,
        window: Window,
    ) -> GlutinWindow<TConfig> {
        GlutinWindow {
            context: Some(context),
            gl_config,
            surface: None,
            window: Some(window),
            renderer: None,
        }
    }
}

///
/// Sends render actions to a window
///0
pub(super) async fn send_actions_to_window<
    RenderStream,
    SuspendResumeStream,
    DrawEventPublisher,
    TConfig,
    TSurfaceType,
>(
    window: GlutinWindow<TConfig>,
    suspend_resume: SuspendResumeStream,
    render_actions: RenderStream,
    events: DrawEventPublisher,
    window_properties: WindowProperties,
) where
    RenderStream: Unpin + Stream<Item = Vec<RenderAction>>,
    SuspendResumeStream: Unpin + Stream<Item = SuspendResume>,
    DrawEventPublisher: MessagePublisher<Message = DrawEvent>,
    TConfig: GlConfig + GetGlDisplay,
    TConfig::Target: GlDisplay<WindowSurface = Surface<TSurfaceType>, Config = TConfig>,
    TSurfaceType: SurfaceTypeTrait,
{
    // Read events from the render actions list
    let mut window = window;
    let mut events = events;
    let mut window_actions = WindowUpdateStream {
        suspend_resume,
        render_stream: render_actions,
        size: follow(window_properties.size),
        min_size: follow(window_properties.min_size),
        max_size: follow(window_properties.max_size),
        title: follow(window_properties.title),
        is_transparent: follow(window_properties.is_transparent),
        is_visible: follow(window_properties.is_visible),
        is_resizable: follow(window_properties.is_resizable),
        is_minimized: follow(window_properties.is_minimized),
        is_maximized: follow(window_properties.is_maximized),
        fullscreen: follow(window_properties.fullscreen),
        has_decorations: follow(window_properties.has_decorations),
        window_level: follow(window_properties.window_level),
        ime_position: follow(window_properties.ime_position),
        ime_allowed: follow(window_properties.ime_allowed),
        theme: follow(window_properties.theme),
        cursor_position: follow(window_properties.cursor_position),
        cursor_icon: follow(window_properties.cursor_icon),
    };

    while let Some(next_action) = window_actions.next().await {
        match next_action {
            WindowUpdate::Resumed => {
                // Create surface
                let surface_attributes = window
                    .window
                    .as_ref()
                    .unwrap()
                    .build_surface_attributes(<_>::default());
                window.surface = unsafe {
                    Some(
                        window
                            .gl_config
                            .display()
                            .create_window_surface(&window.gl_config, &surface_attributes)
                            .unwrap(),
                    )
                };
            }

            WindowUpdate::Suspended => {
                // TODO: remove the surface
            }

            WindowUpdate::Render(next_action) => {
                // Do nothing if there are no actions waiting to be drawn
                if next_action.len() == 0 {
                    continue;
                }

                let show_frame_buffer =
                    if next_action[next_action.len() - 1] == RenderAction::ShowFrameBuffer {
                        // Typically this is the last instruction
                        true
                    } else {
                        // Search harder if it's not the last instruction
                        next_action
                            .iter()
                            .any(|item| item == &RenderAction::ShowFrameBuffer)
                    };

                // TODO: report errors if we can't set the context rather than just stopping mysteriously

                // Fetch the surface, if one has been created (won't be available if we haven't resumed)
                let current_surface = window.surface.take();
                let current_surface = if let Some(current_surface) = current_surface {
                    current_surface
                } else {
                    continue;
                };

                // Make the current context current
                let current_context = window.context.take().expect("Window context");

                let current_context = current_context.make_current(&current_surface);
                let current_context = if let Ok(context) = current_context {
                    context
                } else {
                    break;
                };

                let display = window.gl_config.display();

                // Get informtion about the current context
                let size = window.window.as_ref().unwrap().inner_size();
                let width = size.width as usize;
                let height = size.height as usize;

                // Create the renderer (needs the OpenGL functions to be loaded)
                if window.renderer.is_none() {
                    // Load the functions for the current context
                    // TODO: we're assuming they stay loaded to avoid loading them for every render, which might not be safe
                    // TODO: probably better to have the renderer load the functions itself (gl::load doesn't work well
                    // when we load GL twice, which could happen if we want to use the offscreen renderer)
                    gl::load_with(|symbol_name| {
                        let symbol_name = CString::new(symbol_name).unwrap();
                        display.get_proc_address(symbol_name.as_c_str())
                    });

                    // Create the renderer
                    window.renderer = Some(GlRenderer::new());
                }

                // Perform the rendering actions
                if let Some(renderer) = &mut window.renderer {
                    renderer.prepare_to_render_to_active_framebuffer(width, height);
                    renderer.render(next_action);
                }

                // Swap buffers to finish the drawing
                if show_frame_buffer {
                    current_surface.swap_buffers(&current_context).ok();
                }

                // Release the current context
                let context = current_context.make_not_current();
                let context = if let Ok(context) = context {
                    context
                } else {
                    break;
                };
                window.context = Some(context);
                window.surface = Some(current_surface);

                // Notify that a new frame has been drawn
                events.publish(DrawEvent::NewFrame).await;
            }

            WindowUpdate::SetSize((width, height)) => {
                window
                    .window
                    .as_ref()
                    .map(|win| win.set_inner_size(LogicalSize::new(width as f64, height as _)));
            }
            WindowUpdate::SetMinSize(Some((width, height))) => {
                window
                    .window
                    .as_ref()
                    .map(|win| win.set_min_inner_size(Some(LogicalSize::new(width as f64, height as _))));
            },
            WindowUpdate::SetMinSize(None) => {
                window
                    .window
                    .as_ref()
                    .map(|win| win.set_min_inner_size::<LogicalSize<f64>>(None));
            },
            WindowUpdate::SetMaxSize(Some((width, height))) => {
                window
                    .window
                    .as_ref()
                    .map(|win| win.set_max_inner_size(Some(LogicalSize::new(width as f64, height as _))));
            },
            WindowUpdate::SetMaxSize(None) => {
                window
                    .window
                    .as_ref()
                    .map(|win| win.set_max_inner_size::<LogicalSize<f64>>(None));
            },
            WindowUpdate::SetTitle(new_title) => {
                window
                    .window
                    .as_ref()
                    .map(|win| win.set_title(&new_title));
            }
            WindowUpdate::SetIsTransparent(val) => {
                window.window.as_ref().map(|win| win.set_transparent(val));
            },
            WindowUpdate::SetIsVisible(val) => {
                window.window.as_ref().map(|win| win.set_visible(val));
            },
            WindowUpdate::SetIsResizable(val) => {
                window.window.as_ref().map(|win| win.set_resizable(val));
            },
            WindowUpdate::SetMinimized(val) => {
                window.window.as_ref().map(|win| win.set_minimized(val));
            },
            WindowUpdate::SetMaximized(val) => {
                window.window.as_ref().map(|win| win.set_maximized(val));
            },
            WindowUpdate::SetFullscreen(is_fullscreen) => {
                let fullscreen = if is_fullscreen {
                    Some(Fullscreen::Borderless(None))
                } else {
                    None
                };
                window
                    .window
                    .as_ref()
                    .map(|win| win.set_fullscreen(fullscreen));
            }
            WindowUpdate::SetHasDecorations(decorations) => {
                window
                    .window
                    .as_ref()
                    .map(|win| win.set_decorations(decorations));
            }
            WindowUpdate::SetWindowLevel(level) => {
                window.window.as_ref().map(|win| win.set_window_level(level));
            },
            WindowUpdate::SetImePosition((x, y)) => {
                window.window.as_ref().map(|win| win.set_ime_position(PhysicalPosition::new(x as u32, y as _)));
            },
            WindowUpdate::SetImeAllowed(val) => {
                window.window.as_ref().map(|win| win.set_ime_allowed(val));
            },
            WindowUpdate::SetTheme(theme) => {
                window.window.as_ref().map(|win| win.set_theme(theme));
            },
            WindowUpdate::SetCursorPosition((x, y)) => {
                window.window.as_ref().map(|win| win.set_cursor_position(PhysicalPosition::new(x as u32, y as _)));
            },
            WindowUpdate::SetCursorIcon(MousePointer::None) => {
                window
                    .window
                    .as_ref()
                    .map(|win| win.set_cursor_visible(false));
            }

            WindowUpdate::SetCursorIcon(MousePointer::SystemDefault(mode)) => {
                window
                    .window
                    .as_ref()
                    .map(|win| { win.set_cursor_visible(true); win.set_cursor_icon(mode) });
            }
        }
    }

    // Window will close once the render actions are finished as we drop it here
}

///
/// The list of update events that can occur to a window
///
#[derive(Debug)]
enum WindowUpdate {
    Resumed,
    Suspended,
    Render(Vec<RenderAction>),
    SetSize((u64, u64)),
    SetMinSize(Option<(u64, u64)>),
    SetMaxSize(Option<(u64, u64)>),
    SetTitle(String),
    SetIsTransparent(bool),
    SetIsVisible(bool),
    SetIsResizable(bool),
    SetMinimized(bool),
    SetMaximized(bool),
    SetFullscreen(bool),
    SetHasDecorations(bool),
    SetWindowLevel(WindowLevel),
    SetImePosition((u64, u64)),
    SetImeAllowed(bool),
    SetTheme(Option<Theme>),
    SetCursorPosition((u64, u64)),
    SetCursorIcon(MousePointer),
}

///
/// Stream that merges the streams from the window properties and the renderer into a single stream
///
struct WindowUpdateStream<
    TSuspendResumeStream,
    TRenderStream,
    TSizeStream,
    TMinSizeStream,
    TMaxSizeStream,
    TTitleStream,
    TIsTransparentStream,
    TIsVisibleStream,
    TIsResizableStream,
    TMinimizedStream,
    TMaximizedStream,
    TFullscreenStream,
    THasDecorationsStream,
    TWindowLevelStream,
    TImePositionStream,
    TImeAllowedStream,
    TThemeStream,
    TCursorPositionStream,
    TCursorIconStream,
> {
    suspend_resume: TSuspendResumeStream,
    render_stream: TRenderStream,
    size: TSizeStream,
    min_size: TMinSizeStream,
    max_size: TMaxSizeStream,
    title: TTitleStream,
    is_transparent: TIsTransparentStream,
    is_visible: TIsVisibleStream,
    is_resizable: TIsResizableStream,
    is_minimized: TMinimizedStream,
    is_maximized: TMaximizedStream,
    fullscreen: TFullscreenStream,
    has_decorations: THasDecorationsStream,
    window_level: TWindowLevelStream,
    ime_position: TImePositionStream,
    ime_allowed: TImeAllowedStream,
    theme: TThemeStream,
    cursor_position: TCursorPositionStream,
    cursor_icon: TCursorIconStream,
}

impl<
        TSuspendResumeStream,
        TRenderStream,
        TSizeStream,
        TMinSizeStream,
        TMaxSizeStream,
        TTitleStream,
        TIsTransparentStream,
        TIsVisibleStream,
        TIsResizableStream,
        TMinimizedStream,
        TMaximizedStream,
        TFullscreenStream,
        THasDecorationsStream,
        TWindowLevelStream,
        TImePositionStream,
        TImeAllowedStream,
        TThemeStream,
        TCursorPositionStream,
        TCursorIconStream,
    > Stream
    for WindowUpdateStream<
        TSuspendResumeStream,
        TRenderStream,
        TSizeStream,
        TMinSizeStream,
        TMaxSizeStream,
        TTitleStream,
        TIsTransparentStream,
        TIsVisibleStream,
        TIsResizableStream,
        TMinimizedStream,
        TMaximizedStream,
        TFullscreenStream,
        THasDecorationsStream,
        TWindowLevelStream,
        TImePositionStream,
        TImeAllowedStream,
        TThemeStream,
        TCursorPositionStream,
        TCursorIconStream,
    >
where
    TSuspendResumeStream: Unpin + Stream<Item = SuspendResume>,
    TRenderStream: Unpin + Stream<Item = Vec<RenderAction>>,
    TSizeStream: Unpin + Stream<Item = (u64, u64)>,
    TMinSizeStream: Unpin + Stream<Item = Option<(u64, u64)>>,
    TMaxSizeStream: Unpin + Stream<Item = Option<(u64, u64)>>,
    TTitleStream: Unpin + Stream<Item = String>,
    TIsTransparentStream: Unpin + Stream<Item = bool>,
    TIsVisibleStream: Unpin + Stream<Item = bool>,
    TIsResizableStream: Unpin + Stream<Item = bool>,
    TMinimizedStream: Unpin + Stream<Item = bool>,
    TMaximizedStream: Unpin + Stream<Item = bool>,
    TFullscreenStream: Unpin + Stream<Item = bool>,
    THasDecorationsStream: Unpin + Stream<Item = bool>,
    TWindowLevelStream: Unpin + Stream<Item = WindowLevel>,
    TImePositionStream: Unpin + Stream<Item = (u64, u64)>,
    TImeAllowedStream: Unpin + Stream<Item = bool>,
    TThemeStream: Unpin + Stream<Item = Option<Theme>>,
    TCursorPositionStream: Unpin + Stream<Item = (u64, u64)>,
    TCursorIconStream: Unpin + Stream<Item = MousePointer>,
{
    type Item = WindowUpdate;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        macro_rules! is_ready {
            ($name:ident, $event:ident) => {
                match self.$name.poll_next_unpin(context) {
                    Poll::Ready(Some(item)) => {
                        return Poll::Ready(Some(WindowUpdate::$event(item)));
                    }
                    Poll::Ready(None) => {
                        return Poll::Ready(None);
                    }
                    Poll::Pending => {}
                }
            };
        }

        // Poll each stream in turn to see if they have an item

        // Suspending and resuming has priority
        match self.suspend_resume.poll_next_unpin(context) {
            Poll::Ready(Some(SuspendResume::Suspended)) => {
                return Poll::Ready(Some(WindowUpdate::Suspended));
            }
            Poll::Ready(Some(SuspendResume::Resumed)) => {
                return Poll::Ready(Some(WindowUpdate::Resumed));
            }
            Poll::Ready(None) => {
                return Poll::Ready(None);
            }
            Poll::Pending => {}
        }

        // Followed by render instructions
        is_ready!(render_stream, Render);

        // The various binding streams
        is_ready!(size, SetSize);
        is_ready!(min_size, SetMinSize);
        is_ready!(max_size, SetMaxSize);
        is_ready!(title, SetTitle);
        is_ready!(is_transparent, SetIsTransparent);
        is_ready!(is_visible, SetIsVisible);
        is_ready!(is_resizable, SetIsResizable);
        is_ready!(is_minimized, SetMinimized);
        is_ready!(is_maximized, SetMaximized);
        is_ready!(fullscreen, SetFullscreen);
        is_ready!(has_decorations, SetHasDecorations);
        is_ready!(window_level, SetWindowLevel);
        is_ready!(ime_position, SetImePosition);
        is_ready!(ime_allowed, SetImeAllowed);
        is_ready!(theme, SetTheme);
        is_ready!(cursor_position, SetCursorPosition);
        is_ready!(cursor_icon, SetCursorIcon);


        // No stream matched anything
        Poll::Pending
    }
}
