/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::sync::*;

use crate::notify_fn::*;
use crate::releasable::*;
use crate::traits::*;

///
/// Watcher that calls a 'notify' method whenever its core value changes
///
pub struct NotifyWatcher<TValueFn, TValue>
where
    TValueFn: Fn() -> TValue,
{
    /// Function to retrieve the value that is being watched
    get_value: TValueFn,

    /// Set to true if the value has updated since it was last retrieved via 'get_value'
    value_updated: Arc<Mutex<bool>>,

    /// The notification that is fired for this watcher
    notification: ReleasableNotifiable,
}

impl<TValueFn, TValue> Drop for NotifyWatcher<TValueFn, TValue>
where
    TValueFn: Fn() -> TValue,
{
    fn drop(&mut self) {
        self.notification.done();
    }
}

impl<TValueFn, TValue> Watcher<TValue> for NotifyWatcher<TValueFn, TValue>
where
    TValueFn: Fn() -> TValue,
{
    fn get(&self) -> TValue {
        // Lock the 'updated' mutex so if an update arrives, it will fire the notification
        let mut updated = self.value_updated.lock().unwrap();

        // Retrieve the current value of the binding
        let value = (self.get_value)();

        // Value has not been updated since it was last read
        *updated = false;

        // Return the value
        value
    }
}

impl<TValueFn, TValue> NotifyWatcher<TValueFn, TValue>
where
    TValueFn: Fn() -> TValue,
{
    ///
    /// Creates a new notify watcher
    ///
    /// The return value is the watcher and the function to call to indicate that a change has happened in the
    /// underlying data store (the corresponding `to_notify` notification will be fired only if `get()` has been
    /// called since the last update)
    ///
    pub fn new(
        get_value: TValueFn,
        to_notify: Arc<dyn Notifiable>,
    ) -> (NotifyWatcher<TValueFn, TValue>, ReleasableNotifiable) {
        // Initially the value is 'updated' (ie, we won't fire the event until the first call to `get()`)
        let value_updated = Arc::new(Mutex::new(true));

        // Callback to be called on every change
        let callback_updated = Arc::clone(&value_updated);
        let on_change = move || {
            let should_notify = {
                let mut updated = callback_updated.lock().unwrap();

                if !*updated {
                    // If not previously updated since the last read, then mark as 'updated' and notify
                    *updated = true;
                    true
                } else {
                    // Don't notify if the value hasn't been read since the last notification
                    false
                }
            };

            if should_notify {
                to_notify.mark_as_changed();
            }
        };

        let on_change = ReleasableNotifiable::new(notify(on_change));
        let when_changed = on_change.clone_for_inspection();

        // Create the watcher
        let watcher = NotifyWatcher {
            get_value: get_value,
            value_updated: value_updated,
            notification: on_change,
        };

        (watcher, when_changed)
    }
}
