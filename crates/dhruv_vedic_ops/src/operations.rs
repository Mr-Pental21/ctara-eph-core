//! Canonical non-search operation APIs shared across wrappers and frontends.

use dhruv_core::Engine;
use dhruv_frames::SphericalCoords;
use dhruv_search::sankranti_types::SankrantiConfig;
use dhruv_tara::{
    EarthState, EquatorialPosition, TaraCatalog, TaraConfig, TaraError, TaraId,
    position_ecliptic_with_config, position_equatorial_with_config, sidereal_longitude_with_config,
};
use dhruv_time::{EopKernel, UtcTime};
use dhruv_vedic_base::{
    AyanamshaSystem, GeoLocation, LunarNode, NodeMode, RiseSetConfig, ayanamsha_deg,
    ayanamsha_mean_deg, ayanamsha_true_deg, jd_tdb_to_centuries, lunar_node_deg,
    lunar_node_deg_for_epoch,
};

use crate::error::SearchError;
use crate::panchang_types::{
    AyanaInfo, GhatikaInfo, HoraInfo, KaranaInfo, MasaInfo, PanchangNakshatraInfo, TithiInfo,
    VaarInfo, VarshaInfo, YogaInfo,
};
use crate::{
    ayana_for_date, ghatika_for_date, hora_for_date, karana_for_date, masa_for_date,
    nakshatra_for_date, panchang_for_date, tithi_for_date, vaar_for_date, varsha_for_date,
    yoga_for_date,
};

/// High-level query modes used across operation APIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryMode {
    /// Find the first result after a timestamp.
    Next,
    /// Find the first result before a timestamp.
    Prev,
    /// Find all results inside an interval.
    Range,
    /// Evaluate at one explicit date/time.
    AtDate,
}

/// Ayanamsha computation mode selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AyanamshaMode {
    /// Mean ayanamsha model (no nutation term).
    Mean,
    /// True ayanamsha from explicit delta-psi arcseconds.
    True,
    /// Unified ayanamsha (`use_nutation` flag controls mean/true behavior).
    Unified,
}

/// Canonical ayanamsha operation request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AyanamshaOperation {
    /// Ayanamsha system.
    pub system: AyanamshaSystem,
    /// Computation mode selector.
    pub mode: AyanamshaMode,
    /// Epoch as JD TDB.
    pub at_jd_tdb: f64,
    /// Nutation inclusion flag used by `Unified` mode.
    pub use_nutation: bool,
    /// Delta-psi arcseconds used by `True` mode.
    pub delta_psi_arcsec: f64,
}

/// Execute an ayanamsha operation request.
pub fn ayanamsha(op: &AyanamshaOperation) -> Result<f64, SearchError> {
    let t = jd_tdb_to_centuries(op.at_jd_tdb);
    let deg = match op.mode {
        AyanamshaMode::Mean => ayanamsha_mean_deg(op.system, t),
        AyanamshaMode::True => ayanamsha_true_deg(op.system, t, op.delta_psi_arcsec),
        AyanamshaMode::Unified => ayanamsha_deg(op.system, t, op.use_nutation),
    };
    Ok(deg)
}

/// Lunar-node backend selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeBackend {
    /// Analytic backend (`lunar_node_deg`) that does not use engine states.
    Analytic,
    /// Engine-backed backend (`lunar_node_deg_for_epoch`).
    Engine,
}

/// Canonical lunar-node operation request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeOperation {
    /// Rahu or Ketu selector.
    pub node: LunarNode,
    /// Mean or true node model.
    pub mode: NodeMode,
    /// Backend selector.
    pub backend: NodeBackend,
    /// Epoch as JD TDB.
    pub at_jd_tdb: f64,
}

/// Execute a lunar-node operation request.
pub fn lunar_node(engine: &Engine, op: &NodeOperation) -> Result<f64, SearchError> {
    match op.backend {
        NodeBackend::Analytic => {
            let t = jd_tdb_to_centuries(op.at_jd_tdb);
            Ok(lunar_node_deg(op.node, t, op.mode))
        }
        NodeBackend::Engine => Ok(lunar_node_deg_for_epoch(
            engine,
            op.node,
            op.at_jd_tdb,
            op.mode,
        )?),
    }
}

/// Include bit for Tithi in panchang operations.
pub const PANCHANG_INCLUDE_TITHI: u32 = 1 << 0;
/// Include bit for Karana in panchang operations.
pub const PANCHANG_INCLUDE_KARANA: u32 = 1 << 1;
/// Include bit for Yoga in panchang operations.
pub const PANCHANG_INCLUDE_YOGA: u32 = 1 << 2;
/// Include bit for Vaar in panchang operations.
pub const PANCHANG_INCLUDE_VAAR: u32 = 1 << 3;
/// Include bit for Hora in panchang operations.
pub const PANCHANG_INCLUDE_HORA: u32 = 1 << 4;
/// Include bit for Ghatika in panchang operations.
pub const PANCHANG_INCLUDE_GHATIKA: u32 = 1 << 5;
/// Include bit for Nakshatra in panchang operations.
pub const PANCHANG_INCLUDE_NAKSHATRA: u32 = 1 << 6;
/// Include bit for Masa in panchang operations.
pub const PANCHANG_INCLUDE_MASA: u32 = 1 << 7;
/// Include bit for Ayana in panchang operations.
pub const PANCHANG_INCLUDE_AYANA: u32 = 1 << 8;
/// Include bit for Varsha in panchang operations.
pub const PANCHANG_INCLUDE_VARSHA: u32 = 1 << 9;

/// Include mask containing all core daily panchang elements.
pub const PANCHANG_INCLUDE_ALL_CORE: u32 = PANCHANG_INCLUDE_TITHI
    | PANCHANG_INCLUDE_KARANA
    | PANCHANG_INCLUDE_YOGA
    | PANCHANG_INCLUDE_VAAR
    | PANCHANG_INCLUDE_HORA
    | PANCHANG_INCLUDE_GHATIKA
    | PANCHANG_INCLUDE_NAKSHATRA;

/// Include mask containing all calendar elements.
pub const PANCHANG_INCLUDE_ALL_CALENDAR: u32 =
    PANCHANG_INCLUDE_MASA | PANCHANG_INCLUDE_AYANA | PANCHANG_INCLUDE_VARSHA;

/// Include mask containing all panchang elements.
pub const PANCHANG_INCLUDE_ALL: u32 = PANCHANG_INCLUDE_ALL_CORE | PANCHANG_INCLUDE_ALL_CALENDAR;

/// Canonical panchang operation request.
#[derive(Debug, Clone, PartialEq)]
pub struct PanchangOperation {
    /// Input timestamp in UTC.
    pub at_utc: UtcTime,
    /// Observer location.
    pub location: GeoLocation,
    /// Sunrise/sunset model configuration.
    pub riseset_config: RiseSetConfig,
    /// Ayanamsha/search configuration.
    pub sankranti_config: SankrantiConfig,
    /// Include mask with `PANCHANG_INCLUDE_*` bits.
    pub include_mask: u32,
}

/// Canonical panchang operation response.
#[derive(Debug, Clone, PartialEq)]
pub struct PanchangResult {
    pub tithi: Option<TithiInfo>,
    pub karana: Option<KaranaInfo>,
    pub yoga: Option<YogaInfo>,
    pub vaar: Option<VaarInfo>,
    pub hora: Option<HoraInfo>,
    pub ghatika: Option<GhatikaInfo>,
    pub nakshatra: Option<PanchangNakshatraInfo>,
    pub masa: Option<MasaInfo>,
    pub ayana: Option<AyanaInfo>,
    pub varsha: Option<VarshaInfo>,
}

fn include(mask: u32, bit: u32) -> bool {
    (mask & bit) != 0
}

/// Execute a panchang operation request.
pub fn panchang(
    engine: &Engine,
    eop: &EopKernel,
    op: &PanchangOperation,
) -> Result<PanchangResult, SearchError> {
    if op.include_mask == 0 {
        return Err(SearchError::InvalidConfig("include_mask must be non-zero"));
    }

    let mut result = PanchangResult {
        tithi: None,
        karana: None,
        yoga: None,
        vaar: None,
        hora: None,
        ghatika: None,
        nakshatra: None,
        masa: None,
        ayana: None,
        varsha: None,
    };

    let any_core = (op.include_mask & PANCHANG_INCLUDE_ALL_CORE) != 0;
    let any_calendar = (op.include_mask & PANCHANG_INCLUDE_ALL_CALENDAR) != 0;

    if any_core {
        let full = panchang_for_date(
            engine,
            eop,
            &op.at_utc,
            &op.location,
            &op.riseset_config,
            &op.sankranti_config,
            any_calendar,
        )?;
        if include(op.include_mask, PANCHANG_INCLUDE_TITHI) {
            result.tithi = Some(full.tithi);
        }
        if include(op.include_mask, PANCHANG_INCLUDE_KARANA) {
            result.karana = Some(full.karana);
        }
        if include(op.include_mask, PANCHANG_INCLUDE_YOGA) {
            result.yoga = Some(full.yoga);
        }
        if include(op.include_mask, PANCHANG_INCLUDE_VAAR) {
            result.vaar = Some(full.vaar);
        }
        if include(op.include_mask, PANCHANG_INCLUDE_HORA) {
            result.hora = Some(full.hora);
        }
        if include(op.include_mask, PANCHANG_INCLUDE_GHATIKA) {
            result.ghatika = Some(full.ghatika);
        }
        if include(op.include_mask, PANCHANG_INCLUDE_NAKSHATRA) {
            result.nakshatra = Some(full.nakshatra);
        }
        if include(op.include_mask, PANCHANG_INCLUDE_MASA) {
            result.masa = full.masa;
        }
        if include(op.include_mask, PANCHANG_INCLUDE_AYANA) {
            result.ayana = full.ayana;
        }
        if include(op.include_mask, PANCHANG_INCLUDE_VARSHA) {
            result.varsha = full.varsha;
        }
        return Ok(result);
    }

    if include(op.include_mask, PANCHANG_INCLUDE_MASA) {
        result.masa = Some(masa_for_date(engine, &op.at_utc, &op.sankranti_config)?);
    }
    if include(op.include_mask, PANCHANG_INCLUDE_AYANA) {
        result.ayana = Some(ayana_for_date(engine, &op.at_utc, &op.sankranti_config)?);
    }
    if include(op.include_mask, PANCHANG_INCLUDE_VARSHA) {
        result.varsha = Some(varsha_for_date(engine, &op.at_utc, &op.sankranti_config)?);
    }
    if include(op.include_mask, PANCHANG_INCLUDE_TITHI) {
        result.tithi = Some(tithi_for_date(engine, &op.at_utc)?);
    }
    if include(op.include_mask, PANCHANG_INCLUDE_KARANA) {
        result.karana = Some(karana_for_date(engine, &op.at_utc)?);
    }
    if include(op.include_mask, PANCHANG_INCLUDE_YOGA) {
        result.yoga = Some(yoga_for_date(engine, &op.at_utc, &op.sankranti_config)?);
    }
    if include(op.include_mask, PANCHANG_INCLUDE_NAKSHATRA) {
        result.nakshatra = Some(nakshatra_for_date(
            engine,
            &op.at_utc,
            &op.sankranti_config,
        )?);
    }
    if include(op.include_mask, PANCHANG_INCLUDE_VAAR) {
        result.vaar = Some(vaar_for_date(
            engine,
            eop,
            &op.at_utc,
            &op.location,
            &op.riseset_config,
        )?);
    }
    if include(op.include_mask, PANCHANG_INCLUDE_HORA) {
        result.hora = Some(hora_for_date(
            engine,
            eop,
            &op.at_utc,
            &op.location,
            &op.riseset_config,
        )?);
    }
    if include(op.include_mask, PANCHANG_INCLUDE_GHATIKA) {
        result.ghatika = Some(ghatika_for_date(
            engine,
            eop,
            &op.at_utc,
            &op.location,
            &op.riseset_config,
        )?);
    }
    Ok(result)
}

/// Tara output selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaraOutputKind {
    /// ICRS equatorial position (RA/Dec/distance AU).
    Equatorial,
    /// Ecliptic-of-date spherical coordinates.
    Ecliptic,
    /// Sidereal longitude in degrees.
    Sidereal,
}

/// Canonical tara operation request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TaraOperation {
    /// Tara identifier.
    pub star: TaraId,
    /// Output selector.
    pub output: TaraOutputKind,
    /// Epoch as JD TDB.
    pub at_jd_tdb: f64,
    /// Ayanamsha in degrees (used by sidereal output).
    pub ayanamsha_deg: f64,
    /// Fixed-star computation configuration.
    pub config: TaraConfig,
    /// Optional Earth state for apparent/parallax modes.
    pub earth_state: Option<EarthState>,
}

/// Canonical tara operation response.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TaraResult {
    /// Equatorial output.
    Equatorial(EquatorialPosition),
    /// Ecliptic output.
    Ecliptic(SphericalCoords),
    /// Sidereal longitude in degrees.
    Sidereal(f64),
}

/// Execute a tara operation request.
pub fn tara(catalog: &TaraCatalog, op: &TaraOperation) -> Result<TaraResult, TaraError> {
    match op.output {
        TaraOutputKind::Equatorial => Ok(TaraResult::Equatorial(position_equatorial_with_config(
            catalog,
            op.star,
            op.at_jd_tdb,
            &op.config,
            op.earth_state.as_ref(),
        )?)),
        TaraOutputKind::Ecliptic => Ok(TaraResult::Ecliptic(position_ecliptic_with_config(
            catalog,
            op.star,
            op.at_jd_tdb,
            &op.config,
            op.earth_state.as_ref(),
        )?)),
        TaraOutputKind::Sidereal => Ok(TaraResult::Sidereal(sidereal_longitude_with_config(
            catalog,
            op.star,
            op.at_jd_tdb,
            op.ayanamsha_deg,
            &op.config,
            op.earth_state.as_ref(),
        )?)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_mode_at_date_is_stable() {
        assert_eq!(QueryMode::AtDate, QueryMode::AtDate);
    }

    #[test]
    fn ayanamsha_mode_is_stable() {
        let op = AyanamshaOperation {
            system: AyanamshaSystem::Lahiri,
            mode: AyanamshaMode::Mean,
            at_jd_tdb: 2_451_545.0,
            use_nutation: false,
            delta_psi_arcsec: 0.0,
        };
        assert!(ayanamsha(&op).is_ok());
    }

    #[test]
    fn node_backend_is_stable() {
        assert_eq!(NodeBackend::Analytic, NodeBackend::Analytic);
        assert_eq!(NodeBackend::Engine, NodeBackend::Engine);
    }

    #[test]
    fn panchang_include_mask_is_stable() {
        assert_eq!(PANCHANG_INCLUDE_ALL_CORE, 0x7f);
        assert_eq!(PANCHANG_INCLUDE_ALL_CALENDAR, 0x380);
        assert_eq!(PANCHANG_INCLUDE_ALL, 0x3ff);
    }
}
