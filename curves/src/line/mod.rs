/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//!
//! # Manipulating and describing lines
//!
//! While `flo_curves` deals mostly with curves, it also supplies a small library of functions for manipulating
//! lines. The `Line` trait can be implemented on other types that define lines, enabling them to be used anywhere
//! the library needs to perform an operation on a line.
//!
//! The basic line type is simply a tuple of two points (that is, any tuple of two values of the same type that
//! implements `Coordinate`).
//!

mod coefficients;
mod intersection;
mod line;
mod to_curve;

pub use self::coefficients::*;
pub use self::intersection::*;
pub use self::line::*;
pub use self::to_curve::*;

pub use super::geo::*;
