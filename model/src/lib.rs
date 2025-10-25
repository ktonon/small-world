use image::{imageops::FilterType, Rgb, RgbImage};
use netcdf3::FileReader;
use rayon::prelude::*;
use std::error::Error;
use std::path::Path;

pub fn convert_nc_to_png(nc_path: &Path) -> Result<(), Box<dyn Error>> {
    let var_name = "z";
    let png_out = Path::new("../public/age.2020.1.GTS2012.png");

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

    std::fs::create_dir_all(png_out.parent().unwrap())?;

    let resized = image::imageops::resize(&img, 8192, 4096, FilterType::Lanczos3);
    resized.save(png_out)?;
    println!("Saved → {:?}", png_out);
    Ok(())
}
