//! Open derived Vedic calculations built on core ephemeris outputs.
//!
//! This crate provides:
//! - Ayanamsha computation for 20 sidereal reference systems
//! - Sunrise/sunset and twilight calculations
//! - Lagna (Ascendant) and MC computation
//! - Bhava (house) systems: 10 division methods
//! - Rashi (zodiac sign) and DMS conversion
//! - Nakshatra (lunar mansion) with pada, 27 and 28 schemes
//! - Tithi (lunar day), Karana (half-tithi), Yoga (luni-solar yoga)
//! - Vaar (weekday), Hora (planetary hour), Ghatika (time division)
//! - Graha (planet) enum with rashi lordship
//! - Sphuta (sensitive point) calculations (16 formulas)
//!
//! All implementations are clean-room, derived from IAU standards
//! and public astronomical formulas.

pub mod amsha;
pub mod arudha;
pub mod avastha;
pub mod combustion;
pub mod ashtakavarga;
pub mod ayana_type;
pub mod ayanamsha;
pub mod bhava;
pub mod bhava_types;
pub mod drishti;
pub mod error;
pub mod ghatika;
pub mod graha;
pub mod graha_relationships;
pub mod hora;
pub mod karana;
pub mod lagna;
pub mod lunar_nodes;
pub mod masa;
pub mod nakshatra;
pub mod rashi;
pub mod riseset;
pub mod riseset_types;
pub mod samvatsara;
pub mod shadbala;
pub mod special_lagna;
pub mod sphuta;
pub mod tithi;
pub mod upagraha;
pub mod vimsopaka;
pub mod util;
pub mod vaar;
pub mod yoga;

pub use amsha::{
    ALL_AMSHAS, Amsha, AmshaRequest, AmshaVariation, RashiElement, SHODASHAVARGA,
    amsha_from_rashi_position, amsha_longitude, amsha_longitudes, amsha_rashi_info,
    amsha_rashi_infos, rashi_element, rashi_position_to_longitude,
};
pub use arudha::{ALL_ARUDHA_PADAS, ArudhaPada, ArudhaResult, all_arudha_padas, arudha_pada};
pub use avastha::{
    ALL_NAME_GROUPS, AllGrahaAvasthas, AvasthaInputs, BaladiAvastha, DeeptadiAvastha,
    GrahaAvasthas, JagradadiAvastha, LajjitadiAvastha, LajjitadiInputs, NAME_GROUP_ANKAS,
    NameGroup, SayanadiAvastha, SayanadiInputs, SayanadiResult, SayanadiSubState, all_avasthas,
    all_baladi_avasthas, all_deeptadi_avasthas, all_jagradadi_avasthas, all_lajjitadi_avasthas,
    all_sayanadi_avasthas, baladi_avastha, deeptadi_avastha, jagradadi_avastha, lajjitadi_avastha,
    lost_planetary_war, navamsa_number, sayanadi_all_sub_states, sayanadi_avastha,
    sayanadi_sub_state,
};
pub use combustion::{all_combustion_status, combustion_threshold, is_combust};
pub use ashtakavarga::{
    AshtakavargaResult, BAV_TOTALS, BhinnaAshtakavarga, SAV_TOTAL, SarvaAshtakavarga,
    calculate_all_bav, calculate_ashtakavarga, calculate_bav, calculate_sav, ekadhipatya_sodhana,
    trikona_sodhana,
};
pub use ayana_type::{ALL_AYANAS, Ayana, ayana_from_sidereal_longitude};
pub use ayanamsha::{
    AyanamshaSystem, ayanamsha_deg, ayanamsha_mean_deg, ayanamsha_true_deg, jd_tdb_to_centuries,
    tdb_seconds_to_centuries,
};
pub use bhava::compute_bhavas;
pub use bhava_types::{
    Bhava, BhavaConfig, BhavaReferenceMode, BhavaResult, BhavaStartingPoint, BhavaSystem,
};
pub use drishti::{
    DrishtiEntry, GrahaDrishtiMatrix, base_virupa, graha_drishti, graha_drishti_matrix,
    special_virupa,
};
pub use error::VedicError;
pub use ghatika::{GHATIKA_COUNT, GHATIKA_MINUTES, GhatikaPosition, ghatika_from_elapsed};
pub use graha::{
    ALL_GRAHAS, GRAHA_KAKSHA_VALUES, Graha, SAPTA_GRAHAS, nth_rashi_from, rashi_lord,
    rashi_lord_by_index,
};
pub use graha_relationships::{
    BeneficNature, Dignity, GrahaGender, NaisargikaMaitri, NodeDignityPolicy, PanchadhaMaitri,
    TatkalikaMaitri, debilitation_degree, dignity_in_rashi, dignity_in_rashi_with_positions,
    exaltation_degree, graha_gender, hora_lord, masa_lord, moon_benefic_nature, moolatrikone_range,
    naisargika_maitri, natural_benefic_malefic, node_dignity_in_rashi, own_signs,
    panchadha_maitri, samvatsara_lord, tatkalika_maitri, vaar_lord,
};
pub use hora::{CHALDEAN_SEQUENCE, HORA_COUNT, Hora, hora_at, vaar_day_lord};
pub use karana::{ALL_KARANAS, KARANA_SEGMENT_DEG, Karana, KaranaPosition, karana_from_elongation};
pub use lagna::{lagna_and_mc_rad, lagna_longitude_rad, mc_longitude_rad, ramc_rad};
pub use lunar_nodes::{
    LunarNode, NodeMode, lunar_node_deg, mean_ketu_deg, mean_rahu_deg, true_ketu_deg, true_rahu_deg,
};
pub use masa::{ALL_MASAS, Masa, masa_from_rashi_index};
pub use nakshatra::{
    ALL_NAKSHATRAS_27, ALL_NAKSHATRAS_28, NAKSHATRA_SPAN_27, Nakshatra, Nakshatra28,
    Nakshatra28Info, NakshatraInfo, nakshatra_from_longitude, nakshatra_from_tropical,
    nakshatra28_from_longitude, nakshatra28_from_tropical,
};
pub use rashi::{
    ALL_RASHIS, Dms, Rashi, RashiInfo, deg_to_dms, dms_to_deg, rashi_from_longitude,
    rashi_from_tropical,
};
pub use riseset::{approximate_local_noon_jd, compute_all_events, compute_rise_set};
pub use riseset_types::{GeoLocation, RiseSetConfig, RiseSetEvent, RiseSetResult, SunLimb};
pub use samvatsara::{ALL_SAMVATSARAS, SAMVATSARA_EPOCH_YEAR, Samvatsara, samvatsara_from_year};
pub use shadbala::{
    DIG_BALA_BHAVA, KalaBalaBreakdown, KalaBalaInputs, MAX_SPEED, NAISARGIKA_BALA,
    REQUIRED_STRENGTH, ShadbalaBreakdown, ShadbalaInputs, SthanaBalaBreakdown, abda_bala,
    all_ayana_balas, all_cheshta_balas, all_dig_balas, all_drik_balas, all_drekkana_balas,
    all_hora_balas, all_kala_balas, all_kendradi_balas, all_masa_balas, all_naisargika_balas,
    all_nathonnatha_balas, all_ojhayugma_balas, all_paksha_balas, all_shadbalas_from_inputs,
    all_sthana_balas, all_tribhaga_balas, all_uchcha_balas, all_vara_balas, all_yuddha_balas,
    ayana_bala, cheshta_bala, dig_bala, drik_bala, drekkana_bala, hora_bala as shadbala_hora_bala,
    kala_bala, kendradi_bala, masa_bala as shadbala_masa_bala, naisargika_bala, nathonnatha_bala,
    ojhayugma_bala, paksha_bala, shadbala_from_inputs, sthana_bala, tribhaga_bala, uchcha_bala,
    vara_bala, yuddha_bala,
};
pub use special_lagna::{
    ALL_SPECIAL_LAGNAS, AllSpecialLagnas, SpecialLagna, all_special_lagnas, bhava_lagna,
    ghati_lagna, ghatikas_since_sunrise, hora_lagna, indu_lagna, pranapada_lagna, sree_lagna,
    varnada_lagna, vighati_lagna,
};
pub use sphuta::{
    ALL_SPHUTAS, Sphuta, SphutalInputs, all_sphutas, avayoga_sphuta, beeja_sphuta, bhrigu_bindu,
    chatussphuta, deha_sphuta, kshetra_sphuta, kunda, mrityu_sphuta, panchasphuta, prana_sphuta,
    rahu_tithi_sphuta, sookshma_trisphuta, tithi_sphuta, trisphuta, yoga_sphuta,
    yoga_sphuta_normalized,
};
pub use tithi::{
    ALL_TITHIS, Paksha, TITHI_SEGMENT_DEG, Tithi, TithiPosition, tithi_from_elongation,
};
pub use upagraha::{
    ALL_UPAGRAHAS, AllUpagrahas, SunBasedUpagrahas, TIME_BASED_UPAGRAHAS, Upagraha,
    day_portion_index, night_portion_index, portion_jd_range, sun_based_upagrahas,
    time_upagraha_jd, time_upagraha_planet,
};
pub use util::normalize_360;
pub use vimsopaka::{
    DASHAVARGA, SHADVARGA, SHODASAVARGA, SAPTAVARGA, VargaDignityEntry, VargaWeight,
    VimsopakaBala, all_dashavarga_vimsopaka, all_shadvarga_vimsopaka, all_shodasavarga_vimsopaka,
    all_saptavarga_vimsopaka, all_vimsopaka_balas, dashavarga_vimsopaka, shadvarga_vimsopaka,
    shodasavarga_vimsopaka, saptavarga_vimsopaka, vimsopaka_bala, vimsopaka_dignity_points,
    vimsopaka_from_entries,
};
pub use vaar::{ALL_VAARS, Vaar, vaar_from_jd};
pub use yoga::{ALL_YOGAS, YOGA_SEGMENT_DEG, Yoga, YogaPosition, yoga_from_sum};
