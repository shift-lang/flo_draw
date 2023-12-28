/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//!
//! # Describing circular arcs
//!
//! The `arc` module provides routines for describing circular arcs and converting them to bezier
//! curves.
//!

mod circle;

pub use self::circle::*;

// TODO: represent arcs in more than 2 dimensions
