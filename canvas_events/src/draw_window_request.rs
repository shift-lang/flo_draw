/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use flo_canvas::scenery::*;
use flo_scene::*;

use super::draw_event_request::*;
use super::render_request::*;

///
/// The types of mouse pointer that can be displayed in a window
///
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MousePointer {
    /// No pointer
    None,

    /// The default pointer for the operating system
    SystemDefault,
}

///
/// Messages that can be sent to a flo_draw window that can generate events
///
#[derive(Debug)]
pub enum EventWindowRequest {
    /// Adds a channel that events generated for this window is relayed to
    SendEvents(BoxedEntityChannel<'static, DrawEventRequest>),

    /// Closes the window and shuts down the rendering entity
    CloseWindow,

    /// Sets the title of the window
    SetTitle(String),

    /// Sets whether or not the window should be full-screen
    SetFullScreen(bool),

    /// Sets whehter or not the window decorations are shown
    SetHasDecorations(bool),

    /// Sets the mouse pointer to display for the window
    SetMousePointer(MousePointer),
}

///
/// Messages that can be sent to a flo_draw window that processes 2D graphics instructions
///
#[derive(Debug)]
pub enum DrawingWindowRequest {
    /// Carry out a drawing request
    Draw(DrawingRequest),

    /// Adds a channel that events generated for this window is relayed to
    SendEvents(BoxedEntityChannel<'static, DrawEventRequest>),

    /// Closes the window and shuts down the rendering entity
    CloseWindow,

    /// Sets the title of the window
    SetTitle(String),

    /// Sets whether or not the window should be full-screen
    SetFullScreen(bool),

    /// Sets whehter or not the window decorations are shown
    SetHasDecorations(bool),

    /// Sets the mouse pointer to display for the window
    SetMousePointer(MousePointer),
}

///
/// Messages that can be sent to a flo_draw window that processes low-level 2D graphics instructions
///
pub enum RenderWindowRequest {
    /// Carry out a render request
    Render(RenderRequest),

    /// Adds a channel that events generated for this window is relayed to
    SendEvents(BoxedEntityChannel<'static, DrawEventRequest>),

    /// Closes the window and shuts down the rendering entity
    CloseWindow,

    /// Sets the title of the window
    SetTitle(String),

    /// Sets whether or not the window should be full-screen
    SetFullScreen(bool),

    /// Sets whehter or not the window decorations are shown
    SetHasDecorations(bool),

    /// Sets the mouse pointer to display for the window
    SetMousePointer(MousePointer),
}

impl From<RenderRequest> for RenderWindowRequest {
    fn from(req: RenderRequest) -> RenderWindowRequest {
        RenderWindowRequest::Render(req)
    }
}

impl From<DrawingRequest> for DrawingWindowRequest {
    fn from(req: DrawingRequest) -> DrawingWindowRequest {
        DrawingWindowRequest::Draw(req)
    }
}

impl From<EventWindowRequest> for RenderWindowRequest {
    fn from(req: EventWindowRequest) -> RenderWindowRequest {
        match req {
            EventWindowRequest::SendEvents(events) => RenderWindowRequest::SendEvents(events),
            EventWindowRequest::CloseWindow => RenderWindowRequest::CloseWindow,
            EventWindowRequest::SetTitle(title) => RenderWindowRequest::SetTitle(title),
            EventWindowRequest::SetFullScreen(fullscreen) => {
                RenderWindowRequest::SetFullScreen(fullscreen)
            }
            EventWindowRequest::SetHasDecorations(decorations) => {
                RenderWindowRequest::SetHasDecorations(decorations)
            }
            EventWindowRequest::SetMousePointer(mouse_pointer) => {
                RenderWindowRequest::SetMousePointer(mouse_pointer)
            }
        }
    }
}

impl From<EventWindowRequest> for DrawingWindowRequest {
    fn from(req: EventWindowRequest) -> DrawingWindowRequest {
        match req {
            EventWindowRequest::SendEvents(events) => DrawingWindowRequest::SendEvents(events),
            EventWindowRequest::CloseWindow => DrawingWindowRequest::CloseWindow,
            EventWindowRequest::SetTitle(title) => DrawingWindowRequest::SetTitle(title),
            EventWindowRequest::SetFullScreen(fullscreen) => {
                DrawingWindowRequest::SetFullScreen(fullscreen)
            }
            EventWindowRequest::SetHasDecorations(decorations) => {
                DrawingWindowRequest::SetHasDecorations(decorations)
            }
            EventWindowRequest::SetMousePointer(mouse_pointer) => {
                DrawingWindowRequest::SetMousePointer(mouse_pointer)
            }
        }
    }
}
