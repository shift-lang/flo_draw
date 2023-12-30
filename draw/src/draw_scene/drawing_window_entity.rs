/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::pin::*;
use std::sync::*;

use flo_stream::*;
use futures::prelude::*;
use futures::task::{Context, Poll};

use flo_canvas::scenery::*;
use flo_canvas::*;
use flo_canvas_events::*;
use flo_render_canvas::*;
use flo_scene::*;

///
/// Combines rendering and event messages into one enum
///
enum DrawingOrEvent {
    Drawing(Vec<DrawingWindowRequest>),
    Event(Vec<DrawEventRequest>),
}

///
/// Stream that reads instructions from the drawing or event stream
///
/// Drawing stream may be suspended while we wait for new frames, and the event stream has priority at all other times
///
struct DrawingEventStream<TDrawStream, TEventStream>
where
    TDrawStream: Unpin + Stream<Item = DrawingOrEvent>,
    TEventStream: Unpin + Stream<Item = DrawingOrEvent>,
{
    // If set to true, the stream will not attempt to poll the drawing stream
    waiting_for_new_frame: bool,

    /// The drawing stream, or None if it has been closed
    draw_stream: Option<TDrawStream>,

    /// The event stream, or None if it has been closed
    event_stream: Option<TEventStream>,
}

///
/// Structure used to store the current state of the canvas renderer
///
struct RendererState {
    /// The renderer for the canvas
    renderer: CanvasRenderer,

    /// The transformation from window coordinates to canvas coordinates
    window_transform: Option<Transform2D>,

    /// The scale factor of the canvas
    scale: f64,

    /// The width of the canvas
    width: f64,

    /// The height of the canvas
    height: f64,
}

impl<TDrawStream, TEventStream> Stream for DrawingEventStream<TDrawStream, TEventStream>
where
    TDrawStream: Unpin + Stream<Item = DrawingOrEvent>,
    TEventStream: Unpin + Stream<Item = DrawingOrEvent>,
{
    type Item = DrawingOrEvent;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // First, see if the event stream has anything for us, and return the event from there if it exists
        if let Some(event_stream) = &mut self.event_stream {
            let event_poll_result = event_stream.poll_next_unpin(context);

            match event_poll_result {
                Poll::Ready(Some(event)) => {
                    return Poll::Ready(Some(event));
                }
                Poll::Ready(None) => {
                    self.event_stream = None;
                }
                Poll::Pending => {}
            }
        }

        // Check the draw stream if we're not waiting for a frame
        if !self.waiting_for_new_frame {
            if let Some(draw_stream) = &mut self.draw_stream {
                let draw_poll_result = draw_stream.poll_next_unpin(context);

                match draw_poll_result {
                    Poll::Ready(Some(event)) => {
                        return Poll::Ready(Some(event));
                    }
                    Poll::Ready(None) => {
                        self.draw_stream = None;
                    }
                    Poll::Pending => {}
                }
            }
        }

        // If both streams are done, indicate that we're finished
        if self.draw_stream.is_none() && self.event_stream.is_none() {
            return Poll::Ready(None);
        }

        // Waiting on one or both of the streams
        Poll::Pending
    }
}

///
/// Handles an event from the window
///
/// The return value is any extra events to synthesize as a result of the initial event
///
fn handle_window_event<'a, SendFuture, SendRenderActionsFn>(
    state: &'a mut RendererState,
    event: DrawEvent,
    send_render_actions: &'a mut SendRenderActionsFn,
) -> impl 'a + Send + Future<Output = Vec<DrawEvent>>
where
    SendRenderActionsFn: Send + FnMut(Vec<RenderAction>) -> SendFuture,
    SendFuture: Send + Future<Output = ()>,
{
    async move {
        match event {
            DrawEvent::Redraw => {
                // Drawing nothing will regenerate the current contents of the renderer
                let redraw = state
                    .renderer
                    .draw(vec![].into_iter())
                    .collect::<Vec<_>>()
                    .await;
                send_render_actions(redraw).await;

                let window_transform = state.update_window_transform();
                vec![DrawEvent::CanvasTransform(window_transform)]
            }

            DrawEvent::ScaleFactorChanged { scale_factor, .. } => {
                state.scale = scale_factor;

                let width = state.width as f32;
                let height = state.height as f32;
                let scale = state.scale as f32;

                state
                    .renderer
                    .set_viewport(0.0..width, 0.0..height, width, height, scale);

                vec![]
            }

            DrawEvent::Resized(size) => {
                state.width = size.width as _;
                state.height = size.height as _;

                let width = state.width as f32;
                let height = state.height as f32;
                let scale = state.scale as f32;

                state
                    .renderer
                    .set_viewport(0.0..width, 0.0..height, width, height, scale);

                vec![]
            }

            _ => {
                vec![]
            }
        }
    }
}

impl RendererState {
    ///
    /// Updates the window transform for this state
    ///
    fn update_window_transform(&mut self) -> Transform2D {
        // Fetch the window tranform from the canvas, and invert it to get the transform from window coordinates to canvas coordinates
        let window_transform = self.renderer.get_window_transform().invert().unwrap();

        // Window coordinates are inverted compared to canvas coordinates
        let window_transform = Transform2D::scale(1.0, -1.0) * window_transform;
        let window_transform = window_transform * Transform2D::translate(0.0, -self.height as _);

        // Update the value of the transform in the state
        self.window_transform = Some(window_transform);
        window_transform
    }

    ///
    /// Performs a drawing action and passes it on to the render target
    ///
    async fn draw(
        &mut self,
        draw_actions: impl Send + Iterator<Item = &Draw>,
        render_target: &mut (impl 'static + EntityChannel<Message = RenderWindowRequest>),
    ) {
        let render_actions = self
            .renderer
            .draw(draw_actions.cloned())
            .collect::<Vec<_>>()
            .await;
        render_target
            .send(RenderWindowRequest::Render(RenderRequest::Render(
                render_actions,
            )))
            .await
            .ok();
    }
}

///
/// Creates a drawing window that sends render requests to the specified target
///
pub fn create_drawing_window_entity(
    context: &Arc<SceneContext>,
    entity_id: EntityId,
    render_target: impl 'static + EntityChannel<Message = RenderWindowRequest>,
) -> Result<SimpleEntityChannel<DrawingWindowRequest>, CreateEntityError> {
    // This window can accept a couple of converted messages
    context.convert_message::<DrawingRequest, DrawingWindowRequest>()?;
    context.convert_message::<EventWindowRequest, DrawingWindowRequest>()?;

    // Create the window in context
    context.create_entity(
        entity_id,
        move |context, drawing_window_requests| async move {
            let mut render_target = render_target;

            // We relay events via our own event publisher
            let mut event_publisher = Publisher::new(1000);

            // Set up the renderer and window state
            let mut render_state = RendererState {
                renderer: CanvasRenderer::new(),
                window_transform: None,
                scale: 1.0,
                width: 1.0,
                height: 1.0,
            };

            // Request the events from the render target
            let (channel, events_receiver) = SimpleEntityChannel::new(entity_id, 1000);
            render_target
                .send(RenderWindowRequest::SendEvents(channel.boxed()))
                .await
                .ok();

            // Chunk the requests we receive
            let drawing_window_requests = drawing_window_requests.ready_chunks(100);
            let events_receiver = events_receiver.ready_chunks(100);

            // Combine the two streams (we prioritise events from the window to avoid spending time rendering with out-of-date state)
            let drawing_window_requests =
                drawing_window_requests.map(|evt| DrawingOrEvent::Drawing(evt));
            let events_receiver = events_receiver.map(|evt| DrawingOrEvent::Event(evt));
            let messages = DrawingEventStream {
                waiting_for_new_frame: false,
                draw_stream: Some(drawing_window_requests),
                event_stream: Some(events_receiver),
            };

            // Initially the window is not ready to render (we need to wait for the first 'redraw' event)
            let mut ready_to_render = false;
            let mut waiting_for_new_frame = false;
            let mut drawing_since_last_frame = false;
            let mut closed = false;

            // Pause the drawing using a start frame event
            render_state
                .draw(vec![Draw::StartFrame].iter(), &mut render_target)
                .await;

            // Run the main event loop
            let mut messages = messages;
            while let Some(message) = messages.next().await {
                match message {
                    DrawingOrEvent::Drawing(drawing_list) => {
                        // Perform all the actions in a single frame
                        let mut combined_list = vec![Arc::new(vec![Draw::StartFrame])];

                        // If we've rendered something and 'NewFrame' hasn't yet been generated, add an extra 'StartFrame' to suspend rendering until the last frame is finished
                        if waiting_for_new_frame && !drawing_since_last_frame {
                            drawing_since_last_frame = true;
                            combined_list.push(Arc::new(vec![Draw::StartFrame]));
                        }

                        for draw_msg in drawing_list {
                            match draw_msg {
                                DrawingWindowRequest::Draw(DrawingRequest::Draw(drawing)) => {
                                    // Send the drawing to the renderer
                                    combined_list.push(drawing);
                                }

                                DrawingWindowRequest::CloseWindow => {
                                    // Just stop running when there's a 'close' request
                                    closed = true;
                                }

                                DrawingWindowRequest::SendEvents(event_channel) => {
                                    let mut subscriber = event_publisher.subscribe();

                                    context
                                        .run_in_background(async move {
                                            let mut channel_target = event_channel;

                                            // Pass on events to everything that's listening, until the channel starts generating errors
                                            while let Some(event) = subscriber.next().await {
                                                let result = channel_target.send(event).await;

                                                if result.is_err() {
                                                    break;
                                                }
                                            }
                                        })
                                        .ok();
                                }

                                DrawingWindowRequest::SetMinSize(value) => {
                                    render_target.send(RenderWindowRequest::SetMinSize(value)).await.ok();
                                }
                                DrawingWindowRequest::SetMaxSize(value) => {
                                    render_target.send(RenderWindowRequest::SetMaxSize(value)).await.ok();
                                }
                                DrawingWindowRequest::SetTitle(value) => {
                                    render_target.send(RenderWindowRequest::SetTitle(value)).await.ok();
                                }
                                DrawingWindowRequest::SetIsTransparent(value) => {
                                    render_target.send(RenderWindowRequest::SetIsTransparent(value)).await.ok();
                                }
                                DrawingWindowRequest::SetIsVisible(value) => {
                                    render_target.send(RenderWindowRequest::SetIsVisible(value)).await.ok();
                                }
                                DrawingWindowRequest::SetIsResizable(value) => {
                                    render_target.send(RenderWindowRequest::SetIsResizable(value)).await.ok();
                                }
                                DrawingWindowRequest::SetMinimized(value) => {
                                    render_target.send(RenderWindowRequest::SetMinimized(value)).await.ok();
                                }
                                DrawingWindowRequest::SetMaximized(value) => {
                                    render_target.send(RenderWindowRequest::SetMaximized(value)).await.ok();
                                }
                                DrawingWindowRequest::SetFullscreen(value) => {
                                    render_target.send(RenderWindowRequest::SetFullscreen(value)).await.ok();
                                }
                                DrawingWindowRequest::SetHasDecorations(value) => {
                                    render_target.send(RenderWindowRequest::SetHasDecorations(value)).await.ok();
                                }
                                DrawingWindowRequest::SetWindowLevel(value) => {
                                    render_target.send(RenderWindowRequest::SetWindowLevel(value)).await.ok();
                                }
                                DrawingWindowRequest::SetImePosition(value) => {
                                    render_target.send(RenderWindowRequest::SetImePosition(value)).await.ok();
                                }
                                DrawingWindowRequest::SetImeAllowed(value) => {
                                    render_target.send(RenderWindowRequest::SetImeAllowed(value)).await.ok();
                                }
                                DrawingWindowRequest::SetTheme(value) => {
                                    render_target.send(RenderWindowRequest::SetTheme(value)).await.ok();
                                }
                                DrawingWindowRequest::SetCursorPosition(value) => {
                                    render_target.send(RenderWindowRequest::SetCursorPosition(value)).await.ok();
                                }
                                DrawingWindowRequest::SetCursorIcon(value) => {
                                    render_target.send(RenderWindowRequest::SetCursorIcon(value)).await.ok();
                                }
                            }
                        }

                        // Commit the frame
                        waiting_for_new_frame = true;
                        messages.waiting_for_new_frame = true;

                        combined_list.push(Arc::new(vec![Draw::ShowFrame]));
                        render_state
                            .draw(
                                combined_list.iter().flat_map(|item| item.iter()),
                                &mut render_target,
                            )
                            .await;

                        // Update the window transform according to the drawing actions we processed
                        render_state.update_window_transform();
                    }

                    DrawingOrEvent::Event(event_list) => {
                        for evt_message in event_list.into_iter() {
                            let mut evt_message = evt_message;

                            match &evt_message {
                                // TODO: StartFrame/ShowFrame based on the 'NewFrame' event
                                DrawEvent::CursorMoved { state } => {
                                    // Rewrite pointer events before republishing them
                                    let mut state = state.clone();

                                    if let Some(window_transform) = &render_state.window_transform {
                                        let (x, y) = state.location_in_window;
                                        let (x, y) = (x as _, y as _);
                                        let (cx, cy) = window_transform.transform_point(x, y);
                                        state.location_in_canvas = Some((cx as _, cy as _));
                                    }

                                    evt_message = DrawEvent::CursorMoved { state };
                                }

                                DrawEvent::Redraw => {
                                    // When a redraw event arrives, we're ready to render from the renderer to the window
                                    if !ready_to_render {
                                        // Move to the 'ready to render' state
                                        ready_to_render = true;

                                        // Show the frame from the initial 'StartFrame' request
                                        render_state
                                            .draw(vec![Draw::ShowFrame].iter(), &mut render_target)
                                            .await;
                                    }
                                }

                                DrawEvent::CloseRequested => {
                                    // Close events terminate the loop (after we've finshed processing the events)
                                    closed = true;
                                }

                                DrawEvent::NewFrame => {
                                    // A new frame was displayed
                                    waiting_for_new_frame = false;

                                    if drawing_since_last_frame {
                                        // Finalize any drawing that occurred while we were waiting for the new frame to display
                                        waiting_for_new_frame = true;
                                        render_state
                                            .draw(vec![Draw::ShowFrame].iter(), &mut render_target)
                                            .await;
                                        drawing_since_last_frame = false;
                                    }

                                    messages.waiting_for_new_frame = waiting_for_new_frame;
                                }

                                _ => {}
                            }

                            // Publish the event to any subscribers
                            event_publisher.publish(evt_message.clone()).await;

                            // Handle the next message
                            handle_window_event(
                                &mut render_state,
                                evt_message,
                                &mut |render_actions| {
                                    let send_rendering =
                                        render_target.send(RenderWindowRequest::Render(
                                            RenderRequest::Render(render_actions),
                                        ));
                                    async move {
                                        send_rendering.await.ok();
                                    }
                                },
                            )
                            .await;
                        }

                        // The entity stops when the window is closed
                        if closed {
                            break;
                        }
                    }
                }
            }

            // Shut down
            render_target
                .send(RenderWindowRequest::CloseWindow)
                .await
                .ok();

            use std::mem;

            let when_closed = event_publisher.when_closed();

            // Drop the receivers
            mem::drop(messages);
            mem::drop(event_publisher);

            // Wait for the publisher to finish up
            when_closed.await;
        },
    )
}
