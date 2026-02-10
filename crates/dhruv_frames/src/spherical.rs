//! Cartesian ↔ Spherical coordinate conversion.

use std::f64::consts::PI;

/// Spherical coordinates: longitude, latitude, distance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SphericalCoords {
    /// Longitude in radians, range [0, 2π).
    /// Measured in the x-y plane from +x toward +y.
    pub lon_rad: f64,
    /// Latitude in radians, range [-π/2, π/2].
    /// Elevation above the x-y plane.
    pub lat_rad: f64,
    /// Distance from origin in km.
    pub distance_km: f64,
}

impl SphericalCoords {
    /// Longitude in degrees, range [0, 360).
    pub fn lon_deg(&self) -> f64 {
        self.lon_rad.to_degrees()
    }

    /// Latitude in degrees, range [-90, 90].
    pub fn lat_deg(&self) -> f64 {
        self.lat_rad.to_degrees()
    }
}

/// Convert Cartesian `[x, y, z]` (km) to spherical coordinates.
///
/// Longitude is measured in the x-y plane from +x toward +y.
/// Latitude is elevation above the x-y plane.
pub fn cartesian_to_spherical(xyz: &[f64; 3]) -> SphericalCoords {
    let x = xyz[0];
    let y = xyz[1];
    let z = xyz[2];

    let r = (x * x + y * y + z * z).sqrt();

    if r == 0.0 {
        return SphericalCoords {
            lon_rad: 0.0,
            lat_rad: 0.0,
            distance_km: 0.0,
        };
    }

    let lon = y.atan2(x);
    let lat = (z / r).asin();

    SphericalCoords {
        lon_rad: if lon < 0.0 { lon + 2.0 * PI } else { lon },
        lat_rad: lat,
        distance_km: r,
    }
}

/// Spherical state: position (lon, lat, distance) plus angular velocities.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SphericalState {
    /// Longitude in radians, range [0, 2π).
    pub lon_rad: f64,
    /// Latitude in radians, range [-π/2, π/2].
    pub lat_rad: f64,
    /// Distance from origin in km.
    pub distance_km: f64,
    /// Longitude rate of change in rad/s.
    pub lon_speed: f64,
    /// Latitude rate of change in rad/s.
    pub lat_speed: f64,
    /// Radial velocity in km/s.
    pub distance_speed: f64,
}

/// Convert full Cartesian state (position + velocity) to spherical state
/// with angular velocities.
///
/// Derives `dlon/dt`, `dlat/dt`, `dr/dt` from standard vector calculus.
/// Degenerate cases (r ≈ 0 or rxy ≈ 0) set speeds to zero.
pub fn cartesian_state_to_spherical_state(
    pos: &[f64; 3],
    vel: &[f64; 3],
) -> SphericalState {
    let (x, y, z) = (pos[0], pos[1], pos[2]);
    let (vx, vy, vz) = (vel[0], vel[1], vel[2]);

    let r_sq = x * x + y * y + z * z;
    let r = r_sq.sqrt();

    const TINY: f64 = 1e-30;

    if r < TINY {
        return SphericalState {
            lon_rad: 0.0,
            lat_rad: 0.0,
            distance_km: 0.0,
            lon_speed: 0.0,
            lat_speed: 0.0,
            distance_speed: 0.0,
        };
    }

    let rxy_sq = x * x + y * y;
    let rxy = rxy_sq.sqrt();

    let lon = {
        let raw = y.atan2(x);
        if raw < 0.0 { raw + 2.0 * PI } else { raw }
    };
    let lat = (z / r).asin();

    let dr_dt = (x * vx + y * vy + z * vz) / r;

    let (dlon_dt, dlat_dt) = if rxy_sq < TINY {
        (0.0, 0.0)
    } else {
        let dlon = (x * vy - y * vx) / rxy_sq;
        let dlat = (vz * rxy_sq - z * (x * vx + y * vy)) / (r_sq * rxy);
        (dlon, dlat)
    };

    SphericalState {
        lon_rad: lon,
        lat_rad: lat,
        distance_km: r,
        lon_speed: dlon_dt,
        lat_speed: dlat_dt,
        distance_speed: dr_dt,
    }
}

/// Convert spherical coordinates back to Cartesian `[x, y, z]` (km).
pub fn spherical_to_cartesian(s: &SphericalCoords) -> [f64; 3] {
    let cos_lat = s.lat_rad.cos();
    [
        s.distance_km * cos_lat * s.lon_rad.cos(),
        s.distance_km * cos_lat * s.lon_rad.sin(),
        s.distance_km * s.lat_rad.sin(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-10;

    #[test]
    fn along_x_axis() {
        let s = cartesian_to_spherical(&[1.0e8, 0.0, 0.0]);
        assert!((s.lon_rad - 0.0).abs() < EPS);
        assert!((s.lat_rad - 0.0).abs() < EPS);
        assert!((s.distance_km - 1.0e8).abs() < EPS);
    }

    #[test]
    fn along_y_axis() {
        let s = cartesian_to_spherical(&[0.0, 1.0e8, 0.0]);
        assert!((s.lon_rad - PI / 2.0).abs() < EPS);
        assert!((s.lat_rad - 0.0).abs() < EPS);
    }

    #[test]
    fn along_negative_x() {
        let s = cartesian_to_spherical(&[-1.0e8, 0.0, 0.0]);
        assert!((s.lon_rad - PI).abs() < EPS);
    }

    #[test]
    fn along_z_axis() {
        let s = cartesian_to_spherical(&[0.0, 0.0, 1.0e8]);
        assert!((s.lat_rad - PI / 2.0).abs() < EPS);
        assert!((s.distance_km - 1.0e8).abs() < EPS);
    }

    #[test]
    fn roundtrip() {
        let xyz = [1.234e8, -5.678e7, 3.456e7];
        let s = cartesian_to_spherical(&xyz);
        let back = spherical_to_cartesian(&s);
        for i in 0..3 {
            assert!(
                (xyz[i] - back[i]).abs() < EPS * xyz[i].abs().max(1.0),
                "axis {i}: {:.10e} != {:.10e}",
                xyz[i],
                back[i]
            );
        }
    }

    #[test]
    fn zero_vector() {
        let s = cartesian_to_spherical(&[0.0, 0.0, 0.0]);
        assert_eq!(s.distance_km, 0.0);
    }

    #[test]
    fn longitude_always_positive() {
        // Negative x, negative y → third quadrant → lon in [π, 3π/2)
        let s = cartesian_to_spherical(&[-1.0, -1.0, 0.0]);
        assert!(s.lon_rad >= 0.0 && s.lon_rad < 2.0 * PI);
    }

    #[test]
    fn spherical_state_along_x_with_y_velocity() {
        // Body at (R, 0, 0) moving purely in +y → lon_speed > 0, lat_speed ≈ 0
        let r = 1.0e8;
        let v = 30.0; // km/s
        let s = cartesian_state_to_spherical_state(&[r, 0.0, 0.0], &[0.0, v, 0.0]);
        assert!((s.lon_rad - 0.0).abs() < EPS);
        assert!((s.lat_rad - 0.0).abs() < EPS);
        assert!((s.distance_km - r).abs() < EPS);
        // dlon/dt = (x*vy - y*vx) / rxy² = (R*v) / R² = v/R
        assert!((s.lon_speed - v / r).abs() < EPS);
        assert!(s.lat_speed.abs() < EPS);
        assert!(s.distance_speed.abs() < EPS);
    }

    #[test]
    fn spherical_state_along_x_with_z_velocity() {
        // Body at (R, 0, 0) moving purely in +z → lat_speed > 0, lon_speed ≈ 0
        let r = 1.0e8;
        let v = 30.0;
        let s = cartesian_state_to_spherical_state(&[r, 0.0, 0.0], &[0.0, 0.0, v]);
        assert!(s.lat_speed > 0.0);
        assert!(s.lon_speed.abs() < EPS);
        // dlat/dt = (vz * rxy² - z*(x*vx+y*vy)) / (r²*rxy)
        //         = (v * R² - 0) / (R² * R) = v / R
        assert!((s.lat_speed - v / r).abs() < EPS);
        assert!(s.distance_speed.abs() < EPS);
    }

    #[test]
    fn spherical_state_position_matches_existing() {
        let pos = [1.234e8, -5.678e7, 3.456e7];
        let vel = [10.0, -20.0, 5.0];
        let s = cartesian_state_to_spherical_state(&pos, &vel);
        let c = cartesian_to_spherical(&pos);
        assert!((s.lon_rad - c.lon_rad).abs() < EPS);
        assert!((s.lat_rad - c.lat_rad).abs() < EPS);
        assert!((s.distance_km - c.distance_km).abs() < EPS);
    }

    #[test]
    fn spherical_state_zero_vector() {
        let s = cartesian_state_to_spherical_state(&[0.0, 0.0, 0.0], &[0.0, 0.0, 0.0]);
        assert_eq!(s.distance_km, 0.0);
        assert_eq!(s.lon_speed, 0.0);
        assert_eq!(s.lat_speed, 0.0);
        assert_eq!(s.distance_speed, 0.0);
    }
}
