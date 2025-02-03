use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use glam::{Mat4, Vec3};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{self, ControlFlow, EventLoop},
    window::{Window, WindowAttributes},
};

mod renderer;
use renderer::*;

struct App {
    state: Option<(Arc<Window>, Renderer)>,
    counter: FpsCounter,
    time_since_start: Instant,
}

struct FpsCounter {
    last_frame: Instant,
    num_frames: usize,
}

impl Default for FpsCounter {
    fn default() -> Self {
        Self {
            num_frames: 0,
            last_frame: Instant::now(),
        }
    }
}

impl FpsCounter {
    pub fn get_fps(&mut self) -> Option<usize> {
        let now = Instant::now();
        self.num_frames += 1;

        if now - self.last_frame > Duration::from_secs(1) {
            let fps = self.num_frames;
            self.num_frames = 0;
            self.last_frame = now;

            Some(fps)
        } else {
            None
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &event_loop::ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    WindowAttributes::default()
                        .with_inner_size(PhysicalSize::new(1280, 800))
                        .with_resizable(false)
                        .with_title("raytracer"),
                )
                .unwrap(),
        );
        let window_size = window.inner_size();

        let mut renderer = pollster::block_on(Renderer::new(window.clone()));

        let position = Vec3::new(0.0, 0.0, 0.0);
        let projection = Mat4::perspective_lh(
            90.0f32.to_radians(),
            window_size.width as f32 / window_size.height as f32,
            0.1,
            100.0,
        );
        let view = Mat4::look_to_lh(position, Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 1.0, 0.0));

        renderer.update_camera(&view, &projection);

        self.state = Some((window, renderer));
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
                if let Some((window, renderer)) = &mut self.state {
                    let num_samples = renderer
                        .render(self.time_since_start.elapsed().as_secs_f32())
                        .unwrap();
                    window.request_redraw();

                    if let Some(fps) = self.counter.get_fps() {
                        window
                            .set_title(&format!("raytracer - FPS: {fps}, Samples: {num_samples}"));
                    }
                }
            }
            _ => {}
        }
    }
}

fn main() {
    env_logger::init_from_env(env_logger::Env::default().filter_or("RUST_LOG", "info"));

    let event_loop = EventLoop::new().unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop
        .run_app(&mut App {
            state: None,
            counter: FpsCounter::default(),
            time_since_start: Instant::now(),
        })
        .unwrap();
}
