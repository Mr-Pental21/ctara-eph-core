//! Convenience wrapper for the ctara-dhruv ephemeris engine.
//!
//! Provides a global singleton engine and high-level functions that accept
//! UTC dates directly, removing the need to manually manage `Engine` handles,
//! convert UTC→TDB, or transform Cartesian→spherical.
//!
//! # Quick start
//!
//! ```rust,ignore
//! use std::path::PathBuf;
//! use dhruv_rs::*;
//!
//! let config = EngineConfig::with_single_spk(
//!     PathBuf::from("kernels/data/de442s.bsp"),
//!     PathBuf::from("kernels/data/naif0012.tls"),
//!     256,
//!     true,
//! );
//! init(config).expect("engine init");
//!
//! let date: UtcDate = "2024-03-20T12:00:00Z".parse().unwrap();
//! let lon = longitude(Body::Mars, Observer::Body(Body::Earth), date).unwrap();
//! println!("Mars ecliptic longitude: {:.4}°", lon.to_degrees());
//! ```

pub mod convenience;
pub mod date;
pub mod error;
pub mod global;

// Primary re-exports — users should only need `use dhruv_rs::*`
pub use convenience::{
    all_rise_set_events, amsha_charts, approximate_local_noon_jd, arudha_pada, arudha_padas,
    ashtakavarga, avayoga_sphuta, ayana, ayana_from_sidereal_longitude, ayanamsha, beeja_sphuta,
    bhava_lagna, bhavas, bhrigu_bindu, body_ecliptic_lon_lat, calculate_all_bav,
    calculate_ashtakavarga, calculate_bav, calculate_sav, chatussphuta, core_bindus, deha_sphuta,
    drishti, ekadhipatya_sodhana, elongation_at, full_kundali, ghati_lagna, ghatika,
    ghatika_from_elapsed, ghatikas_since_sunrise, graha_drishti, graha_drishti_matrix,
    graha_longitudes, graha_positions, hora, hora_at, hora_lagna, indu_lagna, karana, karana_at,
    karana_from_elongation, kshetra_sphuta, kunda, lagna, longitude, lunar_node, masa,
    masa_from_rashi_index, mc, moon_nakshatra, mrityu_sphuta, nakshatra, nakshatra_at, nakshatra28,
    next_amavasya, next_chandra_grahan, next_conjunction, next_max_speed, next_purnima,
    next_sankranti, next_specific_sankranti, next_stationary, next_surya_grahan, normalize_360,
    nth_rashi_from, nutation, panchang, panchasphuta, position, position_full, prana_sphuta,
    pranapada_lagna, prev_amavasya, prev_chandra_grahan, prev_conjunction, prev_max_speed,
    prev_purnima, prev_sankranti, prev_specific_sankranti, prev_stationary, prev_surya_grahan,
    query, query_batch, rahu_tithi_sphuta, ramc, rashi, rashi_lord, samvatsara_from_year,
    search_amavasyas, search_chandra_grahan, search_conjunctions, search_max_speed,
    search_purnimas, search_sankrantis, search_stationary, search_surya_grahan, sidereal_longitude,
    sidereal_sum_at, sookshma_trisphuta, special_lagnas, sphutas, sree_lagna, sun_based_upagrahas,
    sunrise, sunset, tithi, tithi_at, tithi_from_elongation, tithi_sphuta, trikona_sodhana,
    trisphuta, upagrahas, vaar, vaar_from_jd, varnada_lagna, varsha, vedic_day_sunrises,
    vighati_lagna, yoga, yoga_at, yoga_from_sum, yoga_sphuta, yoga_sphuta_normalized,
};
pub use date::UtcDate;
pub use error::DhruvError;
pub use global::{init, is_initialized};

// Re-export core types so callers don't need to depend on dhruv_core directly.
pub use dhruv_core::{Body, EngineConfig, Frame, Observer, StateVector};
pub use dhruv_frames::{SphericalCoords, SphericalState};

// Re-export vedic types used by the convenience functions.
pub use dhruv_search::{
    AmshaChart, AmshaChartScope, AmshaEntry, AmshaResult, AmshaSelectionConfig, DrishtiConfig,
    DrishtiResult, FullKundaliConfig, FullKundaliResult, GrahaLongitudes,
};
pub use dhruv_vedic_base::riseset_types::GeoLocation;
pub use dhruv_vedic_base::riseset_types::{RiseSetConfig, RiseSetEvent, RiseSetResult, SunLimb};
pub use dhruv_vedic_base::{
    AllSpecialLagnas, AllUpagrahas, ArudhaPada, ArudhaResult, AshtakavargaResult,
    BhinnaAshtakavarga, Graha, SarvaAshtakavarga, SpecialLagna, Sphuta, SphutalInputs, Upagraha,
};
pub use dhruv_vedic_base::{
    Ayana as AyanaKind, AyanamshaSystem, Dms, GhatikaPosition, Hora as HoraLord,
    Karana as KaranaName, KaranaPosition, Masa as MasaName, Nakshatra as NakshatraName,
    Nakshatra28 as Nakshatra28Name, Nakshatra28Info, NakshatraInfo, Paksha, Rashi as RashiName,
    RashiInfo, Samvatsara as SamvatsaraName, SunBasedUpagrahas, Tithi as TithiName, TithiPosition,
    Vaar as VaarName, Yoga as YogaName, YogaPosition, deg_to_dms,
};
pub use dhruv_vedic_base::{
    Amsha, AmshaRequest, AmshaVariation, RashiElement,
};
pub use dhruv_vedic_base::{Bhava, BhavaConfig, BhavaResult, BhavaSystem, LunarNode, NodeMode};
pub use dhruv_vedic_base::{DrishtiEntry, GrahaDrishtiMatrix};

// Re-export EopKernel for sunrise-based panchang functions.
pub use dhruv_time::EopKernel;

// Re-export search result types used by convenience functions.
pub use dhruv_search::conjunction_types::{ConjunctionConfig, ConjunctionEvent};
pub use dhruv_search::grahan_types::{
    ChandraGrahan, ChandraGrahanType, GrahanConfig, SuryaGrahan, SuryaGrahanType,
};
pub use dhruv_search::lunar_phase_types::LunarPhaseEvent;
pub use dhruv_search::panchang_types::{
    AyanaInfo, GhatikaInfo, HoraInfo, KaranaInfo, MasaInfo, PanchangInfo, TithiInfo, VaarInfo,
    VarshaInfo, YogaInfo,
};
pub use dhruv_search::sankranti_types::{SankrantiConfig, SankrantiEvent};
pub use dhruv_search::stationary_types::{
    MaxSpeedEvent, MaxSpeedType, StationType, StationaryConfig, StationaryEvent,
};
