/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::buffer::*;

use gl;

use std::mem;

impl Vertex2D {
    ///
    /// Defines the attributes for this structure onto whatever vertex array object is currently bound
    ///
    pub fn define_attributes() {
        unsafe {
            // Define the attributes
            let stride = mem::size_of::<Self>() as gl::types::GLint;
            let pos = 0;

            // Attribute 0: a_Pos
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(
                0,
                2,
                gl::FLOAT,
                gl::FALSE,
                stride,
                pos as *const gl::types::GLvoid,
            );

            let pos = pos + 2 * mem::size_of::<f32>();

            // Attribute 1: a_TexCoord
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(
                1,
                2,
                gl::FLOAT,
                gl::FALSE,
                stride,
                pos as *const gl::types::GLvoid,
            );

            let pos = pos + 2 * mem::size_of::<f32>();

            // Attribute 2: a_Color
            gl::EnableVertexAttribArray(2);
            gl::VertexAttribPointer(
                2,
                4,
                gl::UNSIGNED_BYTE,
                gl::FALSE,
                stride,
                pos as *const gl::types::GLvoid,
            );
        }
    }
}
