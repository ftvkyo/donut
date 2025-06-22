use glam::{Mat4, Vec2, Vec3};

pub struct Camera {
    pub position: Vec2,
}

impl Camera {
    const FOV: f32 = std::f32::consts::FRAC_PI_2;
    const DISTANCE: f32 = 4.0; // with PI/2, this means we see 4 tiles up and 4 tiles down
    const NEAR: f32 = 1.0;
    const FAR: f32 = 10.0;

    pub fn new(position: Vec2) -> Self {
        Self { position }
    }

    pub fn matrix_view(&self) -> Mat4 {
        let position = self.position.extend(Self::DISTANCE);
        Mat4::look_to_rh(position, Vec3::NEG_Z, Vec3::Y)
    }

    pub fn matrix_proj(&self, aspect_ratio: f32) -> Mat4 {
        Mat4::perspective_rh(Self::FOV, aspect_ratio, Self::NEAR, Self::FAR)
    }
}
