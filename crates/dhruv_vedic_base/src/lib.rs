//! Open derived Vedic calculations built on core ephemeris outputs.
//!
//! This crate provides:
//! - Ayanamsha computation for 20 sidereal reference systems
//! - Sunrise/sunset and twilight calculations
//! - Ascendant (Lagna) and MC computation
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

pub mod arudha;
pub mod ascendant;
pub mod ashtakavarga;
pub mod ayana_type;
pub mod ayanamsha;
pub mod bhava;
pub mod bhava_types;
pub mod error;
pub mod ghatika;
pub mod graha;
pub mod hora;
pub mod karana;
pub mod lunar_nodes;
pub mod masa;
pub mod nakshatra;
pub mod rashi;
pub mod riseset;
pub mod riseset_types;
pub mod samvatsara;
pub mod special_lagna;
pub mod sphuta;
pub mod tithi;
pub mod upagraha;
pub mod util;
pub mod vaar;
pub mod yoga;

pub use arudha::{
    ALL_ARUDHA_PADAS, ArudhaPada, ArudhaResult, all_arudha_padas, arudha_pada,
};
pub use ascendant::{ascendant_and_mc_rad, ascendant_longitude_rad, mc_longitude_rad, ramc_rad};
pub use ayanamsha::{
    AyanamshaSystem, ayanamsha_deg, ayanamsha_mean_deg, ayanamsha_true_deg,
    jd_tdb_to_centuries, tdb_seconds_to_centuries,
};
pub use bhava::compute_bhavas;
pub use bhava_types::{
    Bhava, BhavaConfig, BhavaReferenceMode, BhavaResult, BhavaStartingPoint, BhavaSystem,
};
pub use error::VedicError;
pub use ghatika::{GHATIKA_COUNT, GHATIKA_MINUTES, GhatikaPosition, ghatika_from_elapsed};
pub use hora::{CHALDEAN_SEQUENCE, HORA_COUNT, Hora, hora_at, vaar_day_lord};
pub use karana::{ALL_KARANAS, KARANA_SEGMENT_DEG, Karana, KaranaPosition, karana_from_elongation};
pub use lunar_nodes::{
    LunarNode, NodeMode, lunar_node_deg, mean_ketu_deg, mean_rahu_deg, true_ketu_deg,
    true_rahu_deg,
};
pub use nakshatra::{
    ALL_NAKSHATRAS_27, ALL_NAKSHATRAS_28, NAKSHATRA_SPAN_27, Nakshatra, Nakshatra28,
    Nakshatra28Info, NakshatraInfo, nakshatra28_from_longitude, nakshatra28_from_tropical,
    nakshatra_from_longitude, nakshatra_from_tropical,
};
pub use rashi::{
    ALL_RASHIS, Dms, Rashi, RashiInfo, deg_to_dms, rashi_from_longitude, rashi_from_tropical,
};
pub use riseset::{approximate_local_noon_jd, compute_all_events, compute_rise_set};
pub use riseset_types::{GeoLocation, RiseSetConfig, RiseSetEvent, RiseSetResult, SunLimb};
pub use tithi::{ALL_TITHIS, TITHI_SEGMENT_DEG, Paksha, Tithi, TithiPosition, tithi_from_elongation};
pub use vaar::{ALL_VAARS, Vaar, vaar_from_jd};
pub use yoga::{ALL_YOGAS, YOGA_SEGMENT_DEG, Yoga, YogaPosition, yoga_from_sum};
pub use masa::{ALL_MASAS, Masa, masa_from_rashi_index};
pub use ayana_type::{ALL_AYANAS, Ayana, ayana_from_sidereal_longitude};
pub use samvatsara::{
    ALL_SAMVATSARAS, SAMVATSARA_EPOCH_YEAR, Samvatsara, samvatsara_from_year,
};
pub use graha::{
    ALL_GRAHAS, Graha, GRAHA_KAKSHA_VALUES, SAPTA_GRAHAS, nth_rashi_from, rashi_lord,
    rashi_lord_by_index,
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
pub use upagraha::{
    ALL_UPAGRAHAS, AllUpagrahas, SunBasedUpagrahas, TIME_BASED_UPAGRAHAS, Upagraha,
    day_portion_index, night_portion_index, portion_jd_range, sun_based_upagrahas,
    time_upagraha_jd, time_upagraha_planet,
};
pub use ashtakavarga::{
    AshtakavargaResult, BAV_TOTALS, BhinnaAshtakavarga, SAV_TOTAL, SarvaAshtakavarga,
    calculate_all_bav, calculate_ashtakavarga, calculate_bav, calculate_sav, ekadhipatya_sodhana,
    trikona_sodhana,
};
pub use util::normalize_360;
