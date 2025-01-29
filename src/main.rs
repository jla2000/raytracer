use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use pollster::block_on;
use wgpu::{
    include_wgsl, util::TextureBlitter, Backends, BindGroup, BindGroupDescriptor, BindGroupEntry,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
    CommandEncoderDescriptor, CompositeAlphaMode, ComputePassDescriptor, ComputePipeline,
    ComputePipelineDescriptor, Device, DeviceDescriptor, Extent3d, Features, Instance,
    InstanceDescriptor, Limits, MemoryHints, PipelineLayoutDescriptor, PowerPreference,
    PresentMode, Queue, RequestAdapterOptions, ShaderStages, StorageTextureAccess, Surface,
    SurfaceConfiguration, SurfaceError, Texture, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension,
};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{self, ControlFlow, EventLoop},
    window::{Window, WindowAttributes},
};

struct State {
    window: Arc<Window>,
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    pipeline: ComputePipeline,
    render_texture: Texture,
    render_texture_view: TextureView,
    bind_group: BindGroup,
    blitter: TextureBlitter,
    window_size: PhysicalSize<u32>,
}

impl State {
    async fn new(window: Arc<Window>) -> Self {
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

        let shader_module = device.create_shader_module(include_wgsl!("render.wgsl"));

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::StorageTexture {
                    access: StorageTextureAccess::ReadWrite,
                    format: texture_format,
                    view_dimension: TextureViewDimension::D2,
                },
                count: None,
            }],
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
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&render_texture_view),
            }],
        });

        let blitter = TextureBlitter::new(&device, texture_format);

        Self {
            window,
            surface,
            device,
            queue,
            pipeline,
            render_texture,
            render_texture_view,
            bind_group,
            blitter,
            window_size,
        }
    }

    fn render(&mut self) -> Result<(), SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let surface_view = output
            .texture
            .create_view(&TextureViewDescriptor::default());

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
            output.texture.as_image_copy(),
            output.texture.size(),
        );

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

struct App {
    state: Option<State>,
    last_frame: Instant,
    num_frames: usize,
}

impl Default for App {
    fn default() -> Self {
        Self {
            state: None,
            last_frame: Instant::now(),
            num_frames: 0,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &event_loop::ActiveEventLoop) {
        let window = event_loop
            .create_window(
                WindowAttributes::default()
                    .with_resizable(false)
                    .with_title("raytracer"),
            )
            .unwrap();

        self.state = Some(block_on(State::new(Arc::new(window))));
    }

    fn window_event(
        &mut self,
        event_loop: &event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                if let Some(state) = &mut self.state {
                    state.render().unwrap();
                    state.window.request_redraw();

                    self.num_frames += 1;

                    let now = Instant::now();
                    if now - self.last_frame > Duration::from_secs(1) {
                        println!("Fps: {}", self.num_frames);
                        self.last_frame = now;
                        self.num_frames = 0;
                    }
                }
            }
            _ => {}
        }
    }
}

fn main() {
    struct Logger;
    impl log::Log for Logger {
        fn enabled(&self, metadata: &log::Metadata) -> bool {
            metadata.level() <= log::Level::Info
        }

        fn log(&self, record: &log::Record) {
            if self.enabled(record.metadata()) {
                println!("{} - {}", record.level(), record.args());
            }
        }

        fn flush(&self) {}
    }
    log::set_boxed_logger(Box::new(Logger))
        .map(|_| log::set_max_level(log::LevelFilter::Info))
        .unwrap();

    let event_loop = EventLoop::new().unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut App::default()).unwrap();
}
