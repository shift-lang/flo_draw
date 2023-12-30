/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use flo_canvas::scenery::*;
use flo_scene::*;
use winit::window::{CursorIcon, Theme, WindowLevel};

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
    SystemDefault(CursorIcon),
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
/// Messages that can be sent to a flo_draw window that processes low-level 2D graphics instructions
///
pub enum RenderWindowRequest {
    /// Carry out a render request
    Render(RenderRequest),

    /// Adds a channel that events generated for this window is relayed to
    SendEvents(BoxedEntityChannel<'static, DrawEventRequest>),

    /// Closes the window and shuts down the rendering entity
    CloseWindow,

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
            EventWindowRequest::SetMinSize(value) => RenderWindowRequest::SetMinSize(value),
            EventWindowRequest::SetMaxSize(value) => RenderWindowRequest::SetMaxSize(value),
            EventWindowRequest::SetTitle(value) => RenderWindowRequest::SetTitle(value),
            EventWindowRequest::SetIsTransparent(value) => RenderWindowRequest::SetIsTransparent(value),
            EventWindowRequest::SetIsVisible(value) => RenderWindowRequest::SetIsVisible(value),
            EventWindowRequest::SetIsResizable(value) => RenderWindowRequest::SetIsResizable(value),
            EventWindowRequest::SetMinimized(value) => RenderWindowRequest::SetMinimized(value),
            EventWindowRequest::SetMaximized(value) => RenderWindowRequest::SetMaximized(value),
            EventWindowRequest::SetFullscreen(value) => RenderWindowRequest::SetFullscreen(value),
            EventWindowRequest::SetHasDecorations(value) => RenderWindowRequest::SetHasDecorations(value),
            EventWindowRequest::SetWindowLevel(value) => RenderWindowRequest::SetWindowLevel(value),
            EventWindowRequest::SetImePosition(value) => RenderWindowRequest::SetImePosition(value),
            EventWindowRequest::SetImeAllowed(value) => RenderWindowRequest::SetImeAllowed(value),
            EventWindowRequest::SetTheme(value) => RenderWindowRequest::SetTheme(value),
            EventWindowRequest::SetCursorPosition(value) => RenderWindowRequest::SetCursorPosition(value),
            EventWindowRequest::SetCursorIcon(value) => RenderWindowRequest::SetCursorIcon(value),
        }
    }
}

impl From<EventWindowRequest> for DrawingWindowRequest {
    fn from(req: EventWindowRequest) -> DrawingWindowRequest {
        match req {
            EventWindowRequest::SendEvents(events) => DrawingWindowRequest::SendEvents(events),
            EventWindowRequest::CloseWindow => DrawingWindowRequest::CloseWindow,
            EventWindowRequest::SetMinSize(value) => DrawingWindowRequest::SetMinSize(value),
            EventWindowRequest::SetMaxSize(value) => DrawingWindowRequest::SetMaxSize(value),
            EventWindowRequest::SetTitle(value) => DrawingWindowRequest::SetTitle(value),
            EventWindowRequest::SetIsTransparent(value) => DrawingWindowRequest::SetIsTransparent(value),
            EventWindowRequest::SetIsVisible(value) => DrawingWindowRequest::SetIsVisible(value),
            EventWindowRequest::SetIsResizable(value) => DrawingWindowRequest::SetIsResizable(value),
            EventWindowRequest::SetMinimized(value) => DrawingWindowRequest::SetMinimized(value),
            EventWindowRequest::SetMaximized(value) => DrawingWindowRequest::SetMaximized(value),
            EventWindowRequest::SetFullscreen(value) => DrawingWindowRequest::SetFullscreen(value),
            EventWindowRequest::SetHasDecorations(value) => DrawingWindowRequest::SetHasDecorations(value),
            EventWindowRequest::SetWindowLevel(value) => DrawingWindowRequest::SetWindowLevel(value),
            EventWindowRequest::SetImePosition(value) => DrawingWindowRequest::SetImePosition(value),
            EventWindowRequest::SetImeAllowed(value) => DrawingWindowRequest::SetImeAllowed(value),
            EventWindowRequest::SetTheme(value) => DrawingWindowRequest::SetTheme(value),
            EventWindowRequest::SetCursorPosition(value) => DrawingWindowRequest::SetCursorPosition(value),
            EventWindowRequest::SetCursorIcon(value) => DrawingWindowRequest::SetCursorIcon(value),
        }
    }
}
