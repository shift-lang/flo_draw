/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use gl;

use std::ffi::CString;
use std::ops::Deref;

///
/// The types of shader that we can create
///
pub enum GlShaderType {
    Vertex,
    Fragment,
}

///
/// Represents an OpenGL shader resource
///
pub struct Shader {
    shader: gl::types::GLuint,

    attributes: Vec<CString>,
}

impl Shader {
    ///
    /// Compiles a shader program with a set of defines
    ///
    pub fn compile_with_defines(
        program: &str,
        attributes: &Vec<&str>,
        shader_type: GlShaderType,
        defines: &Vec<&str>,
    ) -> Shader {
        let program = format!(
            "{}\n\n{}\n{}\n",
            "#version 330 core",
            defines
                .iter()
                .map(|defn| format!("#define {}\n", defn))
                .collect::<Vec<_>>()
                .join(""),
            program
        );

        Self::compile(&program, shader_type, attributes.iter().map(|s| *s))
    }

    ///
    /// Compiles a shader program
    ///
    pub fn compile<'a, AttributeIter: IntoIterator<Item = &'a str>>(
        program: &str,
        shader_type: GlShaderType,
        attributes: AttributeIter,
    ) -> Shader {
        unsafe {
            // Create the shader
            let shader_type = match shader_type {
                GlShaderType::Vertex => gl::VERTEX_SHADER,
                GlShaderType::Fragment => gl::FRAGMENT_SHADER,
            };

            let shader = gl::CreateShader(shader_type);

            // Load the source and compile the shader
            let mut length = [program.len() as i32];
            let program = program.as_bytes().as_ptr() as *const gl::types::GLchar;
            let program = [program];

            gl::ShaderSource(shader, 1, &program[0], &mut length[0]);
            gl::CompileShader(shader);

            // Check that the shader compiled
            let mut is_compiled = 0;
            gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut is_compiled);

            if is_compiled == gl::FALSE.into() {
                // Fetch the logs for this shader
                let mut logs: Vec<gl::types::GLchar> = vec![0; 8192];
                let mut len = 0;
                gl::GetShaderInfoLog(shader, 8192, &mut len, logs.as_mut_ptr());

                // Convert to a string (despite gl using i8s we can just read them as u8s...)
                let len = len as usize;
                let logs = logs[0..len]
                    .into_iter()
                    .map(|c| *c as u8)
                    .collect::<Vec<_>>();
                let logs = String::from_utf8_lossy(&logs);

                println!("=== Shader errors\n {}\n===", logs);
                panic!("Could not compile shader");
            }

            // Store the attributes as C-Strings
            let attributes = attributes
                .into_iter()
                .map(|attr| CString::new(attr).unwrap())
                .collect();

            Shader {
                shader: shader,
                attributes: attributes,
            }
        }
    }

    ///
    /// Retrieves the attributes for this shader
    ///
    pub(super) fn attributes(&self) -> &Vec<CString> {
        &self.attributes
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteShader(self.shader);
        }
    }
}

impl Deref for Shader {
    type Target = gl::types::GLuint;

    fn deref(&self) -> &gl::types::GLuint {
        &self.shader
    }
}
