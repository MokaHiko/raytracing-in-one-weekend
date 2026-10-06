use indicatif::{ProgressBar, ProgressStyle};
use std::{env, error::Error, io::Write};

mod color;
use color::Color;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    assert!(args.len() > 1);

    let width = 256;
    let height = 256;
    assert!(width > 0 && height > 0);

    let out_path = &args[1];

    let header = format!("P3\n{width} {height}\n255\n").into_bytes();

    let mut buf: Vec<u8> = Vec::with_capacity(header.len() + width * height * 12);
    buf.write_all(&header)?;

    let pb = ProgressBar::new(height as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} rows ({eta})",
        )
        .unwrap()
        .progress_chars("=>-"),
    );

    for row in 0..height {
        pb.inc(1);
        for col in 0..width {
            let color = Color::new(
                col as f32 / (width - 1) as f32,
                row as f32 / (height - 1) as f32,
                0.0,
            );

            let ir = (color.r() * 255.99) as u8;
            let ig = (color.g() * 255.99) as u8;
            let ib = 0_u8;
            buf.write_all(format!("{ir} {ig} {ib}\n").as_bytes())?;
        }
    }

    std::fs::write(out_path, buf)?;
    print!("Contents written to {}", out_path);
    Ok(())
}
