//! Based on https://en.wikipedia.org/wiki/Collision_response#Impulse-based_reaction_model

use glam::{Vec2, vec2};
use log::trace;

use crate::phys::object::{PhysObject, PhysObjectShape, SceneObject, SceneObjectShape};

#[derive(Clone, Debug, Default)]
pub struct VelocityAccumulator {
    pub velocity_linear: Vec2,
    pub velocity_angular: f32,
}

pub trait CollideWith<Obj> {
    fn collide(&mut self, with: Obj);
}

impl<M> CollideWith<&SceneObject> for PhysObject<M> {
    fn collide(&mut self, with: &SceneObject) {
        use PhysObjectShape::*;
        use SceneObjectShape::*;

        match (self.shape, with.shape) {
            (Disc { radius }, SegmentH { dx }) => todo!(),
            (Disc { radius }, SegmentV { dy }) => todo!(),

            (Disc { radius }, Segment { dx, dy }) => {
                let (sa, sb) = (
                    with.center - vec2(dx, dy) / 2.0,
                    with.center + vec2(dx, dy) / 2.0,
                );

                trace!(
                    "Colliding Disc {{@{}, r={radius}}} with Segment {{{sa}<=>{sb}}}",
                    self.center
                );

                // Solve a quadratic equation.
                // Its roots `t_in` and `t_out` are parameters that specify points,
                // where the edge of the disc and the line defined by the segment intersect with each other,
                // (t = 0 is segment start, and t = 1 is segment end).
                //
                // Based on: https://stackoverflow.com/questions/1073336/circle-line-segment-collision-detection-algorithm

                let a_b = sa.dir(sb);
                let c_a = self.center.dir(sa);

                let ma = a_b.dot(a_b);
                let mb = 2.0 * c_a.dot(a_b);
                let mc = c_a.dot(c_a) - radius * radius;

                let discr = mb * mb - 4.0 * ma * mc;

                let t_start = 0.0;
                let t_end = 1.0;

                if discr < 0.0 {
                    trace!(" -> The line defined by the segment does not intersect the disc");
                    return;
                } else {
                    trace!(
                        " -> The line defined by the segment is a secant or a tangent to the disc"
                    );

                    let mut collision_for = |t: f32| {
                        let location = sa.lerp(sb, t);

                        // Moving towards the segment
                        if self.velocity_linear.dot(self.center.dir(location)) > 0.0 {
                            let momentum = self.velocity_linear * self.mass;
                            let impulse = momentum.project_onto(self.center.dir(location));

                            // Dampen the impulse a bit
                            let impulse = impulse * 0.95;

                            self.velocity_linear -= 2.0 * impulse / self.mass;
                        }

                        let overlap = radius - self.center.dir(location).length();
                        self.center += location.dir(self.center).clamp_length(overlap, overlap);
                    };

                    // This represents the midpoint of the chord
                    let t_halfway = -mb / (2.0 * ma);
                    trace!(" -> The midpoint of the chord is at t={t_halfway}");

                    if t_start <= t_halfway && t_halfway <= t_end {
                        trace!(" -> The disc hit the \"inner\" part of the segment");
                        collision_for(t_halfway);
                        return;
                    }

                    let discr_rt = discr.sqrt();
                    let t_in = t_halfway - discr_rt / (2.0 * ma);
                    let t_out = t_halfway + discr_rt / (2.0 * ma);

                    if t_halfway <= t_start && t_start <= t_out {
                        trace!(" -> The disc hit the start of the segment");
                        collision_for(t_start);
                        return;
                    }

                    if t_in <= t_end && t_end <= t_halfway {
                        trace!(" -> The disc hit the end of the segment");
                        collision_for(t_end);
                        return;
                    }
                }
            }
        }
    }
}

impl<M1, M2> CollideWith<&mut PhysObject<M2>> for PhysObject<M1> {
    fn collide(&mut self, with: &mut PhysObject<M2>) {
        use PhysObjectShape::*;

        match (self.shape, with.shape) {
            (Disc { radius: r1 }, Disc { radius: r2 }) => {
                let r_sq = (r1 + r2) * (r1 + r2);
                let d_sq = self.center.dist_sq(with.center);
                if d_sq <= r_sq {
                    // There is a collision.

                    // TODO: impulse should take angular velocity into account

                    let location = self.center.lerp(with.center, r1 / (r1 + r2));

                    // Pretend that `with` is not moving
                    let vel_lin = self.velocity_linear - with.velocity_linear;

                    if vel_lin.dot(self.center.dir(location)) > 0.0 {
                        // The objects are moving towards each other

                        let momentum_self = self.velocity_linear * self.mass;
                        let momentum_with = with.velocity_linear * with.mass;

                        let impulse_self = momentum_self.project_onto(self.center.dir(location));
                        let impulse_with = momentum_with.project_onto(with.center.dir(location));

                        self.velocity_acc.velocity_linear -= impulse_self / self.mass;
                        self.velocity_acc.velocity_linear += impulse_with / self.mass;

                        with.velocity_acc.velocity_linear -= impulse_with / with.mass;
                        with.velocity_acc.velocity_linear += impulse_self / with.mass;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::init_logging;

    use super::*;

    #[test]
    fn interaction_two_discs() {
        init_logging();

        fn interact(obj1: &mut PhysObject<()>, obj2: &mut PhysObject<()>) {
            obj1.collide(&mut *obj2);

            obj1.flush_acc();
            obj2.flush_acc();

            let time = 1.0;

            obj1.advance_by(time);
            obj2.advance_by(time);
        }

        {
            let mut d1 = PhysObject::new_disc((0.0, 0.0).into(), 1.0, 1.0);

            let mut d2 = PhysObject::new_disc((2.0, 0.0).into(), 1.0, 1.0);

            interact(&mut d1, &mut d2);

            assert_eq!(d1.center, (0.0, 0.0).into());
            assert_eq!(d1.velocity_linear, vec2(0.0, 0.0));

            assert_eq!(d2.center, (2.0, 0.0).into());
            assert_eq!(d2.velocity_linear, vec2(0.0, 0.0));
        }

        {
            let mut d1 =
                PhysObject::new_disc((0.0, 0.0).into(), 1.0, 1.0).with_velocity(vec2(1.0, 0.0));

            let mut d2 = PhysObject::new_disc((2.0, 0.0).into(), 1.0, 1.0);

            interact(&mut d1, &mut d2);

            assert_eq!(d1.center, (0.0, 0.0).into());
            assert_eq!(d1.velocity_linear, vec2(0.0, 0.0));

            assert_eq!(d2.center, (3.0, 0.0).into());
            assert_eq!(d2.velocity_linear, vec2(1.0, 0.0));
        }

        {
            let mut d1 =
                PhysObject::new_disc((0.0, 0.0).into(), 1.0, 1.0).with_velocity(vec2(-1.0, 0.0));

            let mut d2 =
                PhysObject::new_disc((2.0, 0.0).into(), 1.0, 1.0).with_velocity(vec2(1.0, 0.0));

            interact(&mut d1, &mut d2);

            assert_eq!(d1.center, (-1.0, 0.0).into());
            assert_eq!(d1.velocity_linear, vec2(-1.0, 0.0));

            assert_eq!(d2.center, (3.0, 0.0).into());
            assert_eq!(d2.velocity_linear, vec2(1.0, 0.0));
        }
    }

    #[test]
    fn interaction_disc_segment() {
        init_logging();

        fn interact(obj1: &mut PhysObject<()>, obj2: &SceneObject) {
            obj1.collide(obj2);

            let time = 1.0;

            obj1.advance_by(time);
        }

        {
            let mut d = PhysObject::new_disc((0.0, 1.0).into(), 1.0, 1.0);

            let mut s = SceneObject::new_segment((-1.0, 0.0).into(), (1.0, 0.0).into());

            interact(&mut d, &mut s);

            assert_eq!(d.center, (0.0, 1.0).into());
            assert_eq!(d.velocity_linear, vec2(0.0, 0.0));
        }

        {
            let mut d =
                PhysObject::new_disc((0.0, 1.0).into(), 1.0, 1.0).with_velocity(vec2(0.0, -1.0));

            let mut s = SceneObject::new_segment((-1.0, 0.0).into(), (1.0, 0.0).into());

            interact(&mut d, &mut s);

            assert_eq!(d.center, (0.0, 1.9).into());
            assert_eq!(d.velocity_linear, vec2(0.0, 0.9));
        }
    }
}
