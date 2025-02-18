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
use winit::{dpi::PhysicalSize, event_loop::ActiveEventLoop, window::Window};

pub struct Renderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    swapchain: Arc<Swapchain>,
    compute_command_buffer: Arc<PrimaryAutoCommandBuffer>,
    blit_command_buffers: Vec<Arc<PrimaryAutoCommandBuffer>>,
}

mod compute {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "assets/shaders/ray_tracing.comp",
    }
}

mod vertex {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "assets/shaders/quad.vert",
    }
}

mod fragment {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "assets/shaders/post_processing.frag",
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

        let shader = compute::load(device.clone()).unwrap();
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
                format: Format::R8G8B8A8_UNORM,
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
        let descriptor_set = DescriptorSet::new(
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

        let compute_command_buffer = build_compute_command_buffer(
            command_buffer_allocator.clone(),
            compute_pipeline.clone(),
            descriptor_set.clone(),
            queue_family_index,
            window_size,
        );

        let blit_command_buffers = swapchain_images
            .iter()
            .map(|image| {
                build_blit_command_buffer(
                    command_buffer_allocator.clone(),
                    output_image.clone(),
                    image.clone(),
                    queue_family_index,
                )
            })
            .collect();

        Self {
            device,
            queue,
            swapchain,
            compute_command_buffer,
            blit_command_buffers,
        }
    }

    pub fn update_camera(&mut self, _view: &Mat4, _projection: &Mat4) {}

    pub fn render(&mut self, _time: f32) -> u32 {
        let (image_index, _suboptimal, acquire_future) =
            swapchain::acquire_next_image(self.swapchain.clone(), None).unwrap();

        sync::now(self.device.clone())
            .join(acquire_future)
            .then_execute(self.queue.clone(), self.compute_command_buffer.clone())
            .unwrap()
            .then_execute(
                self.queue.clone(),
                self.blit_command_buffers[image_index as usize].clone(),
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

fn build_compute_command_buffer(
    allocator: Arc<StandardCommandBufferAllocator>,
    pipeline: Arc<ComputePipeline>,
    descriptor_set: Arc<DescriptorSet>,
    queue_family_index: u32,
    window_size: PhysicalSize<u32>,
) -> Arc<PrimaryAutoCommandBuffer> {
    let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
        allocator.clone(),
        queue_family_index,
        CommandBufferUsage::MultipleSubmit,
    )
    .unwrap();

    const WORKGROUP_SIZE: u32 = 32;

    let num_workgroups_x = (window_size.width as f32 / WORKGROUP_SIZE as f32).ceil() as u32;
    let num_workgroups_y = (window_size.height as f32 / WORKGROUP_SIZE as f32).ceil() as u32;

    command_buffer_builder
        .bind_pipeline_compute(pipeline.clone())
        .unwrap()
        .bind_descriptor_sets(
            PipelineBindPoint::Compute,
            pipeline.layout().clone(),
            0,
            descriptor_set,
        )
        .unwrap();

    unsafe { command_buffer_builder.dispatch([num_workgroups_x, num_workgroups_y, 1]) }.unwrap();

    command_buffer_builder.build().unwrap()
}

fn build_blit_command_buffer(
    allocator: Arc<StandardCommandBufferAllocator>,
    computed_image: Arc<Image>,
    swapchain_image: Arc<Image>,
    queue_family_index: u32,
) -> Arc<PrimaryAutoCommandBuffer> {
    let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
        allocator.clone(),
        queue_family_index,
        CommandBufferUsage::MultipleSubmit,
    )
    .unwrap();

    command_buffer_builder
        .blit_image(BlitImageInfo::images(computed_image, swapchain_image))
        .unwrap();

    command_buffer_builder.build().unwrap()
}
