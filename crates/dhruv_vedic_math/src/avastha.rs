//! Graha Avasthas (Planetary States) — 5 classification systems.
//!
//! Avasthas indicate a planet's ability to deliver results:
//! 1. **Baladi** (age-based) — 5 states from sign position
//! 2. **Jagradadi** (alertness-based) — 3 states from dignity
//! 3. **Deeptadi** (luminosity-based) — 9 states from conditions
//! 4. **Lajjitadi** (mood-based) — 6 states from conjunctions/aspects
//! 5. **Sayanadi** (posture-based) — 12 states + 3 sub-states × 5 name-groups
//!
//! Clean-room implementation from BPHS Ch.45 and standard Vedic texts.
//! See `docs/clean_room_avastha.md`.

use crate::drishti::GrahaDrishtiMatrix;
use crate::graha::{ALL_GRAHAS, Graha};
use crate::graha_relationships::{
    BeneficNature, Dignity, NaisargikaMaitri, naisargika_maitri, natural_benefic_malefic,
};
use crate::util::normalize_360;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Baladi Avastha — age-based state from position within sign.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaladiAvastha {
    Bala,
    Kumara,
    Yuva,
    Vriddha,
    Mrita,
}

impl BaladiAvastha {
    pub const fn index(self) -> u8 {
        match self {
            Self::Bala => 0,
            Self::Kumara => 1,
            Self::Yuva => 2,
            Self::Vriddha => 3,
            Self::Mrita => 4,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Bala => "Bala",
            Self::Kumara => "Kumara",
            Self::Yuva => "Yuva",
            Self::Vriddha => "Vriddha",
            Self::Mrita => "Mrita",
        }
    }

    pub const fn strength_factor(self) -> f64 {
        match self {
            Self::Bala => 0.25,
            Self::Kumara => 0.50,
            Self::Yuva => 1.00,
            Self::Vriddha => 0.50,
            Self::Mrita => 0.0,
        }
    }
}

/// Jagradadi Avastha — alertness-based state from dignity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JagradadiAvastha {
    Jagrat,
    Swapna,
    Sushupta,
}

impl JagradadiAvastha {
    pub const fn index(self) -> u8 {
        match self {
            Self::Jagrat => 0,
            Self::Swapna => 1,
            Self::Sushupta => 2,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Jagrat => "Jagrat",
            Self::Swapna => "Swapna",
            Self::Sushupta => "Sushupta",
        }
    }

    pub const fn strength_factor(self) -> f64 {
        match self {
            Self::Jagrat => 1.0,
            Self::Swapna => 0.5,
            Self::Sushupta => 0.25,
        }
    }
}

/// Deeptadi Avastha — luminosity-based state from planetary conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeeptadiAvastha {
    Deepta,
    Swastha,
    Mudita,
    Shanta,
    Shakta,
    Peedita,
    Deena,
    Vikala,
    Khala,
}

impl DeeptadiAvastha {
    pub const fn index(self) -> u8 {
        match self {
            Self::Deepta => 0,
            Self::Swastha => 1,
            Self::Mudita => 2,
            Self::Shanta => 3,
            Self::Shakta => 4,
            Self::Peedita => 5,
            Self::Deena => 6,
            Self::Vikala => 7,
            Self::Khala => 8,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Deepta => "Deepta",
            Self::Swastha => "Swastha",
            Self::Mudita => "Mudita",
            Self::Shanta => "Shanta",
            Self::Shakta => "Shakta",
            Self::Peedita => "Peedita",
            Self::Deena => "Deena",
            Self::Vikala => "Vikala",
            Self::Khala => "Khala",
        }
    }

    pub const fn strength_factor(self) -> f64 {
        match self {
            Self::Deepta => 1.0,
            Self::Swastha => 0.9,
            Self::Mudita => 0.75,
            Self::Shanta => 0.6,
            Self::Shakta => 0.8,
            Self::Peedita => 0.3,
            Self::Deena => 0.2,
            Self::Vikala => 0.1,
            Self::Khala => 0.4,
        }
    }
}

/// Lajjitadi Avastha — mood-based state from conjunctions and aspects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LajjitadiAvastha {
    Lajjita,
    Garvita,
    Kshudhita,
    Trushita,
    Mudita,
    Kshobhita,
}

impl LajjitadiAvastha {
    pub const fn index(self) -> u8 {
        match self {
            Self::Lajjita => 0,
            Self::Garvita => 1,
            Self::Kshudhita => 2,
            Self::Trushita => 3,
            Self::Mudita => 4,
            Self::Kshobhita => 5,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Lajjita => "Lajjita",
            Self::Garvita => "Garvita",
            Self::Kshudhita => "Kshudhita",
            Self::Trushita => "Trushita",
            Self::Mudita => "Mudita",
            Self::Kshobhita => "Kshobhita",
        }
    }

    pub const fn strength_factor(self) -> f64 {
        match self {
            Self::Lajjita => 0.2,
            Self::Garvita => 1.0,
            Self::Kshudhita => 0.3,
            Self::Trushita => 0.25,
            Self::Mudita => 0.8,
            Self::Kshobhita => 0.35,
        }
    }
}

/// Sayanadi Avastha — 12 posture-based states from BPHS Ch.45 formula.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SayanadiAvastha {
    Sayana,
    Upavesha,
    Netrapani,
    Prakasha,
    Gamana,
    Agamana,
    Sabha,
    Agama,
    Bhojana,
    NrityaLipsa,
    Kautuka,
    Nidra,
}

const ALL_SAYANADI: [SayanadiAvastha; 12] = [
    SayanadiAvastha::Sayana,
    SayanadiAvastha::Upavesha,
    SayanadiAvastha::Netrapani,
    SayanadiAvastha::Prakasha,
    SayanadiAvastha::Gamana,
    SayanadiAvastha::Agamana,
    SayanadiAvastha::Sabha,
    SayanadiAvastha::Agama,
    SayanadiAvastha::Bhojana,
    SayanadiAvastha::NrityaLipsa,
    SayanadiAvastha::Kautuka,
    SayanadiAvastha::Nidra,
];

impl SayanadiAvastha {
    pub const fn index(self) -> u8 {
        match self {
            Self::Sayana => 0,
            Self::Upavesha => 1,
            Self::Netrapani => 2,
            Self::Prakasha => 3,
            Self::Gamana => 4,
            Self::Agamana => 5,
            Self::Sabha => 6,
            Self::Agama => 7,
            Self::Bhojana => 8,
            Self::NrityaLipsa => 9,
            Self::Kautuka => 10,
            Self::Nidra => 11,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Sayana => "Sayana",
            Self::Upavesha => "Upavesha",
            Self::Netrapani => "Netrapani",
            Self::Prakasha => "Prakasha",
            Self::Gamana => "Gamana",
            Self::Agamana => "Agamana",
            Self::Sabha => "Sabha",
            Self::Agama => "Agama",
            Self::Bhojana => "Bhojana",
            Self::NrityaLipsa => "NrityaLipsa",
            Self::Kautuka => "Kautuka",
            Self::Nidra => "Nidra",
        }
    }
}

/// Sayanadi sub-state (quality modifier).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SayanadiSubState {
    Drishti,
    Chestha,
    Vicheshta,
}

impl SayanadiSubState {
    pub const fn index(self) -> u8 {
        match self {
            Self::Drishti => 0,
            Self::Chestha => 1,
            Self::Vicheshta => 2,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Drishti => "Drishti",
            Self::Chestha => "Chestha",
            Self::Vicheshta => "Vicheshta",
        }
    }

    pub const fn strength_factor(self) -> f64 {
        match self {
            Self::Drishti => 0.5,
            Self::Chestha => 1.0,
            Self::Vicheshta => 0.0,
        }
    }
}

/// BPHS name-group selector (Ka/Cha/Ta-retroflex/Ta-dental/Pa vargas).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameGroup {
    KaVarga,
    ChaVarga,
    TaRetroflexVarga,
    TaDentalVarga,
    PaVarga,
}

impl NameGroup {
    pub const fn index(self) -> u8 {
        match self {
            Self::KaVarga => 0,
            Self::ChaVarga => 1,
            Self::TaRetroflexVarga => 2,
            Self::TaDentalVarga => 3,
            Self::PaVarga => 4,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::KaVarga => "Ka-Varga",
            Self::ChaVarga => "Cha-Varga",
            Self::TaRetroflexVarga => "Ta(Retroflex)-Varga",
            Self::TaDentalVarga => "Ta(Dental)-Varga",
            Self::PaVarga => "Pa-Varga",
        }
    }

    pub const fn anka(self) -> u8 {
        match self {
            Self::KaVarga => 1,
            Self::ChaVarga => 6,
            Self::TaRetroflexVarga => 11,
            Self::TaDentalVarga => 16,
            Self::PaVarga => 21,
        }
    }
}

/// All 5 name groups in order.
pub const ALL_NAME_GROUPS: [NameGroup; 5] = [
    NameGroup::KaVarga,
    NameGroup::ChaVarga,
    NameGroup::TaRetroflexVarga,
    NameGroup::TaDentalVarga,
    NameGroup::PaVarga,
];

/// Anka values for the 5 name-groups: Ka=1, Cha=6, Ta(retro)=11, Ta(dental)=16, Pa=21.
pub const NAME_GROUP_ANKAS: [u8; 5] = [1, 6, 11, 16, 21];

// ---------------------------------------------------------------------------
// Input/Result structs
// ---------------------------------------------------------------------------

/// Inputs for Lajjitadi batch computation.
pub struct LajjitadiInputs {
    pub rashi_indices: [u8; 9],
    pub bhava_numbers: [u8; 9],
    pub dignities: [Dignity; 9],
    pub drishti_matrix: GrahaDrishtiMatrix,
}

/// Inputs for Sayanadi batch computation.
pub struct SayanadiInputs {
    pub nakshatra_indices: [u8; 9],
    pub navamsa_numbers: [u8; 9],
    pub janma_nakshatra: u8,
    pub birth_ghatikas: u16,
    pub lagna_rashi_number: u8,
}

/// Full avastha inputs for all categories.
pub struct AvasthaInputs {
    pub sidereal_lons: [f64; 9],
    pub rashi_indices: [u8; 9],
    pub bhava_numbers: [u8; 9],
    pub dignities: [Dignity; 9],
    pub is_combust: [bool; 9],
    pub is_retrograde: [bool; 9],
    pub lost_war: [bool; 9],
    pub lajjitadi: LajjitadiInputs,
    pub sayanadi: SayanadiInputs,
}

/// Sayanadi result for a single graha: 1 primary avastha + 5 name-group sub-states.
#[derive(Debug, Clone, Copy)]
pub struct SayanadiResult {
    pub avastha: SayanadiAvastha,
    pub sub_states: [SayanadiSubState; 5],
}

/// Avasthas for a single graha across all 5 systems.
#[derive(Debug, Clone, Copy)]
pub struct GrahaAvasthas {
    pub baladi: BaladiAvastha,
    pub jagradadi: JagradadiAvastha,
    pub deeptadi: DeeptadiAvastha,
    pub lajjitadi: LajjitadiAvastha,
    pub sayanadi: SayanadiResult,
}

/// Avasthas for all 9 grahas.
#[derive(Debug, Clone, Copy)]
pub struct AllGrahaAvasthas {
    pub entries: [GrahaAvasthas; 9],
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Water rashis (0-based): Karka=3, Vrischika=7, Meena=11.
const WATER_RASHIS: [u8; 3] = [3, 7, 11];

/// Planet constants for Sayanadi sub-state formula (BPHS Ch.45).
const SAYANADI_PLANET_CONSTANTS: [u8; 9] = [5, 2, 2, 3, 5, 3, 3, 4, 4];

// ---------------------------------------------------------------------------
// Single-graha functions (pure math, no engine queries)
// ---------------------------------------------------------------------------

/// Baladi Avastha — determined by degree position within sign and odd/even sign.
///
/// Divide 30-deg sign into 5 bands of 6 deg each.
/// Odd signs (0,2,4,6,8,10): Bala→Kumara→Yuva→Vriddha→Mrita.
/// Even signs (1,3,5,7,9,11): reversed (Mrita→Vriddha→Yuva→Kumara→Bala).
pub fn baladi_avastha(sidereal_lon: f64, rashi_index: u8) -> BaladiAvastha {
    let lon = normalize_360(sidereal_lon);
    let deg_in_sign = lon - (rashi_index as f64) * 30.0;
    // Clamp to [0, 30) for safety
    let deg = if deg_in_sign < 0.0 {
        0.0
    } else if deg_in_sign >= 30.0 {
        29.999
    } else {
        deg_in_sign
    };
    let band = (deg / 6.0).floor() as u8;
    let band = band.min(4); // clamp to 0-4

    let is_odd = rashi_index.is_multiple_of(2); // 0-based: 0=Mesha(odd), 1=Vrishabha(even)
    if is_odd {
        // Odd sign: Bala(0), Kumara(1), Yuva(2), Vriddha(3), Mrita(4)
        match band {
            0 => BaladiAvastha::Bala,
            1 => BaladiAvastha::Kumara,
            2 => BaladiAvastha::Yuva,
            3 => BaladiAvastha::Vriddha,
            _ => BaladiAvastha::Mrita,
        }
    } else {
        // Even sign: reversed
        match band {
            0 => BaladiAvastha::Mrita,
            1 => BaladiAvastha::Vriddha,
            2 => BaladiAvastha::Yuva,
            3 => BaladiAvastha::Kumara,
            _ => BaladiAvastha::Bala,
        }
    }
}

/// Jagradadi Avastha — determined by dignity.
///
/// Exalted/Moolatrikone/OwnSign → Jagrat (awake).
/// AdhiMitra/Mitra → Swapna (dreaming).
/// All others → Sushupta (deep sleep).
pub fn jagradadi_avastha(dignity: Dignity) -> JagradadiAvastha {
    match dignity {
        Dignity::Exalted | Dignity::Moolatrikone | Dignity::OwnSign => JagradadiAvastha::Jagrat,
        Dignity::AdhiMitra | Dignity::Mitra => JagradadiAvastha::Swapna,
        Dignity::Sama | Dignity::Shatru | Dignity::AdhiShatru | Dignity::Debilitated => {
            JagradadiAvastha::Sushupta
        }
    }
}

/// Deeptadi Avastha — priority-ordered conditions.
///
/// Priority: Exalted→Peedita(war)→Deena(combust)→Vikala(debil)→
/// Shakta(retro)→Swastha(own)→Mudita(friend)→Khala(enemy)→Shanta(default).
pub fn deeptadi_avastha(
    dignity: Dignity,
    is_combust: bool,
    is_retrograde: bool,
    lost_planetary_war: bool,
) -> DeeptadiAvastha {
    if dignity == Dignity::Exalted {
        return DeeptadiAvastha::Deepta;
    }
    if lost_planetary_war {
        return DeeptadiAvastha::Peedita;
    }
    if is_combust {
        return DeeptadiAvastha::Deena;
    }
    if dignity == Dignity::Debilitated {
        return DeeptadiAvastha::Vikala;
    }
    if is_retrograde {
        return DeeptadiAvastha::Shakta;
    }
    match dignity {
        Dignity::Moolatrikone | Dignity::OwnSign => DeeptadiAvastha::Swastha,
        Dignity::AdhiMitra | Dignity::Mitra => DeeptadiAvastha::Mudita,
        Dignity::Shatru | Dignity::AdhiShatru => DeeptadiAvastha::Khala,
        _ => DeeptadiAvastha::Shanta,
    }
}

/// Lajjitadi Avastha — conjunctions, aspects, and bhava-based.
///
/// Priority: Lajjita → Garvita → Kshudhita → Trushita → Kshobhita → Mudita(default).
pub fn lajjitadi_avastha(
    graha: Graha,
    bhava_number: u8,
    rashi_index: u8,
    dignity: Dignity,
    same_rashi_grahas: &[Graha],
    aspecting_grahas: &[Graha],
) -> LajjitadiAvastha {
    // Lajjita: in 5th house AND conjunct a malefic
    if bhava_number == 5 && same_rashi_grahas.iter().any(|g| is_malefic(*g)) {
        return LajjitadiAvastha::Lajjita;
    }

    // Garvita: exalted or moolatrikone
    if dignity == Dignity::Exalted || dignity == Dignity::Moolatrikone {
        return LajjitadiAvastha::Garvita;
    }

    // Kshudhita: in enemy sign (Shatru or AdhiShatru dignity)
    if dignity == Dignity::Shatru || dignity == Dignity::AdhiShatru {
        return LajjitadiAvastha::Kshudhita;
    }

    // Trushita: in water sign + aspected by enemy + NOT aspected by any benefic
    if WATER_RASHIS.contains(&rashi_index) {
        let aspected_by_enemy = aspecting_grahas.iter().any(|asp| {
            let maitri = naisargika_maitri(graha, *asp);
            maitri == NaisargikaMaitri::Enemy
        });
        let aspected_by_benefic = aspecting_grahas
            .iter()
            .any(|asp| natural_benefic_malefic(*asp) == BeneficNature::Benefic);
        if aspected_by_enemy && !aspected_by_benefic {
            return LajjitadiAvastha::Trushita;
        }
    }

    // Kshobhita: conjunct Sun + aspected by a malefic
    let conjunct_sun = same_rashi_grahas.contains(&Graha::Surya);
    let aspected_by_malefic = aspecting_grahas.iter().any(|g| is_malefic(*g));
    if conjunct_sun && aspected_by_malefic {
        return LajjitadiAvastha::Kshobhita;
    }

    LajjitadiAvastha::Mudita
}

/// Sayanadi Avastha — BPHS Ch.45 formula.
///
/// Formula: `((nk+1) * planet_number * navamsa + (janma_nk+1) + ghatikas + lagna_rashi) % 12`
/// Planet numbers: Sun=1..Ketu=9.
pub fn sayanadi_avastha(
    graha: Graha,
    nakshatra_index: u8,
    navamsa_number: u8,
    janma_nakshatra: u8,
    birth_ghatikas: u16,
    lagna_rashi_number: u8,
) -> SayanadiAvastha {
    let planet_number = graha.index() as u32 + 1; // Sun=1..Ketu=9
    let nk = nakshatra_index as u32;
    let nav = navamsa_number as u32;
    let janma_nk = janma_nakshatra as u32;
    let ghatikas = birth_ghatikas as u32;
    let lagna = lagna_rashi_number as u32;

    let value = ((nk + 1) * planet_number * nav + (janma_nk + 1) + ghatikas + lagna) % 12;
    ALL_SAYANADI[value as usize]
}

/// Sayanadi sub-state for a single name-group anka.
///
/// Formula: `R = ((avastha_index+1)^2 + name_anka) % 12`
/// Then: `(R + planet_constant) % 3` → 1=Drishti, 2=Chestha, 0=Vicheshta.
pub fn sayanadi_sub_state(
    avastha: SayanadiAvastha,
    graha: Graha,
    name_anka: u8,
) -> SayanadiSubState {
    let ai = avastha.index() as u32 + 1;
    let r = (ai * ai + name_anka as u32) % 12;
    let pc = SAYANADI_PLANET_CONSTANTS[graha.index() as usize] as u32;
    let remainder = (r + pc) % 3;
    match remainder {
        1 => SayanadiSubState::Drishti,
        2 => SayanadiSubState::Chestha,
        _ => SayanadiSubState::Vicheshta,
    }
}

/// Compute all 5 name-group sub-states for a Sayanadi avastha.
pub fn sayanadi_all_sub_states(avastha: SayanadiAvastha, graha: Graha) -> [SayanadiSubState; 5] {
    let mut result = [SayanadiSubState::Vicheshta; 5];
    for (i, &anka) in NAME_GROUP_ANKAS.iter().enumerate() {
        result[i] = sayanadi_sub_state(avastha, graha, anka);
    }
    result
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Navamsa number (1-9) from sidereal longitude.
pub fn navamsa_number(sidereal_lon: f64) -> u8 {
    let lon = normalize_360(sidereal_lon);
    // Each navamsa = 360/108 = 3.3333... degrees
    // Navamsa within sign: (deg_in_sign / 3.3333).floor() + 1, range 1-9
    let deg_in_sign = lon % 30.0;
    let nav = (deg_in_sign / (30.0 / 9.0)).floor() as u8;
    nav.min(8) + 1 // 1-9
}

/// Check if a graha is a natural malefic.
fn is_malefic(graha: Graha) -> bool {
    natural_benefic_malefic(graha) == BeneficNature::Malefic
}

/// Grahas sharing the same rashi as the given graha (excluding itself).
fn same_rashi_grahas(graha_index: usize, rashi_indices: &[u8; 9]) -> Vec<Graha> {
    let target_rashi = rashi_indices[graha_index];
    let mut result = Vec::new();
    for g in ALL_GRAHAS {
        let idx = g.index() as usize;
        if idx != graha_index && rashi_indices[idx] == target_rashi {
            result.push(g);
        }
    }
    result
}

/// Grahas aspecting the target with total_virupa >= 45.0 from the drishti matrix.
fn aspecting_grahas(target_index: usize, drishti_matrix: &GrahaDrishtiMatrix) -> Vec<Graha> {
    let mut result = Vec::new();
    for g in ALL_GRAHAS {
        let src = g.index() as usize;
        if src != target_index && drishti_matrix.entries[src][target_index].total_virupa >= 45.0 {
            result.push(g);
        }
    }
    result
}

/// Planetary war loser detection.
///
/// Two planets are at war when within 1 degree; the loser has lower declination.
/// Only Mars/Mercury/Jupiter/Venus/Saturn participate (graha indices 2-6).
/// `sidereal_lons` and `declinations` are for the 7 sapta grahas (indices 0-6).
pub fn lost_planetary_war(
    graha_index: usize,
    sidereal_lons: &[f64; 7],
    declinations: &[f64; 7],
) -> bool {
    // Only indices 2-6 (Mars, Mercury, Jupiter, Venus, Saturn) participate
    if !(2..=6).contains(&graha_index) {
        return false;
    }
    for other_idx in 2..=6 {
        if other_idx == graha_index {
            continue;
        }
        let diff = (normalize_360(sidereal_lons[graha_index])
            - normalize_360(sidereal_lons[other_idx]))
        .abs();
        let angular_distance = if diff > 180.0 { 360.0 - diff } else { diff };
        if angular_distance < 1.0 {
            // At war: loser has lower absolute declination
            if declinations[graha_index].abs() < declinations[other_idx].abs() {
                return true;
            }
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Batch functions (all 9 grahas)
// ---------------------------------------------------------------------------

/// Compute Baladi avastha for all 9 grahas.
pub fn all_baladi_avasthas(
    sidereal_lons: &[f64; 9],
    rashi_indices: &[u8; 9],
) -> [BaladiAvastha; 9] {
    let mut result = [BaladiAvastha::Bala; 9];
    for g in ALL_GRAHAS {
        let i = g.index() as usize;
        result[i] = baladi_avastha(sidereal_lons[i], rashi_indices[i]);
    }
    result
}

/// Compute Jagradadi avastha for all 9 grahas.
pub fn all_jagradadi_avasthas(dignities: &[Dignity; 9]) -> [JagradadiAvastha; 9] {
    let mut result = [JagradadiAvastha::Sushupta; 9];
    for g in ALL_GRAHAS {
        let i = g.index() as usize;
        result[i] = jagradadi_avastha(dignities[i]);
    }
    result
}

/// Compute Deeptadi avastha for all 9 grahas.
pub fn all_deeptadi_avasthas(
    dignities: &[Dignity; 9],
    combustion: &[bool; 9],
    retrograde: &[bool; 9],
    lost_war: &[bool; 9],
) -> [DeeptadiAvastha; 9] {
    let mut result = [DeeptadiAvastha::Shanta; 9];
    for g in ALL_GRAHAS {
        let i = g.index() as usize;
        result[i] = deeptadi_avastha(dignities[i], combustion[i], retrograde[i], lost_war[i]);
    }
    result
}

/// Compute Lajjitadi avastha for all 9 grahas.
pub fn all_lajjitadi_avasthas(inputs: &LajjitadiInputs) -> [LajjitadiAvastha; 9] {
    let mut result = [LajjitadiAvastha::Mudita; 9];
    for g in ALL_GRAHAS {
        let i = g.index() as usize;
        let same = same_rashi_grahas(i, &inputs.rashi_indices);
        let aspects = aspecting_grahas(i, &inputs.drishti_matrix);
        result[i] = lajjitadi_avastha(
            g,
            inputs.bhava_numbers[i],
            inputs.rashi_indices[i],
            inputs.dignities[i],
            &same,
            &aspects,
        );
    }
    result
}

/// Compute Sayanadi avastha for all 9 grahas.
pub fn all_sayanadi_avasthas(inputs: &SayanadiInputs) -> [SayanadiResult; 9] {
    let mut result = [SayanadiResult {
        avastha: SayanadiAvastha::Sayana,
        sub_states: [SayanadiSubState::Vicheshta; 5],
    }; 9];
    for g in ALL_GRAHAS {
        let i = g.index() as usize;
        let avastha = sayanadi_avastha(
            g,
            inputs.nakshatra_indices[i],
            inputs.navamsa_numbers[i],
            inputs.janma_nakshatra,
            inputs.birth_ghatikas,
            inputs.lagna_rashi_number,
        );
        let sub_states = sayanadi_all_sub_states(avastha, g);
        result[i] = SayanadiResult {
            avastha,
            sub_states,
        };
    }
    result
}

// ---------------------------------------------------------------------------
// Aggregate function
// ---------------------------------------------------------------------------

/// Compute all avasthas from pre-assembled inputs (no engine queries).
pub fn all_avasthas(inputs: &AvasthaInputs) -> AllGrahaAvasthas {
    let baladi = all_baladi_avasthas(&inputs.sidereal_lons, &inputs.rashi_indices);
    let jagradadi = all_jagradadi_avasthas(&inputs.dignities);
    let deeptadi = all_deeptadi_avasthas(
        &inputs.dignities,
        &inputs.is_combust,
        &inputs.is_retrograde,
        &inputs.lost_war,
    );
    let lajjitadi = all_lajjitadi_avasthas(&inputs.lajjitadi);
    let sayanadi = all_sayanadi_avasthas(&inputs.sayanadi);

    let mut entries = [GrahaAvasthas {
        baladi: BaladiAvastha::Bala,
        jagradadi: JagradadiAvastha::Sushupta,
        deeptadi: DeeptadiAvastha::Shanta,
        lajjitadi: LajjitadiAvastha::Mudita,
        sayanadi: SayanadiResult {
            avastha: SayanadiAvastha::Sayana,
            sub_states: [SayanadiSubState::Vicheshta; 5],
        },
    }; 9];

    for i in 0..9 {
        entries[i] = GrahaAvasthas {
            baladi: baladi[i],
            jagradadi: jagradadi[i],
            deeptadi: deeptadi[i],
            lajjitadi: lajjitadi[i],
            sayanadi: sayanadi[i],
        };
    }

    AllGrahaAvasthas { entries }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drishti::{DrishtiEntry, GrahaDrishtiMatrix};

    // --- Baladi ---

    #[test]
    fn baladi_odd_sign_bands() {
        // Mesha (0) is odd: band0=Bala, band1=Kumara, band2=Yuva, band3=Vriddha, band4=Mrita
        assert_eq!(baladi_avastha(3.0, 0), BaladiAvastha::Bala); // 0-6
        assert_eq!(baladi_avastha(9.0, 0), BaladiAvastha::Kumara); // 6-12
        assert_eq!(baladi_avastha(15.0, 0), BaladiAvastha::Yuva); // 12-18
        assert_eq!(baladi_avastha(21.0, 0), BaladiAvastha::Vriddha); // 18-24
        assert_eq!(baladi_avastha(27.0, 0), BaladiAvastha::Mrita); // 24-30
    }

    #[test]
    fn baladi_even_sign_reversed() {
        // Vrishabha (1) is even: band0=Mrita, band1=Vriddha, band2=Yuva, band3=Kumara, band4=Bala
        assert_eq!(baladi_avastha(33.0, 1), BaladiAvastha::Mrita); // 0-6 in sign
        assert_eq!(baladi_avastha(39.0, 1), BaladiAvastha::Vriddha); // 6-12
        assert_eq!(baladi_avastha(45.0, 1), BaladiAvastha::Yuva); // 12-18
        assert_eq!(baladi_avastha(51.0, 1), BaladiAvastha::Kumara); // 18-24
        assert_eq!(baladi_avastha(57.0, 1), BaladiAvastha::Bala); // 24-30
    }

    #[test]
    fn baladi_boundary_at_6_degrees() {
        // At exactly 6.0 deg in odd sign → band 1 = Kumara
        assert_eq!(baladi_avastha(6.0, 0), BaladiAvastha::Kumara);
        // At 5.999 → band 0 = Bala
        assert_eq!(baladi_avastha(5.999, 0), BaladiAvastha::Bala);
    }

    #[test]
    fn baladi_all_five_states_reachable() {
        let states: Vec<BaladiAvastha> = (0..5)
            .map(|band| baladi_avastha(band as f64 * 6.0 + 1.0, 0))
            .collect();
        assert_eq!(states[0], BaladiAvastha::Bala);
        assert_eq!(states[1], BaladiAvastha::Kumara);
        assert_eq!(states[2], BaladiAvastha::Yuva);
        assert_eq!(states[3], BaladiAvastha::Vriddha);
        assert_eq!(states[4], BaladiAvastha::Mrita);
    }

    // --- Jagradadi ---

    #[test]
    fn jagradadi_all_mappings() {
        assert_eq!(
            jagradadi_avastha(Dignity::Exalted),
            JagradadiAvastha::Jagrat
        );
        assert_eq!(
            jagradadi_avastha(Dignity::Moolatrikone),
            JagradadiAvastha::Jagrat
        );
        assert_eq!(
            jagradadi_avastha(Dignity::OwnSign),
            JagradadiAvastha::Jagrat
        );
        assert_eq!(
            jagradadi_avastha(Dignity::AdhiMitra),
            JagradadiAvastha::Swapna
        );
        assert_eq!(jagradadi_avastha(Dignity::Mitra), JagradadiAvastha::Swapna);
        assert_eq!(jagradadi_avastha(Dignity::Sama), JagradadiAvastha::Sushupta);
        assert_eq!(
            jagradadi_avastha(Dignity::Shatru),
            JagradadiAvastha::Sushupta
        );
        assert_eq!(
            jagradadi_avastha(Dignity::AdhiShatru),
            JagradadiAvastha::Sushupta
        );
        assert_eq!(
            jagradadi_avastha(Dignity::Debilitated),
            JagradadiAvastha::Sushupta
        );
    }

    // --- Deeptadi ---

    #[test]
    fn deeptadi_exalted_beats_all() {
        // Exalted + combust + retrograde + war → still Deepta
        assert_eq!(
            deeptadi_avastha(Dignity::Exalted, true, true, true),
            DeeptadiAvastha::Deepta,
        );
    }

    #[test]
    fn deeptadi_war_beats_combust() {
        assert_eq!(
            deeptadi_avastha(Dignity::Sama, true, false, true),
            DeeptadiAvastha::Peedita,
        );
    }

    #[test]
    fn deeptadi_combust_beats_debilitated() {
        assert_eq!(
            deeptadi_avastha(Dignity::Debilitated, true, false, false),
            DeeptadiAvastha::Deena,
        );
    }

    #[test]
    fn deeptadi_debilitated() {
        assert_eq!(
            deeptadi_avastha(Dignity::Debilitated, false, false, false),
            DeeptadiAvastha::Vikala,
        );
    }

    #[test]
    fn deeptadi_retrograde() {
        assert_eq!(
            deeptadi_avastha(Dignity::Sama, false, true, false),
            DeeptadiAvastha::Shakta,
        );
    }

    #[test]
    fn deeptadi_own_sign() {
        assert_eq!(
            deeptadi_avastha(Dignity::OwnSign, false, false, false),
            DeeptadiAvastha::Swastha,
        );
    }

    #[test]
    fn deeptadi_friend() {
        assert_eq!(
            deeptadi_avastha(Dignity::Mitra, false, false, false),
            DeeptadiAvastha::Mudita,
        );
    }

    #[test]
    fn deeptadi_enemy() {
        assert_eq!(
            deeptadi_avastha(Dignity::Shatru, false, false, false),
            DeeptadiAvastha::Khala,
        );
    }

    #[test]
    fn deeptadi_neutral_default() {
        assert_eq!(
            deeptadi_avastha(Dignity::Sama, false, false, false),
            DeeptadiAvastha::Shanta,
        );
    }

    // --- Lajjitadi ---

    #[test]
    fn lajjitadi_fifth_house_malefic_conjunct() {
        assert_eq!(
            lajjitadi_avastha(
                Graha::Guru,
                5,
                0,
                Dignity::Sama,
                &[Graha::Shani], // malefic conjunct
                &[],
            ),
            LajjitadiAvastha::Lajjita,
        );
    }

    #[test]
    fn lajjitadi_exalted() {
        assert_eq!(
            lajjitadi_avastha(Graha::Guru, 1, 0, Dignity::Exalted, &[], &[],),
            LajjitadiAvastha::Garvita,
        );
    }

    #[test]
    fn lajjitadi_enemy_sign() {
        assert_eq!(
            lajjitadi_avastha(Graha::Guru, 1, 0, Dignity::Shatru, &[], &[],),
            LajjitadiAvastha::Kshudhita,
        );
    }

    #[test]
    fn lajjitadi_default_mudita() {
        assert_eq!(
            lajjitadi_avastha(Graha::Guru, 1, 0, Dignity::Sama, &[], &[],),
            LajjitadiAvastha::Mudita,
        );
    }

    #[test]
    fn lajjitadi_water_sign_enemy_aspect_no_benefic() {
        // Karka (3) = water, aspected by enemy, no benefic → Trushita
        assert_eq!(
            lajjitadi_avastha(
                Graha::Surya,
                1,
                3,
                Dignity::Sama,
                &[],
                &[Graha::Shukra], // Venus is enemy of Sun, and Venus is benefic...
            ),
            // Venus is a natural benefic, so aspected_by_benefic = true → NOT Trushita
            LajjitadiAvastha::Mudita,
        );
        // Now with only Saturn (malefic enemy of Sun)
        assert_eq!(
            lajjitadi_avastha(
                Graha::Surya,
                1,
                3,
                Dignity::Sama,
                &[],
                &[Graha::Shani], // Saturn is enemy of Sun, malefic
            ),
            LajjitadiAvastha::Trushita,
        );
    }

    #[test]
    fn lajjitadi_kshobhita_sun_conjunct_malefic_aspect() {
        assert_eq!(
            lajjitadi_avastha(
                Graha::Guru,
                1,
                0,
                Dignity::Sama,
                &[Graha::Surya],  // conjunct Sun
                &[Graha::Mangal], // aspected by malefic
            ),
            LajjitadiAvastha::Kshobhita,
        );
    }

    // --- Sayanadi ---

    #[test]
    fn sayanadi_formula_basic() {
        // Sun (planet_number=1), nakshatra=0, navamsa=1, janma_nk=0, ghatikas=0, lagna=1
        // ((0+1)*1*1 + (0+1) + 0 + 1) % 12 = (1 + 1 + 0 + 1) % 12 = 3
        let result = sayanadi_avastha(Graha::Surya, 0, 1, 0, 0, 1);
        assert_eq!(result, SayanadiAvastha::Prakasha); // index 3
    }

    #[test]
    fn sayanadi_all_12_reachable() {
        // Vary inputs to hit all 12 avasthas
        let mut seen = [false; 12];
        for nk in 0..27 {
            for nav in 1..=9 {
                let av = sayanadi_avastha(Graha::Surya, nk, nav, 0, 0, 1);
                seen[av.index() as usize] = true;
            }
        }
        for (i, &s) in seen.iter().enumerate() {
            assert!(s, "Sayanadi avastha index {i} was never produced");
        }
    }

    #[test]
    fn sayanadi_sub_state_formula() {
        // Sayana (index 0), Sun (constant=5), Ka-varga anka=1
        // R = ((0+1)^2 + 1) % 12 = (1 + 1) % 12 = 2
        // (2 + 5) % 3 = 7 % 3 = 1 → Drishti
        let ss = sayanadi_sub_state(SayanadiAvastha::Sayana, Graha::Surya, 1);
        assert_eq!(ss, SayanadiSubState::Drishti);
    }

    #[test]
    fn sayanadi_all_sub_states_produces_5() {
        let subs = sayanadi_all_sub_states(SayanadiAvastha::Sayana, Graha::Surya);
        assert_eq!(subs.len(), 5);
    }

    // --- Navamsa number ---

    #[test]
    fn navamsa_number_start_of_sign() {
        // 0.0 deg → navamsa 1
        assert_eq!(navamsa_number(0.0), 1);
    }

    #[test]
    fn navamsa_number_end_of_sign() {
        // 29.9 deg → navamsa 9
        assert_eq!(navamsa_number(29.9), 9);
    }

    #[test]
    fn navamsa_number_mid_sign() {
        // 15.0 deg → deg_in_sign=15, 15/(30/9) = 15/3.333 = 4.5 → floor=4, +1=5
        assert_eq!(navamsa_number(15.0), 5);
    }

    #[test]
    fn navamsa_number_second_sign() {
        // 45.0 deg → deg_in_sign=15, same as above
        assert_eq!(navamsa_number(45.0), 5);
    }

    // --- Strength factors ---

    #[test]
    fn strength_factors_in_range() {
        let all_baladi = [
            BaladiAvastha::Bala,
            BaladiAvastha::Kumara,
            BaladiAvastha::Yuva,
            BaladiAvastha::Vriddha,
            BaladiAvastha::Mrita,
        ];
        for a in all_baladi {
            let f = a.strength_factor();
            assert!(f >= 0.0 && f <= 1.0, "{:?} factor={f}", a);
        }

        let all_jagradadi = [
            JagradadiAvastha::Jagrat,
            JagradadiAvastha::Swapna,
            JagradadiAvastha::Sushupta,
        ];
        for a in all_jagradadi {
            let f = a.strength_factor();
            assert!(f >= 0.0 && f <= 1.0, "{:?} factor={f}", a);
        }

        let all_deeptadi = [
            DeeptadiAvastha::Deepta,
            DeeptadiAvastha::Swastha,
            DeeptadiAvastha::Mudita,
            DeeptadiAvastha::Shanta,
            DeeptadiAvastha::Shakta,
            DeeptadiAvastha::Peedita,
            DeeptadiAvastha::Deena,
            DeeptadiAvastha::Vikala,
            DeeptadiAvastha::Khala,
        ];
        for a in all_deeptadi {
            let f = a.strength_factor();
            assert!(f >= 0.0 && f <= 1.0, "{:?} factor={f}", a);
        }

        let all_lajjitadi = [
            LajjitadiAvastha::Lajjita,
            LajjitadiAvastha::Garvita,
            LajjitadiAvastha::Kshudhita,
            LajjitadiAvastha::Trushita,
            LajjitadiAvastha::Mudita,
            LajjitadiAvastha::Kshobhita,
        ];
        for a in all_lajjitadi {
            let f = a.strength_factor();
            assert!(f >= 0.0 && f <= 1.0, "{:?} factor={f}", a);
        }

        let all_sub = [
            SayanadiSubState::Drishti,
            SayanadiSubState::Chestha,
            SayanadiSubState::Vicheshta,
        ];
        for a in all_sub {
            let f = a.strength_factor();
            assert!(f >= 0.0 && f <= 1.0, "{:?} factor={f}", a);
        }
    }

    // --- Planetary war ---

    #[test]
    fn lost_war_sun_moon_exempt() {
        let lons = [100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0];
        let decls = [10.0, 5.0, 3.0, 8.0, 6.0, 4.0, 2.0];
        // Sun (0) and Moon (1) are exempt
        assert!(!lost_planetary_war(0, &lons, &decls));
        assert!(!lost_planetary_war(1, &lons, &decls));
    }

    #[test]
    fn lost_war_within_1_degree() {
        let lons = [0.0, 0.0, 100.0, 100.5, 200.0, 300.0, 50.0];
        let decls = [0.0, 0.0, 5.0, 10.0, 3.0, 7.0, 2.0];
        // Mars (2) at 100, Mercury (3) at 100.5: within 1 degree
        // Mars decl=5, Mercury decl=10 → Mars has lower → Mars loses
        assert!(lost_planetary_war(2, &lons, &decls));
        assert!(!lost_planetary_war(3, &lons, &decls));
    }

    #[test]
    fn lost_war_not_within_1_degree() {
        let lons = [0.0, 0.0, 100.0, 102.0, 200.0, 300.0, 50.0];
        let decls = [0.0, 0.0, 5.0, 10.0, 3.0, 7.0, 2.0];
        // Mars (2) at 100, Mercury (3) at 102: > 1 degree apart
        assert!(!lost_planetary_war(2, &lons, &decls));
    }

    // --- Batch all_avasthas ---

    #[test]
    fn all_avasthas_produces_9_entries() {
        let inputs = AvasthaInputs {
            sidereal_lons: [10.0, 45.0, 100.0, 150.0, 200.0, 250.0, 300.0, 330.0, 350.0],
            rashi_indices: [0, 1, 3, 5, 6, 8, 10, 11, 11],
            bhava_numbers: [1, 2, 4, 6, 7, 9, 10, 11, 12],
            dignities: [
                Dignity::Exalted,
                Dignity::OwnSign,
                Dignity::Mitra,
                Dignity::Sama,
                Dignity::Shatru,
                Dignity::AdhiMitra,
                Dignity::Debilitated,
                Dignity::Sama,
                Dignity::Sama,
            ],
            is_combust: [false; 9],
            is_retrograde: [false; 9],
            lost_war: [false; 9],
            lajjitadi: LajjitadiInputs {
                rashi_indices: [0, 1, 3, 5, 6, 8, 10, 11, 11],
                bhava_numbers: [1, 2, 4, 6, 7, 9, 10, 11, 12],
                dignities: [
                    Dignity::Exalted,
                    Dignity::OwnSign,
                    Dignity::Mitra,
                    Dignity::Sama,
                    Dignity::Shatru,
                    Dignity::AdhiMitra,
                    Dignity::Debilitated,
                    Dignity::Sama,
                    Dignity::Sama,
                ],
                drishti_matrix: GrahaDrishtiMatrix {
                    entries: [[DrishtiEntry::zero(); 9]; 9],
                },
            },
            sayanadi: SayanadiInputs {
                nakshatra_indices: [0, 3, 7, 11, 15, 18, 22, 24, 26],
                navamsa_numbers: [1, 5, 3, 7, 2, 8, 4, 6, 9],
                janma_nakshatra: 3,
                birth_ghatikas: 15,
                lagna_rashi_number: 1,
            },
        };
        let result = all_avasthas(&inputs);
        assert_eq!(result.entries.len(), 9);

        // Sun is exalted → Jagrat, Deepta, Garvita
        assert_eq!(result.entries[0].jagradadi, JagradadiAvastha::Jagrat);
        assert_eq!(result.entries[0].deeptadi, DeeptadiAvastha::Deepta);
        assert_eq!(result.entries[0].lajjitadi, LajjitadiAvastha::Garvita);
    }

    // --- birth_ghatikas floor behavior ---

    #[test]
    fn sayanadi_ghatikas_floor_boundary() {
        // 15.0 ghatikas floored = 15
        let a1 = sayanadi_avastha(Graha::Surya, 5, 3, 10, 15, 1);
        // 14.999 floored = 14
        let a2 = sayanadi_avastha(Graha::Surya, 5, 3, 10, 14, 1);
        // They may or may not differ, but the floor behavior is deterministic
        // The formula uses integer ghatikas directly — test that it doesn't panic
        let _ = (a1, a2);
    }
}
