/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use flo_stream::*;
use futures::{future::LocalBoxFuture, prelude::*, task};
use glutin::{
    config::{ConfigTemplateBuilder, GlConfig},
    context::{ContextApi, ContextAttributesBuilder, Version},
    display::{GetGlDisplay, GlDisplay},
};
use glutin_winit::DisplayBuilder;
use raw_window_handle::HasRawWindowHandle;
use winit::{
    event::{DeviceId, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopWindowTarget},
    window::{Fullscreen, WindowId},
};

use flo_binding::*;

use crate::{events::*, window_properties::*};

use super::{glutin_thread::*, glutin_thread_event::*, glutin_window::*};

static NEXT_FUTURE_ID: AtomicU64 = AtomicU64::new(0);

///
/// Represents the state of the Glutin runtime
///
pub(super) struct GlutinRuntime {
    /// The event publishers for the windows being managed by the runtime
    pub(super) window_events: HashMap<WindowId, (Publisher<DrawEvent>, Publisher<SuspendResume>)>,

    /// Maps future IDs to running futures
    pub(super) futures: HashMap<u64, LocalBoxFuture<'static, ()>>,

    /// Set to true if this runtime will stop when all the windows are closed
    pub(super) will_stop_when_no_windows: bool,

    /// The pointer ID that we've assigned to glutin devices
    pub(super) pointer_id: HashMap<DeviceId, PointerId>,

    /// The current state of each pointer (as a glutin device)
    pub(super) pointer_state: HashMap<DeviceId, PointerState>,

    /// Set to true when we'll set the control flow to 'Exit' once the current set of events have finished processing
    pub(super) will_exit: bool,

    /// Set to true if the runtime is suspended
    pub(super) suspended: bool,
}

///
/// Used to wake a future running on the glutin thread
///
struct GlutinFutureWaker {
    /// The ID of the future to wake, or 'None' if the future has already been woken up
    future_id: Mutex<Option<u64>>,
}

impl GlutinRuntime {
    ///
    /// Retrieves or assigns an ID to a device
    ///
    fn id_for_pointer(&mut self, device_id: &DeviceId) -> PointerId {
        if let Some(pointer_id) = self.pointer_id.get(device_id) {
            // Already assigned
            *pointer_id
        } else {
            // Assign a new pointer ID
            let pointer_id = self.pointer_id.len() as _;
            let pointer_id = PointerId(pointer_id);

            // Store in the hash map
            self.pointer_id.insert(*device_id, pointer_id);
            pointer_id
        }
    }

    ///
    /// Retrieves the current state for a particular pointer in a mutable form
    ///
    fn state_for_pointer<'a>(&'a mut self, device_id: &'a DeviceId) -> &'a mut PointerState {
        &mut *self
            .pointer_state
            .entry(*device_id)
            .or_insert_with(|| PointerState::new((0.0, 0.0).into()))
    }

    ///
    /// Handles an event from the rest of the process and updates the state
    ///
    pub fn handle_event(
        &mut self,
        event: Event<'_, GlutinThreadEvent>,
        window_target: &EventLoopWindowTarget<GlutinThreadEvent>,
        control_flow: &mut ControlFlow,
    ) {
        use Event::*;

        if *control_flow != ControlFlow::Exit {
            *control_flow = ControlFlow::Wait;
        }

        match event {
            NewEvents(_cause) => {}
            WindowEvent { window_id, event } => {
                self.handle_window_event(window_id, event);
            }
            DeviceEvent {
                device_id: _,
                event: _,
            } => {}
            UserEvent(thread_event) => {
                self.handle_thread_event(thread_event, window_target);
            }
            Suspended => {
                self.request_suspended();
            }
            Resumed => {
                self.request_resumed();
            }
            RedrawRequested(window_id) => {
                self.request_redraw(window_id);
            }

            MainEventsCleared => {
                // Glutin doesn't always respond to ControlFlow::Exit requests, setting it after the other events have cleared is an attempt
                // to make it exit more reliably (only partially successful).
                if self.will_exit {
                    *control_flow = ControlFlow::Exit;
                }
            }
            RedrawEventsCleared => {}
            LoopDestroyed => {}
        }
    }

    ///
    /// Handles a glutin window event
    ///
    fn handle_window_event(&mut self, window_id: WindowId, event: WindowEvent) {
        if let CloseRequested = event {
            // Glutin has a problem (at least in OS X) where setting control_flow to Exit does not actually shut it down
            // This issue does not occur if the 'Exit' request is done in response to closing a window
            // (This only partially works around the issue: the process will still not quit properly if the last window
            // is closed before the main routine finishes, ie when 'will_stop_when_no_windows' is still false)
            if self.will_stop_when_no_windows && self.window_events.len() <= 1 {
                self.will_exit = true;
            }
        }

        use WindowEvent::*;

        let event = match event {
            Resized(size) => DrawEvent::Resized(size),
            CloseRequested => DrawEvent::CloseRequested,
            Destroyed => DrawEvent::Destroyed,
            DroppedFile(path) => DrawEvent::DroppedFile(path),
            HoveredFile(path) => DrawEvent::HoveredFile(path),
            HoveredFileCancelled => DrawEvent::HoveredFileCancelled,
            ReceivedCharacter(char) => DrawEvent::ReceivedCharacter(char),
            Focused(is_focused) => DrawEvent::Focused(is_focused),
            KeyboardInput {
                input,
                is_synthetic,
                ..
            } => DrawEvent::KeyboardInput {
                input,
                is_synthetic,
            },
            ModifiersChanged(modifier_state) => DrawEvent::ModifiersChanged(modifier_state),
            Ime(ime) => DrawEvent::Ime(ime),
            CursorMoved { position, .. } => DrawEvent::CursorMoved {
                state: PointerState::new(position),
            },
            CursorEntered { .. } => DrawEvent::CursorEntered,
            CursorLeft { .. } => DrawEvent::CursorLeft,
            MouseWheel { delta, phase, .. } => DrawEvent::MouseWheel { delta, phase },
            MouseInput { state, button, .. } => DrawEvent::MouseInput { state, button },
            TouchpadMagnify { delta, phase, .. } => DrawEvent::TouchpadMagnify { delta, phase },
            SmartMagnify { .. } => DrawEvent::SmartMagnify,
            TouchpadRotate { delta, phase, .. } => DrawEvent::TouchpadRotate { delta, phase },
            TouchpadPressure {
                pressure, stage, ..
            } => DrawEvent::TouchpadPressure { pressure, stage },
            AxisMotion { axis, value, .. } => DrawEvent::AxisMotion { axis, value },
            Touch(touch) => DrawEvent::Touch(touch),
            ScaleFactorChanged {
                scale_factor,
                new_inner_size,
            } => DrawEvent::ScaleFactorChanged {
                scale_factor,
                new_inner_size: new_inner_size.clone(),
            },
            ThemeChanged(theme) => DrawEvent::ThemeChanged(theme),
            Occluded(is_occluded) => DrawEvent::Occluded(is_occluded),
            Moved(position) => DrawEvent::Moved(position),
        };

        if let Some(window_events) = self.window_events.get_mut(&window_id) {
            // Dispatch the draw events using a process
            // Need to republish the window events so we can share with the process
            let mut window_events = window_events.0.republish();

            self.run_process(async move {
                window_events.publish(event).await;
            });
        }
    }

    ///
    /// Sends a redraw request to a window
    ///
    fn request_resumed(&mut self) {
        self.suspended = false;

        // Need to republish the window events so we can share with the process
        let window_events = self
            .window_events
            .values()
            .map(|(draw, suspend)| (draw.republish(), suspend.republish()))
            .collect::<Vec<_>>();

        for (mut draw_events, mut suspend_events) in window_events {
            self.run_process(async move {
                suspend_events.publish(SuspendResume::Resumed).await;
                draw_events.publish(DrawEvent::Redraw).await;
            });
        }
    }

    ///
    /// Sends a redraw request to a window
    ///
    fn request_suspended(&mut self) {
        self.suspended = true;

        // Need to republish the window events so we can share with the process
        let window_events = self
            .window_events
            .values()
            .map(|(_, suspend)| suspend.republish())
            .collect::<Vec<_>>();

        for mut suspend_events in window_events {
            self.run_process(async move {
                suspend_events.publish(SuspendResume::Suspended).await;
            });
        }
    }

    ///
    /// Sends a redraw request to a window
    ///
    fn request_redraw(&mut self, window_id: WindowId) {
        if let Some(window_events) = self.window_events.get_mut(&window_id) {
            // Need to republish the window events so we can share with the process
            let mut window_events = window_events.0.republish();

            self.run_process(async move {
                window_events.publish(DrawEvent::Redraw).await;
            });
        }
    }

    ///
    /// Handles one of our user events from the GlutinThreadEvent enum
    ///
    fn handle_thread_event(
        &mut self,
        event: GlutinThreadEvent,
        window_target: &EventLoopWindowTarget<GlutinThreadEvent>,
    ) {
        use GlutinThreadEvent::*;

        match event {
            CreateRenderWindow(actions, events, window_properties) => {
                // Get the initial set of properties for the window
                let title = window_properties.title().get();
                let (size_x, size_y) = window_properties.size().get();
                let fullscreen = window_properties.fullscreen().get();
                let decorations = window_properties.has_decorations().get();

                let fullscreen = if fullscreen {
                    Some(Fullscreen::Borderless(None))
                } else {
                    None
                };

                // Create a window
                let window_builder = winit::window::WindowBuilder::new()
                    .with_title(title)
                    .with_inner_size(winit::dpi::LogicalSize::new(size_x as f64, size_y as _))
                    .with_fullscreen(fullscreen)
                    .with_decorations(decorations);
                let display_builder =
                    DisplayBuilder::new().with_window_builder(Some(window_builder));
                let template = ConfigTemplateBuilder::new()
                    .prefer_hardware_accelerated(Some(true))
                    .with_alpha_size(8);

                let (window, gl_config) = display_builder
                    .build(window_target, template, |configs| {
                        configs
                            .reduce(|a, b| {
                                if a.num_samples() > b.num_samples() {
                                    a
                                } else {
                                    b
                                }
                            })
                            .unwrap()
                    })
                    .unwrap();
                let window = window.unwrap();

                let raw_window_handle = Some(window.raw_window_handle());
                let gl_display = gl_config.display();
                let context_attributes = ContextAttributesBuilder::new().build(raw_window_handle);
                let fallback_context_attributes = ContextAttributesBuilder::new()
                    .with_context_api(ContextApi::Gles(None))
                    .build(raw_window_handle);
                let legacy_context_attributes = ContextAttributesBuilder::new()
                    .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
                    .build(raw_window_handle);
                let windowed_context = unsafe {
                    gl_display
                        .create_context(&gl_config, &context_attributes)
                        .unwrap_or_else(|_| {
                            gl_display
                                .create_context(&gl_config, &fallback_context_attributes)
                                .unwrap_or_else(|_| {
                                    gl_display
                                        .create_context(&gl_config, &legacy_context_attributes)
                                        .expect("failed to create context")
                                })
                        })
                };

                // Finalize the window (might be unsafe under operating systems like Android, but adding this to the window itself requires considerable extra state...)
                let window_builder = winit::window::WindowBuilder::new();
                glutin_winit::finalize_window(window_target, window_builder, &gl_config).unwrap();

                // Store the window context in a new glutin window
                let mut suspend_resume = Publisher::new(1);
                let suspend_resume_subscriber = suspend_resume.subscribe();

                let window_id = window.id();
                let size = window.inner_size();
                let scale = window.scale_factor();
                let window = GlutinWindow::new(windowed_context, gl_config, window);

                // Immediately resume the window if we're not in a suspended state
                if !self.suspended {
                    let mut suspend_resume = suspend_resume.republish_weak();
                    self.run_process(async move {
                        suspend_resume.publish(SuspendResume::Resumed).await;
                    })
                }

                // Store the publisher for the events for this window
                let mut initial_events = events.republish_weak();
                self.window_events
                    .insert(window_id, (events, suspend_resume));

                // Run the window as a process on this thread
                self.run_process(async move {
                    // Send the initial events for this window (set the size and the DPI)
                    initial_events.publish(DrawEvent::Resized(size)).await;
                    initial_events
                        .publish(DrawEvent::ScaleFactorChanged {
                            new_inner_size: size,
                            scale_factor: scale,
                        })
                        .await;
                    initial_events.publish(DrawEvent::Redraw).await;

                    let window_events = initial_events;

                    // Process the actions for the window
                    send_actions_to_window(
                        window,
                        suspend_resume_subscriber,
                        actions,
                        window_events,
                        window_properties,
                    )
                    .await;

                    // Stop processing events for the window once there are no more actions
                    glutin_thread().send_event(GlutinThreadEvent::StopSendingToWindow(window_id));
                });
            }

            StopSendingToWindow(window_id) => {
                self.window_events.remove(&window_id);

                if self.window_events.len() == 0 && self.will_stop_when_no_windows {
                    self.will_exit = true;
                }
            }

            RunProcess(start_process) => {
                self.run_process(start_process());
            }

            WakeFuture(future_id) => {
                self.poll_future(future_id);
            }

            StopWhenAllWindowsClosed => {
                self.will_stop_when_no_windows = true;

                if self.window_events.len() == 0 {
                    self.will_exit = true;
                }
            }
        }
    }

    ///
    /// Runs a process in the context of this runtime
    ///
    fn run_process<Fut: 'static + Future<Output = ()>>(&mut self, future: Fut) {
        // Box the future for polling
        let future = future.boxed_local();

        // Assign an ID to this future (we use this for waking it up)
        let future_id = NEXT_FUTURE_ID.fetch_add(1, Ordering::Relaxed);

        // Store in the runtime
        self.futures.insert(future_id, future);

        // Perform the initial polling operation on the future
        self.poll_future(future_id);
    }

    ///
    /// Causes the future with the specified ID to be polled
    ///
    fn poll_future(&mut self, future_id: u64) {
        if let Some(future) = self.futures.get_mut(&future_id) {
            // Create a context to poll this future in
            let glutin_waker = GlutinFutureWaker {
                future_id: Mutex::new(Some(future_id)),
            };
            let glutin_waker = task::waker(Arc::new(glutin_waker));
            let mut glutin_context = task::Context::from_waker(&glutin_waker);

            // Poll the future
            let poll_result = future.poll_unpin(&mut glutin_context);

            // Remove the future from the list if it has completed
            if let task::Poll::Ready(_) = poll_result {
                self.futures.remove(&future_id);
            }
        }
    }
}

impl task::ArcWake for GlutinFutureWaker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        // If this is the first wake request for this waker...
        if let Some(future_id) = arc_self.future_id.lock().unwrap().take() {
            // Send a wake request to glutin
            glutin_thread().send_event(GlutinThreadEvent::WakeFuture(future_id));
        }
    }
}
