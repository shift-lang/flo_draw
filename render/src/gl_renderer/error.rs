/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use gl;

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum GlError {
    /// Error without a string translation
    UnknownError(u32),

    InvalidOperation,
    InvalidEnum,

    /// Error where we can provide a string version
    #[allow(dead_code)]
    Error(u32, String),
}

///
/// Collects OpenGL errors and panics if there are any
///
#[cfg(debug_assertions)]
pub fn panic_on_gl_error(context: &str) {
    let errors = check_for_gl_errors();

    if errors.len() > 0 {
        panic!("{}: Unexpected OpenGL errors: {:?}", context, errors);
    }
}

///
/// Collects OpenGL errors and panics if there are any
///
#[cfg(not(debug_assertions))]
pub fn panic_on_gl_error(context: &str) {
    let errors = check_for_gl_errors();

    if errors.len() > 0 {
        println!("{}: Unexpected OpenGL errors: {:?}", context, errors);
    }
}

///
/// Returns all errors that are currently set in a GL context
///
pub fn check_for_gl_errors() -> Vec<GlError> {
    let mut result = vec![];

    // Read all of ther errors that are set in the current context
    while let Some(error) = check_next_gl_error() {
        result.push(error)
    }

    result
}

///
/// Returns the next GL error
///
fn check_next_gl_error() -> Option<GlError> {
    let error = unsafe { gl::GetError() };

    match error {
        gl::NO_ERROR => None,
        gl::INVALID_OPERATION => Some(GlError::InvalidOperation),
        gl::INVALID_ENUM => Some(GlError::InvalidEnum),
        unknown => Some(GlError::UnknownError(unknown)),
    }
}
