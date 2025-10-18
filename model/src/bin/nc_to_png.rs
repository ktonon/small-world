use image::{imageops::FilterType, Rgb, RgbImage};
use netcdf3::FileReader;
use std::error::Error;
use std::path::Path;

fn main() -> Result<(), Box<dyn Error>> {
    let nc_path = Path::new("../data/age.2020.1.GTS2012.1m.classic.nc");
    let var_name = "z";
    let png_out = Path::new("../images/age.2020.1.GTS2012.png");

    // Open + read metadata
    let mut reader = FileReader::open(nc_path)?;
    let ds = reader.data_set(); // &DataSet  (metadata only)  (has get_vars, get_var, dim_size, etc.)
                                // Helpful listing
    println!("Variables: {:?}", ds.get_var_names()); // optional

    let var = ds.get_var(var_name).expect("variable not found");

    // Infer 2D grid shape from dimension names
    let dim_names = var.dim_names();
    assert!(dim_names.len() == 2, "expected a 2D variable");
    let ny = ds.dim_size(&dim_names[0]).expect("dim size") as usize;
    let nx = ds.dim_size(&dim_names[1]).expect("dim size") as usize;
    println!("Grid: {} x {}", nx, ny);

    // Read all values as f32 using the typed reader
    let data: Vec<f32> = reader.read_var_f32(var_name)?; // typed read helper

    // Normalize (ignore NaNs)
    let (mut min, mut max) = (f32::MAX, f32::MIN);
    for &v in &data {
        if !v.is_nan() {
            if v < min {
                min = v;
            }
            if v > max {
                max = v;
            }
        }
    }
    let span = if max > min { max - min } else { 1.0 };

    // Render simple blue→red gradient
    let mut img = RgbImage::new(nx as u32, ny as u32);
    for i in 0..data.len() {
        let x = (i % nx) as u32;
        let y = (i / nx) as u32;
        let v = data[i];
        let color = if v.is_nan() {
            Rgb([0, 0, 0])
        } else {
            let t = ((v - min) / span).clamp(0.0_f32, 1.0_f32);
            let r = (255.0 * t) as u8;
            let g =
                (255.0 * (1.0_f32 - ((t - 0.5_f32).abs() * 2.0_f32).clamp(0.0_f32, 1.0_f32))) as u8;
            let b = (255.0 * (1.0 - t)) as u8;
            Rgb([r, g, b])
        };
        // flip Y so origin is lower-left
        img.put_pixel(x, (ny as u32 - 1) - y, color);
    }

    std::fs::create_dir_all(png_out.parent().unwrap())?;

    let resized = image::imageops::resize(&img, 8192, 4096, FilterType::Lanczos3);
    resized.save(png_out)?;
    println!("Saved → {:?}", png_out);
    Ok(())
}
