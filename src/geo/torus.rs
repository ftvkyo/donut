use crate::geo::Point;

#[derive(Clone, Debug)]
pub struct ToricGeometry {
    pub x: f32,
    pub y: f32,
}

impl ToricGeometry {
    /// The copy of `point` nearest to `from`.
    pub fn nearest(&self, from: Point, point: Point) -> Point {
        let mut dir = from.dir(point);
        dir.x -= self.x * (dir.x / self.x).round();
        dir.y -= self.y * (dir.y / self.y).round();
        from + dir
    }

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
