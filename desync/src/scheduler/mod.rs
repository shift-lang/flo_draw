/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//!
//! The scheduler provides the `JobQueue` synchronisation mechanism.
//!
//! # Scheduler
//!
//! The scheduler provides a new synchronisation mechanism: the `JobQueue`. You can get one
//! by calling `scheduler::queue()`:
//!
//! ```
//! use desync::scheduler;
//!
//! let queue = scheduler::queue();
//! ```
//!
//! A `JobQueue` allows jobs to be scheduled in the background. Jobs are scheduled in the order
//! that they arrive, so anything on a queue is run synchronously with respect to the queue
//! itself. The `desync` call can be used to schedule work:
//!
//! ```
//! # use desync::scheduler;
//! #
//! # let queue = scheduler::queue();
//! #
//! scheduler::desync(&queue, || println!("First job"));
//! scheduler::desync(&queue, || println!("Second job"));
//! scheduler::desync(&queue, || println!("Third job"));
//! ```
//!
//! These will be scheduled onto background threads created by the scheduler. There is also a
//! `sync` method. Unlike `desync`, this can return a value from the job function it takes
//! as a parameter and doesn't return until its job has completed:
//!
//! ```
//! # use desync::scheduler;
//! #
//! # let queue = scheduler::queue();
//! #
//! scheduler::desync(&queue, || println!("In the background"));
//! let someval = scheduler::sync(&queue, || { println!("In the foreground"); 42 });
//! # assert_eq!(someval, 42);
//! ```
//!
//! As queues are synchronous with themselves, it's possible to access data without needing
//! extra synchronisation primitives: `desync` is perfect for updating data in the background
//! and `sync` can be used to perform operations where data is returned to the calling thread.
//!

mod active_queue;
mod core;
mod desync_scheduler;
mod future_job;
mod job;
mod job_queue;
mod queue_resumer;
mod queue_state;
mod scheduler_future;
mod scheduler_thread;
mod sync_future;
mod try_sync_error;
mod unsafe_job;
mod wake_queue;
mod wake_thread;

pub use self::desync_scheduler::*;
pub use self::job_queue::JobQueue;
pub use self::queue_resumer::QueueResumer;
pub use self::scheduler_future::SchedulerFuture;
pub use self::try_sync_error::TrySyncError;
