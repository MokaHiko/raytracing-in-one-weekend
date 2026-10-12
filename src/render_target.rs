use glam::IVec2;

use crate::color::Color;

pub struct RenderTarget {
    width: usize,
    height: usize,
    buffer: Vec<Color>,
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

    #[allow(dead_code)]
    pub fn extent(&self) -> IVec2 {
        IVec2 {
            x: self.width as i32,
            y: self.height as i32,
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
