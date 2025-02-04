/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

pub use ::egl::*;

#[link(name = "EGL")]
extern "C" {}

//
// Constants missing from the EGL crate
//

pub const EGL_PLATFORM_GBM_KHR: EGLenum = 0x31D7;
pub const EGL_PLATFORM_WAYLAND_KHR: EGLenum = 0x31D8;
pub const EGL_PLATFORM_X11_KHR: EGLenum = 0x31D5;
pub const EGL_PLATFORM_X11_SCREEN_KHR: EGLenum = 0x31D6;
pub const EGL_PLATFORM_DEVICE_EXT: EGLenum = 0x313F;
pub const EGL_PLATFORM_WAYLAND_EXT: EGLenum = 0x31D8;
pub const EGL_PLATFORM_X11_EXT: EGLenum = 0x31D5;
pub const EGL_PLATFORM_X11_SCREEN_EXT: EGLenum = 0x31D6;
pub const EGL_PLATFORM_GBM_MESA: EGLenum = 0x31D7;
pub const EGL_PLATFORM_SURFACELESS_MESA: EGLenum = 0x31DD;
pub const EGL_CONTEXT_MAJOR_VERSION: EGLint = 0x3098;
pub const EGL_CONTEXT_MINOR_VERSION: EGLint = 0x30FB;

//
// FFI functions missing from the EGL crate
//

pub mod ffi {
    use ::egl::*;
    use std::ffi::c_void;

    extern "C" {
        pub fn eglGetPlatformDisplay(
            platform: EGLenum,
            native_display: *mut c_void,
            attributes: *const EGLint,
        ) -> EGLDisplay;
    }
}
