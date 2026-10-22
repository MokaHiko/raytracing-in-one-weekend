/// Image Raytracer
/// based on the excellent:
/// https://raytracing.github.io/books/RayTracingInOneWeekend.html
///
/// Implementation Details:
/// - Right handed coordinate system
/// - Traced from `pixel_00` (upper left) to pixel_wh (bottom right).
use glam::{IVec2, Vec3};
use indicatif::{ProgressBar, ProgressStyle};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use rayon::prelude::*;
use std::ops::RangeBounds;
use std::sync::Arc;
use std::{env, error::Error, io::Write};

mod camera;
mod color;
mod ray;
mod render_target;

use camera::Camera;
use color::Color;
use ray::Ray;
use render_target::RenderTarget;

/// Seed for scene generation, fixed so a render is reproducible and can be
/// diffed against another run or against the GPU path. The value itself is arbitrary.
const SCENE_SEED: u64 = 0x5EED_B00C_CAFE_1234;

/// Returned by [`Hittable::intersect`].
struct Hit {
    point: Vec3,
    /// Unit normal always opposite to ray.
    normal: Vec3,
    /// `true` if ray came from outside the surface.
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

struct Scatter {
    ray: Ray,
    attenuation: Color,
}

trait Material: Send + Sync {
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

struct Dielectric {
    /// Refractive index in vacuum or air, or the ratio of the material's refractive index over
    /// the refractive index of the enclosing media.
    refraction_index: f32,
}

impl Material for Dielectric {
    fn scatter(&self, rv3: &Vec3, ray: &Ray, attenuation: &Color, hit: &Hit) -> Option<Scatter> {
        let unit_dir = ray.direction.normalize();
        let ri = if hit.front_face {
            self.refraction_index.recip()
        } else {
            self.refraction_index
        };

        // Schlick's approximation for reflectance
        let r0 = ((1.0 - ri) / (1.0 + ri)).powi(2);

        // account floating point error's like 1.0000000002
        let cos_theta = (-unit_dir).dot(hit.normal).min(1.0);
        let reflectance = r0 + (1.0 - r0) * (1.0 - cos_theta).powi(5);

        let refract = unit_dir.refract(hit.normal, ri);

        Some(Scatter {
            ray: Ray {
                origin: hit.point,
                // HACK: Here we're using the general random rv3 with
                // components in [-1.0 .. 1.0] remapped to [0.0 .. 1.0] as our generator.
                direction: if refract == Vec3::ZERO || reflectance > (rv3.x + 1.0) * 0.5 {
                    unit_dir.reflect(hit.normal)
                } else {
                    refract
                },
            },
            attenuation: *attenuation,
        })
    }
}

/// Basic renderable unit.
struct Object {
    shape: Shape,
    material: Arc<dyn Material>,
}

fn render(
    rt: &mut RenderTarget,
    camera: &Camera,
    objects: &[Object],
    max_depth: usize,
    sample_count: usize,
    progress_bar: Option<&ProgressBar>,
) {
    let extent = rt.extent2d();

    // split work between scanlines of render target
    rt.fragments_mut()
        .par_chunks_mut(extent.width)
        .enumerate()
        .for_each(|(y, colors)| {
            let mut rng = SmallRng::seed_from_u64(y as u64);
            for (x, color) in colors.iter_mut().enumerate() {
                let coord = IVec2::new(x as i32, y as i32);
                let acc: Vec3 = (0..sample_count)
                    .map(|_| {
                        let mut depth = 0;
                        let mut attenuation = Color::new(1.0, 1.0, 1.0);
                        let mut ray =
                            camera.get_ray(&mut rng, coord.x as f32, coord.y as f32, &extent);

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
                                        let rv3 = Vec3::random(&mut rng);
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

                *color = Color(acc / sample_count as f32);
            }

            if let Some(pb) = progress_bar {
                pb.inc(1);
            }
        });
}

fn main() -> Result<(), Box<dyn Error>> {
    // Parse command line arguments.
    let args: Vec<String> = env::args().collect();
    let Some(out_path) = args.get(1) else {
        eprintln!("usage: {} <output.ppm>", args[0]);
        std::process::exit(2);
    };

    let aspect_ratio = 16.0 / 9.0_f32;
    assert!(aspect_ratio > 0.0);
    let inv_aspect_ratio = aspect_ratio.recip();
    let iwidth = 1200;
    let iheight = ((iwidth as f32 * inv_aspect_ratio).round() as i32).max(1);
    let byte_count = (iwidth * iheight * 12) as usize;

    // camera args
    let vfov = 20_f32.to_radians();
    let center = Vec3::new(13.0, 2.0, 3.0);

    // Create scene resources and objects.
    let mut rng = SmallRng::seed_from_u64(SCENE_SEED);
    let mut rt = RenderTarget::new(iwidth as usize, iheight as usize);
    let camera = Camera {
        center,
        vfov,
        look_at: Vec3::new(0.0, 0.0, 0.0),
        focus_dist: 10.0,
        defocus_angle: 0.6_f32.to_radians(),
    };

    // objects
    let mut objects: Vec<Object> = vec![
        // ground
        Object {
            shape: Shape::Sphere(Sphere {
                center: Vec3::new(0.0, -1000.0, 0.0),
                radius: 1000.0,
            }),
            material: Arc::new(Lambertian {
                albedo: Color::new(0.5, 0.5, 0.5),
            }),
        },
        Object {
            shape: Shape::Sphere(Sphere {
                center: Vec3::new(0.0, 1.0, 0.0),
                radius: 1.0,
            }),
            material: Arc::new(Dielectric {
                refraction_index: 1.5,
            }),
        },
        Object {
            shape: Shape::Sphere(Sphere {
                center: Vec3::new(-4.0, 1.0, 0.0),
                radius: 1.0,
            }),
            material: Arc::new(Lambertian {
                albedo: Color::new(0.4, 0.2, 0.1),
            }),
        },
        Object {
            shape: Shape::Sphere(Sphere {
                center: Vec3::new(4.0, 1.0, 0.0),
                radius: 1.0,
            }),
            material: Arc::new(Metal {
                albedo: Color::new(0.7, 0.6, 0.5),
                fuzz: 0.0,
            }),
        },
    ];

    for a in -11..11 {
        for b in -11..11 {
            let center = Vec3::new(
                a as f32 + 0.9 * rng.random::<f32>(),
                0.2,
                b as f32 + 0.9_f32 * rng.random::<f32>(),
            );

            if (center - Vec3::new(4.0, 0.2, 0.0)).length() > 0.9 {
                match rng.random::<f32>() {
                    // diffuse
                    x if x < 0.8 => {
                        let albedo = Color::new(rng.random(), rng.random(), rng.random());
                        objects.push(Object {
                            shape: Shape::Sphere(Sphere {
                                center,
                                radius: 0.2,
                            }),
                            material: Arc::new(Lambertian { albedo }),
                        });
                    }
                    // metal
                    x if x < 0.95 => {
                        let albedo = Color(Vec3 {
                            x: rng.random_range(0.5..=1.0),
                            y: rng.random_range(0.5..=1.0),
                            z: rng.random_range(0.5..=1.0),
                        });
                        let fuzz: f32 = rng.random();
                        objects.push(Object {
                            shape: Shape::Sphere(Sphere {
                                center,
                                radius: 0.2,
                            }),
                            material: Arc::new(Metal { albedo, fuzz }),
                        });
                    }
                    // glass
                    _ => {
                        objects.push(Object {
                            shape: Shape::Sphere(Sphere {
                                center,
                                radius: 0.2,
                            }),
                            material: Arc::new(Dielectric {
                                refraction_index: 1.5,
                            }),
                        });
                    }
                }
            }
        }
    }

    // render w progress bar
    let pb = ProgressBar::new(iheight as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} rows ({eta})",
        )
        .unwrap()
        .progress_chars("=>-"),
    );

    render(&mut rt, &camera, &objects, 50, 500, Some(&pb));
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
