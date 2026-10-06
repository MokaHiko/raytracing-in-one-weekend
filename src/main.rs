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

struct Sphere {
    center: Vec3,
    radius: f32,
}

impl Sphere {
    fn did_hit(&self, r: &Ray) -> bool {
        let origin_to_center = self.center - r.origin;
        let a = r.direction.dot(r.direction);
        let b = 2.0 * r.direction.dot(origin_to_center);
        let c = origin_to_center.dot(origin_to_center) - self.radius * self.radius;

        (b * b - 4.0 * a * c) >= 0.0
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
    let sphere = Sphere {
        center: Vec3::new(0.0, 0.0, -2.0),
        radius: 1.0,
    };

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

            if sphere.did_hit(&ray) {
                write_color(&mut buf, &Color::new(1.0, 0.0, 0.0))?;
            } else {
                let unit_dir = ray.direction.normalize();
                let a = 0.5 * (unit_dir.y + 1.0);
                let ray_color =
                    Color((1.0 - a) * Vec3::new(1.0, 1.0, 1.0) + a * Vec3::new(0.5, 0.7, 1.0));
                write_color(&mut buf, &ray_color)?;
            }
        }
    }

    std::fs::write(out_path, buf)?;
    print!("Contents written to {}", out_path);
    Ok(())
}
