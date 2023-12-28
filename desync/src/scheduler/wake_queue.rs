/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::core::*;
use super::job_queue::*;
use super::queue_state::*;

use futures::task::ArcWake;
use std::sync::*;

///
/// Waker that will wake the specified queue in the specified scheduler core
///
pub(super) struct WakeQueue(pub(super) Arc<JobQueue>, pub(super) Arc<SchedulerCore>);

impl ArcWake for WakeQueue {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        // Decompose this structure
        let WakeQueue(ref queue, ref core) = **arc_self;

        // Move the queue to the idle state if we can
        {
            let mut queue_core = queue.core.lock().unwrap();

            // Queue can be woken if it's in the WaitingForWake state
            match queue_core.state {
                QueueState::WaitingForUnpark => {
                    // Assume that this was part of a DoubleWake that has become stale (woke up, reached a park, created a WakeThread for notifications)
                    queue_core.state = QueueState::WaitingForUnpark;
                    return;
                }

                QueueState::WaitingForWake => queue_core.state = QueueState::Idle,
                QueueState::Running => queue_core.state = QueueState::AwokenWhileRunning,
                other_state => queue_core.state = other_state,
            }
        }

        // Cause the core to reschedule its events
        core.reschedule_queue(queue, Arc::clone(core));
    }
}
