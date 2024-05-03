//Main rendering code
//Responsible for initialization needed Vulkan objects (command pool, comand buffer, render pass etc.) and drawing

use std::{fs::File, mem::{self, size_of}};

use ash::{util::read_spv, vk::{self, AttachmentDescription, AttachmentDescriptionFlags, AttachmentLoadOp, AttachmentStoreOp, BorderColor, Buffer, BufferUsageFlags, CommandBuffer, CommandBufferAllocateInfo, 
    CommandBufferBeginInfo, CommandBufferLevel, CommandBufferResetFlags, CommandPool, CommandPoolCreateFlags, CommandPoolCreateInfo, CompareOp, DescriptorSet, DescriptorSetLayout, Fence, FenceCreateFlags, 
    FenceCreateInfo, Filter, Format, FormatFeatureFlags, Framebuffer, ImageAspectFlags, ImageLayout, ImageTiling, ImageUsageFlags, ImageView, IndexType, Pipeline, PipelineBindPoint, PipelineLayout, 
    PipelineLayoutCreateInfo, PipelineStageFlags, PresentInfoKHR, PrimitiveTopology, PushConstantRange, RenderPass, RenderPassBeginInfo, SampleCountFlags, Sampler, SamplerAddressMode, SamplerCreateInfo, 
    SamplerMipmapMode, Semaphore, SemaphoreCreateInfo, ShaderModule, ShaderModuleCreateInfo, ShaderStageFlags, SubmitInfo, SubpassContents}, Device, Entry};

use winit::window::Window;

use crate::maze_renderer::vulkan_renderer::{vulkan_buffer::VulkanBuffer, vulkan_vertex_input::VertexData};

use self::{vulkan_context::VulkanContext, vulkan_descriptor::VulkanDescriptor, vulkan_image::VulkanImage, vulkan_mesh::{PushConstant, VulkanMesh}, vulkan_pipeline::VulkanPipeline, vulkan_vertex_input::VertexInput};

use super::{RenderResult, Renderer, UniformData};

pub mod vulkan_context;
pub mod vulkan_pipeline;
pub mod vulkan_buffer;
pub mod vulkan_vertex_input;
pub mod vulkan_mesh;
pub mod vulkan_image;
pub mod vulkan_descriptor;

impl Renderer for VulkanRenderer {
    fn init_mesh(&mut self, vertex_buffer: Vec<f32>, index_buffer: Vec<u32>) {
        //Expect 8 components which is vertex position XYZ, vertex normal XYZ an texture UV
        if vertex_buffer.len() & 8 != 0 {
            panic!("Incorrect vertex data.");
        }

        let mut i = 0;
        let mut vertex_data = Vec::new();

        while i < vertex_buffer.len() {
            vertex_data.push(VertexData::new(glm::vec3(vertex_buffer[i], vertex_buffer[i + 1], vertex_buffer[i + 2]), 
                glm::vec3(vertex_buffer[i + 5], vertex_buffer[i + 6], vertex_buffer[i + 7]), 
                glm::vec2(vertex_buffer[i + 3], vertex_buffer[i + 4])));

            i += 8;
        }

        let mut maze_mesh = VulkanMesh::new();
        self.populate_vertex_buffer(&mut maze_mesh, vertex_data, index_buffer);
        self.maze_mesh = Some(maze_mesh);
    }

    fn load_textures(&mut self, textures_paths: Vec<String>) {
        let mut maze_textures = Vec::new();

        let mut texture_index = 0;
        for texture_path in textures_paths.iter() {
            let texture_name = "Maze texture ".to_owned() + texture_index.to_string().as_str();
            texture_index += 1;

            maze_textures.push(self.create_texture(texture_path, texture_name.as_str(), true));
        }

        let sampler = self.create_sampler(Filter::LINEAR, SamplerAddressMode::REPEAT, SamplerMipmapMode::LINEAR, 0.0, 15.0);

        self.maze_texture_sampler = Some(sampler);
        self.maze_textures = Some(maze_textures);
    }

    fn load_shaders(&mut self, vertex_shader_path: &str, fragment_shader_path: &str) {
        let maze_textures = self.maze_textures.take().unwrap();

        let mut maze_textures_ref = Vec::new();

        for maze_texture in maze_textures.iter().as_ref() {
            maze_textures_ref.push(maze_texture.image_view.clone());
        }

        let maze_descriptors = self.create_descriptor(mem::size_of::<UniformData>() as u64, "Maze uniform data", maze_textures_ref, self.maze_texture_sampler);

        let maze_pipeline = self.create_pipeline(vertex_shader_path, fragment_shader_path, Some(&maze_descriptors));

        self.maze_descriptors = Some(maze_descriptors);
        self.maze_pipeline = Some(maze_pipeline);
        self.maze_textures = Some(maze_textures);
    }

    fn update_uniform_data(&mut self, uniform_data: UniformData) {
        let maze_descriptors = self.maze_descriptors.take().unwrap();
        let uniform_buffers = maze_descriptors.get_uniform_buffers_memory();

        unsafe {
            for n in uniform_buffers.iter() {
                let uniform_buffer_memory = n.as_ptr();
                let uniform_buffer_data = &[uniform_data];

                std::ptr::copy_nonoverlapping(uniform_buffer_data, uniform_buffer_memory.cast(), uniform_buffer_data.len());
            }
        }

        self.maze_descriptors = Some(maze_descriptors);
    }

    fn draw(&mut self, model_matrix: glm::Mat4, texture_index: i32) {
        let mut maze_mesh = self.maze_mesh.take().unwrap();
        let mut maze_pipeline = self.maze_pipeline.take().unwrap();

        maze_mesh.set_mesh_data(PushConstant {model_matrix, texture_index});
        self.draw_mesh(&mut maze_mesh, &mut maze_pipeline);

        self.maze_mesh = Some(maze_mesh);
        self.maze_pipeline = Some(maze_pipeline);
    }

    fn clear_color(&mut self, color: [f32; 4]) {
        self.clear_color(color);
    }

    fn render(&mut self) -> RenderResult {
        self.render()
    }

    fn resize_viewport(&mut self, window_width: u32, window_height: u32) {
        self.resize_viewport(window_width, window_height);
    }

    fn cleanup(&mut self) {
        unsafe {
            self.vulkan_context.logical_device.device_wait_idle().unwrap();
        }

        let mut maze_mesh = self.maze_mesh.take().unwrap();
        let mut maze_pipeline = self.maze_pipeline.take().unwrap();
        let mut maze_descriptors = self.maze_descriptors.take().unwrap();
        let mut maze_textures = self.maze_textures.take().unwrap();

        self.destroy_sampler(self.maze_texture_sampler.unwrap());

        for maze_texture in maze_textures.iter_mut() {
            self.destroy_texture(maze_texture);
        }

        self.destroy_mesh(&mut maze_mesh);
        self.destroy_descriptor(&mut maze_descriptors);
        self.destroy_pipeline(&mut maze_pipeline);
    }
}

const MAX_FRAMES_IN_FLIGHT: usize = 2;
const SAMPLE_COUNT: SampleCountFlags = SampleCountFlags::TYPE_4;

//Per frame data
struct FrameData {
    pub command_buffer: CommandBuffer,
    pub image_available_semaphore: Semaphore,
    pub render_finished_semaphore: Semaphore,
    pub in_flight_fence: Fence,
}

//Details of one mesh to render copied from VulkanMesh structure
struct RenderableMesh {
    vertex_buffer: Buffer,
    index_buffer: Option<Buffer>,
    vertices_count: u32,
    indices_count: u32,
    push_constants: PushConstant,
    pipeline_layout: PipelineLayout,
    graphics_pipeline: Pipeline,
    descriptor_sets: Vec<DescriptorSet>
}

//Graphics pipeline and related objects. Each mesh can be rendered with different pipeline.
pub struct RenderPipeline {
    pipeline_layout: PipelineLayout,
    graphics_pipeline: Pipeline,
    vertex_shader: ShaderModule,
    fragment_shader: ShaderModule,
    descriptor_sets: Vec<DescriptorSet>
}

pub struct VulkanRenderer {
    _vulkan_entry: Entry,
    vulkan_context: VulkanContext,
    color_image: VulkanImage,
    depth_image: VulkanImage,
    render_pass: RenderPass,
    framebuffers: Vec<Framebuffer>,
    command_pool: CommandPool,
    frame_data: Vec<FrameData>,
    current_frame: usize,
    clear_color: [f32; 4],
    meshes_to_draw: Vec<RenderableMesh>,

    maze_mesh: Option<VulkanMesh>,
    maze_textures: Option<Vec<VulkanImage>>,
    maze_texture_sampler: Option<Sampler>,
    maze_descriptors: Option<VulkanDescriptor>,
    maze_pipeline: Option<RenderPipeline>
}

impl VulkanRenderer {
    pub fn new(window: &Window, vsync_enabled: bool) -> Self {
        let _vulkan_entry = Entry::linked();
        let mut vulkan_context = VulkanContext::new(window, &_vulkan_entry, vsync_enabled);

        let supported_sample_count = vulkan_context.get_physical_device_properties().limits.framebuffer_color_sample_counts;

        if (SAMPLE_COUNT & supported_sample_count).is_empty() {
            panic!("Unsupported sample count.");
        }

        let color_image = VulkanImage::new(&vulkan_context.logical_device, &mut vulkan_context.allocator, "Color image", vulkan_context.surface_resolution.width, 
            vulkan_context.surface_resolution.height, vulkan_context.surface_format.format, ImageTiling::OPTIMAL, ImageUsageFlags::TRANSIENT_ATTACHMENT | ImageUsageFlags::COLOR_ATTACHMENT, 
            ImageAspectFlags::COLOR, false, SAMPLE_COUNT);

        let depth_image = VulkanImage::new(&vulkan_context.logical_device, &mut vulkan_context.allocator, "Depth buffer", vulkan_context.surface_resolution.width, 
            vulkan_context.surface_resolution.height, Format::D32_SFLOAT, ImageTiling::OPTIMAL, ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT, 
            ImageAspectFlags::DEPTH, false, SAMPLE_COUNT);

        let render_pass = Self::create_render_pass(vulkan_context.surface_format.format, &vulkan_context.logical_device, &depth_image);

        let framebuffers: Vec<vk::Framebuffer> = vulkan_context
            .swapchain_image_views
            .iter()
            .map(|&swapchain_image_view| {
                let framebuffer_attachments = [color_image.image_view, depth_image.image_view, swapchain_image_view];
                let frame_buffer_create_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(render_pass)
                    .attachments(&framebuffer_attachments)
                    .width(vulkan_context.surface_resolution.width)
                    .height(vulkan_context.surface_resolution.height)
                    .layers(1);

                unsafe {
                    vulkan_context.logical_device
                        .create_framebuffer(&frame_buffer_create_info, None)
                        .unwrap()
                }
            })
            .collect();

        let (command_pool, command_buffers) = Self::create_commands(&vulkan_context.logical_device, vulkan_context.queue_family_index, MAX_FRAMES_IN_FLIGHT as u32);

        let mut frame_data: Vec<FrameData> = Vec::new();

        for n in 0..MAX_FRAMES_IN_FLIGHT {
            let image_available_semaphore = unsafe {
                vulkan_context.logical_device.create_semaphore(&SemaphoreCreateInfo::default(), None).expect("Creating semaphore failed.")
            };
    
            let render_finished_semaphore = unsafe {
                vulkan_context.logical_device.create_semaphore(&SemaphoreCreateInfo::default(), None).expect("Creating semaphore failed.")
            };
    
            let in_flight_fence = unsafe {
                vulkan_context.logical_device.create_fence(&FenceCreateInfo::builder().flags(FenceCreateFlags::SIGNALED), None).expect("Creating fence failed.")
            };

            let data = FrameData {
                command_buffer: command_buffers[n],
                image_available_semaphore,
                render_finished_semaphore,
                in_flight_fence,
            };

            frame_data.push(data);
        }

        println!("Vulkan renderer initialized.");

        unsafe {
            let device_name = vulkan_context.instance.get_physical_device_properties(vulkan_context.physical_device).device_name.iter().map(|&c| c as u8).collect();

            println!("Selected device: {}", String::from_utf8(device_name).unwrap());
        }
        
        Self {
            _vulkan_entry,
            vulkan_context,
            color_image,
            depth_image,
            render_pass,
            framebuffers,
            command_pool,
            frame_data,
            current_frame: 0,
            clear_color: [0.0, 0.0, 0.0, 1.0],
            meshes_to_draw: Vec::new(),

            maze_mesh: None,
            maze_textures: None,
            maze_texture_sampler: None,
            maze_descriptors: None,
            maze_pipeline: None
        }
    }

    pub fn render(&mut self) -> RenderResult {
        unsafe {
            let logical_device = &self.vulkan_context.logical_device;
            let swapchain_loader = &self.vulkan_context.swapchain_loader;
            let command_buffer = self.frame_data[self.current_frame].command_buffer;
            let in_flight_fence = self.frame_data[self.current_frame].in_flight_fence;
            let image_available_semaphore = self.frame_data[self.current_frame].image_available_semaphore;
            let render_finished_semaphore = self.frame_data[self.current_frame].render_finished_semaphore;

            logical_device.wait_for_fences(&[in_flight_fence], true, u64::MAX).unwrap();
            logical_device.reset_fences(&[in_flight_fence]).unwrap();

            let image_index = match swapchain_loader.acquire_next_image(self.vulkan_context.swapchain_khr, u64::MAX, image_available_semaphore, Fence::null()) {
                Ok((image_index, _)) => image_index,
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => return RenderResult::VkOutOfDate,
                Err(error) => panic!("Acquiring next image failed with error: {}", error)
            };

            logical_device.reset_command_buffer(command_buffer, CommandBufferResetFlags::empty()).unwrap();

            logical_device.begin_command_buffer(command_buffer, &CommandBufferBeginInfo::default()).expect("Command buffer record failed.");

            let clear_values = &[
                vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: self.clear_color,
                    },
                },
                vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0
                    }
                }
                ];

            let render_pass_begin_info = RenderPassBeginInfo::builder()
                .render_pass(self.render_pass)
                .framebuffer(self.framebuffers[image_index as usize])
                .render_area(self.vulkan_context.surface_resolution.into())
                .clear_values(clear_values)
                .build();

            logical_device.cmd_begin_render_pass(command_buffer, &render_pass_begin_info, SubpassContents::INLINE);

            let viewports = [vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: self.vulkan_context.surface_resolution.width as f32,
                height: self.vulkan_context.surface_resolution.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            }];

            let scissors = [self.vulkan_context.surface_resolution.into()];

            logical_device.cmd_set_viewport(command_buffer, 0, &viewports);
            logical_device.cmd_set_scissor(command_buffer, 0, &scissors);

            //Store last used pipelines, descriptor sets and buffers to avoid binding same thing every time
            let mut last_pipeline = Pipeline::null();
            let mut last_descriptor_set = DescriptorSet::null();
            let mut last_vertex_buffer = Buffer::null();
            let mut last_index_buffer = Buffer::null();

            for mesh in self.meshes_to_draw.iter() {
                let pipeline = mesh.graphics_pipeline;
                let pipeline_layout = mesh.pipeline_layout;
                let vertex_buffer = mesh.vertex_buffer;

                if pipeline != last_pipeline {
                    logical_device.cmd_bind_pipeline(command_buffer, PipelineBindPoint::GRAPHICS, pipeline);
                }

                if !mesh.descriptor_sets.is_empty() {
                    let descriptor_set = mesh.descriptor_sets[self.current_frame];

                    if last_descriptor_set != descriptor_set {
                        logical_device.cmd_bind_descriptor_sets(command_buffer, PipelineBindPoint::GRAPHICS, pipeline_layout, 0, 
                            &[descriptor_set], &[]);
                    }

                    last_descriptor_set = descriptor_set;
                }

                if vertex_buffer != last_vertex_buffer {
                    logical_device.cmd_bind_vertex_buffers(command_buffer, 0, &[vertex_buffer], &[0]);
                }

                let push_constant_bytes = std::slice::from_raw_parts(
                    &mesh.push_constants as *const PushConstant as *const u8,
                    size_of::<PushConstant>()
                );

                logical_device.cmd_push_constants(command_buffer, pipeline_layout, ShaderStageFlags::VERTEX, 0, push_constant_bytes);

                //Index buffer is available, draw indexed
                if mesh.index_buffer.is_some() {
                    let index_buffer = mesh.index_buffer.unwrap();

                    if index_buffer != last_index_buffer {
                        logical_device.cmd_bind_index_buffer(command_buffer, mesh.index_buffer.unwrap(), 0, IndexType::UINT32);
                    }

                    logical_device.cmd_draw_indexed(command_buffer, mesh.indices_count, 1, 0, 0, 0);
                    last_index_buffer = mesh.index_buffer.unwrap();
                } 
                else { //No index buffer, draw without it
                    logical_device.cmd_draw(command_buffer, mesh.vertices_count, 1, 0, 0);
                }

                last_pipeline = mesh.graphics_pipeline;
                last_vertex_buffer = mesh.vertex_buffer;
            }

            logical_device.cmd_end_render_pass(command_buffer);

            logical_device.end_command_buffer(command_buffer).expect("Recording command buffer failed.");

            let wait_sempahores = &[image_available_semaphore];
            let wait_dst_stage_mask = &[PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let command_buffers = &[command_buffer];
            let signal_semaphores = &[render_finished_semaphore];

            let submit_info = SubmitInfo::builder()
                .wait_semaphores(wait_sempahores)
                .wait_dst_stage_mask(wait_dst_stage_mask)
                .command_buffers(command_buffers)
                .signal_semaphores(signal_semaphores);

            logical_device.queue_submit(self.vulkan_context.present_queue, &[submit_info.build()], in_flight_fence).unwrap();

            let wait_semaphores = &[render_finished_semaphore];
            let swapchains = &[self.vulkan_context.swapchain_khr];
            let image_indices = &[image_index];

            let present_info = PresentInfoKHR::builder()
                .wait_semaphores(wait_semaphores)
                .swapchains(swapchains)
                .image_indices(image_indices);
            
            match self.vulkan_context.swapchain_loader.queue_present(self.vulkan_context.present_queue, &present_info) {
                Ok(..) => (),
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => return RenderResult::VkOutOfDate,
                Err(error) => panic!("Queue present failed with error: {}", error)
            }

            self.current_frame = (self.current_frame + 1) & MAX_FRAMES_IN_FLIGHT;

            self.meshes_to_draw.clear();
        }

        RenderResult::RenderFinished
    }

    pub fn clear_color(&mut self, color: [f32; 4]) {
        self.clear_color = color;
    }

    pub fn draw_mesh(&mut self, mesh: &VulkanMesh, render_pipeline: &RenderPipeline) {        
        let index_buffer = match &mesh.index_buffer {
            Some(value) => Some(value.buffer),
            None => None,
        };

        let renderable_mesh = RenderableMesh {
            vertex_buffer: mesh.vertex_buffer.as_ref().unwrap().buffer,
            index_buffer: index_buffer,
            vertices_count: mesh.vertex_input.as_ref().unwrap().vertex_data.len() as u32,
            indices_count: mesh.vertex_indices.len() as u32,
            push_constants: mesh.push_constant,
            pipeline_layout: render_pipeline.pipeline_layout,
            graphics_pipeline: render_pipeline.graphics_pipeline,
            descriptor_sets: render_pipeline.descriptor_sets.clone()
        };

        self.meshes_to_draw.push(renderable_mesh);
    }

    pub fn populate_vertex_buffer(&mut self, mesh: &mut VulkanMesh, vertex_data: Vec<VertexData>, vertex_indices: Vec<u32>) {
        mesh.add_mesh_data(vertex_data, vertex_indices, &mut self.vulkan_context, self.command_pool);
    }

    pub fn destroy_mesh(&mut self, mesh: &mut VulkanMesh) {
        mesh.destroy_mesh(&mut self.vulkan_context);
    }

    pub fn create_sampler(&self, filter: Filter, address_mode: SamplerAddressMode, mipmap_mode: SamplerMipmapMode, min_lod: f32, max_lod: f32) -> Sampler {
        let sampler_info = SamplerCreateInfo::builder()
            .mag_filter(filter)
            .min_filter(filter)
            .address_mode_u(address_mode)
            .address_mode_v(address_mode)
            .address_mode_w(address_mode)
            .anisotropy_enable(true)
            .max_anisotropy(self.vulkan_context.get_physical_device_properties().limits.max_sampler_anisotropy)
            .border_color(BorderColor::INT_OPAQUE_BLACK)
            .unnormalized_coordinates(false)
            .compare_enable(false)
            .compare_op(CompareOp::ALWAYS)
            .mipmap_mode(mipmap_mode)
            .mip_lod_bias(0.0)
            .min_lod(min_lod)
            .max_lod(max_lod);

        unsafe {
            self.vulkan_context.logical_device.create_sampler(&sampler_info, None).expect("Sampler creation failed.")
        }
    }

    pub fn destroy_sampler(&self, sampler: Sampler) {
        unsafe {
            self.vulkan_context.logical_device.destroy_sampler(sampler, None);
        }
    }

    pub fn create_texture(&mut self, texture_path: &str, texture_name: &str, generate_mipmaps: bool) -> VulkanImage {
        let image_buffer = image::open(texture_path).expect("Loading texture file failed.").into_rgba8();

        let mut texture_staging_buffer = VulkanBuffer::new(&self.vulkan_context.logical_device, &mut self.vulkan_context.allocator, (image_buffer.width() * image_buffer.height() * 4) as u64, 
        BufferUsageFlags::TRANSFER_SRC, gpu_allocator::MemoryLocation::CpuToGpu, "Texture staging buffer");

        unsafe {
            let texture_memory = image_buffer.as_ptr();
            let texture_buffer_memory = texture_staging_buffer.memory.as_ptr();

            std::ptr::copy_nonoverlapping(texture_memory, texture_buffer_memory.cast(), image_buffer.len());
        }

        let format_properties = self.vulkan_context.get_physical_device_format_properties(Format::R8G8B8A8_SRGB);

        let mut mipmapping = generate_mipmaps;

        if generate_mipmaps && (format_properties.optimal_tiling_features & FormatFeatureFlags::SAMPLED_IMAGE_FILTER_LINEAR).is_empty() {
            println!("Error: Unsupported format property, mipmapping disabled");
            mipmapping = false;
        }

        let mut texture_image = VulkanImage::new(&self.vulkan_context.logical_device, &mut self.vulkan_context.allocator, texture_name, 
            image_buffer.width(), image_buffer.height(), Format::R8G8B8A8_SRGB, ImageTiling::OPTIMAL, ImageUsageFlags::TRANSFER_SRC | 
            ImageUsageFlags::TRANSFER_DST | ImageUsageFlags::SAMPLED, ImageAspectFlags::COLOR, mipmapping, SampleCountFlags::TYPE_1);

        texture_image.transition_image_layout(&self.vulkan_context.logical_device, self.vulkan_context.present_queue, self.command_pool, ImageLayout::TRANSFER_DST_OPTIMAL);
        texture_image.populate_from_buffer(&self.vulkan_context.logical_device, self.vulkan_context.present_queue, self.command_pool, &texture_staging_buffer);

        if mipmapping {
            texture_image.generate_mipmaps(&self.vulkan_context.logical_device, self.vulkan_context.present_queue, self.command_pool);
        }
        else {
            texture_image.transition_image_layout(&self.vulkan_context.logical_device, self.vulkan_context.present_queue, self.command_pool, ImageLayout::SHADER_READ_ONLY_OPTIMAL);
        }

        texture_staging_buffer.free(&self.vulkan_context.logical_device, &mut self.vulkan_context.allocator);
        drop(image_buffer);

        texture_image
    }

    pub fn destroy_texture(&mut self, texture: &mut VulkanImage) {
        texture.free(&self.vulkan_context.logical_device, &mut self.vulkan_context.allocator);
    }

    pub fn create_pipeline(&mut self, vertex_shader_location: &str, fragment_shader_location: &str, descriptor_set: Option<&VulkanDescriptor>) -> RenderPipeline {
        let vertex_shader = Self::create_shader_module(&self.vulkan_context.logical_device, vertex_shader_location);
        let fragment_shader = Self::create_shader_module(&self.vulkan_context.logical_device, fragment_shader_location);

        let (pipeline_layout, graphics_pipeline) = match descriptor_set {
            Some(descriptor_set) => Self::create_graphics_pipeline(&self.vulkan_context.logical_device, vertex_shader, fragment_shader, 
                self.render_pass, Some(descriptor_set.descriptor_set_layout)),

            None => Self::create_graphics_pipeline(&self.vulkan_context.logical_device, vertex_shader, fragment_shader, 
                self.render_pass, None),
        };

        let descriptor_sets = match descriptor_set {
            Some(descriptor_set) => descriptor_set.get_descriptor_sets(),
            None => Vec::new()
        };

        RenderPipeline {
            graphics_pipeline,
            pipeline_layout,
            vertex_shader,
            fragment_shader,
            descriptor_sets
        }
    }

    pub fn destroy_pipeline(&mut self, render_pipeline: &mut RenderPipeline) {
        unsafe {
            self.vulkan_context.logical_device.destroy_shader_module(render_pipeline.vertex_shader, None);
            self.vulkan_context.logical_device.destroy_shader_module(render_pipeline.fragment_shader, None);
            self.vulkan_context.logical_device.destroy_pipeline_layout(render_pipeline.pipeline_layout, None);
            self.vulkan_context.logical_device.destroy_pipeline(render_pipeline.graphics_pipeline, None);
        }
    }

    pub fn create_descriptor(&mut self, uniform_buffer_size: u64, name: &str, image_views: Vec<ImageView>, sampler: Option<Sampler>) -> VulkanDescriptor {
        VulkanDescriptor::new(&self.vulkan_context.logical_device, &mut self.vulkan_context.allocator, MAX_FRAMES_IN_FLIGHT, uniform_buffer_size, name, sampler, image_views)
    }

    pub fn destroy_descriptor(&mut self, descriptor: &mut VulkanDescriptor) {
        descriptor.free(&self.vulkan_context.logical_device, &mut self.vulkan_context.allocator);
    }

    pub fn resize_viewport(&mut self, window_width: u32, window_height: u32) {        
        unsafe {
            self.vulkan_context.logical_device.device_wait_idle().unwrap();
        }

        self.depth_image.free(&self.vulkan_context.logical_device, &mut self.vulkan_context.allocator);
        self.color_image.free(&self.vulkan_context.logical_device, &mut self.vulkan_context.allocator);

        unsafe {
            for &framebuffer in self.framebuffers.iter() {
                self.vulkan_context.logical_device.destroy_framebuffer(framebuffer, None);
            }
        }

        self.vulkan_context.recreate_swapchain(window_width, window_height);

        let color_image = VulkanImage::new(&self.vulkan_context.logical_device, &mut self.vulkan_context.allocator, "Color image", self.vulkan_context.surface_resolution.width, 
            self.vulkan_context.surface_resolution.height, self.vulkan_context.surface_format.format, ImageTiling::OPTIMAL, ImageUsageFlags::TRANSIENT_ATTACHMENT | ImageUsageFlags::COLOR_ATTACHMENT, 
            ImageAspectFlags::COLOR, false, SAMPLE_COUNT);

        let depth_image = VulkanImage::new(&self.vulkan_context.logical_device, &mut self.vulkan_context.allocator, "Depth buffer", self.vulkan_context.surface_resolution.width, 
            self.vulkan_context.surface_resolution.height, Format::D32_SFLOAT, ImageTiling::OPTIMAL, ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT, 
            ImageAspectFlags::DEPTH, false, SAMPLE_COUNT);

        let framebuffers: Vec<vk::Framebuffer> = self.vulkan_context
            .swapchain_image_views
            .iter()
            .map(|&swapchain_image_view| {
                let framebuffer_attachments = [color_image.image_view, depth_image.image_view, swapchain_image_view];
                let frame_buffer_create_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(self.render_pass)
                    .attachments(&framebuffer_attachments)
                    .width(self.vulkan_context.surface_resolution.width)
                    .height(self.vulkan_context.surface_resolution.height)
                    .layers(1);

                unsafe {
                    self.vulkan_context.logical_device
                        .create_framebuffer(&frame_buffer_create_info, None)
                        .unwrap()
                }
            })
            .collect();

        self.color_image = color_image;
        self.depth_image = depth_image;
        self.framebuffers = framebuffers;
    }

    fn create_render_pass(surface_format: Format, logical_device: &Device, depth_image: &VulkanImage) -> RenderPass {
        let attachments = &[
            vk::AttachmentDescription {
                format: surface_format,
                samples: SAMPLE_COUNT,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::STORE,
                stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
                stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
                initial_layout: vk::ImageLayout::UNDEFINED,
                final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                flags: AttachmentDescriptionFlags::empty()
            },
            vk::AttachmentDescription {
                format: depth_image.format,
                samples: SAMPLE_COUNT,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::DONT_CARE,
                stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
                stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
                initial_layout: ImageLayout::UNDEFINED,
                final_layout: ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                flags: AttachmentDescriptionFlags::empty()
            },
            AttachmentDescription {
                format: surface_format,
                samples: SampleCountFlags::TYPE_1,
                load_op: AttachmentLoadOp::DONT_CARE,
                store_op: AttachmentStoreOp::STORE,
                stencil_load_op: AttachmentLoadOp::DONT_CARE,
                stencil_store_op: AttachmentStoreOp::DONT_CARE,
                initial_layout: ImageLayout::UNDEFINED,
                final_layout: ImageLayout::PRESENT_SRC_KHR,
                flags: AttachmentDescriptionFlags::empty()
            }
        ];

        let color_attachment_ref = vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
        };

        let depth_attachment_ref = vk::AttachmentReference {
            attachment: 1,
            layout: ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL
        };

        let color_attachment_resolve_ref = vk::AttachmentReference {
            attachment: 2,
            layout: ImageLayout::COLOR_ATTACHMENT_OPTIMAL
        };

        let subpass = vk::SubpassDescription::builder()
            .color_attachments(std::slice::from_ref(&color_attachment_ref))
            .depth_stencil_attachment(&depth_attachment_ref)
            .resolve_attachments(std::slice::from_ref(&color_attachment_resolve_ref))
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS);

        let dependencies = [vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            ..Default::default()
        }];

        let render_pass_create_info = vk::RenderPassCreateInfo::builder()
            .attachments(attachments)
            .subpasses(std::slice::from_ref(&subpass))
            .dependencies(&dependencies);

        let render_pass = unsafe {
            logical_device.create_render_pass(&render_pass_create_info, None).expect("Render pass creation failed.")
        };

        render_pass
    }

    fn create_shader_module(logical_device: &Device, filename: &str) -> ShaderModule {
        let mut shader_file = File::open(&filename).expect("Failed to shader file.");
        let spv_code = read_spv(&mut shader_file).expect("Reading shader file failed.");

        let shader_module_info = ShaderModuleCreateInfo::builder()
            .code(&spv_code);

        let shader_module = unsafe {
            logical_device.create_shader_module(&shader_module_info, None).expect("Creating shader module failed.")
        };

        shader_module
    }

    fn create_graphics_pipeline(logical_device: &Device, vertex_shader: ShaderModule, fragment_shader: ShaderModule, render_pass: RenderPass, descriptor_set_layout: Option<DescriptorSetLayout>) -> (PipelineLayout, Pipeline) {
        let push_constant_ranges = &[
            PushConstantRange::builder()
            .offset(0)
            .size(mem::size_of::<PushConstant>() as u32)
            .stage_flags(ShaderStageFlags::VERTEX)
            .build()
        ];
        let descriptor_set_layout = match descriptor_set_layout {
            Some(descriptor_set_layout) => descriptor_set_layout,
            None => DescriptorSetLayout::null()
        };

        let set_layouts = &[descriptor_set_layout];

        let pipeline_layout_info = PipelineLayoutCreateInfo::builder()
            .push_constant_ranges(push_constant_ranges)
            .set_layouts(set_layouts);

        let pipeline_layout = unsafe {
            logical_device.create_pipeline_layout(&pipeline_layout_info, None).expect("Pipeline layout creation failed.")
        };

        let mut vulkan_pipeline = VulkanPipeline::new(PrimitiveTopology::TRIANGLE_LIST);
        vulkan_pipeline.add_shader_stage(ShaderStageFlags::VERTEX, vertex_shader);
        vulkan_pipeline.add_shader_stage(ShaderStageFlags::FRAGMENT, fragment_shader);

        vulkan_pipeline.add_vertex_input_bindings(&mut VertexInput::get_binding_descriptions());
        vulkan_pipeline.add_vertex_input_attributes(&mut VertexInput::get_attribute_descriptions());
        
        let graphics_pipeline = vulkan_pipeline.build_pipeline(&logical_device, pipeline_layout, render_pass, SAMPLE_COUNT);

        (pipeline_layout, graphics_pipeline)
    }

    fn create_commands(logical_device: &Device, queue_family_index: u32, command_buffer_count: u32) -> (CommandPool, Vec<CommandBuffer>) {
        let command_pool_info = CommandPoolCreateInfo::builder()
            .queue_family_index(queue_family_index)
            .flags(CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

        let command_pool = unsafe {
            logical_device.create_command_pool(&command_pool_info, None).expect("Command pool creation failed.")
        };

        let command_buffer_info = CommandBufferAllocateInfo::builder()
            .command_pool(command_pool)
            .command_buffer_count(command_buffer_count)
            .level(CommandBufferLevel::PRIMARY);

        let command_buffers = unsafe {
            logical_device.allocate_command_buffers(&command_buffer_info).expect("Command buffer allocation failed")
        };

        (command_pool, command_buffers)
    }
}

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        unsafe {
            self.vulkan_context.logical_device.device_wait_idle().unwrap();

            self.color_image.free(&self.vulkan_context.logical_device, &mut self.vulkan_context.allocator);
            self.depth_image.free(&self.vulkan_context.logical_device, &mut self.vulkan_context.allocator);

            for n in self.frame_data.iter_mut() {
                self.vulkan_context.logical_device.destroy_fence(n.in_flight_fence, None);
                self.vulkan_context.logical_device.destroy_semaphore(n.render_finished_semaphore, None);
                self.vulkan_context.logical_device.destroy_semaphore(n.image_available_semaphore, None);
            }

            self.vulkan_context.logical_device.destroy_command_pool(self.command_pool, None);

            for &framebuffer in self.framebuffers.iter() {
                self.vulkan_context.logical_device.destroy_framebuffer(framebuffer, None);
            }

            self.vulkan_context.logical_device.destroy_render_pass(self.render_pass, None);
        }
    }
}
