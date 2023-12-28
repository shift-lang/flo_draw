/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

///
/// Represents an RGBA colour as 8-bit valus
///
#[derive(Clone, Copy, PartialEq, Debug, Hash)]
pub struct Rgba8(pub [u8; 4]);
