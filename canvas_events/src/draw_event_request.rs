/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::draw_event::*;

/// Draw events are already specified in the flo_canvas_evevents library and are sent singly so this is just an alias for that type
pub type DrawEventRequest = DrawEvent;
