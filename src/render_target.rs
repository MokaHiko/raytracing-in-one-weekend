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

    pub fn fragments_mut(&mut self) -> &mut [Color] {
        self.buffer.as_mut_slice()
    }
}
