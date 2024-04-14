use std::fmt;

pub mod vulkan_renderer;
pub mod gl_renderer;

pub enum RenderingAPI {
    OPENGL,
    VULKAN
}

pub enum RenderResult {
    RenderFinished,
    VkOutOfDate
}

impl fmt::Display for RenderingAPI {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            RenderingAPI::OPENGL => write!(f, "OpenGL"),
            RenderingAPI::VULKAN => write!(f, "Vulkan")
        }
    }
}

#[derive(Copy, Clone)]
pub struct UniformData {
    pub view_matrix: glm::Mat4,
    pub projection_matrix: glm::Mat4,
    pub light_position: glm::Vec3,
    pub _padding: [u8; 4], //vec3 needs to be aligned for 16 bytes, since it's 12 bytes in size, additional 4 bytes are needed between
    pub light_color: glm::Vec3
}

pub trait Renderer {
    fn init_mesh(&mut self, vertex_buffer: Vec<f32>, index_buffer: Vec<u32>);

    fn load_textures(&mut self, textures_paths: Vec<String>);

    fn load_shaders(&mut self, vertex_shader_path: &str, fragment_shader_path: &str);

    fn update_uniform_data(&mut self, uniform_data: UniformData);

    fn draw(&mut self, model_matrix: glm::Mat4, texture_index: i32);

    fn clear_color(&mut self, color: [f32; 4]);

    fn render(&mut self) -> RenderResult;

    fn resize_viewport(&mut self, window_width: u32, window_height: u32);

    fn cleanup(&mut self);
}

pub struct MazeRenderer {
    pub renderer: Box<dyn Renderer>
}

impl MazeRenderer {
    pub fn new(renderer: Box<dyn Renderer>) -> Self {
        Self {
            renderer
        }
    }
}
