/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::io::ErrorKind;

///
/// Possible error conditions from a `try_sync()` call on the scheduler
///
#[derive(Clone, PartialEq, Debug)]
pub enum TrySyncError {
    /// The queue is busy, so the function has not been executed
    Busy,
}

impl Into<ErrorKind> for TrySyncError {
    fn into(self) -> ErrorKind {
        match self {
            TrySyncError::Busy => ErrorKind::WouldBlock,
        }
    }
}
