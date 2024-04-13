//Data related to one mesh (like vertex buffer, index buffer etc.)
//Every mesh have its own data (model matrix and texture index) uploaded to shaders with push constants

use std::mem;

use ash::vk::{BufferUsageFlags, CommandPool};

use super::{vulkan_buffer::VulkanBuffer, vulkan_context::VulkanContext, vulkan_vertex_input::{VertexData, VertexInput}};

#[derive(Copy, Clone)]
pub struct PushConstant {
    pub model_matrix: glm::Mat4,
    pub texture_index: i32
}

pub struct VulkanMesh {
    pub vertex_buffer: Option<VulkanBuffer>,
    pub index_buffer: Option<VulkanBuffer>,
    pub vertex_indices: Vec<u32>,
    pub vertex_input: Option<VertexInput>,
    pub push_constant: PushConstant
}

impl VulkanMesh {
    pub fn new() -> Self {
        Self {
            vertex_buffer: None,
            index_buffer: None,
            vertex_indices: Vec::new(),
            vertex_input: None,
            push_constant: PushConstant {
                model_matrix: glm::Mat4::identity(),
                texture_index: 0
            }
        }
    }

    pub fn add_mesh_data(&mut self, mut vertex_data: Vec<VertexData>, vertex_indices: Vec<u32>, vulkan_context: &mut VulkanContext, command_pool: CommandPool) {
        if vertex_data.is_empty() {
            panic!("Attempted to create vertex buffer without data.");
        }

        let mut vertex_input = VertexInput::new();
        vertex_input.add_vertices(&mut vertex_data);

        let (vertex_buffer, index_buffer) = VulkanMesh::create_buffers(vulkan_context, &vertex_input, &vertex_indices, command_pool);

        self.vertex_buffer = Some(vertex_buffer);
        self.index_buffer = index_buffer;
        self.vertex_indices = vertex_indices;
        self.vertex_input = Some(vertex_input);
    }

    pub fn destroy_mesh(&mut self, vulkan_context: &mut VulkanContext) {
        let logical_device = &vulkan_context.logical_device;
        let allocator = &mut vulkan_context.allocator;

        unsafe {
            logical_device.device_wait_idle().unwrap();
        }

        let mut vertex_buffer = self.vertex_buffer.take().unwrap();
        vertex_buffer.free(logical_device, allocator);

        if self.index_buffer.is_some() {
            let mut index_buffer = self.index_buffer.take().unwrap();
            index_buffer.free(logical_device, allocator);
        }
    }

    pub fn set_mesh_data(&mut self, data: PushConstant) {
        self.push_constant = data;
    }

    fn create_buffers(vulkan_context: &mut VulkanContext, vertex_input: &VertexInput, vertex_indices: &Vec<u32>, command_pool: CommandPool) -> (VulkanBuffer, Option<VulkanBuffer>) {
        let logical_device = &vulkan_context.logical_device;
        let allocator = &mut vulkan_context.allocator;
        let present_queue = vulkan_context.present_queue;
        
        let mut staging_vertex_buffer = VulkanBuffer::new(logical_device, allocator, vertex_input.size() as u64, 
        BufferUsageFlags::VERTEX_BUFFER | BufferUsageFlags::TRANSFER_SRC, gpu_allocator::MemoryLocation::CpuToGpu, "Staging vertex buffer");

        unsafe {
            let vertex_buffer_memory = staging_vertex_buffer.memory.as_ptr();
            let vertex_input_data = &vertex_input.vertex_data[..];

            std::ptr::copy_nonoverlapping(vertex_input_data.as_ptr(), vertex_buffer_memory.cast(), vertex_input_data.len());
        }

        let vertex_buffer = VulkanBuffer::new(logical_device, allocator, vertex_input.size() as u64, 
        BufferUsageFlags::VERTEX_BUFFER | BufferUsageFlags::TRANSFER_DST, gpu_allocator::MemoryLocation::GpuOnly, "Vertex buffer");

        VulkanBuffer::copy_buffer(logical_device, command_pool, present_queue, &staging_vertex_buffer, &vertex_buffer);
        staging_vertex_buffer.free(logical_device, allocator);

        if !vertex_indices.is_empty() {
            let index_buffer_size = (mem::size_of::<u32>()) * vertex_indices.len();

            let mut staging_index_buffer = VulkanBuffer::new(logical_device, allocator, index_buffer_size as u64, 
                BufferUsageFlags::INDEX_BUFFER | BufferUsageFlags::TRANSFER_SRC, gpu_allocator::MemoryLocation::CpuToGpu, "Staging index buffer");

            unsafe {
                std::ptr::copy_nonoverlapping(vertex_indices[..].as_ptr(), staging_index_buffer.memory.as_ptr().cast(), vertex_indices.len());
            }

            let index_buffer = VulkanBuffer::new(logical_device, allocator, index_buffer_size as u64, 
           BufferUsageFlags::INDEX_BUFFER | BufferUsageFlags::TRANSFER_DST, gpu_allocator::MemoryLocation::GpuOnly, "Index buffer");

            VulkanBuffer::copy_buffer(logical_device, command_pool, present_queue, &staging_index_buffer, &index_buffer);
            staging_index_buffer.free(logical_device, allocator);

            return (vertex_buffer, Some(index_buffer));
        }
        else {
            return (vertex_buffer, None);
        }
    }
}
