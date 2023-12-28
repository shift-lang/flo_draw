/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::sync::*;

use super::traits::*;

struct NotifyFn<TFn> {
    when_changed: Mutex<TFn>,
}

impl<TFn> Notifiable for NotifyFn<TFn>
where
    TFn: Send + FnMut() -> (),
{
    fn mark_as_changed(&self) {
        let on_changed = &mut *self.when_changed.lock().unwrap();

        on_changed()
    }
}

///
/// Creates a notifiable reference from a function
///
pub fn notify<TFn>(when_changed: TFn) -> Arc<dyn Notifiable>
where
    TFn: 'static + Send + FnMut() -> (),
{
    Arc::new(NotifyFn {
        when_changed: Mutex::new(when_changed),
    })
}
