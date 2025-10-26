use crate::geometry::{lat_of, lon_of, neighbors_within, GeoGrid};
use crate::map_helpers::{par_min_max, pixel_area_lookup};
use image::{Rgb, RgbImage};
use nalgebra::{Matrix3, Vector2, Vector3};
use netcdf3::FileReader;
use rayon::prelude::*;
use std::error::Error;
use std::path::Path;

pub fn convert_nc_to_gradient_map(nc_path: &Path) -> Result<RgbImage, Box<dyn Error>> {
    let earth_radius = 6_371_008.8; // meters
    let age_var_name = "z";
    let lat_var_name = "lat";
    let lon_var_name = "lon";

    // Open + read metadata
    let mut reader = FileReader::open(nc_path)?;
    let ds = reader.data_set(); // &DataSet  (metadata only)  (has get_vars, get_var, dim_size, etc.)
                                // Helpful listing

    println!("Attributes:");
    for attr in ds.get_global_attrs() {
        if let Some(val) = attr.get_as_string() {
            println!("- {}: {}", attr.name(), val);
        }
    }

    println!("Variables: {:?}", ds.get_var_names()); // optional
    let age_var = ds.get_var(age_var_name).expect("age variable not found");
    let lat_var = ds
        .get_var(lat_var_name)
        .expect("latitude variable not found");
    let lon_var = ds
        .get_var(lon_var_name)
        .expect("longitude variable not found");

    println!("  - age: {} {:?}", age_var.data_type(), age_var.dim_names());
    println!("  - lat: {} {:?}", lat_var.data_type(), lat_var.dim_names(),);
    println!("  - lon: {} {:?}", lon_var.data_type(), lon_var.dim_names());

    let ny = ds.dim_size(&age_var.dim_names()[0]).unwrap() as usize;
    let nx = ds.dim_size(&age_var.dim_names()[1]).unwrap() as usize;
    let grid = GeoGrid {
        nx,
        ny,
        radius: earth_radius,
    };

    println!("Grid: {:?}", &grid);

    let age_data: Vec<f32> = reader.read_var_f32(age_var_name)?;
    let lat_data: Vec<f64> = reader.read_var_f64(lat_var_name)?;
    let lon_data: Vec<f64> = reader.read_var_f64(lon_var_name)?; // -90 to 90
    let (max_area, area_lookup) = pixel_area_lookup(nx, ny, earth_radius);

    let (min_lat, max_lat) = par_min_max(&lat_data);
    let (min_lon, max_lon) = par_min_max(&lon_data);
    let (min, max) = par_min_max(&age_data);
    println!(
        "Ranges:
  - age: [{min}, {max}]
  - lat: [{min_lat}, {max_lat}]
  - lon: [{min_lon}, {max_lon}]
"
    );

    // let (min_lat, max_lat) = lat_data.iter().fold(|| (f64::MAX, f64::MIN), |(min, max), &v| ())
    // Find min and max in parallel (ignoring NaNs)
    // let (min, max) = age_data
    //     .par_iter()
    //     .filter(|v| !v.is_nan())
    //     .fold(
    //         || (f32::MAX, f32::MIN),
    //         |(min, max), &v| (min.min(v), max.max(v)),
    //     )
    //     .reduce(|| (f32::MAX, f32::MIN), |a, b| (a.0.min(b.0), a.1.max(b.1)));

    let span = if max > min { max - min } else { 1.0 };

    // Compute all pixels in parallel
    let pixels: Vec<Rgb<u8>> = age_data
        .par_iter()
        .enumerate()
        .map(|(i, age)| {
            let y = i / nx;
            let x = i % nx;
            let lon = lon_data[x];
            let lat = lat_data[y];
            if age.is_nan() {
                Rgb([0, 0, 0])
            } else {
                let neighbors = neighbors_within(&grid, i, 4000.0);
                if let Some(g) = gradient_tangent(&grid, i, &neighbors, &age_data) {
                    let (mag, bearing) = gradient_magnitude_bearing(g);
                    let (r, g, b) = gradient_to_rgb(100.0, bearing, 100.0);
                    Rgb([r, g, b])
                } else {
                    Rgb([255, 0, 0])
                }
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

/// Compute the local tangent-plane gradient (east,north) in scalar units per meter.
///
/// Returns `None` if fewer than 3 valid samples or if the fit matrix is singular.
///
/// # Notes
/// - The returned vector `(a, b)` represents ∂A/∂x (east) and ∂A/∂y (north).
/// - If all sample values are identical (a flat field), the normal-equation
///   matrix becomes singular and inversion may fail. In that case, you can:
///     * Return `Vector2::zeros()` to represent “no slope” (common choice), or
///     * Return `None` and let the caller decide how to handle flat areas.
/// - Even when the plane is nearly flat, small floating-point noise can make
///   the direction (bearing) jump between ±π. This is expected—bearing is
///   undefined when the gradient magnitude is near zero.
/// - For noisy data, consider weighting by distance, e.g.
///   `w = exp(- (x² + y²) / σ²)`.
///
/// # Units
/// Gradient components are in the same units as `values` per meter.
pub fn gradient_tangent(
    grid: &GeoGrid,
    center_idx: usize,
    indices: &[usize],
    values: &[f32],
) -> Option<Vector2<f32>> {
    if indices.len() < 3 {
        return None;
    }
    // println!("idx count {}", indices.len());

    let nx = grid.nx;
    let ny = grid.ny;
    let (j0, i0) = (center_idx / nx, center_idx % nx);
    let phi0 = lat_of(j0, ny);
    let lam0 = lon_of(i0, nx);

    // local tangent basis
    let n = Vector3::new(phi0.cos() * lam0.cos(), phi0.cos() * lam0.sin(), phi0.sin());
    let e_east = Vector3::new(-lam0.sin(), lam0.cos(), 0.0);
    let e_north = n.cross(&e_east); // ensure orthogonal

    let mut sx = 0.0;
    let mut sy = 0.0;
    let mut sz = 0.0;
    let mut sxx = 0.0;
    let mut syy = 0.0;
    let mut sxy = 0.0;
    let mut sxz = 0.0;
    let mut syz = 0.0;
    let mut wsum = 0.0;

    for &idx in indices.iter() {
        let val = values[idx];
        let j = idx / nx;
        let i = idx % nx;
        let phi = lat_of(j, ny);
        let lam = lon_of(i, nx);

        // convert to 3D unit vector
        let p = Vector3::new(phi.cos() * lam.cos(), phi.cos() * lam.sin(), phi.sin());
        // project onto tangent plane
        let v = grid.radius * (p - n * (p.dot(&n)));
        let x = v.dot(&e_east);
        let y = v.dot(&e_north);

        // weight (optional Gaussian in distance)
        let w = 1.0; // simple unweighted fit

        sx += w * x;
        sy += w * y;
        sz += w * val;
        sxx += w * x * x;
        syy += w * y * y;
        sxy += w * x * y;
        sxz += w * x * val;
        syz += w * y * val;
        wsum += w;
    }

    // normal equations for plane fit
    let m = Matrix3::new(sxx, sxy, sx, sxy, syy, sy, sx, sy, wsum);
    let b = Vector3::new(sxz, syz, sz);

    let sol = m.try_inverse().map(|inv| inv * b)?;
    let (a, b, _c) = (sol.x, sol.y, sol.z);
    Some(Vector2::new(a, b))
}

/// Convert tangent gradient (east,north) into magnitude and bearing.
/// Returns (magnitude, bearing_radians)
pub fn gradient_magnitude_bearing(g: Vector2<f32>) -> (f32, f32) {
    let mag = g.norm(); // scalar change per meter
    let bearing = g.x.atan2(g.y); // east,north → clockwise from north
    (mag, bearing)
}

/// Convert gradient magnitude & bearing into RGB color (0–255).
/// - `mag`: gradient magnitude (arbitrary units)
/// - `bearing`: radians clockwise from north
/// - `mag_max`: magnitude corresponding to full intensity (clamped)
pub fn gradient_to_rgb(mag: f32, bearing: f32, mag_max: f32) -> (u8, u8, u8) {
    // normalize magnitude → [0,1]
    let intensity = (mag / mag_max).clamp(0.0, 1.0);

    // convert bearing (radians) to hue [0,1), rotating so 0°=north→blue
    let hue = ((bearing + std::f32::consts::PI * 2.0) % (2.0 * std::f32::consts::PI))
        / (2.0 * std::f32::consts::PI);

    // simple HSV → RGB
    let h = hue * 6.0;
    let c = intensity;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let (r1, g1, b1) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = 0.0; // no value offset
    let (r, g, b) = ((r1 + m) * 255.0, (g1 + m) * 255.0, (b1 + m) * 255.0);
    (r as u8, g as u8, b as u8)
}

#[cfg(test)]
mod gradient_tests {
    use super::*;
    use crate::geometry::neighbors_within;

    use std::f32::consts::PI;

    #[test]
    fn test_gradient_flat_plane() {
        let grid = GeoGrid {
            nx: 360,
            ny: 180,
            radius: 6_371_000.0,
        };
        let center = (grid.ny / 2) * grid.nx + (grid.nx / 2);
        let neighbors = neighbors_within(&grid, center, 1_000_000.0);
        assert!(neighbors.len() >= 3, "too few neighbors found");

        let values: Vec<f32> = vec![42.0; neighbors.len()];

        let g =
            gradient_tangent(&grid, center, &neighbors, &values).expect("gradient should compute");

        let (mag, bearing) = gradient_magnitude_bearing(g);

        assert!(
            mag.abs() < 1e-12,
            "gradient magnitude should be zero, got {}",
            mag
        );
        assert!(
            bearing.is_finite() && bearing.abs() <= PI,
            "bearing should be finite and within [-π, π], got {}",
            bearing
        );
    }

    #[test]
    fn test_gradient_east_west_increase() {
        let grid = GeoGrid {
            nx: 360,
            ny: 180,
            radius: 6_371_000.0,
        };
        let center = (grid.ny / 2) * grid.nx + (grid.nx / 2); // near 0°, 0°
        let neighbors = neighbors_within(&grid, center, 1_000_000.0);
        assert!(neighbors.len() >= 3, "too few neighbors found");

        // Age increases 1 unit per degree east, constant in latitude
        let values: Vec<f32> = neighbors
            .iter()
            .map(|&idx| {
                let lon_deg = lon_of(idx % grid.nx, grid.nx).to_degrees();
                lon_deg
            })
            .collect();

        let g =
            gradient_tangent(&grid, center, &neighbors, &values).expect("gradient should compute");

        let (mag, bearing) = gradient_magnitude_bearing(g);

        // Expect strong eastward slope (bearing ~ 90°)
        let bearing_deg = bearing.to_degrees();
        assert!(mag > 0.0, "expected positive slope magnitude, got {}", mag);
        assert!(
            (bearing_deg - 90.0).abs() < 5.0 || (bearing_deg + 270.0).abs() < 5.0,
            "expected bearing ≈ 90°, got {}°",
            bearing_deg
        );
    }

    #[test]
    fn test_gradient_north_south_increase() {
        let grid = GeoGrid {
            nx: 360,
            ny: 180,
            radius: 6_371_000.0,
        };
        let center = (grid.ny / 2) * grid.nx + (grid.nx / 2); // near 0°, 0°
        let neighbors = neighbors_within(&grid, center, 1_000_000.0);
        assert!(neighbors.len() >= 3, "too few neighbors found");

        // Age increases 1 unit per degree north, constant in longitude
        let values: Vec<f32> = neighbors
            .iter()
            .map(|&idx| {
                let lat_deg = lat_of(idx / grid.nx, grid.ny).to_degrees();
                lat_deg
            })
            .collect();

        let g =
            gradient_tangent(&grid, center, &neighbors, &values).expect("gradient should compute");

        let (mag, bearing) = gradient_magnitude_bearing(g);

        let bearing_deg = bearing.to_degrees();

        // Expect strong northward slope (bearing ~ 0°)
        assert!(mag > 0.0, "expected positive slope magnitude, got {}", mag);
        assert!(
            bearing_deg.abs() < 5.0 || (bearing_deg - 360.0).abs() < 5.0,
            "expected bearing ≈ 0°, got {}°",
            bearing_deg
        );
    }
}
