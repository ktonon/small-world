use image::{imageops::FilterType, DynamicImage};
use small_world_model::{combine_images, convert_nc_to_png, load_png};
use std::error::Error;
use std::path::Path;

pub fn main() -> Result<(), Box<dyn Error>> {
    let img1 = convert_nc_to_png(Path::new("../data/age.2020.1.GTS2012.1m.classic.nc"))?;
    let img2 = load_png(Path::new("../data/2008_age_of_oceans_plates_fullscale.png"))?;

    let (width, height) = (8192, 4096);
    let img1 = image::imageops::resize(&img1, width, height, FilterType::Lanczos3);
    let img2 = image::imageops::resize(&img2, width, height, FilterType::Lanczos3);
    let img2 = image::imageops::grayscale(&img2);
    let img2 = DynamicImage::ImageLuma8(img2).to_rgb8();

    let png_out = Path::new("../public/age.2020.1.GTS2012.png");
    std::fs::create_dir_all(png_out.parent().unwrap())?;

    let img = combine_images(img1, img2, 0.5)?;
    img.save(png_out)?;

    println!("Saved → {:?}", png_out);
    Ok(())
}
