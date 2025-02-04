/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//!
//! # Events
//!
//! `flo_draw` is currently based on glutin, but uses its own event structure: this is to make it so that
//! it's possible for future versions to replace glutin easily if that ever proves to be necessary, and
//! to support easy porting of code using `flo_draw` to other windowing systems. This also isolates software
//! implemented using `flo_draw` from changes to glutin.
//!

pub use self::draw_event::*;
pub use self::draw_event_request::*;
pub use self::draw_window_request::*;
pub use self::key::*;
pub use self::pointer_event::*;
pub use self::render_request::*;

mod draw_event;
mod key;
mod pointer_event;

mod draw_event_request;
mod render_request;

mod draw_window_request;
