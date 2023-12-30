/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::sync::*;

use flo_stream::*;
use futures::channel::mpsc;
use futures::prelude::*;
use winit::window::{CursorIcon, WindowLevel};

use flo_binding::*;
use flo_canvas_events::*;
use flo_scene::*;

use crate::glutin::*;
use crate::window_properties::*;

///
/// Creates a render window in a scene with the specified entity ID
///
pub fn create_glutin_render_window_entity(
    context: &Arc<SceneContext>,
    entity_id: EntityId,
    initial_size: (u64, u64),
) -> Result<SimpleEntityChannel<RenderWindowRequest>, CreateEntityError> {
    // This window can accept a couple of converted messages
    context.convert_message::<RenderRequest, RenderWindowRequest>()?;
    context.convert_message::<EventWindowRequest, RenderWindowRequest>()?;

    // Create the window in context
    context.create_entity(
        entity_id,
        move |context, render_window_requests| async move {
            // Create the publisher to send the render actions to the stream
            let size = bind(initial_size);
            let title = bind("flo_draw".to_string());
            let fullscreen = bind(false);
            let has_decorations = bind(true);
            let cursor_icon = bind(MousePointer::SystemDefault(CursorIcon::Default));
            let min_size = bind(None);
            let max_size = bind(None);
            let is_transparent = bind(false);
            let is_visible = bind(true);
            let is_resizable = bind(true);
            let is_minimized = bind(false);
            let is_maximized = bind(false);
            let window_level = bind(WindowLevel::Normal);
            let ime_position = bind((0, 0));
            let ime_allowed = bind(false);
            let theme = bind(None);
            let cursor_position = bind((0, 0));

            let window_properties = WindowProperties {
                title: BindRef::from(title.clone()),
                fullscreen: BindRef::from(fullscreen.clone()),
                has_decorations: BindRef::from(has_decorations.clone()),
                cursor_icon: BindRef::from(cursor_icon.clone()),
                size: BindRef::from(size.clone()),
                min_size: BindRef::from(min_size.clone()),
                max_size: BindRef::from(max_size.clone()),
                is_transparent: BindRef::from(is_transparent.clone()),
                is_visible: BindRef::from(is_visible.clone()),
                is_resizable: BindRef::from(is_resizable.clone()),
                is_minimized: BindRef::from(is_minimized.clone()),
                is_maximized: BindRef::from(is_maximized.clone()),
                window_level: BindRef::from(window_level.clone()),
                ime_position: BindRef::from(ime_position.clone()),
                ime_allowed: BindRef::from(ime_allowed.clone()),
                theme: BindRef::from(theme.clone()),
                cursor_position: BindRef::from(cursor_position.clone()),
            };
            let mut event_publisher = Publisher::new(1000);

            // Create a stream for publishing render requests
            let (render_sender, render_receiver) = mpsc::channel(5);

            // Create a window that subscribes to the publisher
            let glutin_thread = glutin_thread();
            glutin_thread.send_event(GlutinThreadEvent::CreateRenderWindow(
                render_receiver.boxed(),
                event_publisher.republish(),
                window_properties.into(),
            ));

            // Run the main event loop
            let mut render_window_requests = render_window_requests;
            let mut render_sender = render_sender;

            while let Some(request) = render_window_requests.next().await {
                let request: RenderWindowRequest = request;

                match request {
                    RenderWindowRequest::Render(RenderRequest::Render(render)) => {
                        // Just pass render requests on to the render window
                        if render_sender.send(render).await.is_err() {
                            // This entity is finished if the window finishes
                            break;
                        }
                    }

                    RenderWindowRequest::SendEvents(channel_target) => {
                        let mut subscriber = event_publisher.subscribe();

                        context
                            .run_in_background(async move {
                                let mut channel_target = channel_target;

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

                    RenderWindowRequest::CloseWindow => {
                        // The window will close its publisher in response to the events stream being closed
                        render_sender.close().await.ok();

                        // Shut down the event publisher
                        use std::mem;
                        let when_closed = event_publisher.when_closed();
                        mem::drop(event_publisher);

                        // Finally, wait for the publisher to finish up
                        when_closed.await;
                        return;
                    }

                    RenderWindowRequest::SetMinSize(value) => min_size.set(value),
                    RenderWindowRequest::SetMaxSize(value) => max_size.set(value),
                    RenderWindowRequest::SetTitle(value) => title.set(value),
                    RenderWindowRequest::SetIsTransparent(value) => is_transparent.set(value),
                    RenderWindowRequest::SetIsVisible(value) => is_visible.set(value),
                    RenderWindowRequest::SetIsResizable(value) => is_resizable.set(value),
                    RenderWindowRequest::SetMinimized(value) => is_minimized.set(value),
                    RenderWindowRequest::SetMaximized(value) => is_maximized.set(value),
                    RenderWindowRequest::SetFullscreen(value) => fullscreen.set(value),
                    RenderWindowRequest::SetHasDecorations(value) => has_decorations.set(value),
                    RenderWindowRequest::SetWindowLevel(value) => window_level.set(value),
                    RenderWindowRequest::SetImePosition(value) => ime_position.set(value),
                    RenderWindowRequest::SetImeAllowed(value) => ime_allowed.set(value),
                    RenderWindowRequest::SetTheme(value) => theme.set(value),
                    RenderWindowRequest::SetCursorPosition(value) => cursor_position.set(value),
                    RenderWindowRequest::SetCursorIcon(value) => cursor_icon.set(value),
                }
            }
        },
    )
}
