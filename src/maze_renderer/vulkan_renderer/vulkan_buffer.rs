//Buffer management

use std::{os::raw::c_void, ptr::NonNull};

use ash::{vk::{Buffer, BufferCopy, BufferCreateInfo, BufferUsageFlags, CommandBufferAllocateInfo, CommandBufferBeginInfo, CommandBufferLevel, CommandPool, Fence, Queue, SharingMode, SubmitInfo}, Device};
use gpu_allocator::{vulkan::{Allocation, AllocationCreateDesc, Allocator}, MemoryLocation};

pub struct VulkanBuffer {
    pub buffer: Buffer,
    allocation: Option<Allocation>,
    pub memory: NonNull<c_void>,
    pub size: u64,
    pub usage_flags: BufferUsageFlags,
    pub location: MemoryLocation
}

impl VulkanBuffer {
    pub fn new(logical_device: &Device, allocator: &mut Allocator, size: u64, usage_flags: BufferUsageFlags, location: MemoryLocation, name: &str) -> Self {
        let buffer_info = BufferCreateInfo::builder()
            .size(size)
            .usage(usage_flags)
            .sharing_mode(SharingMode::EXCLUSIVE);

        let buffer = unsafe {
            logical_device.create_buffer(&buffer_info, None).expect("Creating buffer failed.")
        };

        let requirements = unsafe {
            logical_device.get_buffer_memory_requirements(buffer)
        };

        let allocation = allocator.allocate(&AllocationCreateDesc {
            name: name,
            requirements: requirements,
            location: gpu_allocator::MemoryLocation::CpuToGpu,
            linear: true,
            allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged
        }).expect("Memory allocation failed.");

        let memory = allocation.mapped_ptr().unwrap();

        unsafe {
            logical_device.bind_buffer_memory(buffer, allocation.memory(), allocation.offset()).expect("Binding buffer to memory failed.");
        }
        
        Self {
            buffer,
            allocation: Some(allocation),
            memory,
            size,
            usage_flags,
            location
        } 
    }

    pub fn free(&mut self, logical_device: &Device, allocator: &mut Allocator) {
        let allocation = self.allocation.take().unwrap();

        allocator.free(allocation).expect("Destroying allocation failed.");

        unsafe {
            logical_device.destroy_buffer(self.buffer, None);
        }
    }

    pub fn copy_buffer(logical_device: &Device, command_pool: CommandPool, queue: Queue, src_buffer: &VulkanBuffer, dst_buffer: &VulkanBuffer) {
        if src_buffer.size != dst_buffer.size {
            panic!("Attempted to copy buffers with different size");
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

            let buffer_copy_region = BufferCopy::builder()
                .src_offset(0)
                .dst_offset(0)
                .size(src_buffer.size)
                .build();

            logical_device.cmd_copy_buffer(command_buffer, src_buffer.buffer, dst_buffer.buffer, &[buffer_copy_region]);

            logical_device.end_command_buffer(command_buffer).unwrap();

            let command_buffers = &[command_buffer];

            let submit_info = SubmitInfo::builder()
                .command_buffers(command_buffers);

            logical_device.queue_submit(queue, &[submit_info.build()], Fence::null()).unwrap();
            logical_device.queue_wait_idle(queue).unwrap();

            logical_device.free_command_buffers(command_pool, &[command_buffer]);
        }
    }
}
