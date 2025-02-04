/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::job_queue::*;
use super::queue_state::*;

use futures::task::ArcWake;
use std::sync::*;
use std::thread::Thread;

///
/// Waker that will wake the specified thread
///
pub(super) struct WakeThread(pub(super) Arc<JobQueue>, pub(super) Thread);

impl ArcWake for WakeThread {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        // Decompose this structure
        let WakeThread(ref queue, ref thread) = **arc_self;

        // Move the queue to the idle state if we can
        {
            let mut queue_core = queue.core.lock().unwrap();

            // Queue can be woken if it's in the WaitingForWake state
            match queue_core.state {
                QueueState::WaitingForWake => queue_core.state = QueueState::Idle,
                QueueState::WaitingForUnpark => queue_core.state = QueueState::Running,
                QueueState::Running => queue_core.state = QueueState::AwokenWhileRunning,
                other_state => queue_core.state = other_state,
            }
        }

        // Wake the thread
        thread.unpark();
    }
}
