use glam::{Vec2, Vec3};
use rand::Rng;

use crate::{ray::Ray, render_target::RenderTarget};

pub struct Camera {
    /// Location of the eye of the camera.
    center: Vec3,

    // Viewport direction vectors
    pixel_du: Vec3,
    pixel_dv: Vec3,
    pixel_00: Vec3,

    // Defocus disc direction vectors
    defocus_u: Vec3,
    defocus_v: Vec3,
}

pub struct Description {
    pub center: Vec3,
    pub vfov: f32,
    pub look_at: Vec3,
    pub focus_dist: f32,
    pub defocus_angle: f32,
}

impl Camera {
    pub fn new(render_target: &RenderTarget, desc: &Description) -> Camera {
        let Description {
            center,
            vfov,
            look_at,
            focus_dist,
            defocus_angle,
        } = *desc;
        let extent = render_target.extent2d();

        // right handed orthonormal basis vectors
        // with with camera facing -z
        let w = (center - look_at).normalize();
        let u = Vec3::Y.cross(w).normalize();
        let v = w.cross(u).normalize();

        let viewport_height = 2.0 * (vfov / 2.0).tan() * focus_dist;
        let viewport_width = viewport_height * extent.width as f32 / extent.height as f32;

        let viewport_u = viewport_width * u;
        // -viewport_height to support y down
        let viewport_v = viewport_height * -v;

        // upper left = forward + half width + half height
        let viewport_upper_left = center - (focus_dist * w) - 0.5 * (viewport_u + viewport_v);

        let pixel_du = viewport_u / extent.width as f32;
        let pixel_dv = viewport_v / extent.height as f32;
        let pixel_00 = viewport_upper_left + 0.5 * (pixel_du + pixel_dv);

        // offset origin within defocus disc
        let defocus_radius = (defocus_angle / 2.0).tan() * focus_dist;
        let defocus_u = defocus_radius * u;
        let defocus_v = defocus_radius * v;

        Camera {
            center,
            pixel_du,
            pixel_dv,
            pixel_00,
            defocus_u,
            defocus_v,
        }
    }

    /// Create a [`Ray`] from origin to fragment in an extent.
    pub fn get_ray<T: rand::RngCore>(&self, rng: &mut T, x: f32, y: f32) -> Ray {
        let x = x + rng.random_range(-0.5..=0.5);
        let y = y + rng.random_range(-0.5..=0.5);

        let pixel = self.pixel_00 + x * self.pixel_du + y * self.pixel_dv;

        let rdisc = random_unit_disc(rng);
        let origin = self.center + rdisc.x * self.defocus_u + rdisc.y * self.defocus_v;

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
