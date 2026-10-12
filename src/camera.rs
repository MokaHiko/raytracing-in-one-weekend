use glam::Vec3;

use crate::{ray::Ray, render_target::Extent2D};

pub struct Camera {
    pub focal_length: f32,
    pub center: Vec3,
    pub viewport_height: f32,
}

impl Camera {
    /// Create a [`Ray`] from origin to fragment in an extent.
    pub fn view_point_to_ray(&self, x: i32, y: i32, extent: &Extent2D) -> Ray {
        let viewport_height = self.viewport_height;
        let viewport_width = viewport_height * extent.width as f32 / extent.height as f32;

        let viewport_u = Vec3::new(viewport_width, 0.0, 0.0);
        // -viewport_height to support y down
        let viewport_v = Vec3::new(0.0, -viewport_height, 0.0);

        // upper left = forward + half width + half height
        let viewport_upper_left =
            self.center - Vec3::new(0.0, 0.0, self.focal_length) - 0.5 * (viewport_u + viewport_v);

        let pixel_du = viewport_u / extent.width as f32;
        let pixel_dv = viewport_v / extent.height as f32;
        let pixel_00 = viewport_upper_left + 0.5 * (pixel_du + pixel_dv);

        let pixel = pixel_00 + x as f32 * pixel_du + y as f32 * pixel_dv;
        Ray {
            origin: self.center,
            direction: pixel - self.center,
        }
    }
}
