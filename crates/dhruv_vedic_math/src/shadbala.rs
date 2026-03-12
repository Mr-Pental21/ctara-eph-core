//! Shadbala (six-fold planetary strength) computation.
//!
//! Pure math, no engine dependency. **Sapta grahas only** (Sun through Saturn).
//! Single-graha functions return 0.0 for Rahu/Ketu.
//!
//! The six components:
//! 1. Sthana Bala (positional): uchcha + saptavargaja + ojhayugma + kendradi + drekkana
//! 2. Dig Bala (directional)
//! 3. Kala Bala (temporal): nathonnatha + paksha + tribhaga + abda + masa + vara + hora + ayana + yuddha
//! 4. Cheshta Bala (motional)
//! 5. Naisargika Bala (natural)
//! 6. Drik Bala (aspectual)
//!
//! Clean-room implementation from BPHS.

use crate::drishti::{base_virupa, special_virupa};
use crate::graha::{Graha, SAPTA_GRAHAS};
use crate::graha_relationships::{
    BeneficNature, Dignity, GrahaGender, dignity_in_rashi_with_positions, graha_gender,
    moon_benefic_nature, natural_benefic_malefic,
};
use crate::util::normalize_360;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Natural strength in shashtiamsas (60ths of a rupa), per BPHS.
pub const NAISARGIKA_BALA: [f64; 7] = [60.0, 51.43, 17.14, 25.71, 34.29, 42.86, 8.57];

/// Bhava (house) of maximum directional strength for each graha.
/// Sun=10, Moon=4, Mars=10, Mercury=1, Jupiter=1, Venus=4, Saturn=7.
pub const DIG_BALA_BHAVA: [u8; 7] = [10, 4, 10, 1, 1, 4, 7];

/// Maximum daily speed (deg/day) for cheshta bala normalization.
pub const MAX_SPEED: [f64; 7] = [1.0, 15.0, 0.8, 2.2, 0.25, 1.6, 0.13];

/// Required strength (shashtiamsas) for a graha to be considered strong.
pub const REQUIRED_STRENGTH: [f64; 7] = [390.0, 360.0, 300.0, 420.0, 390.0, 330.0, 300.0];

/// Saptavargaja dignity-to-points mapping.
const SAPTAVARGAJA_POINTS: [f64; 9] = [
    30.0, // Exalted
    22.5, // Moolatrikone
    20.0, // OwnSign
    15.0, // AdhiMitra
    10.0, // Mitra
    7.5,  // Sama
    5.0,  // Shatru
    2.5,  // AdhiShatru
    1.25, // Debilitated
];

fn saptavargaja_dignity_points(dignity: Dignity) -> f64 {
    match dignity {
        Dignity::Exalted => SAPTAVARGAJA_POINTS[0],
        Dignity::Moolatrikone => SAPTAVARGAJA_POINTS[1],
        Dignity::OwnSign => SAPTAVARGAJA_POINTS[2],
        Dignity::AdhiMitra => SAPTAVARGAJA_POINTS[3],
        Dignity::Mitra => SAPTAVARGAJA_POINTS[4],
        Dignity::Sama => SAPTAVARGAJA_POINTS[5],
        Dignity::Shatru => SAPTAVARGAJA_POINTS[6],
        Dignity::AdhiShatru => SAPTAVARGAJA_POINTS[7],
        Dignity::Debilitated => SAPTAVARGAJA_POINTS[8],
    }
}

fn is_sapta_graha(graha: Graha) -> bool {
    graha.index() < 7
}

// ---------------------------------------------------------------------------
// 2b. Sthana Bala Sub-Components
// ---------------------------------------------------------------------------

/// Uchcha Bala: 60 * (1 - distance_from_exaltation / 180).
pub fn uchcha_bala(graha: Graha, sidereal_lon: f64) -> f64 {
    if !is_sapta_graha(graha) {
        return 0.0;
    }
    let exalt = match crate::graha_relationships::exaltation_degree(graha) {
        Some(e) => e,
        None => return 0.0,
    };
    let lon = normalize_360(sidereal_lon);
    let diff = (lon - exalt).abs();
    let dist = if diff > 180.0 { 360.0 - diff } else { diff };
    60.0 * (1.0 - dist / 180.0)
}

/// Uchcha bala for all 7 sapta grahas.
pub fn all_uchcha_balas(sidereal_lons: &[f64; 7]) -> [f64; 7] {
    let mut result = [0.0; 7];
    for (i, g) in SAPTA_GRAHAS.iter().enumerate() {
        result[i] = uchcha_bala(*g, sidereal_lons[i]);
    }
    result
}

/// Saptavargaja Bala: sum of dignity points across 7 vargas (D1,D2,D3,D7,D9,D12,D30).
///
/// `varga_rashi` is a 7x7 array: `varga_rashi[varga][graha]` = rashi index in that varga.
/// Uses per-varga rashi positions for compound friendship, NOT D1 reuse.
pub fn saptavargaja_bala(graha: Graha, sid_lon: f64, varga_rashi: &[[u8; 7]; 7]) -> f64 {
    if !is_sapta_graha(graha) {
        return 0.0;
    }
    let gi = graha.index() as usize;
    let mut total = 0.0;
    for row in varga_rashi.iter().take(7) {
        let rashi_idx = row[gi];
        let dignity = dignity_in_rashi_with_positions(graha, sid_lon, rashi_idx, row);
        total += saptavargaja_dignity_points(dignity);
    }
    total
}

/// Saptavargaja bala for all 7 sapta grahas.
pub fn all_saptavargaja_balas(sidereal_lons: &[f64; 7], varga_rashi: &[[u8; 7]; 7]) -> [f64; 7] {
    let mut result = [0.0; 7];
    for (i, g) in SAPTA_GRAHAS.iter().enumerate() {
        result[i] = saptavargaja_bala(*g, sidereal_lons[i], varga_rashi);
    }
    result
}

/// Ojhayugma Bala: male graha in odd rashi + odd navamsa = 15+15=30.
/// Female graha in even rashi + even navamsa = 15+15=30. Else 0 per component.
pub fn ojhayugma_bala(graha: Graha, sidereal_lon: f64) -> f64 {
    if !is_sapta_graha(graha) {
        return 0.0;
    }
    let gender = graha_gender(graha);
    let lon = normalize_360(sidereal_lon);
    let rashi_idx = (lon / 30.0).floor() as u8;
    // Navamsa rashi: D9
    let navamsa_lon = crate::amsha::amsha_longitude(sidereal_lon, crate::amsha::Amsha::D9, None);
    let navamsa_rashi = (normalize_360(navamsa_lon) / 30.0).floor() as u8;

    let rashi_odd = rashi_idx.is_multiple_of(2); // 0=Mesha(odd), 1=Vrishabha(even), etc.
    let navamsa_odd = navamsa_rashi.is_multiple_of(2);

    let mut score = 0.0;
    match gender {
        GrahaGender::Male => {
            if rashi_odd {
                score += 15.0;
            }
            if navamsa_odd {
                score += 15.0;
            }
        }
        GrahaGender::Female => {
            if !rashi_odd {
                score += 15.0;
            }
            if !navamsa_odd {
                score += 15.0;
            }
        }
        GrahaGender::Neuter => {
            // Mercury/Saturn: same rule as male (odd = strong) per common BPHS reading
            if rashi_odd {
                score += 15.0;
            }
            if navamsa_odd {
                score += 15.0;
            }
        }
    }
    score
}

/// Ojhayugma bala for all 7 sapta grahas.
pub fn all_ojhayugma_balas(sidereal_lons: &[f64; 7]) -> [f64; 7] {
    let mut result = [0.0; 7];
    for (i, g) in SAPTA_GRAHAS.iter().enumerate() {
        result[i] = ojhayugma_bala(*g, sidereal_lons[i]);
    }
    result
}

/// Kendradi Bala: kendra(1,4,7,10)=60, panfar(2,5,8,11)=30, apokalim(3,6,9,12)=15.
pub fn kendradi_bala(bhava_number: u8) -> f64 {
    if bhava_number == 0 || bhava_number > 12 {
        return 0.0;
    }
    match bhava_number {
        1 | 4 | 7 | 10 => 60.0,
        2 | 5 | 8 | 11 => 30.0,
        3 | 6 | 9 | 12 => 15.0,
        _ => 0.0,
    }
}

/// Kendradi bala for all 7 sapta grahas.
pub fn all_kendradi_balas(bhava_numbers: &[u8; 7]) -> [f64; 7] {
    let mut result = [0.0; 7];
    for i in 0..7 {
        result[i] = kendradi_bala(bhava_numbers[i]);
    }
    result
}

/// Drekkana Bala: male in 1st decanate=15, female in 2nd=15, neuter in 3rd=15.
pub fn drekkana_bala(graha: Graha, sidereal_lon: f64) -> f64 {
    if !is_sapta_graha(graha) {
        return 0.0;
    }
    let lon = normalize_360(sidereal_lon);
    let deg_in_rashi = lon - (lon / 30.0).floor() * 30.0;
    let decanate = if deg_in_rashi < 10.0 {
        1
    } else if deg_in_rashi < 20.0 {
        2
    } else {
        3
    };

    let gender = graha_gender(graha);
    match (gender, decanate) {
        (GrahaGender::Male, 1) => 15.0,
        (GrahaGender::Female, 2) => 15.0,
        (GrahaGender::Neuter, 3) => 15.0,
        _ => 0.0,
    }
}

/// Drekkana bala for all 7 sapta grahas.
pub fn all_drekkana_balas(sidereal_lons: &[f64; 7]) -> [f64; 7] {
    let mut result = [0.0; 7];
    for (i, g) in SAPTA_GRAHAS.iter().enumerate() {
        result[i] = drekkana_bala(*g, sidereal_lons[i]);
    }
    result
}

/// Sthana Bala breakdown.
#[derive(Debug, Clone, Copy)]
pub struct SthanaBalaBreakdown {
    pub uchcha: f64,
    pub saptavargaja: f64,
    pub ojhayugma: f64,
    pub kendradi: f64,
    pub drekkana: f64,
    pub total: f64,
}

/// Sthana bala for a single graha.
pub fn sthana_bala(
    graha: Graha,
    sid_lon: f64,
    bhava: u8,
    varga_rashi: &[[u8; 7]; 7],
) -> SthanaBalaBreakdown {
    let u = uchcha_bala(graha, sid_lon);
    let s = saptavargaja_bala(graha, sid_lon, varga_rashi);
    let o = ojhayugma_bala(graha, sid_lon);
    let k = kendradi_bala(bhava);
    let d = drekkana_bala(graha, sid_lon);
    SthanaBalaBreakdown {
        uchcha: u,
        saptavargaja: s,
        ojhayugma: o,
        kendradi: k,
        drekkana: d,
        total: u + s + o + k + d,
    }
}

/// Sthana bala for all 7 sapta grahas.
pub fn all_sthana_balas(
    lons: &[f64; 7],
    bhavas: &[u8; 7],
    varga_rashi: &[[u8; 7]; 7],
) -> [SthanaBalaBreakdown; 7] {
    let mut result = [SthanaBalaBreakdown {
        uchcha: 0.0,
        saptavargaja: 0.0,
        ojhayugma: 0.0,
        kendradi: 0.0,
        drekkana: 0.0,
        total: 0.0,
    }; 7];
    for (i, g) in SAPTA_GRAHAS.iter().enumerate() {
        result[i] = sthana_bala(*g, lons[i], bhavas[i], varga_rashi);
    }
    result
}

// ---------------------------------------------------------------------------
// 2c. Dig Bala
// ---------------------------------------------------------------------------

/// Dig Bala: 60 * (1 - dist/6), dist = min(|bhava-max|, 12-|bhava-max|) capped at 6.
pub fn dig_bala(graha: Graha, bhava_number: u8) -> f64 {
    if !is_sapta_graha(graha) || bhava_number == 0 || bhava_number > 12 {
        return 0.0;
    }
    let max_bhava = DIG_BALA_BHAVA[graha.index() as usize];
    let diff = (bhava_number as i16 - max_bhava as i16).unsigned_abs();
    let dist = diff.min(12 - diff).min(6);
    60.0 * (1.0 - dist as f64 / 6.0)
}

/// Dig bala for all 7 sapta grahas.
pub fn all_dig_balas(bhava_numbers: &[u8; 7]) -> [f64; 7] {
    let mut result = [0.0; 7];
    for (i, g) in SAPTA_GRAHAS.iter().enumerate() {
        result[i] = dig_bala(*g, bhava_numbers[i]);
    }
    result
}

// ---------------------------------------------------------------------------
// 2d. Kala Bala Sub-Components
// ---------------------------------------------------------------------------

/// Nathonnatha Bala: malefics strong by day (60), benefics by night (60).
/// Mercury's nature depends on moon_sun_elongation.
pub fn nathonnatha_bala(graha: Graha, is_daytime: bool, moon_sun_elong: f64) -> f64 {
    if !is_sapta_graha(graha) {
        return 0.0;
    }
    let nature = if graha == Graha::Chandra {
        moon_benefic_nature(moon_sun_elong)
    } else if graha == Graha::Buddh {
        // Mercury inherits Moon's nature for this context
        moon_benefic_nature(moon_sun_elong)
    } else {
        natural_benefic_malefic(graha)
    };

    match (nature, is_daytime) {
        (BeneficNature::Malefic, true) | (BeneficNature::Benefic, false) => 60.0,
        _ => 0.0,
    }
}

/// Nathonnatha bala for all 7.
pub fn all_nathonnatha_balas(is_daytime: bool, moon_sun_elong: f64) -> [f64; 7] {
    let mut result = [0.0; 7];
    for (i, g) in SAPTA_GRAHAS.iter().enumerate() {
        result[i] = nathonnatha_bala(*g, is_daytime, moon_sun_elong);
    }
    result
}

/// Paksha Bala: benefics strong at full moon, malefics strong at new moon.
pub fn paksha_bala(graha: Graha, moon_sun_elong: f64) -> f64 {
    if !is_sapta_graha(graha) {
        return 0.0;
    }
    let elong = normalize_360(moon_sun_elong);
    let phase_angle = if elong <= 180.0 { elong } else { 360.0 - elong };
    let benefic_score = phase_angle / 3.0; // 0 at new moon, 60 at full moon
    let malefic_score = 60.0 - benefic_score;

    let nature = if graha == Graha::Chandra {
        BeneficNature::Benefic // Moon always uses benefic formula for paksha
    } else if graha == Graha::Buddh {
        moon_benefic_nature(moon_sun_elong)
    } else {
        natural_benefic_malefic(graha)
    };

    match nature {
        BeneficNature::Benefic => benefic_score,
        BeneficNature::Malefic => malefic_score,
    }
}

/// Paksha bala for all 7.
pub fn all_paksha_balas(moon_sun_elong: f64) -> [f64; 7] {
    let mut result = [0.0; 7];
    for (i, g) in SAPTA_GRAHAS.iter().enumerate() {
        result[i] = paksha_bala(*g, moon_sun_elong);
    }
    result
}

/// Tribhaga Bala: day/night divided into thirds, specific planets strong in each third.
/// Sun = 60 always. fraction: 0.0-1.0 within day or night.
pub fn tribhaga_bala(graha: Graha, is_daytime: bool, fraction: f64) -> f64 {
    if !is_sapta_graha(graha) {
        return 0.0;
    }
    // Sun always gets 60
    if graha == Graha::Surya {
        return 60.0;
    }

    let third = if fraction < 1.0 / 3.0 {
        0
    } else if fraction < 2.0 / 3.0 {
        1
    } else {
        2
    };

    let strong_graha = if is_daytime {
        match third {
            0 => Graha::Guru,
            1 => Graha::Buddh,
            _ => Graha::Shani,
        }
    } else {
        match third {
            0 => Graha::Chandra,
            1 => Graha::Shukra,
            _ => Graha::Mangal,
        }
    };

    if graha == strong_graha { 60.0 } else { 0.0 }
}

/// Tribhaga bala for all 7.
pub fn all_tribhaga_balas(is_daytime: bool, fraction: f64) -> [f64; 7] {
    let mut result = [0.0; 7];
    for (i, g) in SAPTA_GRAHAS.iter().enumerate() {
        result[i] = tribhaga_bala(*g, is_daytime, fraction);
    }
    result
}

/// Abda (year lord) Bala: 15 if graha is the year lord.
pub fn abda_bala(graha: Graha, year_lord: Graha) -> f64 {
    if is_sapta_graha(graha) && graha == year_lord {
        15.0
    } else {
        0.0
    }
}

/// Abda bala for all 7.
pub fn all_abda_balas(year_lord: Graha) -> [f64; 7] {
    let mut result = [0.0; 7];
    for (i, g) in SAPTA_GRAHAS.iter().enumerate() {
        result[i] = abda_bala(*g, year_lord);
    }
    result
}

/// Masa (month lord) Bala: 30 if graha is the month lord.
pub fn masa_bala(graha: Graha, month_lord: Graha) -> f64 {
    if is_sapta_graha(graha) && graha == month_lord {
        30.0
    } else {
        0.0
    }
}

/// Masa bala for all 7.
pub fn all_masa_balas(month_lord: Graha) -> [f64; 7] {
    let mut result = [0.0; 7];
    for (i, g) in SAPTA_GRAHAS.iter().enumerate() {
        result[i] = masa_bala(*g, month_lord);
    }
    result
}

/// Vara (weekday lord) Bala: 45 if graha is the weekday lord.
pub fn vara_bala(graha: Graha, weekday_lord: Graha) -> f64 {
    if is_sapta_graha(graha) && graha == weekday_lord {
        45.0
    } else {
        0.0
    }
}

/// Vara bala for all 7.
pub fn all_vara_balas(weekday_lord: Graha) -> [f64; 7] {
    let mut result = [0.0; 7];
    for (i, g) in SAPTA_GRAHAS.iter().enumerate() {
        result[i] = vara_bala(*g, weekday_lord);
    }
    result
}

/// Hora (planetary hour lord) Bala: 60 if graha is the hora lord.
pub fn hora_bala(graha: Graha, hora_lord: Graha) -> f64 {
    if is_sapta_graha(graha) && graha == hora_lord {
        60.0
    } else {
        0.0
    }
}

/// Hora bala for all 7.
pub fn all_hora_balas(hora_lord: Graha) -> [f64; 7] {
    let mut result = [0.0; 7];
    for (i, g) in SAPTA_GRAHAS.iter().enumerate() {
        result[i] = hora_bala(*g, hora_lord);
    }
    result
}

/// Ayana Bala: per-graha declination formula.
/// Benefic: (24 + declination) / 48 * 60.
/// Malefic: (24 - declination) / 48 * 60.
/// Moon classification from moon_sun_elongation.
pub fn ayana_bala(graha: Graha, declination_deg: f64, moon_sun_elong: f64) -> f64 {
    if !is_sapta_graha(graha) {
        return 0.0;
    }
    let kranti = declination_deg.clamp(-24.0, 24.0);

    let nature = if graha == Graha::Chandra || graha == Graha::Buddh {
        moon_benefic_nature(moon_sun_elong)
    } else {
        natural_benefic_malefic(graha)
    };

    let score = match nature {
        BeneficNature::Benefic => (24.0 + kranti) / 48.0 * 60.0,
        BeneficNature::Malefic => (24.0 - kranti) / 48.0 * 60.0,
    };
    score.max(0.0)
}

/// Ayana bala for all 7.
pub fn all_ayana_balas(declinations: &[f64; 7], moon_sun_elong: f64) -> [f64; 7] {
    let mut result = [0.0; 7];
    for (i, g) in SAPTA_GRAHAS.iter().enumerate() {
        result[i] = ayana_bala(*g, declinations[i], moon_sun_elong);
    }
    result
}

/// Yuddha (planetary war) Bala: when two planets are within 1 deg longitude.
/// Winner (northernmost declination) gets +60, loser gets -60.
/// Only Mars, Mercury, Jupiter, Venus, Saturn participate. Sun/Moon exempt.
pub fn yuddha_bala(graha: Graha, sidereal_lons: &[f64; 7], declinations: &[f64; 7]) -> f64 {
    if !is_sapta_graha(graha) {
        return 0.0;
    }
    // Sun (0) and Moon (1) don't participate
    let gi = graha.index() as usize;
    if gi < 2 {
        return 0.0;
    }

    let my_lon = normalize_360(sidereal_lons[gi]);
    let my_dec = declinations[gi];
    let mut total = 0.0;

    // Check against other participating planets (indices 2-6)
    for oi in 2..7 {
        if oi == gi {
            continue;
        }
        let other_lon = normalize_360(sidereal_lons[oi]);
        let diff = (my_lon - other_lon).abs();
        let sep = if diff > 180.0 { 360.0 - diff } else { diff };
        if sep < 1.0 {
            // War! Winner = northernmost (highest declination)
            let other_dec = declinations[oi];
            if my_dec > other_dec {
                total += 60.0; // Winner
            } else if my_dec < other_dec {
                total -= 60.0; // Loser
            }
            // Equal declination: no change
        }
    }
    total
}

/// Yuddha bala for all 7.
pub fn all_yuddha_balas(sidereal_lons: &[f64; 7], declinations: &[f64; 7]) -> [f64; 7] {
    let mut result = [0.0; 7];
    for (i, g) in SAPTA_GRAHAS.iter().enumerate() {
        result[i] = yuddha_bala(*g, sidereal_lons, declinations);
    }
    result
}

/// Kala Bala inputs.
#[derive(Debug, Clone, Copy)]
pub struct KalaBalaInputs {
    pub is_daytime: bool,
    pub day_night_fraction: f64,
    pub moon_sun_elongation: f64,
    pub year_lord: Graha,
    pub month_lord: Graha,
    pub weekday_lord: Graha,
    pub hora_lord: Graha,
    pub graha_declinations: [f64; 7],
    pub sidereal_lons: [f64; 7],
}

/// Kala Bala breakdown.
#[derive(Debug, Clone, Copy)]
pub struct KalaBalaBreakdown {
    pub nathonnatha: f64,
    pub paksha: f64,
    pub tribhaga: f64,
    pub abda: f64,
    pub masa: f64,
    pub vara: f64,
    pub hora: f64,
    pub ayana: f64,
    pub yuddha: f64,
    pub total: f64,
}

/// Kala bala for a single graha.
pub fn kala_bala(graha: Graha, inputs: &KalaBalaInputs) -> KalaBalaBreakdown {
    let n = nathonnatha_bala(graha, inputs.is_daytime, inputs.moon_sun_elongation);
    let p = paksha_bala(graha, inputs.moon_sun_elongation);
    let t = tribhaga_bala(graha, inputs.is_daytime, inputs.day_night_fraction);
    let ab = abda_bala(graha, inputs.year_lord);
    let ma = masa_bala(graha, inputs.month_lord);
    let va = vara_bala(graha, inputs.weekday_lord);
    let ho = hora_bala(graha, inputs.hora_lord);
    let ay = ayana_bala(
        graha,
        inputs.graha_declinations[graha.index().min(6) as usize],
        inputs.moon_sun_elongation,
    );
    let yu = yuddha_bala(graha, &inputs.sidereal_lons, &inputs.graha_declinations);
    let total = n + p + t + ab + ma + va + ho + ay + yu;
    KalaBalaBreakdown {
        nathonnatha: n,
        paksha: p,
        tribhaga: t,
        abda: ab,
        masa: ma,
        vara: va,
        hora: ho,
        ayana: ay,
        yuddha: yu,
        total,
    }
}

/// Kala bala for all 7.
pub fn all_kala_balas(inputs: &KalaBalaInputs) -> [KalaBalaBreakdown; 7] {
    let zero = KalaBalaBreakdown {
        nathonnatha: 0.0,
        paksha: 0.0,
        tribhaga: 0.0,
        abda: 0.0,
        masa: 0.0,
        vara: 0.0,
        hora: 0.0,
        ayana: 0.0,
        yuddha: 0.0,
        total: 0.0,
    };
    let mut result = [zero; 7];
    for (i, g) in SAPTA_GRAHAS.iter().enumerate() {
        result[i] = kala_bala(*g, inputs);
    }
    result
}

// ---------------------------------------------------------------------------
// 2e. Cheshta Bala
// ---------------------------------------------------------------------------

/// Cheshta Bala: based on speed ratio.
/// Retrograde (negative speed): 60. Direct: (|speed|/max_speed)*60, capped at 60.
/// Sun/Moon: always 0 (they don't retrograde and have separate rules).
pub fn cheshta_bala(graha: Graha, speed_deg_per_day: f64) -> f64 {
    if !is_sapta_graha(graha) {
        return 0.0;
    }
    let gi = graha.index() as usize;
    // Sun and Moon don't have cheshta bala
    if gi < 2 {
        return 0.0;
    }
    if speed_deg_per_day < 0.0 {
        // Retrograde
        60.0
    } else {
        let ratio = speed_deg_per_day / MAX_SPEED[gi];
        (ratio * 60.0).min(60.0)
    }
}

/// Cheshta bala for all 7.
pub fn all_cheshta_balas(speeds: &[f64; 7]) -> [f64; 7] {
    let mut result = [0.0; 7];
    for (i, g) in SAPTA_GRAHAS.iter().enumerate() {
        result[i] = cheshta_bala(*g, speeds[i]);
    }
    result
}

// ---------------------------------------------------------------------------
// 2f. Naisargika Bala
// ---------------------------------------------------------------------------

/// Naisargika (natural) bala for a graha.
pub fn naisargika_bala(graha: Graha) -> f64 {
    if is_sapta_graha(graha) {
        NAISARGIKA_BALA[graha.index() as usize]
    } else {
        0.0
    }
}

/// Naisargika bala for all 7.
pub fn all_naisargika_balas() -> [f64; 7] {
    NAISARGIKA_BALA
}

// ---------------------------------------------------------------------------
// 2g. Drik Bala
// ---------------------------------------------------------------------------

/// Drik Bala: (benefic_virupa_sum - malefic_virupa_sum) / 4.
///
/// For each incoming aspect, classify the aspecting graha as benefic or malefic,
/// then sum their total virupas. The difference divided by 4 gives drik bala.
/// `sidereal_lons` = all 9 grahas. `moon_sun_elong` for Moon/Mercury classification.
pub fn drik_bala(graha: Graha, sidereal_lons: &[f64; 9], moon_sun_elong: f64) -> f64 {
    if !is_sapta_graha(graha) {
        return 0.0;
    }
    let target_lon = sidereal_lons[graha.index() as usize];
    let mut benefic_sum = 0.0;
    let mut malefic_sum = 0.0;

    for src in crate::graha::ALL_GRAHAS {
        if src == graha {
            continue;
        }
        let src_lon = sidereal_lons[src.index() as usize];
        let ang = normalize_360(target_lon - src_lon);
        let bv = base_virupa(ang);
        let sv = special_virupa(src, ang);
        let total = bv + sv;

        let nature = if src == Graha::Chandra || src == Graha::Buddh {
            moon_benefic_nature(moon_sun_elong)
        } else {
            natural_benefic_malefic(src)
        };

        match nature {
            BeneficNature::Benefic => benefic_sum += total,
            BeneficNature::Malefic => malefic_sum += total,
        }
    }

    (benefic_sum - malefic_sum) / 4.0
}

/// Drik bala for all 7 sapta grahas.
pub fn all_drik_balas(sidereal_lons: &[f64; 9], moon_sun_elong: f64) -> [f64; 7] {
    let mut result = [0.0; 7];
    for (i, g) in SAPTA_GRAHAS.iter().enumerate() {
        result[i] = drik_bala(*g, sidereal_lons, moon_sun_elong);
    }
    result
}

// ---------------------------------------------------------------------------
// 2h. Complete Shadbala
// ---------------------------------------------------------------------------

/// Complete Shadbala breakdown.
#[derive(Debug, Clone, Copy)]
pub struct ShadbalaBreakdown {
    pub sthana: SthanaBalaBreakdown,
    pub dig: f64,
    pub kala: KalaBalaBreakdown,
    pub cheshta: f64,
    pub naisargika: f64,
    pub drik: f64,
    pub total_shashtiamsas: f64,
    pub total_rupas: f64,
    pub required_strength: f64,
    pub is_strong: bool,
}

/// All inputs needed for complete Shadbala computation.
#[derive(Debug, Clone, Copy)]
pub struct ShadbalaInputs {
    /// All 9 grahas sidereal longitudes (for drik bala — needs Rahu/Ketu).
    pub sidereal_lons: [f64; 9],
    /// Bhava numbers (1-12) for sapta grahas.
    pub bhava_numbers: [u8; 7],
    /// Speed (deg/day) for sapta grahas.
    pub speeds: [f64; 7],
    /// Kala bala inputs.
    pub kala: KalaBalaInputs,
    /// 7 vargas x 7 grahas rashi indices (for saptavargaja bala).
    pub varga_rashi_indices: [[u8; 7]; 7],
}

/// Compute complete Shadbala for a single graha.
pub fn shadbala_from_inputs(graha: Graha, inputs: &ShadbalaInputs) -> ShadbalaBreakdown {
    if !is_sapta_graha(graha) {
        return zero_shadbala();
    }
    let gi = graha.index() as usize;

    let sthana_result = sthana_bala(
        graha,
        inputs.sidereal_lons[gi],
        inputs.bhava_numbers[gi],
        &inputs.varga_rashi_indices,
    );
    let dig = dig_bala(graha, inputs.bhava_numbers[gi]);
    let kala_result = kala_bala(graha, &inputs.kala);
    let cheshta = cheshta_bala(graha, inputs.speeds[gi]);
    let nais = naisargika_bala(graha);
    let drik = drik_bala(
        graha,
        &inputs.sidereal_lons,
        inputs.kala.moon_sun_elongation,
    );

    let total = sthana_result.total + dig + kala_result.total + cheshta + nais + drik;
    let rupas = total / 60.0;
    let required = REQUIRED_STRENGTH[gi];

    ShadbalaBreakdown {
        sthana: sthana_result,
        dig,
        kala: kala_result,
        cheshta,
        naisargika: nais,
        drik,
        total_shashtiamsas: total,
        total_rupas: rupas,
        required_strength: required,
        is_strong: total >= required,
    }
}

/// Compute complete Shadbala for all 7 sapta grahas.
pub fn all_shadbalas_from_inputs(inputs: &ShadbalaInputs) -> [ShadbalaBreakdown; 7] {
    let mut result = [zero_shadbala(); 7];
    for (i, g) in SAPTA_GRAHAS.iter().enumerate() {
        result[i] = shadbala_from_inputs(*g, inputs);
    }
    result
}

fn zero_shadbala() -> ShadbalaBreakdown {
    ShadbalaBreakdown {
        sthana: SthanaBalaBreakdown {
            uchcha: 0.0,
            saptavargaja: 0.0,
            ojhayugma: 0.0,
            kendradi: 0.0,
            drekkana: 0.0,
            total: 0.0,
        },
        dig: 0.0,
        kala: KalaBalaBreakdown {
            nathonnatha: 0.0,
            paksha: 0.0,
            tribhaga: 0.0,
            abda: 0.0,
            masa: 0.0,
            vara: 0.0,
            hora: 0.0,
            ayana: 0.0,
            yuddha: 0.0,
            total: 0.0,
        },
        cheshta: 0.0,
        naisargika: 0.0,
        drik: 0.0,
        total_shashtiamsas: 0.0,
        total_rupas: 0.0,
        required_strength: 0.0,
        is_strong: false,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-6;

    // --- Uchcha Bala ---

    #[test]
    fn uchcha_sun_at_exaltation() {
        // Sun exalted at 10 Aries
        assert!((uchcha_bala(Graha::Surya, 10.0) - 60.0).abs() < EPS);
    }

    #[test]
    fn uchcha_sun_at_debilitation() {
        // Sun debilitated at 10 Libra (190 deg)
        assert!(uchcha_bala(Graha::Surya, 190.0).abs() < EPS);
    }

    #[test]
    fn uchcha_sun_at_90deg_from_exalt() {
        // 10+90=100 deg → distance=90 → 60*(1-90/180)=30
        assert!((uchcha_bala(Graha::Surya, 100.0) - 30.0).abs() < EPS);
    }

    #[test]
    fn uchcha_rahu_zero() {
        assert!(uchcha_bala(Graha::Rahu, 100.0).abs() < EPS);
    }

    // --- Dig Bala ---

    #[test]
    fn dig_sun_at_max() {
        // Sun strongest at bhava 10
        assert!((dig_bala(Graha::Surya, 10) - 60.0).abs() < EPS);
    }

    #[test]
    fn dig_sun_at_opposite() {
        // Bhava 4 is 6 away from 10 → 60*(1-6/6)=0
        assert!(dig_bala(Graha::Surya, 4).abs() < EPS);
    }

    #[test]
    fn dig_sun_at_bhava_7() {
        // 10 to 7: |10-7|=3, min(3,9)=3 → 60*(1-3/6)=30
        assert!((dig_bala(Graha::Surya, 7) - 30.0).abs() < EPS);
    }

    // --- Kendradi Bala ---

    #[test]
    fn kendradi_kendra() {
        for b in [1u8, 4, 7, 10] {
            assert!((kendradi_bala(b) - 60.0).abs() < EPS, "bhava {b}");
        }
    }

    #[test]
    fn kendradi_panapara() {
        for b in [2u8, 5, 8, 11] {
            assert!((kendradi_bala(b) - 30.0).abs() < EPS, "bhava {b}");
        }
    }

    #[test]
    fn kendradi_apoklima() {
        for b in [3u8, 6, 9, 12] {
            assert!((kendradi_bala(b) - 15.0).abs() < EPS, "bhava {b}");
        }
    }

    // --- Drekkana Bala ---

    #[test]
    fn drekkana_male_first_decanate() {
        // Sun (male) at 5 deg in rashi (1st decanate)
        assert!((drekkana_bala(Graha::Surya, 5.0) - 15.0).abs() < EPS);
    }

    #[test]
    fn drekkana_male_second_decanate() {
        // Sun at 15 deg (2nd decanate) → 0
        assert!(drekkana_bala(Graha::Surya, 15.0).abs() < EPS);
    }

    #[test]
    fn drekkana_female_second_decanate() {
        // Moon (female) at 15 deg (2nd decanate)
        assert!((drekkana_bala(Graha::Chandra, 15.0) - 15.0).abs() < EPS);
    }

    #[test]
    fn drekkana_neuter_third_decanate() {
        // Mercury (neuter) at 25 deg (3rd decanate)
        assert!((drekkana_bala(Graha::Buddh, 25.0) - 15.0).abs() < EPS);
    }

    // --- Nathonnatha Bala ---

    #[test]
    fn nathonnatha_malefic_daytime() {
        // Mars (malefic) + daytime = 60
        assert!((nathonnatha_bala(Graha::Mangal, true, 180.0) - 60.0).abs() < EPS);
    }

    #[test]
    fn nathonnatha_malefic_nighttime() {
        // Mars + nighttime = 0
        assert!(nathonnatha_bala(Graha::Mangal, false, 180.0).abs() < EPS);
    }

    #[test]
    fn nathonnatha_benefic_nighttime() {
        // Jupiter (benefic) + nighttime = 60
        assert!((nathonnatha_bala(Graha::Guru, false, 180.0) - 60.0).abs() < EPS);
    }

    // --- Paksha Bala ---

    #[test]
    fn paksha_full_moon_benefic() {
        // Full moon (elong=180): benefic score = 180/3 = 60
        assert!((paksha_bala(Graha::Guru, 180.0) - 60.0).abs() < EPS);
    }

    #[test]
    fn paksha_full_moon_malefic() {
        // Full moon: malefic score = 60-60 = 0
        assert!(paksha_bala(Graha::Mangal, 180.0).abs() < EPS);
    }

    #[test]
    fn paksha_new_moon_malefic() {
        // New moon (elong~0): malefic score = 60-0 = 60
        assert!((paksha_bala(Graha::Mangal, 0.0) - 60.0).abs() < EPS);
    }

    // --- Ayana Bala ---

    #[test]
    fn ayana_benefic_max_north() {
        // Benefic at +24 deg declination: (24+24)/48*60 = 60
        assert!((ayana_bala(Graha::Guru, 24.0, 180.0) - 60.0).abs() < EPS);
    }

    #[test]
    fn ayana_malefic_max_south() {
        // Malefic at -24 deg: (24-(-24))/48*60 = 60
        assert!((ayana_bala(Graha::Mangal, -24.0, 180.0) - 60.0).abs() < EPS);
    }

    // --- Cheshta Bala ---

    #[test]
    fn cheshta_retrograde() {
        // Retrograde Mars → 60
        assert!((cheshta_bala(Graha::Mangal, -0.5) - 60.0).abs() < EPS);
    }

    #[test]
    fn cheshta_direct_full_speed() {
        // Mars at max speed (0.8 deg/day) → 60
        assert!((cheshta_bala(Graha::Mangal, 0.8) - 60.0).abs() < EPS);
    }

    #[test]
    fn cheshta_sun_always_zero() {
        assert!(cheshta_bala(Graha::Surya, 1.0).abs() < EPS);
    }

    // --- Naisargika Bala ---

    #[test]
    fn naisargika_sun_strongest() {
        assert!((naisargika_bala(Graha::Surya) - 60.0).abs() < EPS);
    }

    #[test]
    fn naisargika_saturn_weakest() {
        assert!((naisargika_bala(Graha::Shani) - 8.57).abs() < EPS);
    }

    #[test]
    fn naisargika_rahu_zero() {
        assert!(naisargika_bala(Graha::Rahu).abs() < EPS);
    }

    // --- Saptavargaja ---

    #[test]
    fn saptavargaja_per_varga_positions_used() {
        // Mercury at 45 deg (Vrishabha). Not exalted/debilitated/own — relies on friendship.
        // Vrishabha lord = Venus. Mercury-Venus = naisargika Friend.
        // varga1: all grahas at rashi 0 (same sign as Mercury) → tatkalika enemy → Sama
        // varga2: Venus at rashi 1 (near Mercury rashi 1) → tatkalika friend → AdhiMitra
        let varga1: [[u8; 7]; 7] = [[1; 7]; 7]; // all in Vrishabha
        let mut varga2: [[u8; 7]; 7] = [[1; 7]; 7]; // all in Vrishabha
        // In varga2, shift Venus (idx 5) to rashi 2 → dist from Mercury(1)=1 → friend
        for v in &mut varga2 {
            v[5] = 2; // Venus in Mithuna
        }
        // In varga1, Venus at rashi 1 (same) → dist=0 → enemy

        let b1 = saptavargaja_bala(Graha::Buddh, 45.0, &varga1);
        let b2 = saptavargaja_bala(Graha::Buddh, 45.0, &varga2);

        // b1: Mercury-Venus naisargika=Friend, tatkalika=enemy → Sama → 7.5 per varga
        // b2: Mercury-Venus naisargika=Friend, tatkalika=friend → AdhiMitra → 15 per varga
        assert!(
            (b1 - b2).abs() > 0.1,
            "per-varga positions should matter: b1={b1}, b2={b2}"
        );
    }

    // --- all_* = individual ---

    #[test]
    fn all_uchcha_equals_individual() {
        let lons = [10.0, 33.0, 298.0, 165.0, 95.0, 357.0, 200.0];
        let all = all_uchcha_balas(&lons);
        for (i, g) in SAPTA_GRAHAS.iter().enumerate() {
            assert!(
                (all[i] - uchcha_bala(*g, lons[i])).abs() < EPS,
                "graha {:?}",
                g
            );
        }
    }

    // --- Rahu/Ketu rejection ---

    #[test]
    fn rahu_ketu_all_zero() {
        assert!(uchcha_bala(Graha::Rahu, 100.0).abs() < EPS);
        assert!(dig_bala(Graha::Ketu, 1).abs() < EPS);
        assert!(cheshta_bala(Graha::Rahu, -0.5).abs() < EPS);
        assert!(naisargika_bala(Graha::Ketu).abs() < EPS);
    }
}
