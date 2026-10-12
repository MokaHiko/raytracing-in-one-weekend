use glam::IVec2;

use crate::color::Color;

pub struct RenderTarget {
    width: usize,
    height: usize,
    buffer: Vec<Color>,
}

pub struct Extent2D {
    pub width: usize,
    pub height: usize,
}

pub struct FragMut<'a> {
    pub color: &'a mut Color,
    pub coord: IVec2,
}

impl RenderTarget {
    pub fn new(width: usize, height: usize) -> Self {
        assert!(width * height != 0);
        Self {
            width,
            height,
            buffer: vec![Color::new(0.0, 0.0, 0.0); width * height],
        }
    }

    pub fn extent2d(&self) -> Extent2D {
        Extent2D {
            width: self.width,
            height: self.height,
        }
    }

    pub fn colors(&self) -> &[Color] {
        self.buffer.as_slice()
    }

    pub fn fragments_mut(&mut self) -> impl Iterator<Item = FragMut<'_>> {
        self.buffer.iter_mut().enumerate().map(|(i, color)| {
            let coord = IVec2::new((i % self.width) as i32, (i / self.width) as i32);
            FragMut { color, coord }
        })
    }
}
