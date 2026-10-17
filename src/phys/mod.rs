use std::time::Duration;

use glam::{Vec2, vec2};

use crate::{
    geo::ToricGeometry,
    phys::{
        collision::CollideWith,
        object::{PhysObject, SceneObject},
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
            let timestep = self.max_timestep.min(seconds);

            self.for_each_distinct_pair_mut(|obj1, obj2| {
                obj1.collide(obj2);
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
