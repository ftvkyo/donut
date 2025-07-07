use glam::Vec2;

use std::fmt::{Debug, Display};

#[derive(Clone, Copy, PartialEq)]
pub struct Point(Vec2);

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self(Vec2 { x, y })
    }

    // Returns a vector `dir` such that `self + dir == other`.
    pub fn dir(&self, other: Point) -> Vec2 {
        other.0 - self.0
    }
}

impl Debug for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Point")
            .field("x", &self.0.x)
            .field("y", &self.0.y)
            .finish()
    }
}

impl Display for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:2} {:2}]", self.0.x, self.0.y)
    }
}

impl Into<(f32, f32)> for Point {
    fn into(self) -> (f32, f32) {
        (self.0.x, self.0.y)
    }
}

impl Into<Vec2> for Point {
    fn into(self) -> Vec2 {
        self.0
    }
}

impl From<Vec2> for Point {
    fn from(value: Vec2) -> Self {
        Self(value)
    }
}

impl From<(f32, f32)> for Point {
    fn from(value: (f32, f32)) -> Self {
        Self(value.into())
    }
}

impl From<(i32, i32)> for Point {
    fn from(value: (i32, i32)) -> Self {
        Self((value.0 as f32, value.1 as f32).into())
    }
}

impl std::ops::Deref for Point {
    type Target = Vec2;

    fn deref(&self) -> &Self::Target {
        return &self.0;
    }
}

impl std::ops::Add<Vec2> for Point {
    type Output = Point;

    fn add(self, rhs: Vec2) -> Self::Output {
        Point(self.0 + rhs)
    }
}

impl std::ops::Sub<Vec2> for Point {
    type Output = Point;

    fn sub(self, rhs: Vec2) -> Self::Output {
        Point(self.0 - rhs)
    }
}
