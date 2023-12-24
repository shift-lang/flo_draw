use std::fmt;
use std::fmt::*;

use flo_stream::*;
use futures::channel::oneshot;
use futures::future::LocalBoxFuture;
use futures::stream::BoxStream;
use wgpu;
use winit::window::WindowId;

use flo_render::*;

use crate::events::*;
use crate::window_properties::*;

///
/// Event that can be sent to a winit thread
///
pub enum WinitThreadEvent {
    /// Creates a window that will render the specified actions
    CreateRenderWindow(BoxStream<'static, Vec<RenderAction>>, Publisher<DrawEvent>, WindowProperties),

    /// Runs a future on the winit thread
    RunProcess(Box<dyn Send + FnOnce() -> LocalBoxFuture<'static, ()>>),

    /// Polls the future with the specified ID
    WakeFuture(u64),

    /// Presents a surface to the specified window and signals the sender when done (cancelling any previous request for that window)
    PresentSurface(WindowId, wgpu::SurfaceTexture, oneshot::Sender<()>),

    /// Resolves a yield request by sending an empty message (used to yield to process events)
    Yield(oneshot::Sender<()>),

    /// Stop sending events for the specified window
    StopSendingToWindow(WindowId),

    /// Tells the UI thread to stop when there are no more windows open
    StopWhenAllWindowsClosed,
}

impl Debug for WinitThreadEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use self::WinitThreadEvent::*;

        match self {
            CreateRenderWindow(_, _, _) => write!(f, "CreateRenderWindow(...)"),
            RunProcess(_) => write!(f, "RunProcess(...)"),
            WakeFuture(id) => write!(f, "WakeFuture({})", id),
            PresentSurface(id, _, _) => write!(f, "PresentSurface({:?}, ...)", id),
            Yield(_) => write!(f, "Yield(...)"),
            StopSendingToWindow(id) => write!(f, "StopSendingToWindow({:?})", id),
            StopWhenAllWindowsClosed => write!(f, "StopWhenAllWindowsClosed"),
        }
    }
}