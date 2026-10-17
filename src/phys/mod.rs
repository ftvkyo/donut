use std::time::Duration;

use glam::{Vec2, vec2};

use crate::{
    geo::ToricGeometry,
    phys::{
        collision::CollideWith,
        object::{PhysObject, PhysObjectShape, SceneObject},
    },
};

pub mod collision;
pub mod object;
#[cfg(test)]
mod tests;

pub struct Physics<M> {
    objects: Vec<PhysObject<M>>,
    max_timestep: f32,
    geometry: ToricGeometry,
}

impl<M> Physics<M> {
    const G: Vec2 = vec2(0.0, -3.0);

    pub fn new(max_timestep: f32, geometry: ToricGeometry) -> Self {
        Self {
            objects: Vec::new(),
            max_timestep,
            geometry,
        }
    }

    pub fn advance_by(&mut self, scene: &[SceneObject], time: Duration) {
        let mut seconds = time.as_secs_f32();

        while seconds > 0.0 {
            // Objects move about half their radius per step at most, so that they cannot pass
            // through segments
            let timestep = self
                .objects
                .iter()
                .fold(self.max_timestep.min(seconds), |step, obj| {
                    let PhysObjectShape::Disc { radius } = obj.shape;
                    step.min(radius / 2.0 / obj.velocity_linear.length())
                });

            let geometry = self.geometry.clone();
            self.for_each_distinct_pair_mut(|obj1, obj2| {
                // Collide with the copy of `obj2` across the wrap that is nearest to `obj1`
                let offset = obj2.center.dir(geometry.nearest(obj1.center, obj2.center));
                obj2.center += offset;
                obj1.collide(&mut *obj2);
                obj2.center -= offset;
            });

            self.for_each_mut(|obj| {
                obj.flush_acc();
                for i in 0..scene.len() {
                    obj.collide(&scene[i]);
                }
            });

            for obj in self.objects.iter_mut() {
                obj.accelerate(Self::G, timestep);
                obj.advance_by(timestep);
                self.geometry.wrap(&mut obj.center);
            }

            seconds -= timestep;
        }
    }

    pub fn add(&mut self, obj: PhysObject<M>) {
        self.objects.push(obj);
    }

    pub fn objects(&self) -> &[PhysObject<M>] {
        &self.objects
    }

    fn for_each_mut(&mut self, mut f: impl FnMut(&mut PhysObject<M>)) {
        for i in 0..self.objects.len() {
            f(&mut self.objects[i]);
        }
    }

    fn for_each_distinct_pair_mut(
        &mut self,
        mut f: impl FnMut(&mut PhysObject<M>, &mut PhysObject<M>),
    ) {
        for i in 0..self.objects.len() {
            let (left, right) = self.objects.split_at_mut(i + 1);
            for j in 0..right.len() {
                f(&mut left[i], &mut right[j]);
            }
        }
    }
}
