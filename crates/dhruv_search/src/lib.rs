//! Celestial event search engine: conjunctions, oppositions, aspects, grahan,
//! stationary points, and max-speed events.
//!
//! This crate provides:
//! - General-purpose conjunction/separation engine for any body pair
//! - Chandra grahan (lunar eclipse) computation (penumbral, partial, total)
//! - Surya grahan (solar eclipse) computation (geocentric and topocentric)
//! - Stationary point search (retrograde/direct stations)
//! - Max-speed search (velocity extrema)

pub mod conjunction;
pub mod conjunction_types;
pub mod dasha;
pub mod error;
pub mod grahan;
pub mod grahan_types;
pub mod jyotish;
pub mod jyotish_types;
pub mod lunar_phase;
pub mod lunar_phase_types;
pub mod operations;
pub mod panchang;
pub mod panchang_types;
pub mod sankranti;
pub mod sankranti_types;
pub(crate) mod search_util;
pub mod stationary;
pub mod stationary_types;

pub use conjunction::{
    body_ecliptic_lon_lat, body_lon_lat_on_plane, next_conjunction, prev_conjunction,
    search_conjunctions,
};
pub use conjunction_types::{ConjunctionConfig, ConjunctionEvent, SearchDirection};
pub use dasha::{
    DashaInputs, dasha_child_period_for_birth, dasha_child_period_with_inputs,
    dasha_children_for_birth, dasha_children_with_inputs, dasha_complete_level_for_birth,
    dasha_complete_level_with_inputs, dasha_hierarchy_for_birth, dasha_hierarchy_with_inputs,
    dasha_level0_entity_for_birth, dasha_level0_entity_with_inputs, dasha_level0_for_birth,
    dasha_level0_with_inputs, dasha_snapshot_at, dasha_snapshot_with_inputs,
};
pub use dhruv_vedic_base::{
    BhavaBalaBirthPeriod, BhavaBalaEntry, BhavaBalaInputs, BhavaBalaResult, CharakarakaEntry,
    CharakarakaResult, CharakarakaRole, CharakarakaScheme,
};
pub use error::SearchError;
pub use grahan::{
    next_chandra_grahan, next_surya_grahan, prev_chandra_grahan, prev_surya_grahan,
    search_chandra_grahan, search_surya_grahan,
};
pub use grahan_types::{
    ChandraGrahan, ChandraGrahanType, GeoLocation, GrahanConfig, SuryaGrahan, SuryaGrahanType,
};
pub use jyotish::{
    all_upagrahas_for_date, all_upagrahas_for_date_with_config, amsha_charts_for_date,
    amsha_charts_from_kundali, arudha_padas_for_date, ashtakavarga_for_date, avastha_for_date,
    avastha_for_graha, balas_for_date, bhavabala_for_bhava, bhavabala_for_date,
    charakaraka_for_date, core_bindus, drishti_for_date, full_kundali_for_date, graha_longitudes,
    graha_positions, moving_osculating_apogees, moving_osculating_apogees_for_date,
    shadbala_for_date, shadbala_for_graha, sidereal_bhava_results_for_date,
    sidereal_bhavas_for_date, sidereal_lagna_for_date, sidereal_mc_for_date,
    siderealize_bhava_result, special_lagnas_for_date, tropical_to_sidereal_longitude,
    vimsopaka_for_date, vimsopaka_for_graha,
};
pub use jyotish_types::{
    AmshaChart, AmshaChartScope, AmshaEntry, AmshaResult, AmshaSelectionConfig, BalaBundleResult,
    BhavaResultSet, BindusConfig, BindusResult, DashaSelectionConfig, DashaSnapshotTime,
    DrishtiConfig, DrishtiResult, FullKundaliConfig, FullKundaliResult, GrahaEntry,
    GrahaLongitudeKind, GrahaLongitudes, GrahaLongitudesConfig, GrahaPositions,
    GrahaPositionsConfig, MAX_AMSHA_REQUESTS, MovingOsculatingApogeeEntry, MovingOsculatingApogees,
    ShadbalaEntry, ShadbalaResult, SphutalResult, VimsopakaEntry, VimsopakaResult,
};
pub use lunar_phase::{
    next_amavasya, next_purnima, prev_amavasya, prev_purnima, search_amavasyas, search_purnimas,
};
pub use lunar_phase_types::{LunarPhase, LunarPhaseEvent};
pub use operations::{
    AyanamshaMode, AyanamshaOperation, ConjunctionOperation, ConjunctionQuery, ConjunctionResult,
    GrahanKind, GrahanOperation, GrahanQuery, GrahanResult, LunarPhaseKind, LunarPhaseOperation,
    LunarPhaseQuery, LunarPhaseResult, MotionKind, MotionOperation, MotionQuery, MotionResult,
    NodeBackend, NodeOperation, PANCHANG_INCLUDE_ALL, PANCHANG_INCLUDE_ALL_CALENDAR,
    PANCHANG_INCLUDE_ALL_CORE, PANCHANG_INCLUDE_AYANA, PANCHANG_INCLUDE_GHATIKA,
    PANCHANG_INCLUDE_HORA, PANCHANG_INCLUDE_KARANA, PANCHANG_INCLUDE_MASA,
    PANCHANG_INCLUDE_NAKSHATRA, PANCHANG_INCLUDE_TITHI, PANCHANG_INCLUDE_VAAR,
    PANCHANG_INCLUDE_VARSHA, PANCHANG_INCLUDE_YOGA, PanchangOperation, PanchangResult, QueryMode,
    SankrantiOperation, SankrantiQuery, SankrantiResult, SankrantiTarget, TaraOperation,
    TaraOutputKind, TaraResult, ayanamsha, conjunction, grahan, lunar_node, lunar_phase, motion,
    panchang, sankranti, tara,
};
pub use panchang::{
    ayana_for_date, elongation_at, ghatika_for_date, ghatika_from_sunrises, hora_for_date,
    hora_from_sunrises, karana_at, karana_for_date, masa_for_date, moon_sidereal_longitude_at,
    nakshatra_at, nakshatra_for_date, panchang_for_date, sidereal_sum_at, tithi_at, tithi_for_date,
    vaar_for_date, vaar_from_sunrises, varsha_for_date, vedic_day_sunrises, yoga_at, yoga_for_date,
};
pub use panchang_types::{
    AyanaInfo, GhatikaInfo, HoraInfo, KaranaInfo, MasaInfo, PanchangInfo, PanchangNakshatraInfo,
    TithiInfo, VaarInfo, VarshaInfo, YogaInfo,
};
pub use sankranti::{
    next_sankranti, next_specific_sankranti, prev_sankranti, prev_specific_sankranti,
    search_sankrantis,
};
pub use sankranti_types::{SankrantiConfig, SankrantiEvent};
pub use search_util::{set_time_conversion_policy, time_conversion_policy};
pub use stationary::{
    next_max_speed, next_stationary, prev_max_speed, prev_stationary, search_max_speed,
    search_stationary,
};
pub use stationary_types::{
    MaxSpeedEvent, MaxSpeedType, StationType, StationaryConfig, StationaryEvent,
};
