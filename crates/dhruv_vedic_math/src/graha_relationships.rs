//! Graha relationship, dignity, and classification system.
//!
//! Provides exaltation/debilitation data, moolatrikone ranges, own-sign
//! ownership, natural (naisargika) friendship, temporal (tatkalika) friendship,
//! compound (panchadha) friendship, full dignity determination, node dignity
//! policy, benefic/malefic classification, gender, and lord mappers.
//!
//! Clean-room implementation from BPHS (Brihat Parashara Hora Shastra).

use crate::graha::{Graha, SAPTA_GRAHAS, rashi_lord_by_index};
use crate::hora::{Hora, hora_at};
use crate::masa::Masa;
use crate::samvatsara::Samvatsara;
use crate::vaar::Vaar;

// ---------------------------------------------------------------------------
// 1a. Exaltation & Debilitation
// ---------------------------------------------------------------------------

/// Exaltation degree (sidereal) for sapta grahas. Returns None for Rahu/Ketu.
///
/// BPHS: Sun 10 Ari=10, Moon 3 Tau=33, Mars 28 Cap=298,
/// Mercury 15 Vir=165, Jupiter 5 Can=95, Venus 27 Pis=357, Saturn 20 Lib=200.
pub const fn exaltation_degree(graha: Graha) -> Option<f64> {
    match graha {
        Graha::Surya => Some(10.0),   // 10 Aries
        Graha::Chandra => Some(33.0), // 3 Taurus
        Graha::Mangal => Some(298.0), // 28 Capricorn
        Graha::Buddh => Some(165.0),  // 15 Virgo
        Graha::Guru => Some(95.0),    // 5 Cancer
        Graha::Shukra => Some(357.0), // 27 Pisces
        Graha::Shani => Some(200.0),  // 20 Libra
        Graha::Rahu | Graha::Ketu => None,
    }
}

/// Debilitation degree = exaltation + 180 mod 360. Returns None for Rahu/Ketu.
pub const fn debilitation_degree(graha: Graha) -> Option<f64> {
    match exaltation_degree(graha) {
        Some(e) => {
            let d = e + 180.0;
            if d >= 360.0 { Some(d - 360.0) } else { Some(d) }
        }
        None => None,
    }
}

// ---------------------------------------------------------------------------
// 1a cont. Moolatrikone
// ---------------------------------------------------------------------------

/// Moolatrikone range: (rashi_index, start_deg_in_rashi, end_deg_in_rashi).
/// Returns None for Rahu/Ketu.
///
/// Sun 0-20 Leo, Moon 4-20 Tau, Mars 0-12 Ari, Mercury 16-20 Vir,
/// Jupiter 0-10 Sag, Venus 0-15 Lib, Saturn 0-20 Aqu.
pub const fn moolatrikone_range(graha: Graha) -> Option<(u8, f64, f64)> {
    match graha {
        Graha::Surya => Some((4, 0.0, 20.0)),   // Simha (Leo)
        Graha::Chandra => Some((1, 4.0, 20.0)), // Vrishabha (Taurus)
        Graha::Mangal => Some((0, 0.0, 12.0)),  // Mesha (Aries)
        Graha::Buddh => Some((5, 16.0, 20.0)),  // Kanya (Virgo)
        Graha::Guru => Some((8, 0.0, 10.0)),    // Dhanu (Sagittarius)
        Graha::Shukra => Some((6, 0.0, 15.0)),  // Tula (Libra)
        Graha::Shani => Some((10, 0.0, 20.0)),  // Kumbha (Aquarius)
        Graha::Rahu | Graha::Ketu => None,
    }
}

// ---------------------------------------------------------------------------
// 1a cont. Own Signs
// ---------------------------------------------------------------------------

/// Own-sign rashis for sapta grahas. Returns empty slice for Rahu/Ketu.
///
/// Sun [4], Moon [3], Mars [0,7], Mercury [2,5],
/// Jupiter [8,11], Venus [1,6], Saturn [9,10].
pub fn own_signs(graha: Graha) -> &'static [u8] {
    match graha {
        Graha::Surya => &[4],     // Simha
        Graha::Chandra => &[3],   // Karka
        Graha::Mangal => &[0, 7], // Mesha, Vrischika
        Graha::Buddh => &[2, 5],  // Mithuna, Kanya
        Graha::Guru => &[8, 11],  // Dhanu, Meena
        Graha::Shukra => &[1, 6], // Vrishabha, Tula
        Graha::Shani => &[9, 10], // Makara, Kumbha
        Graha::Rahu | Graha::Ketu => &[],
    }
}

// ---------------------------------------------------------------------------
// 1b. Natural Friendship (Naisargika Maitri)
// ---------------------------------------------------------------------------

/// Natural relationship between two grahas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NaisargikaMaitri {
    Friend,
    Enemy,
    Neutral,
}

/// Natural (naisargika) friendship between two sapta grahas (BPHS table).
/// Returns Neutral for any pairing involving Rahu/Ketu.
pub const fn naisargika_maitri(graha: Graha, other: Graha) -> NaisargikaMaitri {
    use Graha::*;
    use NaisargikaMaitri::*;

    // Rahu/Ketu: always Neutral
    match (graha, other) {
        (Rahu | Ketu, _) | (_, Rahu | Ketu) => return Neutral,
        _ => {}
    }

    match (graha, other) {
        // Sun: friends=Moon,Mars,Jupiter; enemies=Venus,Saturn; neutral=Mercury
        (Surya, Chandra | Mangal | Guru) => Friend,
        (Surya, Shukra | Shani) => Enemy,
        (Surya, Buddh) => Neutral,
        (Surya, Surya) => Neutral, // self

        // Moon: friends=Sun,Mercury; enemies=none; neutral=Mars,Jupiter,Venus,Saturn
        (Chandra, Surya | Buddh) => Friend,
        (Chandra, Mangal | Guru | Shukra | Shani) => Neutral,
        (Chandra, Chandra) => Neutral,

        // Mars: friends=Sun,Moon,Jupiter; enemies=Mercury; neutral=Venus,Saturn
        (Mangal, Surya | Chandra | Guru) => Friend,
        (Mangal, Buddh) => Enemy,
        (Mangal, Shukra | Shani) => Neutral,
        (Mangal, Mangal) => Neutral,

        // Mercury: friends=Sun,Venus; enemies=Moon; neutral=Mars,Jupiter,Saturn
        (Buddh, Surya | Shukra) => Friend,
        (Buddh, Chandra) => Enemy,
        (Buddh, Mangal | Guru | Shani) => Neutral,
        (Buddh, Buddh) => Neutral,

        // Jupiter: friends=Sun,Moon,Mars; enemies=Mercury,Venus; neutral=Saturn
        (Guru, Surya | Chandra | Mangal) => Friend,
        (Guru, Buddh | Shukra) => Enemy,
        (Guru, Shani) => Neutral,
        (Guru, Guru) => Neutral,

        // Venus: friends=Mercury,Saturn; enemies=Sun,Moon; neutral=Mars,Jupiter
        (Shukra, Buddh | Shani) => Friend,
        (Shukra, Surya | Chandra) => Enemy,
        (Shukra, Mangal | Guru) => Neutral,
        (Shukra, Shukra) => Neutral,

        // Saturn: friends=Mercury,Venus; enemies=Sun,Moon,Mars; neutral=Jupiter
        (Shani, Buddh | Shukra) => Friend,
        (Shani, Surya | Chandra | Mangal) => Enemy,
        (Shani, Guru) => Neutral,
        (Shani, Shani) => Neutral,

        // Exhaustive: already handled Rahu/Ketu above
        _ => Neutral,
    }
}

// ---------------------------------------------------------------------------
// 1c. Temporal Friendship (Tatkalika Maitri)
// ---------------------------------------------------------------------------

/// Temporal relationship based on current rashi positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TatkalikaMaitri {
    Friend,
    Enemy,
}

/// Temporal friendship: friend if other is in 2nd/3rd/4th/10th/11th/12th from graha.
pub fn tatkalika_maitri(graha_rashi_idx: u8, other_rashi_idx: u8) -> TatkalikaMaitri {
    // Distance = (other - graha + 12) % 12, gives 0-based house offset
    // House numbers 2,3,4,10,11,12 → offsets 1,2,3,9,10,11
    let dist = ((other_rashi_idx as i16 - graha_rashi_idx as i16 + 12) % 12) as u8;
    match dist {
        1 | 2 | 3 | 9 | 10 | 11 => TatkalikaMaitri::Friend,
        _ => TatkalikaMaitri::Enemy, // 0 (same sign), 4, 5, 6, 7, 8
    }
}

// ---------------------------------------------------------------------------
// 1d. Compound Friendship (Panchadha Maitri)
// ---------------------------------------------------------------------------

/// Five-fold compound relationship.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanchadhaMaitri {
    AdhiShatru,
    Shatru,
    Sama,
    Mitra,
    AdhiMitra,
}

/// Combine natural and temporal friendship into five-fold relationship.
pub fn panchadha_maitri(
    naisargika: NaisargikaMaitri,
    tatkalika: TatkalikaMaitri,
) -> PanchadhaMaitri {
    use NaisargikaMaitri as N;
    use PanchadhaMaitri as P;
    use TatkalikaMaitri as T;

    match (naisargika, tatkalika) {
        (N::Friend, T::Friend) => P::AdhiMitra,
        (N::Friend, T::Enemy) => P::Sama,
        (N::Neutral, T::Friend) => P::Mitra,
        (N::Neutral, T::Enemy) => P::Shatru,
        (N::Enemy, T::Friend) => P::Sama,
        (N::Enemy, T::Enemy) => P::AdhiShatru,
    }
}

// ---------------------------------------------------------------------------
// 1e. Dignity Determination (Sapta Grahas)
// ---------------------------------------------------------------------------

/// Dignity of a graha in a rashi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dignity {
    Exalted,
    Moolatrikone,
    OwnSign,
    AdhiMitra,
    Mitra,
    Sama,
    Shatru,
    AdhiShatru,
    Debilitated,
}

/// Check if sidereal longitude falls in the exaltation rashi.
fn is_exalted(graha: Graha, sidereal_lon: f64) -> bool {
    if let Some(exalt) = exaltation_degree(graha) {
        let exalt_rashi = (exalt / 30.0) as u8;
        let lon_rashi = (normalize_360_inner(sidereal_lon) / 30.0).floor() as u8;
        exalt_rashi == lon_rashi.min(11)
    } else {
        false
    }
}

/// Check if sidereal longitude falls in the debilitation rashi.
fn is_debilitated(graha: Graha, sidereal_lon: f64) -> bool {
    if let Some(deb) = debilitation_degree(graha) {
        let deb_rashi = (deb / 30.0) as u8;
        let lon_rashi = (normalize_360_inner(sidereal_lon) / 30.0).floor() as u8;
        deb_rashi == lon_rashi.min(11)
    } else {
        false
    }
}

/// Check if sidereal longitude falls in the moolatrikone range.
fn is_in_moolatrikone(graha: Graha, sidereal_lon: f64) -> bool {
    if let Some((mt_rashi, start, end)) = moolatrikone_range(graha) {
        let lon = normalize_360_inner(sidereal_lon);
        let lon_rashi = (lon / 30.0).floor() as u8;
        if lon_rashi.min(11) != mt_rashi {
            return false;
        }
        let deg_in_rashi = lon - (mt_rashi as f64) * 30.0;
        deg_in_rashi >= start && deg_in_rashi < end
    } else {
        false
    }
}

/// Check if rashi_index is an own sign for the graha.
fn is_own_sign(graha: Graha, rashi_index: u8) -> bool {
    own_signs(graha).contains(&rashi_index)
}

/// Naisargika-only dignity (no temporal context).
///
/// Priority: exaltation > debilitation > moolatrikone > own sign > naisargika friendship with rashi lord.
pub fn dignity_in_rashi(graha: Graha, sidereal_lon: f64, rashi_index: u8) -> Dignity {
    // Rahu/Ketu: always Sama in naisargika-only mode
    if matches!(graha, Graha::Rahu | Graha::Ketu) {
        return Dignity::Sama;
    }

    if is_exalted(graha, sidereal_lon) {
        return Dignity::Exalted;
    }
    if is_debilitated(graha, sidereal_lon) {
        return Dignity::Debilitated;
    }
    if is_in_moolatrikone(graha, sidereal_lon) {
        return Dignity::Moolatrikone;
    }
    if is_own_sign(graha, rashi_index) {
        return Dignity::OwnSign;
    }

    // Naisargika relationship with rashi lord
    let rashi_lord = match rashi_lord_by_index(rashi_index) {
        Some(lord) => lord,
        None => return Dignity::Sama,
    };
    if rashi_lord == graha {
        return Dignity::OwnSign;
    }

    match naisargika_maitri(graha, rashi_lord) {
        NaisargikaMaitri::Friend => Dignity::Mitra,
        NaisargikaMaitri::Enemy => Dignity::Shatru,
        NaisargikaMaitri::Neutral => Dignity::Sama,
    }
}

/// Full dignity with temporal (compound) friendship.
///
/// `all_rashi_indices` = 7 sapta graha rashi positions (indexed 0-6).
/// Priority: exaltation > debilitation > moolatrikone > own sign > compound friendship.
pub fn dignity_in_rashi_with_positions(
    graha: Graha,
    sidereal_lon: f64,
    rashi_index: u8,
    all_rashi_indices: &[u8; 7],
) -> Dignity {
    // Rahu/Ketu: use naisargika-only
    if matches!(graha, Graha::Rahu | Graha::Ketu) {
        return Dignity::Sama;
    }

    if is_exalted(graha, sidereal_lon) {
        return Dignity::Exalted;
    }
    if is_debilitated(graha, sidereal_lon) {
        return Dignity::Debilitated;
    }
    if is_in_moolatrikone(graha, sidereal_lon) {
        return Dignity::Moolatrikone;
    }
    if is_own_sign(graha, rashi_index) {
        return Dignity::OwnSign;
    }

    let rashi_lord = match rashi_lord_by_index(rashi_index) {
        Some(lord) => lord,
        None => return Dignity::Sama,
    };
    if rashi_lord == graha {
        return Dignity::OwnSign;
    }

    // For compound friendship, need naisargika + tatkalika
    let nais = naisargika_maitri(graha, rashi_lord);
    let graha_rashi = all_rashi_indices[graha.index() as usize];
    let lord_rashi = all_rashi_indices[rashi_lord.index() as usize];
    let tatk = tatkalika_maitri(graha_rashi, lord_rashi);
    let compound = panchadha_maitri(nais, tatk);

    match compound {
        PanchadhaMaitri::AdhiMitra => Dignity::AdhiMitra,
        PanchadhaMaitri::Mitra => Dignity::Mitra,
        PanchadhaMaitri::Sama => Dignity::Sama,
        PanchadhaMaitri::Shatru => Dignity::Shatru,
        PanchadhaMaitri::AdhiShatru => Dignity::AdhiShatru,
    }
}

// ---------------------------------------------------------------------------
// 1f. Node Dignity Policy (Rahu/Ketu)
// ---------------------------------------------------------------------------

/// Policy for determining Rahu/Ketu dignity (extension beyond strict BPHS).
///
/// BPHS does not define exaltation/friendship for nodes in a universally agreed way.
/// This is a configurable extension, isolated for auditability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NodeDignityPolicy {
    /// Sign-lord based: dignity = compound relationship between node's dispositor
    /// and target rashi lord. Temporal component uses node's rashi vs other positions.
    #[default]
    SignLordBased,
    /// Always Sama (neutral) — safest conservative choice.
    AlwaysSama,
}

/// Determine dignity for Rahu/Ketu using the selected policy.
///
/// Requires all 9 graha rashi indices for temporal context.
/// Returns Sama for non-node grahas (use `dignity_in_rashi_with_positions` instead).
pub fn node_dignity_in_rashi(
    graha: Graha,
    rashi_index: u8,
    all_rashi_indices_9: &[u8; 9],
    policy: NodeDignityPolicy,
) -> Dignity {
    // Only applies to Rahu/Ketu
    if !matches!(graha, Graha::Rahu | Graha::Ketu) {
        return Dignity::Sama;
    }

    match policy {
        NodeDignityPolicy::AlwaysSama => Dignity::Sama,
        NodeDignityPolicy::SignLordBased => {
            // Dispositor = lord of node's rashi
            let node_rashi = all_rashi_indices_9[graha.index() as usize];
            let dispositor = match rashi_lord_by_index(node_rashi) {
                Some(lord) => lord,
                None => return Dignity::Sama,
            };

            // Target rashi lord
            let target_lord = match rashi_lord_by_index(rashi_index) {
                Some(lord) => lord,
                None => return Dignity::Sama,
            };

            if dispositor == target_lord {
                return Dignity::OwnSign;
            }

            // Compound relationship between dispositor and target lord
            let nais = naisargika_maitri(dispositor, target_lord);
            let disp_rashi = all_rashi_indices_9[dispositor.index() as usize];
            let target_lord_rashi = all_rashi_indices_9[target_lord.index() as usize];
            let tatk = tatkalika_maitri(disp_rashi, target_lord_rashi);
            let compound = panchadha_maitri(nais, tatk);

            match compound {
                PanchadhaMaitri::AdhiMitra => Dignity::AdhiMitra,
                PanchadhaMaitri::Mitra => Dignity::Mitra,
                PanchadhaMaitri::Sama => Dignity::Sama,
                PanchadhaMaitri::Shatru => Dignity::Shatru,
                PanchadhaMaitri::AdhiShatru => Dignity::AdhiShatru,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 1g. Benefic/Malefic, Gender
// ---------------------------------------------------------------------------

/// Natural benefic/malefic classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeneficNature {
    Benefic,
    Malefic,
}

/// Natural benefic/malefic for each graha. Moon defaults Benefic.
pub const fn natural_benefic_malefic(graha: Graha) -> BeneficNature {
    match graha {
        // Benefics: Moon, Mercury, Jupiter, Venus
        Graha::Chandra | Graha::Buddh | Graha::Guru | Graha::Shukra => BeneficNature::Benefic,
        // Malefics: Sun, Mars, Saturn, Rahu, Ketu
        Graha::Surya | Graha::Mangal | Graha::Shani | Graha::Rahu | Graha::Ketu => {
            BeneficNature::Malefic
        }
    }
}

/// Moon's benefic nature depends on elongation from Sun.
/// Benefic if shukla paksha (elongation 0-180 from new moon perspective,
/// i.e., if distance from Sun > 72 degrees on either side of full moon).
/// Simplification: benefic if elongation (0-360) makes phase_angle > 72.
pub fn moon_benefic_nature(moon_sun_elongation: f64) -> BeneficNature {
    let elong = normalize_360_inner(moon_sun_elongation);
    // phase_angle = proximity to full moon (180 deg elongation)
    // benefic when "bright" enough, typically > 72 deg phase
    let phase = if elong <= 180.0 { elong } else { 360.0 - elong };
    if phase >= 72.0 {
        BeneficNature::Benefic
    } else {
        BeneficNature::Malefic
    }
}

/// Graha gender classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrahaGender {
    Male,
    Female,
    Neuter,
}

/// Gender of each graha per BPHS.
/// Male: Sun, Mars, Jupiter. Female: Moon, Venus. Neuter: Mercury, Saturn.
/// Rahu/Ketu treated as Neuter (no BPHS gender assignment).
pub const fn graha_gender(graha: Graha) -> GrahaGender {
    match graha {
        Graha::Surya | Graha::Mangal | Graha::Guru => GrahaGender::Male,
        Graha::Chandra | Graha::Shukra => GrahaGender::Female,
        Graha::Buddh | Graha::Shani | Graha::Rahu | Graha::Ketu => GrahaGender::Neuter,
    }
}

// ---------------------------------------------------------------------------
// 1h. Lord Mapper Functions
// ---------------------------------------------------------------------------

/// Weekday lord: maps Vaar to Graha.
pub const fn vaar_lord(vaar: Vaar) -> Graha {
    match vaar {
        Vaar::Ravivaar => Graha::Surya,
        Vaar::Somvaar => Graha::Chandra,
        Vaar::Mangalvaar => Graha::Mangal,
        Vaar::Budhvaar => Graha::Buddh,
        Vaar::Guruvaar => Graha::Guru,
        Vaar::Shukravaar => Graha::Shukra,
        Vaar::Shanivaar => Graha::Shani,
    }
}

/// Hora lord at hora_index within weekday. Maps Hora enum to Graha.
pub fn hora_lord(vaar: Vaar, hora_index: u8) -> Graha {
    let hora = hora_at(vaar, hora_index);
    hora_to_graha(hora)
}

/// Map Hora enum to Graha.
const fn hora_to_graha(hora: Hora) -> Graha {
    match hora {
        Hora::Surya => Graha::Surya,
        Hora::Shukra => Graha::Shukra,
        Hora::Buddh => Graha::Buddh,
        Hora::Chandra => Graha::Chandra,
        Hora::Shani => Graha::Shani,
        Hora::Guru => Graha::Guru,
        Hora::Mangal => Graha::Mangal,
    }
}

/// Masa lord = rashi lord of corresponding rashi.
/// Chaitra (0) -> Mesha (0) -> Mars, Vaishakha (1) -> Vrishabha (1) -> Venus, etc.
pub fn masa_lord(masa: Masa) -> Graha {
    rashi_lord_by_index(masa.index()).unwrap_or(Graha::Surya)
}

/// Samvatsara lord. BPHS 60-year cycle: samvatsara.index() % 7 -> sapta graha.
pub fn samvatsara_lord(samvatsara: Samvatsara) -> Graha {
    SAPTA_GRAHAS[(samvatsara.index() as usize) % 7]
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn normalize_360_inner(deg: f64) -> f64 {
    let r = deg % 360.0;
    if r < 0.0 { r + 360.0 } else { r }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graha::ALL_GRAHAS;

    // --- Exaltation/Debilitation ---

    #[test]
    fn exaltation_sun() {
        assert_eq!(exaltation_degree(Graha::Surya), Some(10.0));
    }

    #[test]
    fn debilitation_sun() {
        assert_eq!(debilitation_degree(Graha::Surya), Some(190.0));
    }

    #[test]
    fn exaltation_none_for_rahu() {
        assert_eq!(exaltation_degree(Graha::Rahu), None);
    }

    #[test]
    fn exaltation_none_for_ketu() {
        assert_eq!(exaltation_degree(Graha::Ketu), None);
    }

    #[test]
    fn debilitation_opposite_exaltation() {
        for g in SAPTA_GRAHAS {
            if let (Some(e), Some(d)) = (exaltation_degree(g), debilitation_degree(g)) {
                let diff = (e - d).abs();
                let diff_mod = if diff > 180.0 { 360.0 - diff } else { diff };
                assert!(
                    (diff_mod - 180.0).abs() < 1e-10,
                    "{:?}: exalt={e}, debil={d}",
                    g
                );
            }
        }
    }

    // --- Moolatrikone ---

    #[test]
    fn moolatrikone_sun_in_leo() {
        let (rashi, start, end) = moolatrikone_range(Graha::Surya).unwrap();
        assert_eq!(rashi, 4); // Simha
        assert!((start - 0.0).abs() < 1e-10);
        assert!((end - 20.0).abs() < 1e-10);
    }

    #[test]
    fn moolatrikone_none_rahu() {
        assert!(moolatrikone_range(Graha::Rahu).is_none());
    }

    // --- Own Signs ---

    #[test]
    fn own_signs_mars() {
        let signs = own_signs(Graha::Mangal);
        assert_eq!(signs, &[0, 7]); // Mesha, Vrischika
    }

    #[test]
    fn own_signs_empty_for_rahu() {
        assert!(own_signs(Graha::Rahu).is_empty());
    }

    // --- Naisargika Maitri (49 cells) ---

    #[test]
    fn naisargika_sun_moon_friend() {
        assert_eq!(
            naisargika_maitri(Graha::Surya, Graha::Chandra),
            NaisargikaMaitri::Friend
        );
    }

    #[test]
    fn naisargika_sun_venus_enemy() {
        assert_eq!(
            naisargika_maitri(Graha::Surya, Graha::Shukra),
            NaisargikaMaitri::Enemy
        );
    }

    #[test]
    fn naisargika_sun_mercury_neutral() {
        assert_eq!(
            naisargika_maitri(Graha::Surya, Graha::Buddh),
            NaisargikaMaitri::Neutral
        );
    }

    #[test]
    fn naisargika_moon_no_enemies() {
        for g in SAPTA_GRAHAS {
            if g == Graha::Chandra {
                continue;
            }
            assert_ne!(
                naisargika_maitri(Graha::Chandra, g),
                NaisargikaMaitri::Enemy,
                "Moon should have no enemies, but {:?} is enemy",
                g
            );
        }
    }

    #[test]
    fn naisargika_mars_friends() {
        assert_eq!(
            naisargika_maitri(Graha::Mangal, Graha::Surya),
            NaisargikaMaitri::Friend
        );
        assert_eq!(
            naisargika_maitri(Graha::Mangal, Graha::Chandra),
            NaisargikaMaitri::Friend
        );
        assert_eq!(
            naisargika_maitri(Graha::Mangal, Graha::Guru),
            NaisargikaMaitri::Friend
        );
    }

    #[test]
    fn naisargika_mars_enemy_mercury() {
        assert_eq!(
            naisargika_maitri(Graha::Mangal, Graha::Buddh),
            NaisargikaMaitri::Enemy
        );
    }

    #[test]
    fn naisargika_mercury_friends() {
        assert_eq!(
            naisargika_maitri(Graha::Buddh, Graha::Surya),
            NaisargikaMaitri::Friend
        );
        assert_eq!(
            naisargika_maitri(Graha::Buddh, Graha::Shukra),
            NaisargikaMaitri::Friend
        );
    }

    #[test]
    fn naisargika_jupiter_enemies() {
        assert_eq!(
            naisargika_maitri(Graha::Guru, Graha::Buddh),
            NaisargikaMaitri::Enemy
        );
        assert_eq!(
            naisargika_maitri(Graha::Guru, Graha::Shukra),
            NaisargikaMaitri::Enemy
        );
    }

    #[test]
    fn naisargika_venus_friends() {
        assert_eq!(
            naisargika_maitri(Graha::Shukra, Graha::Buddh),
            NaisargikaMaitri::Friend
        );
        assert_eq!(
            naisargika_maitri(Graha::Shukra, Graha::Shani),
            NaisargikaMaitri::Friend
        );
    }

    #[test]
    fn naisargika_saturn_full() {
        assert_eq!(
            naisargika_maitri(Graha::Shani, Graha::Buddh),
            NaisargikaMaitri::Friend
        );
        assert_eq!(
            naisargika_maitri(Graha::Shani, Graha::Shukra),
            NaisargikaMaitri::Friend
        );
        assert_eq!(
            naisargika_maitri(Graha::Shani, Graha::Surya),
            NaisargikaMaitri::Enemy
        );
        assert_eq!(
            naisargika_maitri(Graha::Shani, Graha::Chandra),
            NaisargikaMaitri::Enemy
        );
        assert_eq!(
            naisargika_maitri(Graha::Shani, Graha::Mangal),
            NaisargikaMaitri::Enemy
        );
        assert_eq!(
            naisargika_maitri(Graha::Shani, Graha::Guru),
            NaisargikaMaitri::Neutral
        );
    }

    #[test]
    fn naisargika_rahu_always_neutral() {
        for g in ALL_GRAHAS {
            assert_eq!(naisargika_maitri(Graha::Rahu, g), NaisargikaMaitri::Neutral);
        }
    }

    #[test]
    fn naisargika_all_49_cells_valid() {
        for a in SAPTA_GRAHAS {
            for b in SAPTA_GRAHAS {
                let _ = naisargika_maitri(a, b); // should not panic
            }
        }
    }

    // --- Tatkalika Maitri ---

    #[test]
    fn tatkalika_friend_offsets() {
        // offsets 1,2,3,9,10,11 = friend
        for offset in [1u8, 2, 3, 9, 10, 11] {
            assert_eq!(
                tatkalika_maitri(0, offset),
                TatkalikaMaitri::Friend,
                "offset {offset} should be friend"
            );
        }
    }

    #[test]
    fn tatkalika_enemy_offsets() {
        // offsets 0,4,5,6,7,8 = enemy
        for offset in [0u8, 4, 5, 6, 7, 8] {
            assert_eq!(
                tatkalika_maitri(0, offset),
                TatkalikaMaitri::Enemy,
                "offset {offset} should be enemy"
            );
        }
    }

    #[test]
    fn tatkalika_wraps() {
        // rashi 11 to rashi 0: distance = 1 (2nd house) = friend
        assert_eq!(tatkalika_maitri(11, 0), TatkalikaMaitri::Friend);
    }

    // --- Panchadha Maitri ---

    #[test]
    fn panchadha_all_six_combinations() {
        use NaisargikaMaitri as N;
        use PanchadhaMaitri as P;
        use TatkalikaMaitri as T;

        assert_eq!(panchadha_maitri(N::Friend, T::Friend), P::AdhiMitra);
        assert_eq!(panchadha_maitri(N::Friend, T::Enemy), P::Sama);
        assert_eq!(panchadha_maitri(N::Neutral, T::Friend), P::Mitra);
        assert_eq!(panchadha_maitri(N::Neutral, T::Enemy), P::Shatru);
        assert_eq!(panchadha_maitri(N::Enemy, T::Friend), P::Sama);
        assert_eq!(panchadha_maitri(N::Enemy, T::Enemy), P::AdhiShatru);
    }

    // --- Dignity ---

    #[test]
    fn dignity_sun_exalted_in_aries() {
        // Sun at 10 Aries
        let d = dignity_in_rashi(Graha::Surya, 10.0, 0);
        assert_eq!(d, Dignity::Exalted);
    }

    #[test]
    fn dignity_sun_debilitated_in_libra() {
        // Sun at 190 deg (10 Libra)
        let d = dignity_in_rashi(Graha::Surya, 190.0, 6);
        assert_eq!(d, Dignity::Debilitated);
    }

    #[test]
    fn dignity_sun_moolatrikone_in_leo() {
        // Sun at 130 deg = 10 Simha, within 0-20 Leo
        let d = dignity_in_rashi(Graha::Surya, 130.0, 4);
        assert_eq!(d, Dignity::Moolatrikone);
    }

    #[test]
    fn dignity_sun_own_sign_in_leo_after_mt() {
        // Sun at 145 deg = 25 Leo, past moolatrikone range
        let d = dignity_in_rashi(Graha::Surya, 145.0, 4);
        assert_eq!(d, Dignity::OwnSign);
    }

    #[test]
    fn dignity_mars_own_sign_vrischika() {
        // Mars at 220 deg = 10 Vrischika
        let d = dignity_in_rashi(Graha::Mangal, 220.0, 7);
        assert_eq!(d, Dignity::OwnSign);
    }

    #[test]
    fn dignity_with_positions_compound() {
        // Sun in Vrishabha (1), lord=Venus. Naisargika: Sun-Venus=Enemy.
        // If tatkalika makes it Friend, compound = Sama.
        // Sun rashi=1, Venus rashi=2 → dist=1 → tatkalika friend
        let positions: [u8; 7] = [1, 0, 0, 0, 0, 2, 0]; // Sun=1, Venus=2
        let d = dignity_in_rashi_with_positions(Graha::Surya, 45.0, 1, &positions);
        assert_eq!(d, Dignity::Sama); // Enemy+Friend = Sama
    }

    #[test]
    fn dignity_rahu_always_sama_naisargika() {
        let d = dignity_in_rashi(Graha::Rahu, 100.0, 3);
        assert_eq!(d, Dignity::Sama);
    }

    // --- Node Dignity ---

    #[test]
    fn node_dignity_always_sama_policy() {
        let indices: [u8; 9] = [0, 1, 2, 3, 4, 5, 6, 7, 8];
        let d = node_dignity_in_rashi(Graha::Rahu, 3, &indices, NodeDignityPolicy::AlwaysSama);
        assert_eq!(d, Dignity::Sama);
    }

    #[test]
    fn node_dignity_sign_lord_based() {
        // Rahu in Mesha (0), dispositor=Mars. Target rashi=0 (Mesha), target lord=Mars.
        // Dispositor == target lord → OwnSign.
        let indices: [u8; 9] = [0, 0, 0, 0, 0, 0, 0, 0, 0]; // Rahu in idx 7=0
        let d = node_dignity_in_rashi(Graha::Rahu, 0, &indices, NodeDignityPolicy::SignLordBased);
        assert_eq!(d, Dignity::OwnSign);
    }

    #[test]
    fn node_dignity_non_node_returns_sama() {
        let indices: [u8; 9] = [0; 9];
        let d = node_dignity_in_rashi(Graha::Surya, 0, &indices, NodeDignityPolicy::SignLordBased);
        assert_eq!(d, Dignity::Sama);
    }

    // --- Benefic/Malefic ---

    #[test]
    fn natural_benefic() {
        assert_eq!(natural_benefic_malefic(Graha::Guru), BeneficNature::Benefic);
        assert_eq!(
            natural_benefic_malefic(Graha::Shukra),
            BeneficNature::Benefic
        );
    }

    #[test]
    fn natural_malefic() {
        assert_eq!(
            natural_benefic_malefic(Graha::Surya),
            BeneficNature::Malefic
        );
        assert_eq!(
            natural_benefic_malefic(Graha::Mangal),
            BeneficNature::Malefic
        );
        assert_eq!(
            natural_benefic_malefic(Graha::Shani),
            BeneficNature::Malefic
        );
    }

    #[test]
    fn moon_benefic_at_full() {
        // Full moon: elongation ~180 → phase_angle=180 > 72 → Benefic
        assert_eq!(moon_benefic_nature(180.0), BeneficNature::Benefic);
    }

    #[test]
    fn moon_malefic_near_new() {
        // Near new moon: elongation ~10 → phase_angle=10 < 72 → Malefic
        assert_eq!(moon_benefic_nature(10.0), BeneficNature::Malefic);
    }

    // --- Gender ---

    #[test]
    fn gender_male() {
        assert_eq!(graha_gender(Graha::Surya), GrahaGender::Male);
        assert_eq!(graha_gender(Graha::Mangal), GrahaGender::Male);
        assert_eq!(graha_gender(Graha::Guru), GrahaGender::Male);
    }

    #[test]
    fn gender_female() {
        assert_eq!(graha_gender(Graha::Chandra), GrahaGender::Female);
        assert_eq!(graha_gender(Graha::Shukra), GrahaGender::Female);
    }

    #[test]
    fn gender_neuter() {
        assert_eq!(graha_gender(Graha::Buddh), GrahaGender::Neuter);
        assert_eq!(graha_gender(Graha::Shani), GrahaGender::Neuter);
    }

    // --- Lord Mappers ---

    #[test]
    fn vaar_lord_ravivaar_is_surya() {
        assert_eq!(vaar_lord(Vaar::Ravivaar), Graha::Surya);
    }

    #[test]
    fn vaar_lord_somvaar_is_chandra() {
        assert_eq!(vaar_lord(Vaar::Somvaar), Graha::Chandra);
    }

    #[test]
    fn vaar_lord_all_seven() {
        let expected = [
            Graha::Surya,
            Graha::Chandra,
            Graha::Mangal,
            Graha::Buddh,
            Graha::Guru,
            Graha::Shukra,
            Graha::Shani,
        ];
        for (v, e) in crate::vaar::ALL_VAARS.iter().zip(expected.iter()) {
            assert_eq!(vaar_lord(*v), *e, "vaar {:?}", v);
        }
    }

    #[test]
    fn hora_lord_sunday_first() {
        assert_eq!(hora_lord(Vaar::Ravivaar, 0), Graha::Surya);
    }

    #[test]
    fn hora_lord_sunday_second() {
        assert_eq!(hora_lord(Vaar::Ravivaar, 1), Graha::Shukra);
    }

    #[test]
    fn masa_lord_chaitra_is_mangal() {
        assert_eq!(masa_lord(Masa::Chaitra), Graha::Mangal);
    }

    #[test]
    fn masa_lord_karka_is_chandra() {
        assert_eq!(masa_lord(Masa::Ashadha), Graha::Chandra);
    }

    #[test]
    fn samvatsara_lord_prabhava() {
        // index 0 % 7 = 0 → Surya
        assert_eq!(samvatsara_lord(Samvatsara::Prabhava), Graha::Surya);
    }

    #[test]
    fn samvatsara_lord_vibhava() {
        // index 1 % 7 = 1 → Chandra
        assert_eq!(samvatsara_lord(Samvatsara::Vibhava), Graha::Chandra);
    }

    #[test]
    fn samvatsara_lord_wraps() {
        // index 7 % 7 = 0 → Surya
        assert_eq!(samvatsara_lord(Samvatsara::Bhava), Graha::Surya);
    }
}
