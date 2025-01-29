use std::{f32::consts::PI, num::NonZero, sync::Arc};

use glam::{Mat4, Vec3};
use wgpu::{
    include_wgsl,
    util::{BufferInitDescriptor, DeviceExt},
    Backends, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BufferBindingType, BufferUsages,
    CommandEncoderDescriptor, CompositeAlphaMode, ComputePassDescriptor, ComputePipeline,
    ComputePipelineDescriptor, Device, DeviceDescriptor, Extent3d, Features, Instance,
    InstanceDescriptor, Limits, MemoryHints, PipelineLayoutDescriptor, PowerPreference,
    PresentMode, Queue, RequestAdapterOptions, ShaderStages, StorageTextureAccess, Surface,
    SurfaceConfiguration, SurfaceError, Texture, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages, TextureViewDescriptor, TextureViewDimension,
};
use winit::{dpi::PhysicalSize, window::Window};

pub struct Renderer {
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    pipeline: ComputePipeline,
    render_texture: Texture,
    bind_group: BindGroup,
    window_size: PhysicalSize<u32>,
}

impl Renderer {
    pub async fn new(window: Arc<Window>) -> Self {
        let instance = Instance::new(&InstanceDescriptor {
            backends: Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        println!(
            "INFO - Gpu: {}, Backend: {}",
            adapter.get_info().name,
            adapter.get_info().backend
        );

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: None,
                    required_features: Features::BGRA8UNORM_STORAGE
                        | Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                    required_limits: Limits::default(),
                    memory_hints: MemoryHints::default(),
                },
                None,
            )
            .await
            .unwrap();

        let window_size = window.inner_size();

        let texture_format = TextureFormat::Bgra8Unorm;
        surface.configure(
            &device,
            &SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_DST,
                format: texture_format,
                width: window_size.width,
                height: window_size.height,
                present_mode: PresentMode::Immediate,
                alpha_mode: CompositeAlphaMode::Auto,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            },
        );

        let shader_module = device.create_shader_module(include_wgsl!("shader.wgsl"));

        let inverse_proj = Mat4::perspective_infinite_reverse_rh(
            90.0 * PI / 180.0,
            window_size.width as f32 / window_size.height as f32,
            0.1,
        );

        let position = Vec3::new(0.0, 0.0, 0.0);
        let inverse_view =
            Mat4::look_to_rh(position, Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 1.0, 0.0));

        let mut camera_buffer = [0; 144];

        camera_buffer[0..64].copy_from_slice(bytemuck::bytes_of(&inverse_proj));
        camera_buffer[64..128].copy_from_slice(bytemuck::bytes_of(&inverse_view));
        camera_buffer[128..140].copy_from_slice(bytemuck::bytes_of(&position));

        let camera_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: &camera_buffer,
            usage: BufferUsages::UNIFORM,
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::ReadWrite,
                        format: texture_format,
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(NonZero::new(144).unwrap()),
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: Some("render"),
            compilation_options: Default::default(),
            cache: None,
        });

        let render_texture = device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: window_size.width,
                height: window_size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: texture_format,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let render_texture_view = render_texture.create_view(&TextureViewDescriptor::default());

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&render_texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Buffer(camera_buffer.as_entire_buffer_binding()),
                },
            ],
        });

        Self {
            surface,
            device,
            queue,
            pipeline,
            render_texture,
            bind_group,
            window_size,
        }
    }

    pub fn render(&mut self) -> Result<(), SurfaceError> {
        let surface_texture = self.surface.get_current_texture()?;

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });

        let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, &self.bind_group, &[]);
        compute_pass.dispatch_workgroups(
            self.window_size.width / 10,
            self.window_size.height / 10,
            1,
        );

        drop(compute_pass);

        encoder.copy_texture_to_texture(
            self.render_texture.as_image_copy(),
            surface_texture.texture.as_image_copy(),
            surface_texture.texture.size(),
        );

        self.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();

        Ok(())
    }
}
