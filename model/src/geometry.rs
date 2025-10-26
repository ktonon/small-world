/// Global equirectangular grid (full 360x180 coverage; pixel centers).
#[derive(Debug)]
pub struct GeoGrid {
    pub nx: usize,   // cols (longitude)
    pub ny: usize,   // rows (latitude)
    pub radius: f32, // sphere radius (meters)
}

#[inline]
fn d2r(a: f32) -> f32 {
    a.to_radians()
}
#[inline]
fn wrap_i(i: isize, nx: usize) -> usize {
    let nx_i = nx as isize;
    let mut k = i % nx_i;
    if k < 0 {
        k += nx_i;
    }
    k as usize
}

// pixel-center lat/lon in radians (plate carrée, top row = +90°)
#[inline]
pub fn lat_of(j: usize, ny: usize) -> f32 {
    // map j ∈ [0,ny-1] to φ ∈ [+π/2, -π/2]
    let v = (j as f32 + 0.5) / ny as f32;
    d2r(90.0 - 180.0 * v)
}
#[inline]
pub fn lon_of(i: usize, nx: usize) -> f32 {
    // map i ∈ [0,nx-1] to λ ∈ [-π, +π)
    let u = (i as f32 + 0.5) / nx as f32;
    d2r(-180.0 + 360.0 * u)
}

/// Great-circle central angle (radians) via haversine (stable for small angles).
#[inline]
fn central_angle(phi1: f32, lam1: f32, phi2: f32, lam2: f32) -> f32 {
    let dphi = phi2 - phi1;
    let dlam = (lam2 - lam1 + std::f32::consts::PI).rem_euclid(2.0 * std::f32::consts::PI)
        - std::f32::consts::PI; // wrap to [-π,π]
    let s2 = (dphi * 0.5).sin().powi(2) + phi1.cos() * phi2.cos() * (dlam * 0.5).sin().powi(2);
    2.0 * s2.sqrt().asin()
}

/// Return linear indices of pixels within great-circle distance D (meters).
pub fn neighbors_within(grid: &GeoGrid, center_idx: usize, d_meters: f32) -> Vec<usize> {
    let GeoGrid { nx, ny, radius } = *grid;
    let i0 = center_idx % nx;
    let j0 = center_idx / nx;

    let phi0 = lat_of(j0, ny);
    let lam0 = lon_of(i0, nx);

    // Angular radius (radians)
    let delta = (d_meters / radius).min(std::f32::consts::PI);

    // Cheap bounding box to avoid scanning the whole grid:
    let dphi_pix = std::f32::consts::PI / ny as f32;
    let j_pad = (delta / dphi_pix).ceil() as isize;

    let j_min = 0.max(j0 as isize - j_pad) as usize;
    let j_max = (ny as isize - 1).min(j0 as isize + j_pad) as usize;

    // For longitude, approximate pad by delta / cos(phi0) (guard near poles)
    let cos0 = phi0.cos().abs().max(1e-6);
    let dlam_pad = (delta / cos0).min(std::f32::consts::PI);
    let dlam_pix = 2.0 * std::f32::consts::PI / nx as f32;
    let i_pad = (dlam_pad / dlam_pix).ceil() as isize;

    let mut out = Vec::new();
    for j in j_min..=j_max {
        let phi = lat_of(j, ny);

        // tighten longitude window a bit by using local cos(phi)
        let cos_loc = phi.cos().abs().max(1e-6);
        let i_pad_loc = ((delta / cos_loc).min(std::f32::consts::PI) / dlam_pix).ceil() as isize;

        let i_start = i0 as isize - i_pad.min(i_pad_loc);
        let i_end = i0 as isize + i_pad.min(i_pad_loc);

        for ii in i_start..=i_end {
            let i = wrap_i(ii, nx);
            let lam = lon_of(i, nx);
            let ang = central_angle(phi0, lam0, phi, lam);
            if ang * radius <= d_meters {
                out.push(j * nx + i);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neighbors_within_various_latitudes() {
        let grid = GeoGrid {
            nx: 360,
            ny: 180,
            radius: 6_371_000.0,
        };
        let d = 500_000.0; // 500 km search radius

        // helper to pick index at given lat/lon
        fn idx_at(grid: &GeoGrid, lat_deg: f32, lon_deg: f32) -> usize {
            let j = ((90.0 - lat_deg) / 180.0 * grid.ny as f32).floor() as usize;
            let i = ((lon_deg + 180.0) / 360.0 * grid.nx as f32).floor() as usize;
            j * grid.nx + i
        }

        let equator_idx = idx_at(&grid, 0.0, 0.0);
        let mid_idx = idx_at(&grid, 45.0, 0.0);
        let pole_idx = idx_at(&grid, 89.5, 0.0); // near north pole

        for (name, idx) in [
            ("equator", equator_idx),
            ("mid", mid_idx),
            ("pole", pole_idx),
        ] {
            let neighbors = neighbors_within(&grid, idx, d);
            println!("{}: {} neighbors", name, neighbors.len());
            assert!(!neighbors.is_empty());
            // sanity: all results should be within the given distance
            for &k in &neighbors {
                let j = k / grid.nx;
                let i = k % grid.nx;
                let ang = central_angle(
                    lat_of(j, grid.ny),
                    lon_of(i, grid.nx),
                    lat_of(idx / grid.nx, grid.ny),
                    lon_of(idx % grid.nx, grid.nx),
                );
                assert!(ang * grid.radius <= d * 1.01);
            }
        }
    }

    #[test]
    fn test_neighbors_exhaustive_coverage() {
        let grid = GeoGrid {
            nx: 90,
            ny: 45,
            radius: 6_371_000.0,
        };
        let d = 1_000_000.0; // 1000 km

        // helper
        fn idx_at(grid: &GeoGrid, lat_deg: f32, lon_deg: f32) -> usize {
            let j = ((90.0 - lat_deg) / 180.0 * grid.ny as f32).floor() as usize;
            let i = ((lon_deg + 180.0) / 360.0 * grid.nx as f32).floor() as usize;
            j * grid.nx + i
        }

        let idx = idx_at(&grid, 45.0, 0.0);

        let neighbors = neighbors_within(&grid, idx, d);
        let set: std::collections::HashSet<_> = neighbors.iter().copied().collect();

        // now brute-force compute all grid points actually within D
        let (j0, i0) = (idx / grid.nx, idx % grid.nx);
        let phi0 = lat_of(j0, grid.ny);
        let lam0 = lon_of(i0, grid.nx);

        let mut missing = Vec::new();
        for j in 0..grid.ny {
            let phi = lat_of(j, grid.ny);
            for i in 0..grid.nx {
                let lam = lon_of(i, grid.nx);
                let ang = central_angle(phi0, lam0, phi, lam);
                let dist = ang * grid.radius;
                if dist <= d && !set.contains(&(j * grid.nx + i)) {
                    missing.push((i, j, dist));
                }
            }
        }

        if !missing.is_empty() {
            for (i, j, dist) in &missing[..missing.len().min(10)] {
                eprintln!("missing i={}, j={}, dist={:.1} m", i, j, dist);
            }
        }
        assert!(
            missing.is_empty(),
            "Found {} missing points within great-circle distance D",
            missing.len()
        );
    }
}
