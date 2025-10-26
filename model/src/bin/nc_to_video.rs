use image::{imageops::FilterType, DynamicImage};
use small_world_model::image::{combine_images, convert_nc_to_png, load_png};
use small_world_model::video::make_video;
use std::path::Path;

pub fn main() -> std::io::Result<()> {
    let img1 = convert_nc_to_png(Path::new("../data/age.2020.1.GTS2012.1m.classic.nc")).unwrap();
    let img2 = load_png(Path::new("../data/2008_age_of_oceans_plates_fullscale.png")).unwrap();

    let height = 2048;
    let width = height * 2;

    let img1 = image::imageops::resize(&img1, width, height, FilterType::Lanczos3);
    let img2 = image::imageops::resize(&img2, width, height, FilterType::Lanczos3);
    let img2 = image::imageops::grayscale(&img2);
    let img2 = DynamicImage::ImageLuma8(img2).to_rgb8();

    let fps = 30;
    let duration_sec = 60;
    let half_frame_count: f64 = (fps * duration_sec) as f64 / 2.0;

    make_video(
        width,
        height,
        fps,
        duration_sec,
        "../public/earth.webm",
        |frame_idx| {
            let k = if frame_idx < half_frame_count as u32 {
                frame_idx as f64 / half_frame_count
            } else {
                1.0 - ((frame_idx - half_frame_count as u32) as f64 / half_frame_count)
            };
            combine_images(img1.clone(), img2.clone(), k).unwrap()
        },
    )
    // create_image()
}
