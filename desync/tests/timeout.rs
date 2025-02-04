/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

extern crate desync;

use std::sync::mpsc::*;
use std::thread;
use std::time::*;

use desync::scheduler::*;

#[cfg(all(not(target_family = "wasm"), not(miri)))]
pub fn timeout<TFn: 'static + Send + FnOnce() -> ()>(action: TFn, millis: u64) {
    enum ThreadState {
        Ok,
        Timeout,
        Panic,
    }

    let (tx, rx) = channel();
    let (tx1, tx2) = (tx.clone(), tx.clone());

    thread::Builder::new()
        .name("test timeout thread".to_string())
        .spawn(move || {
            struct DetectPanic(Sender<ThreadState>);
            impl Drop for DetectPanic {
                fn drop(&mut self) {
                    if thread::panicking() {
                        self.0.send(ThreadState::Panic).ok();
                    }
                }
            }

            let _detectpanic = DetectPanic(tx1.clone());

            action();
            tx1.send(ThreadState::Ok).ok();
        })
        .expect("Create timeout run thread");

    let (timer_done, timer_done_recv) = channel();
    let timer = thread::Builder::new()
        .name("timeout thread".to_string())
        .spawn(move || {
            let done = timer_done_recv.recv_timeout(Duration::from_millis(millis));
            if done.is_err() {
                tx2.send(ThreadState::Timeout).ok();
            }
        })
        .expect("Create timeout timer thread");

    match rx.recv().expect("Receive timeout status") {
        ThreadState::Ok => {
            // Stop the timer thread
            timer_done.send(()).expect("Stop timer");
            timer.join().expect("Wait for timer to stop");
        }
        ThreadState::Timeout => {
            println!("{:?}", scheduler());
            panic!("Timeout");
        }
        ThreadState::Panic => {
            println!("{:?}", scheduler());
            panic!("Timed thread panicked");
        }
    }
}

#[cfg(any(target_family = "wasm", miri))]
pub fn timeout<TFn: 'static + Send + FnOnce() -> ()>(action: TFn, _millis: u64) {
    action();
}
