/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use desync::scheduler::*;

#[test]
#[cfg(not(miri))] // slow!
fn will_despawn_extra_threads() {
    // As we join with the threads, we'll timeout if any of the spawned threads fail to end
    let scheduler = scheduler();

    // Maximum of 10 threads, but we'll spawn 20
    scheduler.set_max_threads(10);
    for _ in 1..20 {
        scheduler.spawn_thread();
    }

    scheduler.despawn_threads_if_overloaded();
}
