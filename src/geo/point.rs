use glam::Vec2;

use std::fmt::{Debug, Display};

#[derive(Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Returns a vector `dir` such that `self + dir == other`.    
    pub fn dir(self, other: Point) -> Vec2 {
        other - self
    }

    pub fn dist(self, other: Point) -> f32 {
        (other - self).length()
    }

    pub fn dist_sq(self, other: Point) -> f32 {
        (other - self).length_squared()
    }
}

impl Debug for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Point")
            .field("x", &self.x)
            .field("y", &self.y)
            .finish()
    }
}

impl Display for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:2} {:2}]", self.x, self.y)
    }
}

impl Into<(f32, f32)> for Point {
    fn into(self) -> (f32, f32) {
        (self.x, self.y)
    }
}

impl<T: Into<f32>> From<(T, T)> for Point {
    fn from(value: (T, T)) -> Self {
        Self {
            x: value.0.into(),
            y: value.1.into(),
        }
    }
}

impl std::ops::Add<Vec2> for Point {
    type Output = Point;

    fn add(self, rhs: Vec2) -> Self::Output {
        Self::Output {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl std::ops::Sub<Vec2> for Point {
    type Output = Point;

    fn sub(self, rhs: Vec2) -> Self::Output {
        Self::Output {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl std::ops::Sub<Point> for Point {
    type Output = Vec2;

    fn sub(self, rhs: Point) -> Self::Output {
        Self::Output {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}
