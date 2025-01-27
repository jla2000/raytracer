use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{self, EventLoop},
    window::{Window, WindowAttributes},
};

#[derive(Default)]
struct App {
    window: Option<Window>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &event_loop::ActiveEventLoop) {
        self.window = Some(
            event_loop
                .create_window(WindowAttributes::default())
                .unwrap(),
        );
    }

    fn window_event(
        &mut self,
        event_loop: &event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            _ => {}
        }
    }
}

fn main() {
    env_logger::try_init().unwrap();

    let event_loop = EventLoop::new().unwrap();
    event_loop.run_app(&mut App::default()).unwrap();
}
