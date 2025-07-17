mod point;
mod segment;
mod visibility;

pub use point::Point;
pub use segment::Segment;
pub use visibility::VisibilityPolygon;

const ERR: f32 = 1e-4;
