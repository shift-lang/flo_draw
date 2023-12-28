/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use winit::dpi::PhysicalPosition;

///
/// A unique identifier assigned to a specific pointer on the system (a device that has a mouse and touch input might be tracking
/// multiple pointer devices)
///
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PointerId(pub u64);

///
/// The button on a mouse or other device
///
/// If a device only has one means of input (eg, a pen being pressed against the screen),
/// this is considered to be the 'Left' button.
///
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Button {
    Left,
    Middle,
    Right,
    Other(u64),
}

///
/// The action associated with a pointer event
///
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PointerAction {
    /// The pointer has entered the window
    Enter,

    /// The pointer has left the window
    Leave,

    /// Moving a pointer with no buttons pressed
    Move,

    /// A new button has been pressed
    ButtonDown,

    /// Moving the pointer with a button pressed (drag events can move outside the bounds of the window)
    Drag,

    /// A button has been released
    ButtonUp,

    /// A button has been released in a cancellation gesture (eg, due to palm rejection), invalidating a previous drag action
    Cancel,
}

///
/// Describes the state of a pointer device
///
/// Note: while we support the various different axes that a tablet device might support, presently glutin does not provide
/// this information to us, so these values are currently always set to 'None'.
///
#[derive(Clone, PartialEq, Debug)]
pub struct PointerState {
    /// The x and y coordinates of the pointer's location in the window
    pub location_in_window: (f64, f64),

    /// If the view is displaying scaled content, this is the location of the pointer in the coordinate scheme of that content
    pub location_in_canvas: Option<(f64, f64)>,
}

impl PointerState {
    ///
    /// Creates a pointer state in the default state
    ///
    pub fn new(point: PhysicalPosition<f64>) -> PointerState {
        PointerState {
            location_in_window: (point.x, point.y),
            location_in_canvas: None,
        }
    }
}
