use std::{ffi::{CStr, CString}, mem, num::NonZeroU32, os::raw::c_void, ptr};

use gl::types::{GLsizeiptr, GLuint};
use glutin::{config::{ConfigTemplateBuilder, GlConfig}, context::{ContextApi, ContextAttributesBuilder, GlProfile, NotCurrentGlContext, PossiblyCurrentContext, Version}, 
    display::{GetGlDisplay, GlDisplay}, surface::{GlSurface, Surface, WindowSurface}};
use glutin_winit::{DisplayBuilder, GlWindow};
use raw_window_handle::HasRawWindowHandle;
use winit::{event_loop::EventLoopWindowTarget, window::{Window, WindowBuilder}};

use self::gl_shader::GlShader;

use super::{RenderResult, Renderer, UniformData};

mod gl_shader;

pub struct GLRenderer {
    gl_surface: Surface<WindowSurface>,
    gl_context: PossiblyCurrentContext,
    vertex_array_object: GLuint,
    vertex_buffer_object: GLuint,
    element_buffer_object: GLuint,
    maze_textures: Vec<GLuint>,
    maze_shader: GlShader
}

impl Renderer for GLRenderer {
    fn init_mesh(&mut self, vertex_buffer: Vec<f32>, index_buffer: Vec<u32>) {
        unsafe {
            //VAO
            gl::GenVertexArrays(1, &mut self.vertex_array_object);
            gl::BindVertexArray(self.vertex_array_object);

            //VBO
            gl::GenBuffers(1, &mut self.vertex_buffer_object);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vertex_buffer_object);
            gl::BufferData(gl::ARRAY_BUFFER, (vertex_buffer.len()*mem::size_of::<f32>()) as GLsizeiptr,
                        vertex_buffer.as_ptr() as *const gl::types::GLvoid, gl::STATIC_DRAW);
        
            //VBO Position
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, 8 * mem::size_of::<f32>() as i32, ptr::null());

            //VBO Texture UV
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, 8 * mem::size_of::<f32>() as i32, 
                            (3 * std::mem::size_of::<f32>()) as *const gl::types::GLvoid);

            //VBO Normal vector
            gl::EnableVertexAttribArray(2);
            gl::VertexAttribPointer(2, 3, gl::FLOAT, gl::FALSE, 8 * mem::size_of::<f32>() as i32, 
                            (5 * std::mem::size_of::<f32>()) as *const gl::types::GLvoid);

            //EBO
            gl::GenBuffers(1, &mut self.element_buffer_object);
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.element_buffer_object);
            gl::BufferData(gl::ELEMENT_ARRAY_BUFFER, (index_buffer.len()*mem::size_of::<u32>()) as GLsizeiptr,
                        index_buffer.as_ptr() as *const gl::types::GLvoid, gl::STATIC_DRAW);
        }
    }

    fn load_textures(&mut self, textures_paths: Vec<String>) {
        for texture_path in textures_paths {
            unsafe {
                let mut texture_id: GLuint = 0;

                gl::GenTextures(1, &mut texture_id);
                self.load_texture(texture_id, &texture_path);

                self.maze_textures.push(texture_id);
            }
        }
    }

    fn load_shaders(&mut self, vertex_shader_path: &str, fragment_shader_path: &str) {
        self.maze_shader.load_shaders(vertex_shader_path, fragment_shader_path).unwrap();
    }

    fn update_uniform_data(&mut self, uniform_data: UniformData) {
        self.maze_shader.use_shader();

        self.maze_shader.set_uniform_matrix4fv("view", uniform_data.view_matrix);
        self.maze_shader.set_uniform_matrix4fv("projection", uniform_data.projection_matrix);

        self.maze_shader.set_uniform_vec3fv("lightColor", uniform_data.light_color);
        self.maze_shader.set_uniform_vec3fv("lightVector", uniform_data.light_position);

        unsafe {
            gl::BindVertexArray(self.vertex_array_object);
        }
    }

    fn draw(&mut self, model_matrix: glm::Mat4, texture_index: i32) {
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.maze_textures[texture_index as usize]);

            self.maze_shader.set_uniform_matrix4fv("model", model_matrix);

            gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, 0 as *const _);
        }
    }

    fn clear_color(&mut self, color: [f32; 4]) {
        unsafe {
            gl::ClearColor(color[0], color[1], color[2], color[3]);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }
    }

    fn render(&mut self) -> RenderResult {
        self.gl_surface.swap_buffers(&self.gl_context).unwrap();

        RenderResult::RenderFinished
    }

    fn resize_viewport(&mut self, window_width: u32, window_height: u32) {
        unsafe {
            gl::Viewport(0, 0, window_width as i32, window_height as i32);
        }

        self.gl_surface.resize(&self.gl_context, NonZeroU32::new(window_width).unwrap(), NonZeroU32::new(window_height).unwrap());
    }

    fn cleanup(&mut self) {
        self.maze_shader.delete_program();

        unsafe {
            gl::DeleteBuffers(1, &mut self.vertex_buffer_object);
            gl::DeleteBuffers(1, &mut self.element_buffer_object);
            gl::DeleteVertexArrays(1, &mut self.vertex_array_object);

            for texture in self.maze_textures.iter_mut() {
                gl::DeleteTextures(1, texture);
            }
        }
    }
}

impl GLRenderer {
    pub fn new<T>(window_builder: WindowBuilder, window_target: &EventLoopWindowTarget<T>, vsync_enabled: bool) -> (Self, Window) {
        let display_builder = DisplayBuilder::new().with_window_builder(Some(window_builder));

        let (window, gl_config) = display_builder.build(window_target, ConfigTemplateBuilder::new(), |configs| {
            configs
                .reduce(|accum, config| {
                    if config.num_samples() == 4 {
                        config
                    } else {
                        accum
                    }
                })
                .unwrap()
        }).unwrap();

        let gl_display = gl_config.display();
        let raw_window_handle = window.as_ref().map(|window| window.raw_window_handle());
        let window = window.unwrap();
        let attrs = window.build_surface_attributes(Default::default());

        let gl_surface = unsafe {
            gl_display.create_window_surface(&gl_config, &attrs).unwrap()
        };

        let context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
            .with_profile(GlProfile::Core)
            .build(raw_window_handle);

        let gl_context = unsafe {
            gl_display.create_context(&gl_config, &context_attributes).expect("Failed to create OpenGL context.").make_current(&gl_surface).unwrap()
        };

        match vsync_enabled {
            false => gl_surface.set_swap_interval(&gl_context, glutin::surface::SwapInterval::DontWait).unwrap(),
            true => gl_surface.set_swap_interval(&gl_context, glutin::surface::SwapInterval::Wait(NonZeroU32::new(1).unwrap())).unwrap()
        }

        gl::load_with(|symbol| {
            let symbol = CString::new(symbol).unwrap();
            gl_display.get_proc_address(symbol.as_c_str()).cast()
        });

        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::Enable(gl::CULL_FACE);
            gl::Enable(gl::FRAMEBUFFER_SRGB);
        }

        println!("OpenGL initialized.");

        unsafe {
            let vendor = gl::GetString(gl::VENDOR) as *const i8;
            let vendor = String::from_utf8(CStr::from_ptr(vendor).to_bytes().to_vec()).unwrap();

            let renderer = gl::GetString(gl::RENDERER) as *const i8;
            let renderer = String::from_utf8(CStr::from_ptr(renderer).to_bytes().to_vec()).unwrap();

            let version = gl::GetString(gl::VERSION) as *const i8;
        let version = String::from_utf8(CStr::from_ptr(version).to_bytes().to_vec()).unwrap();

            println!("Vendor: {}", vendor);
            println!("Renderer: {}", renderer);
            println!("Version: {}", version);
        }

        (Self {
            gl_surface, 
            gl_context,
            vertex_array_object: 0,
            vertex_buffer_object: 0,
            element_buffer_object: 0,
            maze_textures: Vec::new(),
            maze_shader: GlShader::new()
        }, window)
    }

    fn load_texture(&mut self, texture_id: GLuint, texture_file: &str) {
        let texture = image::open(texture_file).unwrap().into_rgba8();
        
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, texture_id);
    
            //Setup wrapping and filtering
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
    
            gl::TexImage2D(gl::TEXTURE_2D, 0, gl::SRGB_ALPHA as i32, texture.width() as i32, texture.height() as i32, 
                            0, gl::RGBA, gl::UNSIGNED_BYTE, texture.into_raw().as_ptr() as *const c_void);
    
            gl::GenerateMipmap(gl::TEXTURE_2D);
        }    
    }
}
