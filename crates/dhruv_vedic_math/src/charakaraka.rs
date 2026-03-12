//! Chara Karaka (variable significator) computation.
//!
//! Clean-room implementation from public Jyotisha definitions:
//! - Planet ranking uses degrees within sign (0..30).
//! - Rahu uses reversed progression (30 - deg-in-sign) when included.
//! - Scheme variants model commonly used 7/8-karaka approaches.

use crate::graha::{Graha, SAPTA_GRAHAS};
use crate::normalize_360;

/// Chara Karaka scheme selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum CharakarakaScheme {
    /// 8 chara karakas (includes Rahu).
    #[default]
    Eight = 0,
    /// 7 chara karakas without Pitri karaka.
    SevenNoPitri = 1,
    /// 7 chara karakas with Putra merged into Matri.
    SevenPkMergedMk = 2,
    /// Mixed 7/8 of Parashara.
    ///
    /// Uses 8-karaka scheme when two classical planets share the same
    /// integer degree within sign; otherwise uses the 7 (PK merged with MK)
    /// scheme.
    MixedParashara = 3,
}

impl CharakarakaScheme {
    /// Convert a numeric code to a scheme.
    pub const fn from_u8(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Eight),
            1 => Some(Self::SevenNoPitri),
            2 => Some(Self::SevenPkMergedMk),
            3 => Some(Self::MixedParashara),
            _ => None,
        }
    }
}

/// Chara Karaka role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CharakarakaRole {
    Atma = 0,
    Amatya = 1,
    Bhratri = 2,
    Matri = 3,
    Pitri = 4,
    Putra = 5,
    Gnati = 6,
    Dara = 7,
    /// Combined Matri/Putra role in 7-karaka merged scheme.
    MatriPutra = 8,
}

impl CharakarakaRole {
    /// Convert role to stable numeric code.
    pub const fn code(self) -> u8 {
        self as u8
    }
}

const ROLES_8: [CharakarakaRole; 8] = [
    CharakarakaRole::Atma,
    CharakarakaRole::Amatya,
    CharakarakaRole::Bhratri,
    CharakarakaRole::Matri,
    CharakarakaRole::Pitri,
    CharakarakaRole::Putra,
    CharakarakaRole::Gnati,
    CharakarakaRole::Dara,
];

const ROLES_7_NO_PITRI: [CharakarakaRole; 7] = [
    CharakarakaRole::Atma,
    CharakarakaRole::Amatya,
    CharakarakaRole::Bhratri,
    CharakarakaRole::Matri,
    CharakarakaRole::Putra,
    CharakarakaRole::Gnati,
    CharakarakaRole::Dara,
];

const ROLES_7_MERGED: [CharakarakaRole; 7] = [
    CharakarakaRole::Atma,
    CharakarakaRole::Amatya,
    CharakarakaRole::Bhratri,
    CharakarakaRole::MatriPutra,
    CharakarakaRole::Pitri,
    CharakarakaRole::Gnati,
    CharakarakaRole::Dara,
];

#[derive(Debug, Clone, Copy)]
struct Candidate {
    graha: Graha,
    longitude_deg: f64,
    degrees_in_rashi: f64,
    effective_deg: f64,
}

/// Single Chara Karaka assignment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CharakarakaEntry {
    /// Assigned karaka role.
    pub role: CharakarakaRole,
    /// Assigned graha.
    pub graha: Graha,
    /// 1-based rank by effective degree within sign.
    pub rank: u8,
    /// Full sidereal longitude in [0,360).
    pub longitude_deg: f64,
    /// Degree within rashi [0,30).
    pub degrees_in_rashi: f64,
    /// Ranking degree (Rahu-reversed where applicable).
    pub effective_degrees_in_rashi: f64,
}

/// Chara Karaka output.
#[derive(Debug, Clone, PartialEq)]
pub struct CharakarakaResult {
    /// Requested scheme.
    pub scheme: CharakarakaScheme,
    /// Effective 8-karaka mode used (for mixed scheme).
    pub used_eight_karakas: bool,
    /// Ordered assignments.
    pub entries: Vec<CharakarakaEntry>,
}

fn degrees_in_rashi(lon: f64) -> f64 {
    normalize_360(lon).rem_euclid(30.0)
}

fn classical_same_integer_degree(longitudes: &[f64; 9]) -> bool {
    let mut bins = [0u8; 30];
    for graha in SAPTA_GRAHAS {
        let d = degrees_in_rashi(longitudes[graha.index() as usize]).floor() as usize;
        bins[d] = bins[d].saturating_add(1);
        if bins[d] >= 2 {
            return true;
        }
    }
    false
}

fn candidates_for_scheme(longitudes: &[f64; 9], include_rahu: bool) -> Vec<Candidate> {
    let mut out = Vec::with_capacity(if include_rahu { 8 } else { 7 });
    for graha in SAPTA_GRAHAS {
        let lon = normalize_360(longitudes[graha.index() as usize]);
        let deg = degrees_in_rashi(lon);
        out.push(Candidate {
            graha,
            longitude_deg: lon,
            degrees_in_rashi: deg,
            effective_deg: deg,
        });
    }
    if include_rahu {
        let lon = normalize_360(longitudes[Graha::Rahu.index() as usize]);
        let deg = degrees_in_rashi(lon);
        out.push(Candidate {
            graha: Graha::Rahu,
            longitude_deg: lon,
            degrees_in_rashi: deg,
            effective_deg: 30.0 - deg,
        });
    }
    out.sort_by(|a, b| {
        b.effective_deg
            .total_cmp(&a.effective_deg)
            .then_with(|| b.degrees_in_rashi.total_cmp(&a.degrees_in_rashi))
            .then_with(|| a.graha.index().cmp(&b.graha.index()))
    });
    out
}

/// Compute Chara Karakas from sidereal longitudes of all 9 grahas.
///
/// `longitudes` are indexed by `Graha::index()` (Sun..Ketu).
pub fn charakarakas_from_longitudes(
    longitudes: &[f64; 9],
    scheme: CharakarakaScheme,
) -> CharakarakaResult {
    let (roles, candidates, used_eight) = match scheme {
        CharakarakaScheme::Eight => (
            ROLES_8.as_slice(),
            candidates_for_scheme(longitudes, true),
            true,
        ),
        CharakarakaScheme::SevenNoPitri => (
            ROLES_7_NO_PITRI.as_slice(),
            candidates_for_scheme(longitudes, false),
            false,
        ),
        CharakarakaScheme::SevenPkMergedMk => (
            ROLES_7_MERGED.as_slice(),
            candidates_for_scheme(longitudes, false),
            false,
        ),
        CharakarakaScheme::MixedParashara => {
            let use_eight = classical_same_integer_degree(longitudes);
            if use_eight {
                (
                    ROLES_8.as_slice(),
                    candidates_for_scheme(longitudes, true),
                    true,
                )
            } else {
                (
                    ROLES_7_MERGED.as_slice(),
                    candidates_for_scheme(longitudes, false),
                    false,
                )
            }
        }
    };

    let mut entries = Vec::with_capacity(roles.len());
    for (i, (&role, c)) in roles.iter().zip(candidates.iter()).enumerate() {
        entries.push(CharakarakaEntry {
            role,
            graha: c.graha,
            rank: (i + 1) as u8,
            longitude_deg: c.longitude_deg,
            degrees_in_rashi: c.degrees_in_rashi,
            effective_degrees_in_rashi: c.effective_deg,
        });
    }

    CharakarakaResult {
        scheme,
        used_eight_karakas: used_eight,
        entries,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_lons() -> [f64; 9] {
        // Sun..Saturn are strictly descending in deg-in-sign.
        // Rahu is very high raw degree (29), but reversed to 1.
        [29.0, 28.0, 27.0, 26.0, 25.0, 24.0, 23.0, 29.0, 15.0]
    }

    #[test]
    fn eight_scheme_includes_rahu_and_pitri() {
        let out = charakarakas_from_longitudes(&sample_lons(), CharakarakaScheme::Eight);
        assert_eq!(out.entries.len(), 8);
        assert!(out.entries.iter().any(|e| e.graha == Graha::Rahu));
        assert!(out.entries.iter().any(|e| e.role == CharakarakaRole::Pitri));
    }

    #[test]
    fn seven_no_pitri_has_no_pitri_role() {
        let out = charakarakas_from_longitudes(&sample_lons(), CharakarakaScheme::SevenNoPitri);
        assert_eq!(out.entries.len(), 7);
        assert!(!out.entries.iter().any(|e| e.role == CharakarakaRole::Pitri));
        assert!(!out.entries.iter().any(|e| e.graha == Graha::Rahu));
    }

    #[test]
    fn seven_merged_uses_matri_putra_role() {
        let out = charakarakas_from_longitudes(&sample_lons(), CharakarakaScheme::SevenPkMergedMk);
        assert_eq!(out.entries.len(), 7);
        assert!(
            out.entries
                .iter()
                .any(|e| e.role == CharakarakaRole::MatriPutra)
        );
        assert!(!out.entries.iter().any(|e| e.role == CharakarakaRole::Putra));
    }

    #[test]
    fn mixed_defaults_to_seven_when_no_integer_tie() {
        let out = charakarakas_from_longitudes(&sample_lons(), CharakarakaScheme::MixedParashara);
        assert!(!out.used_eight_karakas);
        assert_eq!(out.entries.len(), 7);
    }

    #[test]
    fn mixed_switches_to_eight_on_integer_degree_tie() {
        let mut lons = sample_lons();
        // Tie on integer degree within sign among classical planets.
        lons[Graha::Chandra.index() as usize] = 29.7;
        lons[Graha::Mangal.index() as usize] = 29.2;
        let out = charakarakas_from_longitudes(&lons, CharakarakaScheme::MixedParashara);
        assert!(out.used_eight_karakas);
        assert_eq!(out.entries.len(), 8);
    }

    #[test]
    fn rahu_is_ranked_using_reverse_degree() {
        let out = charakarakas_from_longitudes(&sample_lons(), CharakarakaScheme::Eight);
        let rahu = out
            .entries
            .iter()
            .find(|e| e.graha == Graha::Rahu)
            .expect("rahu entry");
        assert!((rahu.degrees_in_rashi - 29.0).abs() < 1e-9);
        assert!((rahu.effective_degrees_in_rashi - 1.0).abs() < 1e-9);
    }
}
