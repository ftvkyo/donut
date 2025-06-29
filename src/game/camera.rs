use glam::{Mat4, Vec2, Vec3};

pub struct Camera {
    map_size: [u32; 2],
    position: Vec2,
}

impl Camera {
    const DISTANCE: f32 = 20.0;
    const NEAR: f32 = 1.0;

    pub fn new(position: Vec2, map_size: [u32; 2]) -> Self {
        Self {
            map_size,
            position,
        }
    }

    pub fn matrix_view(&self) -> Mat4 {
        let position = self.position.extend(Self::DISTANCE);
        Mat4::look_to_rh(position, Vec3::NEG_Z, Vec3::Y)
    }

    pub fn matrix_proj(&self, aspect_ratio: f32) -> Mat4 {
        let w = self.map_size[0] as f32;
        let h = self.map_size[1] as f32;
        // TODO: make sure the level fits when aspect ratio is different
        Mat4::orthographic_rh(
            -w / 2.0,
            w / 2.0,
            -h / 2.0,
            h / 2.0,
            Self::NEAR,
            Self::DISTANCE * 2.0,
        )
    }
}
