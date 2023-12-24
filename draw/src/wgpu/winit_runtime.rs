use std::collections::HashMap;
use std::sync::*;
use std::sync::atomic::{AtomicU64, Ordering};

use flo_binding::*;
use flo_stream::*;
use futures::channel::oneshot;
use futures::future::LocalBoxFuture;
use futures::prelude::*;
use futures::task;
use wgpu;
use winit::event::{DeviceId, ElementState, Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoopWindowTarget};
use winit::window::{Fullscreen, Window, WindowId};

use crate::events::*;
use crate::window_properties::*;

use super::event_conversion::*;
use super::winit_thread::*;
use super::winit_thread_event::*;
use super::winit_window::*;

static NEXT_FUTURE_ID: AtomicU64 = AtomicU64::new(0);

pub(super) struct WindowData {
    window: Arc<Window>,
    event_publisher: Publisher<DrawEvent>,
}

///
/// Represents the state of the Winit runtime
///
pub(super) struct WinitRuntime {
    /// The event publishers for the windows being managed by the runtime
    pub(super) window_events: HashMap<WindowId, WindowData>,

    /// Maps future IDs to running futures
    pub(super) futures: HashMap<u64, LocalBoxFuture<'static, ()>>,

    /// Redraws that are pending for a particular window (the texture that's waiting to be displayed and the sender to be informed once the 'events cleared' event has arrived)
    pub(super) pending_redraws: HashMap<WindowId, (wgpu::SurfaceTexture, oneshot::Sender<()>)>,

    /// Yield events waiting for an indication that all events have been processed
    pub(super) pending_yields: Vec<oneshot::Sender<()>>,

    /// Set to true if this runtime will stop when all the windows are closed
    pub(super) will_stop_when_no_windows: bool,

    /// The pointer ID that we've assigned to winit devices
    pub(super) pointer_id: HashMap<DeviceId, PointerId>,

    /// The current state of each pointer (as a winit device)
    pub(super) pointer_state: HashMap<DeviceId, PointerState>,

    /// Set to true when we'll set the control flow to 'Exit' once the current set of events have finished processing
    pub(super) will_exit: bool,
}

///
/// Used to wake a future running on the winit thread
///
struct WinitFutureWaker {
    /// The ID of the future to wake, or 'None' if the future has already been woken up
    future_id: Mutex<Option<u64>>,
}

impl WinitRuntime {
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
        &mut *self.pointer_state.entry(*device_id)
            .or_insert_with(|| PointerState::new())
    }

    ///
    /// Handles an event from the rest of the process and updates the state
    ///
    pub fn handle_event(&mut self, event: Event<'_, WinitThreadEvent>, window_target: &EventLoopWindowTarget<WinitThreadEvent>, control_flow: &mut ControlFlow) {
        use Event::*;

        if *control_flow != ControlFlow::Exit {
            *control_flow = ControlFlow::Wait;
        }

        match event {
            NewEvents(_cause) => {}
            WindowEvent { window_id, event } => { self.handle_window_event(window_id, event); }
            DeviceEvent { device_id: _, event: _ } => {}
            UserEvent(thread_event) => { self.handle_thread_event(thread_event, window_target); }
            Suspended => {}
            Resumed => {}
            RedrawRequested(window_id) => {
                if let Some((pending_surface, redraw_finished)) = self.pending_redraws.remove(&window_id) {
                    // Present the surface
                    pending_surface.present();

                    // Signal the 'finished' event when the redraw events are all clear
                    self.pending_yields.push(redraw_finished);
                } else {
                    // self.request_redraw(window_id); 
                }
            }

            MainEventsCleared => {
                // Winit doesn't always respond to ControlFlow::Exit requests, setting it after the other events have cleared is an attempt
                // to make it exit more reliably (only partially successful).
                if self.will_exit {
                    *control_flow = ControlFlow::Exit;
                }
            }
            RedrawEventsCleared => {
                // Clear any pending yield requests
                for yield_sender in self.pending_yields.drain(..) {
                    yield_sender.send(()).ok();
                }
            }

            LoopDestroyed => {}
        }
    }

    ///
    /// Handles a winit window event
    ///
    fn handle_window_event(&mut self, window_id: WindowId, event: WindowEvent) {
        if let WindowEvent::CloseRequested = event {
            // Winit has a bug (at least in OS X) where setting control_flow to Exit does not actually shut it down
            // This issue does not occur if the 'Exit' request is done in response to closing a window
            // (This only partially works around the bug: the process will still not quit properly if the last window
            // is closed before the main routine finishes, ie when 'will_stop_when_no_windows' is still false)
            if self.will_stop_when_no_windows && self.window_events.len() <= 1 {
                self.will_exit = true;
            }
        }

        use WindowEvent::*;

        // Generate draw_events for the window event
        let draw_events = match event {
            Resized(new_size) => {
                vec![DrawEvent::Resize(new_size.width as f64, new_size.height as f64), DrawEvent::Redraw]
            }

            ScaleFactorChanged { scale_factor, new_inner_size } => {
                vec![DrawEvent::Scale(scale_factor), DrawEvent::Resize(new_inner_size.width as f64, new_inner_size.height as f64), DrawEvent::Redraw]
            }

            Moved(_position) => vec![],
            CloseRequested => vec![DrawEvent::Closed],
            Destroyed => vec![],
            DroppedFile(_path) => vec![],
            HoveredFile(_path) => vec![],
            HoveredFileCancelled => vec![],
            ReceivedCharacter(_c) => vec![],
            Focused(_focused) => vec![],
            ModifiersChanged(_state) => vec![],
            TouchpadPressure { device_id: _, pressure: _, stage: _ } => vec![],
            TouchpadMagnify { .. } => vec![],
            TouchpadRotate { .. } => vec![],
            SmartMagnify { .. } => vec![],
            AxisMotion { device_id: _, axis: _, value: _ } => vec![],
            Touch(_touch) => vec![],
            ThemeChanged(_theme) => vec![],
            Occluded(_) => vec![],

            // Keyboard events
            KeyboardInput { device_id: _, input, is_synthetic: _, } => {
                // Convert the keycode
                let key = input.virtual_keycode.map(|keycode| key_from_winit(&keycode));
                let key = if key == Some(Key::Unknown) { None } else { key };

                // TODO: for modifier keys, generate keydown/up using the modifier state

                // Generate the event for this keypress
                match input.state {
                    ElementState::Pressed => vec![DrawEvent::KeyDown(input.scancode as _, key)],
                    ElementState::Released => vec![DrawEvent::KeyUp(input.scancode as _, key)]
                }
            }

            // Pointer events
            CursorMoved { device_id, position, .. } => {
                // Update the pointer state
                let pointer_id = self.id_for_pointer(&device_id);
                let pointer_state = self.state_for_pointer(&device_id);

                pointer_state.location_in_window = (position.x, position.y);

                // Generate the mouse event
                let pointer_state = pointer_state.clone();
                let is_drag = pointer_state.buttons.len() > 0;
                let action = if is_drag { PointerAction::Drag } else { PointerAction::Move };

                vec![DrawEvent::Pointer(action, pointer_id, pointer_state)]
            }

            CursorEntered { device_id } => {
                // Generate the 'entered' event with the current pointer state
                let pointer_id = self.id_for_pointer(&device_id);
                let pointer_state = self.state_for_pointer(&device_id);

                // Generate the mouse event
                let pointer_state = pointer_state.clone();
                vec![DrawEvent::Pointer(PointerAction::Enter, pointer_id, pointer_state)]
            }

            CursorLeft { device_id } => {
                // Generate the 'entered' event with the current pointer state
                let pointer_id = self.id_for_pointer(&device_id);
                let pointer_state = self.state_for_pointer(&device_id);

                // Generate the mouse event
                let pointer_state = pointer_state.clone();
                vec![DrawEvent::Pointer(PointerAction::Leave, pointer_id, pointer_state)]
            }

            MouseInput { device_id, state, button, .. } => {
                // Generate the 'entered' event with the current pointer state
                let pointer_id = self.id_for_pointer(&device_id);
                let pointer_state = self.state_for_pointer(&device_id);

                // TODO: for modifier keys, generate keydown/up using the modifier state

                // Update the pointe state
                let button = button_from_winit(&button);
                let action = match state {
                    ElementState::Pressed => {
                        if !pointer_state.buttons.contains(&button) {
                            pointer_state.buttons.push(button);
                        }

                        PointerAction::ButtonDown
                    }

                    ElementState::Released => {
                        pointer_state.buttons.retain(|item| item != &button);

                        PointerAction::ButtonUp
                    }
                };

                // Generate the mouse event
                let pointer_state = pointer_state.clone();
                vec![DrawEvent::Pointer(action, pointer_id, pointer_state)]
            }

            MouseWheel { device_id: _, delta: _, phase: _, .. } => vec![],
            Ime(_) => vec![],
        };

        if let Some(window_data) = self.window_events.get_mut(&window_id) {
            // Dispatch the draw events using a process
            if draw_events.len() > 0 {
                // Need to republish the window events so we can share with the process
                let mut window_events = window_data.event_publisher.republish();

                self.run_process(async move {
                    for evt in draw_events {
                        window_events.publish(evt).await;
                    }
                });
            }
        }
    }

    ///
    /// Sends a redraw request to a window
    ///
    fn request_redraw(&mut self, window_id: WindowId) {
        if let Some(window_data) = self.window_events.get_mut(&window_id) {
            // Need to republish the window events so we can share with the process
            let mut window_events = window_data.event_publisher.republish();

            self.run_process(async move {
                window_events.publish(DrawEvent::Redraw).await;
            });
        }
    }

    ///
    /// Handles one of our user events from the WinitThreadEvent enum
    ///
    fn handle_thread_event(&mut self, event: WinitThreadEvent, window_target: &EventLoopWindowTarget<WinitThreadEvent>) {
        use WinitThreadEvent::*;

        match event {
            CreateRenderWindow(actions, events, window_properties) => {
                // Get the initial set of properties for the window
                let title = window_properties.title().get();
                let (size_x, size_y) = window_properties.size().get();
                let fullscreen = window_properties.fullscreen().get();
                let decorations = window_properties.has_decorations().get();

                let fullscreen = if fullscreen { Some(Fullscreen::Borderless(None)) } else { None };

                // Create a window
                let window_builder = winit::window::WindowBuilder::new()
                    .with_title(title)
                    .with_inner_size(winit::dpi::LogicalSize::new(size_x as f64, size_y as _))
                    .with_fullscreen(fullscreen)
                    .with_decorations(decorations);
                let window = window_builder.build(window_target).expect("New window");

                // Build a new Winit window
                let window = Arc::new(window);
                let window_id = window.id();
                let size = window.inner_size();
                let scale = window.scale_factor();

                // Store the publisher for the events for this window
                let mut initial_events = events.republish_weak();
                let window_data = WindowData {
                    window: Arc::clone(&window),
                    event_publisher: events,
                };
                let window = WinitWindow::new(window);
                self.window_events.insert(window_id, window_data);

                // Run the window as a process on this thread
                self.run_process(async move {
                    // Send the initial events for this window (set the size and the DPI)
                    initial_events.publish(DrawEvent::Resize(size.width as f64, size.height as f64)).await;
                    initial_events.publish(DrawEvent::Scale(scale)).await;
                    initial_events.publish(DrawEvent::Redraw).await;

                    let window_events = initial_events;

                    // Process the actions for the window
                    send_actions_to_window(window, actions, window_events, window_properties).await;

                    // Stop processing events for the window once there are no more actions
                    winit_thread().send_event(WinitThreadEvent::StopSendingToWindow(window_id));
                });
            }

            StopSendingToWindow(window_id) => {
                self.window_events.remove(&window_id);
                self.pending_redraws.remove(&window_id);

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

            PresentSurface(window_id, surface_texture, completed) => {
                // Store this present event
                self.pending_redraws.insert(window_id, (surface_texture, completed));

                // Trigger a redraw on the window
                if let Some(window_data) = self.window_events.get(&window_id) {
                    // Queue up a redraw for this window
                    window_data.window.request_redraw();
                } else {
                    // Window doesn't exist, so just cancel the pending redraw
                    self.pending_redraws.remove(&window_id);
                }
            }

            Yield(sender) => {
                self.pending_yields.push(sender);
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
    fn run_process<Fut: 'static + Future<Output=()>>(&mut self, future: Fut) {
        // Box the future for polling
        let future = future.boxed_local();

        // Assign an ID to this future (we use this for waking it up)
        let future_id = NEXT_FUTURE_ID.fetch_add(1, Ordering::Relaxed);

        // Store in the runtime
        self.futures.insert(future_id, future);

        // Wake the future as soon as possible
        winit_thread().send_event(WinitThreadEvent::WakeFuture(future_id));
    }

    ///
    /// Causes the future with the specified ID to be polled
    ///
    fn poll_future(&mut self, future_id: u64) {
        if let Some(future) = self.futures.get_mut(&future_id) {
            // Create a context to poll this future in
            let winit_waker = WinitFutureWaker { future_id: Mutex::new(Some(future_id)) };
            let winit_waker = task::waker(Arc::new(winit_waker));
            let mut winit_context = task::Context::from_waker(&winit_waker);

            // Poll the future
            let poll_result = future.poll_unpin(&mut winit_context);

            // Remove the future from the list if it has completed
            if let task::Poll::Ready(_) = poll_result {
                self.futures.remove(&future_id);
            }
        }
    }
}

impl task::ArcWake for WinitFutureWaker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        // If this is the first wake request for this waker...
        if let Some(future_id) = arc_self.future_id.lock().unwrap().take() {
            // Send a wake request to winit
            winit_thread().send_event(WinitThreadEvent::WakeFuture(future_id));
        }
    }
}
