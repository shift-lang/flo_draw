/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::sync::*;

use flo_stream::*;
use futures::prelude::*;
use futures::stream;

use flo_binding::*;
use flo_render::*;
use flo_scene::*;

use crate::draw_scene::*;
use crate::events::*;
use crate::window_properties::*;

///
/// Creates a window that can be rendered to by sending groups of render actions
///
pub fn create_render_window<'a, TProperties>(
    properties: TProperties,
) -> (
    Publisher<Vec<RenderAction>>,
    impl Send + Stream<Item = DrawEvent>,
)
where
    TProperties: 'a + FloWindowProperties,
{
    // Create the publisher to send the render actions to the stream
    let mut render_publisher = Publisher::new(1);
    let event_subscriber =
        create_render_window_from_stream(render_publisher.subscribe(), properties);

    // Publisher can now be used to render to the window
    (render_publisher, event_subscriber)
}

///
/// Sends the events for changing the properties in a set of WindowProperties
///
pub(crate) fn send_window_properties<TRequest>(
    context: &Arc<SceneContext>,
    window_properties: WindowProperties,
    channel: impl 'static + Send + EntityChannel<Message = TRequest>,
) -> Result<(), EntityFutureError>
where
    TRequest: Send + From<EventWindowRequest>,
{
    context.run_in_background(async move {
        // Follow the properties
        let min_size = follow(window_properties.min_size);
        let max_size = follow(window_properties.max_size);
        let title = follow(window_properties.title);
        let is_transparent = follow(window_properties.is_transparent);
        let is_visible = follow(window_properties.is_visible);
        let is_resizable = follow(window_properties.is_resizable);
        let is_minimized = follow(window_properties.is_minimized);
        let is_maximized = follow(window_properties.is_maximized);
        let fullscreen = follow(window_properties.fullscreen);
        let has_decorations = follow(window_properties.has_decorations);
        let window_level = follow(window_properties.window_level);
        let ime_position = follow(window_properties.ime_position);
        let ime_allowed = follow(window_properties.ime_allowed);
        let theme = follow(window_properties.theme);
        let cursor_position = follow(window_properties.cursor_position);
        let cursor_icon = follow(window_properties.cursor_icon);

        macro_rules! map {
            ($name:ident, $event:ident) => {
                $name.map(|$name| EventWindowRequest::$event($name))
            };
        }
        let min_size = map!(min_size, SetMinSize);
        let max_size = map!(max_size, SetMaxSize);
        let title = map!(title, SetTitle);
        let is_transparent = map!(is_transparent, SetIsTransparent);
        let is_visible = map!(is_visible, SetIsVisible);
        let is_resizable = map!(is_resizable, SetIsResizable);
        let is_minimized = map!(is_minimized, SetMinimized);
        let is_maximized = map!(is_maximized, SetMaximized);
        let fullscreen = map!(fullscreen, SetFullscreen);
        let has_decorations = map!(has_decorations, SetHasDecorations);
        let window_level = map!(window_level, SetWindowLevel);
        let ime_position = map!(ime_position, SetImePosition);
        let ime_allowed = map!(ime_allowed, SetImeAllowed);
        let theme = map!(theme, SetTheme);
        let cursor_position = map!(cursor_position, SetCursorPosition);
        let cursor_icon = map!(cursor_icon, SetCursorIcon);

        let mut requests = stream::select_all(vec![
            min_size.boxed(),
            max_size.boxed(),
            title.boxed(),
            is_transparent.boxed(),
            is_visible.boxed(),
            is_resizable.boxed(),
            is_minimized.boxed(),
            is_maximized.boxed(),
            fullscreen.boxed(),
            has_decorations.boxed(),
            window_level.boxed(),
            ime_position.boxed(),
            ime_allowed.boxed(),
            theme.boxed(),
            cursor_position.boxed(),
            cursor_icon.boxed(),
        ]);

        // Pass the requests on to the underlying window
        let mut channel = channel;
        while let Some(request) = requests.next().await {
            channel.send(request.into()).await.ok();
        }
    })?;

    Ok(())
}

///
/// Creates a window that renders a stream of actions
///
pub fn create_render_window_from_stream<'a, RenderStream, TProperties>(
    render_stream: RenderStream,
    properties: TProperties,
) -> impl Send + Stream<Item = DrawEvent>
where
    RenderStream: 'static + Send + Stream<Item = Vec<RenderAction>>,
    TProperties: 'a + FloWindowProperties,
{
    let properties = WindowProperties::from(&properties);

    // Create a new render window entity
    let render_window_entity = EntityId::new();
    let scene_context = flo_draw_scene_context();

    let render_channel = create_render_window_entity(
        &scene_context,
        render_window_entity,
        properties.size().get(),
    )
    .unwrap();

    // The events send to a channel
    let (events_channel, events_stream) = SimpleEntityChannel::new(render_window_entity, 5);

    // Pass events from the render stream onto the window using another entity (potentially this could be a background task for the render window entity?)
    let process_entity = EntityId::new();
    scene_context
        .create_entity::<(), _, _>(process_entity, move |context, _| {
            async move {
                let mut render_stream = render_stream.boxed();
                let mut render_channel = render_channel;

                send_window_properties(&context, properties, render_channel.clone()).ok();

                // Request event actions from the renderer
                render_channel
                    .send(RenderWindowRequest::SendEvents(events_channel.boxed()))
                    .await
                    .ok();

                // Main loop passes on the render actions (we don't process messages directed at this entity)
                while let Some(render_actions) = render_stream.next().await {
                    let maybe_err = render_channel
                        .send(RenderWindowRequest::Render(RenderRequest::Render(
                            render_actions,
                        )))
                        .await;

                    if maybe_err.is_err() {
                        // Stop if the request doesn't go through
                        break;
                    }
                }
            }
        })
        .unwrap();

    // We don't process messages in our background entity, so seal it off
    scene_context.seal_entity(process_entity).unwrap();

    // The events stream is the result
    events_stream
}
