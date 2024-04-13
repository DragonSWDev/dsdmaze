//Image management
//Allocating, loading, transitioning layout, generating mipmaps etc.

use ash::{vk::{self, AccessFlags, BufferImageCopy, CommandBufferAllocateInfo, CommandBufferBeginInfo, CommandBufferLevel, CommandPool, DependencyFlags, Extent3D, Fence, Filter, Format, Image, 
    ImageAspectFlags, ImageBlit, ImageCreateInfo, ImageLayout, ImageMemoryBarrier, ImageSubresourceLayers, ImageSubresourceRange, ImageTiling, ImageType, ImageUsageFlags, ImageView, ImageViewCreateInfo, 
    ImageViewType, Offset3D, PipelineStageFlags, Queue, SampleCountFlags, SharingMode, SubmitInfo}, Device};

use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, Allocator};

use super::vulkan_buffer::VulkanBuffer;

pub struct VulkanImage {
    pub image: Image,
    pub image_view: ImageView,
    allocation: Option<Allocation>,
    pub format: Format,
    pub width: u32,
    pub height: u32,
    pub aspect_flags: ImageAspectFlags,
    layout: ImageLayout,
    mip_levels: u32
}

impl VulkanImage {
    pub fn new(logical_device: &Device, allocator: &mut Allocator, name: &str, width: u32, height: u32, format: Format, tiling: ImageTiling, 
            usage: ImageUsageFlags, aspect_flags: ImageAspectFlags, enable_mipmapping: bool, sample_count: SampleCountFlags) -> Self {

        let mip_levels = match enable_mipmapping {
            true => {
                let mip_levels = (width.max(height) as f32).log2();
                let mip_levels = (mip_levels.floor() as u32) + 1;

                mip_levels
            },
            false => 1
        };
        
        let image_create_info = ImageCreateInfo::builder()
            .image_type(ImageType::TYPE_2D)
            .extent(Extent3D {
                width,
                height,
                depth: 1
            })
            .mip_levels(mip_levels)
            .array_layers(1)
            .format(format)
            .tiling(tiling)
            .initial_layout(ImageLayout::UNDEFINED)
            .usage(usage)
            .samples(sample_count)
            .sharing_mode(SharingMode::EXCLUSIVE);

        let image = unsafe {
            logical_device.create_image(&image_create_info, None).expect("Image creation failed.")
        };

        let requirements = unsafe {
            logical_device.get_image_memory_requirements(image)
        };

        let allocation = allocator.allocate(&AllocationCreateDesc {
            name: name,
            requirements: requirements,
            location: gpu_allocator::MemoryLocation::GpuOnly,
            linear: true,
            allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged
        }).expect("Image memory allocation failed.");

        unsafe {
            logical_device.bind_image_memory(image, allocation.memory(), allocation.offset()).expect("Binding image memory failed.");
        }

        let image_view_create_info = ImageViewCreateInfo::builder()
            .image(image)
            .view_type(ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(ImageSubresourceRange {
                aspect_mask: aspect_flags,
                base_mip_level: 0,
                level_count: mip_levels,
                base_array_layer: 0,
                layer_count: 1
            });

        let image_view = unsafe {
            logical_device.create_image_view(&image_view_create_info, None).expect("Image view creation failed.")
        };

        Self {
            image,
            image_view,
            allocation: Some(allocation),
            format,
            width,
            height,
            aspect_flags,
            layout: ImageLayout::UNDEFINED,
            mip_levels: mip_levels
        }
    }

    pub fn free(&mut self, logical_device: &Device, allocator: &mut Allocator) {
        let allocation = self.allocation.take().unwrap();

        allocator.free(allocation).expect("Destroying allocation failed.");

        unsafe {
            logical_device.destroy_image_view(self.image_view, None);
            logical_device.destroy_image(self.image, None);
        }
    }

    pub fn transition_image_layout(&mut self, logical_device: &Device, present_queue: Queue, command_pool: CommandPool, new_layout: ImageLayout) {
        let command_buffer_info = CommandBufferAllocateInfo::builder()
            .command_pool(command_pool)
            .command_buffer_count(1)
            .level(CommandBufferLevel::PRIMARY);

        let command_buffer = unsafe {
            let command_buffers = logical_device.allocate_command_buffers(&command_buffer_info).expect("Command buffer allocation failed.");

            command_buffers[0]
        };

        let (src_access_mask, dst_access_mask, src_stage, dst_stage) = match (self.layout, new_layout) {
            (ImageLayout::UNDEFINED, ImageLayout::TRANSFER_DST_OPTIMAL) => (
                AccessFlags::empty(),
                AccessFlags::TRANSFER_WRITE,
                PipelineStageFlags::TOP_OF_PIPE,
                PipelineStageFlags::TRANSFER
            ),
            (ImageLayout::TRANSFER_DST_OPTIMAL, ImageLayout::SHADER_READ_ONLY_OPTIMAL) =>
            (
                AccessFlags::TRANSFER_WRITE,
                AccessFlags::SHADER_READ,
                PipelineStageFlags::TRANSFER,
                PipelineStageFlags::FRAGMENT_SHADER
            ),
            _ => panic!("Unsupported transition requested.")
        };

        unsafe {
            logical_device.begin_command_buffer(command_buffer, &CommandBufferBeginInfo::default()).expect("Command buffer record failed.");

            let image_memory_barrier = ImageMemoryBarrier::builder()
                .old_layout(self.layout)
                .new_layout(new_layout)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(self.image)
                .subresource_range(ImageSubresourceRange {
                    aspect_mask: self.aspect_flags,
                    base_mip_level: 0,
                    level_count: self.mip_levels,
                    base_array_layer: 0,
                    layer_count: 1
                })
                .src_access_mask(src_access_mask) 
                .dst_access_mask(dst_access_mask)
                .build();

            logical_device.cmd_pipeline_barrier(command_buffer, src_stage, dst_stage, DependencyFlags::empty(), &[], 
                &[], &[image_memory_barrier]);

            logical_device.end_command_buffer(command_buffer).unwrap();

            let command_buffers = &[command_buffer];

            let submit_info = SubmitInfo::builder()
                .command_buffers(command_buffers);
    
            logical_device.queue_submit(present_queue, &[submit_info.build()], Fence::null()).unwrap();
            logical_device.queue_wait_idle(present_queue).unwrap();
    
            logical_device.free_command_buffers(command_pool, &[command_buffer]);
        }

        self.layout = new_layout;
    }

    pub fn populate_from_buffer(&mut self, logical_device: &Device, present_queue: Queue, command_pool: CommandPool, src_buffer: &VulkanBuffer) {
        let command_buffer_info = CommandBufferAllocateInfo::builder()
            .command_pool(command_pool)
            .command_buffer_count(1)
            .level(CommandBufferLevel::PRIMARY);

        let command_buffer = unsafe {
            let command_buffers = logical_device.allocate_command_buffers(&command_buffer_info).expect("Command buffer allocation failed.");

            command_buffers[0]
        };

        unsafe {
            logical_device.begin_command_buffer(command_buffer, &CommandBufferBeginInfo::default()).expect("Command buffer record failed.");

            let image_copy_region = BufferImageCopy::builder()
                .buffer_offset(0)
                .buffer_row_length(0)
                .buffer_image_height(0)
                .image_subresource(ImageSubresourceLayers {
                    aspect_mask: self.aspect_flags,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1
                })
                .image_offset(Offset3D {
                    x: 0,
                    y: 0,
                    z: 0
                })
                .image_extent(Extent3D {
                    width: self.width,
                    height: self.height,
                    depth: 1
                })
                .build();

            logical_device.cmd_copy_buffer_to_image(command_buffer, src_buffer.buffer, self.image, ImageLayout::TRANSFER_DST_OPTIMAL, &[image_copy_region]);

            logical_device.end_command_buffer(command_buffer).unwrap();

            let command_buffers = &[command_buffer];

            let submit_info = SubmitInfo::builder()
                .command_buffers(command_buffers);

            logical_device.queue_submit(present_queue, &[submit_info.build()], Fence::null()).unwrap();
            logical_device.queue_wait_idle(present_queue).unwrap();

            logical_device.free_command_buffers(command_pool, &[command_buffer]);
        }
    }

    pub fn generate_mipmaps(&mut self, logical_device: &Device, present_queue: Queue, command_pool: CommandPool) {
        if self.mip_levels == 1 {
            panic!("Attempted to generate mipmaps on image without mipmaping enabled.");
        }

        let command_buffer_info = CommandBufferAllocateInfo::builder()
            .command_pool(command_pool)
            .command_buffer_count(1)
            .level(CommandBufferLevel::PRIMARY);

        let command_buffer = unsafe {
            let command_buffers = logical_device.allocate_command_buffers(&command_buffer_info).expect("Command buffer allocation failed.");

            command_buffers[0]
        };

        unsafe {
            logical_device.begin_command_buffer(command_buffer, &CommandBufferBeginInfo::default()).expect("Command buffer record failed.");

            let mut mip_width = self.width;
            let mut mip_height = self.height;

            let mut image_memory_barrier = ImageMemoryBarrier::builder()
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(self.image)
                .build();

            for n in 1..self.mip_levels {
                image_memory_barrier.old_layout = ImageLayout::TRANSFER_DST_OPTIMAL;
                image_memory_barrier.new_layout = ImageLayout::TRANSFER_SRC_OPTIMAL;
                image_memory_barrier.src_access_mask = AccessFlags::TRANSFER_WRITE;
                image_memory_barrier.dst_access_mask = AccessFlags::TRANSFER_READ;

                image_memory_barrier.subresource_range = ImageSubresourceRange {
                    aspect_mask: self.aspect_flags,
                    base_mip_level: n - 1,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1
                };

                logical_device.cmd_pipeline_barrier(command_buffer, PipelineStageFlags::TRANSFER, PipelineStageFlags::TRANSFER, DependencyFlags::empty(), &[], 
                    &[], &[image_memory_barrier]);


                let image_blit = ImageBlit::builder()
                    .src_offsets([
                        Offset3D {
                            x: 0,
                            y: 0,
                            z: 0
                        },
                        Offset3D {
                            x: mip_width as i32,
                            y: mip_height as i32,
                            z: 1
                        }
                    ])
                    .src_subresource(ImageSubresourceLayers {
                        aspect_mask: ImageAspectFlags::COLOR,
                        mip_level: n - 1,
                        base_array_layer: 0,
                        layer_count: 1
                    })
                    .dst_offsets([
                        Offset3D {
                            x: 0,
                            y: 0,
                            z: 0
                        },
                        Offset3D {
                            x: if mip_width > 1 { (mip_width / 2) as i32 } else { 1 },
                            y: if mip_height > 1 { (mip_height / 2) as i32 } else { 1 },
                            z: 1
                        }
                    ])
                    .dst_subresource(ImageSubresourceLayers {
                        aspect_mask: ImageAspectFlags::COLOR,
                        mip_level: n,
                        base_array_layer: 0,
                        layer_count: 1
                    })
                    .build();

                logical_device.cmd_blit_image(command_buffer, self.image, ImageLayout::TRANSFER_SRC_OPTIMAL, self.image, ImageLayout::TRANSFER_DST_OPTIMAL, 
                    &[image_blit], Filter::LINEAR);

                image_memory_barrier.old_layout = ImageLayout::TRANSFER_SRC_OPTIMAL;
                image_memory_barrier.new_layout = ImageLayout::SHADER_READ_ONLY_OPTIMAL;
                image_memory_barrier.src_access_mask = AccessFlags::TRANSFER_READ;
                image_memory_barrier.dst_access_mask = AccessFlags::SHADER_READ;

                image_memory_barrier.subresource_range = ImageSubresourceRange {
                    aspect_mask: self.aspect_flags,
                    base_mip_level: n - 1,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1
                };

                logical_device.cmd_pipeline_barrier(command_buffer, PipelineStageFlags::TRANSFER, PipelineStageFlags::FRAGMENT_SHADER, DependencyFlags::empty(), &[], 
                    &[], &[image_memory_barrier]);

                if mip_width > 1 {
                    mip_width /= 2;
                }

                if mip_height >1 {
                    mip_height /= 2;
                }
            }

            image_memory_barrier.old_layout = ImageLayout::TRANSFER_DST_OPTIMAL;
            image_memory_barrier.new_layout = ImageLayout::SHADER_READ_ONLY_OPTIMAL;
            image_memory_barrier.src_access_mask = AccessFlags::TRANSFER_READ;
            image_memory_barrier.dst_access_mask = AccessFlags::SHADER_READ;

            image_memory_barrier.subresource_range = ImageSubresourceRange {
                aspect_mask: self.aspect_flags,
                base_mip_level: self.mip_levels - 1,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1
            };

            logical_device.cmd_pipeline_barrier(command_buffer, PipelineStageFlags::TRANSFER, PipelineStageFlags::FRAGMENT_SHADER, DependencyFlags::empty(), &[], 
                &[], &[image_memory_barrier]);

            logical_device.end_command_buffer(command_buffer).unwrap();

            let command_buffers = &[command_buffer];

            let submit_info = SubmitInfo::builder()
                .command_buffers(command_buffers);
    
            logical_device.queue_submit(present_queue, &[submit_info.build()], Fence::null()).unwrap();
            logical_device.queue_wait_idle(present_queue).unwrap();
    
            logical_device.free_command_buffers(command_pool, &[command_buffer]);
        }
    }
}
