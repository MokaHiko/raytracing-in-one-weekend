use glam::{Vec2, Vec3};
use rand::Rng;

use crate::{ray::Ray, render_target::Extent2D};

pub struct Camera {
    pub center: Vec3,
    /// Vertical fov in radians.
    pub vfov: f32,
    pub look_at: Vec3,
    pub focus_dist: f32,
    /// Variation of angle rays through each pixel in radians.
    pub defocus_angle: f32,
}

impl Camera {
    /// Create a [`Ray`] from origin to fragment in an extent.
    pub fn get_ray<T: rand::RngCore>(&self, rng: &mut T, x: f32, y: f32, extent: &Extent2D) -> Ray {
        let x = x + rng.random_range(-0.5..=0.5);
        let y = y + rng.random_range(-0.5..=0.5);

        // right handed orthonormal basis vectors
        // with with camera facing -z
        let w = (self.center - self.look_at).normalize();
        let u = Vec3::Y.cross(w).normalize();
        let v = w.cross(u).normalize();

        let viewport_height = 2.0 * (self.vfov / 2.0).tan() * self.focus_dist;
        let viewport_width = viewport_height * extent.width as f32 / extent.height as f32;

        let viewport_u = viewport_width * u;
        // -viewport_height to support y down
        let viewport_v = viewport_height * -v;

        // upper left = forward + half width + half height
        let viewport_upper_left =
            self.center - (self.focus_dist * w) - 0.5 * (viewport_u + viewport_v);

        let pixel_du = viewport_u / extent.width as f32;
        let pixel_dv = viewport_v / extent.height as f32;
        let pixel_00 = viewport_upper_left + 0.5 * (pixel_du + pixel_dv);

        let pixel = pixel_00 + x * pixel_du + y * pixel_dv;

        // offset origin within defocus disc
        let defocus_radius = (self.defocus_angle / 2.0).tan() * self.focus_dist;
        let defocus_u = defocus_radius * u;
        let defocus_v = defocus_radius * v;

        let rdisc = random_unit_disc(rng);
        let origin = self.center + rdisc.x * defocus_u + rdisc.y * defocus_v;

        Ray {
            origin,
            direction: pixel - origin,
        }
    }
}

fn random_unit_disc<T: rand::RngCore>(rng: &mut T) -> Vec2 {
    loop {
        let rv2 = Vec2::new(rng.random_range(-1.0..=1.0), rng.random_range(-1.0..=1.0));
        if rv2.length_squared() <= 1.0 {
            break rv2;
        }
    }
}
