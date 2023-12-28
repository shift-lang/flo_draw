/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// See "A bezier curve-based root-finder", Philip J Schneider, Graphics Gems

mod find_roots;
mod nearest_point_bezier_root_finder;
mod polynomial_to_bezier;

pub use find_roots::*;
pub use nearest_point_bezier_root_finder::*;
pub use polynomial_to_bezier::*;
