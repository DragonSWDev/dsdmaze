extern crate gl;

use gl::types::*;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::ffi::CString;
use std::ptr;

pub struct GlShader {
    program_id: GLuint,     
}

impl GlShader {
    pub fn new() -> GlShader {
        GlShader {
            program_id: 0,
        }
    }

    pub fn load_shaders(&mut self, vertex_shader_path: &str, fragment_shader_path: &str) -> Result<(), Box<dyn Error>> {
        let mut vertex_shader_source = String::new();
        let mut fragment_shader_source = String::new();

        let mut vertex_shader_file = File::open(vertex_shader_path).unwrap();
        let mut fragment_shader_file = File::open(fragment_shader_path).unwrap();

        vertex_shader_file.read_to_string(&mut vertex_shader_source).unwrap();
        fragment_shader_file.read_to_string(&mut fragment_shader_source).unwrap();
        
        let vertex_shader: GLuint;
        let fragment_shader: GLuint;

        unsafe {
            vertex_shader = gl::CreateShader(gl::VERTEX_SHADER);
            let vertex_shader_source = CString::new(vertex_shader_source).unwrap();
            gl::ShaderSource(vertex_shader, 1, &vertex_shader_source.as_ptr(), ptr::null());
            gl::CompileShader(vertex_shader);

            let mut status = gl::FALSE as GLint;
            gl::GetShaderiv(vertex_shader, gl::COMPILE_STATUS, &mut status);

            if status != (gl::TRUE as GLint) {
                Err("Vertex shader compilation failed.")?;
            }

            fragment_shader = gl::CreateShader(gl::FRAGMENT_SHADER);
            let fragment_shader_source = CString::new(fragment_shader_source).unwrap();
            gl::ShaderSource(fragment_shader, 1, &fragment_shader_source.as_ptr(), ptr::null());
            gl::CompileShader(fragment_shader);

            gl::GetShaderiv(fragment_shader, gl::COMPILE_STATUS, &mut status);

            if status != (gl::TRUE as GLint) {
                Err("Fragment shader compilation failed.")?;
            }

            self.program_id = gl::CreateProgram();
            gl::AttachShader(self.program_id, vertex_shader);
            gl::AttachShader(self.program_id, fragment_shader);
            gl::LinkProgram(self.program_id);

            gl::GetProgramiv(self.program_id, gl::LINK_STATUS, &mut status);

            if status != (gl::TRUE as GLint) {
                Err("Shader program link failed.")?;
            }

            gl::DeleteShader(vertex_shader);
            gl::DeleteShader(fragment_shader);
        }
        
        Ok(())
    }

    pub fn use_shader(&mut self) {
        unsafe {
            gl::UseProgram(self.program_id);
        }
    }

    pub fn set_uniform_matrix4fv(&mut self, name: &str, uniform: glm::Mat4) {
        unsafe {
            let uniform_name = CString::new(name).unwrap();
            let location = gl::GetUniformLocation(self.program_id, uniform_name.as_ptr());
            gl::UniformMatrix4fv(location, 1, gl::FALSE, uniform.as_ptr());
        }
    }

    pub fn set_uniform_vec3fv(&mut self, name: &str, uniform: glm::Vec3) {
        unsafe {
            let uniform_name = CString::new(name).unwrap();
            let location = gl::GetUniformLocation(self.program_id, uniform_name.as_ptr());
            gl::Uniform3fv(location, 1, uniform.as_ptr());
        }
    }

    pub fn delete_program(&mut self) {
        unsafe {
            gl::DeleteProgram(self.program_id);
        }
    }
}

