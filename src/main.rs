/// Image Raytracer
/// based on the excellent:
/// https://raytracing.github.io/books/RayTracingInOneWeekend.html
///
/// Implementation Details:
/// - Right handed coordinate system
/// - Traced from `pixel_00` (upper left) to pixel_wh (bottom right).
use glam::Vec3;
use indicatif::{ProgressBar, ProgressStyle};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::ops::RangeBounds;
use std::rc::Rc;
use std::{env, error::Error, io::Write};

mod camera;
mod color;
mod ray;
mod render_target;

use camera::Camera;
use color::Color;
use ray::Ray;
use render_target::RenderTarget;

struct Scatter {
    ray: Ray,
    attenuation: Color,
}

trait Material {
    fn scatter(&self, rv3: &Vec3, ray: &Ray, attenuation: &Color, hit: &Hit) -> Option<Scatter>;
}

struct Lambertian {
    albedo: Color,
}

impl Material for Lambertian {
    fn scatter(&self, rv3: &Vec3, _: &Ray, attenuation: &Color, hit: &Hit) -> Option<Scatter> {
        Some(Scatter {
            ray: Ray {
                origin: hit.point,
                // bias new direction towards normal
                direction: rv3 + hit.normal,
            },
            attenuation: Color(attenuation.0 * self.albedo.0),
        })
    }
}

struct Metal {
    albedo: Color,
    /// Fuziness value for reflection in the range 0.0 - 1.0.
    fuzz: f32,
}

impl Material for Metal {
    fn scatter(&self, rv3: &Vec3, ray: &Ray, attenuation: &Color, hit: &Hit) -> Option<Scatter> {
        let ray = Ray {
            origin: hit.point,
            // mirror direction around normal
            direction: ray.direction.reflect(hit.normal).normalize() + self.fuzz * rv3,
        };

        if ray.direction.dot(hit.normal) > 0.0 {
            Some(Scatter {
                ray,
                attenuation: Color(attenuation.0 * self.albedo.0),
            })
        } else {
            None
        }
    }
}

/// Returned by [`Hittable::intersect`].
struct Hit {
    point: Vec3,
    /// Unit normal always opposite to ray.
    normal: Vec3,
    /// `true` if ray came from outside the surface.
    #[allow(dead_code)]
    front_face: bool,
    /// Parameter `t` of intersecting ray.
    t: f32,
}

trait Hittable {
    /// Intersects `r` with object surface.
    ///
    /// Returns the ray parameter `t` of hit point `P(t) = O + tD`
    /// or [`None`] if misses.
    fn intersect<T: RangeBounds<f32>>(&self, r: &Ray, range: T) -> Option<Hit>;
}

enum Shape {
    Sphere(Sphere),
}

impl Shape {
    fn intersect<T: RangeBounds<f32>>(&self, r: &Ray, range: T) -> Option<Hit> {
        match self {
            Shape::Sphere(sphere) => sphere.intersect(r, range),
        }
    }
}

struct Sphere {
    center: Vec3,
    radius: f32,
}

impl Hittable for Sphere {
    fn intersect<T: RangeBounds<f32>>(&self, r: &Ray, range: T) -> Option<Hit> {
        let origin_to_center = self.center - r.origin;
        let a = r.direction.length_squared();
        let h = r.direction.dot(origin_to_center);
        let c = origin_to_center.length_squared() - self.radius * self.radius;

        // discriminant
        let d = h * h - a * c;
        if d < 0.0 {
            return None;
        }

        // use `-` root of quadratic first
        let sqrtd = d.sqrt();
        let mut t = (h - sqrtd) / a;
        if !range.contains(&t) {
            // try `+` root
            t = (h + sqrtd) / a;
            if !range.contains(&t) {
                return None;
            }
        };

        let point = r.at(t);
        let normal = (point - self.center).normalize();
        // determine if ray come from is outside shape's surface
        let front_face = r.direction.dot(normal) < 0.0;
        let normal = if front_face { normal } else { normal * -1.0 };

        Some(Hit {
            point,
            normal,
            t,
            front_face,
        })
    }
}

trait Random {
    fn random<T: rand::RngCore>(rng: &mut T) -> Self;
}

impl Random for Vec3 {
    fn random<T: rand::RngCore>(rng: &mut T) -> Self {
        Self::new(
            rng.random_range(-1.0..1.0),
            rng.random_range(-1.0..1.0),
            rng.random_range(-1.0..1.0),
        )
    }
}

/// Basic renderable unit.
struct Object {
    shape: Shape,
    material: Rc<dyn Material>,
}

fn render<T: rand::RngCore>(
    rt: &mut RenderTarget,
    camera: &Camera,
    objects: &[Object],
    rng: &mut T,
    max_depth: usize,
    sample_count: usize,
    progress_bar: Option<&ProgressBar>,
) {
    let extent = rt.extent2d();
    for frag in rt.fragments_mut() {
        let acc: Vec3 = (0..sample_count)
            .map(|_| {
                let mut depth = 0;
                let mut attenuation = Color::new(1.0, 1.0, 1.0);
                let mut ray = camera.view_point_to_ray(
                    frag.coord.x as f32 + rng.random_range(-0.5..=0.5),
                    frag.coord.y as f32 + rng.random_range(-0.5..=0.5),
                    &extent,
                );

                loop {
                    if depth >= max_depth {
                        break Vec3::ZERO;
                    }

                    match objects
                        .iter()
                        .filter_map(|f| {
                            f.shape
                                .intersect(&ray, 0.001..=f32::INFINITY)
                                .map(|hit| (hit, &f.material))
                        })
                        .min_by(|(a, _), (b, _)| a.t.total_cmp(&b.t))
                    {
                        Some((hit, material)) => {
                            depth += 1;

                            // Generate random vector on unit sphere via rejection.
                            let rv3 = loop {
                                let rv3 = Vec3::random(rng);
                                if rv3.length_squared() > 1.0 {
                                    continue;
                                }

                                if let Some(rv3) = rv3.try_normalize() {
                                    break rv3;
                                }
                            };

                            if let Some(Scatter {
                                ray: scattered_ray,
                                attenuation: scattered_attenuation,
                            }) = material.scatter(&rv3, &ray, &attenuation, &hit)
                            {
                                ray = scattered_ray;
                                attenuation = scattered_attenuation;
                            } else {
                                return Vec3::ZERO;
                            }
                        }
                        None => {
                            let unit_dir = ray.direction.normalize();
                            let a = 0.5 * (unit_dir.y + 1.0);
                            return attenuation.0
                                * ((1.0 - a) * Vec3::ONE + a * Vec3::new(0.5, 0.7, 1.0));
                        }
                    }
                }
            })
            .sum();

        *frag.color = Color(acc / sample_count as f32);
        if let Some(pb) = progress_bar
            && frag.coord.x == 0
        {
            pb.inc(1);
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Parse command line arguments.
    let args: Vec<String> = env::args().collect();
    assert!(args.len() > 1);

    // image args
    let out_path = &args[1];
    let aspect_ratio = 16.0 / 9.0_f32;
    assert!(aspect_ratio > 0.0);
    let inv_aspect_ratio = aspect_ratio.recip();
    let iwidth = 720;
    let iheight = ((iwidth as f32 * inv_aspect_ratio).round() as i32).max(1);
    let byte_count = (iwidth * iheight * 12) as usize;

    // camera args
    let focal_length = 1.0_f32;
    let viewport_height = 2.0;
    let center = Vec3::ZERO;

    // Create scene resources and objects.
    let mut rng = SmallRng::from_rng(&mut rand::rng());
    let mut rt = RenderTarget::new(iwidth as usize, iheight as usize);
    let camera = Camera {
        focal_length,
        center,
        viewport_height,
    };

    // materials
    let center = Rc::new(Lambertian {
        albedo: Color::new(0.1, 0.2, 0.5),
    });

    let ground = Rc::new(Lambertian {
        albedo: Color::new(0.8, 0.8, 0.0),
    });

    let left = Rc::new(Metal {
        albedo: Color::new(0.8, 0.8, 0.8),
        fuzz: 0.25,
    });

    let right = Rc::new(Metal {
        albedo: Color::new(0.8, 0.6, 0.2),
        fuzz: 0.8,
    });

    // objects
    let objects: Vec<Object> = vec![
        Object {
            shape: Shape::Sphere(Sphere {
                center: Vec3::new(0.0, 0.5, -2.0),
                radius: 1.0,
            }),
            material: center,
        },
        Object {
            shape: Shape::Sphere(Sphere {
                center: Vec3::new(0.0, -100.5, -2.0),
                radius: 100.0,
            }),
            material: ground,
        },
        Object {
            shape: Shape::Sphere(Sphere {
                center: Vec3::new(-2.0, 0.5, -2.0),
                radius: 1.0,
            }),
            material: left,
        },
        Object {
            shape: Shape::Sphere(Sphere {
                center: Vec3::new(2.0, 0.5, -2.0),
                radius: 1.0,
            }),
            material: right,
        },
    ];

    // render w progress bar
    let pb = ProgressBar::new(iheight as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} rows ({eta})",
        )
        .unwrap()
        .progress_chars("=>-"),
    );

    render(&mut rt, &camera, &objects, &mut rng, 10, 100, Some(&pb));
    pb.finish();

    // Output to '.ppm' format.
    let header = format!("P3\n{iwidth} {iheight}\n255\n").into_bytes();
    let mut buf: Vec<u8> = Vec::with_capacity(header.len() + byte_count);
    buf.write_all(&header)?;

    for color in rt.colors() {
        let gamma_color = color.0.sqrt();
        let ir = (gamma_color.x * 255.99) as u8;
        let ig = (gamma_color.y * 255.99) as u8;
        let ib = (gamma_color.z * 255.99) as u8;
        writeln!(buf, "{ir} {ig} {ib}")?;
    }

    std::fs::write(out_path, buf)?;
    println!("Contents written to '{}'", out_path);

    Ok(())
}
