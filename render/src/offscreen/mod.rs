/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod error;
mod offscreen_trait;

#[cfg(feature = "osx-metal")]
mod metal;
#[cfg(feature = "opengl")]
mod opengl;
#[cfg(all(feature = "opengl", target_os = "macos"))]
mod opengl_cgl_init;
#[cfg(all(feature = "opengl", target_os = "linux"))]
mod opengl_egl_init;
#[cfg(all(feature = "opengl", target_os = "windows"))]
mod opengl_wgl_init;
#[cfg(feature = "render-wgpu")]
mod wgpu_offscreen;

pub use self::error::*;
pub use self::offscreen_trait::*;

#[cfg(feature = "osx-metal")]
pub use self::metal::*;
#[cfg(all(feature = "opengl", target_os = "macos"))]
pub use self::opengl_cgl_init::*;
#[cfg(all(feature = "opengl", target_os = "linux"))]
pub use self::opengl_egl_init::*;
#[cfg(all(feature = "opengl", target_os = "windows"))]
pub use self::opengl_wgl_init::*;
#[cfg(feature = "render-wgpu")]
pub use self::wgpu_offscreen::*;

#[cfg(test)]
mod test;
