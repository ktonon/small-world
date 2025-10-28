use num_traits::Float;
use rayon::prelude::*;

/// Returns a vector of pixel areas (m²) for each latitude row (y index)
/// in an equirectangular map of size nx × ny.
pub fn pixel_area_lookup(nx: usize, ny: usize, radius: f32) -> (f32, Vec<f32>) {
    use std::f32::consts::PI;

    let dlon = 2.0 * PI / nx as f32;
    let dlat = PI / ny as f32;
    let max_area = radius * radius * dlon * dlat;

    (
        max_area,
        (0..ny)
            .map(|y| {
                // latitude at the center of row y (top = +90°, bottom = −90°)
                let lat = (PI / 2.0) - (y as f32 + 0.5) * dlat;
                max_area * lat.cos()
            })
            .collect(),
    )
}

pub fn area_of_sphere(radius: f32) -> f32 {
    use std::f32::consts::PI;
    4.0 * PI * radius * radius
}

pub fn par_min_max<T>(data: &[T]) -> (T, T)
where
    T: Float + Send + Sync,
{
    data.par_iter()
        .filter(|v| !v.is_nan())
        .fold(
            || (T::max_value(), T::min_value()),
            |(min, max), &v| (min.min(v), max.max(v)),
        )
        .reduce(
            || (T::max_value(), T::min_value()),
            |a, b| (a.0.min(b.0), a.1.max(b.1)),
        )
}
