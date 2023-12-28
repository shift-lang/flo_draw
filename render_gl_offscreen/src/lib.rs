/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#[cfg(target_os = "linux")]
pub mod egl;
#[cfg(target_os = "linux")]
pub mod gbm;

#[cfg(target_os = "macos")]
pub mod cgl {
    pub use ::cgl::*;
}

#[cfg(target_os = "windows")]
pub mod wgl {
    pub use ::glutin_wgl_sys::*;
}

#[cfg(target_os = "windows")]
pub mod winapi {
    pub use ::winapi::*;
}
