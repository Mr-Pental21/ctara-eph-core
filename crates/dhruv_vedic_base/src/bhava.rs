//! Bhava (house) system computation for 10 house division methods.
//!
//! Implements Equal, Surya Siddhanta, Sripati (Porphyry), KP (Placidus),
//! Koch, Regiomontanus, Campanus, Axial Rotation, Topocentric (Polich-Page),
//! and Alcabitus house systems.
//!
//! Sources: standard spherical astronomy (Meeus, Montenbruck & Pfleger),
//! Polich/Page original 1961 methodology, IAU 1976 obliquity.
//! See `docs/clean_room_bhava.md`.

use std::f64::consts::{PI, TAU};

use dhruv_core::{Body, Engine, Frame, Observer, Query};
use dhruv_frames::{OBLIQUITY_J2000_RAD, cartesian_to_spherical};
use dhruv_time::{LeapSecondKernel, jd_to_tdb_seconds, tdb_seconds_to_jd};

use crate::bhava_types::{
    Bhava, BhavaConfig, BhavaReferenceMode, BhavaResult, BhavaStartingPoint, BhavaSystem,
    normalize_deg,
};
use crate::error::VedicError;
use crate::lagna::{compute_lst_rad_pub, lagna_mc_ramc_from_lst};
use crate::riseset_types::GeoLocation;

use dhruv_time::EopKernel;

/// Maximum latitude (degrees) for time-based house systems.
const MAX_LATITUDE_DEG: f64 = 66.5;

/// Compute bhava cusps for the given configuration.
///
/// # Arguments
/// * `engine` — ephemeris engine (needed for BodyLongitude starting point)
/// * `lsk` — leap second kernel for time conversions
/// * `eop` — IERS EOP for UTC->UT1 conversion
/// * `location` — geographic observer location
/// * `jd_utc` — Julian Date in UTC
/// * `config` — house system, starting point, reference mode
///
/// # Returns
/// A `BhavaResult` with 12 bhavas, Ascendant, and MC.
pub fn compute_bhavas(
    engine: &Engine,
    lsk: &LeapSecondKernel,
    eop: &EopKernel,
    location: &GeoLocation,
    jd_utc: f64,
    config: &BhavaConfig,
) -> Result<BhavaResult, VedicError> {
    // Compute LST once, then derive Lagna, MC, RAMC
    let lst = compute_lst_rad_pub(lsk, eop, location, jd_utc)?;
    let lat_rad = location.latitude_rad();
    let (asc_rad, mc_rad, ramc) = lagna_mc_ramc_from_lst(lst, lat_rad);
    let eps = OBLIQUITY_J2000_RAD;

    let asc_deg = normalize_deg(asc_rad.to_degrees());
    let mc_deg = normalize_deg(mc_rad.to_degrees());

    // Resolve starting point
    let start_deg =
        resolve_starting_point_deg(engine, &config.starting_point, asc_deg, jd_utc, lsk)?;

    // Compute raw cusps (12 ecliptic longitudes in degrees)
    let cusps = match config.system {
        BhavaSystem::Equal | BhavaSystem::SuryaSiddhanta => compute_equal(start_deg),
        BhavaSystem::Sripati => compute_sripati(asc_deg, mc_deg),
        BhavaSystem::KP => {
            check_latitude(location)?;
            compute_placidus(asc_deg, mc_deg, ramc, lat_rad, eps)?
        }
        BhavaSystem::Koch => {
            check_latitude(location)?;
            compute_koch(asc_deg, mc_deg, ramc, lat_rad, eps)?
        }
        BhavaSystem::Regiomontanus => compute_regiomontanus(ramc, lat_rad, eps),
        BhavaSystem::Campanus => compute_campanus(ramc, lat_rad, eps),
        BhavaSystem::AxialRotation => compute_axial_rotation(ramc, eps),
        BhavaSystem::Topocentric => {
            check_latitude(location)?;
            compute_topocentric(asc_deg, mc_deg, ramc, lat_rad, eps)?
        }
        BhavaSystem::Alcabitus => {
            check_latitude(location)?;
            compute_alcabitus(asc_deg, mc_deg, ramc, lat_rad, eps)?
        }
    };

    // For non-equal systems with BodyLongitude/CustomDeg starting point,
    // the Asc/MC/IC/Desc are still computed normally; only the cusp numbering
    // is shifted for equal systems
    let final_cusps = if !config.system.is_equal_division()
        && config.starting_point != BhavaStartingPoint::Lagna
    {
        // For quadrant systems, starting point only applies to equal division
        cusps
    } else {
        cusps
    };

    let bhavas = build_bhavas(&final_cusps, config.reference_mode);

    Ok(BhavaResult {
        bhavas,
        lagna_deg: asc_deg,
        mc_deg,
    })
}

/// Check that latitude is within the limit for time-based systems.
fn check_latitude(location: &GeoLocation) -> Result<(), VedicError> {
    if location.latitude_deg.abs() > MAX_LATITUDE_DEG {
        return Err(VedicError::InvalidLocation(
            "latitude exceeds 66.5 deg limit for this house system",
        ));
    }
    Ok(())
}

/// Resolve the starting point to an ecliptic degree.
fn resolve_starting_point_deg(
    engine: &Engine,
    starting_point: &BhavaStartingPoint,
    asc_deg: f64,
    jd_utc: f64,
    lsk: &LeapSecondKernel,
) -> Result<f64, VedicError> {
    match starting_point {
        BhavaStartingPoint::Lagna => Ok(asc_deg),
        BhavaStartingPoint::CustomDeg(deg) => Ok(normalize_deg(*deg)),
        BhavaStartingPoint::BodyLongitude(body) => {
            body_ecliptic_longitude_deg(engine, *body, jd_utc, lsk)
        }
    }
}

/// Query a body's geocentric ecliptic longitude in degrees.
fn body_ecliptic_longitude_deg(
    engine: &Engine,
    body: Body,
    jd_utc: f64,
    lsk: &LeapSecondKernel,
) -> Result<f64, VedicError> {
    // Convert UTC to TDB
    let utc_s = jd_to_tdb_seconds(jd_utc);
    let tdb_s = lsk.utc_to_tdb(utc_s);
    let jd_tdb = tdb_seconds_to_jd(tdb_s);

    let query = Query {
        target: body,
        observer: Observer::Body(Body::Earth),
        frame: Frame::EclipticJ2000,
        epoch_tdb_jd: jd_tdb,
    };
    let state = engine.query(query)?;
    let sph = cartesian_to_spherical(&state.position_km);
    Ok(normalize_deg(sph.lon_deg))
}

/// Equal house division: cusp[i] = start + i*30.
fn compute_equal(start_deg: f64) -> [f64; 12] {
    let mut cusps = [0.0; 12];
    for (i, cusp) in cusps.iter_mut().enumerate() {
        *cusp = normalize_deg(start_deg + (i as f64) * 30.0);
    }
    cusps
}

/// Sripati (Porphyry): trisect the four quadrant arcs between Asc/IC/Desc/MC.
///
/// Cusp 1 = Asc, Cusp 4 = IC, Cusp 7 = Desc, Cusp 10 = MC.
/// Cusps 2,3 trisect (Asc→IC); Cusps 5,6 trisect (IC→Desc);
/// Cusps 8,9 trisect (Desc→MC); Cusps 11,12 trisect (MC→Asc).
fn compute_sripati(asc_deg: f64, mc_deg: f64) -> [f64; 12] {
    let desc_deg = normalize_deg(asc_deg + 180.0);
    let ic_deg = normalize_deg(mc_deg + 180.0);

    let mut cusps = [0.0; 12];
    cusps[0] = asc_deg; // cusp 1
    cusps[3] = ic_deg; // cusp 4
    cusps[6] = desc_deg; // cusp 7
    cusps[9] = mc_deg; // cusp 10

    // Trisect Asc -> IC (quadrant 1: houses 2, 3)
    let arc1 = arc_forward(asc_deg, ic_deg);
    cusps[1] = normalize_deg(asc_deg + arc1 / 3.0);
    cusps[2] = normalize_deg(asc_deg + 2.0 * arc1 / 3.0);

    // Trisect IC -> Desc (quadrant 2: houses 5, 6)
    let arc2 = arc_forward(ic_deg, desc_deg);
    cusps[4] = normalize_deg(ic_deg + arc2 / 3.0);
    cusps[5] = normalize_deg(ic_deg + 2.0 * arc2 / 3.0);

    // Trisect Desc -> MC (quadrant 3: houses 8, 9)
    let arc3 = arc_forward(desc_deg, mc_deg);
    cusps[7] = normalize_deg(desc_deg + arc3 / 3.0);
    cusps[8] = normalize_deg(desc_deg + 2.0 * arc3 / 3.0);

    // Trisect MC -> Asc (quadrant 4: houses 11, 12)
    let arc4 = arc_forward(mc_deg, asc_deg);
    cusps[10] = normalize_deg(mc_deg + arc4 / 3.0);
    cusps[11] = normalize_deg(mc_deg + 2.0 * arc4 / 3.0);

    cusps
}

/// Placidus (KP) house system: time-based semi-arc trisection.
///
/// Cusps 1 = Asc, 4 = IC, 7 = Desc, 10 = MC.
/// Intermediate cusps are found by trisecting the diurnal/nocturnal semi-arc
/// in terms of time.
fn compute_placidus(
    asc_deg: f64,
    mc_deg: f64,
    ramc: f64,
    lat: f64,
    eps: f64,
) -> Result<[f64; 12], VedicError> {
    let desc_deg = normalize_deg(asc_deg + 180.0);
    let ic_deg = normalize_deg(mc_deg + 180.0);

    let mut cusps = [0.0; 12];
    cusps[0] = asc_deg;
    cusps[3] = ic_deg;
    cusps[6] = desc_deg;
    cusps[9] = mc_deg;

    // Cusps 11, 12: MC -> Asc (above horizon, diurnal semi-arc trisection)
    cusps[10] = placidus_cusp(ramc, lat, eps, 1.0 / 3.0, true)?;
    cusps[11] = placidus_cusp(ramc, lat, eps, 2.0 / 3.0, true)?;

    // Cusps 2, 3: Asc -> IC (below horizon, nocturnal semi-arc trisection)
    cusps[1] = placidus_cusp(ramc + PI, lat, eps, 1.0 / 3.0, false)?;
    cusps[2] = placidus_cusp(ramc + PI, lat, eps, 2.0 / 3.0, false)?;

    // Cusps 5, 6: IC -> Desc (below horizon)
    cusps[4] = normalize_deg(cusps[10] + 180.0);
    cusps[5] = normalize_deg(cusps[11] + 180.0);

    // Cusps 8, 9: Desc -> MC (above horizon)
    cusps[7] = normalize_deg(cusps[1] + 180.0);
    cusps[8] = normalize_deg(cusps[2] + 180.0);

    Ok(cusps)
}

/// Compute a single Placidus cusp by iterative semi-arc trisection.
///
/// `fraction` = 1/3 or 2/3 of the semi-arc.
/// `above_horizon` = true for houses 10->1 (diurnal), false for 1->4 (nocturnal).
fn placidus_cusp(
    ramc: f64,
    lat: f64,
    eps: f64,
    fraction: f64,
    above_horizon: bool,
) -> Result<f64, VedicError> {
    let mut ra = ramc + fraction * PI / 2.0;
    if !above_horizon {
        ra = ramc + PI + fraction * PI / 2.0;
    }

    for _ in 0..50 {
        let dec = (eps.sin() * ra.sin()).asin();
        let semi_arc = semi_arc_rad(dec, lat, above_horizon);
        let f = fraction * semi_arc;

        let new_ra = if above_horizon {
            ramc + f
        } else {
            ramc + PI + f
        };

        if (new_ra - ra).abs() < 1e-10 {
            ra = new_ra;
            break;
        }
        ra = new_ra;
    }

    Ok(normalize_deg(
        equator_to_ecliptic_longitude_rad(ra, eps).to_degrees(),
    ))
}

/// Koch house system: MC-to-horizon time division.
///
/// Uses the time it takes for the MC degree to rise to the Ascendant.
fn compute_koch(
    asc_deg: f64,
    mc_deg: f64,
    ramc: f64,
    lat: f64,
    eps: f64,
) -> Result<[f64; 12], VedicError> {
    let desc_deg = normalize_deg(asc_deg + 180.0);
    let ic_deg = normalize_deg(mc_deg + 180.0);

    // Declination of MC
    let dec_mc = (eps.sin() * ramc.sin()).asin();
    // Semi-arc of MC degree
    let sa = semi_arc_rad(dec_mc, lat, true);

    // Koch cusp = RAMC + fraction * SA, projected to ecliptic
    let mut cusps = [0.0; 12];
    cusps[0] = asc_deg;
    cusps[3] = ic_deg;
    cusps[6] = desc_deg;
    cusps[9] = mc_deg;

    // Houses 11, 12
    cusps[10] = normalize_deg(equator_to_ecliptic_longitude_rad(ramc + sa / 3.0, eps).to_degrees());
    cusps[11] =
        normalize_deg(equator_to_ecliptic_longitude_rad(ramc + 2.0 * sa / 3.0, eps).to_degrees());

    // Houses 2, 3 (below horizon)
    cusps[1] =
        normalize_deg(equator_to_ecliptic_longitude_rad(ramc + PI + sa / 3.0, eps).to_degrees());
    cusps[2] = normalize_deg(
        equator_to_ecliptic_longitude_rad(ramc + PI + 2.0 * sa / 3.0, eps).to_degrees(),
    );

    // Houses 5, 6 (opposite of 11, 12)
    cusps[4] = normalize_deg(cusps[10] + 180.0);
    cusps[5] = normalize_deg(cusps[11] + 180.0);

    // Houses 8, 9 (opposite of 2, 3)
    cusps[7] = normalize_deg(cusps[1] + 180.0);
    cusps[8] = normalize_deg(cusps[2] + 180.0);

    Ok(cusps)
}

/// Regiomontanus: 30-degree equator arcs from East Point, projected to ecliptic.
fn compute_regiomontanus(ramc: f64, lat: f64, eps: f64) -> [f64; 12] {
    let mut cusps = [0.0; 12];
    for (i, cusp) in cusps.iter_mut().enumerate() {
        *cusp = normalize_deg(regiomontanus_cusp_rad(ramc, lat, eps, i as i32).to_degrees());
    }
    cusps
}

/// Compute a single Regiomontanus cusp.
///
/// The Regiomontanus method divides the celestial equator into 30-degree
/// segments starting from the East Point (RAMC + 90 deg), then projects
/// each division point to the ecliptic via the local horizon.
fn regiomontanus_cusp_rad(ramc: f64, lat: f64, eps: f64, house_index: i32) -> f64 {
    let h = (house_index as f64) * PI / 6.0; // 0, 30, 60, ... deg in radians
    let ra_cusp = ramc + PI / 2.0 + h;

    // Declination of the house cusp point on the Regiomontanus circle
    let dec = (lat.tan() * h.sin() / (PI / 2.0).cos()).atan();

    // Project equatorial (RA, Dec) to ecliptic longitude
    let sin_lon = ra_cusp.sin() * eps.cos() + dec.tan() * eps.sin();
    let cos_lon = ra_cusp.cos();
    f64::atan2(sin_lon, cos_lon).rem_euclid(TAU)
}

/// Campanus: 30-degree prime vertical arcs, projected to ecliptic.
fn compute_campanus(ramc: f64, lat: f64, eps: f64) -> [f64; 12] {
    let mut cusps = [0.0; 12];
    for (i, cusp) in cusps.iter_mut().enumerate() {
        *cusp = normalize_deg(campanus_cusp_rad(ramc, lat, eps, i as i32).to_degrees());
    }
    cusps
}

/// Compute a single Campanus cusp.
///
/// Divides the prime vertical into 30-degree arcs, then projects each
/// division point onto the ecliptic.
fn campanus_cusp_rad(ramc: f64, lat: f64, eps: f64, house_index: i32) -> f64 {
    let a = (house_index as f64) * PI / 6.0; // azimuth on prime vertical

    // Altitude and azimuth on the prime vertical
    let dec = (lat.sin() * a.cos()).asin();
    let ra_offset = f64::atan2(a.sin(), lat.cos() * a.cos());
    let ra = ramc + PI / 2.0 + ra_offset;

    // Project to ecliptic longitude
    let sin_lon = ra.sin() * eps.cos() + dec.tan() * eps.sin();
    let cos_lon = ra.cos();
    f64::atan2(sin_lon, cos_lon).rem_euclid(TAU)
}

/// Axial Rotation (Meridian) house system.
///
/// RAMC + 30-degree equator arcs, projected to ecliptic. Independent of latitude.
fn compute_axial_rotation(ramc: f64, eps: f64) -> [f64; 12] {
    let mut cusps = [0.0; 12];
    for (i, cusp) in cusps.iter_mut().enumerate() {
        let ra = ramc + (i as f64) * PI / 6.0;
        *cusp = normalize_deg(equator_to_ecliptic_longitude_rad(ra, eps).to_degrees());
    }
    cusps
}

/// Topocentric (Polich-Page): tangent-ratio semi-arc method.
///
/// Similar to Placidus but uses tangent-ratio interpolation.
fn compute_topocentric(
    asc_deg: f64,
    mc_deg: f64,
    ramc: f64,
    lat: f64,
    eps: f64,
) -> Result<[f64; 12], VedicError> {
    let desc_deg = normalize_deg(asc_deg + 180.0);
    let ic_deg = normalize_deg(mc_deg + 180.0);

    let mut cusps = [0.0; 12];
    cusps[0] = asc_deg;
    cusps[3] = ic_deg;
    cusps[6] = desc_deg;
    cusps[9] = mc_deg;

    // The Topocentric method uses modified latitudes for intermediate cusps:
    // House 11: tan(phi_11) = tan(phi) / 3
    // House 12: tan(phi_12) = 2*tan(phi) / 3
    // Then proceeds like Placidus with the modified latitude.

    let tan_lat = lat.tan();

    // Cusps 11, 12 (MC -> Asc quadrant)
    let lat_11 = (tan_lat / 3.0).atan();
    let lat_12 = (2.0 * tan_lat / 3.0).atan();

    cusps[10] = topocentric_cusp(ramc, lat_11, eps, 1.0 / 3.0, true)?;
    cusps[11] = topocentric_cusp(ramc, lat_12, eps, 2.0 / 3.0, true)?;

    // Cusps 2, 3 (Asc -> IC quadrant)
    cusps[1] = topocentric_cusp(ramc + PI, lat_11, eps, 1.0 / 3.0, false)?;
    cusps[2] = topocentric_cusp(ramc + PI, lat_12, eps, 2.0 / 3.0, false)?;

    // Opposite cusps
    cusps[4] = normalize_deg(cusps[10] + 180.0);
    cusps[5] = normalize_deg(cusps[11] + 180.0);
    cusps[7] = normalize_deg(cusps[1] + 180.0);
    cusps[8] = normalize_deg(cusps[2] + 180.0);

    Ok(cusps)
}

/// Compute a single Topocentric cusp (uses modified latitude).
fn topocentric_cusp(
    ramc: f64,
    modified_lat: f64,
    eps: f64,
    fraction: f64,
    above_horizon: bool,
) -> Result<f64, VedicError> {
    let mut ra = ramc + fraction * PI / 2.0;
    if !above_horizon {
        ra = ramc + PI + fraction * PI / 2.0;
    }

    for _ in 0..50 {
        let dec = (eps.sin() * ra.sin()).asin();
        let semi_arc = semi_arc_rad(dec, modified_lat, above_horizon);
        let f = fraction * semi_arc;

        let new_ra = if above_horizon {
            ramc + f
        } else {
            ramc + PI + f
        };

        if (new_ra - ra).abs() < 1e-10 {
            ra = new_ra;
            break;
        }
        ra = new_ra;
    }

    Ok(normalize_deg(
        equator_to_ecliptic_longitude_rad(ra, eps).to_degrees(),
    ))
}

/// Alcabitus: semi-arc equator division + ecliptic projection.
///
/// Divides the diurnal and nocturnal semi-arcs of the Ascendant on the
/// equator into thirds, then projects to the ecliptic.
fn compute_alcabitus(
    asc_deg: f64,
    mc_deg: f64,
    ramc: f64,
    lat: f64,
    eps: f64,
) -> Result<[f64; 12], VedicError> {
    let desc_deg = normalize_deg(asc_deg + 180.0);
    let ic_deg = normalize_deg(mc_deg + 180.0);

    // Declination of the Ascendant
    let ra_asc = ramc + PI / 2.0; // RA of Ascendant ≈ RAMC + 90 deg
    let dec_asc = (eps.sin() * ra_asc.sin()).asin();
    let sa_diurnal = semi_arc_rad(dec_asc, lat, true);

    let mut cusps = [0.0; 12];
    cusps[0] = asc_deg;
    cusps[3] = ic_deg;
    cusps[6] = desc_deg;
    cusps[9] = mc_deg;

    // Cusps 11, 12: divide the diurnal semi-arc of the Ascendant
    cusps[10] =
        normalize_deg(equator_to_ecliptic_longitude_rad(ramc + sa_diurnal / 3.0, eps).to_degrees());
    cusps[11] = normalize_deg(
        equator_to_ecliptic_longitude_rad(ramc + 2.0 * sa_diurnal / 3.0, eps).to_degrees(),
    );

    // Cusps 2, 3: divide the nocturnal semi-arc
    let sa_nocturnal = PI - sa_diurnal;
    cusps[1] = normalize_deg(
        equator_to_ecliptic_longitude_rad(ramc + PI + sa_nocturnal / 3.0, eps).to_degrees(),
    );
    cusps[2] = normalize_deg(
        equator_to_ecliptic_longitude_rad(ramc + PI + 2.0 * sa_nocturnal / 3.0, eps).to_degrees(),
    );

    // Opposite cusps
    cusps[4] = normalize_deg(cusps[10] + 180.0);
    cusps[5] = normalize_deg(cusps[11] + 180.0);
    cusps[7] = normalize_deg(cusps[1] + 180.0);
    cusps[8] = normalize_deg(cusps[2] + 180.0);

    Ok(cusps)
}

/// Diurnal or nocturnal semi-arc in radians.
///
/// `semi_arc = acos(-tan(dec) * tan(lat))`
/// For diurnal: returns the semi-arc above the horizon.
/// For nocturnal: returns pi - diurnal semi-arc.
fn semi_arc_rad(dec: f64, lat: f64, diurnal: bool) -> f64 {
    let cos_ha = -(dec.tan() * lat.tan());
    let ha = cos_ha.clamp(-1.0, 1.0).acos();
    if diurnal { ha } else { PI - ha }
}

/// Convert equatorial RA to ecliptic longitude.
///
/// For a point on the celestial equator (dec=0):
/// `lon_ecl = atan2(sin(RA)*cos(eps), cos(RA))`
///
/// For a point with declination:
/// `lon_ecl = atan2(sin(RA)*cos(eps) + tan(dec)*sin(eps), cos(RA))`
///
/// House cusps computed via equator division have dec derived from RA and eps:
/// `dec = asin(sin(eps)*sin(RA))`, so we use the full formula.
fn equator_to_ecliptic_longitude_rad(ra: f64, eps: f64) -> f64 {
    // For points where dec = asin(sin(eps)*sin(RA)):
    let dec = (eps.sin() * ra.sin()).asin();
    let sin_lon = ra.sin() * eps.cos() + dec.tan() * eps.sin();
    let cos_lon = ra.cos();
    f64::atan2(sin_lon, cos_lon).rem_euclid(TAU)
}

/// Forward arc from a to b in degrees (always positive, 0..360).
fn arc_forward(a: f64, b: f64) -> f64 {
    (b - a).rem_euclid(360.0)
}

/// Build the 12 Bhava structs from cusp degrees, applying reference mode.
fn build_bhavas(cusps_deg: &[f64; 12], reference_mode: BhavaReferenceMode) -> [Bhava; 12] {
    let adjusted = match reference_mode {
        BhavaReferenceMode::StartOfFirst => *cusps_deg,
        BhavaReferenceMode::MiddleOfFirst => {
            // Shift all cusps back by half the width of bhava 1
            let width1 = arc_forward(cusps_deg[0], cusps_deg[1]);
            let shift = width1 / 2.0;
            let mut shifted = [0.0; 12];
            for i in 0..12 {
                shifted[i] = normalize_deg(cusps_deg[i] - shift);
            }
            shifted
        }
    };

    let mut bhavas = [Bhava {
        number: 0,
        cusp_deg: 0.0,
        start_deg: 0.0,
        end_deg: 0.0,
    }; 12];

    for i in 0..12 {
        let next = (i + 1) % 12;
        bhavas[i] = Bhava {
            number: (i as u8) + 1,
            cusp_deg: adjusted[i],
            start_deg: adjusted[i],
            end_deg: adjusted[next],
        };
    }

    bhavas
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_cusps_30_deg_apart() {
        let cusps = compute_equal(100.0);
        for i in 0..12 {
            let expected = normalize_deg(100.0 + (i as f64) * 30.0);
            assert!(
                (cusps[i] - expected).abs() < 1e-10,
                "cusp[{i}] = {}, expected {expected}",
                cusps[i]
            );
        }
    }

    #[test]
    fn equal_cusps_wrap_around() {
        let cusps = compute_equal(350.0);
        assert!((cusps[0] - 350.0).abs() < 1e-10);
        assert!((cusps[1] - 20.0).abs() < 1e-10); // 350 + 30 = 380 -> 20
        assert!((cusps[11] - 320.0).abs() < 1e-10); // 350 + 330 = 680 -> 320
    }

    #[test]
    fn sripati_angular_cusps() {
        let cusps = compute_sripati(90.0, 0.0);
        // cusp 1 = Asc = 90
        assert!((cusps[0] - 90.0).abs() < 1e-10);
        // cusp 10 = MC = 0
        assert!((cusps[9] - 0.0).abs() < 1e-10 || (cusps[9] - 360.0).abs() < 1e-10);
        // cusp 7 = Desc = 270
        assert!((cusps[6] - 270.0).abs() < 1e-10);
        // cusp 4 = IC = 180
        assert!((cusps[3] - 180.0).abs() < 1e-10);
    }

    #[test]
    fn sripati_trisection() {
        let asc = 90.0;
        let mc = 0.0;
        let cusps = compute_sripati(asc, mc);

        // Asc -> IC = 90 deg arc, trisected = 30 deg each
        let arc = arc_forward(asc, 180.0);
        assert!((arc - 90.0).abs() < 1e-10);
        assert!((cusps[1] - 120.0).abs() < 1e-10);
        assert!((cusps[2] - 150.0).abs() < 1e-10);
    }

    #[test]
    fn build_bhavas_continuity() {
        let cusps = compute_equal(0.0);
        let bhavas = build_bhavas(&cusps, BhavaReferenceMode::StartOfFirst);

        for i in 0..12 {
            let next = (i + 1) % 12;
            assert!(
                (bhavas[i].end_deg - bhavas[next].start_deg).abs() < 1e-10,
                "end[{i}]={} != start[{next}]={}",
                bhavas[i].end_deg,
                bhavas[next].start_deg
            );
        }
    }

    #[test]
    fn build_bhavas_numbers() {
        let cusps = compute_equal(0.0);
        let bhavas = build_bhavas(&cusps, BhavaReferenceMode::StartOfFirst);
        for i in 0..12 {
            assert_eq!(bhavas[i].number, (i as u8) + 1);
        }
    }

    #[test]
    fn reference_mode_middle_shifts() {
        let cusps = compute_equal(100.0);
        let bhavas_start = build_bhavas(&cusps, BhavaReferenceMode::StartOfFirst);
        let bhavas_mid = build_bhavas(&cusps, BhavaReferenceMode::MiddleOfFirst);

        // For equal houses, shift = 15 deg
        let diff = arc_forward(bhavas_mid[0].cusp_deg, bhavas_start[0].cusp_deg);
        assert!((diff - 15.0).abs() < 1e-10, "shift = {diff}, expected 15");
    }

    #[test]
    fn arc_forward_normal() {
        assert!((arc_forward(10.0, 40.0) - 30.0).abs() < 1e-10);
    }

    #[test]
    fn arc_forward_wrap() {
        assert!((arc_forward(350.0, 20.0) - 30.0).abs() < 1e-10);
    }

    #[test]
    fn equator_to_ecliptic_identity_at_zero() {
        // At RA=0, ecliptic longitude should also be 0
        let lon = equator_to_ecliptic_longitude_rad(0.0, OBLIQUITY_J2000_RAD);
        assert!(lon.abs() < 1e-10, "lon at RA=0 = {}", lon.to_degrees());
    }

    #[test]
    fn equator_to_ecliptic_at_90() {
        // At RA=90 deg, ecliptic longitude = 90 deg (equinox symmetry)
        let lon = equator_to_ecliptic_longitude_rad(PI / 2.0, OBLIQUITY_J2000_RAD);
        assert!(
            (lon - PI / 2.0).abs() < 1e-10,
            "lon at RA=90 = {} deg",
            lon.to_degrees()
        );
    }

    #[test]
    fn semi_arc_equator_equinox() {
        // At equator, dec=0: semi-arc = acos(0) = pi/2 = 6 hours
        let sa = semi_arc_rad(0.0, 0.0, true);
        assert!(
            (sa - PI / 2.0).abs() < 1e-10,
            "semi_arc at equator/equinox = {} rad",
            sa
        );
    }

    #[test]
    fn semi_arc_nocturnal_complement() {
        let dec = 10.0_f64.to_radians();
        let lat = 40.0_f64.to_radians();
        let diurnal = semi_arc_rad(dec, lat, true);
        let nocturnal = semi_arc_rad(dec, lat, false);
        assert!(
            (diurnal + nocturnal - PI).abs() < 1e-10,
            "diurnal + nocturnal = {} rad, expected pi",
            diurnal + nocturnal
        );
    }

    #[test]
    fn axial_rotation_cusps_valid() {
        let ramc = 1.5;
        let eps = OBLIQUITY_J2000_RAD;
        let cusps = compute_axial_rotation(ramc, eps);
        for i in 0..12 {
            assert!(
                cusps[i] >= 0.0 && cusps[i] < 360.0,
                "cusp[{i}] = {} out of range",
                cusps[i]
            );
        }
    }

    #[test]
    fn regiomontanus_cusps_valid() {
        let ramc = 1.0;
        let lat = 28.6_f64.to_radians();
        let eps = OBLIQUITY_J2000_RAD;
        let cusps = compute_regiomontanus(ramc, lat, eps);
        for i in 0..12 {
            assert!(
                cusps[i] >= 0.0 && cusps[i] < 360.0,
                "regiomontanus cusp[{i}] = {} out of range",
                cusps[i]
            );
        }
    }

    #[test]
    fn campanus_cusps_valid() {
        let ramc = 2.0;
        let lat = 28.6_f64.to_radians();
        let eps = OBLIQUITY_J2000_RAD;
        let cusps = compute_campanus(ramc, lat, eps);
        for i in 0..12 {
            assert!(
                cusps[i] >= 0.0 && cusps[i] < 360.0,
                "campanus cusp[{i}] = {} out of range",
                cusps[i]
            );
        }
    }
}
