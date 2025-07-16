use crate::geo::Point;

#[derive(Clone, Debug)]
pub struct ToricGeometry {
    pub x: f32,
    pub y: f32,
}

impl ToricGeometry {
    pub fn wrap(&self, point: &mut Point) {
        if point.x < -self.x / 2.0 {
            point.x += self.x;
        } else if point.x > self.x / 2.0 {
            point.x -= self.x;
        }

        if point.y < -self.y / 2.0 {
            point.y += self.y;
        } else if point.y > self.y / 2.0 {
            point.y -= self.y;
        }
    }
}
