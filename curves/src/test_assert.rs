/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#[cfg(not(any(test, extra_checks)))]
macro_rules! test_assert {
    ($cond:expr) => {{}};
    ($cond:expr,) => {{}};
    ($cond:expr, $($arg:tt)+) => {{}};
}

#[cfg(any(test, extra_checks))]
macro_rules! test_assert {
    ($cond:expr) => ({ assert!($cond); });
    ($cond:expr,) => ({ assert!($cond); });
    ($cond:expr, $($arg:tt)+) => ({ assert!($cond, $($arg)*); });
}
