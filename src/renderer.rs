use std::{num::NonZero, sync::Arc};

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::{
    hal::AccelerationStructureGeometryFlags,
    include_wgsl,
    util::{BufferInitDescriptor, DeviceExt},
    AccelerationStructureFlags, AccelerationStructureUpdateMode, Backends, BindGroup,
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, BlasBuildEntry, BlasGeometries, BlasGeometrySizeDescriptors,
    BlasTriangleGeometry, BlasTriangleGeometrySizeDescriptor, Buffer, BufferBindingType,
    BufferDescriptor, BufferUsages, CommandEncoderDescriptor, CompositeAlphaMode,
    ComputePassDescriptor, ComputePipeline, ComputePipelineDescriptor, CreateBlasDescriptor,
    CreateTlasDescriptor, Device, DeviceDescriptor, Extent3d, Features, Instance,
    InstanceDescriptor, Limits, MemoryHints, PipelineLayoutDescriptor, PowerPreference,
    PresentMode, PushConstantRange, Queue, RequestAdapterOptions, ShaderStages,
    StorageTextureAccess, Surface, SurfaceConfiguration, SurfaceError, Texture, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureViewDescriptor, TextureViewDimension,
    TlasInstance, TlasPackage, VertexFormat,
};
use winit::{dpi::PhysicalSize, window::Window};

const CAMERA_BUFFER_SIZE: usize = 128;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct CameraMatrices {
    inverse_proj: Mat4,
    inverse_view: Mat4,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct PushConstants {
    time: f32,
    num_samples: u32,
}

pub struct Renderer {
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    pipeline: ComputePipeline,
    render_texture: Texture,
    bind_group: BindGroup,
    camera_buffer: Buffer,
    window_size: PhysicalSize<u32>,
    num_samples: u32,
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
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        log::info!(
            "Gpu: {}, Backend: {}",
            adapter.get_info().name,
            adapter.get_info().backend
        );

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: None,
                    required_features: Features::BGRA8UNORM_STORAGE
                        | Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                        | Features::PUSH_CONSTANTS
                        | Features::EXPERIMENTAL_RAY_TRACING_ACCELERATION_STRUCTURE
                        | Features::EXPERIMENTAL_RAY_QUERY,
                    required_limits: Limits {
                        max_push_constant_size: size_of::<PushConstants>() as u32,
                        ..Default::default()
                    },
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

        let camera_buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            size: CAMERA_BUFFER_SIZE as u64,
            mapped_at_creation: false,
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
                        min_binding_size: Some(NonZero::new(CAMERA_BUFFER_SIZE as u64).unwrap()),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::AccelerationStructure,
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::COMPUTE,
                range: 0..(size_of::<PushConstants>() as u32),
            }],
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

        let tlas = device.create_tlas(&CreateTlasDescriptor {
            label: None,
            max_instances: 1,
            flags: AccelerationStructureFlags::PREFER_FAST_TRACE,
            update_mode: AccelerationStructureUpdateMode::Build,
        });

        let geometry_size = BlasTriangleGeometrySizeDescriptor {
            vertex_format: VertexFormat::Float32x3,
            vertex_count: 3,
            index_format: None,
            index_count: None,
            flags: AccelerationStructureGeometryFlags::OPAQUE,
        };
        let blas = device.create_blas(
            &CreateBlasDescriptor {
                label: None,
                flags: AccelerationStructureFlags::PREFER_FAST_TRACE,
                update_mode: AccelerationStructureUpdateMode::Build,
            },
            BlasGeometrySizeDescriptors::Triangles {
                descriptors: vec![geometry_size.clone()],
            },
        );

        let tlas_package = TlasPackage::new_with_instances(
            tlas,
            vec![Some(TlasInstance::new(
                &blas,
                [
                    1.0, 0.0, 0.0, 0.0, //
                    0.0, 1.0, 0.0, 0.0, //
                    0.0, 0.0, 1.0, 0.0, //
                ],
                0,
                1,
            ))],
        );

        #[derive(Copy, Clone, Pod, Zeroable)]
        #[repr(C)]
        struct Triangle {
            v0: Vec3,
            v1: Vec3,
            v2: Vec3,
        }

        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("vertex buffer"),
            contents: bytemuck::bytes_of(&Triangle {
                v0: Vec3::new(-0.5, 0.0, -1.0),
                v1: Vec3::new(-0.5, 1.0, -1.0),
                v2: Vec3::new(0.5, 0.0, -1.0),
            }),
            usage: BufferUsages::BLAS_INPUT,
        });

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor { label: None });
        encoder.build_acceleration_structures(
            std::iter::once(&BlasBuildEntry {
                blas: &blas,
                geometry: BlasGeometries::TriangleGeometries(vec![BlasTriangleGeometry {
                    size: &geometry_size,
                    vertex_buffer: &vertex_buffer,
                    first_vertex: 0,
                    vertex_stride: 3,
                    index_buffer: None,
                    first_index: None,
                    transform_buffer: None,
                    transform_buffer_offset: None,
                }]),
            }),
            std::iter::once(&tlas_package),
        );
        queue.submit(std::iter::once(encoder.finish()));

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
                BindGroupEntry {
                    binding: 2,
                    resource: tlas_package.as_binding(),
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
            camera_buffer,
            window_size,
            num_samples: 0,
        }
    }

    pub fn update_camera(&mut self, view: &Mat4, projection: &Mat4) {
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::bytes_of(&CameraMatrices {
                inverse_view: view.inverse(),
                inverse_proj: projection.inverse(),
            }),
        );
        self.num_samples = 0;
    }

    pub fn render(&mut self, time: f32) -> Result<u32, SurfaceError> {
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
        compute_pass.set_push_constants(
            0,
            bytemuck::bytes_of(&PushConstants {
                time,
                num_samples: self.num_samples,
            }),
        );
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

        self.num_samples += 1;

        Ok(self.num_samples)
    }
}
