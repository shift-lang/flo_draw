/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![allow(clippy::new_without_default)]

use uuid::*;

///
/// Uniquely identifies an entity
///
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct EntityId(Uuid);

impl EntityId {
    ///
    /// Creates a new, unique, entity ID
    ///
    pub fn new() -> EntityId {
        EntityId(Uuid::new_v4())
    }

    ///
    /// Creates an entity ID with a well-known UUID
    ///
    pub const fn well_known(uuid: Uuid) -> EntityId {
        EntityId(uuid)
    }
}
