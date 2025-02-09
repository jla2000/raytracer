use std::f32::consts::PI;

use glam::{Mat4, Vec3};
use winit::dpi::PhysicalSize;

pub struct Camera {
    target: Vec3,
    radius: f32,
    yaw: f32,
    pitch: f32,
}

const MOVE_SENSITIVITY: f32 = 0.001;
const SCROLL_SENSITIVITY: f32 = 0.1;
const MIN_RADIUS: f32 = 0.1;
const MIN_PITCH: f32 = -PI / 2.0 + 0.01;
const MAX_PITCH: f32 = PI / 2.0 - 0.01;

impl Camera {
    pub fn new(target: Vec3, radius: f32) -> Self {
        Self {
            target,
            radius,
            yaw: 0.0,
            pitch: 0.0,
        }
    }

    pub fn update_angles(&mut self, delta_x: f32, delta_y: f32) {
        self.yaw += delta_x * MOVE_SENSITIVITY;
        self.pitch += delta_y * MOVE_SENSITIVITY;
        self.pitch = self.pitch.clamp(MIN_PITCH, MAX_PITCH);
    }

    pub fn zoom(&mut self, delta_radius: f32) {
        self.radius -= delta_radius * SCROLL_SENSITIVITY;
        self.radius = self.radius.max(MIN_RADIUS);
    }

    pub fn calculate_view(&self) -> Mat4 {
        let position = self.target
            + Vec3::new(
                self.radius * self.pitch.cos() * self.yaw.sin(),
                self.radius * self.pitch.sin(),
                self.radius * self.pitch.cos() * self.yaw.cos(),
            );

        Mat4::look_at_lh(position, self.target, Vec3::new(0.0, 1.0, 0.0))
    }

    pub fn calculate_projection(&self, window_size: &PhysicalSize<u32>) -> Mat4 {
        Mat4::perspective_lh(
            90.0f32.to_radians(),
            window_size.width as f32 / window_size.height as f32,
            0.1,
            100.0,
        )
    }
}
