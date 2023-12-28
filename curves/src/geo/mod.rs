/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//!
//! # Traits for basic geometric definitions
//!
//! This provides some basic geometric definitions. The `Geo` trait can be implemented by any type that has
//! a particular type of coordinate - for example, implementations of `BezierCurve` need to implement `Geo`
//! in order to describe what type they use for coordinates.
//!
//! `BoundingBox` provides a way to describe axis-aligned bounding boxes. It too is a trait, making it
//! possible to request bounding boxes in types other than the default `Bounds` type supplied by the
//! library.
//!

mod bounding_box;
mod coord1;
mod coord2;
mod coord3;
mod coordinate;
mod coordinate_ext;
mod geo;
mod has_bounds;
mod space1;
mod sweep;

pub use self::bounding_box::*;
pub use self::coord1::*;
pub use self::coord2::*;
pub use self::coord3::*;
pub use self::coordinate::*;
pub use self::coordinate_ext::*;
pub use self::geo::*;
pub use self::has_bounds::*;
pub use self::space1::*;
pub use self::sweep::*;
