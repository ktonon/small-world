use image::{ImageReader, Rgb, RgbImage};
use netcdf3::FileReader;
use rayon::prelude::*;
use std::error::Error;
use std::fs;
use std::path::Path;
use webp::Encoder;

pub fn convert_nc_to_png(nc_path: &Path) -> Result<RgbImage, Box<dyn Error>> {
    let var_name = "z";

    // Open + read metadata
    let mut reader = FileReader::open(nc_path)?;
    let ds = reader.data_set(); // &DataSet  (metadata only)  (has get_vars, get_var, dim_size, etc.)
                                // Helpful listing
    println!("Variables: {:?}", ds.get_var_names()); // optional

    let var = ds.get_var(var_name).expect("variable not found");

    let ny = ds.dim_size(&var.dim_names()[0]).unwrap() as usize;
    let nx = ds.dim_size(&var.dim_names()[1]).unwrap() as usize;
    println!("Grid: {} x {}", nx, ny);

    let data: Vec<f32> = reader.read_var_f32(var_name)?;

    // Find min and max in parallel (ignoring NaNs)
    let (min, max) = data
        .par_iter()
        .filter(|v| !v.is_nan())
        .fold(
            || (f32::MAX, f32::MIN),
            |(min, max), &v| (min.min(v), max.max(v)),
        )
        .reduce(|| (f32::MAX, f32::MIN), |a, b| (a.0.min(b.0), a.1.max(b.1)));

    let span = if max > min { max - min } else { 1.0 };

    // Compute all pixels in parallel
    let pixels: Vec<Rgb<u8>> = data
        .par_iter()
        .map(|&v| {
            if v.is_nan() {
                Rgb([0, 0, 0])
            } else {
                let t = ((v - min) / span).clamp(0.0, 1.0);
                let r = (255.0 * t) as u8;
                let g = (255.0 * (1.0 - ((t - 0.5).abs() * 2.0).clamp(0.0, 1.0))) as u8;
                let b = (255.0 * (1.0 - t)) as u8;
                Rgb([r, g, b])
            }
        })
        .collect();

    // Convert to image
    let mut img = RgbImage::new(nx as u32, ny as u32);
    for (i, px) in pixels.into_iter().enumerate() {
        let x = (i % nx) as u32;
        let y = (i / nx) as u32;
        img.put_pixel(x, (ny as u32 - 1) - y, px);
    }
    Ok(img)
}

pub fn load_png(png_path: &Path) -> Result<RgbImage, Box<dyn Error>> {
    let img = ImageReader::open(png_path)?.decode()?.to_rgb8();
    Ok(img)
}

pub fn combine_images(
    mut img1: RgbImage,
    img2: RgbImage,
    k: f64,
) -> Result<RgbImage, Box<dyn Error>> {
    // Ensure same dimensions or handle resizing
    let (width, height) = img1.dimensions();
    let (ow, oh) = img2.dimensions();

    for y in 0..height.min(oh) {
        for x in 0..width.min(ow) {
            let base_px = img1.get_pixel_mut(x, y);
            let over_px = img2.get_pixel(x, y);

            // Simple averaging blend
            for i in 0..3 {
                base_px.0[i] =
                    (base_px.0[i] as f64 * k) as u8 + (over_px.0[i] as f64 * (1.0 - k)) as u8;
            }
        }
    }
    Ok(img1)
}

pub fn save_webp_lossy(img: &RgbImage, quality: f32, path: &Path) -> std::io::Result<()> {
    let (w, h) = img.dimensions();
    // RgbImage data is already RGB8
    let enc = Encoder::from_rgb(img.as_raw(), w as u32, h as u32);
    let webp = enc.encode(quality); // 0.0–100.0
    fs::write(path, &*webp)
}
