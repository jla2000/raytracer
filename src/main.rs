use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use camera::Camera;
use glam::{Mat4, Vec3};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{self, ControlFlow, EventLoop},
    window::{Window, WindowAttributes},
};

mod renderer;
use renderer::*;

mod camera;
mod model;

struct App {
    state: Option<(Arc<Window>, Renderer, Camera, MouseDrag)>,
    counter: FpsCounter,
    time_since_start: Instant,
}

struct FpsCounter {
    last_frame: Instant,
    num_frames: usize,
}

struct MouseDrag {
    is_dragging: bool,
    last_x_position: f32,
    last_y_position: f32,
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
                        .with_inner_size(PhysicalSize::new(1920, 1080))
                        .with_resizable(false)
                        .with_title("raytracer"),
                )
                .unwrap(),
        );
        window.focus_window();

        let window_size = window.inner_size();

        let mut renderer = pollster::block_on(Renderer::new(window.clone()));
        let camera = Camera::new(Vec3::ZERO, 3.0);

        renderer.update_camera(
            &camera.calculate_view(),
            &camera.calculate_projection(&window_size),
        );

        let mouse_drag = MouseDrag {
            is_dragging: false,
            last_x_position: 0.0,
            last_y_position: 0.0,
        };

        self.state = Some((window, renderer, camera, mouse_drag));
    }

    fn window_event(
        &mut self,
        event_loop: &event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if let Some((window, renderer, camera, mouse_drag)) = &mut self.state {
            match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::RedrawRequested => {
                    let num_samples = renderer
                        .render(self.time_since_start.elapsed().as_secs_f32())
                        .unwrap();
                    window.request_redraw();

                    if let Some(fps) = self.counter.get_fps() {
                        window
                            .set_title(&format!("raytracer - FPS: {fps}, Samples: {num_samples}"));
                    }
                }
                WindowEvent::MouseInput {
                    state,
                    button: MouseButton::Left,
                    ..
                } => {
                    mouse_drag.is_dragging = state == ElementState::Pressed;
                }
                WindowEvent::CursorMoved { position, .. } => {
                    if mouse_drag.is_dragging {
                        let delta_x = position.x as f32 - mouse_drag.last_x_position;
                        let delta_y = position.y as f32 - mouse_drag.last_y_position;
                        camera.update_angles(delta_x, delta_y);

                        let window_size = window.inner_size();
                        renderer.update_camera(
                            &camera.calculate_view(),
                            &camera.calculate_projection(&window_size),
                        );
                    }
                    mouse_drag.last_x_position = position.x as f32;
                    mouse_drag.last_y_position = position.y as f32;
                }
                WindowEvent::MouseWheel {
                    delta: MouseScrollDelta::LineDelta(_, scroll_y),
                    ..
                } => {
                    camera.zoom(scroll_y);
                    let window_size = window.inner_size();
                    renderer.update_camera(
                        &camera.calculate_view(),
                        &camera.calculate_projection(&window_size),
                    );
                }
                _ => {}
            }
        }
    }
}

fn main() {
    env_logger::init_from_env(env_logger::Env::default().filter_or("RUST_LOG", "wgpu=error,info"));

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
