use crate::geometry::GeoGrid;
use crate::map_helpers::par_min_max;
use image::{Rgb, RgbImage};
use netcdf3::FileReader;
use rayon::prelude::*;
use std::error::Error;
use std::path::Path;

pub fn convert_nc_to_partition_map(nc_path: &Path) -> Result<RgbImage, Box<dyn Error>> {
    let earth_radius = 6_371_008.8; // meters
    let age_var_name = "z";
    let mut reader = FileReader::open(nc_path)?;
    let ds = reader.data_set();

    println!("Attributes:");
    for attr in ds.get_global_attrs() {
        if let Some(val) = attr.get_as_string() {
            println!("- {}: {}", attr.name(), val);
        }
    }

    println!("Variables: {:?}", ds.get_var_names()); // optional
    let age_var = ds.get_var(age_var_name).expect("age variable not found");
    println!("  - age: {} {:?}", age_var.data_type(), age_var.dim_names());

    let ny = ds.dim_size(&age_var.dim_names()[0]).unwrap() as usize;
    let nx = ds.dim_size(&age_var.dim_names()[1]).unwrap() as usize;
    let grid = GeoGrid {
        nx,
        ny,
        radius: earth_radius,
    };

    println!("Grid: {:?}", &grid);
    let age_data: Vec<f32> = reader.read_var_f32(age_var_name)?;
    let (min, max) = par_min_max(&age_data);
    println!(
        "Ranges:
  - age: [{min}, {max}]
"
    );

    let partitions = partition_crust(&age_data, (nx, ny), 10.0);
    println!("Found {} partitions", partitions.len());
    let labels = label_partitions(&partitions, age_data.len());
    let colors = generate_colors(partitions.len() + 1);
    let n = colors.len();
    let partition_map: Vec<Rgb<u8>> = labels
        .into_par_iter()
        .map(|i| {
            assert!(i < n, "{i} is not < {n}");
            colors[i]
        })
        .collect();

    // Convert to image
    let mut img = RgbImage::new(nx as u32, ny as u32);
    for (i, px) in partition_map.into_iter().enumerate() {
        let x = (i % nx) as u32;
        let y = (i / nx) as u32;
        img.put_pixel(x, (ny as u32 - 1) - y, px);
    }
    Ok(img)
}

pub fn partition_crust(
    ages: &[f32],
    shape: (usize, usize),
    min_age_to_keep: f32,
) -> Vec<Vec<usize>> {
    assert!(shape.0 > 0 && shape.1 > 0);
    assert_eq!(ages.len(), shape.0 * shape.1);

    let mut visited = vec![false; ages.len()];
    let mut patches: Vec<Vec<usize>> = Vec::new();

    let idx = |x: usize, y: usize| -> usize { y * shape.0 + x };
    let keep_crust = |i: usize| -> bool { ages[i].is_nan() || ages[i] >= min_age_to_keep };

    for y in 0..shape.1 {
        for x in 0..shape.0 {
            let i0 = idx(x, y);
            if visited[i0] || !keep_crust(i0) {
                continue;
            }

            // start a new patch
            let mut patch = Vec::new();
            let mut stack = vec![(x, y)];
            visited[i0] = true;

            while let Some((cx, cy)) = stack.pop() {
                let ci = idx(cx, cy);
                patch.push(ci);

                for (nx, ny) in get_neighbours((cx, cy), shape) {
                    let ni = idx(nx, ny);
                    if visited[ni] {
                        continue;
                    }
                    if keep_crust(ni) {
                        visited[ni] = true;
                        stack.push((nx, ny));
                    }
                }
            }

            patches.push(patch);
        }
    }

    patches
}

fn get_neighbours((cx, cy): (usize, usize), (nx, ny): (usize, usize)) -> Vec<(usize, usize)> {
    // wrap longitudinally
    let left = ((cx + nx - 1) % nx, cy);
    let right = ((cx + 1) % nx, cy);
    let mut neighbours = vec![left, right];
    if cy > 0 {
        neighbours.push((cx, cy - 1));
    }
    if cy + 1 < ny {
        neighbours.push((cx, cy + 1));
    }
    neighbours
}

/// Converts patch index lists into a flat partition map.
/// Each element in the returned Vec corresponds to the same index
/// in the original dataset and holds the patch ID (or usize::MAX if none).
pub fn label_partitions(patches: &[Vec<usize>], total_len: usize) -> Vec<usize> {
    let null_parition = patches.len();

    println!("Labelling partitions ({} data points)", total_len);
    let mut labels = vec![null_parition; total_len];

    for (patch_id, patch) in patches.iter().enumerate() {
        for &i in patch {
            labels[i] = patch_id;
        }
    }

    labels
}

/// Generate `n` distinct RGB colors spaced evenly around the hue circle.
pub fn generate_colors(n: usize) -> Vec<Rgb<u8>> {
    let mut colors = Vec::with_capacity(n);
    let golden_ratio = 0.618_033_988_75; // good hue spacing

    for i in 0..n - 1 {
        let hue = (i as f32 * golden_ratio) % 1.0;
        let h = hue * 6.0;
        let c = 255.0;
        let x = c * (1.0 - (h % 2.0 - 1.0).abs());

        let (r, g, b) = match h as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };

        colors.push(Rgb([r as u8, g as u8, b as u8]));
    }
    colors.push(Rgb([0, 0, 0]));

    colors
}
