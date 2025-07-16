use glam::{Mat4, Vec2, Vec3};
use winit::dpi::{LogicalSize, PhysicalSize};

pub struct Camera {
    map_size: LogicalSize<u32>,
    position: Vec2,
}

impl Camera {
    const DISTANCE: f32 = 20.0;
    const NEAR: f32 = 1.0;

    pub fn new(position: Vec2, map_size: LogicalSize<u32>) -> Self {
        Self { map_size, position }
    }

    pub fn matrix_view(&self) -> Mat4 {
        let position = self.position.extend(Self::DISTANCE);
        Mat4::look_to_rh(position, Vec3::NEG_Z, Vec3::Y)
    }

    pub fn matrix_proj(&self, win_size: PhysicalSize<u32>) -> Mat4 {
        let (map_width, map_height) = (self.map_size.width as f32, self.map_size.height as f32);
        let map_aspect = map_width / map_height;
        let (win_width, win_height) = (win_size.width as f32, win_size.height as f32);
        let win_aspect = win_width / win_height;

        let (width, height) = if map_aspect >= win_aspect {
            // Map rect is wider than Window rect, or has the same aspect ratio.
            // Should show full map width, and should expand the height shown.
            (map_width, map_width / win_aspect)
        } else {
            // Map rect is narrower than Window rect.
            // Should show full map height, and should expand the width shown.
            (map_height * win_aspect, map_height)
        };

        Mat4::orthographic_rh(
            -width / 2.0,
            width / 2.0,
            -height / 2.0,
            height / 2.0,
            Self::NEAR,
            Self::DISTANCE * 2.0,
        )
    }
}
