use std::sync::Arc;

use glam::Mat4;
use vulkano::{
    command_buffer::{
        allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo},
        AutoCommandBufferBuilder, BlitImageInfo, CommandBufferUsage, PrimaryAutoCommandBuffer,
    },
    descriptor_set::{
        allocator::StandardDescriptorSetAllocator, DescriptorSet, WriteDescriptorSet,
    },
    device::{
        physical::PhysicalDeviceType, Device, DeviceCreateInfo, DeviceExtensions, DeviceFeatures,
        Queue, QueueCreateInfo, QueueFlags,
    },
    format::{Format, NumericFormat},
    image::{view::ImageView, Image, ImageCreateInfo, ImageType, ImageUsage},
    instance::{Instance, InstanceCreateInfo},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
    pipeline::{
        compute::ComputePipelineCreateInfo, layout::PipelineDescriptorSetLayoutCreateInfo,
        ComputePipeline, Pipeline, PipelineBindPoint, PipelineLayout,
        PipelineShaderStageCreateInfo,
    },
    swapchain::{
        self, PresentMode, Surface, SurfaceInfo, Swapchain, SwapchainCreateInfo,
        SwapchainPresentInfo,
    },
    sync::{self, GpuFuture},
    VulkanLibrary,
};
use winit::{event_loop::ActiveEventLoop, window::Window};

pub struct Renderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    swapchain: Arc<Swapchain>,
    command_buffers: Vec<Arc<PrimaryAutoCommandBuffer>>,
}

mod cs {
    vulkano_shaders::shader! {
        ty: "compute",
        src: /* glsl */ r"
            #version 460

            layout(local_size_x = 32, local_size_y = 32) in;
            layout(binding = 0, location = 0, rgba32f) uniform image2D output_texture;

            void main() {
                imageStore(output_texture, ivec2(gl_GlobalInvocationID.xy), vec4(1.0, 0.0, 0.0, 1.0));
            }
        "
    }
}

impl Renderer {
    pub fn new(window: Arc<Window>, event_loop: &ActiveEventLoop) -> Self {
        let library = VulkanLibrary::new().unwrap();
        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                enabled_extensions: Surface::required_extensions(event_loop).unwrap(),
                ..Default::default()
            },
        )
        .unwrap();

        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            //khr_acceleration_structure: true,
            //khr_ray_tracing_pipeline: true,
            ..Default::default()
        };

        let device_features = DeviceFeatures {
            //ray_tracing_pipeline: true,
            //acceleration_structure: true,
            ..Default::default()
        };

        let (physical_device, queue_family_index) = instance
            .enumerate_physical_devices()
            .unwrap()
            .filter(|device| {
                device.supported_extensions().contains(&device_extensions)
                    && device.supported_features().contains(&device_features)
            })
            .filter_map(|device| {
                device
                    .queue_family_properties()
                    .iter()
                    .enumerate()
                    .position(|(queue_index, queue_properties)| {
                        queue_properties
                            .queue_flags
                            .contains(QueueFlags::GRAPHICS | QueueFlags::COMPUTE)
                            && device
                                .presentation_support(queue_index as u32, event_loop)
                                .unwrap()
                    })
                    .map(|queue_index| (device, queue_index as u32))
            })
            .min_by_key(|(device, _)| match device.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                PhysicalDeviceType::Other => 4,
                _ => 5,
            })
            .unwrap();

        log::info!(
            "Using device: {} (type: {:?})",
            physical_device.properties().device_name,
            physical_device.properties().device_type,
        );

        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                enabled_extensions: device_extensions,
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                ..Default::default()
            },
        )
        .unwrap();

        let queue = queues.next().unwrap();

        let surface = Surface::from_window(instance.clone(), window.clone()).unwrap();
        let window_size = window.inner_size();

        let surface_capabilities = device
            .physical_device()
            .surface_capabilities(&surface, SurfaceInfo::default())
            .unwrap();

        let surface_formats = device
            .physical_device()
            .surface_formats(&surface, SurfaceInfo::default())
            .unwrap();

        let (surface_format, _) = surface_formats
            .iter()
            .find(|(format, _)| format.numeric_format_color() == Some(NumericFormat::SRGB))
            .unwrap();

        log::info!("Using surface format: {surface_format:?}");

        let (swapchain, swapchain_images) = Swapchain::new(
            device.clone(),
            surface,
            SwapchainCreateInfo {
                min_image_count: surface_capabilities.min_image_count.max(2),
                image_extent: window_size.into(),
                image_usage: ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSFER_DST,
                image_format: *surface_format,
                present_mode: PresentMode::Immediate,
                composite_alpha: surface_capabilities
                    .supported_composite_alpha
                    .into_iter()
                    .next()
                    .unwrap(),
                ..Default::default()
            },
        )
        .unwrap();

        let shader = cs::load(device.clone()).unwrap();
        let stage = PipelineShaderStageCreateInfo::new(shader.entry_point("main").unwrap());
        let layout = PipelineLayout::new(
            device.clone(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages([&stage])
                .into_pipeline_layout_create_info(device.clone())
                .unwrap(),
        )
        .unwrap();

        let compute_pipeline = ComputePipeline::new(
            device.clone(),
            None,
            ComputePipelineCreateInfo::stage_layout(stage, layout),
        )
        .unwrap();

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
        let descriptor_set_allocator = Arc::new(StandardDescriptorSetAllocator::new(
            device.clone(),
            Default::default(),
        ));

        let output_image = Image::new(
            memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R32G32B32A32_SFLOAT,
                extent: [window_size.width, window_size.height, 1],
                usage: ImageUsage::STORAGE | ImageUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
        )
        .unwrap();
        let output_image_view = ImageView::new_default(output_image.clone()).unwrap();

        let layout = compute_pipeline.layout().set_layouts().first().unwrap();
        let set = DescriptorSet::new(
            descriptor_set_allocator.clone(),
            layout.clone(),
            [WriteDescriptorSet::image_view(0, output_image_view)],
            [],
        )
        .unwrap();

        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
            device.clone(),
            StandardCommandBufferAllocatorCreateInfo::default(),
        ));

        let command_buffers = swapchain_images
            .iter()
            .map(|image| {
                let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
                    command_buffer_allocator.clone(),
                    queue_family_index,
                    CommandBufferUsage::MultipleSubmit,
                )
                .unwrap();

                unsafe {
                    command_buffer_builder
                        .bind_pipeline_compute(compute_pipeline.clone())
                        .unwrap()
                        .bind_descriptor_sets(
                            PipelineBindPoint::Compute,
                            compute_pipeline.layout().clone(),
                            0,
                            set.clone(),
                        )
                        .unwrap()
                        .dispatch([window_size.width / 32, window_size.height / 32, 1])
                        .unwrap()
                        .blit_image(BlitImageInfo::images(output_image.clone(), image.clone()))
                        .unwrap();
                }

                command_buffer_builder.build().unwrap()
            })
            .collect();

        Self {
            device,
            queue,
            swapchain,
            command_buffers,
        }
    }

    pub fn update_camera(&mut self, _view: &Mat4, _projection: &Mat4) {}

    pub fn render(&mut self, _time: f32) -> u32 {
        let (image_index, _suboptimal, acquire_future) =
            swapchain::acquire_next_image(self.swapchain.clone(), None).unwrap();

        sync::now(self.device.clone())
            .join(acquire_future)
            .then_execute(
                self.queue.clone(),
                self.command_buffers[image_index as usize].clone(),
            )
            .unwrap()
            .then_swapchain_present(
                self.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_index),
            )
            .then_signal_fence_and_flush()
            .unwrap()
            .wait(None)
            .unwrap();

        0
    }
}
