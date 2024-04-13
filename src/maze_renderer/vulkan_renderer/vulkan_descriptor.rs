//Descriptor sets management
//Allocating buffers, descriptor pool and sets
//Allocates one set of uniform buffers (each for frame in flight), optionally with images for texture array

use std::{os::raw::c_void, ptr::NonNull, str::FromStr};

use ash::{vk::{BufferUsageFlags, DescriptorBufferInfo, DescriptorImageInfo, DescriptorPool, DescriptorPoolCreateInfo, DescriptorPoolSize, DescriptorSet, DescriptorSetAllocateInfo, DescriptorSetLayout, 
    DescriptorSetLayoutBinding, DescriptorSetLayoutCreateInfo, DescriptorType, ImageLayout, ImageView, Sampler, ShaderStageFlags, WriteDescriptorSet}, Device};

use gpu_allocator::vulkan::Allocator;

use super::vulkan_buffer::VulkanBuffer;

pub struct VulkanDescriptor {
    pub descriptor_set_layout: DescriptorSetLayout,
    uniform_buffers: Vec<VulkanBuffer>,
    descriptor_pool: DescriptorPool,
    descriptor_sets: Vec<DescriptorSet>
}

impl VulkanDescriptor {
    pub fn new(logical_device: &Device, allocator: &mut Allocator, frames_in_flight: usize, uniform_buffer_size: u64, name: &str, sampler: Option<Sampler>, image_views: Vec<ImageView>) -> Self {
        if sampler.is_some() && image_views.len() == 0 {
            panic!("Attempted to use sampler without images.");
        }
        
        let mut descriptor_set_layout_binding: Vec<DescriptorSetLayoutBinding> = Vec::new();
        
        let uniform_buffer_binding = DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(ShaderStageFlags::VERTEX)
            .build();

        descriptor_set_layout_binding.push(uniform_buffer_binding);

        match sampler {
            Some(_) => {
                let sampler_binding = DescriptorSetLayoutBinding::builder()
                    .binding(1)
                    .descriptor_type(DescriptorType::SAMPLER)
                    .descriptor_count(1)
                    .stage_flags(ShaderStageFlags::FRAGMENT)
                    .build();

                let texture_binding = DescriptorSetLayoutBinding::builder()
                    .binding(2)
                    .descriptor_type(DescriptorType::SAMPLED_IMAGE)
                    .descriptor_count(image_views.len() as u32)
                    .stage_flags(ShaderStageFlags::FRAGMENT)
                    .build();

                descriptor_set_layout_binding.push(sampler_binding);
                descriptor_set_layout_binding.push(texture_binding);
            },
            None => ()
        };

        let descriptor_set_layout_info = DescriptorSetLayoutCreateInfo::builder()
            .bindings(&descriptor_set_layout_binding.as_slice());

        let descriptor_set_layout = unsafe {
             logical_device.create_descriptor_set_layout(&descriptor_set_layout_info, None).expect("Descriptor set layout creation failed.")
        };

        let mut uniform_buffers: Vec<VulkanBuffer> = Vec::new();

        for n in 0..frames_in_flight {
            let mut buffer_name = String::from_str("Uniform buffer ").unwrap();
            buffer_name = buffer_name + name + " " + n.to_string().as_str();

            let uniform_buffer = VulkanBuffer::new(logical_device, allocator, uniform_buffer_size, 
                BufferUsageFlags::UNIFORM_BUFFER, gpu_allocator::MemoryLocation::CpuToGpu, buffer_name.as_str());

            uniform_buffers.push(uniform_buffer); 
        }

        let mut descriptor_pool_sizes: Vec<DescriptorPoolSize> = Vec::new();

        descriptor_pool_sizes.push(DescriptorPoolSize::builder()
            .ty(DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(frames_in_flight as u32)
            .build());

        if sampler.is_some() {
            descriptor_pool_sizes.push(DescriptorPoolSize::builder()
                .ty(DescriptorType::SAMPLER)
                .descriptor_count(frames_in_flight as u32)
                .build());

            descriptor_pool_sizes.push(DescriptorPoolSize::builder()
                .ty(DescriptorType::SAMPLED_IMAGE)
                .descriptor_count((frames_in_flight * image_views.len()) as u32)
                .build());
        }

        let descriptor_pool_create_info = DescriptorPoolCreateInfo::builder()
            .pool_sizes(descriptor_pool_sizes.as_slice())
            .max_sets(frames_in_flight as u32)
            .build();

        let descriptor_pool = unsafe {
            logical_device.create_descriptor_pool(&descriptor_pool_create_info, None).expect("Descriptor pool creation failed.")
        };

        let mut descriptor_set_layouts: Vec<DescriptorSetLayout> = Vec::new();

        for _n in 0..frames_in_flight {
            descriptor_set_layouts.push(descriptor_set_layout);
        }

        let descriptor_set_allocate_info = DescriptorSetAllocateInfo::builder()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&descriptor_set_layouts[..])
            .build();

        let descriptor_sets = unsafe {
            logical_device.allocate_descriptor_sets(&descriptor_set_allocate_info).expect("Allocating descriptor sets failed.")
        };

        let mut descriptor_image_infos: Vec<DescriptorImageInfo> = Vec::new();

        if sampler.is_some() {
            for n in 0..image_views.len() {
                let descriptor_image_info = DescriptorImageInfo::builder()
                    .image_view(image_views[n])
                    .sampler(sampler.unwrap())
                    .image_layout(ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .build();
    
                descriptor_image_infos.push(descriptor_image_info);
            }
        }

        for n in 0..frames_in_flight {
            let descriptor_buffer_info = DescriptorBufferInfo::builder()
                .buffer(uniform_buffers[n].buffer)
                .offset(0)
                .range(uniform_buffer_size)
                .build();

            let mut write_descriptor_sets: Vec<WriteDescriptorSet> = Vec::new();

            write_descriptor_sets.push(WriteDescriptorSet {
                dst_set: descriptor_sets[n],
                dst_binding: 0,
                dst_array_element: 0,
                descriptor_type: DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
                p_buffer_info: &descriptor_buffer_info,
                ..Default::default()
            });

            if sampler.is_some() {
                let sampler_info = DescriptorImageInfo::builder()
                    .sampler(sampler.unwrap())
                    .build();

                write_descriptor_sets.push(WriteDescriptorSet {
                    dst_set: descriptor_sets[n],
                    dst_binding: 1,
                    dst_array_element: 0,
                    descriptor_type: DescriptorType::SAMPLER,
                    descriptor_count: 1,
                    p_image_info: &sampler_info,
                    ..Default::default()
                });

                write_descriptor_sets.push(WriteDescriptorSet {
                    dst_set: descriptor_sets[n],
                    dst_binding: 2,
                    dst_array_element: 0,
                    descriptor_type: DescriptorType::SAMPLED_IMAGE,
                    descriptor_count: image_views.len() as u32,
                    p_image_info: descriptor_image_infos.as_ptr(),
                    ..Default::default()
                });
            }

            unsafe {
                logical_device.update_descriptor_sets(write_descriptor_sets.as_slice(), &[]);
            }
        }

        Self {
            descriptor_set_layout,
            uniform_buffers,
            descriptor_pool,
            descriptor_sets
        }
    }

    pub fn free(&mut self, logical_device: &Device, allocator: &mut Allocator) {
        for n in self.uniform_buffers.iter_mut() {
            n.free(logical_device, allocator);
        }

        unsafe {
            logical_device.destroy_descriptor_pool(self.descriptor_pool, None);
            logical_device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }

    pub fn get_descriptor_sets(&self) -> Vec<DescriptorSet> {
        let mut descriptor_sets = Vec::new();

        for n in self.descriptor_sets.iter() {
            descriptor_sets.push(n.clone());
        }

        descriptor_sets
    }

    pub fn get_uniform_buffers_memory(&self) -> Vec<NonNull<c_void>> {
        let mut uniforms_buffer_memory = Vec::new();

        for n in self.uniform_buffers.iter() {
            uniforms_buffer_memory.push(n.memory);
        }

        uniforms_buffer_memory
    }
}
