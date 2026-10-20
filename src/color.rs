use ::glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color(pub Vec3);

impl Color {
    pub fn new(r: f32, g: f32, b: f32) -> Self {
        Self(Vec3::new(r, g, b))
    }
}
