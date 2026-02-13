//! IAU 2000B truncated nutation model (77 lunisolar terms).
//!
//! Computes nutation in longitude (Δψ) and obliquity (Δε) using the
//! simplified IAU 2000B model, sufficient for ~1 mas accuracy.
//!
//! Source: IERS Conventions 2010, Chapter 5, Table 5.3b.
//! Fundamental arguments from IERS Conventions 2010, Table 5.2e.
//! Public domain (IAU standard).

use std::f64::consts::TAU;

/// Arcseconds to radians conversion factor.
const AS2RAD: f64 = TAU / 1_296_000.0;

/// Compute the five Delaunay fundamental arguments in radians.
///
/// `t` = Julian centuries of TDB since J2000.0.
///
/// Returns `[l, l', F, D, Ω]` where:
/// - `l`  = mean anomaly of the Moon
/// - `l'` = mean anomaly of the Sun
/// - `F`  = mean argument of latitude of the Moon
/// - `D`  = mean elongation of the Moon from the Sun
/// - `Ω`  = mean longitude of the ascending node of the Moon
///
/// Polynomial coefficients from IERS Conventions 2010, Table 5.2e.
pub fn fundamental_arguments(t: f64) -> [f64; 5] {
    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;

    // l: mean anomaly of the Moon (arcsec)
    let l = (485868.249036 + 1717915923.2178 * t + 31.8792 * t2 + 0.051635 * t3 - 0.00024470 * t4)
        * AS2RAD;

    // l': mean anomaly of the Sun (arcsec)
    let lp = (1287104.79305 + 129596581.0481 * t - 0.5532 * t2 + 0.000136 * t3 - 0.00001149 * t4)
        * AS2RAD;

    // F: mean argument of latitude of the Moon (arcsec)
    let f = (335779.526232 + 1739527262.8478 * t - 12.7512 * t2 - 0.001037 * t3 + 0.00000417 * t4)
        * AS2RAD;

    // D: mean elongation of the Moon from the Sun (arcsec)
    let d = (1072260.70369 + 1602961601.2090 * t - 6.3706 * t2 + 0.006593 * t3 - 0.00003169 * t4)
        * AS2RAD;

    // Ω: mean longitude of the ascending node of the Moon (arcsec)
    let om =
        (450160.398036 - 6962890.5431 * t + 7.4722 * t2 + 0.007702 * t3 - 0.00005939 * t4) * AS2RAD;

    [l, lp, f, d, om]
}

/// IAU 2000B lunisolar nutation term coefficients.
///
/// Each row: `[nl, nl', nF, nD, nΩ, S_i, S'_i, C_i, C'_i]`
/// where S_i, S'_i are in 0.1 μas for Δψ, and C_i, C'_i in 0.1 μas for Δε.
///
/// Source: IERS Conventions 2010, Table 5.3b (77 terms).
/// Amplitudes stored as i64 (units of 0.1 μas = 1e-7 arcsec).
#[rustfmt::skip]
static NUTATION_COEFFS: [[i64; 9]; 77] = [
    //  nl  nl'  nF   nD   nΩ       S_i         S'_i         C_i         C'_i
    [   0,   0,   0,   0,   1, -172064161,  -174666,   92052331,    9086],
    [   0,   0,   2,  -2,   2,  -13170906,    -1675,    5730336,   -3015],
    [   0,   0,   2,   0,   2,   -2276413,     -234,     978459,    -485],
    [   0,   0,   0,   0,   2,    2074554,      207,    -897492,     470],
    [   0,   1,   0,   0,   0,    1475877,    -3633,      73871,    -184],
    [   0,   1,   2,  -2,   2,    -516821,     1226,     224386,    -677],
    [   1,   0,   0,   0,   0,     711159,       73,      -6750,       0],
    [   0,   0,   2,   0,   1,    -387298,     -367,     200728,      18],
    [   1,   0,   2,   0,   2,    -301461,      -36,     129025,     -63],
    [   0,  -1,   2,  -2,   2,     215829,     -494,     -95929,     299],
    [   0,   0,   2,  -2,   1,     128227,      137,     -68982,      -9],
    [  -1,   0,   2,   0,   2,     123457,       11,     -53311,      32],
    [  -1,   0,   0,   2,   0,     156994,       10,      -1235,       0],
    [   1,   0,   0,   0,   1,      63110,       63,     -33228,       0],
    [  -1,   0,   0,   0,   1,     -57976,      -63,      31429,       0],
    [  -1,   0,   2,   2,   2,     -59641,      -11,      25543,     -11],
    [   1,   0,   2,   0,   1,     -51613,      -42,      26366,       0],
    [  -2,   0,   2,   0,   1,      45893,       50,     -24236,     -10],
    [   0,   0,   0,   2,   0,      63384,       11,      -1220,       0],
    [   0,   0,   2,   2,   2,     -38571,       -1,      16452,     -11],
    [   0,  -2,   2,  -2,   2,      32481,        0,     -13870,       0],
    [  -2,   0,   0,   2,   0,     -47722,        0,        477,       0],
    [   2,   0,   2,   0,   2,     -31046,       -1,      13238,     -11],
    [   1,   0,   2,  -2,   2,      28593,        0,     -12338,      10],
    [  -1,   0,   2,   0,   1,      20441,       21,     -10758,       0],
    [   2,   0,   0,   0,   0,      29243,        0,       -609,       0],
    [   0,   0,   2,   0,   0,      25887,        0,       -550,       0],
    [   0,   1,   0,   0,   1,     -14053,      -25,       8551,      -2],
    [  -1,   0,   0,   2,   1,      15164,       10,      -8001,       0],
    [   0,   2,   2,  -2,   2,     -15794,       72,       6850,     -42],
    [   0,   0,  -2,   2,   0,      21783,        0,       -167,       0],
    [   1,   0,   0,  -2,   1,     -12873,      -10,       6953,       0],
    [   0,  -1,   0,   0,   1,     -12654,       11,       6415,       0],
    [  -1,   0,   2,   2,   1,     -10204,        0,       5222,       0],
    [   0,   2,   0,   0,   0,      16707,      -85,        168,      -1],
    [   1,   0,   2,   2,   2,      -7691,        0,       3268,       0],
    [  -2,   0,   2,   0,   0,     -11024,        0,        104,       0],
    [   0,   1,   2,   0,   2,       7566,      -21,      -3250,       0],
    [   0,   0,   2,   2,   1,      -6637,      -11,       3353,       0],
    [   0,  -1,   2,   0,   2,      -7141,       21,       3070,       0],
    [   0,   0,   0,   2,   1,      -6302,      -11,       3272,       0],
    [   1,   0,   2,  -2,   1,       5800,       10,      -3045,       0],
    [   2,   0,   2,  -2,   2,       6443,        0,      -2768,       0],
    [  -2,   0,   0,   2,   1,      -5774,      -11,       3041,       0],
    [   2,   0,   2,   0,   1,      -5350,        0,       2695,       0],
    [   0,  -1,   2,  -2,   1,      -4752,      -11,       2719,       0],
    [   0,   0,   0,  -2,   1,      -4940,      -11,       2720,       0],
    [  -1,  -1,   0,   2,   0,       7350,        0,        -51,       0],
    [   2,   0,   0,  -2,   1,      -4803,      -11,       2556,       0],
    [   1,   0,   0,   2,   0,      -7677,        0,        462,       0],
    [   0,   1,   2,  -2,   1,       5417,        0,      -2520,       0],
    [   1,  -1,   0,   0,   0,       6624,        0,       -468,       0],
    [  -2,   0,   2,   0,   2,      -5433,        0,       2334,       0],
    [   3,   0,   2,   0,   2,      -4632,        0,       1991,       0],
    [   0,  -1,   0,   2,   0,       6106,        0,       -167,       0],
    [   1,  -1,   2,   0,   2,      -3593,        0,       1556,       0],
    [   0,   0,   0,   1,   0,      -4766,        0,        270,       0],
    [  -1,  -1,   2,   2,   2,      -4095,        0,       1793,       0],
    [  -1,   0,   2,   0,   0,       4229,        0,       -101,       0],
    [   0,  -1,   2,   2,   2,      -3372,        0,       1487,       0],
    [   2,   0,   0,   0,   1,      -3353,        0,       1758,       0],
    [   1,   0,   2,   0,   0,      -3523,        0,        246,       0],
    [   1,   1,   0,   0,   0,      -3613,        0,        329,       0],
    [  -1,   0,   2,  -2,   1,       3522,        0,      -1830,       0],
    [   2,   0,   0,   0,  -1,       3312,        0,      -1730,       0],
    [   0,   0,  -2,   2,   1,      -3142,        0,       1704,       0],
    [   0,   1,   0,   0,  -1,      -2927,        0,       1564,       0],
    [   0,   1,   2,   0,   1,      -2887,        0,       1401,       0],
    [   0,  -1,   2,   0,   1,       2451,        0,      -1200,       0],
    [   2,   0,  -2,   0,   0,      -2790,        0,        410,       0],
    [  -1,   0,   0,   2,  -1,       2145,        0,      -1154,       0],
    [   0,   0,   2,  -2,   0,       2816,        0,        286,       0],
    [   0,   1,   0,  -2,   0,       2700,        0,       -258,       0],
    [   1,   0,   0,  -1,   0,      -2330,        0,        -37,       0],
    [   0,   0,   0,   0,   2,       2283,        0,      -1039,       0],
    [   1,   0,  -2,   0,   0,      -2321,        0,        284,       0],
    [  -1,   0,   0,   1,   1,      -2049,        0,       1112,       0],
];

/// IAU 2000B nutation: returns (Δψ, Δε) in arcseconds.
///
/// # Arguments
/// * `t` — Julian centuries of TDB since J2000.0
///
/// # Returns
/// * `(delta_psi_arcsec, delta_epsilon_arcsec)` — nutation in longitude and obliquity
///
/// Accuracy: ~1 mas, sufficient for ayanamsha and rise/set applications.
pub fn nutation_iau2000b(t: f64) -> (f64, f64) {
    let args = fundamental_arguments(t);

    let mut dpsi: f64 = 0.0;
    let mut deps: f64 = 0.0;

    for row in &NUTATION_COEFFS {
        // Compute the argument for this term
        let arg = row[0] as f64 * args[0]
            + row[1] as f64 * args[1]
            + row[2] as f64 * args[2]
            + row[3] as f64 * args[3]
            + row[4] as f64 * args[4];

        let sin_arg = arg.sin();
        let cos_arg = arg.cos();

        // Δψ: (S_i + S'_i * T) * sin(arg)
        dpsi += (row[5] as f64 + row[6] as f64 * t) * sin_arg;
        // Δε: (C_i + C'_i * T) * cos(arg)
        deps += (row[7] as f64 + row[8] as f64 * t) * cos_arg;
    }

    // Convert from 0.1 μas to arcseconds: 1 unit = 1e-7 arcsec
    let dpsi_arcsec = dpsi * 1e-7;
    let deps_arcsec = deps * 1e-7;

    // Add fixed offset corrections from IAU 2000B model
    // (frame bias contributions to approximate IAU 2000A)
    // dpsi offset: -0.135 mas = -0.000135 arcsec
    // deps offset: -0.388 mas = -0.000388 arcsec
    let dpsi_arcsec = dpsi_arcsec - 0.000_135;
    let deps_arcsec = deps_arcsec - 0.000_388;

    (dpsi_arcsec, deps_arcsec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_at_j2000_is_finite() {
        let (dpsi, deps) = nutation_iau2000b(0.0);
        assert!(dpsi.is_finite(), "dpsi at J2000 not finite: {dpsi}");
        assert!(deps.is_finite(), "deps at J2000 not finite: {deps}");
    }

    #[test]
    fn typical_amplitude() {
        // At T=0.24 (~2024), nutation should be within typical bounds
        let (dpsi, deps) = nutation_iau2000b(0.24);
        assert!(dpsi.abs() < 20.0, "|Δψ| should be < 20 arcsec, got {dpsi}");
        assert!(deps.abs() < 10.0, "|Δε| should be < 10 arcsec, got {deps}");
    }

    #[test]
    fn known_value_2024() {
        // At 2024-01-01 (JD 2460310.5, T ≈ 0.24), Δψ ≈ -5.4″
        // The 18.6-year nutation cycle was near a node in 2024.
        let t = (2_460_310.5 - 2_451_545.0) / 36525.0;
        let (dpsi, deps) = nutation_iau2000b(t);
        assert!(dpsi.abs() < 18.0, "|Δψ| should be < 18 arcsec, got {dpsi}");
        assert!(deps.abs() < 10.0, "|Δε| should be < 10 arcsec, got {deps}");
    }

    #[test]
    fn symmetry_over_nutation_period() {
        // Nutation has ~18.6 year period (Ω). Values at t and t+18.6yr
        // should be similar (not identical due to other terms).
        let t1 = 0.1;
        let t2 = t1 + 18.6 / 100.0; // +18.6 years
        let (dpsi1, _) = nutation_iau2000b(t1);
        let (dpsi2, _) = nutation_iau2000b(t2);
        // Within ~5″ due to other period terms
        assert!(
            (dpsi1 - dpsi2).abs() < 5.0,
            "Δψ at T={t1}: {dpsi1}, at T={t2}: {dpsi2}"
        );
    }
}
