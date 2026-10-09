/// Image Raytracer
/// based on the excellent:
/// https://raytracing.github.io/books/RayTracingInOneWeekend.html
///
/// Implementation Details:
/// - Right handed coordinate system
/// - Traced from `pixel_00` (upper left) to pixel_wh (bottom right).
use glam::Vec3;
use indicatif::{ProgressBar, ProgressStyle};
use std::{env, error::Error, io::Write};

mod color;
mod ray;

use color::Color;
use ray::Ray;

/// Returned by [`Hittable::intersect`].
struct Hit {
    #[allow(dead_code)]
    point: Vec3,
    /// Unit normal always opposite to ray.
    normal: Vec3,
    #[allow(dead_code)]
    /// `true` if ray came from outside the surface.
    front_face: bool,
    #[allow(dead_code)]
    t: f32,
}

trait Hittable {
    /// Intersects `r` with object surface.
    ///
    /// Returns the ray parameter `t` of hit point `P(t) = O + tD`
    /// or [`None`] if misses.
    fn intersect(&self, r: &Ray) -> Option<Hit>;
}

enum Shape {
    Sphere(Sphere),
}

impl Shape {
    fn intersect(&self, r: &Ray) -> Option<Hit> {
        match self {
            Shape::Sphere(sphere) => sphere.intersect(r),
        }
    }
}

struct Sphere {
    center: Vec3,
    radius: f32,
}

impl Hittable for Sphere {
    fn intersect(&self, r: &Ray) -> Option<Hit> {
        let origin_to_center = self.center - r.origin;
        let a = r.direction.length_squared();
        let h = r.direction.dot(origin_to_center);
        let c = origin_to_center.length_squared() - self.radius * self.radius;

        // discriminant
        let d = h * h - a * c;
        if d < 0.0 {
            return None;
        }

        // use `-` root of quadratic to get first hit
        let t = (h - d.sqrt()) / a;

        // t > 0.0 guarantees infront of ray
        if t > 0.0 {
            let point = r.at(t);
            let normal = (point - self.center).normalize();
            // determine if ray come from is outside shape's surface
            let front_face = r.direction.dot(normal) < 0.0;
            Some(Hit {
                point,
                normal,
                t,
                front_face,
            })
        } else {
            None
        }
    }
}

fn write_color<T: Write>(out: &mut T, color: &Color) -> Result<(), std::io::Error> {
    let ir = (color.r() * 255.99) as u8;
    let ig = (color.g() * 255.99) as u8;
    let ib = (color.b() * 255.99) as u8;
    writeln!(out, "{ir} {ig} {ib}")
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    assert!(args.len() > 1);

    // Image
    let out_path = &args[1];
    let aspect_ratio = 16.0 / 9.0_f32;
    assert!(aspect_ratio > 0.0);
    let inv_aspect_ratio = aspect_ratio.recip();

    let iwidth = 256;
    let iheight = ((iwidth as f32 * inv_aspect_ratio).round() as i32).max(1);
    let pixel_count = (iwidth * iheight * 12) as usize;

    // Camera
    let focal_length = 1.0_f32;
    let camera_center = Vec3::ZERO;

    let viewport_height = 2.0;
    let viewport_width = viewport_height * iwidth as f32 / iheight as f32;

    let viewport_u = Vec3::new(viewport_width, 0.0, 0.0);
    // -viewport_height to support y down
    let viewport_v = Vec3::new(0.0, -viewport_height, 0.0);

    // upper left = forward + half width + half height
    let viewport_upper_left =
        camera_center - Vec3::new(0.0, 0.0, focal_length) - 0.5 * (viewport_u + viewport_v);

    let pixel_du = viewport_u / iwidth as f32;
    let pixel_dv = viewport_v / iheight as f32;
    let pixel_00 = viewport_upper_left + 0.5 * (pixel_du + pixel_dv);

    // Objects
    let shapes: Vec<Shape> = vec![
        Shape::Sphere(Sphere {
            center: Vec3::new(0.0, 0.0, -2.0),
            radius: 1.0,
        }),
        Shape::Sphere(Sphere {
            center: Vec3::new(0.0, -100.5, -2.0),
            radius: 100.0,
        }),
    ];

    // Render
    let header = format!("P3\n{iwidth} {iheight}\n255\n").into_bytes();
    let mut buf: Vec<u8> = Vec::with_capacity(header.len() + pixel_count);
    buf.write_all(&header)?;

    let pb = ProgressBar::new(iheight as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} rows ({eta})",
        )
        .unwrap()
        .progress_chars("=>-"),
    );

    for row in 0..iheight {
        pb.inc(1);
        for col in 0..iwidth {
            let pixel = pixel_00 + col as f32 * pixel_du + row as f32 * pixel_dv;
            let ray = Ray {
                origin: camera_center,
                direction: pixel - camera_center,
            };

            let color = match shapes.iter().find_map(|f| f.intersect(&ray)) {
                Some(hit) => {
                    let n_rgb = 0.5 * (hit.normal + Vec3::ONE);
                    Color::new(n_rgb.x, n_rgb.y, n_rgb.z)
                }
                None => {
                    let unit_dir = ray.direction.normalize();
                    let a = 0.5 * (unit_dir.y + 1.0);
                    Color((1.0 - a) * Vec3::new(1.0, 1.0, 1.0) + a * Vec3::new(0.5, 0.7, 1.0))
                }
            };
            write_color(&mut buf, &color)?;
        }
    }

    std::fs::write(out_path, buf)?;
    print!("Contents written to '{}'", out_path);
    Ok(())
}
