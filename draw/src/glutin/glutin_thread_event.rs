/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use flo_stream::*;
use futures::future::LocalBoxFuture;
use futures::stream::BoxStream;
use winit::window::WindowId;

use flo_canvas_events::DrawEvent;
use flo_render::*;

use crate::window_properties::*;

///
/// Event that can be sent to a glutin thread
///
pub enum GlutinThreadEvent {
    /// Creates a window that will render the specified actions
    CreateRenderWindow(
        BoxStream<'static, Vec<RenderAction>>,
        Publisher<DrawEvent>,
        WindowProperties,
    ),

    /// Runs a future on the Glutin thread
    RunProcess(Box<dyn Send + FnOnce() -> LocalBoxFuture<'static, ()>>),

    /// Polls the future with the specified ID
    WakeFuture(u64),

    /// Stop sending events for the specified window
    StopSendingToWindow(WindowId),

    /// Tells the UI thread to stop when there are no more windows open
    StopWhenAllWindowsClosed,
}
