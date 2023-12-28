/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

///
/// Handle referencing a renderer layer
///
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct LayerHandle(pub u64);
