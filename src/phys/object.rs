use glam::Vec2;

use crate::{geo::Point, phys::collision::VelocityAccumulator};

#[derive(Clone, Copy, Debug)]
pub enum SceneObjectShape {
    SegmentH { dx: f32 },
    SegmentV { dy: f32 },
    Segment { dx: f32, dy: f32 },
}

#[derive(Clone, Debug)]
pub struct SceneObject {
    pub center: Point,
    pub shape: SceneObjectShape,
}

impl SceneObject {
    pub fn new_segment_h(start: Point, dx: f32) -> Self {
        todo!()
    }

    pub fn new_segment_v(start: Point, dy: f32) -> Self {
        todo!()
    }

    pub fn new_segment(start: Point, end: Point) -> Self {
        Self {
            center: start.midpoint(end),
            shape: SceneObjectShape::Segment {
                dx: end.x - start.x,
                dy: end.y - start.y,
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum PhysObjectShape {
    Disc { radius: f32 },
}

#[derive(Clone, Debug)]
pub struct PhysObject<M> {
    /// Center of mass
    pub center: Point,
    pub shape: PhysObjectShape,

    /// Kilograms
    pub mass: f32,
    /// Meters per second
    pub velocity_linear: Vec2,
    /// Radians per second
    pub velocity_angular: f32,

    pub meta: M,

    pub velocity_acc: VelocityAccumulator,
}

impl PhysObject<()> {
    pub fn new_disc(center: Point, radius: f32, mass: f32) -> Self {
        Self {
            center,
            shape: PhysObjectShape::Disc { radius },
            mass,
            velocity_linear: Vec2::ZERO,
            velocity_angular: 0.0,

            meta: (),

            velocity_acc: Default::default(),
        }
    }
}

impl<M> PhysObject<M> {
    pub fn flush_acc(&mut self) {
        self.velocity_linear += self.velocity_acc.velocity_linear;
        self.velocity_angular += self.velocity_acc.velocity_angular;
        self.velocity_acc = Default::default();
    }

    pub fn accelerate(&mut self, direction: Vec2, seconds: f32) {
        self.velocity_linear += direction * seconds;
    }

    pub fn advance_by(&mut self, seconds: f32) {
        assert_eq!(self.velocity_acc.velocity_linear, Vec2::ZERO);
        assert_eq!(self.velocity_acc.velocity_angular, 0.0);
        self.center += self.velocity_linear * seconds;
    }

    pub fn with_velocity(mut self, velocity_linear: Vec2) -> Self {
        self.velocity_linear = velocity_linear;
        self
    }

    pub fn with_meta<N>(self, meta: N) -> PhysObject<N> {
        let Self {
            center,
            shape,
            mass,
            velocity_linear,
            velocity_angular,
            velocity_acc: collision_acc,
            ..
        } = self;

        PhysObject {
            center,
            shape,
            mass,
            velocity_linear,
            velocity_angular,
            meta,
            velocity_acc: collision_acc,
        }
    }

    pub fn map_meta<N>(self, meta_f: impl FnOnce(&Self) -> N) -> PhysObject<N> {
        let meta = meta_f(&self);

        let Self {
            center,
            shape,
            mass,
            velocity_linear,
            velocity_angular,
            velocity_acc: collision_acc,
            ..
        } = self;

        PhysObject {
            center,
            shape,
            mass,
            velocity_linear,
            velocity_angular,
            meta,
            velocity_acc: collision_acc,
        }
    }
}
