/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::error::*;
use super::offscreen_trait::*;
use super::opengl::*;

use core_foundation::base::*;
use core_foundation::bundle::*;
use core_foundation::string::*;
use flo_render_gl_offscreen::cgl;
use gl;

use std::ptr;
use std::str;

///
/// An OpenGL offscreen rendering context initialised by CGL
///
struct CglOffscreenRenderContext {
    /// The pixel format used for the context
    _pixel_format: cgl::CGLPixelFormatObj,

    /// The CGL context itself
    context: cgl::CGLContextObj,
}

///
/// Finds the address of an OpenGL function
///
/// (Based on the similar function found in glutin)
///
fn get_proc_address(addr: &str) -> *const libc::c_void {
    let symbol_name: CFString = str::FromStr::from_str(addr).unwrap();
    let framework_name: CFString = str::FromStr::from_str("com.apple.opengl").unwrap();
    let framework =
        unsafe { CFBundleGetBundleWithIdentifier(framework_name.as_concrete_TypeRef()) };
    let symbol =
        unsafe { CFBundleGetFunctionPointerForName(framework, symbol_name.as_concrete_TypeRef()) };
    symbol as *const _
}

///
/// Converts a CGLError into a result
///
fn to_render_error(error: cgl::CGLError) -> Result<(), RenderInitError> {
    match error {
        cgl::kCGLNoError => Ok(()),
        _ => Err(RenderInitError::CannotStartGraphicsDriver),
    }
}

///
/// Performs on-startup initialisation steps for offscreen rendering
///
/// Only required if not using a toolkit renderer (eg, in an HTTP renderer or command-line tool). Will likely replace
/// the bindings for any GUI toolkit, so this is not appropriate for desktop-type apps.
///
/// This version is the CGL version for Mac OS X
///
pub fn opengl_initialize_offscreen_rendering(
) -> Result<impl OffscreenRenderContext, RenderInitError> {
    unsafe {
        // Try to select a pixel format
        let pixel_attributes = vec![
            cgl::kCGLPFAAccelerated,
            cgl::kCGLPFAOpenGLProfile,
            0x3200,
            cgl::kCGLPFAColorSize,
            24,
            cgl::kCGLPFADepthSize,
            16,
            0,
        ];
        let mut pixel_format = ptr::null_mut();
        let mut num_pixel_formats = 0;
        let pixel_format_error = cgl::CGLChoosePixelFormat(
            pixel_attributes.as_ptr(),
            &mut pixel_format,
            &mut num_pixel_formats,
        );
        to_render_error(pixel_format_error)?;

        if pixel_format.is_null() {
            Err(RenderInitError::DisplayNotAvailable)?
        }

        // Try to create a context from the pixel format we selected
        let mut context = ptr::null_mut();
        let context_error = cgl::CGLCreateContext(pixel_format, ptr::null_mut(), &mut context);
        to_render_error(context_error)?;

        if context.is_null() {
            Err(RenderInitError::CouldNotCreateContext)?
        }

        // Try to set this as the current thread's context
        let set_context_error = cgl::CGLSetCurrentContext(context);
        if set_context_error != 0 {
            Err(RenderInitError::ContextDidNotStart)?
        }

        // Load as the GL functions
        gl::load_with(|name| get_proc_address(name));

        // Check for errors
        let error = gl::GetError();
        if error != gl::NO_ERROR {
            println!("gl::GetError {:x}", error);
            Err(RenderInitError::ContextDidNotStart)?
        }
        assert!(error == gl::NO_ERROR);

        // Result is a CGL offscreen context
        Ok(CglOffscreenRenderContext {
            _pixel_format: pixel_format,
            context: context,
        })
    }
}

///
/// Performs on-startup initialisation steps for offscreen rendering
///
/// Only required if not using a toolkit renderer (eg, in an HTTP renderer or command-line tool). Will likely replace
/// the bindings for any GUI toolkit, so this is not appropriate for desktop-type apps.
///
/// This version is the Metal version for Mac OS X
///
#[cfg(not(feature = "osx-metal"))]
pub fn initialize_offscreen_rendering() -> Result<impl OffscreenRenderContext, RenderInitError> {
    opengl_initialize_offscreen_rendering()
}

impl OffscreenRenderContext for CglOffscreenRenderContext {
    type RenderTarget = OpenGlOffscreenRenderer;

    ///
    /// Creates a new render target for this context
    ///
    fn create_render_target(&mut self, width: usize, height: usize) -> Self::RenderTarget {
        unsafe {
            let set_context_error = cgl::CGLSetCurrentContext(self.context);
            if set_context_error != 0 {
                panic!("CGLSetCurrentContext {:x}", set_context_error);
            }

            OpenGlOffscreenRenderer::new(width, height)
        }
    }
}
