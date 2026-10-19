use glam::Vec3;

use crate::{ray::Ray, render_target::Extent2D};

pub struct Camera {
    pub center: Vec3,
    /// Vertical fov in radians.
    pub vfov: f32,
    pub look_at: Vec3,
}

impl Camera {
    /// Create a [`Ray`] from origin to fragment in an extent.
    pub fn view_point_to_ray(&self, x: f32, y: f32, extent: &Extent2D) -> Ray {
        // right handed orthonormal basis vectors
        // with with camera facing -z
        let w = (self.center - self.look_at).normalize();
        let u = Vec3::Y.cross(w).normalize();
        let v = w.cross(u).normalize();

        let focal_length = (self.center - self.look_at).length();

        let viewport_height = 2.0 * (self.vfov / 2.0).tan() * focal_length;
        let viewport_width = viewport_height * extent.width as f32 / extent.height as f32;

        let viewport_u = viewport_width * u;
        // -viewport_height to support y down
        let viewport_v = viewport_height * -v;

        // upper left = forward + half width + half height
        let viewport_upper_left =
            self.center - (focal_length * w) - 0.5 * (viewport_u + viewport_v);

        let pixel_du = viewport_u / extent.width as f32;
        let pixel_dv = viewport_v / extent.height as f32;
        let pixel_00 = viewport_upper_left + 0.5 * (pixel_du + pixel_dv);

        let pixel = pixel_00 + x * pixel_du + y * pixel_dv;
        Ray {
            origin: self.center,
            direction: pixel - self.center,
        }
    }
}
