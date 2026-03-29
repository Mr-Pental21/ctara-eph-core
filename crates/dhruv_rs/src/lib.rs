//! Context-first Rust facade for ctara-dhruv.
//!
//! High-level operations are executed through [`DhruvContext`], which owns an
//! engine and optional layered configuration resolver.

pub mod amsha;
pub mod context;
pub mod date;
pub mod error;
pub mod ops;

pub use amsha::{
    ALL, ALL_AMSHAS, Amsha, AmshaChart, AmshaChartScope, AmshaRequest, AmshaResult,
    AmshaSelectionConfig, AmshaVariation, RashiElement, SHODASHA, SHODASHAVARGA,
    amsha_from_rashi_position, amsha_longitude, amsha_longitudes, amsha_rashi_info,
    amsha_rashi_infos, chart as amsha_chart, chart_for_date as amsha_chart_for_date,
    charts as amsha_charts, charts_for_date as amsha_charts_for_date, longitude as amsha_lon,
    longitudes as amsha_lons, rashi_element, rashi_info as amsha_rashi,
    rashi_position_to_longitude,
};
pub use context::DhruvContext;
pub use date::UtcDate;
pub use error::DhruvError;
pub use ops::{
    AvasthaRequest, AvasthaResult, AvasthaTarget, AyanamshaRequest, AyanamshaRequestMode,
    CharakarakaRequest, ConjunctionRequest, ConjunctionRequestQuery, FullKundaliRequest,
    GrahanRequest, GrahanRequestQuery, LunarPhaseRequest, LunarPhaseRequestQuery, MotionRequest,
    MotionRequestQuery, NodeRequest, PanchangRequest, SankrantiRequest, SankrantiRequestQuery,
    TaraRequest, TimeInput, UpagrahaRequest, avastha_op, ayanamsha_op, charakaraka, conjunction,
    full_kundali, grahan, lunar_node_op, lunar_phase, motion, panchang_op, sankranti, tara_op,
    upagraha_op,
};

// Re-export core types so callers don't need to depend on dhruv_core directly.
pub use dhruv_core::{Body, EngineConfig, Frame, Observer, StateVector};

// Re-export commonly used config/result types.
pub use dhruv_frames::{ReferencePlane, SphericalCoords, SphericalState};
pub use dhruv_search::conjunction_types::{ConjunctionConfig, ConjunctionEvent};
pub use dhruv_search::grahan_types::{
    ChandraGrahan, ChandraGrahanType, GrahanConfig, SuryaGrahan, SuryaGrahanType,
};
pub use dhruv_search::sankranti_types::{SankrantiConfig, SankrantiEvent};
pub use dhruv_search::stationary_types::{
    MaxSpeedEvent, MaxSpeedType, StationType, StationaryConfig, StationaryEvent,
};
pub use dhruv_search::{
    BalaBundleResult, DashaSelectionConfig, FullKundaliConfig, FullKundaliResult, ShadbalaEntry,
    ShadbalaResult, VimsopakaEntry, VimsopakaResult, balas_for_date, bhavabala_for_bhava,
    bhavabala_for_date, full_kundali_for_date, shadbala_for_date, shadbala_for_graha,
    vimsopaka_for_date, vimsopaka_for_graha,
};
pub use dhruv_search::{
    ConjunctionResult, GrahanKind, GrahanResult, LunarPhaseKind, LunarPhaseResult, MotionKind,
    MotionResult, SankrantiResult, SankrantiTarget,
};
pub use dhruv_tara::{
    EarthState, EquatorialPosition, TaraAccuracy, TaraCatalog, TaraConfig, TaraError, TaraId,
};
pub use dhruv_time::{EopKernel, TimeConversionOptions, TimeConversionPolicy};
pub use dhruv_vedic_base::dasha::{
    DashaEntity, DashaHierarchy, DashaLevel, DashaPeriod, DashaSnapshot, DashaSystem,
    DashaVariationConfig, SubPeriodMethod, YoginiScheme,
};
pub use dhruv_vedic_base::riseset_types::{
    GeoLocation, RiseSetConfig, RiseSetEvent, RiseSetResult,
};
pub use dhruv_vedic_base::{
    AllGrahaAvasthas, AllUpagrahas, AshtakavargaResult, AyanamshaSystem, BhavaBalaBirthPeriod,
    BhavaBalaEntry, BhavaBalaInputs, BhavaBalaResult, BhavaConfig, BhinnaAshtakavarga,
    CharakarakaEntry, CharakarakaResult, CharakarakaRole, CharakarakaScheme, Graha, GrahaAvasthas,
    GulikaMaandiPlanet, LunarNode, NodeDignityPolicy, NodeMode, SarvaAshtakavarga,
    TimeUpagrahaConfig, TimeUpagrahaPoint, calculate_all_bav, calculate_ashtakavarga,
    calculate_bav, calculate_bhava_bala, calculate_sav,
};
pub use dhruv_vedic_ops::{
    NodeBackend, PANCHANG_INCLUDE_ALL, PANCHANG_INCLUDE_ALL_CALENDAR, PANCHANG_INCLUDE_ALL_CORE,
    PANCHANG_INCLUDE_AYANA, PANCHANG_INCLUDE_GHATIKA, PANCHANG_INCLUDE_HORA,
    PANCHANG_INCLUDE_KARANA, PANCHANG_INCLUDE_MASA, PANCHANG_INCLUDE_NAKSHATRA,
    PANCHANG_INCLUDE_TITHI, PANCHANG_INCLUDE_VAAR, PANCHANG_INCLUDE_VARSHA, PANCHANG_INCLUDE_YOGA,
    PanchangResult, SphutalResult, TaraOutputKind, TaraResult, dasha_child_period_for_birth,
    dasha_children_for_birth, dasha_complete_level_for_birth, dasha_hierarchy_for_birth,
    dasha_level0_entity_for_birth, dasha_level0_for_birth, dasha_snapshot_at,
};

#[cfg(test)]
mod tests {
    use super::calculate_bav;

    #[test]
    fn ashtakavarga_bav_includes_contributors() {
        let bav = calculate_bav(0, &[0, 1, 2, 3, 4, 5, 6], 0);
        for rashi in 0..12 {
            let row_sum: u8 = bav.contributors[rashi].iter().sum();
            assert_eq!(row_sum, bav.points[rashi]);
        }
    }
}
