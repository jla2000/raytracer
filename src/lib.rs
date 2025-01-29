use wasm_bindgen::prelude::*;

use std::sync::Arc;

use wgpu::{
    include_wgsl, util::TextureBlitter, Backends, BindGroup, BindGroupDescriptor, BindGroupEntry,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
    CommandEncoderDescriptor, CompositeAlphaMode, ComputePassDescriptor, ComputePipeline,
    ComputePipelineDescriptor, Device, DeviceDescriptor, Extent3d, Features, Instance,
    InstanceDescriptor, Limits, MemoryHints, PipelineLayoutDescriptor, PowerPreference,
    PresentMode, Queue, RequestAdapterOptions, ShaderStages, StorageTextureAccess, Surface,
    SurfaceConfiguration, SurfaceError, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension,
};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{self, ControlFlow, EventLoop},
    platform::web::WindowExtWebSys,
    window::{Window, WindowAttributes},
};

struct State {
    window: Arc<Window>,
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    pipeline: ComputePipeline,
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
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: None,
                    required_features: Features::default(),
                    required_limits: Limits::default(),
                    memory_hints: MemoryHints::default(),
                },
                None,
            )
            .await
            .unwrap();

        let window_size = window.inner_size();

        let surface_texture_format = surface
            .get_capabilities(&adapter)
            .formats
            .first()
            .copied()
            .unwrap();

        surface.configure(
            &device,
            &SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format: surface_texture_format,
                width: window_size.width,
                height: window_size.height,
                present_mode: PresentMode::Fifo,
                alpha_mode: CompositeAlphaMode::Auto,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            },
        );

        let shader_module = device.create_shader_module(include_wgsl!("render.wgsl"));

        let render_texture_format = TextureFormat::Rgba32Float;
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::StorageTexture {
                    access: StorageTextureAccess::WriteOnly,
                    format: render_texture_format,
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

        let texture = device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: window_size.width,
                height: window_size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: render_texture_format,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let render_texture_view = texture.create_view(&TextureViewDescriptor::default());

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&render_texture_view),
            }],
        });

        let blitter = TextureBlitter::new(&device, surface_texture_format);

        Self {
            window,
            surface,
            device,
            queue,
            pipeline,
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

        self.blitter.copy(
            &self.device,
            &mut encoder,
            &self.render_texture_view,
            &surface_view,
        );

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

struct App {
    state: State,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, _event_loop: &event_loop::ActiveEventLoop) {}

    fn window_event(
        &mut self,
        event_loop: &event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                self.state.render().unwrap();
                self.state.window.request_redraw();
            }
            _ => {}
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn wasm_main() {
    wasm_bindgen_futures::spawn_local(run());
}

async fn run() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Info).expect("Failed to initialize logger");

    let event_loop = EventLoop::builder().build().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let window = event_loop
        .create_window(
            WindowAttributes::default()
                .with_resizable(false)
                .with_title("raytracer"),
        )
        .unwrap();

    web_sys::window()
        .and_then(|win| win.document())
        .and_then(|doc| {
            let dst = doc.get_element_by_id("render-surface")?;
            let canvas = web_sys::Element::from(window.canvas()?);
            dst.append_child(&canvas).ok()?;
            Some(())
        })
        .expect("Couldn't append canvas to document body.");

    _ = window.request_inner_size(PhysicalSize::new(800, 600));

    event_loop
        .run_app(&mut App {
            state: State::new(Arc::new(window)).await,
        })
        .unwrap();
}
