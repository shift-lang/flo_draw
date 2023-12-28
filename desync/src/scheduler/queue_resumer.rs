/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use futures::channel::oneshot;

///
/// The queue resumer is used to resume a queue that was suspended using the `suspend()` function in the scheduler
///
pub struct QueueResumer {
    pub(super) resume: oneshot::Sender<()>,
}

impl QueueResumer {
    ///
    /// Resumes a suspended queue
    ///
    pub fn resume(self) {
        // Send to the channel
        self.resume.send(()).ok();
    }
}
