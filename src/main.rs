use std::{
    sync::Arc,
    time::{Duration, Instant},
};

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
        let window = Arc::new(
            event_loop
                .create_window(
                    WindowAttributes::default()
                        .with_inner_size(PhysicalSize::new(1920, 1080))
                        .with_resizable(false)
                        .with_title("raytracer"),
                )
                .unwrap(),
        );

        self.state = Some((window.clone(), pollster::block_on(Renderer::new(window))));
        self.last_frame = Instant::now();
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
                    renderer.render().unwrap();
                    window.request_redraw();

                    self.num_frames += 1;

                    let now = Instant::now();
                    if now - self.last_frame > Duration::from_secs(1) {
                        window.set_title(&format!("raytracer - FPS: {}", self.num_frames));
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
        .map(|_| log::set_max_level(log::LevelFilter::Warn))
        .unwrap();

    let event_loop = EventLoop::new().unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut App::default()).unwrap();
}
