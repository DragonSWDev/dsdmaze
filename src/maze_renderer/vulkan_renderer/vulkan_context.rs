//Responsible for creating instance, debug utils messenger, creating surface, picking physical device,
//creating logical device and creating (or recreating) swapchain

use std::{ffi::CStr, mem::ManuallyDrop};

use ash::{extensions::{ext::DebugUtils, khr::{Surface, Swapchain}}, vk::{self, ColorSpaceKHR, Extent2D, Format, FormatProperties, Image, ImageView, PhysicalDevice, 
    PhysicalDeviceProperties, Queue, SurfaceFormatKHR, SurfaceKHR, SwapchainKHR}, Device, Entry, Instance};
use gpu_allocator::vulkan::{Allocator, AllocatorCreateDesc};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use winit::window::Window;

pub struct VulkanContext {
    pub instance: Instance,
    surface_loader: Surface,
    surface_khr: SurfaceKHR,
    pub physical_device: PhysicalDevice,
    pub queue_family_index: u32,
    pub logical_device: Device,
    pub present_queue: Queue,
    pub surface_format: SurfaceFormatKHR,
    pub surface_resolution: Extent2D,
    pub swapchain_loader: Swapchain,
    pub swapchain_khr: SwapchainKHR,
    _swapchain_images: Vec<Image>,
    pub swapchain_image_views: Vec<ImageView>,
    pub allocator: ManuallyDrop<Allocator>,
}

impl VulkanContext {
    pub fn new(window: &Window, entry: &Entry) -> Self {
        let instance = Self::create_instance(window, entry);
        let surface_loader = Surface::new(entry, &instance);

        let surface_khr = unsafe {
            ash_window::create_surface(&entry, &instance, window.raw_display_handle(), window.raw_window_handle(), None).expect("SurfaceKHR creation failed.")
        };

        let (physical_device, queue_family_index) = Self::pick_physical_device(&instance, &surface_loader, surface_khr);

        let logical_device = Self::create_logical_device(&instance, physical_device, queue_family_index);

        let present_queue = unsafe {
            logical_device.get_device_queue(queue_family_index, 0)
        };

        let (surface_format, surface_resolution, swapchain_loader, swapchain_khr) = Self::create_swapchain(&instance, &logical_device, physical_device, 
            &surface_loader, surface_khr, window.inner_size().width, window.inner_size().height);

        let (_swapchain_images, swapchain_image_views) = Self::get_swapchain_image_imageviews(&swapchain_loader, swapchain_khr, &logical_device, surface_format);

        let allocator = Allocator::new(&AllocatorCreateDesc {
            instance: instance.clone(),
            device: logical_device.clone(),
            physical_device: physical_device,
            debug_settings: Default::default(),
            buffer_device_address: false,
            allocation_sizes: Default::default()
        }).expect("Allocator creation failed");

        let allocator = ManuallyDrop::new(allocator);
        
        Self {
            instance,
            surface_loader,
            surface_khr,
            physical_device,
            queue_family_index,
            logical_device,
            present_queue,
            surface_format,
            surface_resolution,
            swapchain_loader,
            swapchain_khr,
            _swapchain_images,
            swapchain_image_views,
            allocator
        }
    }

    pub fn get_physical_device_properties(&self) -> PhysicalDeviceProperties {
        unsafe {
            self.instance.get_physical_device_properties(self.physical_device)
        }
    }

    pub fn get_physical_device_format_properties(&self, format: Format) -> FormatProperties {
        unsafe {
            self.instance.get_physical_device_format_properties(self.physical_device, format)
        }
    }

    pub fn recreate_swapchain(&mut self, window_width: u32, window_height: u32) {
        self.destroy_swapchain();

        let (surface_format, surface_resolution, swapchain_loader, swapchain_khr) = Self::create_swapchain(&self.instance, &self.logical_device, self.physical_device, 
            &self.surface_loader, self.surface_khr, window_width, window_height);

        let (_swapchain_images, swapchain_image_views) = Self::get_swapchain_image_imageviews(&swapchain_loader, swapchain_khr, &self.logical_device, surface_format);
        
        self.surface_format = surface_format;
        self.surface_resolution = surface_resolution;
        self.swapchain_loader = swapchain_loader;
        self.swapchain_khr = swapchain_khr;
        self.swapchain_image_views = swapchain_image_views;
    }

    fn create_instance(window: &Window , entry: &Entry) -> Instance {
        let app_name = unsafe {
            CStr::from_bytes_with_nul_unchecked(b"maze_renderer_vk\0")
        };

        let app_info = vk::ApplicationInfo::builder()
            .application_name(app_name)
            .application_version(0)
            .engine_name(app_name)
            .engine_version(0)
            .api_version(vk::make_api_version(0, 1, 0, 0));

        let mut extension_names = ash_window::enumerate_required_extensions(window.raw_display_handle())
            .unwrap()
            .to_vec();

        #[cfg(any(target_os = "macos"))]
        {
            extension_names.push(KhrPortabilityEnumerationFn::name().as_ptr());
            extension_names.push(KhrGetPhysicalDeviceProperties2Fn::name().as_ptr());
        }

        extension_names.push(DebugUtils::name().as_ptr());

        let instance_flags = if cfg!(any(target_os = "macos")) {
            vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
        } else {
            vk::InstanceCreateFlags::default()
        };

        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(&extension_names)
            .flags(instance_flags);

        let instance = unsafe {
            entry.create_instance(&create_info, None).expect("Instance creation failed.")
        };

        instance
    }    

    fn pick_physical_device(instance: &Instance, surface_loader: &Surface, surface_khr: SurfaceKHR) -> (PhysicalDevice, u32) {
        let devices = unsafe {
            instance.enumerate_physical_devices().expect("Device enumeration failed.")
        };

        let (selected_device, queue_index) = unsafe {
            devices
                .iter()
                .find_map(|device| {
                    instance
                        .get_physical_device_queue_family_properties(*device)
                        .iter()
                        .enumerate()
                        .find_map(|(index, info)| {
                            let supports_graphic_and_surface =
                                info.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                                    && surface_loader
                                        .get_physical_device_surface_support(
                                            *device,
                                            index as u32,
                                            surface_khr,
                                        )
                                        .unwrap();
                            if supports_graphic_and_surface {
                                Some((*device, index))
                            } else {
                                None
                            }
                        })
                })
                .expect("Couldn't find suitable device.")
        };

        (selected_device, queue_index as u32)
    }

    fn create_logical_device(instance: &Instance, physical_device: PhysicalDevice, queue_index: u32) -> Device {
        let device_extension_names_raw = [
            Swapchain::name().as_ptr(),
            #[cfg(any(target_os = "macos"))]
            KhrPortabilitySubsetFn::name().as_ptr(),
        ];

        let features = vk::PhysicalDeviceFeatures {
            shader_clip_distance: 1,
            sampler_anisotropy: vk::TRUE,
            sample_rate_shading: vk::TRUE,
            ..Default::default()
        };
        
        let priorities = [1.0];

        let queue_info = vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_index)
            .queue_priorities(&priorities);

        let device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(std::slice::from_ref(&queue_info))
            .enabled_extension_names(&device_extension_names_raw)
            .enabled_features(&features);

        let device = unsafe {
            instance.create_device(physical_device, &device_create_info, None).expect("Logical device creation failed.")
        };

        device
    }

    fn create_swapchain(instance: &Instance, logical_device: &Device, physical_device: PhysicalDevice, surface_loader: &Surface, 
        surface_khr: SurfaceKHR, window_width: u32, window_height: u32) -> (SurfaceFormatKHR, Extent2D, Swapchain, SwapchainKHR) {

        let surface_format =  unsafe {
            let supported_surface_formats = surface_loader.get_physical_device_surface_formats(physical_device, surface_khr).unwrap();

            supported_surface_formats
                .iter()
                .cloned()
                .find(|format| {
                    format.format == Format::B8G8R8A8_SRGB &&
                        format.color_space == ColorSpaceKHR::SRGB_NONLINEAR
                })
                .unwrap_or(supported_surface_formats[0])
        };

        let surface_capabilities = unsafe {
            surface_loader.get_physical_device_surface_capabilities(physical_device, surface_khr).unwrap()
        };

        let mut desired_image_count = surface_capabilities.min_image_count + 1;

        if surface_capabilities.max_image_count > 0 && desired_image_count > surface_capabilities.max_image_count {
            desired_image_count = surface_capabilities.max_image_count;
        }

        let surface_resolution = match surface_capabilities.current_extent.width {
            std::u32::MAX => vk::Extent2D {
                width: window_width,
                height: window_height
            },
            _ => surface_capabilities.current_extent
        };

        let pre_transform = if surface_capabilities.supported_transforms.contains(vk::SurfaceTransformFlagsKHR::IDENTITY) {
            vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            surface_capabilities.current_transform
        };

        let swapchain_loader = Swapchain::new(&instance, &logical_device);

        let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(surface_khr)
            .min_image_count(desired_image_count)
            .image_color_space(surface_format.color_space)
            .image_format(surface_format.format)
            .image_extent(surface_resolution)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(pre_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(vk::PresentModeKHR::FIFO)
            .clipped(true)
            .image_array_layers(1);

        let swapchain_khr = unsafe {
            swapchain_loader.create_swapchain(&swapchain_create_info, None).expect("Swapchain creation failed")
        };

        (surface_format, surface_resolution, swapchain_loader, swapchain_khr)
    }

    fn get_swapchain_image_imageviews(swapchain_loader: &Swapchain, swapchain_khr: SwapchainKHR, logical_device: &Device, surface_format: SurfaceFormatKHR) -> (Vec<Image>, Vec<ImageView>) {
        let swapchain_images = unsafe {
            swapchain_loader.get_swapchain_images(swapchain_khr).unwrap()
        };

        let swapchain_image_views: Vec<vk::ImageView> = unsafe {
            swapchain_images.iter()
                .map(|&image| {
                    let create_view_info = vk::ImageViewCreateInfo::builder()
                        .view_type(vk::ImageViewType::TYPE_2D)
                        .format(surface_format.format)
                        .components(vk::ComponentMapping {
                            r: vk::ComponentSwizzle::R,
                            g: vk::ComponentSwizzle::G,
                            b: vk::ComponentSwizzle::B,
                            a: vk::ComponentSwizzle::A,
                        })
                        .subresource_range(vk::ImageSubresourceRange {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            base_mip_level: 0,
                            level_count: 1,
                            base_array_layer: 0,
                            layer_count: 1,
                        })
                        .image(image);

                    logical_device.create_image_view(&create_view_info, None).unwrap()
                })
                .collect()
        };

        (swapchain_images, swapchain_image_views)
    }

    fn destroy_swapchain(&mut self) {
        unsafe {
            for &image_view in self.swapchain_image_views.iter() {
                self.logical_device.destroy_image_view(image_view, None);
            }
    
            self.swapchain_loader.destroy_swapchain(self.swapchain_khr, None);
        }
    }
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        unsafe {
            self.destroy_swapchain();
            ManuallyDrop::drop(&mut self.allocator);
            self.logical_device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface_khr, None);
            self.instance.destroy_instance(None);
        }
    }
}
