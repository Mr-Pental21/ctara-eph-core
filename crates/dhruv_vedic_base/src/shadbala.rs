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
    BeneficNature, ChandraBeneficRule, Dignity, GrahaGender,
    buddh_association_nature_with_chandra_rule, compound_dignity_in_rashi, graha_gender,
    is_own_sign_at_longitude, moolatrikone_range, moon_benefic_nature,
    moon_benefic_nature_with_rule, natural_benefic_malefic, own_signs,
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

/// Legacy maximum daily speeds (deg/day), retained for source compatibility.
///
/// Cheshta Bala is computed from correction-model madhyama/sphuta/chaloccha
/// longitudes, not speed normalization.
pub const MAX_SPEED: [f64; 7] = [1.0, 15.0, 0.8, 2.2, 0.25, 1.6, 0.13];

/// Required strength (shashtiamsas) for a graha to be considered strong.
pub const REQUIRED_STRENGTH: [f64; 7] = [390.0, 360.0, 300.0, 420.0, 390.0, 330.0, 300.0];

/// Saptavargaja dignity-to-points mapping.
///
/// BPHS Saptavargaja uses moolatrikona/own/five-fold friendship categories
/// for each varga sign; exaltation and debilitation are not separate
/// categories in this component.
const SAPTAVARGAJA_POINTS: [f64; 7] = [
    45.0, // Moolatrikone
    30.0, // OwnSign
    22.5, // AdhiMitra
    15.0, // Mitra
    7.5,  // Sama
    3.75, // Shatru
    1.875, // AdhiShatru
];

fn saptavargaja_dignity_points(dignity: Dignity) -> f64 {
    match dignity {
        Dignity::Moolatrikone => SAPTAVARGAJA_POINTS[0],
        Dignity::OwnSign => SAPTAVARGAJA_POINTS[1],
        Dignity::AdhiMitra => SAPTAVARGAJA_POINTS[2],
        Dignity::Mitra => SAPTAVARGAJA_POINTS[3],
        Dignity::Sama => SAPTAVARGAJA_POINTS[4],
        Dignity::Shatru => SAPTAVARGAJA_POINTS[5],
        Dignity::AdhiShatru => SAPTAVARGAJA_POINTS[6],
        Dignity::Exalted | Dignity::Debilitated => {
            unreachable!("Saptavargaja scoring does not classify exaltation/debilitation")
        }
    }
}

fn is_sapta_graha(graha: Graha) -> bool {
    graha.index() < 7
}

fn is_saptavargaja_moolatrikone(graha: Graha, varga_lon: f64) -> bool {
    let Some((mt_rashi, start, end)) = moolatrikone_range(graha) else {
        return false;
    };
    let lon = normalize_360(varga_lon);
    let lon_rashi = ((lon / 30.0).floor() as u8).min(11);
    if lon_rashi != mt_rashi {
        return false;
    }
    let deg_in_rashi = lon - (mt_rashi as f64) * 30.0;
    deg_in_rashi >= start && deg_in_rashi < end
}

fn saptavargaja_dignity(
    graha: Graha,
    varga_lon: f64,
    rashi_index: u8,
    d1_rashi_indices: &[u8; 7],
    is_rashi_chart: bool,
) -> Dignity {
    if matches!(graha, Graha::Rahu | Graha::Ketu) {
        return Dignity::Sama;
    }

    if is_rashi_chart {
        if is_saptavargaja_moolatrikone(graha, varga_lon) {
            return Dignity::Moolatrikone;
        }

        if is_own_sign_at_longitude(graha, varga_lon, rashi_index) {
            return Dignity::OwnSign;
        }
    } else if own_signs(graha).contains(&rashi_index) {
        return Dignity::OwnSign;
    }

    compound_dignity_in_rashi(graha, rashi_index, d1_rashi_indices)
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
/// `varga_longitudes` is the matching amsha longitude array.
/// D1 can use moolatrikona and degree-specific own-house dignity. Non-D1 vargas
/// use own sign where applicable, otherwise compound friendship to the varga
/// sign lord with temporary friendship from D1.
/// Exaltation/debilitation are not Saptavargaja categories.
pub fn saptavargaja_bala(
    graha: Graha,
    varga_rashi: &[[u8; 7]; 7],
    varga_longitudes: &[[f64; 7]; 7],
) -> f64 {
    if !is_sapta_graha(graha) {
        return 0.0;
    }
    let gi = graha.index() as usize;
    let d1_rashi_indices = &varga_rashi[0];
    let mut total = 0.0;
    for (vi, row) in varga_rashi.iter().take(7).enumerate() {
        let rashi_idx = row[gi];
        let dignity = saptavargaja_dignity(
            graha,
            varga_longitudes[vi][gi],
            rashi_idx,
            d1_rashi_indices,
            vi == 0,
        );
        total += saptavargaja_dignity_points(dignity);
    }
    total
}

/// Saptavargaja bala for all 7 sapta grahas.
pub fn all_saptavargaja_balas(
    varga_rashi: &[[u8; 7]; 7],
    varga_longitudes: &[[f64; 7]; 7],
) -> [f64; 7] {
    let mut result = [0.0; 7];
    for (i, g) in SAPTA_GRAHAS.iter().enumerate() {
        result[i] = saptavargaja_bala(*g, varga_rashi, varga_longitudes);
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

/// Drekkana Bala: male in 1st decanate=15, neuter in 2nd=15, female in 3rd=15.
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
        (GrahaGender::Neuter, 2) => 15.0,
        (GrahaGender::Female, 3) => 15.0,
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
    varga_longitudes: &[[f64; 7]; 7],
) -> SthanaBalaBreakdown {
    let u = uchcha_bala(graha, sid_lon);
    let s = saptavargaja_bala(graha, varga_rashi, varga_longitudes);
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
    varga_longitudes: &[[f64; 7]; 7],
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
        result[i] = sthana_bala(*g, lons[i], bhavas[i], varga_rashi, varga_longitudes);
    }
    result
}

// ---------------------------------------------------------------------------
// 2c. Dig Bala
// ---------------------------------------------------------------------------

/// Dig Bala from exact angular distance to the graha's max-strength cusp.
pub fn dig_bala(graha: Graha, graha_sidereal_lon: f64, max_cusp_sidereal_lon: f64) -> f64 {
    if !is_sapta_graha(graha) {
        return 0.0;
    }
    let diff = normalize_360(graha_sidereal_lon - max_cusp_sidereal_lon);
    let smaller_angle = diff.min(360.0 - diff).min(180.0);
    (60.0 * (1.0 - smaller_angle / 180.0)).clamp(0.0, 60.0)
}

/// Dig bala for all 7 sapta grahas.
pub fn all_dig_balas(sidereal_lons: &[f64; 7], max_cusp_sidereal_lons: &[f64; 7]) -> [f64; 7] {
    let mut result = [0.0; 7];
    for (i, g) in SAPTA_GRAHAS.iter().enumerate() {
        result[i] = dig_bala(*g, sidereal_lons[i], max_cusp_sidereal_lons[i]);
    }
    result
}

// ---------------------------------------------------------------------------
// 2d. Kala Bala Sub-Components
// ---------------------------------------------------------------------------

fn dynamic_benefic_nature(
    graha: Graha,
    sidereal_lons: &[f64; 9],
    moon_sun_elong: f64,
    chandra_rule: ChandraBeneficRule,
) -> BeneficNature {
    match graha {
        Graha::Chandra => moon_benefic_nature_with_rule(moon_sun_elong, chandra_rule),
        Graha::Buddh => buddh_association_nature_with_chandra_rule(sidereal_lons, chandra_rule),
        _ => natural_benefic_malefic(graha),
    }
}

fn sapta_to_all_sidereal_lons(sidereal_lons: &[f64; 7]) -> [f64; 9] {
    let mut all = [f64::NAN; 9];
    all[..7].copy_from_slice(sidereal_lons);
    all
}

/// Nathonnatha Bala: malefics strong by day (60), benefics by night (60).
///
/// This low-level helper only has Moon-Sun elongation, so Buddh falls back to
/// its default benefic nature. Full Shadbala uses
/// `nathonnatha_bala_with_longitudes`, which applies same-rashi association.
pub fn nathonnatha_bala(graha: Graha, is_daytime: bool, moon_sun_elong: f64) -> f64 {
    if !is_sapta_graha(graha) {
        return 0.0;
    }
    let nature = if graha == Graha::Chandra {
        moon_benefic_nature(moon_sun_elong)
    } else {
        natural_benefic_malefic(graha)
    };

    nathonnatha_bala_for_nature(nature, is_daytime)
}

pub fn nathonnatha_bala_with_longitudes(
    graha: Graha,
    is_daytime: bool,
    sidereal_lons: &[f64; 9],
    moon_sun_elong: f64,
) -> f64 {
    nathonnatha_bala_with_longitudes_and_rule(
        graha,
        is_daytime,
        sidereal_lons,
        moon_sun_elong,
        ChandraBeneficRule::default(),
    )
}

fn nathonnatha_bala_with_longitudes_and_rule(
    graha: Graha,
    is_daytime: bool,
    sidereal_lons: &[f64; 9],
    moon_sun_elong: f64,
    chandra_rule: ChandraBeneficRule,
) -> f64 {
    if !is_sapta_graha(graha) {
        return 0.0;
    }
    let nature = dynamic_benefic_nature(graha, sidereal_lons, moon_sun_elong, chandra_rule);
    nathonnatha_bala_for_nature(nature, is_daytime)
}

fn nathonnatha_bala_for_nature(nature: BeneficNature, is_daytime: bool) -> f64 {
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

    let nature = if graha == Graha::Chandra {
        moon_benefic_nature(moon_sun_elong)
    } else {
        natural_benefic_malefic(graha)
    };

    paksha_bala_for_nature(nature, moon_sun_elong)
}

pub fn paksha_bala_with_longitudes(
    graha: Graha,
    sidereal_lons: &[f64; 9],
    moon_sun_elong: f64,
) -> f64 {
    paksha_bala_with_longitudes_and_rule(
        graha,
        sidereal_lons,
        moon_sun_elong,
        ChandraBeneficRule::default(),
    )
}

fn paksha_bala_with_longitudes_and_rule(
    graha: Graha,
    sidereal_lons: &[f64; 9],
    moon_sun_elong: f64,
    chandra_rule: ChandraBeneficRule,
) -> f64 {
    if !is_sapta_graha(graha) {
        return 0.0;
    }
    let nature = dynamic_benefic_nature(graha, sidereal_lons, moon_sun_elong, chandra_rule);
    paksha_bala_for_nature(nature, moon_sun_elong)
}

fn paksha_bala_for_nature(nature: BeneficNature, moon_sun_elong: f64) -> f64 {
    let elong = normalize_360(moon_sun_elong);
    let phase_angle = if elong <= 180.0 { elong } else { 360.0 - elong };
    let benefic_score = phase_angle / 3.0; // 0 at new moon, 60 at full moon
    let malefic_score = 60.0 - benefic_score;

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
/// Chandra classification from Moon-Sun elongation. This low-level helper only
/// has Moon-Sun elongation, so Buddh falls back to its default benefic nature.
/// Full Shadbala uses `ayana_bala_with_longitudes`, which applies same-rashi
/// association for Buddh.
pub fn ayana_bala(graha: Graha, declination_deg: f64, moon_sun_elong: f64) -> f64 {
    if !is_sapta_graha(graha) {
        return 0.0;
    }

    let nature = if graha == Graha::Chandra {
        moon_benefic_nature(moon_sun_elong)
    } else {
        natural_benefic_malefic(graha)
    };

    ayana_bala_for_nature(nature, declination_deg)
}

pub fn ayana_bala_with_longitudes(
    graha: Graha,
    declination_deg: f64,
    sidereal_lons: &[f64; 9],
    moon_sun_elong: f64,
) -> f64 {
    ayana_bala_with_longitudes_and_rule(
        graha,
        declination_deg,
        sidereal_lons,
        moon_sun_elong,
        ChandraBeneficRule::default(),
    )
}

fn ayana_bala_with_longitudes_and_rule(
    graha: Graha,
    declination_deg: f64,
    sidereal_lons: &[f64; 9],
    moon_sun_elong: f64,
    chandra_rule: ChandraBeneficRule,
) -> f64 {
    if !is_sapta_graha(graha) {
        return 0.0;
    }
    let nature = dynamic_benefic_nature(graha, sidereal_lons, moon_sun_elong, chandra_rule);
    ayana_bala_for_nature(nature, declination_deg)
}

fn ayana_bala_for_nature(nature: BeneficNature, declination_deg: f64) -> f64 {
    let kranti = declination_deg.clamp(-24.0, 24.0);
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
    let sidereal_lons = sapta_to_all_sidereal_lons(&inputs.sidereal_lons);
    kala_bala_with_sidereal_lons(graha, inputs, &sidereal_lons)
}

/// Kala bala for a single graha with full 9-graha longitude context.
pub fn kala_bala_with_sidereal_lons(
    graha: Graha,
    inputs: &KalaBalaInputs,
    sidereal_lons: &[f64; 9],
) -> KalaBalaBreakdown {
    kala_bala_with_sidereal_lons_and_rule(
        graha,
        inputs,
        sidereal_lons,
        ChandraBeneficRule::default(),
    )
}

fn kala_bala_with_sidereal_lons_and_rule(
    graha: Graha,
    inputs: &KalaBalaInputs,
    sidereal_lons: &[f64; 9],
    chandra_rule: ChandraBeneficRule,
) -> KalaBalaBreakdown {
    let n = nathonnatha_bala_with_longitudes_and_rule(
        graha,
        inputs.is_daytime,
        sidereal_lons,
        inputs.moon_sun_elongation,
        chandra_rule,
    );
    let p = paksha_bala_with_longitudes_and_rule(
        graha,
        sidereal_lons,
        inputs.moon_sun_elongation,
        chandra_rule,
    );
    let t = tribhaga_bala(graha, inputs.is_daytime, inputs.day_night_fraction);
    let ab = abda_bala(graha, inputs.year_lord);
    let ma = masa_bala(graha, inputs.month_lord);
    let va = vara_bala(graha, inputs.weekday_lord);
    let ho = hora_bala(graha, inputs.hora_lord);
    let ay = ayana_bala_with_longitudes_and_rule(
        graha,
        inputs.graha_declinations[graha.index().min(6) as usize],
        sidereal_lons,
        inputs.moon_sun_elongation,
        chandra_rule,
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

/// Circular midpoint of two longitudes.
///
/// The second longitude is first unwrapped to the nearest equivalent position
/// relative to the first longitude so a pair such as 350° and 10° averages to
/// 0° instead of 180°.
pub fn circular_longitude_midpoint(a: f64, b: f64) -> f64 {
    let delta = normalize_360(b - a);
    let signed_delta = if delta > 180.0 { delta - 360.0 } else { delta };
    normalize_360(a + signed_delta / 2.0)
}

/// Fold an angular separation to the smaller 0..=180° arc.
pub fn smaller_longitude_arc(separation: f64) -> f64 {
    let normalized = normalize_360(separation);
    if normalized <= 180.0 {
        normalized
    } else {
        360.0 - normalized
    }
}

/// Cheshta Kendra from correction-model madhyama/sphuta/chaloccha longitudes.
pub fn cheshta_kendra(
    madhyama_sidereal_longitude: f64,
    sphuta_sidereal_longitude: f64,
    chaloccha_sidereal_longitude: f64,
) -> f64 {
    let midpoint =
        circular_longitude_midpoint(madhyama_sidereal_longitude, sphuta_sidereal_longitude);
    smaller_longitude_arc(chaloccha_sidereal_longitude - midpoint)
}

/// Cheshta Bala from correction-model chaloccha.
///
/// Mangal through Shani use the smaller angular distance between their
/// correction-model chaloccha and the midpoint of madhyama+sphuta, divided by
/// 3. Surya and Chandra remain 0.
pub fn cheshta_bala(
    graha: Graha,
    madhyama_sidereal_longitude: f64,
    sphuta_sidereal_longitude: f64,
    chaloccha_sidereal_longitude: f64,
) -> f64 {
    if !is_sapta_graha(graha) {
        return 0.0;
    }
    let gi = graha.index() as usize;
    if gi < 2 {
        return 0.0;
    }

    cheshta_kendra(
        madhyama_sidereal_longitude,
        sphuta_sidereal_longitude,
        chaloccha_sidereal_longitude,
    ) / 3.0
}

/// Cheshta bala for all 7.
pub fn all_cheshta_balas(
    madhyama_sidereal_lons: &[f64; 7],
    sphuta_sidereal_lons: &[f64; 7],
    chaloccha_sidereal_lons: &[f64; 7],
) -> [f64; 7] {
    let mut result = [0.0; 7];
    for (i, g) in SAPTA_GRAHAS.iter().enumerate() {
        result[i] = cheshta_bala(
            *g,
            madhyama_sidereal_lons[i],
            sphuta_sidereal_lons[i],
            chaloccha_sidereal_lons[i],
        );
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

/// Drik Bala: signed aspect strength.
///
/// For each incoming aspect, classify the aspecting graha as benefic or malefic,
/// then sum total virupas. By default Guru and Buddh are included in this
/// divided balance. When `divide_guru_buddh_drishti_by_4` is false, their signed
/// full incoming drishti virupa is added after the divided balance.
/// `sidereal_lons` = all 9 grahas. `moon_sun_elong` classifies Chandra;
/// Buddh is classified by same-rashi association.
pub fn drik_bala_with_node_aspects(
    graha: Graha,
    sidereal_lons: &[f64; 9],
    moon_sun_elong: f64,
    include_node_aspects: bool,
    divide_guru_buddh_drishti_by_4: bool,
    chandra_rule: ChandraBeneficRule,
) -> f64 {
    if !is_sapta_graha(graha) {
        return 0.0;
    }
    let target_lon = sidereal_lons[graha.index() as usize];
    let mut benefic_sum = 0.0;
    let mut malefic_sum = 0.0;
    let mut guru_buddh_full_drishti = 0.0;

    for src in crate::graha::ALL_GRAHAS {
        if src == graha {
            continue;
        }
        if !include_node_aspects && matches!(src, Graha::Rahu | Graha::Ketu) {
            continue;
        }
        let src_lon = sidereal_lons[src.index() as usize];
        let ang = normalize_360(target_lon - src_lon);
        let bv = base_virupa(ang);
        let sv = special_virupa(src, ang);
        let total = bv + sv;
        let nature = dynamic_benefic_nature(src, sidereal_lons, moon_sun_elong, chandra_rule);

        if matches!(src, Graha::Guru | Graha::Buddh) && !divide_guru_buddh_drishti_by_4 {
            match nature {
                BeneficNature::Benefic => guru_buddh_full_drishti += total,
                BeneficNature::Malefic => guru_buddh_full_drishti -= total,
            }
            continue;
        }

        match nature {
            BeneficNature::Benefic => benefic_sum += total,
            BeneficNature::Malefic => malefic_sum += total,
        }
    }

    (benefic_sum - malefic_sum) / 4.0 + guru_buddh_full_drishti
}

/// Drik Bala with Rahu/Ketu incoming aspects excluded.
pub fn drik_bala(graha: Graha, sidereal_lons: &[f64; 9], moon_sun_elong: f64) -> f64 {
    drik_bala_with_node_aspects(
        graha,
        sidereal_lons,
        moon_sun_elong,
        false,
        true,
        ChandraBeneficRule::default(),
    )
}

/// Drik bala for all 7 sapta grahas.
pub fn all_drik_balas_with_node_aspects(
    sidereal_lons: &[f64; 9],
    moon_sun_elong: f64,
    include_node_aspects: bool,
    divide_guru_buddh_drishti_by_4: bool,
    chandra_rule: ChandraBeneficRule,
) -> [f64; 7] {
    let mut result = [0.0; 7];
    for (i, g) in SAPTA_GRAHAS.iter().enumerate() {
        result[i] = drik_bala_with_node_aspects(
            *g,
            sidereal_lons,
            moon_sun_elong,
            include_node_aspects,
            divide_guru_buddh_drishti_by_4,
            chandra_rule,
        );
    }
    result
}

/// Drik bala for all 7 sapta grahas with Rahu/Ketu incoming aspects excluded.
pub fn all_drik_balas(sidereal_lons: &[f64; 9], moon_sun_elong: f64) -> [f64; 7] {
    all_drik_balas_with_node_aspects(
        sidereal_lons,
        moon_sun_elong,
        false,
        true,
        ChandraBeneficRule::default(),
    )
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
    /// Sidereal max-strength cusp longitude for each sapta graha's Dig Bala.
    pub dig_bala_max_cusp_lons: [f64; 7],
    /// Correction-model madhyama longitude for sapta grahas.
    pub cheshta_madhyama_lons: [f64; 7],
    /// Correction-model chaloccha longitude for sapta grahas.
    pub cheshta_chaloccha_lons: [f64; 7],
    /// Kala bala inputs.
    pub kala: KalaBalaInputs,
    /// Include Rahu/Ketu incoming aspect contributions in Drik Bala.
    pub include_node_aspects_for_drik_bala: bool,
    /// Divide Guru/Buddh incoming drishti by 4 in Drik Bala.
    pub divide_guru_buddh_drishti_by_4_for_drik_bala: bool,
    /// Rule for Chandra benefic/malefic classification in nature-dependent bala.
    pub chandra_benefic_rule: ChandraBeneficRule,
    /// 7 vargas x 7 grahas rashi indices (for saptavargaja bala).
    pub varga_rashi_indices: [[u8; 7]; 7],
    /// 7 vargas x 7 grahas amsha longitudes (for degree-specific saptavargaja bala).
    pub varga_longitudes: [[f64; 7]; 7],
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
        &inputs.varga_longitudes,
    );
    let dig = dig_bala(
        graha,
        inputs.sidereal_lons[gi],
        inputs.dig_bala_max_cusp_lons[gi],
    );
    let kala_result = kala_bala_with_sidereal_lons_and_rule(
        graha,
        &inputs.kala,
        &inputs.sidereal_lons,
        inputs.chandra_benefic_rule,
    );
    let cheshta = cheshta_bala(
        graha,
        inputs.cheshta_madhyama_lons[gi],
        inputs.sidereal_lons[gi],
        inputs.cheshta_chaloccha_lons[gi],
    );
    let nais = naisargika_bala(graha);
    let drik = drik_bala_with_node_aspects(
        graha,
        &inputs.sidereal_lons,
        inputs.kala.moon_sun_elongation,
        inputs.include_node_aspects_for_drik_bala,
        inputs.divide_guru_buddh_drishti_by_4_for_drik_bala,
        inputs.chandra_benefic_rule,
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
        assert!((dig_bala(Graha::Surya, 90.0, 90.0) - 60.0).abs() < EPS);
    }

    #[test]
    fn dig_sun_at_opposite() {
        assert!(dig_bala(Graha::Surya, 270.0, 90.0).abs() < EPS);
    }

    #[test]
    fn dig_sun_90_degrees_from_max() {
        assert!((dig_bala(Graha::Surya, 180.0, 90.0) - 30.0).abs() < EPS);
    }

    #[test]
    fn dig_same_bhava_can_differ_by_exact_longitude() {
        let near = dig_bala(Graha::Surya, 95.0, 90.0);
        let far = dig_bala(Graha::Surya, 119.0, 90.0);
        assert!(near > far);
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
    fn drekkana_female_third_decanate() {
        // Moon (female) at 25 deg (3rd decanate)
        assert!((drekkana_bala(Graha::Chandra, 25.0) - 15.0).abs() < EPS);
    }

    #[test]
    fn drekkana_neuter_second_decanate() {
        // Mercury (neuter) at 15 deg (2nd decanate)
        assert!((drekkana_bala(Graha::Buddh, 15.0) - 15.0).abs() < EPS);
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
    fn circular_midpoint_handles_wrap() {
        assert!((circular_longitude_midpoint(350.0, 10.0) - 0.0).abs() < EPS);
        assert!((circular_longitude_midpoint(10.0, 350.0) - 0.0).abs() < EPS);
        assert!((circular_longitude_midpoint(20.0, 80.0) - 50.0).abs() < EPS);
    }

    #[test]
    fn cheshta_kendra_formula_quadrants() {
        assert!((cheshta_bala(Graha::Mangal, 10.0, 10.0, 10.0) - 0.0).abs() < EPS);
        assert!((cheshta_bala(Graha::Mangal, 10.0, 10.0, 100.0) - 30.0).abs() < EPS);
        assert!((cheshta_bala(Graha::Mangal, 10.0, 10.0, 190.0) - 60.0).abs() < EPS);
        assert!((cheshta_bala(Graha::Mangal, 10.0, 10.0, 280.0) - 30.0).abs() < EPS);
        assert!((cheshta_bala(Graha::Mangal, 10.0, 10.0, 370.0) - 0.0).abs() < EPS);
    }

    #[test]
    fn cheshta_kendra_uses_midpoint_and_folds_arc() {
        assert!((cheshta_kendra(350.0, 10.0, 270.0) - 90.0).abs() < EPS);
        assert!((cheshta_bala(Graha::Mangal, 350.0, 10.0, 350.0) - (10.0 / 3.0)).abs() < EPS);
    }

    #[test]
    fn cheshta_sun_always_zero() {
        assert!(cheshta_bala(Graha::Surya, 10.0, 10.0, 190.0).abs() < EPS);
        assert!(cheshta_bala(Graha::Chandra, 10.0, 10.0, 190.0).abs() < EPS);
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

    #[test]
    fn drik_bala_divides_guru_and_buddh_drishti_by_default() {
        let mut lons = [0.0f64; 9];
        lons[Graha::Surya.index() as usize] = 0.0;
        lons[Graha::Chandra.index() as usize] = 10.0; // zero aspect to Surya
        lons[Graha::Mangal.index() as usize] = 20.0; // zero aspect to Surya
        lons[Graha::Buddh.index() as usize] = 270.0; // 90° to Surya -> 45 virupa
        lons[Graha::Guru.index() as usize] = 180.0; // 180° to Surya -> 60 virupa
        lons[Graha::Shukra.index() as usize] = 40.0; // zero aspect to Surya
        lons[Graha::Shani.index() as usize] = 50.0; // zero aspect to Surya
        lons[Graha::Rahu.index() as usize] = 60.0; // 300° to Surya -> 0 virupa
        lons[Graha::Ketu.index() as usize] = 330.0; // 30° to Surya -> 0 virupa

        let expected = (60.0 + 45.0) / 4.0;

        assert!((drik_bala(Graha::Surya, &lons, 180.0) - expected).abs() < EPS);

        let full = drik_bala_with_node_aspects(
            Graha::Surya,
            &lons,
            180.0,
            false,
            false,
            ChandraBeneficRule::default(),
        );
        assert!((full - 105.0).abs() < EPS);
    }

    #[test]
    fn drik_bala_divides_malefic_buddh_by_default() {
        let mut lons = [0.0f64; 9];
        lons[Graha::Surya.index() as usize] = 0.0;
        lons[Graha::Chandra.index() as usize] = 0.0;
        lons[Graha::Mangal.index() as usize] = 270.0;
        lons[Graha::Buddh.index() as usize] = 270.0; // Same rashi as Mangal.
        lons[Graha::Guru.index() as usize] = 0.0;
        lons[Graha::Shukra.index() as usize] = 0.0;
        lons[Graha::Shani.index() as usize] = 0.0;
        lons[Graha::Rahu.index() as usize] = 0.0;
        lons[Graha::Ketu.index() as usize] = 0.0;

        // Mangal and malefic Buddh both contribute through the /4 balance.
        let expected = (0.0 - 105.0) / 4.0;
        assert!((drik_bala(Graha::Surya, &lons, 0.0) - expected).abs() < EPS);

        let full_buddh = drik_bala_with_node_aspects(
            Graha::Surya,
            &lons,
            0.0,
            false,
            false,
            ChandraBeneficRule::default(),
        );
        assert!((full_buddh - ((0.0 - 60.0) / 4.0 - 45.0)).abs() < EPS);
    }

    #[test]
    fn drik_bala_excludes_node_aspects_by_default_and_can_include_them() {
        let mut lons = [0.0f64; 9];
        lons[Graha::Surya.index() as usize] = 0.0;
        lons[Graha::Rahu.index() as usize] = 180.0; // Full incoming Rahu aspect to Surya.
        lons[Graha::Ketu.index() as usize] = 0.0;

        assert!(drik_bala(Graha::Surya, &lons, 180.0).abs() < EPS);

        let with_nodes = drik_bala_with_node_aspects(
            Graha::Surya,
            &lons,
            180.0,
            true,
            true,
            ChandraBeneficRule::default(),
        );
        assert!((with_nodes + 15.0).abs() < EPS);
    }

    // --- Saptavargaja ---

    #[test]
    fn saptavargaja_uses_d1_positions_for_temporal_friendship() {
        // Mercury at 45 deg (Vrishabha). Not exalted/debilitated/own — relies on friendship.
        // Vrishabha lord = Venus. Mercury-Venus = naisargika Friend.
        // D1 keeps Venus in the same sign as Mercury: temporary enemy → Sama.
        // Other vargas move Venus to the next sign, which would be temporary friend
        // if the implementation incorrectly used each varga for temporal friendship.
        let mut varga_rashi: [[u8; 7]; 7] = [[1; 7]; 7]; // all in Vrishabha
        for row in varga_rashi.iter_mut().skip(1) {
            row[5] = 2; // Venus in Mithuna outside D1
        }
        let varga_longitudes = [[45.0; 7]; 7];

        let bala = saptavargaja_bala(Graha::Buddh, &varga_rashi, &varga_longitudes);

        // Mercury-Venus naisargika=Friend, D1 tatkalika=enemy → Sama → 7.5 per varga.
        assert!((bala - 52.5).abs() < EPS, "expected 7 Sama vargas");
    }

    #[test]
    fn saptavargaja_uses_varga_sign_not_natal_exaltation() {
        // Natal Sun in Mesha must not make Sun exalted in Vrishabha vargas.
        // Vrishabha lord Venus is Sun's natural enemy; with all grahas in the
        // same sign, temporal relation is enemy, so each varga is AdhiShatru.
        let varga_rashi: [[u8; 7]; 7] = [[1; 7]; 7];
        let varga_longitudes = [[45.0; 7]; 7];
        let bala = saptavargaja_bala(Graha::Surya, &varga_rashi, &varga_longitudes);
        assert!((bala - 13.125).abs() < EPS);
    }

    #[test]
    fn saptavargaja_moolatrikona_is_degree_specific() {
        let mut varga_rashi = [[0u8; 7]; 7];
        let mut varga_longitudes = [[0.0f64; 7]; 7];

        // Before the MT zone, Mercury in Kanya falls through to self-lord
        // compound friendship (Shatru here), and Moon in Vrishabha falls
        // through to Venus compound friendship (Mitra here).
        for row in &mut varga_rashi {
            row[Graha::Buddh.index() as usize] = 5; // Kanya
            row[Graha::Chandra.index() as usize] = 1; // Vrishabha
        }
        for row in &mut varga_longitudes {
            row[Graha::Buddh.index() as usize] = 165.0; // Kanya 15° -> MT starts
            row[Graha::Chandra.index() as usize] = 33.0; // Vrishabha 3° -> MT starts
        }

        assert!(
            (saptavargaja_bala(Graha::Buddh, &varga_rashi, &varga_longitudes) - 225.0).abs()
                < EPS
        );
        assert!(
            (saptavargaja_bala(Graha::Chandra, &varga_rashi, &varga_longitudes) - 135.0).abs()
                < EPS
        );

        for row in &mut varga_longitudes {
            row[Graha::Buddh.index() as usize] = 164.999; // still exaltation zone, not MT
            row[Graha::Chandra.index() as usize] = 32.999; // still exaltation zone, not MT
        }

        assert!(
            (saptavargaja_bala(Graha::Buddh, &varga_rashi, &varga_longitudes) - 183.75).abs()
                < EPS
        );
        assert!(
            (saptavargaja_bala(Graha::Chandra, &varga_rashi, &varga_longitudes) - 105.0).abs()
                < EPS
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
        assert!(dig_bala(Graha::Ketu, 0.0, 0.0).abs() < EPS);
        assert!(cheshta_bala(Graha::Rahu, 10.0, 10.0, 190.0).abs() < EPS);
        assert!(naisargika_bala(Graha::Ketu).abs() < EPS);
    }
}
