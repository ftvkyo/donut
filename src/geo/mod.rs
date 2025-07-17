mod point;
mod segment;
mod visibility;

pub use point::Point;
pub use segment::Segment;
pub use visibility::VisibilityPolygon;

pub trait ImpreciseEq: Sized {
    const E: f32 = 1e-4;
    const ZERO: Self;

    fn is_basically_zero(&self) -> bool {
        self.is_basically_equal(&Self::ZERO)
    }
    
    fn is_basically_equal(&self, other: &Self) -> bool;
}

impl ImpreciseEq for f32 {
    const ZERO: Self = 0.0;

    fn is_basically_equal(&self, other: &Self) -> bool {
        (self - other).abs() <= Self::E
    }
}

impl ImpreciseEq for Point {
    const ZERO: Self = Self { x: 0.0, y: 0.0 };

    fn is_basically_equal(&self, other: &Self) -> bool {
        self.x.is_basically_equal(&other.x) && self.y.is_basically_equal(&other.y)
    }
}
