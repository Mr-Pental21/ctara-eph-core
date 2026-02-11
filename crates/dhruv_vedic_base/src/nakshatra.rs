//! Nakshatra (lunar mansion) computation for 27-fold and 28-fold schemes.
//!
//! The ecliptic circle is divided into 27 equal nakshatras of 13 deg 20'
//! (13.3333... deg) each, or 28 non-uniform nakshatras when Abhijit is
//! included.
//!
//! Each of the 27 nakshatras has 4 padas (quarters) of 3 deg 20' each.
//! Abhijit (in the 28-scheme) has no standard pada division (pada = 0).
//!
//! Clean-room implementation from universal Vedic convention.
//! See `docs/clean_room_rashi_nakshatra.md`.

use crate::ayanamsha::{AyanamshaSystem, ayanamsha_deg, jd_tdb_to_centuries};

/// Span of one nakshatra in the 27-scheme: 360/27 = 13.3333... degrees.
pub const NAKSHATRA_SPAN_27: f64 = 360.0 / 27.0;

/// Span of one pada: 13.3333.../4 = 3.3333... degrees.
pub const PADA_SPAN: f64 = NAKSHATRA_SPAN_27 / 4.0;

// ---------------------------------------------------------------------------
// 27-Nakshatra scheme
// ---------------------------------------------------------------------------

/// The 27 nakshatras from Ashwini to Revati (uniform 13 deg 20' each).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Nakshatra {
    Ashwini,
    Bharani,
    Krittika,
    Rohini,
    Mrigashira,
    Ardra,
    Punarvasu,
    Pushya,
    Ashlesha,
    Magha,
    PurvaPhalguni,
    UttaraPhalguni,
    Hasta,
    Chitra,
    Swati,
    Vishakha,
    Anuradha,
    Jyeshtha,
    Mula,
    PurvaAshadha,
    UttaraAshadha,
    Shravana,
    Dhanishtha,
    Shatabhisha,
    PurvaBhadrapada,
    UttaraBhadrapada,
    Revati,
}

/// All 27 nakshatras in order, for FFI indexing (0 = Ashwini, 26 = Revati).
pub const ALL_NAKSHATRAS_27: [Nakshatra; 27] = [
    Nakshatra::Ashwini,
    Nakshatra::Bharani,
    Nakshatra::Krittika,
    Nakshatra::Rohini,
    Nakshatra::Mrigashira,
    Nakshatra::Ardra,
    Nakshatra::Punarvasu,
    Nakshatra::Pushya,
    Nakshatra::Ashlesha,
    Nakshatra::Magha,
    Nakshatra::PurvaPhalguni,
    Nakshatra::UttaraPhalguni,
    Nakshatra::Hasta,
    Nakshatra::Chitra,
    Nakshatra::Swati,
    Nakshatra::Vishakha,
    Nakshatra::Anuradha,
    Nakshatra::Jyeshtha,
    Nakshatra::Mula,
    Nakshatra::PurvaAshadha,
    Nakshatra::UttaraAshadha,
    Nakshatra::Shravana,
    Nakshatra::Dhanishtha,
    Nakshatra::Shatabhisha,
    Nakshatra::PurvaBhadrapada,
    Nakshatra::UttaraBhadrapada,
    Nakshatra::Revati,
];

impl Nakshatra {
    /// Sanskrit name of the nakshatra.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Ashwini => "Ashwini",
            Self::Bharani => "Bharani",
            Self::Krittika => "Krittika",
            Self::Rohini => "Rohini",
            Self::Mrigashira => "Mrigashira",
            Self::Ardra => "Ardra",
            Self::Punarvasu => "Punarvasu",
            Self::Pushya => "Pushya",
            Self::Ashlesha => "Ashlesha",
            Self::Magha => "Magha",
            Self::PurvaPhalguni => "Purva Phalguni",
            Self::UttaraPhalguni => "Uttara Phalguni",
            Self::Hasta => "Hasta",
            Self::Chitra => "Chitra",
            Self::Swati => "Swati",
            Self::Vishakha => "Vishakha",
            Self::Anuradha => "Anuradha",
            Self::Jyeshtha => "Jyeshtha",
            Self::Mula => "Mula",
            Self::PurvaAshadha => "Purva Ashadha",
            Self::UttaraAshadha => "Uttara Ashadha",
            Self::Shravana => "Shravana",
            Self::Dhanishtha => "Dhanishtha",
            Self::Shatabhisha => "Shatabhisha",
            Self::PurvaBhadrapada => "Purva Bhadrapada",
            Self::UttaraBhadrapada => "Uttara Bhadrapada",
            Self::Revati => "Revati",
        }
    }

    /// 0-based index (Ashwini=0 .. Revati=26).
    pub const fn index(self) -> u8 {
        match self {
            Self::Ashwini => 0,
            Self::Bharani => 1,
            Self::Krittika => 2,
            Self::Rohini => 3,
            Self::Mrigashira => 4,
            Self::Ardra => 5,
            Self::Punarvasu => 6,
            Self::Pushya => 7,
            Self::Ashlesha => 8,
            Self::Magha => 9,
            Self::PurvaPhalguni => 10,
            Self::UttaraPhalguni => 11,
            Self::Hasta => 12,
            Self::Chitra => 13,
            Self::Swati => 14,
            Self::Vishakha => 15,
            Self::Anuradha => 16,
            Self::Jyeshtha => 17,
            Self::Mula => 18,
            Self::PurvaAshadha => 19,
            Self::UttaraAshadha => 20,
            Self::Shravana => 21,
            Self::Dhanishtha => 22,
            Self::Shatabhisha => 23,
            Self::PurvaBhadrapada => 24,
            Self::UttaraBhadrapada => 25,
            Self::Revati => 26,
        }
    }

    /// All 27 nakshatras in order.
    pub const fn all() -> &'static [Nakshatra; 27] {
        &ALL_NAKSHATRAS_27
    }
}

/// Result of 27-nakshatra lookup.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NakshatraInfo {
    /// The nakshatra.
    pub nakshatra: Nakshatra,
    /// 0-based index (0 = Ashwini).
    pub nakshatra_index: u8,
    /// Pada (quarter) within the nakshatra, 1-4.
    pub pada: u8,
    /// Decimal degrees within the nakshatra [0.0, 13.333...).
    pub degrees_in_nakshatra: f64,
    /// Decimal degrees within the pada [0.0, 3.333...).
    pub degrees_in_pada: f64,
}

/// Normalize longitude to [0, 360).
fn normalize_360(deg: f64) -> f64 {
    let r = deg % 360.0;
    if r < 0.0 { r + 360.0 } else { r }
}

/// Determine nakshatra and pada from sidereal ecliptic longitude (27-scheme).
///
/// Each nakshatra spans 13 deg 20' (13.3333... deg). Each pada spans 3 deg 20'.
pub fn nakshatra_from_longitude(sidereal_lon_deg: f64) -> NakshatraInfo {
    let lon = normalize_360(sidereal_lon_deg);
    let nak_idx = (lon / NAKSHATRA_SPAN_27).floor() as u8;
    let nak_idx = nak_idx.min(26);
    let degrees_in_nakshatra = lon - (nak_idx as f64) * NAKSHATRA_SPAN_27;
    let pada_idx = (degrees_in_nakshatra / PADA_SPAN).floor() as u8;
    let pada = pada_idx.min(3) + 1; // 1-based
    let degrees_in_pada = degrees_in_nakshatra - (pada_idx.min(3) as f64) * PADA_SPAN;

    NakshatraInfo {
        nakshatra: ALL_NAKSHATRAS_27[nak_idx as usize],
        nakshatra_index: nak_idx,
        pada,
        degrees_in_nakshatra,
        degrees_in_pada,
    }
}

/// Convenience: determine nakshatra from tropical longitude + ayanamsha (27-scheme).
pub fn nakshatra_from_tropical(
    tropical_lon_deg: f64,
    system: AyanamshaSystem,
    jd_tdb: f64,
    use_nutation: bool,
) -> NakshatraInfo {
    let t = jd_tdb_to_centuries(jd_tdb);
    let aya = ayanamsha_deg(system, t, use_nutation);
    nakshatra_from_longitude(tropical_lon_deg - aya)
}

// ---------------------------------------------------------------------------
// 28-Nakshatra scheme (with Abhijit)
// ---------------------------------------------------------------------------

/// The 28 nakshatras including Abhijit (inserted between Uttara Ashadha and
/// Shravana at index 21).
///
/// In this scheme, Uttara Ashadha, Abhijit, and Shravana have non-uniform
/// spans. All other nakshatras retain their 13 deg 20' span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Nakshatra28 {
    Ashwini,
    Bharani,
    Krittika,
    Rohini,
    Mrigashira,
    Ardra,
    Punarvasu,
    Pushya,
    Ashlesha,
    Magha,
    PurvaPhalguni,
    UttaraPhalguni,
    Hasta,
    Chitra,
    Swati,
    Vishakha,
    Anuradha,
    Jyeshtha,
    Mula,
    PurvaAshadha,
    UttaraAshadha,
    Abhijit,
    Shravana,
    Dhanishtha,
    Shatabhisha,
    PurvaBhadrapada,
    UttaraBhadrapada,
    Revati,
}

/// All 28 nakshatras in order, for FFI indexing.
pub const ALL_NAKSHATRAS_28: [Nakshatra28; 28] = [
    Nakshatra28::Ashwini,
    Nakshatra28::Bharani,
    Nakshatra28::Krittika,
    Nakshatra28::Rohini,
    Nakshatra28::Mrigashira,
    Nakshatra28::Ardra,
    Nakshatra28::Punarvasu,
    Nakshatra28::Pushya,
    Nakshatra28::Ashlesha,
    Nakshatra28::Magha,
    Nakshatra28::PurvaPhalguni,
    Nakshatra28::UttaraPhalguni,
    Nakshatra28::Hasta,
    Nakshatra28::Chitra,
    Nakshatra28::Swati,
    Nakshatra28::Vishakha,
    Nakshatra28::Anuradha,
    Nakshatra28::Jyeshtha,
    Nakshatra28::Mula,
    Nakshatra28::PurvaAshadha,
    Nakshatra28::UttaraAshadha,
    Nakshatra28::Abhijit,
    Nakshatra28::Shravana,
    Nakshatra28::Dhanishtha,
    Nakshatra28::Shatabhisha,
    Nakshatra28::PurvaBhadrapada,
    Nakshatra28::UttaraBhadrapada,
    Nakshatra28::Revati,
];

impl Nakshatra28 {
    /// Name of the nakshatra.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Ashwini => "Ashwini",
            Self::Bharani => "Bharani",
            Self::Krittika => "Krittika",
            Self::Rohini => "Rohini",
            Self::Mrigashira => "Mrigashira",
            Self::Ardra => "Ardra",
            Self::Punarvasu => "Punarvasu",
            Self::Pushya => "Pushya",
            Self::Ashlesha => "Ashlesha",
            Self::Magha => "Magha",
            Self::PurvaPhalguni => "Purva Phalguni",
            Self::UttaraPhalguni => "Uttara Phalguni",
            Self::Hasta => "Hasta",
            Self::Chitra => "Chitra",
            Self::Swati => "Swati",
            Self::Vishakha => "Vishakha",
            Self::Anuradha => "Anuradha",
            Self::Jyeshtha => "Jyeshtha",
            Self::Mula => "Mula",
            Self::PurvaAshadha => "Purva Ashadha",
            Self::UttaraAshadha => "Uttara Ashadha",
            Self::Abhijit => "Abhijit",
            Self::Shravana => "Shravana",
            Self::Dhanishtha => "Dhanishtha",
            Self::Shatabhisha => "Shatabhisha",
            Self::PurvaBhadrapada => "Purva Bhadrapada",
            Self::UttaraBhadrapada => "Uttara Bhadrapada",
            Self::Revati => "Revati",
        }
    }

    /// 0-based index (Ashwini=0 .. Revati=27).
    pub const fn index(self) -> u8 {
        match self {
            Self::Ashwini => 0,
            Self::Bharani => 1,
            Self::Krittika => 2,
            Self::Rohini => 3,
            Self::Mrigashira => 4,
            Self::Ardra => 5,
            Self::Punarvasu => 6,
            Self::Pushya => 7,
            Self::Ashlesha => 8,
            Self::Magha => 9,
            Self::PurvaPhalguni => 10,
            Self::UttaraPhalguni => 11,
            Self::Hasta => 12,
            Self::Chitra => 13,
            Self::Swati => 14,
            Self::Vishakha => 15,
            Self::Anuradha => 16,
            Self::Jyeshtha => 17,
            Self::Mula => 18,
            Self::PurvaAshadha => 19,
            Self::UttaraAshadha => 20,
            Self::Abhijit => 21,
            Self::Shravana => 22,
            Self::Dhanishtha => 23,
            Self::Shatabhisha => 24,
            Self::PurvaBhadrapada => 25,
            Self::UttaraBhadrapada => 26,
            Self::Revati => 27,
        }
    }

    /// All 28 nakshatras in order.
    pub const fn all() -> &'static [Nakshatra28; 28] {
        &ALL_NAKSHATRAS_28
    }
}

/// Result of 28-nakshatra lookup.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Nakshatra28Info {
    /// The nakshatra (28-scheme).
    pub nakshatra: Nakshatra28,
    /// 0-based index (0 = Ashwini, 21 = Abhijit, 27 = Revati).
    pub nakshatra_index: u8,
    /// Pada within the nakshatra. 1-4 for standard nakshatras, 0 for Abhijit
    /// (Abhijit pada is traditionally debated and not standardized).
    pub pada: u8,
    /// Decimal degrees within this nakshatra.
    pub degrees_in_nakshatra: f64,
}

/// Abhijit start: 276 deg 40' = 276 + 40/60 degrees.
const ABHIJIT_START: f64 = 276.0 + 40.0 / 60.0;
/// Abhijit end: 280 deg 53' 20" = 280 + 53/60 + 20/3600 degrees.
const ABHIJIT_END: f64 = 280.0 + 53.0 / 60.0 + 20.0 / 3600.0;

/// 28-nakshatra boundary table: (start_deg, end_deg) for each nakshatra.
///
/// Nakshatras 0-19 (Ashwini through Purva Ashadha) and 23-27 (Dhanishtha
/// through Revati) have uniform 13 deg 20' spans (same as 27-scheme).
///
/// The non-uniform region covers:
/// - Uttara Ashadha (20): [266deg40', 276deg40'] — 10 deg (shortened from 13deg20')
/// - Abhijit (21): [276deg40', 280deg53'20"] — 4deg13'20" (the Vega/Abhijit segment)
/// - Shravana (22): [280deg53'20', 293deg20'] — 12deg26'40" (shortened from 13deg20')
///
/// Note: Uttara Ashadha's 27-scheme span is [266deg40', 280deg00']. In the
/// 28-scheme, the portion [276deg40', 280deg00'] is given to Abhijit, and
/// [280deg00', 280deg53'20"] is taken from Shravana's start.
fn nakshatra_28_boundaries() -> [(f64, f64); 28] {
    let span = NAKSHATRA_SPAN_27; // 13.3333...
    let mut bounds = [(0.0_f64, 0.0_f64); 28];

    // Nakshatras 0-19: uniform, same as 27-scheme indices 0-19
    for i in 0..20 {
        bounds[i] = (i as f64 * span, (i + 1) as f64 * span);
    }

    // Non-uniform region (indices 20-22):
    // In 27-scheme: Uttara Ashadha(20)=[266.667, 280.000], Shravana(21)=[280.000, 293.333]
    // In 28-scheme:
    bounds[20] = (20.0 * span, ABHIJIT_START); // Uttara Ashadha: [266.667, 276.667]
    bounds[21] = (ABHIJIT_START, ABHIJIT_END); // Abhijit: [276.667, 280.889]
    bounds[22] = (ABHIJIT_END, 22.0 * span); // Shravana: [280.889, 293.333]

    // Nakshatras 23-27: offset by +1 from 27-scheme indices 22-26
    for i in 23..28 {
        let nak27_idx = i - 1; // map 28-scheme 23..27 to 27-scheme 22..26
        bounds[i] = (nak27_idx as f64 * span, (nak27_idx + 1) as f64 * span);
    }

    bounds
}

/// Determine nakshatra from sidereal longitude (28-scheme with Abhijit).
///
/// Abhijit spans approximately 276deg40' to 280deg53'20". Within Abhijit,
/// pada is set to 0 (not applicable).
pub fn nakshatra28_from_longitude(sidereal_lon_deg: f64) -> Nakshatra28Info {
    let lon = normalize_360(sidereal_lon_deg);
    let bounds = nakshatra_28_boundaries();

    for (i, &(start, end)) in bounds.iter().enumerate() {
        if lon >= start && (lon < end || (i == 27 && lon <= end)) {
            let degrees_in = lon - start;
            let pada = if i == 21 {
                // Abhijit: no standard pada
                0
            } else {
                let span = end - start;
                let pada_span = span / 4.0;
                let p = (degrees_in / pada_span).floor() as u8;
                p.min(3) + 1
            };
            return Nakshatra28Info {
                nakshatra: ALL_NAKSHATRAS_28[i],
                nakshatra_index: i as u8,
                pada,
                degrees_in_nakshatra: degrees_in,
            };
        }
    }

    // Should not reach here for valid [0, 360), but handle gracefully
    // by wrapping to last nakshatra.
    let last = bounds.len() - 1;
    Nakshatra28Info {
        nakshatra: ALL_NAKSHATRAS_28[last],
        nakshatra_index: last as u8,
        pada: 4,
        degrees_in_nakshatra: lon - bounds[last].0,
    }
}

/// Convenience: determine nakshatra from tropical longitude + ayanamsha (28-scheme).
pub fn nakshatra28_from_tropical(
    tropical_lon_deg: f64,
    system: AyanamshaSystem,
    jd_tdb: f64,
    use_nutation: bool,
) -> Nakshatra28Info {
    let t = jd_tdb_to_centuries(jd_tdb);
    let aya = ayanamsha_deg(system, t, use_nutation);
    nakshatra28_from_longitude(tropical_lon_deg - aya)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_nakshatras_27_count() {
        assert_eq!(ALL_NAKSHATRAS_27.len(), 27);
    }

    #[test]
    fn all_nakshatras_28_count() {
        assert_eq!(ALL_NAKSHATRAS_28.len(), 28);
    }

    #[test]
    fn nakshatra_27_indices_sequential() {
        for (i, n) in ALL_NAKSHATRAS_27.iter().enumerate() {
            assert_eq!(n.index() as usize, i);
        }
    }

    #[test]
    fn nakshatra_28_indices_sequential() {
        for (i, n) in ALL_NAKSHATRAS_28.iter().enumerate() {
            assert_eq!(n.index() as usize, i);
        }
    }

    #[test]
    fn nakshatra_27_names_nonempty() {
        for n in ALL_NAKSHATRAS_27 {
            assert!(!n.name().is_empty());
        }
    }

    #[test]
    fn nakshatra_28_names_nonempty() {
        for n in ALL_NAKSHATRAS_28 {
            assert!(!n.name().is_empty());
        }
    }

    #[test]
    fn nakshatra_span_correct() {
        assert!((NAKSHATRA_SPAN_27 - 13.333_333_333_333_334).abs() < 1e-10);
        assert!((PADA_SPAN - 3.333_333_333_333_333_5).abs() < 1e-10);
    }

    #[test]
    fn nakshatra_at_0() {
        let info = nakshatra_from_longitude(0.0);
        assert_eq!(info.nakshatra, Nakshatra::Ashwini);
        assert_eq!(info.nakshatra_index, 0);
        assert_eq!(info.pada, 1);
        assert!(info.degrees_in_nakshatra.abs() < 1e-10);
    }

    #[test]
    fn nakshatra_all_27_boundaries() {
        for i in 0..27u8 {
            let lon = i as f64 * NAKSHATRA_SPAN_27;
            let info = nakshatra_from_longitude(lon);
            assert_eq!(info.nakshatra_index, i, "boundary at nakshatra {i}");
            assert_eq!(info.pada, 1, "pada at boundary of nakshatra {i}");
        }
    }

    #[test]
    fn nakshatra_padas() {
        // Pada 1: 0 deg within nakshatra
        let info = nakshatra_from_longitude(0.0);
        assert_eq!(info.pada, 1);

        // Pada 2: starts at 3.333 deg within nakshatra
        let info = nakshatra_from_longitude(PADA_SPAN + 0.1);
        assert_eq!(info.pada, 2);

        // Pada 3: starts at 6.667 deg
        let info = nakshatra_from_longitude(2.0 * PADA_SPAN + 0.1);
        assert_eq!(info.pada, 3);

        // Pada 4: starts at 10.0 deg
        let info = nakshatra_from_longitude(3.0 * PADA_SPAN + 0.1);
        assert_eq!(info.pada, 4);
    }

    #[test]
    fn nakshatra_wrap() {
        let info = nakshatra_from_longitude(361.0);
        assert_eq!(info.nakshatra, Nakshatra::Ashwini);
        assert!((info.degrees_in_nakshatra - 1.0).abs() < 1e-10);
    }

    #[test]
    fn nakshatra_negative() {
        let info = nakshatra_from_longitude(-1.0);
        // -1 -> 359 deg → last nakshatra (Revati starts at 346.667)
        assert_eq!(info.nakshatra, Nakshatra::Revati);
    }

    #[test]
    fn nakshatra_last() {
        let info = nakshatra_from_longitude(350.0);
        assert_eq!(info.nakshatra, Nakshatra::Revati);
        assert_eq!(info.nakshatra_index, 26);
    }

    #[test]
    fn nakshatra_mula() {
        // Mula is index 18, starts at 18*13.333 = 240 deg
        let info = nakshatra_from_longitude(245.0);
        assert_eq!(info.nakshatra, Nakshatra::Mula);
        assert_eq!(info.nakshatra_index, 18);
    }

    // --- 28-scheme tests ---

    #[test]
    fn nakshatra28_boundaries_cover_360() {
        let bounds = nakshatra_28_boundaries();
        assert!((bounds[0].0 - 0.0).abs() < 1e-10, "starts at 0");
        assert!((bounds[27].1 - 360.0).abs() < 1e-10, "ends at 360");
        for i in 1..28 {
            assert!(
                (bounds[i].0 - bounds[i - 1].1).abs() < 1e-10,
                "gap between nakshatra {i} and {}",
                i - 1
            );
        }
    }

    #[test]
    fn nakshatra28_abhijit_region() {
        // Middle of Abhijit: ~278.5 deg
        let info = nakshatra28_from_longitude(278.5);
        assert_eq!(info.nakshatra, Nakshatra28::Abhijit);
        assert_eq!(info.nakshatra_index, 21);
        assert_eq!(info.pada, 0); // Abhijit has no pada
    }

    #[test]
    fn nakshatra28_before_abhijit() {
        // Just inside Uttara Ashadha (28-scheme): 270 deg
        let info = nakshatra28_from_longitude(270.0);
        assert_eq!(info.nakshatra, Nakshatra28::UttaraAshadha);
        assert_eq!(info.nakshatra_index, 20);
    }

    #[test]
    fn nakshatra28_after_abhijit() {
        // Just after Abhijit: 281 deg → Shravana in 28-scheme
        let info = nakshatra28_from_longitude(281.0);
        assert_eq!(info.nakshatra, Nakshatra28::Shravana);
        assert_eq!(info.nakshatra_index, 22);
    }

    #[test]
    fn nakshatra28_at_0() {
        let info = nakshatra28_from_longitude(0.0);
        assert_eq!(info.nakshatra, Nakshatra28::Ashwini);
        assert_eq!(info.nakshatra_index, 0);
        assert_eq!(info.pada, 1);
    }

    #[test]
    fn nakshatra28_last() {
        let info = nakshatra28_from_longitude(350.0);
        assert_eq!(info.nakshatra, Nakshatra28::Revati);
        assert_eq!(info.nakshatra_index, 27);
    }

    #[test]
    fn nakshatra28_abhijit_span() {
        let span = ABHIJIT_END - ABHIJIT_START;
        // ~4.222 deg (4 deg 13' 20")
        assert!((span - (4.0 + 13.0 / 60.0 + 20.0 / 3600.0)).abs() < 1e-10);
    }

    #[test]
    fn nakshatra_from_tropical_lahiri() {
        // Tropical ~280.5, Lahiri ~23.853 → sidereal ~256.65 → Mula (index 18 + some)
        let info = nakshatra_from_tropical(280.5, AyanamshaSystem::Lahiri, 2_451_545.0, false);
        // 256.65 / 13.333 = 19.2 → index 19 = Purva Ashadha
        assert_eq!(info.nakshatra, Nakshatra::PurvaAshadha);
    }
}
