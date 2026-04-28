use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use clap::{Parser, Subcommand, ValueEnum};
use dhruv_config::{ConfigResolver, DefaultsMode, EngineConfigPatch, load_with_discovery};
use dhruv_core::{Body, Engine, EngineConfig, Frame, Observer, Query};
use dhruv_frames::{
    PrecessionModel, ReferencePlane, cartesian_to_spherical, icrf_to_ecliptic, nutation_iau2000b,
    precess_ecliptic_j2000_to_date,
};
use dhruv_search::conjunction_types::{ConjunctionConfig, ConjunctionEvent};
use dhruv_search::grahan_types::GrahanConfig;
use dhruv_search::sankranti_types::SankrantiConfig;
use dhruv_search::stationary_types::StationaryConfig;
use dhruv_search::{
    ConjunctionOperation, ConjunctionQuery, ConjunctionResult, GrahanKind, GrahanOperation,
    GrahanQuery, GrahanResult, LunarPhaseKind, LunarPhaseOperation, LunarPhaseQuery,
    LunarPhaseResult, MotionKind, MotionOperation, MotionQuery, MotionResult, SankrantiOperation,
    SankrantiQuery, SankrantiResult, SankrantiTarget,
};
use dhruv_tara::{EarthState, TaraAccuracy, TaraCatalog, TaraConfig, TaraId};
use dhruv_time::{
    DeltaTModel, EopKernel, FutureDeltaTTransition, LeapSecondKernel, SmhFutureParabolaFamily,
    TimeConversionOptions, TimeConversionPolicy, TimeWarning, UtcTime, calendar_to_jd,
    jd_to_calendar, jd_to_tdb_seconds, tdb_seconds_to_jd,
};
use dhruv_vedic_base::bhava_types::SayanadiGhatikaRounding;
use dhruv_vedic_base::riseset_types::{GeoLocation, RiseSetConfig, RiseSetResult};
use dhruv_vedic_base::{
    ALL_GRAHAS, AyanamshaSystem, Graha, GulikaMaandiPlanet, LunarNode, NodeDignityPolicy, NodeMode,
    Rashi, TimeUpagrahaConfig, TimeUpagrahaPoint, ayanamsha_deg, ayanamsha_deg_with_catalog,
    ayanamsha_mean_deg_with_catalog, ayanamsha_true_deg, deg_to_dms, jd_tdb_to_centuries,
    nakshatra_from_longitude, nakshatra_from_tropical, nakshatra28_from_longitude,
    nakshatra28_from_tropical, rashi_from_longitude, rashi_from_tropical,
};
use dhruv_vedic_base::{BhavaConfig, ChandraBeneficRule};
use dhruv_vedic_ops::{
    NodeBackend, NodeOperation, PANCHANG_INCLUDE_ALL, PANCHANG_INCLUDE_ALL_CALENDAR,
    PANCHANG_INCLUDE_ALL_CORE, PANCHANG_INCLUDE_AYANA, PANCHANG_INCLUDE_GHATIKA,
    PANCHANG_INCLUDE_HORA, PANCHANG_INCLUDE_KARANA, PANCHANG_INCLUDE_MASA,
    PANCHANG_INCLUDE_NAKSHATRA, PANCHANG_INCLUDE_TITHI, PANCHANG_INCLUDE_VAAR,
    PANCHANG_INCLUDE_VARSHA, PANCHANG_INCLUDE_YOGA, PanchangOperation, TaraOperation,
    TaraOutputKind, TaraResult,
};

#[derive(Parser)]
#[command(name = "dhruv", about = "Dhruv ephemeris CLI")]
struct Cli {
    /// Optional explicit config file path (TOML or JSON)
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    /// Disable config file discovery and loading
    #[arg(long, global = true, default_value_t = false)]
    no_config: bool,
    /// Config defaults mode: recommended or none
    #[arg(long, global = true, default_value = "recommended")]
    defaults_mode: String,
    /// UTC->TDB conversion policy: strict-lsk or hybrid-deltat
    #[arg(long, global = true, default_value = "hybrid-deltat")]
    time_policy: String,
    /// Delta-T model for hybrid-deltat policy: legacy-em2006 or smh2016
    #[arg(long, global = true, default_value = "smh2016")]
    delta_t_model: String,
    /// SMH future parabola-family selector when post-EOP asymptotic fallback
    /// is active (hybrid-deltat + --future-delta-t-transition bridge-modern-endpoint).
    /// Values: addendum2020, c-20, c-17.52, c-15.32, stephenson1997,
    /// stephenson2016
    #[arg(long, global = true, default_value = "addendum2020")]
    smh_future_family: String,
    /// Future Delta-T transition strategy for UTC beyond LSK coverage.
    /// Values: legacy-tt-utc-blend, bridge-modern-endpoint.
    #[arg(long, global = true, default_value = "legacy-tt-utc-blend")]
    future_delta_t_transition: String,
    /// For hybrid-deltat policy: do not freeze DUT1 after EOP coverage.
    /// By default DUT1 is frozen to last known EOP value.
    #[arg(long, global = true, default_value_t = false)]
    no_freeze_future_dut1: bool,
    /// For hybrid-deltat policy: suppress fallback warnings in diagnostics/output.
    #[arg(long, global = true, default_value_t = false)]
    no_warn_on_fallback: bool,
    /// For hybrid-deltat policy: DUT1 fallback to use before EOP coverage (seconds).
    #[arg(long, global = true)]
    pre_range_dut1: Option<f64>,
    /// Transition window in years for blending from leap-table anchor TT-UTC
    /// to model fallback when `--future-delta-t-transition bridge-modern-endpoint` is active.
    #[arg(long, global = true)]
    future_transition_years: Option<f64>,
    /// Optional staleness warning threshold for LSK coverage end (days).
    /// Example: --stale-lsk-threshold-days 365
    #[arg(long, global = true)]
    stale_lsk_threshold_days: Option<f64>,
    /// Optional staleness warning threshold for EOP coverage end (days).
    /// Example: --stale-eop-threshold-days 30
    #[arg(long, global = true)]
    stale_eop_threshold_days: Option<f64>,
    /// Optional path to IERS C04 file for historical/final DUT1 backfill
    /// (typically `eopc04.1962-now`).
    #[arg(long, global = true)]
    eop_c04: Option<PathBuf>,
    /// Optional path to IERS daily finals file for fresher prediction tail
    /// (typically `finals2000A.daily.extended`).
    #[arg(long, global = true)]
    eop_daily: Option<PathBuf>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Args)]
struct RashiTropicalArgs {
    /// Tropical ecliptic longitude in degrees
    lon: f64,
    /// Ayanamsha system code (0-19)
    #[arg(long)]
    ayanamsha: i32,
    /// Julian Date TDB
    #[arg(long)]
    jd: f64,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
}

#[derive(clap::Args)]
struct NakshatraTropicalArgs {
    /// Tropical ecliptic longitude in degrees
    lon: f64,
    /// Ayanamsha system code (0-19)
    #[arg(long)]
    ayanamsha: i32,
    /// Julian Date TDB
    #[arg(long)]
    jd: f64,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
    /// Scheme: 27 (default) or 28
    #[arg(long, default_value = "27")]
    scheme: u32,
}

#[derive(clap::Args)]
struct NextSankrantiArgs {
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// Ayanamsha system code (0-19, default 0=Lahiri)
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct MasaArgs {
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// Ayanamsha system code (0-19, default 0=Lahiri)
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct AyanaArgs {
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// Ayanamsha system code (0-19, default 0=Lahiri)
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct VarshaArgs {
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// Ayanamsha system code (0-19, default 0=Lahiri)
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct YogaArgs {
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// Ayanamsha system code (0-19, default 0=Lahiri)
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct MoonNakshatraArgs {
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// Ayanamsha system code (0-19, default 0=Lahiri)
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct VaarArgs {
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// Latitude in degrees (north positive)
    #[arg(long)]
    lat: f64,
    /// Longitude in degrees (east positive)
    #[arg(long)]
    lon: f64,
    /// Altitude in meters (default 0)
    #[arg(long, default_value = "0")]
    alt: f64,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
    /// Path to IERS EOP file (finals2000A.all)
    #[arg(long)]
    eop: PathBuf,
    #[command(flatten)]
    bhava_behavior: BhavaBehaviorArgs,
}

#[derive(clap::Args)]
struct HoraArgs {
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// Latitude in degrees (north positive)
    #[arg(long)]
    lat: f64,
    /// Longitude in degrees (east positive)
    #[arg(long)]
    lon: f64,
    /// Altitude in meters (default 0)
    #[arg(long, default_value = "0")]
    alt: f64,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
    /// Path to IERS EOP file (finals2000A.all)
    #[arg(long)]
    eop: PathBuf,
}

#[derive(clap::Args)]
struct GhatikaArgs {
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// Latitude in degrees (north positive)
    #[arg(long)]
    lat: f64,
    /// Longitude in degrees (east positive)
    #[arg(long)]
    lon: f64,
    /// Altitude in meters (default 0)
    #[arg(long, default_value = "0")]
    alt: f64,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
    /// Path to IERS EOP file (finals2000A.all)
    #[arg(long)]
    eop: PathBuf,
}

#[derive(clap::Args)]
struct SphutasArgs {
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// Latitude in degrees (north positive)
    #[arg(long)]
    lat: f64,
    /// Longitude in degrees (east positive)
    #[arg(long)]
    lon: f64,
    /// Altitude in meters (default 0)
    #[arg(long, default_value = "0")]
    alt: f64,
    /// Ayanamsha system code (0-19, default 0=Lahiri)
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
    /// Path to IERS EOP file (finals2000A.all)
    #[arg(long)]
    eop: PathBuf,
}

#[derive(clap::Args)]
struct SpecialLagnasArgs {
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// Latitude in degrees (north positive)
    #[arg(long)]
    lat: f64,
    /// Longitude in degrees (east positive)
    #[arg(long)]
    lon: f64,
    /// Altitude in meters (default 0)
    #[arg(long, default_value = "0")]
    alt: f64,
    /// Ayanamsha system code (0-19, default 0=Lahiri)
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
    /// Path to IERS EOP file (finals2000A.all)
    #[arg(long)]
    eop: PathBuf,
}

#[derive(clap::Args)]
struct ArudhaPadasArgs {
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// Latitude in degrees (north positive)
    #[arg(long)]
    lat: f64,
    /// Longitude in degrees (east positive)
    #[arg(long)]
    lon: f64,
    /// Altitude in meters (default 0)
    #[arg(long, default_value = "0")]
    alt: f64,
    /// Ayanamsha system code (0-19, default 0=Lahiri)
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
    /// Path to IERS EOP file (finals2000A.all)
    #[arg(long)]
    eop: PathBuf,
    #[command(flatten)]
    bhava_behavior: BhavaBehaviorArgs,
}

#[derive(clap::Args)]
struct PanchangArgs {
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// Latitude in degrees (north positive)
    #[arg(long)]
    lat: f64,
    /// Longitude in degrees (east positive)
    #[arg(long)]
    lon: f64,
    /// Altitude in meters (default 0)
    #[arg(long, default_value = "0")]
    alt: f64,
    /// Ayanamsha system code (0-19, default 0=Lahiri)
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
    /// Include calendar elements (masa, ayana, varsha)
    #[arg(long)]
    calendar: bool,
    /// Include mask tokens (comma-separated):
    /// tithi,karana,yoga,vaar,hora,ghatika,nakshatra,masa,ayana,varsha,core,calendar,all
    #[arg(long)]
    include: Option<String>,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
    /// Path to IERS EOP file (finals2000A.all)
    #[arg(long)]
    eop: PathBuf,
    #[command(flatten)]
    bhava_behavior: BhavaBehaviorArgs,
}

#[derive(clap::Args)]
struct TimeUpagrahaArgs {
    /// Gulika point within its selected period: start, middle, or end
    #[arg(long, value_enum)]
    gulika_point: Option<TimeUpagrahaPointArg>,
    /// Maandi point within its selected period: start, middle, or end
    #[arg(long, value_enum)]
    maandi_point: Option<TimeUpagrahaPointArg>,
    /// Point for Kaala/Mrityu/Artha Prahara/Yama Ghantaka: start, middle, or end
    #[arg(long, value_enum)]
    other_upagraha_point: Option<TimeUpagrahaPointArg>,
    /// Planet period used for Gulika: rahu or saturn
    #[arg(long, value_enum)]
    gulika_planet: Option<GulikaMaandiPlanetArg>,
    /// Planet period used for Maandi: rahu or saturn
    #[arg(long, value_enum)]
    maandi_planet: Option<GulikaMaandiPlanetArg>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum TimeUpagrahaPointArg {
    Start,
    Middle,
    End,
}

impl From<TimeUpagrahaPointArg> for TimeUpagrahaPoint {
    fn from(value: TimeUpagrahaPointArg) -> Self {
        match value {
            TimeUpagrahaPointArg::Start => Self::Start,
            TimeUpagrahaPointArg::Middle => Self::Middle,
            TimeUpagrahaPointArg::End => Self::End,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum GulikaMaandiPlanetArg {
    Rahu,
    Saturn,
}

impl From<GulikaMaandiPlanetArg> for GulikaMaandiPlanet {
    fn from(value: GulikaMaandiPlanetArg) -> Self {
        match value {
            GulikaMaandiPlanetArg::Rahu => Self::Rahu,
            GulikaMaandiPlanetArg::Saturn => Self::Saturn,
        }
    }
}

#[derive(clap::Args)]
struct AshtakavargaArgs {
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// Latitude in degrees (north positive)
    #[arg(long)]
    lat: f64,
    /// Longitude in degrees (east positive)
    #[arg(long)]
    lon: f64,
    /// Altitude in meters (default 0)
    #[arg(long, default_value = "0")]
    alt: f64,
    /// Ayanamsha system code (0-19, default 0=Lahiri)
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
    /// Path to IERS EOP file (finals2000A.all)
    #[arg(long)]
    eop: PathBuf,
}

#[derive(clap::Args)]
struct UpagrahasArgs {
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// Latitude in degrees (north positive)
    #[arg(long)]
    lat: f64,
    /// Longitude in degrees (east positive)
    #[arg(long)]
    lon: f64,
    /// Altitude in meters (default 0)
    #[arg(long, default_value = "0")]
    alt: f64,
    /// Ayanamsha system code (0-19, default 0=Lahiri)
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
    /// Path to IERS EOP file (finals2000A.all)
    #[arg(long)]
    eop: PathBuf,
    #[command(flatten)]
    upagraha: TimeUpagrahaArgs,
    #[command(flatten)]
    bhava_behavior: BhavaBehaviorArgs,
}

#[derive(clap::Args)]
struct GrahaPositionsArgs {
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// Latitude in degrees (north positive)
    #[arg(long)]
    lat: f64,
    /// Longitude in degrees (east positive)
    #[arg(long)]
    lon: f64,
    /// Altitude in meters (default 0)
    #[arg(long, default_value = "0")]
    alt: f64,
    /// Ayanamsha system code (0-19, default 0=Lahiri)
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
    /// Include nakshatra and pada
    #[arg(long)]
    nakshatra: bool,
    /// Include lagna (ascendant)
    #[arg(long)]
    lagna: bool,
    /// Include outer planets (Uranus, Neptune, Pluto)
    #[arg(long, conflicts_with = "no_outer")]
    outer: bool,
    /// Suppress outer planets (Uranus, Neptune, Pluto)
    #[arg(long = "no-outer")]
    no_outer: bool,
    /// Include bhava placement
    #[arg(long)]
    bhava: bool,
    /// Output tropical (ecliptic-of-date) longitudes instead of sidereal
    #[arg(long, conflicts_with_all = ["nakshatra", "lagna", "outer", "no_outer", "bhava"])]
    tropical: bool,
    /// Precession model: vondrak2011 (default), iau2006, lieske1977, newcomb1895
    #[arg(long, default_value = "vondrak2011")]
    precession: String,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
    /// Path to IERS EOP file (finals2000A.all)
    #[arg(long)]
    eop: PathBuf,
    #[command(flatten)]
    bhava_behavior: BhavaBehaviorArgs,
}

#[derive(clap::Args)]
struct CoreBindusArgs {
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// Latitude in degrees (north positive)
    #[arg(long)]
    lat: f64,
    /// Longitude in degrees (east positive)
    #[arg(long)]
    lon: f64,
    /// Altitude in meters (default 0)
    #[arg(long, default_value = "0")]
    alt: f64,
    /// Ayanamsha system code (0-19, default 0=Lahiri)
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
    /// Include nakshatra and pada
    #[arg(long)]
    nakshatra: bool,
    /// Include bhava placement
    #[arg(long)]
    bhava: bool,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
    /// Path to IERS EOP file (finals2000A.all)
    #[arg(long)]
    eop: PathBuf,
    #[command(flatten)]
    upagraha: TimeUpagrahaArgs,
    #[command(flatten)]
    bhava_behavior: BhavaBehaviorArgs,
}

#[derive(clap::Args)]
struct DrishtiArgs {
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// Latitude in degrees (north positive)
    #[arg(long)]
    lat: f64,
    /// Longitude in degrees (east positive)
    #[arg(long)]
    lon: f64,
    /// Altitude in meters (default 0)
    #[arg(long, default_value = "0")]
    alt: f64,
    /// Ayanamsha system code (0-19, default 0=Lahiri)
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
    /// Include graha-to-bhava-cusp drishti
    #[arg(long)]
    bhava: bool,
    /// Include graha-to-lagna drishti
    #[arg(long)]
    lagna: bool,
    /// Include graha-to-core-bindus drishti
    #[arg(long)]
    bindus: bool,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
    /// Path to IERS EOP file (finals2000A.all)
    #[arg(long)]
    eop: PathBuf,
    #[command(flatten)]
    bhava_behavior: BhavaBehaviorArgs,
}

#[derive(clap::Args)]
struct BhavaBehaviorArgs {
    /// Use rashi-bhava/equal-house basis for bala and avastha calculations
    #[arg(long, conflicts_with = "use_configured_bhava_for_bala_avastha")]
    use_rashi_bhava_for_bala_avastha: bool,
    /// Use configured bhava-system basis for bala and avastha calculations
    #[arg(long)]
    use_configured_bhava_for_bala_avastha: bool,
    /// Include Rahu/Ketu incoming aspects in Shadbala Drik Bala and Bhava Bala Drishti Bala
    #[arg(long, conflicts_with = "exclude_node_aspects_for_drik_bala")]
    include_node_aspects_for_drik_bala: bool,
    /// Exclude Rahu/Ketu incoming aspects from Shadbala Drik Bala and Bhava Bala Drishti Bala
    #[arg(long)]
    exclude_node_aspects_for_drik_bala: bool,
    /// Include occupation and rising special rules in Bhava Bala totals
    #[arg(long, conflicts_with = "exclude_special_bhavabala_rules")]
    include_special_bhavabala_rules: bool,
    /// Exclude occupation and rising special rules from Bhava Bala totals
    #[arg(long)]
    exclude_special_bhavabala_rules: bool,
    /// Divide Guru/Buddh incoming aspects by 4 in Shadbala Drik Bala
    #[arg(long, conflicts_with = "add_full_guru_buddh_drishti_for_drik_bala")]
    divide_guru_buddh_drishti_by_4_for_drik_bala: bool,
    /// Add full signed Guru/Buddh incoming aspects in Shadbala Drik Bala
    #[arg(long)]
    add_full_guru_buddh_drishti_for_drik_bala: bool,
    /// Chandra benefic/malefic rule for Shadbala nature calculations
    #[arg(long, value_enum)]
    chandra_benefic_rule: Option<ChandraBeneficRuleArg>,
    /// Birth ghatika rounding for Sayanadi Avastha
    #[arg(long, value_enum)]
    sayanadi_ghatika_rounding: Option<SayanadiGhatikaRoundingArg>,
    /// Include rashi-bhava sibling result sections/columns
    #[arg(long, conflicts_with = "no_rashi_bhava_results")]
    include_rashi_bhava_results: bool,
    /// Suppress rashi-bhava sibling result sections/columns
    #[arg(long)]
    no_rashi_bhava_results: bool,
}

fn bhava_config_from_cli(args: &BhavaBehaviorArgs) -> BhavaConfig {
    let mut config = BhavaConfig::default();
    if args.use_configured_bhava_for_bala_avastha {
        config.use_rashi_bhava_for_bala_avastha = false;
    }
    if args.use_rashi_bhava_for_bala_avastha {
        config.use_rashi_bhava_for_bala_avastha = true;
    }
    if args.include_node_aspects_for_drik_bala {
        config.include_node_aspects_for_drik_bala = true;
    }
    if args.exclude_node_aspects_for_drik_bala {
        config.include_node_aspects_for_drik_bala = false;
    }
    if args.include_special_bhavabala_rules {
        config.include_special_bhavabala_rules = true;
    }
    if args.exclude_special_bhavabala_rules {
        config.include_special_bhavabala_rules = false;
    }
    if args.add_full_guru_buddh_drishti_for_drik_bala {
        config.divide_guru_buddh_drishti_by_4_for_drik_bala = false;
    }
    if args.divide_guru_buddh_drishti_by_4_for_drik_bala {
        config.divide_guru_buddh_drishti_by_4_for_drik_bala = true;
    }
    if let Some(rule) = args.chandra_benefic_rule {
        config.chandra_benefic_rule = rule.into();
    }
    if let Some(rounding) = args.sayanadi_ghatika_rounding {
        config.sayanadi_ghatika_rounding = rounding.into();
    }
    if args.no_rashi_bhava_results {
        config.include_rashi_bhava_results = false;
    }
    if args.include_rashi_bhava_results {
        config.include_rashi_bhava_results = true;
    }
    config
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum ChandraBeneficRuleArg {
    /// Benefic only when at least 72 degrees away from Surya
    #[value(name = "brightness-72", alias = "brightness72")]
    Brightness72,
    /// Benefic while Chandra is 0..=180 degrees ahead of Surya
    #[value(name = "waxing-180", alias = "waxing180")]
    Waxing180,
}

impl From<ChandraBeneficRuleArg> for ChandraBeneficRule {
    fn from(value: ChandraBeneficRuleArg) -> Self {
        match value {
            ChandraBeneficRuleArg::Brightness72 => Self::Brightness72,
            ChandraBeneficRuleArg::Waxing180 => Self::Waxing180,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum SayanadiGhatikaRoundingArg {
    /// Use completed ghatikas since sunrise
    Floor,
    /// Count any partial current ghatika
    #[value(alias = "ceiling")]
    Ceil,
}

impl From<SayanadiGhatikaRoundingArg> for SayanadiGhatikaRounding {
    fn from(value: SayanadiGhatikaRoundingArg) -> Self {
        match value {
            SayanadiGhatikaRoundingArg::Floor => Self::Floor,
            SayanadiGhatikaRoundingArg::Ceil => Self::Ceil,
        }
    }
}

fn rashi_bhava_result_from_lagna(lagna_deg: f64) -> dhruv_vedic_base::BhavaResult {
    let lagna = lagna_deg.rem_euclid(360.0);
    let lagna_rashi = (lagna / 30.0).floor() as u8;
    let degree_in_rashi = lagna % 30.0;
    let mut bhavas = [dhruv_vedic_base::Bhava {
        number: 0,
        cusp_deg: 0.0,
        start_deg: 0.0,
        end_deg: 0.0,
    }; 12];
    for i in 0..12 {
        let rashi = (lagna_rashi + i as u8) % 12;
        let start = f64::from(rashi) * 30.0;
        bhavas[i] = dhruv_vedic_base::Bhava {
            number: (i + 1) as u8,
            cusp_deg: (start + degree_in_rashi).rem_euclid(360.0),
            start_deg: start,
            end_deg: (start + 30.0).rem_euclid(360.0),
        };
    }
    dhruv_vedic_base::BhavaResult {
        bhavas,
        lagna_deg: lagna,
        mc_deg: bhavas[9].cusp_deg,
    }
}

fn print_bhava_table(title: &str, result: &dhruv_vedic_base::BhavaResult) {
    println!("{title}");
    println!(
        "  Lagna: {:.6}°  MC: {:.6}°\n",
        result.lagna_deg, result.mc_deg
    );
    println!(
        "{:>6} {:>10} {:>10} {:>10}",
        "Bhava", "Cusp", "Start", "End"
    );
    println!("{}", "-".repeat(40));
    for b in &result.bhavas {
        println!(
            "{:>6} {:>11.6}° {:>11.6}° {:>11.6}°",
            b.number, b.cusp_deg, b.start_deg, b.end_deg
        );
    }
}

#[derive(clap::Args)]
struct KundaliArgs {
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// Latitude in degrees (north positive)
    #[arg(long)]
    lat: f64,
    /// Longitude in degrees (east positive)
    #[arg(long)]
    lon: f64,
    /// Altitude in meters (default 0)
    #[arg(long, default_value = "0")]
    alt: f64,
    /// Ayanamsha system code (0-19, default 0=Lahiri)
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
    /// Path to IERS EOP file (finals2000A.all)
    #[arg(long)]
    eop: PathBuf,
    /// Comma-separated dasha systems (e.g. "vimshottari,chara")
    #[arg(long)]
    dasha_systems: Option<String>,
    /// Max dasha hierarchy depth (0-4, default 2)
    #[arg(long, default_value = "2")]
    dasha_max_level: u8,
    /// UTC datetime for dasha snapshot query
    #[arg(long)]
    dasha_snapshot_date: Option<String>,
    /// Include graha positions
    #[arg(long)]
    include_graha: bool,
    /// Include core bindus
    #[arg(long)]
    include_bindus: bool,
    /// Include drishti
    #[arg(long)]
    include_drishti: bool,
    /// Include ashtakavarga
    #[arg(long)]
    include_ashtakavarga: bool,
    /// Include upagrahas
    #[arg(long)]
    include_upagrahas: bool,
    /// Include special lagnas
    #[arg(long)]
    include_special_lagnas: bool,
    /// Suppress outer planets in root graha positions
    #[arg(long = "no-outer")]
    no_outer: bool,
    /// Include amsha (divisional charts)
    #[arg(long)]
    include_amshas: bool,
    /// Comma-separated amsha specs for kundali output, e.g. "D9,D10,D2:cancer-leo-only"
    #[arg(long)]
    amsha: Option<String>,
    /// Include bhava cusps inside amsha charts
    #[arg(long)]
    amsha_include_bhava_cusps: bool,
    /// Include arudha padas inside amsha charts
    #[arg(long)]
    amsha_include_arudha_padas: bool,
    /// Include upagrahas inside amsha charts
    #[arg(long)]
    amsha_include_upagrahas: bool,
    /// Include sphutas inside amsha charts
    #[arg(long)]
    amsha_include_sphutas: bool,
    /// Include special lagnas inside amsha charts
    #[arg(long)]
    amsha_include_special_lagnas: bool,
    /// Suppress outer planets inside amsha charts
    #[arg(long = "amsha-no-outer-planets")]
    amsha_no_outer_planets: bool,
    /// Include shadbala
    #[arg(long)]
    include_shadbala: bool,
    /// Include bhava bala
    #[arg(long)]
    include_bhavabala: bool,
    /// Include vimsopaka bala
    #[arg(long)]
    include_vimsopaka: bool,
    /// Include graha avasthas
    #[arg(long)]
    include_avastha: bool,
    /// Include charakaraka section
    #[arg(long)]
    include_charakaraka: bool,
    /// Charakaraka scheme used when charakaraka is included
    #[arg(long, default_value = "mixed-parashara")]
    charakaraka_scheme: String,
    /// Include panchang (tithi, karana, yoga, vaar, hora, ghatika, nakshatra)
    #[arg(long)]
    include_panchang: bool,
    /// Include calendar (masa, ayana, varsha). Implies --include-panchang
    #[arg(long)]
    include_calendar: bool,
    /// Node dignity policy: "sign-lord" (default) or "sama"
    #[arg(long)]
    node_policy: Option<String>,
    /// Enable all sections (except dasha, which requires --dasha-systems)
    #[arg(long)]
    all: bool,
    #[command(flatten)]
    upagraha: TimeUpagrahaArgs,
    #[command(flatten)]
    bhava_behavior: BhavaBehaviorArgs,
}

#[derive(clap::Args)]
struct AmshaChartArgs {
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// Latitude in degrees (north positive)
    #[arg(long)]
    lat: f64,
    /// Longitude in degrees (east positive)
    #[arg(long)]
    lon: f64,
    /// Altitude in meters (default 0)
    #[arg(long, default_value = "0")]
    alt: f64,
    /// Ayanamsha system code (0-19, default 0=Lahiri)
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
    /// Path to IERS EOP file (finals2000A.all)
    #[arg(long)]
    eop: PathBuf,
    /// Comma-separated amsha specs: D<n>[:variation], e.g. D9,D10,D2:cancer-leo-only
    #[arg(long)]
    amsha: String,
    /// Include bhava cusps inside amsha charts
    #[arg(long)]
    include_bhava_cusps: bool,
    /// Include arudha padas inside amsha charts
    #[arg(long)]
    include_arudha_padas: bool,
    /// Include upagrahas inside amsha charts
    #[arg(long)]
    include_upagrahas: bool,
    /// Include sphutas inside amsha charts
    #[arg(long)]
    include_sphutas: bool,
    /// Include special lagnas inside amsha charts
    #[arg(long)]
    include_special_lagnas: bool,
    /// Suppress outer planets inside amsha charts
    #[arg(long = "no-outer-planets")]
    no_outer_planets: bool,
    #[command(flatten)]
    bhava_behavior: BhavaBehaviorArgs,
}

#[derive(clap::Args)]
struct AmshaArgs {
    /// Sidereal longitude in degrees
    #[arg(long)]
    lon: f64,
    /// Comma-separated amsha specs: D<n>[:variation], e.g. D9,D10,D2:cancer-leo-only
    #[arg(long)]
    amsha: String,
    /// Which amsha transform shape to print
    #[arg(long, value_enum, default_value_t = AmshaOutputMode::Rashi)]
    output: AmshaOutputMode,
    /// Output format for batch or scripting use
    #[arg(long, value_enum, default_value_t = AmshaOutputFormat::Text)]
    format: AmshaOutputFormat,
}

#[derive(clap::Args)]
struct AmshaVariationsArgs {
    /// Comma-separated amsha codes, e.g. D2,D9,D10
    #[arg(long)]
    amsha: String,
    /// Output format for batch or scripting use
    #[arg(long, value_enum, default_value_t = AmshaOutputFormat::Text)]
    format: AmshaOutputFormat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum AmshaOutputMode {
    Longitude,
    Rashi,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum AmshaOutputFormat {
    Text,
    Tsv,
}

#[derive(clap::Args)]
struct PrevSankrantiArgs {
    #[arg(long)]
    date: String,
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    #[arg(long)]
    nutation: bool,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct SearchPurnimasArgs {
    #[arg(long)]
    start: String,
    #[arg(long)]
    end: String,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct SearchAmavasyasArgs {
    #[arg(long)]
    start: String,
    #[arg(long)]
    end: String,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct SearchSankrantisArgs {
    #[arg(long)]
    start: String,
    #[arg(long)]
    end: String,
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    #[arg(long)]
    nutation: bool,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct NextSpecificSankrantiArgs {
    #[arg(long)]
    date: String,
    /// Rashi index (0=Mesha .. 11=Meena)
    #[arg(long)]
    rashi: u8,
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    #[arg(long)]
    nutation: bool,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct PrevSpecificSankrantiArgs {
    #[arg(long)]
    date: String,
    /// Rashi index (0=Mesha .. 11=Meena)
    #[arg(long)]
    rashi: u8,
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    #[arg(long)]
    nutation: bool,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct AyanamshaComputeArgs {
    #[arg(long)]
    date: String,
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Mode: unified (default), mean, or true
    #[arg(long, value_parser = ["unified", "mean", "true"], default_value = "unified")]
    mode: String,
    #[arg(long)]
    nutation: bool,
    /// Delta-psi (arcsec) used by `--mode true`
    #[arg(long, default_value_t = 0.0)]
    delta_psi_arcsec: f64,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
    /// Optional star catalog for proper-motion-corrected anchors
    #[arg(long)]
    catalog: Option<PathBuf>,
}

#[derive(clap::Args)]
struct SunriseArgs {
    #[arg(long)]
    date: String,
    #[arg(long)]
    lat: f64,
    #[arg(long)]
    lon: f64,
    #[arg(long, default_value = "0")]
    alt: f64,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
    #[arg(long)]
    eop: PathBuf,
}

#[derive(clap::Args)]
struct BhavasArgs {
    #[arg(long)]
    date: String,
    #[arg(long)]
    lat: f64,
    #[arg(long)]
    lon: f64,
    #[arg(long, default_value = "0")]
    alt: f64,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
    #[arg(long)]
    eop: PathBuf,
    /// Return bhava cusps, lagna, and MC on the configured sidereal zodiac.
    #[arg(long)]
    sidereal: bool,
    /// Ayanamsha system code used with `--sidereal` (0-19, default 0=Lahiri).
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation when computing sidereal output.
    #[arg(long)]
    nutation: bool,
    #[command(flatten)]
    bhava_behavior: BhavaBehaviorArgs,
}

#[derive(clap::Args)]
struct LagnaComputeArgs {
    #[arg(long)]
    date: String,
    #[arg(long)]
    lat: f64,
    #[arg(long)]
    lon: f64,
    #[arg(long, default_value = "0")]
    alt: f64,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
    #[arg(long)]
    eop: PathBuf,
    /// Return lagna and MC on the configured sidereal zodiac.
    #[arg(long)]
    sidereal: bool,
    /// Ayanamsha system code used with `--sidereal` (0-19, default 0=Lahiri).
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation when computing sidereal output.
    #[arg(long)]
    nutation: bool,
    #[command(flatten)]
    bhava_behavior: BhavaBehaviorArgs,
}

#[derive(clap::Args)]
struct LunarNodeArgs {
    #[arg(long)]
    date: String,
    /// Node: rahu or ketu
    #[arg(long, default_value = "rahu")]
    node: String,
    /// Mode: mean or true
    #[arg(long, default_value = "true")]
    mode: String,
    /// Backend: engine (default) or analytic
    #[arg(long, value_parser = ["engine", "analytic"], default_value = "engine")]
    backend: String,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct ConjunctionOpArgs {
    /// Mode: next, prev, or range
    #[arg(long, value_parser = ["next", "prev", "range"])]
    mode: String,
    /// UTC datetime for next/prev mode (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: Option<String>,
    /// UTC start datetime for range mode (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    start: Option<String>,
    /// UTC end datetime for range mode (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    end: Option<String>,
    /// NAIF body code for first body (e.g. 10=Sun, 301=Moon)
    #[arg(long)]
    body1: i32,
    /// NAIF body code for second body
    #[arg(long)]
    body2: i32,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct NextConjunctionArgs {
    #[arg(long)]
    date: String,
    /// NAIF body code for first body (e.g. 10=Sun, 301=Moon)
    #[arg(long)]
    body1: i32,
    /// NAIF body code for second body
    #[arg(long)]
    body2: i32,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct PrevConjunctionArgs {
    #[arg(long)]
    date: String,
    #[arg(long)]
    body1: i32,
    #[arg(long)]
    body2: i32,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct SearchConjunctionsArgs {
    #[arg(long)]
    start: String,
    #[arg(long)]
    end: String,
    #[arg(long)]
    body1: i32,
    #[arg(long)]
    body2: i32,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct GrahanOpArgs {
    /// Grahan kind: chandra or surya
    #[arg(long, value_parser = ["chandra", "surya"])]
    kind: String,
    /// Mode: next, prev, or range
    #[arg(long, value_parser = ["next", "prev", "range"])]
    mode: String,
    /// UTC datetime for next/prev mode (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: Option<String>,
    /// UTC start datetime for range mode (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    start: Option<String>,
    /// UTC end datetime for range mode (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    end: Option<String>,
    /// Exclude penumbral-only chandra grahan from results
    #[arg(long, default_value_t = false)]
    no_penumbral: bool,
    /// Exclude peak-detail fields in results
    #[arg(long, default_value_t = false)]
    no_peak_details: bool,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct LunarPhaseOpArgs {
    /// Lunar phase kind: amavasya or purnima
    #[arg(long, value_parser = ["amavasya", "purnima"])]
    kind: String,
    /// Mode: next, prev, or range
    #[arg(long, value_parser = ["next", "prev", "range"])]
    mode: String,
    /// UTC datetime for next/prev mode (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: Option<String>,
    /// UTC start datetime for range mode (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    start: Option<String>,
    /// UTC end datetime for range mode (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    end: Option<String>,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct SankrantiOpArgs {
    /// Mode: next, prev, or range
    #[arg(long, value_parser = ["next", "prev", "range"])]
    mode: String,
    /// UTC datetime for next/prev mode (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: Option<String>,
    /// UTC start datetime for range mode (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    start: Option<String>,
    /// UTC end datetime for range mode (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    end: Option<String>,
    /// Optional specific rashi index (0=Mesha .. 11=Meena)
    #[arg(long)]
    rashi: Option<i32>,
    /// Ayanamsha system code (0-19, default 0=Lahiri)
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct SearchChandraGrahanArgs {
    #[arg(long)]
    start: String,
    #[arg(long)]
    end: String,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct SearchSuryaGrahanArgs {
    #[arg(long)]
    start: String,
    #[arg(long)]
    end: String,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct NextStationaryArgs {
    #[arg(long)]
    date: String,
    /// NAIF body code (e.g. 499=Mars, 599=Jupiter)
    #[arg(long)]
    body: i32,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct PrevStationaryArgs {
    #[arg(long)]
    date: String,
    #[arg(long)]
    body: i32,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct SearchStationaryArgs {
    #[arg(long)]
    start: String,
    #[arg(long)]
    end: String,
    #[arg(long)]
    body: i32,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct NextMaxSpeedArgs {
    #[arg(long)]
    date: String,
    #[arg(long)]
    body: i32,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct PrevMaxSpeedArgs {
    #[arg(long)]
    date: String,
    #[arg(long)]
    body: i32,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct SearchMaxSpeedArgs {
    #[arg(long)]
    start: String,
    #[arg(long)]
    end: String,
    #[arg(long)]
    body: i32,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct MotionOpArgs {
    /// Motion kind: stationary or max-speed
    #[arg(long, value_parser = ["stationary", "max-speed"])]
    kind: String,
    /// Mode: next, prev, or range
    #[arg(long, value_parser = ["next", "prev", "range"])]
    mode: String,
    /// NAIF body code (e.g. 499=Mars, 599=Jupiter)
    #[arg(long)]
    body: i32,
    /// UTC datetime for next/prev mode (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: Option<String>,
    /// UTC start datetime for range mode (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    start: Option<String>,
    /// UTC end datetime for range mode (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    end: Option<String>,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct PositionArgs {
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// NAIF body code for target (e.g. 10=Sun, 301=Moon, 499=Mars)
    #[arg(long)]
    target: i32,
    /// NAIF body code for observer (0=SSB, 399=Earth)
    #[arg(long, default_value = "399")]
    observer: i32,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct SiderealLongitudeArgs {
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// NAIF body code for target
    #[arg(long)]
    target: i32,
    /// NAIF body code for observer (default 399=Earth)
    #[arg(long, default_value = "399")]
    observer: i32,
    /// Ayanamsha system code (0-19, default 0=Lahiri)
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct GrahaLongitudesArgs {
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// Ayanamsha system code (0-19, default 0=Lahiri)
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
    /// Output reference-plane longitudes without ayanamsha subtraction
    #[arg(long)]
    tropical: bool,
    /// Precession model: vondrak2011 (default), iau2006, lieske1977, newcomb1895
    #[arg(long, default_value = "vondrak2011")]
    precession: String,
    /// Reference plane: default, ecliptic, invariable
    #[arg(long, default_value = "default")]
    reference_plane: String,
    /// Suppress outer planets (Uranus, Neptune, Pluto)
    #[arg(long = "no-outer")]
    no_outer: bool,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct OsculatingApogeeArgs {
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// Comma-separated grahas: Mangal,Buddh,Guru,Shukra,Shani
    #[arg(long)]
    graha: String,
    /// Ayanamsha system code (0-19, default 0=Lahiri)
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
    /// Precession model: vondrak2011 (default), iau2006, lieske1977, newcomb1895
    #[arg(long, default_value = "vondrak2011")]
    precession: String,
    /// Reference plane: default, ecliptic, invariable
    #[arg(long, default_value = "default")]
    reference_plane: String,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct KshetraSphutaArgs {
    #[arg(long)]
    venus: f64,
    #[arg(long)]
    moon: f64,
    #[arg(long)]
    mars: f64,
    #[arg(long)]
    jupiter: f64,
    #[arg(long)]
    lagna: f64,
}

#[derive(clap::Args)]
struct SookshmaTrisphutaArgs {
    #[arg(long)]
    lagna: f64,
    #[arg(long)]
    moon: f64,
    #[arg(long)]
    gulika: f64,
    #[arg(long)]
    sun: f64,
}

#[derive(clap::Args)]
struct SiderealSumAtArgs {
    #[arg(long)]
    date: String,
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    #[arg(long)]
    nutation: bool,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct BodyLonLatArgs {
    #[arg(long)]
    date: String,
    /// NAIF body code
    #[arg(long)]
    body: i32,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct VedicDaySunrisesArgs {
    #[arg(long)]
    date: String,
    #[arg(long)]
    lat: f64,
    #[arg(long)]
    lon: f64,
    #[arg(long, default_value = "0")]
    alt: f64,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
    #[arg(long)]
    eop: PathBuf,
}

#[derive(clap::Args)]
struct TithiAtArgs {
    #[arg(long)]
    date: String,
    /// Pre-computed elongation in degrees
    #[arg(long)]
    elongation: f64,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct KaranaAtArgs {
    #[arg(long)]
    date: String,
    #[arg(long)]
    elongation: f64,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct YogaAtArgs {
    #[arg(long)]
    date: String,
    /// Pre-computed sidereal sum in degrees
    #[arg(long)]
    sum: f64,
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    #[arg(long)]
    nutation: bool,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct NakshatraAtArgs {
    #[arg(long)]
    date: String,
    /// Moon sidereal longitude in degrees
    #[arg(long)]
    moon_sid: f64,
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    #[arg(long)]
    nutation: bool,
    #[arg(long)]
    bsp: Option<PathBuf>,
    #[arg(long)]
    lsk: Option<PathBuf>,
}

#[derive(clap::Args)]
struct ShadbalaArgs {
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// Latitude in degrees (north positive)
    #[arg(long)]
    lat: f64,
    /// Longitude in degrees (east positive)
    #[arg(long)]
    lon: f64,
    /// Altitude in meters (default 0)
    #[arg(long, default_value = "0")]
    alt: f64,
    /// Ayanamsha system code (0-19, default 0=Lahiri)
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
    /// Optional graha filter (Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn)
    #[arg(long)]
    graha: Option<String>,
    /// Amsha selection list (e.g. D9,D10,D2:cancer-leo-only)
    #[arg(long)]
    amsha: Option<String>,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
    /// Path to IERS EOP file (finals2000A.all)
    #[arg(long)]
    eop: PathBuf,
    #[command(flatten)]
    bhava_behavior: BhavaBehaviorArgs,
}

#[derive(clap::Args)]
struct BhavabalaArgs {
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// Latitude in degrees (north positive)
    #[arg(long)]
    lat: f64,
    /// Longitude in degrees (east positive)
    #[arg(long)]
    lon: f64,
    /// Altitude in meters (default 0)
    #[arg(long, default_value = "0")]
    alt: f64,
    /// Ayanamsha system code (0-19, default 0=Lahiri)
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
    /// Optional house filter (1-12)
    #[arg(long)]
    bhava: Option<u8>,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
    /// Path to IERS EOP file (finals2000A.all)
    #[arg(long)]
    eop: PathBuf,
    #[command(flatten)]
    bhava_behavior: BhavaBehaviorArgs,
}

#[derive(clap::Args)]
struct VimsopakaArgs {
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// Latitude in degrees (north positive)
    #[arg(long)]
    lat: f64,
    /// Longitude in degrees (east positive)
    #[arg(long)]
    lon: f64,
    /// Altitude in meters (default 0)
    #[arg(long, default_value = "0")]
    alt: f64,
    /// Ayanamsha system code (0-19, default 0=Lahiri)
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
    /// Optional graha filter (Sun..Ketu)
    #[arg(long)]
    graha: Option<String>,
    /// Amsha selection list (e.g. D9,D10,D2:cancer-leo-only)
    #[arg(long)]
    amsha: Option<String>,
    /// Node dignity policy: sign-lord (default) or sama
    #[arg(long, default_value = "sign-lord")]
    node_policy: String,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
    /// Path to IERS EOP file (finals2000A.all)
    #[arg(long)]
    eop: PathBuf,
}

#[derive(clap::Args)]
struct BalasArgs {
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// Latitude in degrees (north positive)
    #[arg(long)]
    lat: f64,
    /// Longitude in degrees (east positive)
    #[arg(long)]
    lon: f64,
    /// Altitude in meters (default 0)
    #[arg(long, default_value = "0")]
    alt: f64,
    /// Ayanamsha system code (0-19, default 0=Lahiri)
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
    /// Node dignity policy: sign-lord (default) or sama
    #[arg(long, default_value = "sign-lord")]
    node_policy: String,
    /// Amsha selection list (e.g. D9,D10,D2:cancer-leo-only)
    #[arg(long)]
    amsha: Option<String>,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
    /// Path to IERS EOP file (finals2000A.all)
    #[arg(long)]
    eop: PathBuf,
    #[command(flatten)]
    bhava_behavior: BhavaBehaviorArgs,
}

#[derive(clap::Args)]
struct AvasthaArgs {
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// Latitude in degrees (north positive)
    #[arg(long)]
    lat: f64,
    /// Longitude in degrees (east positive)
    #[arg(long)]
    lon: f64,
    /// Altitude in meters (default 0)
    #[arg(long, default_value = "0")]
    alt: f64,
    /// Ayanamsha system code (0-19, default 0=Lahiri)
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
    /// Optional graha filter (Sun..Ketu)
    #[arg(long)]
    graha: Option<String>,
    /// Amsha selection list (e.g. D9,D10,D2:cancer-leo-only)
    #[arg(long)]
    amsha: Option<String>,
    /// Node dignity policy: sign-lord (default) or sama
    #[arg(long, default_value = "sign-lord")]
    node_policy: String,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
    /// Path to IERS EOP file (finals2000A.all)
    #[arg(long)]
    eop: PathBuf,
    #[command(flatten)]
    bhava_behavior: BhavaBehaviorArgs,
}

#[derive(clap::Args)]
struct CharakarakaArgs {
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// Ayanamsha system code (0-19, default 0=Lahiri)
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
    /// Charakaraka scheme: eight, seven-no-pitri, seven-pk-merged-mk, mixed-parashara
    #[arg(long, default_value = "mixed-parashara")]
    scheme: String,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
    /// Path to IERS EOP file (finals2000A.all)
    #[arg(long)]
    eop: PathBuf,
}

#[derive(clap::Args)]
struct DashaArgs {
    /// Dasha system (vimshottari)
    #[arg(long, default_value = "vimshottari")]
    system: String,
    /// Dasha mode: hierarchy, snapshot, level0, level0-entity, children, child-period, complete-level
    #[arg(long)]
    mode: Option<String>,
    /// Birth UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    birth_date: Option<String>,
    /// Birth JD UTC. Useful when supplying raw dasha inputs directly.
    #[arg(long)]
    birth_jd: Option<f64>,
    /// Query UTC datetime for snapshot mode (omit for hierarchy-only)
    #[arg(long)]
    query_date: Option<String>,
    /// Query JD UTC for snapshot mode.
    #[arg(long)]
    query_jd: Option<f64>,
    /// Latitude in degrees (north positive)
    #[arg(long)]
    lat: Option<f64>,
    /// Longitude in degrees (east positive)
    #[arg(long)]
    lon: Option<f64>,
    /// Altitude in meters (default 0 when location is provided)
    #[arg(long)]
    alt: Option<f64>,
    /// Precomputed Moon sidereal longitude in degrees.
    #[arg(long)]
    moon_sid_lon: Option<f64>,
    /// Precomputed graha sidereal longitudes as 9 comma-separated degrees.
    #[arg(long)]
    graha_sidereal_lons: Option<String>,
    /// Precomputed sidereal lagna longitude in degrees.
    #[arg(long)]
    lagna_sidereal_lon: Option<f64>,
    /// Precomputed sunrise JD UTC for Kala/Chakra-derived inputs.
    #[arg(long)]
    sunrise_jd: Option<f64>,
    /// Precomputed sunset JD UTC for Kala/Chakra-derived inputs.
    #[arg(long)]
    sunset_jd: Option<f64>,
    /// Maximum dasha depth (0-4, default 2)
    #[arg(long, default_value = "2")]
    max_level: u8,
    /// Parent level index for children/child-period/complete-level (0-4)
    #[arg(long)]
    parent_level: Option<u8>,
    /// Parent index within the selected level for children/child-period
    #[arg(long)]
    parent_index: Option<u32>,
    /// Entity selector for level0-entity/child-period: graha:0..8, rashi:0..11, yogini:0..7
    #[arg(long)]
    entity: Option<String>,
    /// Ayanamsha system code (0-19, default 0=Lahiri)
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
    /// Path to SPK kernel
    #[arg(long)]
    bsp: Option<PathBuf>,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
    /// Path to IERS EOP file (finals2000A.all)
    #[arg(long)]
    eop: PathBuf,
}

#[derive(clap::Args)]
struct TaraPositionArgs {
    /// Star name (e.g., "Chitra", "Arcturus")
    #[arg(long)]
    star: String,
    /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    date: String,
    /// Path to star catalog JSON
    #[arg(long)]
    catalog: PathBuf,
    /// Path to leap second kernel
    #[arg(long)]
    lsk: Option<PathBuf>,
    /// Ayanamsha system code (0-19, for sidereal output)
    #[arg(long, default_value = "0")]
    ayanamsha: i32,
    /// Apply nutation correction
    #[arg(long)]
    nutation: bool,
    /// Use Apparent accuracy tier (requires --bsp for Earth state)
    #[arg(long)]
    apparent: bool,
    /// Apply parallax correction (requires --bsp for Earth state)
    #[arg(long)]
    parallax: bool,
    /// Path to SPK kernel (required for --apparent or --parallax)
    #[arg(long)]
    bsp: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum GrahaHelperOp {
    HoraLord,
    MasaLord,
    SamvatsaraLord,
    ExaltationDegree,
    DebilitationDegree,
    MoolatrikoneRange,
    CombustionThreshold,
    IsCombust,
    AllCombustionStatus,
    NaisargikaMaitri,
    TatkalikaMaitri,
    PanchadhaMaitri,
    DignityInRashi,
    DignityInRashiWithPositions,
    NodeDignityInRashi,
    NaturalBeneficMalefic,
    MoonBeneficNature,
    GrahaGender,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum TimeUtilityOp {
    AyanamshaSystemCount,
    ReferencePlaneDefault,
    ApproximateLocalNoon,
    MonthFromAbbrev,
    CalendarToJd,
    JdToCalendar,
    MeanObliquityOfDateArcsec,
    MeanObliquityOfDateRad,
    IcrfToReferencePlane,
    EclipticToInvariable,
    InvariableToEcliptic,
    EclipticLonToInvariableLon,
    InvariableLonToEclipticLon,
    PrecessEclipticJ2000ToDate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum TaraPrimitiveOp {
    PropagatePosition,
    ApplyAberration,
    ApplyLightDeflection,
    GalacticAnticenterIcrs,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum CliNaisargikaArg {
    Friend,
    Enemy,
    Neutral,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum CliTatkalikaArg {
    Friend,
    Enemy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum CliNodeArg {
    Rahu,
    Ketu,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum CliNodePolicyArg {
    SignLord,
    Sama,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum CliCalendarPolicyArg {
    ProlepticGregorian,
    GregorianCutover1582,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum CliReferencePlaneArg {
    Ecliptic,
    Invariable,
}

#[derive(clap::Args)]
struct GrahaHelperArgs {
    #[arg(long, value_enum)]
    op: GrahaHelperOp,
    #[arg(long)]
    graha: Option<u8>,
    #[arg(long)]
    other: Option<u8>,
    #[arg(long)]
    vaar: Option<u8>,
    #[arg(long)]
    hora_index: Option<u8>,
    #[arg(long)]
    masa: Option<u8>,
    #[arg(long)]
    samvatsara: Option<u8>,
    #[arg(long)]
    rashi: Option<u8>,
    #[arg(long)]
    other_rashi: Option<u8>,
    #[arg(long)]
    sidereal_lon: Option<f64>,
    #[arg(long)]
    sun_lon: Option<f64>,
    #[arg(long)]
    retrograde: bool,
    #[arg(long)]
    longitudes: Option<String>,
    #[arg(long)]
    retrograde_flags: Option<String>,
    #[arg(long)]
    all_rashi_indices_7: Option<String>,
    #[arg(long)]
    all_rashi_indices_9: Option<String>,
    #[arg(long, value_enum)]
    naisargika: Option<CliNaisargikaArg>,
    #[arg(long, value_enum)]
    tatkalika: Option<CliTatkalikaArg>,
    #[arg(long, value_enum)]
    node: Option<CliNodeArg>,
    #[arg(long, value_enum, default_value = "sign-lord")]
    node_policy: CliNodePolicyArg,
    #[arg(long)]
    moon_sun_elongation: Option<f64>,
}

#[derive(clap::Args)]
struct TimeUtilityArgs {
    #[arg(long, value_enum)]
    op: TimeUtilityOp,
    #[arg(long)]
    ayanamsha: Option<i32>,
    #[arg(long)]
    jd_ut_midnight: Option<f64>,
    #[arg(long)]
    longitude_deg: Option<f64>,
    #[arg(long)]
    month_abbrev: Option<String>,
    #[arg(long)]
    year: Option<i32>,
    #[arg(long)]
    month: Option<u32>,
    #[arg(long)]
    day: Option<f64>,
    #[arg(long, value_enum, default_value = "proleptic-gregorian")]
    calendar_policy: CliCalendarPolicyArg,
    #[arg(long)]
    jd: Option<f64>,
    /// Comma-separated vector x,y,z
    #[arg(long)]
    vector: Option<String>,
    #[arg(long, value_enum)]
    reference_plane: Option<CliReferencePlaneArg>,
    #[arg(long)]
    t_centuries: Option<f64>,
    #[arg(long)]
    precession: Option<String>,
}

#[derive(clap::Args)]
struct TaraPrimitiveArgs {
    #[arg(long, value_enum)]
    op: TaraPrimitiveOp,
    #[arg(long)]
    ra_deg: Option<f64>,
    #[arg(long)]
    dec_deg: Option<f64>,
    #[arg(long)]
    parallax_mas: Option<f64>,
    #[arg(long)]
    pm_ra_mas_yr: Option<f64>,
    #[arg(long)]
    pm_dec_mas_yr: Option<f64>,
    #[arg(long)]
    rv_km_s: Option<f64>,
    #[arg(long)]
    dt_years: Option<f64>,
    /// Comma-separated vector x,y,z
    #[arg(long)]
    direction: Option<String>,
    /// Comma-separated vector x,y,z
    #[arg(long)]
    earth_velocity: Option<String>,
    /// Comma-separated vector x,y,z
    #[arg(long)]
    earth_position: Option<String>,
    #[arg(long)]
    observer_sun_distance_au: Option<f64>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show effective resolved configuration (debug utility)
    ConfigShowEffective,
    /// Rashi from sidereal longitude
    Rashi {
        /// Sidereal ecliptic longitude in degrees
        lon: f64,
    },
    /// Nakshatra from sidereal longitude
    Nakshatra {
        /// Sidereal ecliptic longitude in degrees
        lon: f64,
        /// Scheme: 27 (default) or 28
        #[arg(long, default_value = "27")]
        scheme: u32,
    },
    /// Rashi from tropical longitude + ayanamsha
    RashiTropical(RashiTropicalArgs),
    /// Nakshatra from tropical longitude + ayanamsha
    NakshatraTropical(NakshatraTropicalArgs),
    /// Convert degrees to DMS
    Dms {
        /// Angle in decimal degrees
        deg: f64,
    },
    /// Find next Purnima (full moon)
    NextPurnima {
        /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
        #[arg(long)]
        date: String,
        /// Path to SPK kernel (de442s.bsp)
        #[arg(long)]
        bsp: Option<PathBuf>,
        /// Path to leap second kernel (naif0012.tls)
        #[arg(long)]
        lsk: Option<PathBuf>,
    },
    /// Find next Amavasya (new moon)
    NextAmavasya {
        /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
        #[arg(long)]
        date: String,
        /// Path to SPK kernel
        #[arg(long)]
        bsp: Option<PathBuf>,
        /// Path to leap second kernel
        #[arg(long)]
        lsk: Option<PathBuf>,
    },
    /// Find next Sankranti (Sun entering a rashi)
    NextSankranti(NextSankrantiArgs),
    /// Determine the Masa (lunar month) for a date
    Masa(MasaArgs),
    /// Determine the Ayana (Uttarayana/Dakshinayana) for a date
    Ayana(AyanaArgs),
    /// Determine the Varsha (60-year samvatsara cycle) for a date
    Varsha(VarshaArgs),
    /// Determine the Tithi (lunar day) for a date
    Tithi {
        /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
        #[arg(long)]
        date: String,
        /// Path to SPK kernel
        #[arg(long)]
        bsp: Option<PathBuf>,
        /// Path to leap second kernel
        #[arg(long)]
        lsk: Option<PathBuf>,
    },
    /// Determine the Karana (half-tithi) for a date
    Karana {
        /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
        #[arg(long)]
        date: String,
        /// Path to SPK kernel
        #[arg(long)]
        bsp: Option<PathBuf>,
        /// Path to leap second kernel
        #[arg(long)]
        lsk: Option<PathBuf>,
    },
    /// Determine the Yoga (luni-solar yoga) for a date
    Yoga(YogaArgs),
    /// Determine the Moon's Nakshatra (27-scheme) with start/end times for a date
    MoonNakshatra(MoonNakshatraArgs),
    /// Determine the Vaar (Vedic weekday) for a date and location
    Vaar(VaarArgs),
    /// Determine the Hora (planetary hour) for a date and location
    Hora(HoraArgs),
    /// Determine the Ghatika (1-60) for a date and location
    Ghatika(GhatikaArgs),
    /// Compute all 16 sphutas for a date and location
    Sphutas(SphutasArgs),
    /// Compute all 8 special lagnas for a date and location
    SpecialLagnas(SpecialLagnasArgs),
    /// Compute all 12 arudha padas for a date and location
    ArudhaPadas(ArudhaPadasArgs),
    /// Combined panchang: tithi, karana, yoga, vaar, hora, ghatika
    Panchang(PanchangArgs),
    /// Compute Ashtakavarga (BAV + SAV) for a date and location
    Ashtakavarga(AshtakavargaArgs),
    /// Compute all 11 upagrahas for a date and location
    Upagrahas(UpagrahasArgs),
    /// Compute comprehensive graha positions
    GrahaPositions(GrahaPositionsArgs),
    /// Compute curated sensitive points (bindus) with optional nakshatra/bhava
    CoreBindus(CoreBindusArgs),
    /// Compute graha drishti (planetary aspects) with virupa strength
    Drishti(DrishtiArgs),
    /// Compute full kundali in one call (shared intermediates across sections)
    Kundali(KundaliArgs),
    /// Find previous Purnima (full moon)
    PrevPurnima {
        #[arg(long)]
        date: String,
        #[arg(long)]
        bsp: Option<PathBuf>,
        #[arg(long)]
        lsk: Option<PathBuf>,
    },
    /// Find previous Amavasya (new moon)
    PrevAmavasya {
        #[arg(long)]
        date: String,
        #[arg(long)]
        bsp: Option<PathBuf>,
        #[arg(long)]
        lsk: Option<PathBuf>,
    },
    /// Find previous Sankranti
    PrevSankranti(PrevSankrantiArgs),
    /// Search Purnimas in a date range
    SearchPurnimas(SearchPurnimasArgs),
    /// Search Amavasyas in a date range
    SearchAmavasyas(SearchAmavasyasArgs),
    /// Search Sankrantis in a date range
    SearchSankrantis(SearchSankrantisArgs),
    /// Find next entry of Sun into a specific Rashi
    NextSpecificSankranti(NextSpecificSankrantiArgs),
    /// Find previous entry of Sun into a specific Rashi
    PrevSpecificSankranti(PrevSpecificSankrantiArgs),
    /// Compute ayanamsha for a date
    AyanamshaCompute(AyanamshaComputeArgs),
    /// Compute nutation (dpsi, deps) for a date
    NutationCompute {
        #[arg(long)]
        date: String,
        #[arg(long)]
        bsp: Option<PathBuf>,
        #[arg(long)]
        lsk: Option<PathBuf>,
    },
    /// Compute sunrise/sunset and twilight events
    Sunrise(SunriseArgs),
    /// Compute bhava (house) cusps
    Bhavas(BhavasArgs),
    /// Compute Lagna (Ascendant), MC, and RAMC
    LagnaCompute(LagnaComputeArgs),
    /// Compute Rahu/Ketu (lunar node) longitude
    LunarNode(LunarNodeArgs),
    /// Unified conjunction operation (`--mode next|prev|range`)
    Conjunction(ConjunctionOpArgs),
    /// Find next conjunction between two bodies
    NextConjunction(NextConjunctionArgs),
    /// Find previous conjunction between two bodies
    PrevConjunction(PrevConjunctionArgs),
    /// Search conjunctions between two bodies in a date range
    SearchConjunctions(SearchConjunctionsArgs),
    /// Unified grahan operation (`--kind chandra|surya --mode next|prev|range`)
    Grahan(GrahanOpArgs),
    /// Unified lunar-phase operation (`--kind amavasya|purnima --mode next|prev|range`)
    LunarPhase(LunarPhaseOpArgs),
    /// Unified sankranti operation (`--mode next|prev|range [--rashi 0..11]`)
    Sankranti(SankrantiOpArgs),
    /// Find next lunar eclipse
    NextChandraGrahan {
        #[arg(long)]
        date: String,
        #[arg(long)]
        bsp: Option<PathBuf>,
        #[arg(long)]
        lsk: Option<PathBuf>,
    },
    /// Find previous lunar eclipse
    PrevChandraGrahan {
        #[arg(long)]
        date: String,
        #[arg(long)]
        bsp: Option<PathBuf>,
        #[arg(long)]
        lsk: Option<PathBuf>,
    },
    /// Search lunar eclipses in a date range
    SearchChandraGrahan(SearchChandraGrahanArgs),
    /// Find next solar eclipse
    NextSuryaGrahan {
        #[arg(long)]
        date: String,
        #[arg(long)]
        bsp: Option<PathBuf>,
        #[arg(long)]
        lsk: Option<PathBuf>,
    },
    /// Find previous solar eclipse
    PrevSuryaGrahan {
        #[arg(long)]
        date: String,
        #[arg(long)]
        bsp: Option<PathBuf>,
        #[arg(long)]
        lsk: Option<PathBuf>,
    },
    /// Search solar eclipses in a date range
    SearchSuryaGrahan(SearchSuryaGrahanArgs),
    /// Unified motion operation (`--kind stationary|max-speed --mode next|prev|range`)
    Motion(MotionOpArgs),
    /// Find next stationary point of a planet
    NextStationary(NextStationaryArgs),
    /// Find previous stationary point of a planet
    PrevStationary(PrevStationaryArgs),
    /// Search stationary points of a planet in a date range
    SearchStationary(SearchStationaryArgs),
    /// Find next max-speed event of a planet
    NextMaxSpeed(NextMaxSpeedArgs),
    /// Find previous max-speed event of a planet
    PrevMaxSpeed(PrevMaxSpeedArgs),
    /// Search max-speed events of a planet in a date range
    SearchMaxSpeed(SearchMaxSpeedArgs),
    /// Query spherical position of a body (lon, lat, distance)
    Position(PositionArgs),
    /// Sidereal longitude of a body
    SiderealLongitude(SiderealLongitudeArgs),
    /// Sidereal longitudes of all 9 grahas
    GrahaLongitudes(GrahaLongitudesArgs),
    /// Moving heliocentric osculating apogee longitudes for Mangal/Buddh/Guru/Shukra/Shani
    OsculatingApogee(OsculatingApogeeArgs),

    // -------------------------------------------------------------------
    // Individual Sphuta Formulas (pure math)
    // -------------------------------------------------------------------
    /// Compute Bhrigu Bindu = midpoint(Rahu, Moon)
    BhriguBindu {
        /// Rahu sidereal longitude in degrees
        #[arg(long)]
        rahu: f64,
        /// Moon sidereal longitude in degrees
        #[arg(long)]
        moon: f64,
    },
    /// Compute Prana Sphuta = Lagna + Moon
    PranaSphuta {
        #[arg(long)]
        lagna: f64,
        #[arg(long)]
        moon: f64,
    },
    /// Compute Deha Sphuta = Moon + Lagna
    DehaSphuta {
        #[arg(long)]
        moon: f64,
        #[arg(long)]
        lagna: f64,
    },
    /// Compute Mrityu Sphuta = 8th lord + Lagna
    MrityuSphuta {
        #[arg(long)]
        eighth_lord: f64,
        #[arg(long)]
        lagna: f64,
    },
    /// Compute Tithi Sphuta = (Moon - Sun) + Lagna
    TithiSphuta {
        #[arg(long)]
        moon: f64,
        #[arg(long)]
        sun: f64,
        #[arg(long)]
        lagna: f64,
    },
    /// Compute Yoga Sphuta = Sun + Moon (raw sum)
    YogaSphuta {
        #[arg(long)]
        sun: f64,
        #[arg(long)]
        moon: f64,
    },
    /// Compute Yoga Sphuta Normalized = (Sun + Moon) mod 360
    YogaSphutaNormalized {
        #[arg(long)]
        sun: f64,
        #[arg(long)]
        moon: f64,
    },
    /// Compute Rahu Tithi Sphuta = (Rahu - Sun) + Lagna
    RahuTithiSphuta {
        #[arg(long)]
        rahu: f64,
        #[arg(long)]
        sun: f64,
        #[arg(long)]
        lagna: f64,
    },
    /// Compute Kshetra Sphuta from Venus, Moon, Mars, Jupiter, Lagna
    KshetraSphuta(KshetraSphutaArgs),
    /// Compute Beeja Sphuta from Sun, Venus, Jupiter
    BeejaSphuta {
        #[arg(long)]
        sun: f64,
        #[arg(long)]
        venus: f64,
        #[arg(long)]
        jupiter: f64,
    },
    /// Compute TriSphuta = Lagna + Moon + Gulika
    TriSphuta {
        #[arg(long)]
        lagna: f64,
        #[arg(long)]
        moon: f64,
        #[arg(long)]
        gulika: f64,
    },
    /// Compute ChatusSphuta = TriSphuta + Sun
    ChatusSphuta {
        #[arg(long)]
        trisphuta: f64,
        #[arg(long)]
        sun: f64,
    },
    /// Compute PanchaSphuta = ChatusSphuta + Rahu
    PanchaSphuta {
        #[arg(long)]
        chatussphuta: f64,
        #[arg(long)]
        rahu: f64,
    },
    /// Compute Sookshma TriSphuta = Lagna + Moon + Gulika + Sun
    SookshmaTrisphuta(SookshmaTrisphutaArgs),
    /// Compute Avayoga Sphuta
    AvayogaSphuta {
        #[arg(long)]
        sun: f64,
        #[arg(long)]
        moon: f64,
    },
    /// Compute Kunda = Lagna + Moon + Mars
    Kunda {
        #[arg(long)]
        lagna: f64,
        #[arg(long)]
        moon: f64,
        #[arg(long)]
        mars: f64,
    },

    // -------------------------------------------------------------------
    // Individual Special Lagna Formulas (pure math)
    // -------------------------------------------------------------------
    /// Compute Bhava Lagna from Sun longitude and ghatikas
    BhavaLagna {
        #[arg(long)]
        sun_lon: f64,
        #[arg(long)]
        ghatikas: f64,
    },
    /// Compute Hora Lagna from Sun longitude and ghatikas
    HoraLagna {
        #[arg(long)]
        sun_lon: f64,
        #[arg(long)]
        ghatikas: f64,
    },
    /// Compute Ghati Lagna from Sun longitude and ghatikas
    GhatiLagna {
        #[arg(long)]
        sun_lon: f64,
        #[arg(long)]
        ghatikas: f64,
    },
    /// Compute Vighati Lagna from Lagna longitude and vighatikas
    VighatiLagna {
        #[arg(long)]
        lagna_lon: f64,
        #[arg(long)]
        vighatikas: f64,
    },
    /// Compute Varnada Lagna from Lagna and Hora Lagna longitudes
    VarnadaLagna {
        #[arg(long)]
        lagna_lon: f64,
        #[arg(long)]
        hora_lagna_lon: f64,
    },
    /// Compute Sree Lagna from Moon and Lagna longitudes
    SreeLagna {
        #[arg(long)]
        moon_lon: f64,
        #[arg(long)]
        lagna_lon: f64,
    },
    /// Compute Pranapada Lagna from Sun longitude and ghatikas
    PranapadaLagna {
        #[arg(long)]
        sun_lon: f64,
        #[arg(long)]
        ghatikas: f64,
    },
    /// Compute Indu Lagna from Moon longitude and graha lord indices
    InduLagna {
        #[arg(long)]
        moon_lon: f64,
        /// Graha index of lagna lord (0-8: Sun..Ketu)
        #[arg(long)]
        lagna_lord: u8,
        /// Graha index of Moon's 9th lord (0-8)
        #[arg(long)]
        moon_9th_lord: u8,
    },

    // -------------------------------------------------------------------
    // Utility Primitives
    // -------------------------------------------------------------------
    /// Determine Tithi from Moon-Sun elongation (degrees)
    TithiFromElongation {
        /// Elongation (Moon_lon - Sun_lon) mod 360 in degrees
        #[arg(long)]
        elongation: f64,
    },
    /// Determine Karana from Moon-Sun elongation (degrees)
    KaranaFromElongation {
        #[arg(long)]
        elongation: f64,
    },
    /// Determine Yoga from sidereal sum (Sun + Moon) degrees
    YogaFromSum {
        /// Sidereal sum (Sun_sid + Moon_sid) mod 360
        #[arg(long)]
        sum: f64,
    },
    /// Determine Vaar (weekday) from Julian Date
    VaarFromJd {
        /// Julian Date
        #[arg(long)]
        jd: f64,
    },
    /// Determine Masa from rashi index (0-11)
    MasaFromRashi {
        /// Rashi index (0=Mesha .. 11=Meena)
        #[arg(long)]
        rashi: u8,
    },
    /// Determine Ayana from sidereal longitude
    AyanaFromLon {
        /// Sidereal longitude in degrees
        #[arg(long)]
        lon: f64,
    },
    /// Determine Samvatsara from a year
    SamvatsaraCompute {
        /// CE year
        #[arg(long)]
        year: i32,
    },
    /// Compute the rashi index that is N signs from a starting rashi
    NthRashiFrom {
        /// Starting rashi index (0-11)
        #[arg(long)]
        rashi: u8,
        /// Offset in signs
        #[arg(long)]
        offset: u8,
    },
    /// Determine the rashi lord for a rashi index
    RashiLord {
        /// Rashi index (0-11)
        #[arg(long)]
        rashi: u8,
    },
    /// Normalize angle to [0, 360)
    Normalize360 {
        /// Angle in degrees
        #[arg(long)]
        deg: f64,
    },
    /// Compute a single arudha pada from bhava cusp and lord longitudes
    ArudhaPadaCompute {
        /// Bhava cusp longitude in degrees
        #[arg(long)]
        cusp_lon: f64,
        /// Lord longitude in degrees
        #[arg(long)]
        lord_lon: f64,
    },
    /// Compute 5 sun-based upagrahas from Sun's sidereal longitude
    SunBasedUpagrahas {
        /// Sun's sidereal longitude in degrees
        #[arg(long)]
        sun_lon: f64,
    },
    /// Low-level graha relationship, dignity, and combustion helpers
    GrahaHelper(GrahaHelperArgs),
    /// Low-level time and frame utility helpers
    TimeUtility(TimeUtilityArgs),
    /// Low-level tara propagation and correction primitives
    TaraPrimitive(TaraPrimitiveArgs),

    // -------------------------------------------------------------------
    // Panchang Intermediates (engine required)
    // -------------------------------------------------------------------
    /// Compute Moon-Sun elongation at a date
    ElongationAt {
        #[arg(long)]
        date: String,
        #[arg(long)]
        bsp: Option<PathBuf>,
        #[arg(long)]
        lsk: Option<PathBuf>,
    },
    /// Compute sidereal sum (Moon + Sun) at a date
    SiderealSumAt(SiderealSumAtArgs),
    /// Query body ecliptic longitude and latitude
    BodyLonLat(BodyLonLatArgs),
    /// Compute Vedic day sunrise bracket (today's and next sunrise)
    VedicDaySunrises(VedicDaySunrisesArgs),
    /// Compute Tithi from pre-computed elongation at a date
    TithiAt(TithiAtArgs),
    /// Compute Karana from pre-computed elongation at a date
    KaranaAt(KaranaAtArgs),
    /// Compute Yoga from pre-computed sidereal sum at a date
    YogaAt(YogaAtArgs),
    /// Compute Moon nakshatra from pre-computed sidereal longitude at a date
    NakshatraAt(NakshatraAtArgs),

    // -------------------------------------------------------------------
    // Low-level Ashtakavarga / Drishti
    // -------------------------------------------------------------------
    /// Compute full Ashtakavarga from rashi positions
    CalculateAshtakavarga {
        /// Comma-separated rashi indices for Sun,Moon,Mars,Mercury,Jupiter,Venus,Saturn (0-11)
        #[arg(long)]
        graha_rashis: String,
        /// Lagna rashi index (0-11)
        #[arg(long)]
        lagna_rashi: u8,
    },
    /// Compute graha drishti between two points
    GrahaDrishtiCompute {
        /// Graha index (0=Sun, 1=Moon, ..., 8=Ketu)
        #[arg(long)]
        graha: u8,
        /// Source longitude in degrees
        #[arg(long)]
        source: f64,
        /// Target longitude in degrees
        #[arg(long)]
        target: f64,
    },
    /// Compute full 9×9 graha drishti matrix from longitudes
    GrahaDrishtiMatrixCompute {
        /// Comma-separated sidereal longitudes for all 9 grahas
        #[arg(long)]
        longitudes: String,
    },
    /// Compute Shadbala (six-fold planetary strength) for a date and location
    Shadbala(ShadbalaArgs),
    /// Compute Bhava Bala (house strength) for a date and location
    Bhavabala(BhavabalaArgs),
    /// Compute bundled balas for a date and location
    Balas(BalasArgs),
    /// Compute Vimsopaka Bala (20-point varga dignity strength) for a date and location
    Vimsopaka(VimsopakaArgs),
    /// Compute Chara Karaka assignments for a date
    Charakaraka(CharakarakaArgs),
    /// Transform a sidereal longitude through amsha (divisional chart) mappings
    Amsha(AmshaArgs),
    /// List supported variation codes and names for one or more amshas
    AmshaVariations(AmshaVariationsArgs),
    /// Compute amsha charts for a date and location
    AmshaChart(AmshaChartArgs),
    /// Compute Graha Avasthas (planetary states) for a date and location
    Avastha(AvasthaArgs),
    /// Compute Dasha (planetary period) hierarchy or snapshot
    Dasha(DashaArgs),
    /// List all fixed stars in a catalog
    TaraList {
        /// Path to star catalog JSON
        #[arg(long)]
        catalog: PathBuf,
        /// Filter by category: yogatara, rashi, special, galactic (optional)
        #[arg(long)]
        category: Option<String>,
    },
    /// Compute fixed star position (equatorial, ecliptic, or sidereal)
    TaraPosition(TaraPositionArgs),
}

fn aya_system_from_code(code: i32) -> Option<AyanamshaSystem> {
    let all = AyanamshaSystem::all();
    let idx = usize::try_from(code).ok()?;
    all.get(idx).copied()
}

fn parse_utc(s: &str) -> Result<UtcTime, String> {
    // Parse "YYYY-MM-DDThh:mm:ssZ" or "YYYY-MM-DDThh:mm:ss"
    let s = s.trim_end_matches('Z');
    let parts: Vec<&str> = s.split('T').collect();
    if parts.len() != 2 {
        return Err(format!("expected YYYY-MM-DDThh:mm:ssZ, got {s}"));
    }
    let date_parts: Vec<&str> = parts[0].split('-').collect();
    let time_parts: Vec<&str> = parts[1].split(':').collect();
    if date_parts.len() != 3 || time_parts.len() != 3 {
        return Err(format!("invalid date/time format: {s}"));
    }
    let year: i32 = date_parts[0].parse().map_err(|e| format!("{e}"))?;
    let month: u32 = date_parts[1].parse().map_err(|e| format!("{e}"))?;
    let day: u32 = date_parts[2].parse().map_err(|e| format!("{e}"))?;
    let hour: u32 = time_parts[0].parse().map_err(|e| format!("{e}"))?;
    let minute: u32 = time_parts[1].parse().map_err(|e| format!("{e}"))?;
    let second: f64 = time_parts[2].parse().map_err(|e| format!("{e}"))?;
    UtcTime::try_new(year, month, day, hour, minute, second, None).map_err(|e| e.to_string())
}

fn parse_defaults_mode(s: &str) -> DefaultsMode {
    match s.trim().to_ascii_lowercase().as_str() {
        "recommended" => DefaultsMode::Recommended,
        "none" => DefaultsMode::None,
        other => {
            eprintln!("Invalid --defaults-mode: {other} (recommended|none)");
            std::process::exit(1);
        }
    }
}

fn cli_resolver() -> Option<&'static ConfigResolver> {
    CLI_CONFIG_RESOLVER.get().and_then(|v| v.as_ref())
}

fn resolve_engine_config_for_cli(
    bsp: &Option<PathBuf>,
    lsk: &Option<PathBuf>,
) -> Result<EngineConfig, String> {
    if let Some(resolver) = cli_resolver() {
        let patch = EngineConfigPatch {
            spk_paths: bsp.as_ref().map(|p| vec![p.to_string_lossy().to_string()]),
            lsk_path: lsk.as_ref().map(|p| p.to_string_lossy().to_string()),
            cache_capacity: Some(256),
            strict_validation: Some(true),
        };
        return resolver
            .resolve_engine(Some(patch))
            .map(|v| v.value)
            .map_err(|e| e.to_string());
    }

    let bsp_path = bsp
        .as_ref()
        .ok_or_else(|| "missing --bsp and no config file engine.spk_paths".to_string())?;
    let lsk_path = lsk
        .as_ref()
        .ok_or_else(|| "missing --lsk and no config file engine.lsk_path".to_string())?;
    Ok(EngineConfig::with_single_spk(
        bsp_path.clone(),
        lsk_path.clone(),
        256,
        true,
    ))
}

fn load_engine(bsp: &Option<PathBuf>, lsk: &Option<PathBuf>) -> Engine {
    let config = resolve_engine_config_for_cli(bsp, lsk).unwrap_or_else(|e| {
        eprintln!("Failed to resolve engine config: {e}");
        std::process::exit(1);
    });
    let engine = Engine::new(config).unwrap_or_else(|e| {
        eprintln!("Failed to load engine: {e}");
        std::process::exit(1);
    });
    maybe_warn_stale_lsk(engine.lsk());
    engine
}

fn require_aya_system(code: i32) -> AyanamshaSystem {
    aya_system_from_code(code).unwrap_or_else(|| {
        eprintln!("Invalid ayanamsha code: {code} (0-19)");
        std::process::exit(1);
    })
}

fn parse_precession_model(s: &str) -> PrecessionModel {
    match s {
        "vondrak2011" | "vondrak" => PrecessionModel::Vondrak2011,
        "iau2006" => PrecessionModel::Iau2006,
        "lieske1977" | "lieske" => PrecessionModel::Lieske1977,
        "newcomb1895" | "newcomb" => PrecessionModel::Newcomb1895,
        _ => {
            eprintln!(
                "Invalid precession model: {s} (vondrak2011, iau2006, lieske1977, newcomb1895)"
            );
            std::process::exit(1);
        }
    }
}

fn parse_reference_plane_arg(s: &str, default_plane: ReferencePlane) -> ReferencePlane {
    match s {
        "default" => default_plane,
        "ecliptic" => ReferencePlane::Ecliptic,
        "invariable" => ReferencePlane::Invariable,
        _ => {
            eprintln!("Invalid reference plane: {s} (default, ecliptic, invariable)");
            std::process::exit(1);
        }
    }
}

fn load_eop(path: &Path) -> EopKernel {
    let c04_path = EOP_C04_PATH
        .get()
        .and_then(|p| p.as_ref())
        .map(|p| p.as_path());
    let daily_path = EOP_DAILY_PATH
        .get()
        .and_then(|p| p.as_ref())
        .map(|p| p.as_path());
    let eop = if c04_path.is_some() || daily_path.is_some() {
        EopKernel::load_merged(path, c04_path, daily_path)
    } else {
        EopKernel::load(path)
    }
    .unwrap_or_else(|e| {
        eprintln!("Failed to load EOP: {e}");
        std::process::exit(1);
    });
    maybe_warn_stale_eop(&eop);
    eop
}

fn parse_graha_name(s: &str) -> Graha {
    match s.to_lowercase().as_str() {
        "sun" | "surya" => Graha::Surya,
        "moon" | "chandra" => Graha::Chandra,
        "mars" | "mangal" => Graha::Mangal,
        "mercury" | "buddh" => Graha::Buddh,
        "jupiter" | "guru" => Graha::Guru,
        "venus" | "shukra" => Graha::Shukra,
        "saturn" | "shani" => Graha::Shani,
        "rahu" => Graha::Rahu,
        "ketu" => Graha::Ketu,
        _ => {
            eprintln!("Invalid graha name: {s}");
            eprintln!("Valid: Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn, Rahu, Ketu");
            std::process::exit(1);
        }
    }
}

fn graha_display_name(graha: Graha) -> &'static str {
    match graha {
        Graha::Surya => "Surya",
        Graha::Chandra => "Chandra",
        Graha::Mangal => "Mangal",
        Graha::Buddh => "Buddh",
        Graha::Guru => "Guru",
        Graha::Shukra => "Shukra",
        Graha::Shani => "Shani",
        Graha::Rahu => "Rahu",
        Graha::Ketu => "Ketu",
    }
}

fn parse_node_policy(s: &str) -> NodeDignityPolicy {
    match s.to_lowercase().as_str() {
        "sign-lord" | "signlord" => NodeDignityPolicy::SignLordBased,
        "sama" | "always-sama" => NodeDignityPolicy::AlwaysSama,
        _ => {
            eprintln!("Invalid node policy: {s}");
            eprintln!("Valid: sign-lord (default), sama");
            std::process::exit(1);
        }
    }
}

fn require_body(code: i32) -> Body {
    Body::from_code(code).unwrap_or_else(|| {
        eprintln!("Invalid body code: {code}");
        std::process::exit(1);
    })
}

fn require_observer(code: i32) -> Observer {
    Observer::from_code(code).unwrap_or_else(|| {
        eprintln!("Invalid observer code: {code}");
        std::process::exit(1);
    })
}

fn parse_panchang_include_mask(raw: &str) -> Result<u32, String> {
    let mut mask = 0_u32;
    for token in raw.split(',').map(str::trim).filter(|t| !t.is_empty()) {
        match token.to_ascii_lowercase().as_str() {
            "all" => mask |= PANCHANG_INCLUDE_ALL,
            "core" => mask |= PANCHANG_INCLUDE_ALL_CORE,
            "calendar" => mask |= PANCHANG_INCLUDE_ALL_CALENDAR,
            "tithi" => mask |= PANCHANG_INCLUDE_TITHI,
            "karana" => mask |= PANCHANG_INCLUDE_KARANA,
            "yoga" => mask |= PANCHANG_INCLUDE_YOGA,
            "vaar" => mask |= PANCHANG_INCLUDE_VAAR,
            "hora" => mask |= PANCHANG_INCLUDE_HORA,
            "ghatika" => mask |= PANCHANG_INCLUDE_GHATIKA,
            "nakshatra" => mask |= PANCHANG_INCLUDE_NAKSHATRA,
            "masa" => mask |= PANCHANG_INCLUDE_MASA,
            "ayana" => mask |= PANCHANG_INCLUDE_AYANA,
            "varsha" => mask |= PANCHANG_INCLUDE_VARSHA,
            other => {
                return Err(format!(
                    "invalid include token '{other}' (use: tithi,karana,yoga,vaar,hora,ghatika,nakshatra,masa,ayana,varsha,core,calendar,all)"
                ));
            }
        }
    }
    if mask == 0 {
        return Err("include mask is empty".to_string());
    }
    Ok(mask)
}

fn utc_to_jd_utc(utc: &UtcTime) -> f64 {
    let day_frac = utc.day as f64
        + utc.hour as f64 / 24.0
        + utc.minute as f64 / 1440.0
        + utc.second / 86_400.0;
    calendar_to_jd(utc.year, utc.month, day_frac)
}

static CLI_WARNED_LSK_PRE: AtomicBool = AtomicBool::new(false);
static CLI_WARNED_LSK_FUTURE: AtomicBool = AtomicBool::new(false);
static CLI_WARNED_DELTA_T: AtomicBool = AtomicBool::new(false);
static CLI_WARNED_STALE_LSK: AtomicBool = AtomicBool::new(false);
static CLI_WARNED_STALE_EOP: AtomicBool = AtomicBool::new(false);
static STALE_LSK_THRESHOLD_DAYS: OnceLock<Option<f64>> = OnceLock::new();
static STALE_EOP_THRESHOLD_DAYS: OnceLock<Option<f64>> = OnceLock::new();
static EOP_C04_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();
static EOP_DAILY_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();
static CLI_CONFIG_RESOLVER: OnceLock<Option<ConfigResolver>> = OnceLock::new();

fn now_jd_utc() -> f64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();
    2_440_587.5 + now / 86_400.0
}

fn jd_to_ymd_string(jd: f64) -> String {
    let (y, m, d) = jd_to_calendar(jd);
    format!("{y:04}-{m:02}-{:02}", d.floor() as u32)
}

fn jd_utc_to_iso_string(jd: f64) -> String {
    let (year, month, day_frac) = jd_to_calendar(jd);
    let day = day_frac.floor() as u32;
    let frac = day_frac.fract();
    let total_seconds = frac * 86_400.0;
    let hour = (total_seconds / 3600.0).floor() as u32;
    let minute = ((total_seconds % 3600.0) / 60.0).floor() as u32;
    let second = total_seconds % 60.0;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:09.6}Z")
}

fn maybe_warn_stale_lsk(lsk: &LeapSecondKernel) {
    let Some(Some(threshold_days)) = STALE_LSK_THRESHOLD_DAYS.get().copied() else {
        return;
    };
    let Some((delta_at, end_tdb_s)) = lsk.data().last_delta_at() else {
        return;
    };
    let end_jd = tdb_seconds_to_jd(end_tdb_s);
    let age_days = now_jd_utc() - end_jd;
    if age_days > threshold_days && !CLI_WARNED_STALE_LSK.swap(true, Ordering::Relaxed) {
        eprintln!(
            "Warning: LSK appears stale (coverage end {}, ~{:.1} days old, DELTA_AT={}s). Consider updating naif0012.tls.",
            jd_to_ymd_string(end_jd),
            age_days,
            delta_at
        );
    }
}

fn maybe_warn_stale_eop(eop: &EopKernel) {
    let Some(Some(threshold_days)) = STALE_EOP_THRESHOLD_DAYS.get().copied() else {
        return;
    };
    let (_start_mjd, end_mjd) = eop.data().range();
    let end_jd = end_mjd + 2_400_000.5;
    let age_days = now_jd_utc() - end_jd;
    if age_days > threshold_days && !CLI_WARNED_STALE_EOP.swap(true, Ordering::Relaxed) {
        eprintln!(
            "Warning: EOP appears stale (coverage end {}, ~{:.1} days old). Consider updating finals2000A.all/finals2000A.daily.extended.",
            jd_to_ymd_string(end_jd),
            age_days
        );
    }
}

fn parse_time_policy(
    s: &str,
    delta_t_model: DeltaTModel,
    smh_future_family: SmhFutureParabolaFamily,
    future_delta_t_transition: FutureDeltaTTransition,
    no_freeze_future_dut1: bool,
    no_warn_on_fallback: bool,
    pre_range_dut1: Option<f64>,
    future_transition_years: Option<f64>,
) -> TimeConversionPolicy {
    match s {
        "strict-lsk" => TimeConversionPolicy::StrictLsk,
        "hybrid-deltat" => {
            let mut opts = TimeConversionOptions::default();
            opts.warn_on_fallback = !no_warn_on_fallback;
            opts.delta_t_model = delta_t_model;
            opts.smh_future_family = smh_future_family;
            opts.future_delta_t_transition = future_delta_t_transition;
            opts.freeze_future_dut1 = !no_freeze_future_dut1;
            if let Some(v) = pre_range_dut1 {
                opts.pre_range_dut1 = v;
            }
            if let Some(v) = future_transition_years {
                opts.future_transition_years = v;
            }
            TimeConversionPolicy::HybridDeltaT(opts)
        }
        _ => {
            eprintln!("Invalid time policy: {s} (strict-lsk, hybrid-deltat)");
            std::process::exit(1);
        }
    }
}

fn parse_future_delta_t_transition(s: &str) -> FutureDeltaTTransition {
    match s.to_lowercase().as_str() {
        "legacy-tt-utc-blend" | "legacy" => FutureDeltaTTransition::LegacyTtUtcBlend,
        "bridge-modern-endpoint" | "bridge" => FutureDeltaTTransition::BridgeFromModernEndpoint,
        _ => {
            eprintln!(
                "Invalid future delta-t transition: {s} (legacy-tt-utc-blend, bridge-modern-endpoint)"
            );
            std::process::exit(1);
        }
    }
}

fn parse_delta_t_model(s: &str) -> DeltaTModel {
    match s {
        "legacy-em2006" | "legacy" => DeltaTModel::LegacyEspenakMeeus2006,
        "smh2016" => DeltaTModel::Smh2016WithPre720Quadratic,
        _ => {
            eprintln!("Invalid delta-T model: {s} (legacy-em2006, smh2016)");
            std::process::exit(1);
        }
    }
}

fn parse_smh_future_family(s: &str) -> SmhFutureParabolaFamily {
    match s.to_lowercase().as_str() {
        "addendum2020" | "smh2020" | "piecewise" => SmhFutureParabolaFamily::Addendum2020Piecewise,
        "c-20" | "c-20.0" | "cminus20" => SmhFutureParabolaFamily::ConstantCMinus20,
        "c-17.52" | "cminus17.52" | "cminus17p52" => SmhFutureParabolaFamily::ConstantCMinus17p52,
        "c-15.32" | "cminus15.32" | "cminus15p32" => SmhFutureParabolaFamily::ConstantCMinus15p32,
        "stephenson1997" | "st97" | "swiss-stephenson1997" | "swisseph-stephenson1997" => {
            SmhFutureParabolaFamily::Stephenson1997
        }
        "stephenson2016"
        | "st2016"
        | "stephenson2016-cubic"
        | "st2016-cubic"
        | "stephenson2016cubic" => SmhFutureParabolaFamily::Stephenson2016,
        _ => {
            eprintln!(
                "Invalid smh future family: {s} (addendum2020, c-20, c-17.52, c-15.32, stephenson1997, stephenson2016)"
            );
            std::process::exit(1);
        }
    }
}

fn emit_cli_time_warning_once(warning: &TimeWarning) {
    match warning {
        TimeWarning::LskPreRangeFallback { .. } => {
            if !CLI_WARNED_LSK_PRE.swap(true, Ordering::Relaxed) {
                eprintln!("Warning: {warning}");
            }
        }
        TimeWarning::LskFutureFrozen { .. } => {
            if !CLI_WARNED_LSK_FUTURE.swap(true, Ordering::Relaxed) {
                eprintln!("Warning: {warning}");
            }
        }
        TimeWarning::DeltaTModelUsed { .. } => {
            if !CLI_WARNED_DELTA_T.swap(true, Ordering::Relaxed) {
                eprintln!("Warning: {warning}");
            }
        }
        _ => {}
    }
}

fn utc_to_jd_tdb_with_policy(
    utc: &UtcTime,
    lsk: &LeapSecondKernel,
    policy: TimeConversionPolicy,
) -> f64 {
    utc_to_jd_tdb_with_policy_and_eop(utc, lsk, None, policy)
}

fn utc_to_jd_tdb_with_policy_and_eop(
    utc: &UtcTime,
    lsk: &LeapSecondKernel,
    eop: Option<&EopKernel>,
    policy: TimeConversionPolicy,
) -> f64 {
    let jd_utc = utc_to_jd_utc(utc);
    let utc_s = jd_to_tdb_seconds(jd_utc);
    let out = lsk.utc_to_tdb_with_policy_and_eop(utc_s, eop, policy);
    for w in &out.diagnostics.warnings {
        emit_cli_time_warning_once(w);
    }
    tdb_seconds_to_jd(out.tdb_seconds)
}

fn rashi_from_index(idx: u8) -> Rashi {
    dhruv_vedic_base::ALL_RASHIS
        .get(idx as usize)
        .copied()
        .unwrap_or_else(|| {
            eprintln!("Invalid rashi index: {idx} (0-11)");
            std::process::exit(1);
        })
}

fn parse_lunar_node(s: &str) -> LunarNode {
    match s.to_lowercase().as_str() {
        "rahu" => LunarNode::Rahu,
        "ketu" => LunarNode::Ketu,
        _ => {
            eprintln!("Invalid node: {s} (rahu or ketu)");
            std::process::exit(1);
        }
    }
}

fn parse_node_mode(s: &str) -> NodeMode {
    match s.to_lowercase().as_str() {
        "mean" => NodeMode::Mean,
        "true" => NodeMode::True,
        _ => {
            eprintln!("Invalid mode: {s} (mean or true)");
            std::process::exit(1);
        }
    }
}

fn require_graha(index: u8) -> Graha {
    ALL_GRAHAS.get(index as usize).copied().unwrap_or_else(|| {
        eprintln!("Invalid graha index: {index} (0-8: Surya..Ketu)");
        std::process::exit(1);
    })
}

fn require_vaar(index: u8) -> dhruv_vedic_base::Vaar {
    dhruv_vedic_base::ALL_VAARS
        .get(index as usize)
        .copied()
        .unwrap_or_else(|| {
            eprintln!("Invalid vaar index: {index} (0-6: Ravivaar..Shanivaar)");
            std::process::exit(1);
        })
}

fn require_masa(index: u8) -> dhruv_vedic_base::Masa {
    dhruv_vedic_base::ALL_MASAS
        .get(index as usize)
        .copied()
        .unwrap_or_else(|| {
            eprintln!("Invalid masa index: {index} (0-11: Chaitra..Phalguna)");
            std::process::exit(1);
        })
}

fn require_samvatsara(index: u8) -> dhruv_vedic_base::Samvatsara {
    dhruv_vedic_base::ALL_SAMVATSARAS
        .get(index as usize)
        .copied()
        .unwrap_or_else(|| {
            eprintln!("Invalid samvatsara index: {index} (0-59)");
            std::process::exit(1);
        })
}

fn parse_graha_rashis(s: &str) -> [u8; 7] {
    let vals: Vec<u8> = s
        .split(',')
        .map(|v| {
            v.trim().parse::<u8>().unwrap_or_else(|e| {
                eprintln!("Invalid rashi value '{v}': {e}");
                std::process::exit(1);
            })
        })
        .collect();
    if vals.len() != 7 {
        eprintln!(
            "Expected 7 comma-separated rashi indices, got {}",
            vals.len()
        );
        std::process::exit(1);
    }
    let mut arr = [0u8; 7];
    arr.copy_from_slice(&vals);
    arr
}

fn parse_longitudes_9(s: &str) -> [f64; 9] {
    let vals: Vec<f64> = s
        .split(',')
        .map(|v| {
            v.trim().parse::<f64>().unwrap_or_else(|e| {
                eprintln!("Invalid longitude '{v}': {e}");
                std::process::exit(1);
            })
        })
        .collect();
    if vals.len() != 9 {
        eprintln!("Expected 9 comma-separated longitudes, got {}", vals.len());
        std::process::exit(1);
    }
    let mut arr = [0.0f64; 9];
    arr.copy_from_slice(&vals);
    arr
}

fn parse_u8s<const N: usize>(raw: &str, what: &str) -> [u8; N] {
    let vals: Vec<u8> = raw
        .split(',')
        .map(|v| {
            v.trim().parse::<u8>().unwrap_or_else(|e| {
                eprintln!("Invalid {what} value '{v}': {e}");
                std::process::exit(1);
            })
        })
        .collect();
    if vals.len() != N {
        eprintln!(
            "Expected {N} comma-separated {what} values, got {}",
            vals.len()
        );
        std::process::exit(1);
    }
    let mut arr = [0u8; N];
    arr.copy_from_slice(&vals);
    arr
}

fn parse_bools_9(s: &str) -> [bool; 9] {
    let vals: Vec<bool> = s
        .split(',')
        .map(|v| match v.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "t" | "yes" | "y" => true,
            "0" | "false" | "f" | "no" | "n" => false,
            other => {
                eprintln!("Invalid retrograde flag '{other}' (use true/false or 1/0)");
                std::process::exit(1);
            }
        })
        .collect();
    if vals.len() != 9 {
        eprintln!(
            "Expected 9 comma-separated retrograde flags, got {}",
            vals.len()
        );
        std::process::exit(1);
    }
    let mut arr = [false; 9];
    arr.copy_from_slice(&vals);
    arr
}

fn parse_vec3(s: &str, what: &str) -> [f64; 3] {
    let vals: Vec<f64> = s
        .split(',')
        .map(|v| {
            v.trim().parse::<f64>().unwrap_or_else(|e| {
                eprintln!("Invalid {what} component '{v}': {e}");
                std::process::exit(1);
            })
        })
        .collect();
    if vals.len() != 3 {
        eprintln!(
            "Expected 3 comma-separated values for {what}, got {}",
            vals.len()
        );
        std::process::exit(1);
    }
    [vals[0], vals[1], vals[2]]
}

fn naisargika_label(value: dhruv_vedic_base::NaisargikaMaitri) -> &'static str {
    match value {
        dhruv_vedic_base::NaisargikaMaitri::Friend => "friend",
        dhruv_vedic_base::NaisargikaMaitri::Enemy => "enemy",
        dhruv_vedic_base::NaisargikaMaitri::Neutral => "neutral",
    }
}

fn tatkalika_label(value: dhruv_vedic_base::TatkalikaMaitri) -> &'static str {
    match value {
        dhruv_vedic_base::TatkalikaMaitri::Friend => "friend",
        dhruv_vedic_base::TatkalikaMaitri::Enemy => "enemy",
    }
}

fn panchadha_label(value: dhruv_vedic_base::PanchadhaMaitri) -> &'static str {
    match value {
        dhruv_vedic_base::PanchadhaMaitri::AdhiShatru => "adhi-shatru",
        dhruv_vedic_base::PanchadhaMaitri::Shatru => "shatru",
        dhruv_vedic_base::PanchadhaMaitri::Sama => "sama",
        dhruv_vedic_base::PanchadhaMaitri::Mitra => "mitra",
        dhruv_vedic_base::PanchadhaMaitri::AdhiMitra => "adhi-mitra",
    }
}

fn dignity_label(value: dhruv_vedic_base::Dignity) -> &'static str {
    match value {
        dhruv_vedic_base::Dignity::Exalted => "exalted",
        dhruv_vedic_base::Dignity::Moolatrikone => "moolatrikone",
        dhruv_vedic_base::Dignity::OwnSign => "own-sign",
        dhruv_vedic_base::Dignity::AdhiMitra => "adhi-mitra",
        dhruv_vedic_base::Dignity::Mitra => "mitra",
        dhruv_vedic_base::Dignity::Sama => "sama",
        dhruv_vedic_base::Dignity::Shatru => "shatru",
        dhruv_vedic_base::Dignity::AdhiShatru => "adhi-shatru",
        dhruv_vedic_base::Dignity::Debilitated => "debilitated",
    }
}

fn benefic_label(value: dhruv_vedic_base::BeneficNature) -> &'static str {
    match value {
        dhruv_vedic_base::BeneficNature::Benefic => "benefic",
        dhruv_vedic_base::BeneficNature::Malefic => "malefic",
    }
}

fn gender_label(value: dhruv_vedic_base::GrahaGender) -> &'static str {
    match value {
        dhruv_vedic_base::GrahaGender::Male => "male",
        dhruv_vedic_base::GrahaGender::Female => "female",
        dhruv_vedic_base::GrahaGender::Neuter => "neuter",
    }
}

fn parse_cli_naisargika(value: CliNaisargikaArg) -> dhruv_vedic_base::NaisargikaMaitri {
    match value {
        CliNaisargikaArg::Friend => dhruv_vedic_base::NaisargikaMaitri::Friend,
        CliNaisargikaArg::Enemy => dhruv_vedic_base::NaisargikaMaitri::Enemy,
        CliNaisargikaArg::Neutral => dhruv_vedic_base::NaisargikaMaitri::Neutral,
    }
}

fn parse_cli_tatkalika(value: CliTatkalikaArg) -> dhruv_vedic_base::TatkalikaMaitri {
    match value {
        CliTatkalikaArg::Friend => dhruv_vedic_base::TatkalikaMaitri::Friend,
        CliTatkalikaArg::Enemy => dhruv_vedic_base::TatkalikaMaitri::Enemy,
    }
}

fn parse_cli_node(value: CliNodeArg) -> Graha {
    match value {
        CliNodeArg::Rahu => Graha::Rahu,
        CliNodeArg::Ketu => Graha::Ketu,
    }
}

fn parse_cli_node_policy(value: CliNodePolicyArg) -> NodeDignityPolicy {
    match value {
        CliNodePolicyArg::SignLord => NodeDignityPolicy::SignLordBased,
        CliNodePolicyArg::Sama => NodeDignityPolicy::AlwaysSama,
    }
}

fn parse_cli_calendar_policy(value: CliCalendarPolicyArg) -> dhruv_time::CalendarPolicy {
    match value {
        CliCalendarPolicyArg::ProlepticGregorian => dhruv_time::CalendarPolicy::ProlepticGregorian,
        CliCalendarPolicyArg::GregorianCutover1582 => {
            dhruv_time::CalendarPolicy::GregorianCutover1582
        }
    }
}

fn parse_cli_reference_plane(value: CliReferencePlaneArg) -> ReferencePlane {
    match value {
        CliReferencePlaneArg::Ecliptic => ReferencePlane::Ecliptic,
        CliReferencePlaneArg::Invariable => ReferencePlane::Invariable,
    }
}

fn main() {
    let cli = Cli::parse();
    if let Some(v) = cli.stale_lsk_threshold_days
        && v < 0.0
    {
        eprintln!("Invalid --stale-lsk-threshold-days: must be >= 0");
        std::process::exit(1);
    }
    if let Some(v) = cli.stale_eop_threshold_days
        && v < 0.0
    {
        eprintln!("Invalid --stale-eop-threshold-days: must be >= 0");
        std::process::exit(1);
    }
    if let Some(v) = cli.future_transition_years
        && v < 0.0
    {
        eprintln!("Invalid --future-transition-years: must be >= 0");
        std::process::exit(1);
    }
    let _ = STALE_LSK_THRESHOLD_DAYS.set(cli.stale_lsk_threshold_days);
    let _ = STALE_EOP_THRESHOLD_DAYS.set(cli.stale_eop_threshold_days);
    let _ = EOP_C04_PATH.set(cli.eop_c04.clone());
    let _ = EOP_DAILY_PATH.set(cli.eop_daily.clone());

    let defaults_mode = parse_defaults_mode(&cli.defaults_mode);
    let loaded_config =
        load_with_discovery(cli.config.as_deref(), cli.no_config).unwrap_or_else(|e| {
            eprintln!("Failed to load config: {e}");
            std::process::exit(1);
        });
    if let Some(loaded) = &loaded_config {
        eprintln!("Loaded config: {}", loaded.path.display());
    }
    let resolver = loaded_config.map(|loaded| ConfigResolver::new(loaded.file, defaults_mode));
    let _ = CLI_CONFIG_RESOLVER.set(resolver);

    let delta_t_model = parse_delta_t_model(&cli.delta_t_model);
    let smh_future_family = parse_smh_future_family(&cli.smh_future_family);
    let future_delta_t_transition = parse_future_delta_t_transition(&cli.future_delta_t_transition);
    let time_policy = parse_time_policy(
        &cli.time_policy,
        delta_t_model,
        smh_future_family,
        future_delta_t_transition,
        cli.no_freeze_future_dut1,
        cli.no_warn_on_fallback,
        cli.pre_range_dut1,
        cli.future_transition_years,
    );
    dhruv_search::set_time_conversion_policy(time_policy);

    match cli.command {
        Commands::ConfigShowEffective => {
            let Some(resolver) = cli_resolver() else {
                println!(
                    "No config file loaded; effective config comes from explicit flags and built-in defaults."
                );
                return;
            };

            let engine = resolver.resolve_engine(None);
            let conjunction = resolver.resolve_conjunction(None);
            let grahan = resolver.resolve_grahan(None);
            let stationary = resolver.resolve_stationary(None);
            let sankranti = resolver.resolve_sankranti(None);
            let riseset = resolver.resolve_riseset(None);
            let bhava = resolver.resolve_bhava(None);
            let tara = resolver.resolve_tara(None);
            let graha_positions = resolver.resolve_graha_positions(None);
            let bindus = resolver.resolve_bindus(None);
            let drishti = resolver.resolve_drishti(None);
            let full_kundali = resolver.resolve_full_kundali(None);

            println!("engine={engine:#?}");
            println!("conjunction={conjunction:#?}");
            println!("grahan={grahan:#?}");
            println!("stationary={stationary:#?}");
            println!("sankranti={sankranti:#?}");
            println!("riseset={riseset:#?}");
            println!("bhava={bhava:#?}");
            println!("tara={tara:#?}");
            println!("graha_positions={graha_positions:#?}");
            println!("bindus={bindus:#?}");
            println!("drishti={drishti:#?}");
            println!("full_kundali={full_kundali:#?}");
        }
        Commands::Rashi { lon } => {
            let info = rashi_from_longitude(lon);
            let dms = info.dms;
            println!(
                "{} - {} deg {} min {:.1} sec ({:.6} deg in rashi)",
                info.rashi.name(),
                dms.degrees,
                dms.minutes,
                dms.seconds,
                info.degrees_in_rashi
            );
        }

        Commands::Nakshatra { lon, scheme } => match scheme {
            27 => {
                let info = nakshatra_from_longitude(lon);
                println!(
                    "{} (index {}) - Pada {} ({:.6} deg in nakshatra, {:.6} deg in pada)",
                    info.nakshatra.name(),
                    info.nakshatra_index,
                    info.pada,
                    info.degrees_in_nakshatra,
                    info.degrees_in_pada
                );
            }
            28 => {
                let info = nakshatra28_from_longitude(lon);
                println!(
                    "{} (index {}) - Pada {} ({:.6} deg in nakshatra)",
                    info.nakshatra.name(),
                    info.nakshatra_index,
                    info.pada,
                    info.degrees_in_nakshatra
                );
            }
            _ => {
                eprintln!("Invalid scheme: {scheme}. Use 27 or 28.");
                std::process::exit(1);
            }
        },

        Commands::RashiTropical(args) => {
            let system = require_aya_system(args.ayanamsha);
            let t = jd_tdb_to_centuries(args.jd);
            let aya = ayanamsha_deg(system, t, args.nutation);
            let info = rashi_from_tropical(args.lon, system, args.jd, args.nutation);
            let dms = info.dms;
            println!("Ayanamsha: {:.6} deg", aya);
            println!("Sidereal: {:.6} deg", args.lon - aya);
            println!(
                "{} - {} deg {} min {:.1} sec ({:.6} deg in rashi)",
                info.rashi.name(),
                dms.degrees,
                dms.minutes,
                dms.seconds,
                info.degrees_in_rashi
            );
        }

        Commands::NakshatraTropical(args) => {
            let system = require_aya_system(args.ayanamsha);
            let t = jd_tdb_to_centuries(args.jd);
            let aya = ayanamsha_deg(system, t, args.nutation);
            println!("Ayanamsha: {:.6} deg", aya);
            println!("Sidereal: {:.6} deg", args.lon - aya);
            match args.scheme {
                27 => {
                    let info = nakshatra_from_tropical(args.lon, system, args.jd, args.nutation);
                    println!(
                        "{} (index {}) - Pada {} ({:.6} deg in nakshatra, {:.6} deg in pada)",
                        info.nakshatra.name(),
                        info.nakshatra_index,
                        info.pada,
                        info.degrees_in_nakshatra,
                        info.degrees_in_pada
                    );
                }
                28 => {
                    let info = nakshatra28_from_tropical(args.lon, system, args.jd, args.nutation);
                    println!(
                        "{} (index {}) - Pada {} ({:.6} deg in nakshatra)",
                        info.nakshatra.name(),
                        info.nakshatra_index,
                        info.pada,
                        info.degrees_in_nakshatra
                    );
                }
                _ => {
                    eprintln!("Invalid scheme: {}. Use 27 or 28.", args.scheme);
                    std::process::exit(1);
                }
            }
        }

        Commands::Dms { deg } => {
            let d = deg_to_dms(deg);
            println!("{} deg {} min {:.2} sec", d.degrees, d.minutes, d.seconds);
        }

        Commands::NextPurnima { date, bsp, lsk } => {
            let utc = parse_utc(&date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&bsp, &lsk);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            let op = LunarPhaseOperation {
                kind: LunarPhaseKind::Purnima,
                query: LunarPhaseQuery::Next { at_jd_tdb: jd_tdb },
            };
            match dhruv_search::lunar_phase(&engine, &op) {
                Ok(LunarPhaseResult::Single(Some(ev))) => {
                    println!("Next Purnima: {}", ev.utc);
                    println!(
                        "  Moon lon: {:.6} deg  Sun lon: {:.6} deg",
                        ev.moon_longitude_deg, ev.sun_longitude_deg
                    );
                }
                Ok(LunarPhaseResult::Single(None)) => println!("No Purnima found in search range"),
                Ok(LunarPhaseResult::Many(_)) => {
                    eprintln!("Error: unexpected search result shape");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::NextAmavasya { date, bsp, lsk } => {
            let utc = parse_utc(&date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&bsp, &lsk);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            let op = LunarPhaseOperation {
                kind: LunarPhaseKind::Amavasya,
                query: LunarPhaseQuery::Next { at_jd_tdb: jd_tdb },
            };
            match dhruv_search::lunar_phase(&engine, &op) {
                Ok(LunarPhaseResult::Single(Some(ev))) => {
                    println!("Next Amavasya: {}", ev.utc);
                    println!(
                        "  Moon lon: {:.6} deg  Sun lon: {:.6} deg",
                        ev.moon_longitude_deg, ev.sun_longitude_deg
                    );
                }
                Ok(LunarPhaseResult::Single(None)) => println!("No Amavasya found in search range"),
                Ok(LunarPhaseResult::Many(_)) => {
                    eprintln!("Error: unexpected search result shape");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::NextSankranti(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let system = require_aya_system(args.ayanamsha);
            let engine = load_engine(&args.bsp, &args.lsk);
            let config = SankrantiConfig::new(system, args.nutation);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            let op = SankrantiOperation {
                target: SankrantiTarget::Any,
                config,
                query: SankrantiQuery::Next { at_jd_tdb: jd_tdb },
            };
            match dhruv_search::sankranti(&engine, &op) {
                Ok(SankrantiResult::Single(Some(ev))) => {
                    println!("Next Sankranti: {}", ev.rashi.name());
                    println!("  Time: {}", ev.utc);
                    println!(
                        "  Sidereal lon: {:.6} deg  Tropical lon: {:.6} deg",
                        ev.sun_sidereal_longitude_deg, ev.sun_tropical_longitude_deg
                    );
                }
                Ok(SankrantiResult::Single(None)) => println!("No Sankranti found in search range"),
                Ok(SankrantiResult::Many(_)) => {
                    eprintln!("Error: unexpected search result shape");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Masa(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let system = require_aya_system(args.ayanamsha);
            let engine = load_engine(&args.bsp, &args.lsk);
            let config = SankrantiConfig::new(system, args.nutation);
            match dhruv_search::masa_for_date(&engine, &utc, &config) {
                Ok(info) => {
                    let adhika_str = if info.adhika { " (Adhika)" } else { "" };
                    println!("Masa: {}{}", info.masa.name(), adhika_str);
                    println!("  Start: {}", info.start);
                    println!("  End:   {}", info.end);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Ayana(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let system = require_aya_system(args.ayanamsha);
            let engine = load_engine(&args.bsp, &args.lsk);
            let config = SankrantiConfig::new(system, args.nutation);
            match dhruv_search::ayana_for_date(&engine, &utc, &config) {
                Ok(info) => {
                    println!("Ayana: {}", info.ayana.name());
                    println!("  Start: {}", info.start);
                    println!("  End:   {}", info.end);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Varsha(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let system = require_aya_system(args.ayanamsha);
            let engine = load_engine(&args.bsp, &args.lsk);
            let config = SankrantiConfig::new(system, args.nutation);
            match dhruv_search::varsha_for_date(&engine, &utc, &config) {
                Ok(info) => {
                    println!(
                        "Samvatsara: {} (#{} in 60-year cycle)",
                        info.samvatsara.name(),
                        info.order
                    );
                    println!("  Start: {}", info.start);
                    println!("  End:   {}", info.end);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Tithi { date, bsp, lsk } => {
            let utc = parse_utc(&date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&bsp, &lsk);
            match dhruv_search::tithi_for_date(&engine, &utc) {
                Ok(info) => {
                    println!("Tithi: {} (index {})", info.tithi.name(), info.tithi_index);
                    println!(
                        "  Paksha: {}  Tithi in paksha: {}",
                        info.paksha.name(),
                        info.tithi_in_paksha
                    );
                    println!("  Start: {}", info.start);
                    println!("  End:   {}", info.end);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Karana { date, bsp, lsk } => {
            let utc = parse_utc(&date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&bsp, &lsk);
            match dhruv_search::karana_for_date(&engine, &utc) {
                Ok(info) => {
                    println!(
                        "Karana: {} (sequence index {})",
                        info.karana.name(),
                        info.karana_index
                    );
                    println!("  Start: {}", info.start);
                    println!("  End:   {}", info.end);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Yoga(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let system = require_aya_system(args.ayanamsha);
            let engine = load_engine(&args.bsp, &args.lsk);
            let config = SankrantiConfig::new(system, args.nutation);
            match dhruv_search::yoga_for_date(&engine, &utc, &config) {
                Ok(info) => {
                    println!("Yoga: {} (index {})", info.yoga.name(), info.yoga_index);
                    println!("  Start: {}", info.start);
                    println!("  End:   {}", info.end);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::MoonNakshatra(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let system = require_aya_system(args.ayanamsha);
            let engine = load_engine(&args.bsp, &args.lsk);
            let config = SankrantiConfig::new(system, args.nutation);
            match dhruv_search::nakshatra_for_date(&engine, &utc, &config) {
                Ok(info) => {
                    println!(
                        "Nakshatra: {} (index {}, pada {})",
                        info.nakshatra.name(),
                        info.nakshatra_index,
                        info.pada
                    );
                    println!("  Start: {}", info.start);
                    println!("  End:   {}", info.end);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Vaar(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&args.bsp, &args.lsk);
            let eop_kernel = load_eop(&args.eop);
            let location = GeoLocation::new(args.lat, args.lon, args.alt);
            let rs_config = RiseSetConfig::default();
            match dhruv_search::vaar_for_date(&engine, &eop_kernel, &utc, &location, &rs_config) {
                Ok(info) => {
                    println!("Vaar: {}", info.vaar.name());
                    println!("  Start (sunrise): {}", info.start);
                    println!("  End (next sunrise): {}", info.end);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Hora(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&args.bsp, &args.lsk);
            let eop_kernel = load_eop(&args.eop);
            let location = GeoLocation::new(args.lat, args.lon, args.alt);
            let rs_config = RiseSetConfig::default();
            match dhruv_search::hora_for_date(&engine, &eop_kernel, &utc, &location, &rs_config) {
                Ok(info) => {
                    println!(
                        "Hora: {} (position {} of 24)",
                        info.hora.name(),
                        info.hora_index
                    );
                    println!("  Start: {}", info.start);
                    println!("  End:   {}", info.end);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Ghatika(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&args.bsp, &args.lsk);
            let eop_kernel = load_eop(&args.eop);
            let location = GeoLocation::new(args.lat, args.lon, args.alt);
            let rs_config = RiseSetConfig::default();
            match dhruv_search::ghatika_for_date(&engine, &eop_kernel, &utc, &location, &rs_config)
            {
                Ok(info) => {
                    println!("Ghatika: {}/60", info.value);
                    println!("  Start: {}", info.start);
                    println!("  End:   {}", info.end);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Sphutas(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let system = require_aya_system(args.ayanamsha);
            let engine = load_engine(&args.bsp, &args.lsk);
            let eop_kernel = load_eop(&args.eop);
            let location = GeoLocation::new(args.lat, args.lon, args.alt);

            // Get graha sidereal longitudes
            let jd_tdb = utc_to_jd_tdb_with_policy_and_eop(
                &utc,
                engine.lsk(),
                Some(&eop_kernel),
                time_policy,
            );
            let graha_lons = dhruv_search::graha_longitudes(
                &engine,
                jd_tdb,
                &dhruv_search::GrahaLongitudesConfig::sidereal(system, args.nutation),
            )
            .unwrap_or_else(|e| {
                eprintln!("Error computing graha longitudes: {e}");
                std::process::exit(1);
            });

            // Get lagna (sidereal)
            let jd_utc = jd_tdb; // approximate; for more precision would use LSK
            let asc_rad =
                dhruv_vedic_base::lagna_longitude_rad(engine.lsk(), &eop_kernel, &location, jd_utc)
                    .unwrap_or_else(|e| {
                        eprintln!("Error computing lagna: {e}");
                        std::process::exit(1);
                    });
            let t = dhruv_vedic_base::jd_tdb_to_centuries(jd_tdb);
            let aya = dhruv_vedic_base::ayanamsha_deg(system, t, args.nutation);
            let lagna_sid = (asc_rad.to_degrees() - aya).rem_euclid(360.0);

            // Get 8th lord longitude
            let lagna_rashi_idx = (lagna_sid / 30.0).floor() as u8;
            let eighth_rashi_idx = dhruv_vedic_base::nth_rashi_from(lagna_rashi_idx, 8);
            let eighth_lord = dhruv_vedic_base::rashi_lord_by_index(eighth_rashi_idx).unwrap();
            let eighth_lord_lon = graha_lons.longitude(eighth_lord);

            // Build sphuta inputs (gulika = 0 for now, as it requires upagraha computation)
            let inputs = dhruv_vedic_base::SphutalInputs {
                sun: graha_lons.longitude(dhruv_vedic_base::Graha::Surya),
                moon: graha_lons.longitude(dhruv_vedic_base::Graha::Chandra),
                mars: graha_lons.longitude(dhruv_vedic_base::Graha::Mangal),
                jupiter: graha_lons.longitude(dhruv_vedic_base::Graha::Guru),
                venus: graha_lons.longitude(dhruv_vedic_base::Graha::Shukra),
                rahu: graha_lons.longitude(dhruv_vedic_base::Graha::Rahu),
                lagna: lagna_sid,
                eighth_lord: eighth_lord_lon,
                gulika: 0.0,
            };

            let results = dhruv_vedic_base::all_sphutas(&inputs);
            println!(
                "Sphutas for {} at {:.6}°N, {:.6}°E\n",
                args.date, args.lat, args.lon
            );
            println!(
                "Graha longitudes (sidereal, aya code={} {}):",
                args.ayanamsha,
                if args.nutation { "+nutation" } else { "" }
            );
            for graha in dhruv_vedic_base::graha::ALL_GRAHAS {
                println!(
                    "  {:8} {:>10.6}°",
                    graha.name(),
                    graha_lons.longitude(graha)
                );
            }
            println!("  {:8} {:>10.6}°\n", "Lagna", lagna_sid);
            println!("Sphutas:");
            for (sphuta, lon) in &results {
                let rashi_info = dhruv_vedic_base::rashi_from_longitude(*lon);
                println!(
                    "  {:24} {:>10.6}° ({} {}°{:02}'{:04.1}\")",
                    sphuta.name(),
                    lon,
                    rashi_info.rashi.name(),
                    rashi_info.dms.degrees,
                    rashi_info.dms.minutes,
                    rashi_info.dms.seconds,
                );
            }
            println!("\nNote: Gulika=0° (placeholder until upagraha computation is available).");
            println!(
                "  TriSphuta, ChatusSphuta, PanchaSphuta, SookshmaTrisphuta depend on Gulika."
            );
        }

        Commands::SpecialLagnas(args) => {
            let system = aya_system_from_code(args.ayanamsha)
                .unwrap_or_else(|| panic!("Invalid ayanamsha code: {}", args.ayanamsha));
            let utc = parse_utc(&args.date).unwrap_or_else(|e| panic!("Invalid date: {e}"));
            let engine = load_engine(&args.bsp, &args.lsk);
            let eop_kernel = load_eop(&args.eop);
            let location = GeoLocation::new(args.lat, args.lon, args.alt);
            let rs_config = RiseSetConfig::default();
            let config = SankrantiConfig::new(system, args.nutation);

            let result = dhruv_search::special_lagnas_for_date(
                &engine,
                &eop_kernel,
                &utc,
                &location,
                &rs_config,
                &config,
            )
            .unwrap_or_else(|e| panic!("special_lagnas_for_date failed: {e}"));

            println!(
                "Special Lagnas for {} at {:.6}°N, {:.6}°E\n",
                args.date, args.lat, args.lon
            );
            println!("  Bhava Lagna:     {:>12.6}°", result.bhava_lagna);
            println!("  Hora Lagna:      {:>12.6}°", result.hora_lagna);
            println!("  Ghati Lagna:     {:>12.6}°", result.ghati_lagna);
            println!("  Vighati Lagna:   {:>12.6}°", result.vighati_lagna);
            println!("  Varnada Lagna:   {:>12.6}°", result.varnada_lagna);
            println!("  Sree Lagna:      {:>12.6}°", result.sree_lagna);
            println!("  Pranapada Lagna: {:>12.6}°", result.pranapada_lagna);
            println!("  Indu Lagna:      {:>12.6}°", result.indu_lagna);
        }

        Commands::ArudhaPadas(args) => {
            let system = require_aya_system(args.ayanamsha);
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&args.bsp, &args.lsk);
            let eop_kernel = load_eop(&args.eop);
            let location = GeoLocation::new(args.lat, args.lon, args.alt);
            let bhava_config = bhava_config_from_cli(&args.bhava_behavior);
            let aya_config = SankrantiConfig::new(system, args.nutation);

            let results = dhruv_search::arudha_padas_for_date(
                &engine,
                &eop_kernel,
                &utc,
                &location,
                &bhava_config,
                &aya_config,
            )
            .unwrap_or_else(|e| {
                eprintln!("Error: {e}");
                std::process::exit(1);
            });

            println!(
                "Arudha Padas for {} at {:.6}°N, {:.6}°E\n",
                args.date, args.lat, args.lon
            );
            for r in &results {
                let rashi_info = dhruv_vedic_base::rashi_from_longitude(r.longitude_deg);
                println!(
                    "  {:16} {:>10.6}° ({} {}°{:02}'{:04.1}\")",
                    r.pada.name(),
                    r.longitude_deg,
                    rashi_info.rashi.name(),
                    rashi_info.dms.degrees,
                    rashi_info.dms.minutes,
                    rashi_info.dms.seconds,
                );
            }
        }

        Commands::Panchang(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let system = require_aya_system(args.ayanamsha);
            let engine = load_engine(&args.bsp, &args.lsk);
            let eop_kernel = load_eop(&args.eop);
            let location = GeoLocation::new(args.lat, args.lon, args.alt);
            let rs_config = RiseSetConfig::default();
            let config = SankrantiConfig::new(system, args.nutation);
            let include_mask = if let Some(raw) = args.include.as_deref() {
                parse_panchang_include_mask(raw).unwrap_or_else(|e| {
                    eprintln!("Invalid --include value: {e}");
                    std::process::exit(1);
                })
            } else {
                let mut mask = PANCHANG_INCLUDE_ALL_CORE;
                if args.calendar {
                    mask |= PANCHANG_INCLUDE_ALL_CALENDAR;
                }
                mask
            };
            let op = PanchangOperation {
                at_utc: utc,
                location,
                riseset_config: rs_config,
                sankranti_config: config,
                include_mask,
            };
            match dhruv_vedic_ops::panchang(&engine, &eop_kernel, &op) {
                Ok(info) => {
                    println!(
                        "Panchang for {} at {:.6}°N, {:.6}°E (mask=0x{:x})\n",
                        args.date, args.lat, args.lon, include_mask
                    );
                    if let Some(tithi) = info.tithi {
                        println!(
                            "Tithi:    {} (index {})",
                            tithi.tithi.name(),
                            tithi.tithi_index
                        );
                        println!(
                            "  Paksha: {}  Tithi in paksha: {}",
                            tithi.paksha.name(),
                            tithi.tithi_in_paksha
                        );
                        println!("  Start:  {}  End: {}", tithi.start, tithi.end);
                    }
                    if let Some(karana) = info.karana {
                        println!(
                            "Karana:   {} (sequence {})",
                            karana.karana.name(),
                            karana.karana_index
                        );
                        println!("  Start:  {}  End: {}", karana.start, karana.end);
                    }
                    if let Some(yoga) = info.yoga {
                        println!("Yoga:     {} (index {})", yoga.yoga.name(), yoga.yoga_index);
                        println!("  Start:  {}  End: {}", yoga.start, yoga.end);
                    }
                    if let Some(vaar) = info.vaar {
                        println!("Vaar:     {}", vaar.vaar.name());
                        println!("  Start:  {}  End: {}", vaar.start, vaar.end);
                    }
                    if let Some(hora) = info.hora {
                        println!(
                            "Hora:     {} (position {} of 24)",
                            hora.hora.name(),
                            hora.hora_index
                        );
                        println!("  Start:  {}  End: {}", hora.start, hora.end);
                    }
                    if let Some(ghatika) = info.ghatika {
                        println!("Ghatika:  {}/60", ghatika.value);
                        println!("  Start:  {}  End: {}", ghatika.start, ghatika.end);
                    }
                    if let Some(nakshatra) = info.nakshatra {
                        println!(
                            "Nakshatra: {} (index {}, pada {})",
                            nakshatra.nakshatra.name(),
                            nakshatra.nakshatra_index,
                            nakshatra.pada
                        );
                        println!("  Start:  {}  End: {}", nakshatra.start, nakshatra.end);
                    }
                    if let Some(m) = info.masa {
                        let adhika_str = if m.adhika { " (Adhika)" } else { "" };
                        println!("Masa:     {}{}", m.masa.name(), adhika_str);
                        println!("  Start:  {}  End: {}", m.start, m.end);
                    }
                    if let Some(a) = info.ayana {
                        println!("Ayana:    {}", a.ayana.name());
                        println!("  Start:  {}  End: {}", a.start, a.end);
                    }
                    if let Some(v) = info.varsha {
                        println!(
                            "Varsha:   {} (order {} of 60)",
                            v.samvatsara.name(),
                            v.order
                        );
                        println!("  Start:  {}  End: {}", v.start, v.end);
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Ashtakavarga(args) => {
            let system = require_aya_system(args.ayanamsha);
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&args.bsp, &args.lsk);
            let eop_kernel = load_eop(&args.eop);
            let location = GeoLocation::new(args.lat, args.lon, args.alt);
            let config = dhruv_search::sankranti_types::SankrantiConfig::new(system, args.nutation);

            let result =
                dhruv_search::ashtakavarga_for_date(&engine, &eop_kernel, &utc, &location, &config)
                    .unwrap_or_else(|e| {
                        eprintln!("Error: {e}");
                        std::process::exit(1);
                    });

            let graha_names = [
                "Sun", "Moon", "Mars", "Mercury", "Jupiter", "Venus", "Saturn",
            ];
            let rashi_names = [
                "Mes", "Vrs", "Mit", "Kar", "Sim", "Kan", "Tul", "Vri", "Dha", "Mak", "Kum", "Mee",
            ];

            println!(
                "Ashtakavarga for {} at {:.6}°N, {:.6}°E\n",
                args.date, args.lat, args.lon
            );

            // BAV tables
            println!("Bhinna Ashtakavarga (BAV):\n");
            print!("{:>10}", "");
            for name in &rashi_names {
                print!("{:>5}", name);
            }
            println!("  Total");
            println!("{}", "-".repeat(10 + 5 * 12 + 7));

            for (i, bav) in result.bavs.iter().enumerate() {
                print!("{:>10}", graha_names[i]);
                for &p in &bav.points {
                    print!("{:>5}", p);
                }
                let total: u8 = bav.points.iter().sum();
                println!("{:>7}", total);
            }

            println!(
                "\nBAV Contributor Matrix (row=rashi, cols=Sun Moon Mars Mercury Jupiter Venus Saturn Lagna):\n"
            );
            for (i, bav) in result.bavs.iter().enumerate() {
                println!("{:>10}", graha_names[i]);
                for (rashi_idx, row) in bav.contributors.iter().enumerate() {
                    print!("  {:>3} {:>2}:", rashi_names[rashi_idx], rashi_idx);
                    for &v in row {
                        print!(" {:>1}", v);
                    }
                    let row_sum: u8 = row.iter().sum();
                    println!("  | points={}", row_sum);
                }
            }

            // SAV
            println!("\nSarva Ashtakavarga (SAV):\n");
            print!("{:>10}", "");
            for name in &rashi_names {
                print!("{:>5}", name);
            }
            println!("  Total");
            println!("{}", "-".repeat(10 + 5 * 12 + 7));

            print!("{:>10}", "SAV");
            for &p in &result.sav.total_points {
                print!("{:>5}", p);
            }
            let sav_total: u16 = result.sav.total_points.iter().map(|&p| p as u16).sum();
            println!("{:>7}", sav_total);

            print!("{:>10}", "Trikona");
            for &p in &result.sav.after_trikona {
                print!("{:>5}", p);
            }
            let tri_total: u16 = result.sav.after_trikona.iter().map(|&p| p as u16).sum();
            println!("{:>7}", tri_total);

            print!("{:>10}", "Ekadhi");
            for &p in &result.sav.after_ekadhipatya {
                print!("{:>5}", p);
            }
            let ek_total: u16 = result.sav.after_ekadhipatya.iter().map(|&p| p as u16).sum();
            println!("{:>7}", ek_total);
        }

        Commands::Upagrahas(args) => {
            let system = require_aya_system(args.ayanamsha);
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&args.bsp, &args.lsk);
            let eop_kernel = load_eop(&args.eop);
            let location = GeoLocation::new(args.lat, args.lon, args.alt);
            let rs_config = RiseSetConfig::default();
            let config = dhruv_search::sankranti_types::SankrantiConfig::new(system, args.nutation);
            let upagraha_config = build_time_upagraha_config(&args.upagraha);

            let result = dhruv_search::all_upagrahas_for_date_with_config(
                &engine,
                &eop_kernel,
                &utc,
                &location,
                &rs_config,
                &config,
                &upagraha_config,
            )
            .unwrap_or_else(|e| {
                eprintln!("Error: {e}");
                std::process::exit(1);
            });

            println!(
                "Upagrahas for {} at {:.6}°N, {:.6}°E\n",
                args.date, args.lat, args.lon
            );
            println!("Time-based:");
            for (name, lon) in [
                ("Gulika", result.gulika),
                ("Maandi", result.maandi),
                ("Kaala", result.kaala),
                ("Mrityu", result.mrityu),
                ("Artha Prahara", result.artha_prahara),
                ("Yama Ghantaka", result.yama_ghantaka),
            ] {
                let rashi_info = dhruv_vedic_base::rashi_from_longitude(lon);
                println!(
                    "  {:16} {:>10.6}° ({} {}°{:02}'{:04.1}\")",
                    name,
                    lon,
                    rashi_info.rashi.name(),
                    rashi_info.dms.degrees,
                    rashi_info.dms.minutes,
                    rashi_info.dms.seconds,
                );
            }
            println!("\nSun-based:");
            for (name, lon) in [
                ("Dhooma", result.dhooma),
                ("Vyatipata", result.vyatipata),
                ("Parivesha", result.parivesha),
                ("Indra Chapa", result.indra_chapa),
                ("Upaketu", result.upaketu),
            ] {
                let rashi_info = dhruv_vedic_base::rashi_from_longitude(lon);
                println!(
                    "  {:16} {:>10.6}° ({} {}°{:02}'{:04.1}\")",
                    name,
                    lon,
                    rashi_info.rashi.name(),
                    rashi_info.dms.degrees,
                    rashi_info.dms.minutes,
                    rashi_info.dms.seconds,
                );
            }
        }
        Commands::GrahaPositions(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&args.bsp, &args.lsk);
            let eop_kernel = load_eop(&args.eop);

            if args.tropical {
                let jd_tdb = utc_to_jd_tdb_with_policy_and_eop(
                    &utc,
                    engine.lsk(),
                    Some(&eop_kernel),
                    time_policy,
                );
                let prec = parse_precession_model(&args.precession);
                let result = dhruv_search::graha_longitudes(
                    &engine,
                    jd_tdb,
                    &dhruv_search::GrahaLongitudesConfig::tropical_with_model(
                        args.nutation,
                        prec,
                        dhruv_frames::ReferencePlane::Ecliptic,
                    ),
                )
                .unwrap_or_else(|e| {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                });

                println!(
                    "Graha Positions for {} at {:.6}°N, {:.6}°E\n",
                    args.date, args.lat, args.lon
                );

                let graha_names = [
                    "Sun", "Moon", "Mars", "Mercury", "Jupiter", "Venus", "Saturn", "Rahu", "Ketu",
                ];
                println!("{:<10} {:>10}", "Graha", "Longitude");
                println!("{}", "-".repeat(22));

                for graha in ALL_GRAHAS {
                    let idx = graha.index() as usize;
                    let lon = result.longitude(graha);
                    println!("{:<10} {:>11.6}°", graha_names[idx], lon);
                }
            } else {
                let system = require_aya_system(args.ayanamsha);
                let location = GeoLocation::new(args.lat, args.lon, args.alt);
                let bhava_config = bhava_config_from_cli(&args.bhava_behavior);
                let prec = parse_precession_model(&args.precession);
                let aya_config = SankrantiConfig::new_with_model(system, args.nutation, prec);
                let gp_config = dhruv_search::GrahaPositionsConfig {
                    include_nakshatra: args.nakshatra,
                    include_lagna: args.lagna,
                    include_outer_planets: args.outer || !args.no_outer,
                    include_bhava: args.bhava,
                };

                let result = dhruv_search::graha_positions(
                    &engine,
                    &eop_kernel,
                    &utc,
                    &location,
                    &bhava_config,
                    &aya_config,
                    &gp_config,
                )
                .unwrap_or_else(|e| {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                });

                println!(
                    "Graha Positions for {} at {:.6}°N, {:.6}°E\n",
                    args.date, args.lat, args.lon
                );

                // Header
                let graha_names = [
                    "Sun", "Moon", "Mars", "Mercury", "Jupiter", "Venus", "Saturn", "Rahu", "Ketu",
                ];
                print!("{:<10} {:>10}  {:<10}", "Graha", "Longitude", "Rashi");
                if args.nakshatra {
                    print!("  {:<18} {:>4}", "Nakshatra", "Pada");
                }
                if args.bhava {
                    print!("  {:>5}", "Bhava");
                }
                println!();
                let width =
                    32 + if args.nakshatra { 24 } else { 0 } + if args.bhava { 7 } else { 0 };
                println!("{}", "-".repeat(width));

                let print_entry =
                    |name: &str, entry: &dhruv_search::GrahaEntry, force_bhava: Option<u8>| {
                        print!(
                            "{:<10} {:>11.6}°  {:<10}",
                            name,
                            entry.sidereal_longitude,
                            entry.rashi.name(),
                        );
                        if args.nakshatra {
                            print!(
                                "  {:<18} {:>4}",
                                entry.nakshatra.name(),
                                if entry.pada > 0 {
                                    entry.pada.to_string()
                                } else {
                                    "-".into()
                                },
                            );
                        }
                        if args.bhava {
                            let bh = force_bhava.unwrap_or(entry.bhava_number);
                            print!("  {:>5}", if bh > 0 { bh.to_string() } else { "-".into() },);
                        }
                        println!();
                    };

                for (i, entry) in result.grahas.iter().enumerate() {
                    print_entry(graha_names[i], entry, None);
                }

                if args.lagna {
                    print_entry("Lagna", &result.lagna, Some(1));
                }

                if gp_config.include_outer_planets {
                    let planet_names = ["Uranus", "Neptune", "Pluto"];
                    for (i, entry) in result.outer_planets.iter().enumerate() {
                        print_entry(planet_names[i], entry, None);
                    }
                }
            }
        }
        Commands::CoreBindus(args) => {
            let system = require_aya_system(args.ayanamsha);
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&args.bsp, &args.lsk);
            let eop_kernel = load_eop(&args.eop);
            let location = GeoLocation::new(args.lat, args.lon, args.alt);
            let bhava_config = bhava_config_from_cli(&args.bhava_behavior);
            let rs_config = RiseSetConfig::default();
            let aya_config = SankrantiConfig::new(system, args.nutation);
            let upagraha_config = build_time_upagraha_config(&args.upagraha);
            let bindus_config = dhruv_search::BindusConfig {
                include_nakshatra: args.nakshatra,
                include_bhava: args.bhava,
                upagraha_config,
            };

            let result = dhruv_search::core_bindus(
                &engine,
                &eop_kernel,
                &utc,
                &location,
                &bhava_config,
                &rs_config,
                &aya_config,
                &bindus_config,
            )
            .unwrap_or_else(|e| {
                eprintln!("Error: {e}");
                std::process::exit(1);
            });

            println!(
                "Core Bindus for {} at {:.6}°N, {:.6}°E\n",
                args.date, args.lat, args.lon
            );

            // Header
            print!("{:<16} {:>10}  {:<10}", "Name", "Longitude", "Rashi");
            if args.nakshatra {
                print!("  {:<18} {:>4}", "Nakshatra", "Pada");
            }
            if args.bhava {
                print!("  {:>5}", "Bhava");
            }
            println!();
            let width = 38 + if args.nakshatra { 24 } else { 0 } + if args.bhava { 7 } else { 0 };
            println!("{}", "-".repeat(width));

            let print_entry = |name: &str, entry: &dhruv_search::GrahaEntry| {
                print!(
                    "{:<16} {:>11.6}°  {:<10}",
                    name,
                    entry.sidereal_longitude,
                    entry.rashi.name(),
                );
                if args.nakshatra {
                    print!(
                        "  {:<18} {:>4}",
                        entry.nakshatra.name(),
                        if entry.pada > 0 {
                            entry.pada.to_string()
                        } else {
                            "-".into()
                        },
                    );
                }
                if args.bhava {
                    print!(
                        "  {:>5}",
                        if entry.bhava_number > 0 {
                            entry.bhava_number.to_string()
                        } else {
                            "-".into()
                        },
                    );
                }
                println!();
            };

            println!("\nArudha Padas:");
            let pada_names = [
                "A1 (Lagna)",
                "A2 (Dhana)",
                "A3 (Sahaja)",
                "A4 (Sukha)",
                "A5 (Putra)",
                "A6 (Ari)",
                "A7 (Dara)",
                "A8 (Mrityu)",
                "A9 (Dharma)",
                "A10 (Karma)",
                "A11 (Labha)",
                "A12 (UL)",
            ];
            for (i, entry) in result.arudha_padas.iter().enumerate() {
                print_entry(pada_names[i], entry);
            }

            println!("\nSensitive Points:");
            print_entry("Bhrigu Bindu", &result.bhrigu_bindu);
            print_entry("Pranapada", &result.pranapada_lagna);
            print_entry("Gulika", &result.gulika);
            print_entry("Maandi", &result.maandi);
            print_entry("Hora Lagna", &result.hora_lagna);
            print_entry("Ghati Lagna", &result.ghati_lagna);
            print_entry("Sree Lagna", &result.sree_lagna);
        }
        Commands::Drishti(args) => {
            let system = require_aya_system(args.ayanamsha);
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&args.bsp, &args.lsk);
            let eop_kernel = load_eop(&args.eop);
            let location = GeoLocation::new(args.lat, args.lon, args.alt);
            let bhava_config = bhava_config_from_cli(&args.bhava_behavior);
            let rs_config = RiseSetConfig::default();
            let aya_config = SankrantiConfig::new(system, args.nutation);
            let drishti_config = dhruv_search::DrishtiConfig {
                include_bhava: args.bhava,
                include_lagna: args.lagna,
                include_bindus: args.bindus,
            };

            let result = dhruv_search::drishti_for_date(
                &engine,
                &eop_kernel,
                &utc,
                &location,
                &bhava_config,
                &rs_config,
                &aya_config,
                &drishti_config,
            )
            .unwrap_or_else(|e| {
                eprintln!("Error: {e}");
                std::process::exit(1);
            });

            let graha_names = [
                "Sun", "Moon", "Mars", "Merc", "Jup", "Ven", "Sat", "Rahu", "Ketu",
            ];

            println!(
                "Graha Drishti for {} at {:.6}°N, {:.6}°E\n",
                args.date, args.lat, args.lon
            );

            // 9x9 graha-to-graha matrix
            println!("Graha-to-Graha (total virupa):");
            print!("{:<8}", "From\\To");
            for name in &graha_names {
                print!("{:>8}", name);
            }
            println!();
            println!("{}", "-".repeat(8 + 8 * 9));
            for (i, name) in graha_names.iter().enumerate() {
                print!("{:<8}", name);
                for j in 0..9 {
                    let v = result.graha_to_graha.entries[i][j].total_virupa;
                    if i == j {
                        print!("{:>8}", "-");
                    } else {
                        print!("{:>8.1}", v);
                    }
                }
                println!();
            }

            if args.lagna {
                println!("\nGraha-to-Lagna:");
                println!(
                    "{:<8} {:>8} {:>8} {:>8} {:>8}",
                    "Graha", "Dist", "Base", "Special", "Total"
                );
                println!("{}", "-".repeat(44));
                for (i, name) in graha_names.iter().enumerate() {
                    let e = &result.graha_to_lagna[i];
                    println!(
                        "{:<8} {:>7.1}° {:>8.1} {:>8.1} {:>8.1}",
                        name, e.angular_distance, e.base_virupa, e.special_virupa, e.total_virupa
                    );
                }
            }

            if args.bhava {
                println!("\nGraha-to-Bhava Cusps (total virupa):");
                print!("{:<8}", "Graha");
                for b in 1..=12 {
                    print!("{:>6}", format!("B{b}"));
                }
                println!();
                println!("{}", "-".repeat(8 + 6 * 12));
                for (i, name) in graha_names.iter().enumerate() {
                    print!("{:<8}", name);
                    for j in 0..12 {
                        print!("{:>6.1}", result.graha_to_bhava[i][j].total_virupa);
                    }
                    println!();
                }
            }

            if args.bindus {
                let bindu_names = [
                    "A1", "A2", "A3", "A4", "A5", "A6", "A7", "A8", "A9", "A10", "A11", "A12",
                    "BhrBin", "Prana", "Gulik", "Maand", "HoraL", "GhatiL", "SreeL",
                ];
                println!("\nGraha-to-Core Bindus (total virupa):");
                print!("{:<8}", "Graha");
                for name in &bindu_names {
                    print!("{:>7}", name);
                }
                println!();
                println!("{}", "-".repeat(8 + 7 * 19));
                for (i, name) in graha_names.iter().enumerate() {
                    print!("{:<8}", name);
                    for j in 0..19 {
                        print!("{:>7.1}", result.graha_to_bindus[i][j].total_virupa);
                    }
                    println!();
                }
            }
        }
        Commands::Kundali(args) => {
            if args.dasha_snapshot_date.is_some() && args.dasha_systems.is_none() {
                eprintln!("Error: --dasha-snapshot-date requires --dasha-systems");
                std::process::exit(1);
            }
            let system = require_aya_system(args.ayanamsha);
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&args.bsp, &args.lsk);
            let eop_kernel = load_eop(&args.eop);
            let location = GeoLocation::new(args.lat, args.lon, args.alt);
            let bhava_config = bhava_config_from_cli(&args.bhava_behavior);
            let rs_config = RiseSetConfig::default();
            let aya_config = SankrantiConfig::new(system, args.nutation);

            let node_dignity_policy = match args.node_policy.as_deref() {
                Some("sama") => NodeDignityPolicy::AlwaysSama,
                Some("sign-lord") | None => NodeDignityPolicy::SignLordBased,
                Some(other) => {
                    eprintln!("Error: unknown --node-policy '{other}', use 'sign-lord' or 'sama'");
                    std::process::exit(1);
                }
            };

            let requested_amsha_scope = amsha_scope(
                args.amsha_include_bhava_cusps,
                args.amsha_include_arudha_padas,
                args.amsha_include_upagrahas,
                args.amsha_include_sphutas,
                args.amsha_include_special_lagnas,
                !args.amsha_no_outer_planets,
            );
            let requested_amsha_selection = args
                .amsha
                .as_deref()
                .map(parse_amsha_specs)
                .map(|requests| amsha_selection_from_requests(&requests));

            let mut resolved = resolve_kundali_flags(
                args.all,
                args.include_graha,
                args.include_bindus,
                args.include_drishti,
                args.include_ashtakavarga,
                args.include_upagrahas,
                args.include_special_lagnas,
                args.include_amshas,
                args.include_shadbala,
                args.include_bhavabala,
                args.include_vimsopaka,
                args.include_avastha,
                args.include_charakaraka,
                args.include_panchang,
                args.include_calendar,
            );
            if requested_amsha_selection.is_some() || has_amsha_scope(&requested_amsha_scope) {
                resolved.include_amshas = true;
            }

            let snapshot_time = args.dasha_snapshot_date.as_ref().map(|d| {
                let snap_utc = parse_utc(d).unwrap_or_else(|e| {
                    eprintln!("{e}");
                    std::process::exit(1);
                });
                dhruv_search::DashaSnapshotTime::Utc(snap_utc)
            });

            let full_config = build_kundali_config(
                &resolved,
                args.dasha_systems.as_deref(),
                args.dasha_max_level,
                snapshot_time,
                node_dignity_policy,
                parse_charakaraka_scheme(&args.charakaraka_scheme),
                requested_amsha_selection.as_ref(),
                &requested_amsha_scope,
                build_time_upagraha_config(&args.upagraha),
                !args.no_outer,
            );

            let result = dhruv_search::full_kundali_for_date(
                &engine,
                &eop_kernel,
                &utc,
                &location,
                &bhava_config,
                &rs_config,
                &aya_config,
                &full_config,
            )
            .unwrap_or_else(|e| {
                eprintln!("Error: {e}");
                std::process::exit(1);
            });

            println!(
                "Kundali for {} at {:.6}°N, {:.6}°E\n",
                args.date, args.lat, args.lon
            );
            print_kundali(&mut std::io::stdout(), &result, &resolved).unwrap_or_else(|e| {
                eprintln!("Error writing output: {e}");
                std::process::exit(1);
            });
        }

        Commands::PrevPurnima { date, bsp, lsk } => {
            let utc = parse_utc(&date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&bsp, &lsk);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            let op = LunarPhaseOperation {
                kind: LunarPhaseKind::Purnima,
                query: LunarPhaseQuery::Prev { at_jd_tdb: jd_tdb },
            };
            match dhruv_search::lunar_phase(&engine, &op) {
                Ok(LunarPhaseResult::Single(Some(ev))) => {
                    println!("Previous Purnima: {}", ev.utc);
                    println!(
                        "  Moon lon: {:.6} deg  Sun lon: {:.6} deg",
                        ev.moon_longitude_deg, ev.sun_longitude_deg
                    );
                }
                Ok(LunarPhaseResult::Single(None)) => println!("No Purnima found in search range"),
                Ok(LunarPhaseResult::Many(_)) => {
                    eprintln!("Error: unexpected search result shape");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::PrevAmavasya { date, bsp, lsk } => {
            let utc = parse_utc(&date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&bsp, &lsk);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            let op = LunarPhaseOperation {
                kind: LunarPhaseKind::Amavasya,
                query: LunarPhaseQuery::Prev { at_jd_tdb: jd_tdb },
            };
            match dhruv_search::lunar_phase(&engine, &op) {
                Ok(LunarPhaseResult::Single(Some(ev))) => {
                    println!("Previous Amavasya: {}", ev.utc);
                    println!(
                        "  Moon lon: {:.6} deg  Sun lon: {:.6} deg",
                        ev.moon_longitude_deg, ev.sun_longitude_deg
                    );
                }
                Ok(LunarPhaseResult::Single(None)) => println!("No Amavasya found in search range"),
                Ok(LunarPhaseResult::Many(_)) => {
                    eprintln!("Error: unexpected search result shape");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::PrevSankranti(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let system = require_aya_system(args.ayanamsha);
            let engine = load_engine(&args.bsp, &args.lsk);
            let config = SankrantiConfig::new(system, args.nutation);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            let op = SankrantiOperation {
                target: SankrantiTarget::Any,
                config,
                query: SankrantiQuery::Prev { at_jd_tdb: jd_tdb },
            };
            match dhruv_search::sankranti(&engine, &op) {
                Ok(SankrantiResult::Single(Some(ev))) => {
                    println!("Previous Sankranti: {}", ev.rashi.name());
                    println!("  Time: {}", ev.utc);
                    println!(
                        "  Sidereal lon: {:.6} deg  Tropical lon: {:.6} deg",
                        ev.sun_sidereal_longitude_deg, ev.sun_tropical_longitude_deg
                    );
                }
                Ok(SankrantiResult::Single(None)) => println!("No Sankranti found in search range"),
                Ok(SankrantiResult::Many(_)) => {
                    eprintln!("Error: unexpected search result shape");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::SearchPurnimas(args) => {
            let s = parse_utc(&args.start).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let e = parse_utc(&args.end).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&args.bsp, &args.lsk);
            let jd_start = utc_to_jd_tdb_with_policy(&s, engine.lsk(), time_policy);
            let jd_end = utc_to_jd_tdb_with_policy(&e, engine.lsk(), time_policy);
            let op = LunarPhaseOperation {
                kind: LunarPhaseKind::Purnima,
                query: LunarPhaseQuery::Range {
                    start_jd_tdb: jd_start,
                    end_jd_tdb: jd_end,
                },
            };
            match dhruv_search::lunar_phase(&engine, &op) {
                Ok(LunarPhaseResult::Many(events)) => {
                    println!("Found {} Purnimas:", events.len());
                    for ev in &events {
                        println!(
                            "  {}  Moon: {:.6}°  Sun: {:.6}°",
                            ev.utc, ev.moon_longitude_deg, ev.sun_longitude_deg
                        );
                    }
                }
                Ok(LunarPhaseResult::Single(_)) => {
                    eprintln!("Error: unexpected search result shape");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::SearchAmavasyas(args) => {
            let s = parse_utc(&args.start).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let e = parse_utc(&args.end).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&args.bsp, &args.lsk);
            let jd_start = utc_to_jd_tdb_with_policy(&s, engine.lsk(), time_policy);
            let jd_end = utc_to_jd_tdb_with_policy(&e, engine.lsk(), time_policy);
            let op = LunarPhaseOperation {
                kind: LunarPhaseKind::Amavasya,
                query: LunarPhaseQuery::Range {
                    start_jd_tdb: jd_start,
                    end_jd_tdb: jd_end,
                },
            };
            match dhruv_search::lunar_phase(&engine, &op) {
                Ok(LunarPhaseResult::Many(events)) => {
                    println!("Found {} Amavasyas:", events.len());
                    for ev in &events {
                        println!(
                            "  {}  Moon: {:.6}°  Sun: {:.6}°",
                            ev.utc, ev.moon_longitude_deg, ev.sun_longitude_deg
                        );
                    }
                }
                Ok(LunarPhaseResult::Single(_)) => {
                    eprintln!("Error: unexpected search result shape");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::SearchSankrantis(args) => {
            let s = parse_utc(&args.start).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let e = parse_utc(&args.end).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let system = require_aya_system(args.ayanamsha);
            let engine = load_engine(&args.bsp, &args.lsk);
            let config = SankrantiConfig::new(system, args.nutation);
            let jd_start = utc_to_jd_tdb_with_policy(&s, engine.lsk(), time_policy);
            let jd_end = utc_to_jd_tdb_with_policy(&e, engine.lsk(), time_policy);
            let op = SankrantiOperation {
                target: SankrantiTarget::Any,
                config,
                query: SankrantiQuery::Range {
                    start_jd_tdb: jd_start,
                    end_jd_tdb: jd_end,
                },
            };
            match dhruv_search::sankranti(&engine, &op) {
                Ok(SankrantiResult::Many(events)) => {
                    println!("Found {} Sankrantis:", events.len());
                    for ev in &events {
                        println!(
                            "  {} at {}  sid: {:.6}°  trop: {:.6}°",
                            ev.rashi.name(),
                            ev.utc,
                            ev.sun_sidereal_longitude_deg,
                            ev.sun_tropical_longitude_deg
                        );
                    }
                }
                Ok(SankrantiResult::Single(_)) => {
                    eprintln!("Error: unexpected search result shape");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::NextSpecificSankranti(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let target = rashi_from_index(args.rashi);
            let system = require_aya_system(args.ayanamsha);
            let engine = load_engine(&args.bsp, &args.lsk);
            let config = SankrantiConfig::new(system, args.nutation);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            let op = SankrantiOperation {
                target: SankrantiTarget::SpecificRashi(target),
                config,
                query: SankrantiQuery::Next { at_jd_tdb: jd_tdb },
            };
            match dhruv_search::sankranti(&engine, &op) {
                Ok(SankrantiResult::Single(Some(ev))) => {
                    println!("Next {} Sankranti: {}", ev.rashi.name(), ev.utc);
                    println!(
                        "  Sidereal lon: {:.6}°  Tropical lon: {:.6}°",
                        ev.sun_sidereal_longitude_deg, ev.sun_tropical_longitude_deg
                    );
                }
                Ok(SankrantiResult::Single(None)) => {
                    println!("No {} Sankranti found", target.name())
                }
                Ok(SankrantiResult::Many(_)) => {
                    eprintln!("Error: unexpected search result shape");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::PrevSpecificSankranti(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let target = rashi_from_index(args.rashi);
            let system = require_aya_system(args.ayanamsha);
            let engine = load_engine(&args.bsp, &args.lsk);
            let config = SankrantiConfig::new(system, args.nutation);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            let op = SankrantiOperation {
                target: SankrantiTarget::SpecificRashi(target),
                config,
                query: SankrantiQuery::Prev { at_jd_tdb: jd_tdb },
            };
            match dhruv_search::sankranti(&engine, &op) {
                Ok(SankrantiResult::Single(Some(ev))) => {
                    println!("Previous {} Sankranti: {}", ev.rashi.name(), ev.utc);
                    println!(
                        "  Sidereal lon: {:.6}°  Tropical lon: {:.6}°",
                        ev.sun_sidereal_longitude_deg, ev.sun_tropical_longitude_deg
                    );
                }
                Ok(SankrantiResult::Single(None)) => {
                    println!("No {} Sankranti found", target.name())
                }
                Ok(SankrantiResult::Many(_)) => {
                    eprintln!("Error: unexpected search result shape");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::AyanamshaCompute(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let system = require_aya_system(args.ayanamsha);
            let engine = load_engine(&args.bsp, &args.lsk);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            let t = jd_tdb_to_centuries(jd_tdb);
            let cat = args.catalog.map(|p| {
                TaraCatalog::load(&p).unwrap_or_else(|e| {
                    eprintln!("Failed to load star catalog: {e}");
                    std::process::exit(1);
                })
            });
            let aya = match args.mode.as_str() {
                "mean" => ayanamsha_mean_deg_with_catalog(system, t, cat.as_ref()),
                "true" => ayanamsha_true_deg(system, t, args.delta_psi_arcsec),
                "unified" => ayanamsha_deg_with_catalog(system, t, args.nutation, cat.as_ref()),
                _ => {
                    eprintln!("Invalid mode: {} (unified|mean|true)", args.mode);
                    std::process::exit(1);
                }
            };
            println!(
                "Ayanamsha ({:?}, {}): {:.6}°{}{}{}",
                system,
                args.mode,
                aya,
                if args.mode == "unified" && args.nutation {
                    " (with nutation)"
                } else {
                    ""
                },
                if cat.is_some() {
                    " (with star catalog)"
                } else {
                    ""
                },
                if args.mode == "true" {
                    " (delta-psi provided)"
                } else {
                    ""
                }
            );
        }

        Commands::NutationCompute { date, bsp, lsk } => {
            let utc = parse_utc(&date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&bsp, &lsk);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            let t = jd_tdb_to_centuries(jd_tdb);
            let (dpsi, deps) = nutation_iau2000b(t);
            println!("Nutation at {}:", date);
            println!("  dpsi (longitude): {:.6} arcsec", dpsi);
            println!("  deps (obliquity): {:.6} arcsec", deps);
        }

        Commands::Sunrise(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&args.bsp, &args.lsk);
            let eop_kernel = load_eop(&args.eop);
            let location = GeoLocation::new(args.lat, args.lon, args.alt);
            let rs_config = RiseSetConfig::default();
            let jd_utc = utc_to_jd_utc(&utc);
            let jd_noon = dhruv_vedic_base::approximate_local_noon_jd(
                dhruv_vedic_base::utc_day_start_jd(jd_utc),
                location.longitude_deg,
            );

            let events = dhruv_vedic_base::compute_all_events(
                &engine,
                engine.lsk(),
                &eop_kernel,
                &location,
                jd_noon,
                &rs_config,
            )
            .unwrap_or_else(|e| {
                eprintln!("Error: {e}");
                std::process::exit(1);
            });

            println!(
                "Rise/Set events for {} at {:.6}°N, {:.6}°E:\n",
                args.date, args.lat, args.lon
            );
            for result in &events {
                match result {
                    RiseSetResult::Event { jd_tdb, event } => {
                        println!("  {:20} JD TDB {:.6}", format!("{event:?}"), jd_tdb);
                    }
                    RiseSetResult::NeverRises => println!("  Sun never rises (polar night)"),
                    RiseSetResult::NeverSets => println!("  Sun never sets (midnight sun)"),
                }
            }
        }

        Commands::Bhavas(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&args.bsp, &args.lsk);
            let eop_kernel = load_eop(&args.eop);
            let location = GeoLocation::new(args.lat, args.lon, args.alt);
            let bhava_config = bhava_config_from_cli(&args.bhava_behavior);
            let jd_utc = utc_to_jd_utc(&utc);
            let output_label = if args.sidereal {
                "sidereal"
            } else {
                "tropical"
            };
            let result = if args.sidereal {
                let system = require_aya_system(args.ayanamsha);
                let aya_config = SankrantiConfig::new(system, args.nutation);
                dhruv_search::sidereal_bhavas_for_date(
                    &engine,
                    &eop_kernel,
                    &utc,
                    &location,
                    &bhava_config,
                    &aya_config,
                )
            } else {
                dhruv_vedic_base::compute_bhavas(
                    &engine,
                    engine.lsk(),
                    &eop_kernel,
                    &location,
                    jd_utc,
                    &bhava_config,
                )
                .map_err(dhruv_search::SearchError::from)
            }
            .unwrap_or_else(|e| {
                eprintln!("Error: {e}");
                std::process::exit(1);
            });

            println!(
                "Bhavas for {} at {:.6}°N, {:.6}°E\n",
                args.date, args.lat, args.lon
            );
            print_bhava_table(
                &format!("Configured bhava system ({output_label})"),
                &result,
            );
            if bhava_config.include_rashi_bhava_results {
                let rashi_bhava = rashi_bhava_result_from_lagna(result.lagna_deg);
                println!();
                print_bhava_table(
                    &format!("Rashi-bhava / equal-house sibling ({output_label})"),
                    &rashi_bhava,
                );
            }
        }

        Commands::LagnaCompute(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&args.bsp, &args.lsk);
            let eop_kernel = load_eop(&args.eop);
            let location = GeoLocation::new(args.lat, args.lon, args.alt);
            let jd_utc = utc_to_jd_utc(&utc);
            let bhava_config = bhava_config_from_cli(&args.bhava_behavior);
            let (lagna_label, lagna_deg, mc_label, mc_deg) = if args.sidereal {
                let system = require_aya_system(args.ayanamsha);
                let aya_config = SankrantiConfig::new(system, args.nutation);
                let lagna = dhruv_search::sidereal_lagna_for_date(
                    &engine,
                    &eop_kernel,
                    &utc,
                    &location,
                    &aya_config,
                )
                .unwrap_or_else(|e| {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                });
                let mc = dhruv_search::sidereal_mc_for_date(
                    &engine,
                    &eop_kernel,
                    &utc,
                    &location,
                    &bhava_config,
                    &aya_config,
                )
                .unwrap_or_else(|e| {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                });
                ("sidereal", lagna, "sidereal", mc)
            } else {
                let lagna = dhruv_vedic_base::lagna_longitude_rad(
                    engine.lsk(),
                    &eop_kernel,
                    &location,
                    jd_utc,
                )
                .unwrap_or_else(|e| {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                })
                .to_degrees()
                .rem_euclid(360.0);
                let mc = dhruv_vedic_base::mc_longitude_rad(
                    engine.lsk(),
                    &eop_kernel,
                    &location,
                    jd_utc,
                )
                .unwrap_or_else(|e| {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                })
                .to_degrees()
                .rem_euclid(360.0);
                ("tropical", lagna, "tropical", mc)
            };
            let ramc = dhruv_vedic_base::ramc_rad(engine.lsk(), &eop_kernel, &location, jd_utc)
                .unwrap_or_else(|e| {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                });

            println!("Lagna ({lagna_label}): {:.6}°", lagna_deg);
            println!("MC ({mc_label}):    {:.6}°", mc_deg);
            println!(
                "RAMC:               {:.6}°",
                ramc.to_degrees().rem_euclid(360.0)
            );
        }

        Commands::LunarNode(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&args.bsp, &args.lsk);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            let lunar_node = parse_lunar_node(&args.node);
            let node_mode = parse_node_mode(&args.mode);
            let backend = match args.backend.as_str() {
                "engine" => NodeBackend::Engine,
                "analytic" => NodeBackend::Analytic,
                _ => {
                    eprintln!("Invalid backend: {} (engine|analytic)", args.backend);
                    std::process::exit(1);
                }
            };
            let op = NodeOperation {
                node: lunar_node,
                mode: node_mode,
                backend,
                at_jd_tdb: jd_tdb,
            };
            let lon = dhruv_vedic_ops::lunar_node(&engine, &op).unwrap_or_else(|e| {
                eprintln!("Error: {e}");
                std::process::exit(1);
            });
            println!(
                "{:?} ({:?}, {:?}): {:.6}°",
                lunar_node, node_mode, backend, lon
            );
        }

        Commands::Conjunction(args) => {
            let b1 = require_body(args.body1);
            let b2 = require_body(args.body2);
            let engine = load_engine(&args.bsp, &args.lsk);
            let config = ConjunctionConfig::conjunction(1.0);
            let query = match args.mode.as_str() {
                "next" => {
                    let date = args.date.as_deref().unwrap_or_else(|| {
                        eprintln!("--date is required when --mode next");
                        std::process::exit(1);
                    });
                    let utc = parse_utc(date).unwrap_or_else(|e| {
                        eprintln!("{e}");
                        std::process::exit(1);
                    });
                    let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
                    ConjunctionQuery::Next { at_jd_tdb: jd_tdb }
                }
                "prev" => {
                    let date = args.date.as_deref().unwrap_or_else(|| {
                        eprintln!("--date is required when --mode prev");
                        std::process::exit(1);
                    });
                    let utc = parse_utc(date).unwrap_or_else(|e| {
                        eprintln!("{e}");
                        std::process::exit(1);
                    });
                    let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
                    ConjunctionQuery::Prev { at_jd_tdb: jd_tdb }
                }
                "range" => {
                    let start = args.start.as_deref().unwrap_or_else(|| {
                        eprintln!("--start is required when --mode range");
                        std::process::exit(1);
                    });
                    let end = args.end.as_deref().unwrap_or_else(|| {
                        eprintln!("--end is required when --mode range");
                        std::process::exit(1);
                    });
                    let utc_start = parse_utc(start).unwrap_or_else(|e| {
                        eprintln!("{e}");
                        std::process::exit(1);
                    });
                    let utc_end = parse_utc(end).unwrap_or_else(|e| {
                        eprintln!("{e}");
                        std::process::exit(1);
                    });
                    let jd_start = utc_to_jd_tdb_with_policy(&utc_start, engine.lsk(), time_policy);
                    let jd_end = utc_to_jd_tdb_with_policy(&utc_end, engine.lsk(), time_policy);
                    ConjunctionQuery::Range {
                        start_jd_tdb: jd_start,
                        end_jd_tdb: jd_end,
                    }
                }
                _ => {
                    eprintln!("Invalid mode: {}", args.mode);
                    std::process::exit(1);
                }
            };
            let op = ConjunctionOperation {
                body1: b1,
                body2: b2,
                config,
                query,
            };
            match dhruv_search::conjunction(&engine, &op) {
                Ok(ConjunctionResult::Single(Some(ev))) => {
                    let label = match args.mode.as_str() {
                        "next" => "Next conjunction",
                        "prev" => "Previous conjunction",
                        _ => "Conjunction",
                    };
                    print_conjunction_event(label, &ev);
                }
                Ok(ConjunctionResult::Single(None)) => {
                    println!("No conjunction found");
                }
                Ok(ConjunctionResult::Many(events)) => {
                    println!("Found {} conjunctions:", events.len());
                    for ev in &events {
                        print_conjunction_event("  Conjunction", ev);
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::NextConjunction(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let b1 = require_body(args.body1);
            let b2 = require_body(args.body2);
            let engine = load_engine(&args.bsp, &args.lsk);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            let config = ConjunctionConfig::conjunction(1.0);
            let op = ConjunctionOperation {
                body1: b1,
                body2: b2,
                config,
                query: ConjunctionQuery::Next { at_jd_tdb: jd_tdb },
            };
            match dhruv_search::conjunction(&engine, &op) {
                Ok(ConjunctionResult::Single(Some(ev))) => {
                    print_conjunction_event("Next conjunction", &ev)
                }
                Ok(ConjunctionResult::Single(None)) => println!("No conjunction found"),
                Ok(ConjunctionResult::Many(_)) => {
                    eprintln!("Error: unexpected search result shape");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::PrevConjunction(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let b1 = require_body(args.body1);
            let b2 = require_body(args.body2);
            let engine = load_engine(&args.bsp, &args.lsk);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            let config = ConjunctionConfig::conjunction(1.0);
            let op = ConjunctionOperation {
                body1: b1,
                body2: b2,
                config,
                query: ConjunctionQuery::Prev { at_jd_tdb: jd_tdb },
            };
            match dhruv_search::conjunction(&engine, &op) {
                Ok(ConjunctionResult::Single(Some(ev))) => {
                    print_conjunction_event("Previous conjunction", &ev)
                }
                Ok(ConjunctionResult::Single(None)) => println!("No conjunction found"),
                Ok(ConjunctionResult::Many(_)) => {
                    eprintln!("Error: unexpected search result shape");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::SearchConjunctions(args) => {
            let s = parse_utc(&args.start).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let e = parse_utc(&args.end).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let b1 = require_body(args.body1);
            let b2 = require_body(args.body2);
            let engine = load_engine(&args.bsp, &args.lsk);
            let jd_start = utc_to_jd_tdb_with_policy(&s, engine.lsk(), time_policy);
            let jd_end = utc_to_jd_tdb_with_policy(&e, engine.lsk(), time_policy);
            let config = ConjunctionConfig::conjunction(1.0);
            let op = ConjunctionOperation {
                body1: b1,
                body2: b2,
                config,
                query: ConjunctionQuery::Range {
                    start_jd_tdb: jd_start,
                    end_jd_tdb: jd_end,
                },
            };
            match dhruv_search::conjunction(&engine, &op) {
                Ok(ConjunctionResult::Many(events)) => {
                    println!("Found {} conjunctions:", events.len());
                    for ev in &events {
                        print_conjunction_event("  Conjunction", ev);
                    }
                }
                Ok(ConjunctionResult::Single(_)) => {
                    eprintln!("Error: unexpected search result shape");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Grahan(args) => {
            let kind = match args.kind.as_str() {
                "chandra" => GrahanKind::Chandra,
                "surya" => GrahanKind::Surya,
                _ => {
                    eprintln!("Invalid kind: {}", args.kind);
                    std::process::exit(1);
                }
            };
            let engine = load_engine(&args.bsp, &args.lsk);
            let config = GrahanConfig {
                include_penumbral: !args.no_penumbral,
                include_peak_details: !args.no_peak_details,
            };
            let query = match args.mode.as_str() {
                "next" => {
                    let date = args.date.as_deref().unwrap_or_else(|| {
                        eprintln!("--date is required when --mode next");
                        std::process::exit(1);
                    });
                    let utc = parse_utc(date).unwrap_or_else(|e| {
                        eprintln!("{e}");
                        std::process::exit(1);
                    });
                    let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
                    GrahanQuery::Next { at_jd_tdb: jd_tdb }
                }
                "prev" => {
                    let date = args.date.as_deref().unwrap_or_else(|| {
                        eprintln!("--date is required when --mode prev");
                        std::process::exit(1);
                    });
                    let utc = parse_utc(date).unwrap_or_else(|e| {
                        eprintln!("{e}");
                        std::process::exit(1);
                    });
                    let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
                    GrahanQuery::Prev { at_jd_tdb: jd_tdb }
                }
                "range" => {
                    let start = args.start.as_deref().unwrap_or_else(|| {
                        eprintln!("--start is required when --mode range");
                        std::process::exit(1);
                    });
                    let end = args.end.as_deref().unwrap_or_else(|| {
                        eprintln!("--end is required when --mode range");
                        std::process::exit(1);
                    });
                    let utc_start = parse_utc(start).unwrap_or_else(|e| {
                        eprintln!("{e}");
                        std::process::exit(1);
                    });
                    let utc_end = parse_utc(end).unwrap_or_else(|e| {
                        eprintln!("{e}");
                        std::process::exit(1);
                    });
                    let jd_start = utc_to_jd_tdb_with_policy(&utc_start, engine.lsk(), time_policy);
                    let jd_end = utc_to_jd_tdb_with_policy(&utc_end, engine.lsk(), time_policy);
                    GrahanQuery::Range {
                        start_jd_tdb: jd_start,
                        end_jd_tdb: jd_end,
                    }
                }
                _ => {
                    eprintln!("Invalid mode: {}", args.mode);
                    std::process::exit(1);
                }
            };
            let op = GrahanOperation {
                kind,
                config,
                query,
            };
            match dhruv_search::grahan(&engine, &op) {
                Ok(GrahanResult::ChandraSingle(Some(ev))) => {
                    let label = match args.mode.as_str() {
                        "next" => "Next Chandra Grahan",
                        "prev" => "Previous Chandra Grahan",
                        _ => "Chandra Grahan",
                    };
                    print_chandra_grahan(label, &ev);
                }
                Ok(GrahanResult::ChandraSingle(None)) => {
                    println!("No lunar eclipse found");
                }
                Ok(GrahanResult::ChandraMany(events)) => {
                    println!("Found {} lunar eclipses:", events.len());
                    for ev in &events {
                        print_chandra_grahan("  Chandra Grahan", ev);
                    }
                }
                Ok(GrahanResult::SuryaSingle(Some(ev))) => {
                    let label = match args.mode.as_str() {
                        "next" => "Next Surya Grahan",
                        "prev" => "Previous Surya Grahan",
                        _ => "Surya Grahan",
                    };
                    print_surya_grahan(label, &ev);
                }
                Ok(GrahanResult::SuryaSingle(None)) => {
                    println!("No solar eclipse found");
                }
                Ok(GrahanResult::SuryaMany(events)) => {
                    println!("Found {} solar eclipses:", events.len());
                    for ev in &events {
                        print_surya_grahan("  Surya Grahan", ev);
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::LunarPhase(args) => {
            let kind = match args.kind.as_str() {
                "amavasya" => LunarPhaseKind::Amavasya,
                "purnima" => LunarPhaseKind::Purnima,
                _ => {
                    eprintln!("Invalid kind: {}", args.kind);
                    std::process::exit(1);
                }
            };
            let engine = load_engine(&args.bsp, &args.lsk);
            let query = match args.mode.as_str() {
                "next" => {
                    let date = args.date.as_deref().unwrap_or_else(|| {
                        eprintln!("--date is required when --mode next");
                        std::process::exit(1);
                    });
                    let utc = parse_utc(date).unwrap_or_else(|e| {
                        eprintln!("{e}");
                        std::process::exit(1);
                    });
                    let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
                    LunarPhaseQuery::Next { at_jd_tdb: jd_tdb }
                }
                "prev" => {
                    let date = args.date.as_deref().unwrap_or_else(|| {
                        eprintln!("--date is required when --mode prev");
                        std::process::exit(1);
                    });
                    let utc = parse_utc(date).unwrap_or_else(|e| {
                        eprintln!("{e}");
                        std::process::exit(1);
                    });
                    let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
                    LunarPhaseQuery::Prev { at_jd_tdb: jd_tdb }
                }
                "range" => {
                    let start = args.start.as_deref().unwrap_or_else(|| {
                        eprintln!("--start is required when --mode range");
                        std::process::exit(1);
                    });
                    let end = args.end.as_deref().unwrap_or_else(|| {
                        eprintln!("--end is required when --mode range");
                        std::process::exit(1);
                    });
                    let utc_start = parse_utc(start).unwrap_or_else(|e| {
                        eprintln!("{e}");
                        std::process::exit(1);
                    });
                    let utc_end = parse_utc(end).unwrap_or_else(|e| {
                        eprintln!("{e}");
                        std::process::exit(1);
                    });
                    let jd_start = utc_to_jd_tdb_with_policy(&utc_start, engine.lsk(), time_policy);
                    let jd_end = utc_to_jd_tdb_with_policy(&utc_end, engine.lsk(), time_policy);
                    LunarPhaseQuery::Range {
                        start_jd_tdb: jd_start,
                        end_jd_tdb: jd_end,
                    }
                }
                _ => {
                    eprintln!("Invalid mode: {}", args.mode);
                    std::process::exit(1);
                }
            };
            let op = LunarPhaseOperation { kind, query };
            match dhruv_search::lunar_phase(&engine, &op) {
                Ok(LunarPhaseResult::Single(Some(ev))) => {
                    let label = match (args.mode.as_str(), kind) {
                        ("next", LunarPhaseKind::Amavasya) => "Next Amavasya",
                        ("next", LunarPhaseKind::Purnima) => "Next Purnima",
                        ("prev", LunarPhaseKind::Amavasya) => "Previous Amavasya",
                        ("prev", LunarPhaseKind::Purnima) => "Previous Purnima",
                        _ => "Lunar phase",
                    };
                    println!("{label}: {}", ev.utc);
                    println!(
                        "  Moon lon: {:.6} deg  Sun lon: {:.6} deg",
                        ev.moon_longitude_deg, ev.sun_longitude_deg
                    );
                }
                Ok(LunarPhaseResult::Single(None)) => {
                    let label = match kind {
                        LunarPhaseKind::Amavasya => "Amavasya",
                        LunarPhaseKind::Purnima => "Purnima",
                    };
                    println!("No {label} found");
                }
                Ok(LunarPhaseResult::Many(events)) => {
                    let label = match kind {
                        LunarPhaseKind::Amavasya => "Amavasyas",
                        LunarPhaseKind::Purnima => "Purnimas",
                    };
                    println!("Found {} {label}:", events.len());
                    for ev in &events {
                        println!(
                            "  {}  Moon: {:.6}°  Sun: {:.6}°",
                            ev.utc, ev.moon_longitude_deg, ev.sun_longitude_deg
                        );
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Sankranti(args) => {
            let target = match args.rashi {
                Some(idx) => {
                    let idx_u8 = u8::try_from(idx)
                        .ok()
                        .filter(|v| *v < 12)
                        .unwrap_or_else(|| {
                            eprintln!("Invalid --rashi: {idx} (0-11)");
                            std::process::exit(1);
                        });
                    SankrantiTarget::SpecificRashi(rashi_from_index(idx_u8))
                }
                None => SankrantiTarget::Any,
            };
            let system = require_aya_system(args.ayanamsha);
            let engine = load_engine(&args.bsp, &args.lsk);
            let config = SankrantiConfig::new(system, args.nutation);
            let query = match args.mode.as_str() {
                "next" => {
                    let date = args.date.as_deref().unwrap_or_else(|| {
                        eprintln!("--date is required when --mode next");
                        std::process::exit(1);
                    });
                    let utc = parse_utc(date).unwrap_or_else(|e| {
                        eprintln!("{e}");
                        std::process::exit(1);
                    });
                    let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
                    SankrantiQuery::Next { at_jd_tdb: jd_tdb }
                }
                "prev" => {
                    let date = args.date.as_deref().unwrap_or_else(|| {
                        eprintln!("--date is required when --mode prev");
                        std::process::exit(1);
                    });
                    let utc = parse_utc(date).unwrap_or_else(|e| {
                        eprintln!("{e}");
                        std::process::exit(1);
                    });
                    let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
                    SankrantiQuery::Prev { at_jd_tdb: jd_tdb }
                }
                "range" => {
                    let start = args.start.as_deref().unwrap_or_else(|| {
                        eprintln!("--start is required when --mode range");
                        std::process::exit(1);
                    });
                    let end = args.end.as_deref().unwrap_or_else(|| {
                        eprintln!("--end is required when --mode range");
                        std::process::exit(1);
                    });
                    let utc_start = parse_utc(start).unwrap_or_else(|e| {
                        eprintln!("{e}");
                        std::process::exit(1);
                    });
                    let utc_end = parse_utc(end).unwrap_or_else(|e| {
                        eprintln!("{e}");
                        std::process::exit(1);
                    });
                    let jd_start = utc_to_jd_tdb_with_policy(&utc_start, engine.lsk(), time_policy);
                    let jd_end = utc_to_jd_tdb_with_policy(&utc_end, engine.lsk(), time_policy);
                    SankrantiQuery::Range {
                        start_jd_tdb: jd_start,
                        end_jd_tdb: jd_end,
                    }
                }
                _ => {
                    eprintln!("Invalid mode: {}", args.mode);
                    std::process::exit(1);
                }
            };
            let op = SankrantiOperation {
                target,
                config,
                query,
            };
            match dhruv_search::sankranti(&engine, &op) {
                Ok(SankrantiResult::Single(Some(ev))) => {
                    let label = match (args.mode.as_str(), target) {
                        ("next", SankrantiTarget::Any) => "Next Sankranti".to_string(),
                        ("prev", SankrantiTarget::Any) => "Previous Sankranti".to_string(),
                        ("next", SankrantiTarget::SpecificRashi(r)) => {
                            format!("Next {} Sankranti", r.name())
                        }
                        ("prev", SankrantiTarget::SpecificRashi(r)) => {
                            format!("Previous {} Sankranti", r.name())
                        }
                        (_, SankrantiTarget::SpecificRashi(r)) => format!("{} Sankranti", r.name()),
                        _ => "Sankranti".to_string(),
                    };
                    println!("{label}: {}", ev.rashi.name());
                    println!("  Time: {}", ev.utc);
                    println!(
                        "  Sidereal lon: {:.6} deg  Tropical lon: {:.6} deg",
                        ev.sun_sidereal_longitude_deg, ev.sun_tropical_longitude_deg
                    );
                }
                Ok(SankrantiResult::Single(None)) => match target {
                    SankrantiTarget::Any => println!("No Sankranti found"),
                    SankrantiTarget::SpecificRashi(r) => {
                        println!("No {} Sankranti found", r.name())
                    }
                },
                Ok(SankrantiResult::Many(events)) => {
                    match target {
                        SankrantiTarget::Any => println!("Found {} Sankrantis:", events.len()),
                        SankrantiTarget::SpecificRashi(r) => {
                            println!("Found {} {} Sankrantis:", events.len(), r.name())
                        }
                    }
                    for ev in &events {
                        println!(
                            "  {} at {}  sid: {:.6}°  trop: {:.6}°",
                            ev.rashi.name(),
                            ev.utc,
                            ev.sun_sidereal_longitude_deg,
                            ev.sun_tropical_longitude_deg
                        );
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::NextChandraGrahan { date, bsp, lsk } => {
            let utc = parse_utc(&date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&bsp, &lsk);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            let config = GrahanConfig {
                include_penumbral: true,
                include_peak_details: true,
            };
            let op = GrahanOperation {
                kind: GrahanKind::Chandra,
                config,
                query: GrahanQuery::Next { at_jd_tdb: jd_tdb },
            };
            match dhruv_search::grahan(&engine, &op) {
                Ok(GrahanResult::ChandraSingle(Some(ev))) => {
                    print_chandra_grahan("Next Chandra Grahan", &ev)
                }
                Ok(GrahanResult::ChandraSingle(None)) => println!("No lunar eclipse found"),
                Ok(_) => {
                    eprintln!("Error: unexpected search result shape");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::PrevChandraGrahan { date, bsp, lsk } => {
            let utc = parse_utc(&date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&bsp, &lsk);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            let config = GrahanConfig {
                include_penumbral: true,
                include_peak_details: true,
            };
            let op = GrahanOperation {
                kind: GrahanKind::Chandra,
                config,
                query: GrahanQuery::Prev { at_jd_tdb: jd_tdb },
            };
            match dhruv_search::grahan(&engine, &op) {
                Ok(GrahanResult::ChandraSingle(Some(ev))) => {
                    print_chandra_grahan("Previous Chandra Grahan", &ev)
                }
                Ok(GrahanResult::ChandraSingle(None)) => println!("No lunar eclipse found"),
                Ok(_) => {
                    eprintln!("Error: unexpected search result shape");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::SearchChandraGrahan(args) => {
            let s = parse_utc(&args.start).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let e = parse_utc(&args.end).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&args.bsp, &args.lsk);
            let jd_start = utc_to_jd_tdb_with_policy(&s, engine.lsk(), time_policy);
            let jd_end = utc_to_jd_tdb_with_policy(&e, engine.lsk(), time_policy);
            let config = GrahanConfig {
                include_penumbral: true,
                include_peak_details: true,
            };
            let op = GrahanOperation {
                kind: GrahanKind::Chandra,
                config,
                query: GrahanQuery::Range {
                    start_jd_tdb: jd_start,
                    end_jd_tdb: jd_end,
                },
            };
            match dhruv_search::grahan(&engine, &op) {
                Ok(GrahanResult::ChandraMany(events)) => {
                    println!("Found {} lunar eclipses:", events.len());
                    for ev in &events {
                        print_chandra_grahan("  Chandra Grahan", ev);
                    }
                }
                Ok(_) => {
                    eprintln!("Error: unexpected search result shape");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::NextSuryaGrahan { date, bsp, lsk } => {
            let utc = parse_utc(&date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&bsp, &lsk);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            let config = GrahanConfig {
                include_penumbral: true,
                include_peak_details: true,
            };
            let op = GrahanOperation {
                kind: GrahanKind::Surya,
                config,
                query: GrahanQuery::Next { at_jd_tdb: jd_tdb },
            };
            match dhruv_search::grahan(&engine, &op) {
                Ok(GrahanResult::SuryaSingle(Some(ev))) => {
                    print_surya_grahan("Next Surya Grahan", &ev)
                }
                Ok(GrahanResult::SuryaSingle(None)) => println!("No solar eclipse found"),
                Ok(_) => {
                    eprintln!("Error: unexpected search result shape");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::PrevSuryaGrahan { date, bsp, lsk } => {
            let utc = parse_utc(&date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&bsp, &lsk);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            let config = GrahanConfig {
                include_penumbral: true,
                include_peak_details: true,
            };
            let op = GrahanOperation {
                kind: GrahanKind::Surya,
                config,
                query: GrahanQuery::Prev { at_jd_tdb: jd_tdb },
            };
            match dhruv_search::grahan(&engine, &op) {
                Ok(GrahanResult::SuryaSingle(Some(ev))) => {
                    print_surya_grahan("Previous Surya Grahan", &ev)
                }
                Ok(GrahanResult::SuryaSingle(None)) => println!("No solar eclipse found"),
                Ok(_) => {
                    eprintln!("Error: unexpected search result shape");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::SearchSuryaGrahan(args) => {
            let s = parse_utc(&args.start).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let e = parse_utc(&args.end).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&args.bsp, &args.lsk);
            let jd_start = utc_to_jd_tdb_with_policy(&s, engine.lsk(), time_policy);
            let jd_end = utc_to_jd_tdb_with_policy(&e, engine.lsk(), time_policy);
            let config = GrahanConfig {
                include_penumbral: true,
                include_peak_details: true,
            };
            let op = GrahanOperation {
                kind: GrahanKind::Surya,
                config,
                query: GrahanQuery::Range {
                    start_jd_tdb: jd_start,
                    end_jd_tdb: jd_end,
                },
            };
            match dhruv_search::grahan(&engine, &op) {
                Ok(GrahanResult::SuryaMany(events)) => {
                    println!("Found {} solar eclipses:", events.len());
                    for ev in &events {
                        print_surya_grahan("  Surya Grahan", ev);
                    }
                }
                Ok(_) => {
                    eprintln!("Error: unexpected search result shape");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Motion(args) => {
            let kind = match args.kind.as_str() {
                "stationary" => MotionKind::Stationary,
                "max-speed" => MotionKind::MaxSpeed,
                _ => {
                    eprintln!("Invalid kind: {}", args.kind);
                    std::process::exit(1);
                }
            };
            let body = require_body(args.body);
            let engine = load_engine(&args.bsp, &args.lsk);
            let config = StationaryConfig::inner_planet();
            let query = match args.mode.as_str() {
                "next" => {
                    let date = args.date.as_deref().unwrap_or_else(|| {
                        eprintln!("--date is required when --mode next");
                        std::process::exit(1);
                    });
                    let utc = parse_utc(date).unwrap_or_else(|e| {
                        eprintln!("{e}");
                        std::process::exit(1);
                    });
                    let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
                    MotionQuery::Next { at_jd_tdb: jd_tdb }
                }
                "prev" => {
                    let date = args.date.as_deref().unwrap_or_else(|| {
                        eprintln!("--date is required when --mode prev");
                        std::process::exit(1);
                    });
                    let utc = parse_utc(date).unwrap_or_else(|e| {
                        eprintln!("{e}");
                        std::process::exit(1);
                    });
                    let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
                    MotionQuery::Prev { at_jd_tdb: jd_tdb }
                }
                "range" => {
                    let start = args.start.as_deref().unwrap_or_else(|| {
                        eprintln!("--start is required when --mode range");
                        std::process::exit(1);
                    });
                    let end = args.end.as_deref().unwrap_or_else(|| {
                        eprintln!("--end is required when --mode range");
                        std::process::exit(1);
                    });
                    let utc_start = parse_utc(start).unwrap_or_else(|e| {
                        eprintln!("{e}");
                        std::process::exit(1);
                    });
                    let utc_end = parse_utc(end).unwrap_or_else(|e| {
                        eprintln!("{e}");
                        std::process::exit(1);
                    });
                    let jd_start = utc_to_jd_tdb_with_policy(&utc_start, engine.lsk(), time_policy);
                    let jd_end = utc_to_jd_tdb_with_policy(&utc_end, engine.lsk(), time_policy);
                    MotionQuery::Range {
                        start_jd_tdb: jd_start,
                        end_jd_tdb: jd_end,
                    }
                }
                _ => {
                    eprintln!("Invalid mode: {}", args.mode);
                    std::process::exit(1);
                }
            };
            let op = MotionOperation {
                body,
                kind,
                config,
                query,
            };
            match dhruv_search::motion(&engine, &op) {
                Ok(MotionResult::StationarySingle(Some(ev))) => {
                    let label = match args.mode.as_str() {
                        "next" => "Next stationary",
                        "prev" => "Previous stationary",
                        _ => "Stationary",
                    };
                    print_stationary_event(label, &ev);
                }
                Ok(MotionResult::StationarySingle(None)) => {
                    println!("No stationary point found");
                }
                Ok(MotionResult::StationaryMany(events)) => {
                    println!("Found {} stationary points:", events.len());
                    for ev in &events {
                        print_stationary_event("  Station", ev);
                    }
                }
                Ok(MotionResult::MaxSpeedSingle(Some(ev))) => {
                    let label = match args.mode.as_str() {
                        "next" => "Next max-speed",
                        "prev" => "Previous max-speed",
                        _ => "Max-speed",
                    };
                    print_max_speed_event(label, &ev);
                }
                Ok(MotionResult::MaxSpeedSingle(None)) => {
                    println!("No max-speed event found");
                }
                Ok(MotionResult::MaxSpeedMany(events)) => {
                    println!("Found {} max-speed events:", events.len());
                    for ev in &events {
                        print_max_speed_event("  Max-speed", ev);
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::NextStationary(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let b = require_body(args.body);
            let engine = load_engine(&args.bsp, &args.lsk);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            let config = StationaryConfig::inner_planet();
            let op = MotionOperation {
                body: b,
                kind: MotionKind::Stationary,
                config,
                query: MotionQuery::Next { at_jd_tdb: jd_tdb },
            };
            match dhruv_search::motion(&engine, &op) {
                Ok(MotionResult::StationarySingle(Some(ev))) => {
                    print_stationary_event("Next stationary", &ev)
                }
                Ok(MotionResult::StationarySingle(None)) => println!("No stationary point found"),
                Ok(_) => {
                    eprintln!("Error: unexpected search result shape");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::PrevStationary(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let b = require_body(args.body);
            let engine = load_engine(&args.bsp, &args.lsk);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            let config = StationaryConfig::inner_planet();
            let op = MotionOperation {
                body: b,
                kind: MotionKind::Stationary,
                config,
                query: MotionQuery::Prev { at_jd_tdb: jd_tdb },
            };
            match dhruv_search::motion(&engine, &op) {
                Ok(MotionResult::StationarySingle(Some(ev))) => {
                    print_stationary_event("Previous stationary", &ev)
                }
                Ok(MotionResult::StationarySingle(None)) => println!("No stationary point found"),
                Ok(_) => {
                    eprintln!("Error: unexpected search result shape");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::SearchStationary(args) => {
            let s = parse_utc(&args.start).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let e = parse_utc(&args.end).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let b = require_body(args.body);
            let engine = load_engine(&args.bsp, &args.lsk);
            let jd_start = utc_to_jd_tdb_with_policy(&s, engine.lsk(), time_policy);
            let jd_end = utc_to_jd_tdb_with_policy(&e, engine.lsk(), time_policy);
            let config = StationaryConfig::inner_planet();
            let op = MotionOperation {
                body: b,
                kind: MotionKind::Stationary,
                config,
                query: MotionQuery::Range {
                    start_jd_tdb: jd_start,
                    end_jd_tdb: jd_end,
                },
            };
            match dhruv_search::motion(&engine, &op) {
                Ok(MotionResult::StationaryMany(events)) => {
                    println!("Found {} stationary points:", events.len());
                    for ev in &events {
                        print_stationary_event("  Station", ev);
                    }
                }
                Ok(_) => {
                    eprintln!("Error: unexpected search result shape");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::NextMaxSpeed(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let b = require_body(args.body);
            let engine = load_engine(&args.bsp, &args.lsk);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            let config = StationaryConfig::inner_planet();
            let op = MotionOperation {
                body: b,
                kind: MotionKind::MaxSpeed,
                config,
                query: MotionQuery::Next { at_jd_tdb: jd_tdb },
            };
            match dhruv_search::motion(&engine, &op) {
                Ok(MotionResult::MaxSpeedSingle(Some(ev))) => {
                    print_max_speed_event("Next max-speed", &ev)
                }
                Ok(MotionResult::MaxSpeedSingle(None)) => println!("No max-speed event found"),
                Ok(_) => {
                    eprintln!("Error: unexpected search result shape");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::PrevMaxSpeed(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let b = require_body(args.body);
            let engine = load_engine(&args.bsp, &args.lsk);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            let config = StationaryConfig::inner_planet();
            let op = MotionOperation {
                body: b,
                kind: MotionKind::MaxSpeed,
                config,
                query: MotionQuery::Prev { at_jd_tdb: jd_tdb },
            };
            match dhruv_search::motion(&engine, &op) {
                Ok(MotionResult::MaxSpeedSingle(Some(ev))) => {
                    print_max_speed_event("Previous max-speed", &ev)
                }
                Ok(MotionResult::MaxSpeedSingle(None)) => println!("No max-speed event found"),
                Ok(_) => {
                    eprintln!("Error: unexpected search result shape");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::SearchMaxSpeed(args) => {
            let s = parse_utc(&args.start).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let e = parse_utc(&args.end).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let b = require_body(args.body);
            let engine = load_engine(&args.bsp, &args.lsk);
            let jd_start = utc_to_jd_tdb_with_policy(&s, engine.lsk(), time_policy);
            let jd_end = utc_to_jd_tdb_with_policy(&e, engine.lsk(), time_policy);
            let config = StationaryConfig::inner_planet();
            let op = MotionOperation {
                body: b,
                kind: MotionKind::MaxSpeed,
                config,
                query: MotionQuery::Range {
                    start_jd_tdb: jd_start,
                    end_jd_tdb: jd_end,
                },
            };
            match dhruv_search::motion(&engine, &op) {
                Ok(MotionResult::MaxSpeedMany(events)) => {
                    println!("Found {} max-speed events:", events.len());
                    for ev in &events {
                        print_max_speed_event("  Max-speed", ev);
                    }
                }
                Ok(_) => {
                    eprintln!("Error: unexpected search result shape");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Position(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let t = require_body(args.target);
            let obs = require_observer(args.observer);
            let engine = load_engine(&args.bsp, &args.lsk);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);

            // Helper: ecliptic-of-date spherical coords at a given JD TDB.
            let ecl_sph = |jd: f64| {
                let q = Query {
                    target: t,
                    observer: obs,
                    frame: Frame::IcrfJ2000,
                    epoch_tdb_jd: jd,
                };
                let sv = engine.query(q).unwrap_or_else(|e| {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                });
                let ecl_j2000 = icrf_to_ecliptic(&sv.position_km);
                let tc = (jd - 2_451_545.0) / 36525.0;
                let ecl_date = precess_ecliptic_j2000_to_date(&ecl_j2000, tc);
                cartesian_to_spherical(&ecl_date)
            };

            let sph = ecl_sph(jd_tdb);
            const DT: f64 = 1.0 / 1440.0;
            let sph_p = ecl_sph(jd_tdb + DT);
            let sph_m = ecl_sph(jd_tdb - DT);
            let dlon = ((sph_p.lon_deg - sph_m.lon_deg + 180.0).rem_euclid(360.0)) - 180.0;
            let lon_speed = dlon / (2.0 * DT);
            let lat_speed = (sph_p.lat_deg - sph_m.lat_deg) / (2.0 * DT);
            let dist_speed = (sph_p.distance_km - sph_m.distance_km) / (2.0 * DT * 86_400.0);
            println!("Position of {:?} from {:?}:", t, obs);
            println!("  Longitude:      {:.6}°", sph.lon_deg);
            println!("  Latitude:       {:.6}°", sph.lat_deg);
            println!("  Distance:       {:.6} km", sph.distance_km);
            println!("  Lon speed:      {:.6} deg/day", lon_speed);
            println!("  Lat speed:      {:.6} deg/day", lat_speed);
            println!("  Distance speed: {:.6} km/s", dist_speed);
        }

        Commands::SiderealLongitude(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let t = require_body(args.target);
            let obs = require_observer(args.observer);
            let system = require_aya_system(args.ayanamsha);
            let engine = load_engine(&args.bsp, &args.lsk);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            let query = Query {
                target: t,
                observer: obs,
                frame: Frame::IcrfJ2000,
                epoch_tdb_jd: jd_tdb,
            };
            let state = engine.query(query).unwrap_or_else(|e| {
                eprintln!("Error: {e}");
                std::process::exit(1);
            });
            let tc = jd_tdb_to_centuries(jd_tdb);
            let ecl_j2000 = icrf_to_ecliptic(&state.position_km);
            let ecl_date = precess_ecliptic_j2000_to_date(&ecl_j2000, tc);
            let tropical_lon = cartesian_to_spherical(&ecl_date).lon_deg;
            let aya = ayanamsha_deg(system, tc, args.nutation);
            let sid = (tropical_lon - aya).rem_euclid(360.0);
            println!("Tropical longitude: {:.6}°", tropical_lon);
            println!("Ayanamsha ({:?}): {:.6}°", system, aya);
            println!("Sidereal longitude: {:.6}°", sid);
        }

        Commands::GrahaLongitudes(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let system = require_aya_system(args.ayanamsha);
            let engine = load_engine(&args.bsp, &args.lsk);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            let precession_model = parse_precession_model(&args.precession);
            let default_plane = if args.tropical {
                ReferencePlane::Ecliptic
            } else {
                system.default_reference_plane()
            };
            let reference_plane = parse_reference_plane_arg(&args.reference_plane, default_plane);
            let lon_config = if args.tropical {
                dhruv_search::GrahaLongitudesConfig::tropical_with_model(
                    args.nutation,
                    precession_model,
                    reference_plane,
                )
            } else {
                dhruv_search::GrahaLongitudesConfig::sidereal_with_model(
                    system,
                    args.nutation,
                    precession_model,
                    reference_plane,
                )
            };
            let lons =
                dhruv_search::graha_longitudes(&engine, jd_tdb, &lon_config).unwrap_or_else(|e| {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                });
            let outer_lons = if args.no_outer {
                None
            } else {
                lons.outer_planets
            };

            println!(
                "Graha {} longitudes (plane={:?}, precession={:?}{}):\n",
                if args.tropical {
                    "reference-plane"
                } else {
                    "sidereal"
                },
                reference_plane,
                precession_model,
                if !args.tropical {
                    format!(
                        ", system={system:?}{}",
                        if args.nutation { " +nutation" } else { "" }
                    )
                } else if args.nutation {
                    ", +nutation".to_string()
                } else {
                    String::new()
                }
            );
            let graha_names = [
                "Surya", "Chandra", "Mangal", "Buddh", "Guru", "Shukra", "Shani", "Rahu", "Ketu",
            ];
            for (i, name) in graha_names.iter().enumerate() {
                let lon = lons.longitudes[i];
                let rashi_info = dhruv_vedic_base::rashi_from_longitude(lon);
                println!(
                    "  {:8} {:>11.6}° ({} {}°{:02}'{:04.1}\")",
                    name,
                    lon,
                    rashi_info.rashi.name(),
                    rashi_info.dms.degrees,
                    rashi_info.dms.minutes,
                    rashi_info.dms.seconds,
                );
            }
            if let Some(outer_lons) = outer_lons {
                let outer_names = ["Uranus", "Neptune", "Pluto"];
                println!("\nOuter Grahas:");
                for (name, lon) in outer_names.iter().zip(outer_lons.iter()) {
                    let rashi_info = dhruv_vedic_base::rashi_from_longitude(*lon);
                    println!(
                        "  {:8} {:>11.6}° ({} {}°{:02}'{:04.1}\")",
                        name,
                        lon,
                        rashi_info.rashi.name(),
                        rashi_info.dms.degrees,
                        rashi_info.dms.minutes,
                        rashi_info.dms.seconds,
                    );
                }
            }
        }

        Commands::OsculatingApogee(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let system = require_aya_system(args.ayanamsha);
            let engine = load_engine(&args.bsp, &args.lsk);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            let precession_model = parse_precession_model(&args.precession);
            let reference_plane =
                parse_reference_plane_arg(&args.reference_plane, system.default_reference_plane());
            let grahas: Vec<Graha> = args
                .graha
                .split(',')
                .map(|name| parse_graha_name(name.trim()))
                .collect();
            let result = dhruv_search::moving_osculating_apogees(
                &engine,
                jd_tdb,
                &dhruv_search::GrahaLongitudesConfig::sidereal_with_model(
                    system,
                    args.nutation,
                    precession_model,
                    reference_plane,
                ),
                &grahas,
            )
            .unwrap_or_else(|e| {
                eprintln!("Error: {e}");
                std::process::exit(1);
            });

            println!(
                "Moving heliocentric osculating apogees (system={system:?}{}, plane={reference_plane:?}, precession={precession_model:?}):",
                if args.nutation { " +nutation" } else { "" }
            );
            println!(
                "{:<10} {:>14} {:>14} {:>14}",
                "Graha", "Sidereal", "Ayanamsha", "Ref-plane"
            );
            for entry in result.entries {
                println!(
                    "{:<10} {:>13.6}° {:>13.6}° {:>13.6}°",
                    graha_display_name(entry.graha),
                    entry.sidereal_longitude,
                    entry.ayanamsha_deg,
                    entry.reference_plane_longitude,
                );
            }
        }

        // -----------------------------------------------------------
        // Individual Sphuta Formulas (pure math)
        // -----------------------------------------------------------
        Commands::BhriguBindu { rahu, moon } => {
            println!("{:.6}°", dhruv_vedic_base::bhrigu_bindu(rahu, moon));
        }

        Commands::PranaSphuta { lagna, moon } => {
            println!("{:.6}°", dhruv_vedic_base::prana_sphuta(lagna, moon));
        }

        Commands::DehaSphuta { moon, lagna } => {
            println!("{:.6}°", dhruv_vedic_base::deha_sphuta(moon, lagna));
        }

        Commands::MrityuSphuta { eighth_lord, lagna } => {
            println!(
                "{:.6}°",
                dhruv_vedic_base::mrityu_sphuta(eighth_lord, lagna)
            );
        }

        Commands::TithiSphuta { moon, sun, lagna } => {
            println!("{:.6}°", dhruv_vedic_base::tithi_sphuta(moon, sun, lagna));
        }

        Commands::YogaSphuta { sun, moon } => {
            println!("{:.6}°", dhruv_vedic_base::yoga_sphuta(sun, moon));
        }

        Commands::YogaSphutaNormalized { sun, moon } => {
            println!(
                "{:.6}°",
                dhruv_vedic_base::yoga_sphuta_normalized(sun, moon)
            );
        }

        Commands::RahuTithiSphuta { rahu, sun, lagna } => {
            println!(
                "{:.6}°",
                dhruv_vedic_base::rahu_tithi_sphuta(rahu, sun, lagna)
            );
        }

        Commands::KshetraSphuta(args) => {
            println!(
                "{:.6}°",
                dhruv_vedic_base::kshetra_sphuta(
                    args.venus,
                    args.moon,
                    args.mars,
                    args.jupiter,
                    args.lagna
                )
            );
        }

        Commands::BeejaSphuta {
            sun,
            venus,
            jupiter,
        } => {
            println!(
                "{:.6}°",
                dhruv_vedic_base::beeja_sphuta(sun, venus, jupiter)
            );
        }

        Commands::TriSphuta {
            lagna,
            moon,
            gulika,
        } => {
            println!("{:.6}°", dhruv_vedic_base::trisphuta(lagna, moon, gulika));
        }

        Commands::ChatusSphuta { trisphuta, sun } => {
            println!("{:.6}°", dhruv_vedic_base::chatussphuta(trisphuta, sun));
        }

        Commands::PanchaSphuta { chatussphuta, rahu } => {
            println!("{:.6}°", dhruv_vedic_base::panchasphuta(chatussphuta, rahu));
        }

        Commands::SookshmaTrisphuta(args) => {
            println!(
                "{:.6}°",
                dhruv_vedic_base::sookshma_trisphuta(args.lagna, args.moon, args.gulika, args.sun)
            );
        }

        Commands::AvayogaSphuta { sun, moon } => {
            println!("{:.6}°", dhruv_vedic_base::avayoga_sphuta(sun, moon));
        }

        Commands::Kunda { lagna, moon, mars } => {
            println!("{:.6}°", dhruv_vedic_base::kunda(lagna, moon, mars));
        }

        // -----------------------------------------------------------
        // Individual Special Lagna Formulas (pure math)
        // -----------------------------------------------------------
        Commands::BhavaLagna { sun_lon, ghatikas } => {
            println!("{:.6}°", dhruv_vedic_base::bhava_lagna(sun_lon, ghatikas));
        }

        Commands::HoraLagna { sun_lon, ghatikas } => {
            println!("{:.6}°", dhruv_vedic_base::hora_lagna(sun_lon, ghatikas));
        }

        Commands::GhatiLagna { sun_lon, ghatikas } => {
            println!("{:.6}°", dhruv_vedic_base::ghati_lagna(sun_lon, ghatikas));
        }

        Commands::VighatiLagna {
            lagna_lon,
            vighatikas,
        } => {
            println!(
                "{:.6}°",
                dhruv_vedic_base::vighati_lagna(lagna_lon, vighatikas)
            );
        }

        Commands::VarnadaLagna {
            lagna_lon,
            hora_lagna_lon,
        } => {
            println!(
                "{:.6}°",
                dhruv_vedic_base::varnada_lagna(lagna_lon, hora_lagna_lon)
            );
        }

        Commands::SreeLagna {
            moon_lon,
            lagna_lon,
        } => {
            println!("{:.6}°", dhruv_vedic_base::sree_lagna(moon_lon, lagna_lon));
        }

        Commands::PranapadaLagna { sun_lon, ghatikas } => {
            println!(
                "{:.6}°",
                dhruv_vedic_base::pranapada_lagna(sun_lon, ghatikas)
            );
        }

        Commands::InduLagna {
            moon_lon,
            lagna_lord,
            moon_9th_lord,
        } => {
            let ll = require_graha(lagna_lord);
            let m9l = require_graha(moon_9th_lord);
            println!("{:.6}°", dhruv_vedic_base::indu_lagna(moon_lon, ll, m9l));
        }

        // -----------------------------------------------------------
        // Utility Primitives
        // -----------------------------------------------------------
        Commands::TithiFromElongation { elongation } => {
            let pos = dhruv_vedic_base::tithi_from_elongation(elongation);
            println!(
                "{} ({} {}) - {:.6}° into tithi",
                pos.tithi.name(),
                pos.paksha.name(),
                pos.tithi_in_paksha,
                pos.degrees_in_tithi
            );
        }

        Commands::KaranaFromElongation { elongation } => {
            let pos = dhruv_vedic_base::karana_from_elongation(elongation);
            println!(
                "{} (index {}) - {:.6}° into karana",
                pos.karana.name(),
                pos.karana_index,
                pos.degrees_in_karana
            );
        }

        Commands::YogaFromSum { sum } => {
            let pos = dhruv_vedic_base::yoga_from_sum(sum);
            println!(
                "{} (index {}) - {:.6}° into yoga",
                pos.yoga.name(),
                pos.yoga_index,
                pos.degrees_in_yoga
            );
        }

        Commands::VaarFromJd { jd } => {
            let vaar = dhruv_vedic_base::vaar_from_jd(jd);
            println!("{}", vaar.name());
        }

        Commands::MasaFromRashi { rashi } => {
            let masa = dhruv_vedic_base::masa_from_rashi_index(rashi);
            println!("{}", masa.name());
        }

        Commands::AyanaFromLon { lon } => {
            let ayana = dhruv_vedic_base::ayana_from_sidereal_longitude(lon);
            println!("{}", ayana.name());
        }

        Commands::SamvatsaraCompute { year } => {
            let (samvatsara, cycle_index) = dhruv_vedic_base::samvatsara_from_year(year);
            println!(
                "{} (index {} in 60-year cycle)",
                samvatsara.name(),
                cycle_index
            );
        }

        Commands::NthRashiFrom { rashi, offset } => {
            let result = dhruv_vedic_base::nth_rashi_from(rashi, offset);
            let ri = rashi_from_index(result);
            println!("{} (index {})", ri.name(), result);
        }

        Commands::RashiLord { rashi } => match dhruv_vedic_base::rashi_lord_by_index(rashi) {
            Some(graha) => println!("{}", graha.name()),
            None => {
                eprintln!("Invalid rashi index: {rashi} (0-11)");
                std::process::exit(1);
            }
        },

        Commands::Normalize360 { deg } => {
            println!("{:.6}°", dhruv_vedic_base::normalize_360(deg));
        }

        Commands::ArudhaPadaCompute { cusp_lon, lord_lon } => {
            let (lon, rashi_idx) = dhruv_vedic_base::arudha_pada(cusp_lon, lord_lon);
            let ri = rashi_from_index(rashi_idx);
            println!("{:.6}° ({})", lon, ri.name());
        }

        Commands::SunBasedUpagrahas { sun_lon } => {
            let upa = dhruv_vedic_base::sun_based_upagrahas(sun_lon);
            println!("Dhooma:      {:.6}°", upa.dhooma);
            println!("Vyatipata:   {:.6}°", upa.vyatipata);
            println!("Parivesha:   {:.6}°", upa.parivesha);
            println!("Indra Chapa: {:.6}°", upa.indra_chapa);
            println!("Upaketu:     {:.6}°", upa.upaketu);
        }

        Commands::GrahaHelper(args) => match args.op {
            GrahaHelperOp::HoraLord => {
                let vaar = require_vaar(args.vaar.unwrap_or_else(|| {
                    eprintln!("--vaar is required for --op hora-lord");
                    std::process::exit(1);
                }));
                let hora_index = args.hora_index.unwrap_or_else(|| {
                    eprintln!("--hora-index is required for --op hora-lord");
                    std::process::exit(1);
                });
                let lord = dhruv_vedic_base::hora_lord(vaar, hora_index);
                println!("{} ({})", lord.index(), lord.name());
            }
            GrahaHelperOp::MasaLord => {
                let masa = require_masa(args.masa.unwrap_or_else(|| {
                    eprintln!("--masa is required for --op masa-lord");
                    std::process::exit(1);
                }));
                let lord = dhruv_vedic_base::masa_lord(masa);
                println!("{} ({})", lord.index(), lord.name());
            }
            GrahaHelperOp::SamvatsaraLord => {
                let samvatsara = require_samvatsara(args.samvatsara.unwrap_or_else(|| {
                    eprintln!("--samvatsara is required for --op samvatsara-lord");
                    std::process::exit(1);
                }));
                let lord = dhruv_vedic_base::samvatsara_lord(samvatsara);
                println!("{} ({})", lord.index(), lord.name());
            }
            GrahaHelperOp::ExaltationDegree => {
                let graha = require_graha(args.graha.unwrap_or_else(|| {
                    eprintln!("--graha is required for --op exaltation-degree");
                    std::process::exit(1);
                }));
                match dhruv_vedic_base::exaltation_degree(graha) {
                    Some(value) => println!("{value:.6}"),
                    None => println!("none"),
                }
            }
            GrahaHelperOp::DebilitationDegree => {
                let graha = require_graha(args.graha.unwrap_or_else(|| {
                    eprintln!("--graha is required for --op debilitation-degree");
                    std::process::exit(1);
                }));
                match dhruv_vedic_base::debilitation_degree(graha) {
                    Some(value) => println!("{value:.6}"),
                    None => println!("none"),
                }
            }
            GrahaHelperOp::MoolatrikoneRange => {
                let graha = require_graha(args.graha.unwrap_or_else(|| {
                    eprintln!("--graha is required for --op moolatrikone-range");
                    std::process::exit(1);
                }));
                match dhruv_vedic_base::moolatrikone_range(graha) {
                    Some((rashi_index, start_deg, end_deg)) => {
                        println!("{rashi_index},{start_deg:.6},{end_deg:.6}")
                    }
                    None => println!("none"),
                }
            }
            GrahaHelperOp::CombustionThreshold => {
                let graha = require_graha(args.graha.unwrap_or_else(|| {
                    eprintln!("--graha is required for --op combustion-threshold");
                    std::process::exit(1);
                }));
                match dhruv_vedic_base::combustion_threshold(graha, args.retrograde) {
                    Some(value) => println!("{value:.6}"),
                    None => println!("none"),
                }
            }
            GrahaHelperOp::IsCombust => {
                let graha = require_graha(args.graha.unwrap_or_else(|| {
                    eprintln!("--graha is required for --op is-combust");
                    std::process::exit(1);
                }));
                let graha_lon = args.sidereal_lon.unwrap_or_else(|| {
                    eprintln!("--sidereal-lon is required for --op is-combust");
                    std::process::exit(1);
                });
                let sun_lon = args.sun_lon.unwrap_or_else(|| {
                    eprintln!("--sun-lon is required for --op is-combust");
                    std::process::exit(1);
                });
                println!(
                    "{}",
                    dhruv_vedic_base::is_combust(graha, graha_lon, sun_lon, args.retrograde)
                );
            }
            GrahaHelperOp::AllCombustionStatus => {
                let longitudes =
                    parse_longitudes_9(args.longitudes.as_deref().unwrap_or_else(|| {
                        eprintln!("--longitudes is required for --op all-combustion-status");
                        std::process::exit(1);
                    }));
                let retrograde_flags =
                    parse_bools_9(args.retrograde_flags.as_deref().unwrap_or_else(|| {
                        eprintln!("--retrograde-flags is required for --op all-combustion-status");
                        std::process::exit(1);
                    }));
                let out = dhruv_vedic_base::all_combustion_status(&longitudes, &retrograde_flags);
                println!(
                    "{}",
                    out.iter()
                        .map(|value| if *value { "true" } else { "false" })
                        .collect::<Vec<_>>()
                        .join(",")
                );
            }
            GrahaHelperOp::NaisargikaMaitri => {
                let graha = require_graha(args.graha.unwrap_or_else(|| {
                    eprintln!("--graha is required for --op naisargika-maitri");
                    std::process::exit(1);
                }));
                let other = require_graha(args.other.unwrap_or_else(|| {
                    eprintln!("--other is required for --op naisargika-maitri");
                    std::process::exit(1);
                }));
                println!(
                    "{}",
                    naisargika_label(dhruv_vedic_base::naisargika_maitri(graha, other))
                );
            }
            GrahaHelperOp::TatkalikaMaitri => {
                let graha_rashi = args.rashi.unwrap_or_else(|| {
                    eprintln!("--rashi is required for --op tatkalika-maitri");
                    std::process::exit(1);
                });
                let other_rashi = args.other_rashi.unwrap_or_else(|| {
                    eprintln!("--other-rashi is required for --op tatkalika-maitri");
                    std::process::exit(1);
                });
                println!(
                    "{}",
                    tatkalika_label(dhruv_vedic_base::tatkalika_maitri(graha_rashi, other_rashi))
                );
            }
            GrahaHelperOp::PanchadhaMaitri => {
                let naisargika = parse_cli_naisargika(args.naisargika.unwrap_or_else(|| {
                    eprintln!("--naisargika is required for --op panchadha-maitri");
                    std::process::exit(1);
                }));
                let tatkalika = parse_cli_tatkalika(args.tatkalika.unwrap_or_else(|| {
                    eprintln!("--tatkalika is required for --op panchadha-maitri");
                    std::process::exit(1);
                }));
                println!(
                    "{}",
                    panchadha_label(dhruv_vedic_base::panchadha_maitri(naisargika, tatkalika))
                );
            }
            GrahaHelperOp::DignityInRashi => {
                let graha = require_graha(args.graha.unwrap_or_else(|| {
                    eprintln!("--graha is required for --op dignity-in-rashi");
                    std::process::exit(1);
                }));
                let sidereal_lon = args.sidereal_lon.unwrap_or_else(|| {
                    eprintln!("--sidereal-lon is required for --op dignity-in-rashi");
                    std::process::exit(1);
                });
                let rashi = args.rashi.unwrap_or_else(|| {
                    eprintln!("--rashi is required for --op dignity-in-rashi");
                    std::process::exit(1);
                });
                println!(
                    "{}",
                    dignity_label(dhruv_vedic_base::dignity_in_rashi(
                        graha,
                        sidereal_lon,
                        rashi
                    ))
                );
            }
            GrahaHelperOp::DignityInRashiWithPositions => {
                let graha = require_graha(args.graha.unwrap_or_else(|| {
                    eprintln!("--graha is required for --op dignity-in-rashi-with-positions");
                    std::process::exit(1);
                }));
                let sidereal_lon = args.sidereal_lon.unwrap_or_else(|| {
                    eprintln!(
                        "--sidereal-lon is required for --op dignity-in-rashi-with-positions"
                    );
                    std::process::exit(1);
                });
                let rashi = args.rashi.unwrap_or_else(|| {
                    eprintln!("--rashi is required for --op dignity-in-rashi-with-positions");
                    std::process::exit(1);
                });
                let all_rashi_indices = parse_u8s::<7>(
                    args.all_rashi_indices_7.as_deref().unwrap_or_else(|| {
                        eprintln!(
                            "--all-rashi-indices-7 is required as D1 positions for --op dignity-in-rashi-with-positions"
                        );
                        std::process::exit(1);
                    }),
                    "D1 rashi index",
                );
                println!(
                    "{}",
                    dignity_label(dhruv_vedic_base::dignity_in_rashi_with_positions(
                        graha,
                        sidereal_lon,
                        rashi,
                        &all_rashi_indices,
                    ))
                );
            }
            GrahaHelperOp::NodeDignityInRashi => {
                let node = parse_cli_node(args.node.unwrap_or_else(|| {
                    eprintln!("--node is required for --op node-dignity-in-rashi");
                    std::process::exit(1);
                }));
                let rashi = args.rashi.unwrap_or_else(|| {
                    eprintln!("--rashi is required for --op node-dignity-in-rashi");
                    std::process::exit(1);
                });
                let all_rashi_indices = parse_u8s::<9>(
                    args.all_rashi_indices_9.as_deref().unwrap_or_else(|| {
                        eprintln!(
                            "--all-rashi-indices-9 is required as D1 positions for --op node-dignity-in-rashi"
                        );
                        std::process::exit(1);
                    }),
                    "D1 rashi index",
                );
                println!(
                    "{}",
                    dignity_label(dhruv_vedic_base::node_dignity_in_rashi(
                        node,
                        rashi,
                        &all_rashi_indices,
                        parse_cli_node_policy(args.node_policy),
                    ))
                );
            }
            GrahaHelperOp::NaturalBeneficMalefic => {
                let graha = require_graha(args.graha.unwrap_or_else(|| {
                    eprintln!("--graha is required for --op natural-benefic-malefic");
                    std::process::exit(1);
                }));
                println!(
                    "{}",
                    benefic_label(dhruv_vedic_base::natural_benefic_malefic(graha))
                );
            }
            GrahaHelperOp::MoonBeneficNature => {
                let elongation = args.moon_sun_elongation.unwrap_or_else(|| {
                    eprintln!("--moon-sun-elongation is required for --op moon-benefic-nature");
                    std::process::exit(1);
                });
                println!(
                    "{}",
                    benefic_label(dhruv_vedic_base::moon_benefic_nature(elongation))
                );
            }
            GrahaHelperOp::GrahaGender => {
                let graha = require_graha(args.graha.unwrap_or_else(|| {
                    eprintln!("--graha is required for --op graha-gender");
                    std::process::exit(1);
                }));
                println!("{}", gender_label(dhruv_vedic_base::graha_gender(graha)));
            }
        },

        Commands::TimeUtility(args) => match args.op {
            TimeUtilityOp::AyanamshaSystemCount => {
                println!("{}", AyanamshaSystem::all().len());
            }
            TimeUtilityOp::ReferencePlaneDefault => {
                let system = require_aya_system(args.ayanamsha.unwrap_or_else(|| {
                    eprintln!("--ayanamsha is required for --op reference-plane-default");
                    std::process::exit(1);
                }));
                let label = match system.default_reference_plane() {
                    ReferencePlane::Ecliptic => "ecliptic",
                    ReferencePlane::Invariable => "invariable",
                };
                println!("{label}");
            }
            TimeUtilityOp::ApproximateLocalNoon => {
                let jd_ut_midnight = args.jd_ut_midnight.unwrap_or_else(|| {
                    eprintln!("--jd-ut-midnight is required for --op approximate-local-noon");
                    std::process::exit(1);
                });
                let longitude_deg = args.longitude_deg.unwrap_or_else(|| {
                    eprintln!("--longitude-deg is required for --op approximate-local-noon");
                    std::process::exit(1);
                });
                println!(
                    "{:.6}",
                    dhruv_vedic_base::approximate_local_noon_jd(jd_ut_midnight, longitude_deg)
                );
            }
            TimeUtilityOp::MonthFromAbbrev => {
                let month_abbrev = args.month_abbrev.as_deref().unwrap_or_else(|| {
                    eprintln!("--month-abbrev is required for --op month-from-abbrev");
                    std::process::exit(1);
                });
                match dhruv_time::julian::month_from_abbrev(month_abbrev) {
                    Some(value) => println!("{value}"),
                    None => println!("none"),
                }
            }
            TimeUtilityOp::CalendarToJd => {
                let year = args.year.unwrap_or_else(|| {
                    eprintln!("--year is required for --op calendar-to-jd");
                    std::process::exit(1);
                });
                let month = args.month.unwrap_or_else(|| {
                    eprintln!("--month is required for --op calendar-to-jd");
                    std::process::exit(1);
                });
                let day = args.day.unwrap_or_else(|| {
                    eprintln!("--day is required for --op calendar-to-jd");
                    std::process::exit(1);
                });
                println!(
                    "{:.9}",
                    dhruv_time::calendar_to_jd_with_policy(
                        year,
                        month,
                        day,
                        parse_cli_calendar_policy(args.calendar_policy),
                    )
                );
            }
            TimeUtilityOp::JdToCalendar => {
                let jd = args.jd.unwrap_or_else(|| {
                    eprintln!("--jd is required for --op jd-to-calendar");
                    std::process::exit(1);
                });
                let (year, month, day) = dhruv_time::jd_to_calendar_with_policy(
                    jd,
                    parse_cli_calendar_policy(args.calendar_policy),
                );
                println!("year={year} month={month} day={day:.9}");
            }
            TimeUtilityOp::MeanObliquityOfDateArcsec => {
                let t_centuries = args.t_centuries.unwrap_or_else(|| {
                    eprintln!("--t-centuries is required for --op mean-obliquity-of-date-arcsec");
                    std::process::exit(1);
                });
                println!(
                    "{:.9}",
                    dhruv_frames::mean_obliquity_of_date_arcsec(t_centuries)
                );
            }
            TimeUtilityOp::MeanObliquityOfDateRad => {
                let t_centuries = args.t_centuries.unwrap_or_else(|| {
                    eprintln!("--t-centuries is required for --op mean-obliquity-of-date-rad");
                    std::process::exit(1);
                });
                println!(
                    "{:.15}",
                    dhruv_frames::mean_obliquity_of_date_rad(t_centuries)
                );
            }
            TimeUtilityOp::IcrfToReferencePlane => {
                let vector = parse_vec3(
                    args.vector.as_deref().unwrap_or_else(|| {
                        eprintln!("--vector is required for --op icrf-to-reference-plane");
                        std::process::exit(1);
                    }),
                    "vector",
                );
                let plane = parse_cli_reference_plane(args.reference_plane.unwrap_or_else(|| {
                    eprintln!("--reference-plane is required for --op icrf-to-reference-plane");
                    std::process::exit(1);
                }));
                let out = dhruv_frames::icrf_to_reference_plane(&vector, plane);
                println!("{:.12},{:.12},{:.12}", out[0], out[1], out[2]);
            }
            TimeUtilityOp::EclipticToInvariable => {
                let vector = parse_vec3(
                    args.vector.as_deref().unwrap_or_else(|| {
                        eprintln!("--vector is required for --op ecliptic-to-invariable");
                        std::process::exit(1);
                    }),
                    "vector",
                );
                let out = dhruv_frames::ecliptic_to_invariable(&vector);
                println!("{:.12},{:.12},{:.12}", out[0], out[1], out[2]);
            }
            TimeUtilityOp::InvariableToEcliptic => {
                let vector = parse_vec3(
                    args.vector.as_deref().unwrap_or_else(|| {
                        eprintln!("--vector is required for --op invariable-to-ecliptic");
                        std::process::exit(1);
                    }),
                    "vector",
                );
                let out = dhruv_frames::invariable_to_ecliptic(&vector);
                println!("{:.12},{:.12},{:.12}", out[0], out[1], out[2]);
            }
            TimeUtilityOp::EclipticLonToInvariableLon => {
                let longitude_deg = args.longitude_deg.unwrap_or_else(|| {
                    eprintln!(
                        "--longitude-deg is required for --op ecliptic-lon-to-invariable-lon"
                    );
                    std::process::exit(1);
                });
                println!(
                    "{:.9}",
                    dhruv_frames::ecliptic_lon_to_invariable_lon(longitude_deg)
                );
            }
            TimeUtilityOp::InvariableLonToEclipticLon => {
                let longitude_deg = args.longitude_deg.unwrap_or_else(|| {
                    eprintln!(
                        "--longitude-deg is required for --op invariable-lon-to-ecliptic-lon"
                    );
                    std::process::exit(1);
                });
                println!(
                    "{:.9}",
                    dhruv_frames::invariable_lon_to_ecliptic_lon(longitude_deg)
                );
            }
            TimeUtilityOp::PrecessEclipticJ2000ToDate => {
                let vector = parse_vec3(
                    args.vector.as_deref().unwrap_or_else(|| {
                        eprintln!("--vector is required for --op precess-ecliptic-j2000-to-date");
                        std::process::exit(1);
                    }),
                    "vector",
                );
                let t_centuries = args.t_centuries.unwrap_or_else(|| {
                    eprintln!("--t-centuries is required for --op precess-ecliptic-j2000-to-date");
                    std::process::exit(1);
                });
                let out = match args.precession.as_deref() {
                    Some(model) => dhruv_frames::precess_ecliptic_j2000_to_date_with_model(
                        &vector,
                        t_centuries,
                        parse_precession_model(model),
                    ),
                    None => dhruv_frames::precess_ecliptic_j2000_to_date(&vector, t_centuries),
                };
                println!("{:.12},{:.12},{:.12}", out[0], out[1], out[2]);
            }
        },

        Commands::TaraPrimitive(args) => match args.op {
            TaraPrimitiveOp::PropagatePosition => {
                let pos = dhruv_tara::propagate_position(
                    args.ra_deg.unwrap_or_else(|| {
                        eprintln!("--ra-deg is required for --op propagate-position");
                        std::process::exit(1);
                    }),
                    args.dec_deg.unwrap_or_else(|| {
                        eprintln!("--dec-deg is required for --op propagate-position");
                        std::process::exit(1);
                    }),
                    args.parallax_mas.unwrap_or_else(|| {
                        eprintln!("--parallax-mas is required for --op propagate-position");
                        std::process::exit(1);
                    }),
                    args.pm_ra_mas_yr.unwrap_or_else(|| {
                        eprintln!("--pm-ra-mas-yr is required for --op propagate-position");
                        std::process::exit(1);
                    }),
                    args.pm_dec_mas_yr.unwrap_or_else(|| {
                        eprintln!("--pm-dec-mas-yr is required for --op propagate-position");
                        std::process::exit(1);
                    }),
                    args.rv_km_s.unwrap_or_else(|| {
                        eprintln!("--rv-km-s is required for --op propagate-position");
                        std::process::exit(1);
                    }),
                    args.dt_years.unwrap_or_else(|| {
                        eprintln!("--dt-years is required for --op propagate-position");
                        std::process::exit(1);
                    }),
                );
                println!(
                    "ra_deg={:.12} dec_deg={:.12} distance_au={:.12}",
                    pos.ra_deg, pos.dec_deg, pos.distance_au
                );
            }
            TaraPrimitiveOp::ApplyAberration => {
                let direction = parse_vec3(
                    args.direction.as_deref().unwrap_or_else(|| {
                        eprintln!("--direction is required for --op apply-aberration");
                        std::process::exit(1);
                    }),
                    "direction",
                );
                let earth_velocity = parse_vec3(
                    args.earth_velocity.as_deref().unwrap_or_else(|| {
                        eprintln!("--earth-velocity is required for --op apply-aberration");
                        std::process::exit(1);
                    }),
                    "earth velocity",
                );
                let out = dhruv_tara::apply_aberration(&direction, &earth_velocity);
                println!("{:.12},{:.12},{:.12}", out[0], out[1], out[2]);
            }
            TaraPrimitiveOp::ApplyLightDeflection => {
                let direction = parse_vec3(
                    args.direction.as_deref().unwrap_or_else(|| {
                        eprintln!("--direction is required for --op apply-light-deflection");
                        std::process::exit(1);
                    }),
                    "direction",
                );
                let earth_position = parse_vec3(
                    args.earth_position.as_deref().unwrap_or_else(|| {
                        eprintln!("--earth-position is required for --op apply-light-deflection");
                        std::process::exit(1);
                    }),
                    "earth position",
                );
                let observer_sun_distance_au = args.observer_sun_distance_au.unwrap_or_else(|| {
                    eprintln!(
                        "--observer-sun-distance-au is required for --op apply-light-deflection"
                    );
                    std::process::exit(1);
                });
                let out = dhruv_tara::apply_light_deflection(
                    &direction,
                    &earth_position,
                    observer_sun_distance_au,
                );
                println!("{:.12},{:.12},{:.12}", out[0], out[1], out[2]);
            }
            TaraPrimitiveOp::GalacticAnticenterIcrs => {
                let out = dhruv_tara::galactic_anticenter_icrs();
                println!("{:.12},{:.12},{:.12}", out[0], out[1], out[2]);
            }
        },

        // -----------------------------------------------------------
        // Panchang Intermediates (engine required)
        // -----------------------------------------------------------
        Commands::ElongationAt { date, bsp, lsk } => {
            let utc = parse_utc(&date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&bsp, &lsk);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            match dhruv_search::elongation_at(&engine, jd_tdb) {
                Ok(val) => println!("{:.6}°", val),
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::SiderealSumAt(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let system = require_aya_system(args.ayanamsha);
            let engine = load_engine(&args.bsp, &args.lsk);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            let config = SankrantiConfig::new(system, args.nutation);
            match dhruv_search::sidereal_sum_at(&engine, jd_tdb, &config) {
                Ok(val) => println!("{:.6}°", val),
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::BodyLonLat(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let b = require_body(args.body);
            let engine = load_engine(&args.bsp, &args.lsk);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            match dhruv_search::body_ecliptic_lon_lat(&engine, b, jd_tdb) {
                Ok((lon, lat)) => println!("Longitude: {:.6}°  Latitude: {:.6}°", lon, lat),
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::VedicDaySunrises(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&args.bsp, &args.lsk);
            let eop_kernel = load_eop(&args.eop);
            let location = GeoLocation {
                latitude_deg: args.lat,
                longitude_deg: args.lon,
                altitude_m: args.alt,
            };
            let rs_config = RiseSetConfig::default();
            match dhruv_search::vedic_day_sunrises(
                &engine,
                &eop_kernel,
                &utc,
                &location,
                &rs_config,
            ) {
                Ok((sunrise_jd, next_sunrise_jd)) => {
                    println!("Sunrise JD:      {:.6}", sunrise_jd);
                    println!("Next Sunrise JD: {:.6}", next_sunrise_jd);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::TithiAt(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&args.bsp, &args.lsk);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            match dhruv_search::tithi_at(&engine, jd_tdb, args.elongation) {
                Ok(info) => {
                    println!(
                        "{} ({} {}) - Start: {} End: {}",
                        info.tithi.name(),
                        info.paksha.name(),
                        info.tithi_in_paksha,
                        info.start,
                        info.end
                    );
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::KaranaAt(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&args.bsp, &args.lsk);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            match dhruv_search::karana_at(&engine, jd_tdb, args.elongation) {
                Ok(info) => {
                    println!(
                        "{} (index {}) - Start: {} End: {}",
                        info.karana.name(),
                        info.karana_index,
                        info.start,
                        info.end
                    );
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::YogaAt(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let system = require_aya_system(args.ayanamsha);
            let engine = load_engine(&args.bsp, &args.lsk);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            let config = SankrantiConfig::new(system, args.nutation);
            match dhruv_search::yoga_at(&engine, jd_tdb, args.sum, &config) {
                Ok(info) => {
                    println!(
                        "{} (index {}) - Start: {} End: {}",
                        info.yoga.name(),
                        info.yoga_index,
                        info.start,
                        info.end
                    );
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::NakshatraAt(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let system = require_aya_system(args.ayanamsha);
            let engine = load_engine(&args.bsp, &args.lsk);
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, engine.lsk(), time_policy);
            let config = SankrantiConfig::new(system, args.nutation);
            match dhruv_search::nakshatra_at(&engine, jd_tdb, args.moon_sid, &config) {
                Ok(info) => {
                    println!(
                        "{} (index {}) Pada {} - Start: {} End: {}",
                        info.nakshatra.name(),
                        info.nakshatra_index,
                        info.pada,
                        info.start,
                        info.end
                    );
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        // -----------------------------------------------------------
        // Low-level Ashtakavarga / Drishti
        // -----------------------------------------------------------
        Commands::CalculateAshtakavarga {
            graha_rashis,
            lagna_rashi,
        } => {
            let rashis = parse_graha_rashis(&graha_rashis);
            let result = dhruv_vedic_base::calculate_ashtakavarga(&rashis, lagna_rashi);
            let graha_names = [
                "Sun", "Moon", "Mars", "Mercury", "Jupiter", "Venus", "Saturn",
            ];
            println!("BAV (Bhinna Ashtakavarga):");
            for (i, bav) in result.bavs.iter().enumerate() {
                println!(
                    "  {:8} {:?} (total: {})",
                    graha_names[i],
                    bav.points,
                    bav.total()
                );
                println!("           contributors [rashi][Sun..Lagna]:");
                for (rashi_idx, row) in bav.contributors.iter().enumerate() {
                    print!("             {:>2}: [", rashi_idx);
                    for (j, &v) in row.iter().enumerate() {
                        if j > 0 {
                            print!(", ");
                        }
                        print!("{v}");
                    }
                    let row_sum: u8 = row.iter().sum();
                    println!("] -> {}", row_sum);
                }
            }
            println!("\nSAV (Sarva Ashtakavarga):");
            println!("  Total:       {:?}", result.sav.total_points);
            println!("  Trikona:     {:?}", result.sav.after_trikona);
            println!("  Ekadhipatya: {:?}", result.sav.after_ekadhipatya);
        }

        Commands::GrahaDrishtiCompute {
            graha,
            source,
            target,
        } => {
            let g = require_graha(graha);
            let entry = dhruv_vedic_base::graha_drishti(g, source, target);
            println!(
                "Distance: {:.2}°  Base: {:.2}  Special: {:.2}  Total: {:.2} virupa",
                entry.angular_distance, entry.base_virupa, entry.special_virupa, entry.total_virupa
            );
        }

        Commands::GrahaDrishtiMatrixCompute { longitudes } => {
            let lons = parse_longitudes_9(&longitudes);
            let matrix = dhruv_vedic_base::graha_drishti_matrix(&lons);
            let names = [
                "Surya", "Chandra", "Mangal", "Buddh", "Guru", "Shukra", "Shani", "Rahu", "Ketu",
            ];
            println!("Graha Drishti Matrix (total virupa):\n");
            print!("{:>10}", "");
            for name in &names {
                print!("{:>9}", name);
            }
            println!();
            for (i, name) in names.iter().enumerate() {
                print!("{:>10}", name);
                for j in 0..9 {
                    print!("{:>9.1}", matrix.entries[i][j].total_virupa);
                }
                println!();
            }
        }
        Commands::Shadbala(args) => {
            let system = require_aya_system(args.ayanamsha);
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&args.bsp, &args.lsk);
            let eop_kernel = load_eop(&args.eop);
            let location = GeoLocation::new(args.lat, args.lon, args.alt);
            let bhava_config = bhava_config_from_cli(&args.bhava_behavior);
            let rs_config = RiseSetConfig::default();
            let aya_config = SankrantiConfig::new(system, args.nutation);
            let amsha_selection = args
                .amsha
                .as_deref()
                .map(parse_amsha_specs)
                .map(|requests| amsha_selection_from_requests(&requests))
                .unwrap_or_default();

            let graha_names = [
                "Sun", "Moon", "Mars", "Mercury", "Jupiter", "Venus", "Saturn",
            ];

            if let Some(name) = args.graha {
                let g = parse_graha_name(&name);
                let entry = dhruv_search::shadbala_for_graha(
                    &engine,
                    &eop_kernel,
                    &utc,
                    &location,
                    &bhava_config,
                    &rs_config,
                    &aya_config,
                    &amsha_selection,
                    g,
                )
                .unwrap_or_else(|e| {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                });
                println!("Shadbala for {} on {}\n", g.name(), args.date);
                print_shadbala_entry(&entry);
            } else {
                let result = dhruv_search::shadbala_for_date(
                    &engine,
                    &eop_kernel,
                    &utc,
                    &location,
                    &bhava_config,
                    &rs_config,
                    &aya_config,
                    &amsha_selection,
                )
                .unwrap_or_else(|e| {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                });

                println!(
                    "Shadbala for {} at {:.6}°N, {:.6}°E\n",
                    args.date, args.lat, args.lon
                );
                println!(
                    "{:<8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>6}",
                    "Graha",
                    "Sthana",
                    "Dig",
                    "Kala",
                    "Cheshta",
                    "Nais",
                    "Drik",
                    "Total",
                    "Reqd",
                    "Strong"
                );
                println!("{}", "-".repeat(88));
                for (i, entry) in result.entries.iter().enumerate() {
                    println!(
                        "{:<8} {:>8.2} {:>8.2} {:>8.2} {:>8.2} {:>8.2} {:>8.2} {:>8.2} {:>8.2} {:>6}",
                        graha_names[i],
                        entry.sthana.total,
                        entry.dig,
                        entry.kala.total,
                        entry.cheshta,
                        entry.naisargika,
                        entry.drik,
                        entry.total_shashtiamsas,
                        entry.required_strength,
                        if entry.is_strong { "Yes" } else { "No" },
                    );
                }
            }
        }
        Commands::Bhavabala(args) => {
            let system = require_aya_system(args.ayanamsha);
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&args.bsp, &args.lsk);
            let eop_kernel = load_eop(&args.eop);
            let location = GeoLocation::new(args.lat, args.lon, args.alt);
            let bhava_config = bhava_config_from_cli(&args.bhava_behavior);
            let rs_config = RiseSetConfig::default();
            let aya_config = SankrantiConfig::new(system, args.nutation);

            if let Some(bhava_number) = args.bhava {
                let entry = dhruv_search::bhavabala_for_bhava(
                    &engine,
                    &eop_kernel,
                    &utc,
                    &location,
                    &bhava_config,
                    &rs_config,
                    &aya_config,
                    bhava_number,
                )
                .unwrap_or_else(|e| {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                });
                println!("Bhava Bala for Bhava {} on {}\n", bhava_number, args.date);
                print_bhavabala_entry(&entry);
            } else {
                let result = dhruv_search::bhavabala_for_date(
                    &engine,
                    &eop_kernel,
                    &utc,
                    &location,
                    &bhava_config,
                    &rs_config,
                    &aya_config,
                )
                .unwrap_or_else(|e| {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                });

                println!(
                    "Bhava Bala for {} at {:.6}°N, {:.6}°E\n",
                    args.date, args.lat, args.lon
                );
                println!(
                    "{:<6} {:<10} {:<8} {:>11} {:>8} {:>8} {:>8} {:>8} {:>8}",
                    "Bhava",
                    "Rashi",
                    "Lord",
                    "Bhavadhip",
                    "Dig",
                    "Drishti",
                    "Occup",
                    "Rise",
                    "Total"
                );
                println!("{}", "-".repeat(86));
                for entry in &result.entries {
                    println!(
                        "{:<6} {:<10} {:<8} {:>11.2} {:>8.2} {:>8.2} {:>8.2} {:>8.2} {:>8.2}",
                        entry.bhava_number,
                        dhruv_vedic_base::ALL_RASHIS[entry.rashi_index as usize].name(),
                        entry.lord.name(),
                        entry.bhavadhipati,
                        entry.dig,
                        entry.drishti,
                        entry.occupation_bonus,
                        entry.rising_bonus,
                        entry.total_virupas,
                    );
                }
            }
        }
        Commands::Charakaraka(args) => {
            let system = require_aya_system(args.ayanamsha);
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let scheme = parse_charakaraka_scheme(&args.scheme);
            let engine = load_engine(&args.bsp, &args.lsk);
            let eop_kernel = load_eop(&args.eop);
            let aya_config = SankrantiConfig::new(system, args.nutation);

            let result =
                dhruv_search::charakaraka_for_date(&engine, &eop_kernel, &utc, &aya_config, scheme)
                    .unwrap_or_else(|e| {
                        eprintln!("Error: {e}");
                        std::process::exit(1);
                    });

            println!(
                "Charakaraka ({:?}, ayanamsha={:?}, nutation={}) for {}\n",
                result.scheme, system, args.nutation, args.date
            );
            println!(
                "{:<4} {:<14} {:<8} {:<26} {:>8}",
                "Rank", "Role", "Graha", "Longitude", "Eff°"
            );
            println!("{}", "-".repeat(68));
            for e in &result.entries {
                println!(
                    "{:<4} {:<14} {:<8} {:<26} {:>8.3}",
                    e.rank,
                    charakaraka_role_name(e.role),
                    e.graha.name(),
                    format_rashi_dms(e.longitude_deg),
                    e.effective_degrees_in_rashi
                );
            }
            if result.scheme == dhruv_vedic_base::CharakarakaScheme::MixedParashara {
                println!(
                    "\nMixed mode resolved to {}-karaka set",
                    if result.used_eight_karakas { 8 } else { 7 }
                );
            }
        }
        Commands::Vimsopaka(args) => {
            let system = require_aya_system(args.ayanamsha);
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&args.bsp, &args.lsk);
            let eop_kernel = load_eop(&args.eop);
            let location = GeoLocation::new(args.lat, args.lon, args.alt);
            let aya_config = SankrantiConfig::new(system, args.nutation);
            let policy = parse_node_policy(&args.node_policy);
            let amsha_selection = args
                .amsha
                .as_deref()
                .map(parse_amsha_specs)
                .map(|requests| amsha_selection_from_requests(&requests))
                .unwrap_or_default();

            let graha_names = [
                "Sun", "Moon", "Mars", "Mercury", "Jupiter", "Venus", "Saturn", "Rahu", "Ketu",
            ];

            if let Some(name) = args.graha {
                let g = parse_graha_name(&name);
                let entry = dhruv_search::vimsopaka_for_graha(
                    &engine,
                    &eop_kernel,
                    &utc,
                    &location,
                    &aya_config,
                    policy,
                    &amsha_selection,
                    g,
                )
                .unwrap_or_else(|e| {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                });
                println!("Vimsopaka for {} on {}\n", g.name(), args.date);
                println!("  Shadvarga:     {:>6.2}/20", entry.shadvarga);
                println!("  Saptavarga:    {:>6.2}/20", entry.saptavarga);
                println!("  Dashavarga:    {:>6.2}/20", entry.dashavarga);
                println!("  Shodasavarga:  {:>6.2}/20", entry.shodasavarga);
            } else {
                let result = dhruv_search::vimsopaka_for_date(
                    &engine,
                    &eop_kernel,
                    &utc,
                    &location,
                    &aya_config,
                    policy,
                    &amsha_selection,
                )
                .unwrap_or_else(|e| {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                });

                println!(
                    "Vimsopaka Bala for {} at {:.6}°N, {:.6}°E\n",
                    args.date, args.lat, args.lon
                );
                println!(
                    "{:<8} {:>10} {:>10} {:>10} {:>12}",
                    "Graha", "Shadvarga", "Saptavarga", "Dashavarga", "Shodasavarga"
                );
                println!("{}", "-".repeat(58));
                for (i, entry) in result.entries.iter().enumerate() {
                    println!(
                        "{:<8} {:>10.2} {:>10.2} {:>10.2} {:>12.2}",
                        graha_names[i],
                        entry.shadvarga,
                        entry.saptavarga,
                        entry.dashavarga,
                        entry.shodasavarga,
                    );
                }
            }
        }
        Commands::Balas(args) => {
            let system = require_aya_system(args.ayanamsha);
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&args.bsp, &args.lsk);
            let eop_kernel = load_eop(&args.eop);
            let location = GeoLocation::new(args.lat, args.lon, args.alt);
            let bhava_config = bhava_config_from_cli(&args.bhava_behavior);
            let rs_config = RiseSetConfig::default();
            let aya_config = SankrantiConfig::new(system, args.nutation);
            let policy = parse_node_policy(&args.node_policy);
            let amsha_selection = args
                .amsha
                .as_deref()
                .map(parse_amsha_specs)
                .map(|requests| amsha_selection_from_requests(&requests))
                .unwrap_or_default();

            let result = dhruv_search::balas_for_date(
                &engine,
                &eop_kernel,
                &utc,
                &location,
                &bhava_config,
                &rs_config,
                &aya_config,
                policy,
                &amsha_selection,
            )
            .unwrap_or_else(|e| {
                eprintln!("Error: {e}");
                std::process::exit(1);
            });

            println!(
                "Balas for {} at {:.6}°N, {:.6}°E\n",
                args.date, args.lat, args.lon
            );
            println!("Shadbala:");
            println!(
                "{:<8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>6}",
                "Graha",
                "Sthana",
                "Dig",
                "Kala",
                "Cheshta",
                "Nais",
                "Drik",
                "Total",
                "Reqd",
                "Strong"
            );
            println!("{}", "-".repeat(88));
            for entry in &result.shadbala.entries {
                println!(
                    "{:<8} {:>8.2} {:>8.2} {:>8.2} {:>8.2} {:>8.2} {:>8.2} {:>8.2} {:>8.2} {:>6}",
                    entry.graha.name(),
                    entry.sthana.total,
                    entry.dig,
                    entry.kala.total,
                    entry.cheshta,
                    entry.naisargika,
                    entry.drik,
                    entry.total_shashtiamsas,
                    entry.required_strength,
                    if entry.is_strong { "Yes" } else { "No" },
                );
            }
            println!();

            println!("Vimsopaka Bala:");
            println!(
                "{:<8} {:>10} {:>10} {:>10} {:>12}",
                "Graha", "Shadvarga", "Saptavarga", "Dashavarga", "Shodasavarga"
            );
            println!("{}", "-".repeat(58));
            for entry in &result.vimsopaka.entries {
                println!(
                    "{:<8} {:>10.2} {:>10.2} {:>10.2} {:>12.2}",
                    entry.graha.name(),
                    entry.shadvarga,
                    entry.saptavarga,
                    entry.dashavarga,
                    entry.shodasavarga,
                );
            }
            println!();

            println!("Sarva Ashtakavarga totals:");
            print!("  Total      ");
            for &p in &result.ashtakavarga.sav.total_points {
                print!("{:>4}", p);
            }
            println!();
            print!("  Trikona    ");
            for &p in &result.ashtakavarga.sav.after_trikona {
                print!("{:>4}", p);
            }
            println!();
            print!("  Ekadhipatya");
            for &p in &result.ashtakavarga.sav.after_ekadhipatya {
                print!("{:>4}", p);
            }
            println!("\n");

            println!("Bhava Bala:");
            println!(
                "{:<6} {:<10} {:<8} {:>11} {:>8} {:>8} {:>8} {:>8} {:>8}",
                "Bhava", "Rashi", "Lord", "Bhavadhip", "Dig", "Drishti", "Occup", "Rise", "Total"
            );
            println!("{}", "-".repeat(86));
            for entry in &result.bhavabala.entries {
                println!(
                    "{:<6} {:<10} {:<8} {:>11.2} {:>8.2} {:>8.2} {:>8.2} {:>8.2} {:>8.2}",
                    entry.bhava_number,
                    dhruv_vedic_base::ALL_RASHIS[entry.rashi_index as usize].name(),
                    entry.lord.name(),
                    entry.bhavadhipati,
                    entry.dig,
                    entry.drishti,
                    entry.occupation_bonus,
                    entry.rising_bonus,
                    entry.total_virupas,
                );
            }
        }
        Commands::Amsha(args) => {
            let requests = parse_amsha_specs(&args.amsha);
            let rows = compute_amsha_transform_rows(args.lon, &requests);
            let mut stdout = std::io::stdout();
            write_amsha_transform_rows(&mut stdout, &rows, args.output, args.format)
                .unwrap_or_else(|e| {
                    eprintln!("Failed to write amsha output: {e}");
                    std::process::exit(1);
                });
        }
        Commands::AmshaVariations(args) => {
            let amshas = parse_amsha_list(&args.amsha);
            let mut stdout = std::io::stdout();
            write_amsha_variation_catalogs(&mut stdout, &amshas, args.format).unwrap_or_else(|e| {
                eprintln!("Failed to write amsha variation output: {e}");
                std::process::exit(1);
            });
        }
        Commands::AmshaChart(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let system = require_aya_system(args.ayanamsha);
            let engine = load_engine(&args.bsp, &args.lsk);
            let eop_kernel = load_eop(&args.eop);
            let location = GeoLocation::new(args.lat, args.lon, args.alt);
            let bhava_config = bhava_config_from_cli(&args.bhava_behavior);
            let rs_config = RiseSetConfig::default();
            let aya_config = SankrantiConfig::new(system, args.nutation);
            let requests = parse_amsha_specs(&args.amsha);
            let scope = amsha_scope(
                args.include_bhava_cusps,
                args.include_arudha_padas,
                args.include_upagrahas,
                args.include_sphutas,
                args.include_special_lagnas,
                !args.no_outer_planets,
            );
            let result = dhruv_search::amsha_charts_for_date(
                &engine,
                &eop_kernel,
                &utc,
                &location,
                &bhava_config,
                &rs_config,
                &aya_config,
                &requests,
                &scope,
            )
            .unwrap_or_else(|e| {
                eprintln!("Error: {e}");
                std::process::exit(1);
            });
            println!(
                "Amsha charts for {} at {:.6}°N, {:.6}°E\n",
                args.date, args.lat, args.lon
            );
            let mut stdout = std::io::stdout();
            for (index, chart) in result.charts.iter().enumerate() {
                if index > 0 {
                    println!();
                }
                write_amsha_chart(&mut stdout, chart, "").unwrap_or_else(|e| {
                    eprintln!("Error writing amsha chart output: {e}");
                    std::process::exit(1);
                });
            }
        }
        Commands::Avastha(args) => {
            let system = require_aya_system(args.ayanamsha);
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&args.bsp, &args.lsk);
            let eop_kernel = load_eop(&args.eop);
            let location = GeoLocation::new(args.lat, args.lon, args.alt);
            let bhava_config = bhava_config_from_cli(&args.bhava_behavior);
            let rs_config = RiseSetConfig::default();
            let aya_config = SankrantiConfig::new(system, args.nutation);
            let policy = parse_node_policy(&args.node_policy);
            let amsha_selection = args
                .amsha
                .as_deref()
                .map(parse_amsha_specs)
                .map(|requests| amsha_selection_from_requests(&requests))
                .unwrap_or_default();

            let graha_names = [
                "Sun", "Moon", "Mars", "Mercury", "Jupiter", "Venus", "Saturn", "Rahu", "Ketu",
            ];

            if let Some(name) = args.graha {
                let g = parse_graha_name(&name);
                let entry = dhruv_search::avastha_for_graha(
                    &engine,
                    &eop_kernel,
                    &location,
                    &utc,
                    &bhava_config,
                    &rs_config,
                    &aya_config,
                    policy,
                    &amsha_selection,
                    g,
                )
                .unwrap_or_else(|e| {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                });
                println!("Avasthas for {} on {}\n", g.name(), args.date);
                print_graha_avastha(&entry);
            } else {
                let result = dhruv_search::avastha_for_date(
                    &engine,
                    &eop_kernel,
                    &location,
                    &utc,
                    &bhava_config,
                    &rs_config,
                    &aya_config,
                    policy,
                    &amsha_selection,
                )
                .unwrap_or_else(|e| {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                });

                println!(
                    "Graha Avasthas for {} at {:.6}°N, {:.6}°E\n",
                    args.date, args.lat, args.lon
                );
                println!(
                    "{:<8} {:>10} {:>10} {:>30} {:>10} {:>12}",
                    "Graha", "Baladi", "Jagradadi", "Deeptadi", "Lajjitadi", "Sayanadi"
                );
                println!("{}", "-".repeat(88));
                for (i, entry) in result.entries.iter().enumerate() {
                    println!(
                        "{:<8} {:>10} {:>10} {:>30} {:>10} {:>12}",
                        graha_names[i],
                        entry.baladi.name(),
                        entry.jagradadi.name(),
                        format_deeptadi_states(entry),
                        format_lajjitadi_states(entry),
                        entry.sayanadi.avastha.name(),
                    );
                }
            }
        }
        Commands::Dasha(args) => {
            let aya_system = require_aya_system(args.ayanamsha);
            let engine = load_engine(&args.bsp, &args.lsk);
            let eop_kernel = load_eop(&args.eop);
            let bhava_config = BhavaConfig::default();
            let rs_config = RiseSetConfig::default();
            let aya_config = SankrantiConfig::new(aya_system, args.nutation);
            let dasha_system = parse_dasha_system(&args.system);
            let variation = dhruv_vedic_base::dasha::DashaVariationConfig::default();
            let clamped_level = args.max_level.min(dhruv_vedic_base::dasha::MAX_DASHA_LEVEL);
            let mode = args.mode.as_deref().unwrap_or(
                if args.query_date.is_some() || args.query_jd.is_some() {
                    "snapshot"
                } else {
                    "hierarchy"
                },
            );
            let birth_utc = args.birth_date.as_ref().map(|date| {
                parse_utc(date).unwrap_or_else(|e| {
                    eprintln!("{e}");
                    std::process::exit(1);
                })
            });
            let location = match (args.lat, args.lon) {
                (Some(lat), Some(lon)) => Some(GeoLocation::new(lat, lon, args.alt.unwrap_or(0.0))),
                (None, None) => None,
                _ => {
                    eprintln!("--lat and --lon must be provided together.");
                    std::process::exit(1);
                }
            };
            let raw_rashi_inputs = match (
                args.graha_sidereal_lons.as_ref(),
                args.lagna_sidereal_lon,
            ) {
                (Some(lons), Some(lagna_sidereal_lon)) => {
                    Some(dhruv_vedic_base::dasha::RashiDashaInputs::new(
                        parse_longitudes_9(lons),
                        lagna_sidereal_lon,
                    ))
                }
                (None, None) => None,
                _ => {
                    eprintln!(
                        "--graha-sidereal-lons and --lagna-sidereal-lon must be provided together."
                    );
                    std::process::exit(1);
                }
            };
            let raw_sunrise_sunset = match (args.sunrise_jd, args.sunset_jd) {
                (Some(sunrise_jd), Some(sunset_jd)) => Some((sunrise_jd, sunset_jd)),
                (None, None) => None,
                _ => {
                    eprintln!("--sunrise-jd and --sunset-jd must be provided together.");
                    std::process::exit(1);
                }
            };
            let raw_inputs_requested = args.moon_sid_lon.is_some()
                || raw_rashi_inputs.is_some()
                || raw_sunrise_sunset.is_some();
            let raw_inputs = dhruv_search::DashaInputs {
                moon_sid_lon: args.moon_sid_lon,
                rashi_inputs: raw_rashi_inputs.as_ref(),
                sunrise_sunset: raw_sunrise_sunset,
            };
            let birth_jd = if raw_inputs_requested {
                args.birth_jd
                    .or_else(|| birth_utc.as_ref().map(utc_to_jd_utc))
                    .unwrap_or_else(|| {
                        eprintln!(
                            "--birth-jd or --birth-date is required when raw dasha inputs are provided."
                        );
                        std::process::exit(1);
                    })
            } else {
                0.0
            };
            if !raw_inputs_requested && args.birth_jd.is_some() {
                eprintln!("--birth-jd is only supported together with raw dasha inputs.");
                std::process::exit(1);
            }
            if !raw_inputs_requested && args.query_jd.is_some() {
                eprintln!("--query-jd is only supported together with raw dasha inputs.");
                std::process::exit(1);
            }
            let birth_label = if raw_inputs_requested {
                if let Some(date) = args.birth_date.as_deref() {
                    date.to_string()
                } else {
                    format!("JD UTC {:.6}", birth_jd)
                }
            } else {
                args.birth_date.clone().unwrap_or_else(|| {
                    eprintln!("--birth-date is required when raw dasha inputs are not provided.");
                    std::process::exit(1);
                })
            };

            match mode {
                "snapshot" => {
                    let snapshot = if raw_inputs_requested {
                        let query_jd = args
                            .query_jd
                            .or_else(|| {
                                args.query_date.as_ref().map(|date| {
                                    let utc = parse_utc(date).unwrap_or_else(|e| {
                                        eprintln!("{e}");
                                        std::process::exit(1);
                                    });
                                    utc_to_jd_utc(&utc)
                                })
                            })
                            .unwrap_or_else(|| {
                                eprintln!(
                                    "--query-jd or --query-date is required for --mode snapshot"
                                );
                                std::process::exit(1);
                            });
                        dhruv_search::dasha_snapshot_with_inputs(
                            birth_jd,
                            query_jd,
                            dasha_system,
                            clamped_level,
                            &variation,
                            &raw_inputs,
                        )
                    } else {
                        let birth_utc = birth_utc.as_ref().unwrap_or_else(|| {
                            eprintln!("--birth-date is required for --mode snapshot");
                            std::process::exit(1);
                        });
                        let location = location.as_ref().unwrap_or_else(|| {
                            eprintln!("--lat and --lon are required for --mode snapshot");
                            std::process::exit(1);
                        });
                        let q_date = args.query_date.as_ref().unwrap_or_else(|| {
                            eprintln!("--query-date is required for --mode snapshot");
                            std::process::exit(1);
                        });
                        let query_utc = parse_utc(q_date).unwrap_or_else(|e| {
                            eprintln!("{e}");
                            std::process::exit(1);
                        });
                        dhruv_search::dasha_snapshot_at(
                            &engine,
                            &eop_kernel,
                            birth_utc,
                            &query_utc,
                            location,
                            dasha_system,
                            clamped_level,
                            &bhava_config,
                            &rs_config,
                            &aya_config,
                            &variation,
                        )
                    }
                    .unwrap_or_else(|e| {
                        eprintln!("Error: {e}");
                        std::process::exit(1);
                    });
                    println!(
                        "Dasha Snapshot ({}) at {} for birth {}\n",
                        dasha_system.name(),
                        args.query_date
                            .clone()
                            .unwrap_or_else(|| format!("JD UTC {:.6}", args.query_jd.unwrap())),
                        birth_label
                    );
                    print_dasha_periods(&snapshot.periods, 0, 50);
                }
                "hierarchy" => {
                    let hierarchy = if raw_inputs_requested {
                        dhruv_search::dasha_hierarchy_with_inputs(
                            birth_jd,
                            dasha_system,
                            clamped_level,
                            &variation,
                            &raw_inputs,
                        )
                    } else {
                        let birth_utc = birth_utc.as_ref().unwrap_or_else(|| {
                            eprintln!("--birth-date is required for --mode hierarchy");
                            std::process::exit(1);
                        });
                        let location = location.as_ref().unwrap_or_else(|| {
                            eprintln!("--lat and --lon are required for --mode hierarchy");
                            std::process::exit(1);
                        });
                        dhruv_search::dasha_hierarchy_for_birth(
                            &engine,
                            &eop_kernel,
                            birth_utc,
                            location,
                            dasha_system,
                            clamped_level,
                            &bhava_config,
                            &rs_config,
                            &aya_config,
                            &variation,
                        )
                    }
                    .unwrap_or_else(|e| {
                        eprintln!("Error: {e}");
                        std::process::exit(1);
                    });
                    println!(
                        "Dasha Hierarchy ({}) for birth {} ({} levels)\n",
                        dasha_system.name(),
                        birth_label,
                        hierarchy.levels.len()
                    );
                    print_dasha_hierarchy(&hierarchy);
                }
                "level0" => {
                    let periods = if raw_inputs_requested {
                        dhruv_search::dasha_level0_with_inputs(birth_jd, dasha_system, &raw_inputs)
                    } else {
                        let birth_utc = birth_utc.as_ref().unwrap_or_else(|| {
                            eprintln!("--birth-date is required for --mode level0");
                            std::process::exit(1);
                        });
                        let location = location.as_ref().unwrap_or_else(|| {
                            eprintln!("--lat and --lon are required for --mode level0");
                            std::process::exit(1);
                        });
                        dhruv_search::dasha_level0_for_birth(
                            &engine,
                            &eop_kernel,
                            birth_utc,
                            location,
                            dasha_system,
                            &bhava_config,
                            &rs_config,
                            &aya_config,
                        )
                    }
                    .unwrap_or_else(|e| {
                        eprintln!("Error: {e}");
                        std::process::exit(1);
                    });
                    println!(
                        "Dasha Level0 ({}) for birth {}\n",
                        dasha_system.name(),
                        birth_label
                    );
                    print_dasha_periods(&periods, 1, 100);
                }
                "level0-entity" => {
                    let entity =
                        parse_dasha_entity_spec(args.entity.as_deref().unwrap_or_else(|| {
                            eprintln!("--entity is required for --mode level0-entity");
                            std::process::exit(1);
                        }));
                    let period = if raw_inputs_requested {
                        dhruv_search::dasha_level0_entity_with_inputs(
                            birth_jd,
                            dasha_system,
                            entity,
                            &raw_inputs,
                        )
                    } else {
                        let birth_utc = birth_utc.as_ref().unwrap_or_else(|| {
                            eprintln!("--birth-date is required for --mode level0-entity");
                            std::process::exit(1);
                        });
                        let location = location.as_ref().unwrap_or_else(|| {
                            eprintln!("--lat and --lon are required for --mode level0-entity");
                            std::process::exit(1);
                        });
                        dhruv_search::dasha_level0_entity_for_birth(
                            &engine,
                            &eop_kernel,
                            birth_utc,
                            location,
                            dasha_system,
                            entity,
                            &bhava_config,
                            &rs_config,
                            &aya_config,
                        )
                    }
                    .unwrap_or_else(|e| {
                        eprintln!("Error: {e}");
                        std::process::exit(1);
                    });
                    match period {
                        Some(period) => {
                            println!(
                                "Dasha Level0 Entity ({}) for birth {}\n",
                                dasha_system.name(),
                                birth_label
                            );
                            print_dasha_periods(&[period], 1, 1);
                        }
                        None => println!("No matching level0 period found."),
                    }
                }
                "children" | "child-period" | "complete-level" => {
                    let parent_level = args.parent_level.unwrap_or_else(|| {
                        eprintln!("--parent-level is required for --mode {mode}");
                        std::process::exit(1);
                    });
                    let hierarchy = if raw_inputs_requested {
                        dhruv_search::dasha_hierarchy_with_inputs(
                            birth_jd,
                            dasha_system,
                            parent_level.min(dhruv_vedic_base::dasha::MAX_DASHA_LEVEL),
                            &variation,
                            &raw_inputs,
                        )
                    } else {
                        let birth_utc = birth_utc.as_ref().unwrap_or_else(|| {
                            eprintln!("--birth-date is required for --mode {mode}");
                            std::process::exit(1);
                        });
                        let location = location.as_ref().unwrap_or_else(|| {
                            eprintln!("--lat and --lon are required for --mode {mode}");
                            std::process::exit(1);
                        });
                        dhruv_search::dasha_hierarchy_for_birth(
                            &engine,
                            &eop_kernel,
                            birth_utc,
                            location,
                            dasha_system,
                            parent_level.min(dhruv_vedic_base::dasha::MAX_DASHA_LEVEL),
                            &bhava_config,
                            &rs_config,
                            &aya_config,
                            &variation,
                        )
                    }
                    .unwrap_or_else(|e| {
                        eprintln!("Error: {e}");
                        std::process::exit(1);
                    });
                    let parent_level_idx = parent_level as usize;
                    if parent_level_idx >= hierarchy.levels.len() {
                        eprintln!("Parent level {parent_level} is not available.");
                        std::process::exit(1);
                    }
                    let parent_periods = &hierarchy.levels[parent_level_idx];

                    if mode == "complete-level" {
                        let child_level = dhruv_vedic_base::dasha::DashaLevel::from_u8(
                            parent_level.saturating_add(1),
                        )
                        .unwrap_or_else(|| {
                            eprintln!(
                                "No deeper child level exists for parent level {parent_level}"
                            );
                            std::process::exit(1);
                        });
                        let periods = if raw_inputs_requested {
                            dhruv_search::dasha_complete_level_with_inputs(
                                dasha_system,
                                parent_periods,
                                child_level,
                                &variation,
                                &raw_inputs,
                            )
                        } else {
                            let birth_utc = birth_utc.as_ref().unwrap_or_else(|| {
                                eprintln!("--birth-date is required for --mode complete-level");
                                std::process::exit(1);
                            });
                            let location = location.as_ref().unwrap_or_else(|| {
                                eprintln!("--lat and --lon are required for --mode complete-level");
                                std::process::exit(1);
                            });
                            dhruv_search::dasha_complete_level_for_birth(
                                &engine,
                                &eop_kernel,
                                birth_utc,
                                location,
                                dasha_system,
                                parent_periods,
                                child_level,
                                &bhava_config,
                                &rs_config,
                                &aya_config,
                                &variation,
                            )
                        }
                        .unwrap_or_else(|e| {
                            eprintln!("Error: {e}");
                            std::process::exit(1);
                        });
                        println!(
                            "Dasha Complete Level ({}) child level {}\n",
                            dasha_system.name(),
                            child_level.name()
                        );
                        print_dasha_periods(&periods, 1, 100);
                    } else {
                        let parent_index = args.parent_index.unwrap_or_else(|| {
                            eprintln!("--parent-index is required for --mode {mode}");
                            std::process::exit(1);
                        }) as usize;
                        let parent = parent_periods.get(parent_index).unwrap_or_else(|| {
                            eprintln!("Parent index {parent_index} is out of range.");
                            std::process::exit(1);
                        });

                        if mode == "children" {
                            let children = if raw_inputs_requested {
                                dhruv_search::dasha_children_with_inputs(
                                    dasha_system,
                                    parent,
                                    &variation,
                                    &raw_inputs,
                                )
                            } else {
                                let birth_utc = birth_utc.as_ref().unwrap_or_else(|| {
                                    eprintln!("--birth-date is required for --mode children");
                                    std::process::exit(1);
                                });
                                let location = location.as_ref().unwrap_or_else(|| {
                                    eprintln!("--lat and --lon are required for --mode children");
                                    std::process::exit(1);
                                });
                                dhruv_search::dasha_children_for_birth(
                                    &engine,
                                    &eop_kernel,
                                    birth_utc,
                                    location,
                                    dasha_system,
                                    parent,
                                    &bhava_config,
                                    &rs_config,
                                    &aya_config,
                                    &variation,
                                )
                            }
                            .unwrap_or_else(|e| {
                                eprintln!("Error: {e}");
                                std::process::exit(1);
                            });
                            println!(
                                "Dasha Children ({}) parent level {} index {}\n",
                                dasha_system.name(),
                                parent_level,
                                parent_index
                            );
                            print_dasha_periods(&children, 1, 100);
                        } else {
                            let entity = parse_dasha_entity_spec(
                                args.entity.as_deref().unwrap_or_else(|| {
                                    eprintln!("--entity is required for --mode child-period");
                                    std::process::exit(1);
                                }),
                            );
                            let child = if raw_inputs_requested {
                                dhruv_search::dasha_child_period_with_inputs(
                                    dasha_system,
                                    parent,
                                    entity,
                                    &variation,
                                    &raw_inputs,
                                )
                            } else {
                                let birth_utc = birth_utc.as_ref().unwrap_or_else(|| {
                                    eprintln!("--birth-date is required for --mode child-period");
                                    std::process::exit(1);
                                });
                                let location = location.as_ref().unwrap_or_else(|| {
                                    eprintln!(
                                        "--lat and --lon are required for --mode child-period"
                                    );
                                    std::process::exit(1);
                                });
                                dhruv_search::dasha_child_period_for_birth(
                                    &engine,
                                    &eop_kernel,
                                    birth_utc,
                                    location,
                                    dasha_system,
                                    parent,
                                    entity,
                                    &bhava_config,
                                    &rs_config,
                                    &aya_config,
                                    &variation,
                                )
                            }
                            .unwrap_or_else(|e| {
                                eprintln!("Error: {e}");
                                std::process::exit(1);
                            });
                            match child {
                                Some(period) => {
                                    println!(
                                        "Dasha Child Period ({}) parent level {} index {}\n",
                                        dasha_system.name(),
                                        parent_level,
                                        parent_index
                                    );
                                    print_dasha_periods(&[period], 1, 1);
                                }
                                None => println!("No matching child period found."),
                            }
                        }
                    }
                }
                other => {
                    eprintln!("Unknown dasha mode: {other}");
                    eprintln!(
                        "Valid: hierarchy, snapshot, level0, level0-entity, children, child-period, complete-level"
                    );
                    std::process::exit(1);
                }
            }
        }
        Commands::TaraList { catalog, category } => {
            let cat = TaraCatalog::load(&catalog).unwrap_or_else(|e| {
                eprintln!("Failed to load catalog: {e}");
                std::process::exit(1);
            });
            let filter = category.as_deref();
            println!(
                "{:<20} {:<12} {:>8} {:>10} {:>10} {:>8}",
                "ID", "Category", "RA(°)", "Dec(°)", "Plx(mas)", "Vmag"
            );
            println!("{}", "-".repeat(78));
            for (id, entry) in cat.iter() {
                let cat_name = format!("{:?}", id.category());
                if let Some(f) = filter {
                    let matches = match f {
                        "yogatara" => {
                            matches!(id.category(), dhruv_tara::TaraCategory::Yogatara)
                        }
                        "rashi" => {
                            matches!(id.category(), dhruv_tara::TaraCategory::RashiConstellation)
                        }
                        "special" => {
                            matches!(id.category(), dhruv_tara::TaraCategory::SpecialVedic)
                        }
                        "galactic" => {
                            matches!(id.category(), dhruv_tara::TaraCategory::GalacticReference)
                        }
                        _ => true,
                    };
                    if !matches {
                        continue;
                    }
                }
                println!(
                    "{:<20} {:<12} {:>8.3} {:>10.3} {:>10.2} {:>8.2}",
                    id.as_str(),
                    cat_name,
                    entry.ra_deg,
                    entry.dec_deg,
                    entry.parallax_mas,
                    entry.v_mag,
                );
            }
        }
        Commands::TaraPosition(args) => {
            let id = TaraId::from_str(&args.star).unwrap_or_else(|| {
                eprintln!("Unknown star: {}", args.star);
                std::process::exit(1);
            });
            let cat = TaraCatalog::load(&args.catalog).unwrap_or_else(|e| {
                eprintln!("Failed to load catalog: {e}");
                std::process::exit(1);
            });
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let resolved_engine_cfg = resolve_engine_config_for_cli(&args.bsp, &args.lsk)
                .unwrap_or_else(|e| {
                    eprintln!("Failed to resolve engine for tara position: {e}");
                    std::process::exit(1);
                });
            let lsk_kernel = dhruv_time::LeapSecondKernel::load(&resolved_engine_cfg.lsk_path)
                .unwrap_or_else(|e| {
                    eprintln!("Failed to load LSK: {e}");
                    std::process::exit(1);
                });
            let jd_tdb = utc_to_jd_tdb_with_policy(&utc, &lsk_kernel, time_policy);

            let config = TaraConfig {
                accuracy: if args.apparent {
                    TaraAccuracy::Apparent
                } else {
                    TaraAccuracy::Astrometric
                },
                apply_parallax: args.parallax,
            };

            // Get Earth state if needed
            let earth_state = if args.apparent || args.parallax {
                let engine = load_engine(&args.bsp, &args.lsk);
                let q = Query {
                    target: Body::Earth,
                    observer: Observer::SolarSystemBarycenter,
                    frame: Frame::IcrfJ2000,
                    epoch_tdb_jd: jd_tdb,
                };
                let state = engine.query(q).unwrap_or_else(|e| {
                    eprintln!("Failed to query Earth state: {e}");
                    std::process::exit(1);
                });
                let au_km = dhruv_tara::propagation::AU_KM;
                let km_s_to_au_day = 86400.0 / au_km;
                Some(EarthState {
                    position_au: [
                        state.position_km[0] / au_km,
                        state.position_km[1] / au_km,
                        state.position_km[2] / au_km,
                    ],
                    velocity_au_day: [
                        state.velocity_km_s[0] * km_s_to_au_day,
                        state.velocity_km_s[1] * km_s_to_au_day,
                        state.velocity_km_s[2] * km_s_to_au_day,
                    ],
                })
            } else {
                None
            };

            let op_equatorial = TaraOperation {
                star: id,
                output: TaraOutputKind::Equatorial,
                at_jd_tdb: jd_tdb,
                ayanamsha_deg: 0.0,
                config,
                earth_state,
            };
            match dhruv_vedic_ops::tara(&cat, &op_equatorial) {
                Ok(TaraResult::Equatorial(pos)) => {
                    println!("Equatorial (ICRS):");
                    println!("  RA:       {:.6}°", pos.ra_deg);
                    println!("  Dec:      {:.6}°", pos.dec_deg);
                    println!("  Distance: {:.2} AU", pos.distance_au);
                }
                Ok(_) => {}
                Err(e) => eprintln!("Equatorial error: {e}"),
            }

            let op_ecliptic = TaraOperation {
                star: id,
                output: TaraOutputKind::Ecliptic,
                at_jd_tdb: jd_tdb,
                ayanamsha_deg: 0.0,
                config,
                earth_state,
            };
            match dhruv_vedic_ops::tara(&cat, &op_ecliptic) {
                Ok(TaraResult::Ecliptic(sc)) => {
                    println!("Ecliptic (of date):");
                    println!("  Longitude: {:.6}°", sc.lon_deg);
                    println!("  Latitude:  {:.6}°", sc.lat_deg);
                }
                Ok(_) => {}
                Err(e) => eprintln!("Ecliptic error: {e}"),
            }

            // Sidereal longitude
            let system = require_aya_system(args.ayanamsha);
            let t = jd_tdb_to_centuries(jd_tdb);
            let aya = ayanamsha_deg(system, t, args.nutation);
            let op_sidereal = TaraOperation {
                star: id,
                output: TaraOutputKind::Sidereal,
                at_jd_tdb: jd_tdb,
                ayanamsha_deg: aya,
                config,
                earth_state,
            };
            match dhruv_vedic_ops::tara(&cat, &op_sidereal) {
                Ok(TaraResult::Sidereal(lon)) => {
                    let rashi_info = rashi_from_longitude(lon);
                    let nak_info = nakshatra_from_longitude(lon);
                    println!("Sidereal ({:?}, nutation={}):", system, args.nutation);
                    println!("  Longitude: {:.6}°", lon);
                    println!("  Rashi:     {}", rashi_info.rashi.name());
                    println!(
                        "  Nakshatra: {} (pada {})",
                        nak_info.nakshatra.name(),
                        nak_info.pada
                    );
                    println!("  Ayanamsha: {:.6}°", aya);
                }
                Ok(_) => {}
                Err(e) => eprintln!("Sidereal error: {e}"),
            }
        }
    }
}

fn parse_dasha_systems_config(s: &str, max_level: u8) -> dhruv_search::DashaSelectionConfig {
    let mut seen = std::collections::HashSet::new();
    let mut systems = [0xFFu8; dhruv_vedic_base::dasha::MAX_DASHA_SYSTEMS];
    let mut count = 0usize;

    for token in s.split(',') {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            eprintln!("Error: empty dasha system name in --dasha-systems");
            std::process::exit(1);
        }
        let ds = parse_dasha_system(trimmed);
        let code = ds as u8;
        if !seen.insert(code) {
            eprintln!("Warning: ignoring duplicate system: {trimmed}");
            continue;
        }
        if count >= dhruv_vedic_base::dasha::MAX_DASHA_SYSTEMS {
            eprintln!(
                "Error: too many dasha systems (max {})",
                dhruv_vedic_base::dasha::MAX_DASHA_SYSTEMS
            );
            std::process::exit(1);
        }
        systems[count] = code;
        count += 1;
    }

    dhruv_search::DashaSelectionConfig {
        count: count as u8,
        systems,
        max_level,
        snapshot_time: None,
        ..dhruv_search::DashaSelectionConfig::default()
    }
}

fn parse_dasha_system(s: &str) -> dhruv_vedic_base::dasha::DashaSystem {
    match s.to_lowercase().as_str() {
        "vimshottari" => dhruv_vedic_base::dasha::DashaSystem::Vimshottari,
        "ashtottari" => dhruv_vedic_base::dasha::DashaSystem::Ashtottari,
        "shodsottari" => dhruv_vedic_base::dasha::DashaSystem::Shodsottari,
        "dwadashottari" => dhruv_vedic_base::dasha::DashaSystem::Dwadashottari,
        "panchottari" => dhruv_vedic_base::dasha::DashaSystem::Panchottari,
        "shatabdika" => dhruv_vedic_base::dasha::DashaSystem::Shatabdika,
        "chaturashiti" => dhruv_vedic_base::dasha::DashaSystem::Chaturashiti,
        "dwisaptati-sama" => dhruv_vedic_base::dasha::DashaSystem::DwisaptatiSama,
        "shashtihayani" => dhruv_vedic_base::dasha::DashaSystem::Shashtihayani,
        "shat-trimsha-sama" => dhruv_vedic_base::dasha::DashaSystem::ShatTrimshaSama,
        "yogini" => dhruv_vedic_base::dasha::DashaSystem::Yogini,
        "chara" => dhruv_vedic_base::dasha::DashaSystem::Chara,
        "sthira" => dhruv_vedic_base::dasha::DashaSystem::Sthira,
        "yogardha" => dhruv_vedic_base::dasha::DashaSystem::Yogardha,
        "driga" => dhruv_vedic_base::dasha::DashaSystem::Driga,
        "shoola" => dhruv_vedic_base::dasha::DashaSystem::Shoola,
        "mandooka" => dhruv_vedic_base::dasha::DashaSystem::Mandooka,
        "chakra" => dhruv_vedic_base::dasha::DashaSystem::Chakra,
        "kala" => dhruv_vedic_base::dasha::DashaSystem::Kala,
        "kaal-chakra" => dhruv_vedic_base::dasha::DashaSystem::KaalChakra,
        "kendradi" => dhruv_vedic_base::dasha::DashaSystem::Kendradi,
        "karaka-kendradi" => dhruv_vedic_base::dasha::DashaSystem::KarakaKendradi,
        "karaka-kendradi-graha" => dhruv_vedic_base::dasha::DashaSystem::KarakaKendradiGraha,
        other => {
            eprintln!("Unknown dasha system: {other}");
            eprintln!("Valid: vimshottari, ashtottari, shodsottari, dwadashottari, panchottari,");
            eprintln!("       shatabdika, chaturashiti, dwisaptati-sama, shashtihayani,");
            eprintln!("       shat-trimsha-sama, yogini, chara, sthira, yogardha, driga,");
            eprintln!("       shoola, mandooka, chakra, kala, kaal-chakra, kendradi,");
            eprintln!("       karaka-kendradi, karaka-kendradi-graha");
            std::process::exit(1);
        }
    }
}

fn parse_dasha_entity_spec(spec: &str) -> dhruv_vedic_base::dasha::DashaEntity {
    let (kind, value) = spec.split_once(':').unwrap_or_else(|| {
        eprintln!("Invalid dasha entity: {spec}");
        eprintln!("Expected: graha:0..8, rashi:0..11, yogini:0..7");
        std::process::exit(1);
    });
    let idx: u8 = value.parse().unwrap_or_else(|_| {
        eprintln!("Invalid dasha entity index: {value}");
        std::process::exit(1);
    });
    match kind.to_ascii_lowercase().as_str() {
        "graha" => match idx {
            0 => dhruv_vedic_base::dasha::DashaEntity::Graha(dhruv_vedic_base::Graha::Surya),
            1 => dhruv_vedic_base::dasha::DashaEntity::Graha(dhruv_vedic_base::Graha::Chandra),
            2 => dhruv_vedic_base::dasha::DashaEntity::Graha(dhruv_vedic_base::Graha::Mangal),
            3 => dhruv_vedic_base::dasha::DashaEntity::Graha(dhruv_vedic_base::Graha::Buddh),
            4 => dhruv_vedic_base::dasha::DashaEntity::Graha(dhruv_vedic_base::Graha::Guru),
            5 => dhruv_vedic_base::dasha::DashaEntity::Graha(dhruv_vedic_base::Graha::Shukra),
            6 => dhruv_vedic_base::dasha::DashaEntity::Graha(dhruv_vedic_base::Graha::Shani),
            7 => dhruv_vedic_base::dasha::DashaEntity::Graha(dhruv_vedic_base::Graha::Rahu),
            8 => dhruv_vedic_base::dasha::DashaEntity::Graha(dhruv_vedic_base::Graha::Ketu),
            _ => {
                eprintln!("Graha index must be 0..8");
                std::process::exit(1);
            }
        },
        "rashi" => {
            if idx >= 12 {
                eprintln!("Rashi index must be 0..11");
                std::process::exit(1);
            }
            dhruv_vedic_base::dasha::DashaEntity::Rashi(idx)
        }
        "yogini" => {
            if idx >= 8 {
                eprintln!("Yogini index must be 0..7");
                std::process::exit(1);
            }
            dhruv_vedic_base::dasha::DashaEntity::Yogini(idx)
        }
        other => {
            eprintln!("Unknown dasha entity type: {other}");
            eprintln!("Valid: graha, rashi, yogini");
            std::process::exit(1);
        }
    }
}

fn parse_charakaraka_scheme(s: &str) -> dhruv_vedic_base::CharakarakaScheme {
    match s.to_ascii_lowercase().replace('_', "-").as_str() {
        "eight" | "8" | "8-chara" | "8-charakaraka" | "jaimini-8" => {
            dhruv_vedic_base::CharakarakaScheme::Eight
        }
        "seven-no-pitri" | "7-no-pitri" | "7-planet" | "seven-planet" => {
            dhruv_vedic_base::CharakarakaScheme::SevenNoPitri
        }
        "seven-pk-merged-mk" | "7-pk-merged-mk" | "pk-merged-mk" => {
            dhruv_vedic_base::CharakarakaScheme::SevenPkMergedMk
        }
        "mixed-parashara" | "mixed" | "parashari" | "parashara" | "7-8-parashara" => {
            dhruv_vedic_base::CharakarakaScheme::MixedParashara
        }
        "jaimini" => dhruv_vedic_base::CharakarakaScheme::Eight,
        other => {
            eprintln!("Unknown charakaraka scheme: {other}");
            eprintln!(
                "Valid: eight, seven-no-pitri, seven-pk-merged-mk, mixed-parashara (aliases: 7-planet, parashari, jaimini)"
            );
            std::process::exit(1);
        }
    }
}

fn charakaraka_role_name(role: dhruv_vedic_base::CharakarakaRole) -> &'static str {
    match role {
        dhruv_vedic_base::CharakarakaRole::Atma => "Atma",
        dhruv_vedic_base::CharakarakaRole::Amatya => "Amatya",
        dhruv_vedic_base::CharakarakaRole::Bhratri => "Bhratri",
        dhruv_vedic_base::CharakarakaRole::Matri => "Matri",
        dhruv_vedic_base::CharakarakaRole::Pitri => "Pitri",
        dhruv_vedic_base::CharakarakaRole::Putra => "Putra",
        dhruv_vedic_base::CharakarakaRole::Gnati => "Gnati",
        dhruv_vedic_base::CharakarakaRole::Dara => "Dara",
        dhruv_vedic_base::CharakarakaRole::MatriPutra => "Matri/Putra",
    }
}

fn format_dasha_entity(entity: &dhruv_vedic_base::dasha::DashaEntity) -> String {
    entity.name().to_string()
}

fn print_dasha_periods(
    periods: &[dhruv_vedic_base::dasha::DashaPeriod],
    base_indent: usize,
    max_display: usize,
) {
    let display_count = periods.len().min(max_display);
    for period in &periods[..display_count] {
        let indent = "  ".repeat(base_indent + period.level as usize);
        println!(
            "{}[{}] {} {} (UTC {} - {}, JD {:.4} - {:.4}, {:.1} days)",
            indent,
            period.order,
            period.level.name(),
            format_dasha_entity(&period.entity),
            jd_utc_to_iso_string(period.start_jd),
            jd_utc_to_iso_string(period.end_jd),
            period.start_jd,
            period.end_jd,
            period.duration_days(),
        );
    }
    if periods.len() > display_count {
        println!("  ... and {} more periods", periods.len() - display_count);
    }
}

fn print_dasha_hierarchy(hierarchy: &dhruv_vedic_base::dasha::DashaHierarchy) {
    for (lvl_idx, level) in hierarchy.levels.iter().enumerate() {
        let level_name = dhruv_vedic_base::dasha::DashaLevel::from_u8(lvl_idx as u8)
            .map(|l| l.name())
            .unwrap_or("Unknown");
        println!(
            "Level {} ({}) — {} periods:",
            lvl_idx,
            level_name,
            level.len()
        );
        print_dasha_periods(level, 1, 50);
        println!();
    }
}

#[derive(Clone, Copy, Debug)]
struct AmshaTransformRow {
    amsha: dhruv_vedic_base::Amsha,
    variation_code: u8,
    longitude: f64,
    info: dhruv_vedic_base::RashiInfo,
}

fn compute_amsha_transform_rows(
    lon: f64,
    requests: &[dhruv_vedic_base::AmshaRequest],
) -> Vec<AmshaTransformRow> {
    requests
        .iter()
        .map(|req| {
            let variation_code = req.effective_variation();
            let info = dhruv_vedic_base::amsha_rashi_info(lon, req.amsha, Some(variation_code));
            let longitude = info.rashi_index as f64 * 30.0 + info.degrees_in_rashi;
            AmshaTransformRow {
                amsha: req.amsha,
                variation_code,
                longitude,
                info,
            }
        })
        .collect()
}

fn amsha_variation_name(amsha: dhruv_vedic_base::Amsha, variation_code: u8) -> &'static str {
    dhruv_vedic_base::amsha_variation_info(amsha, variation_code)
        .map(|info| info.name)
        .unwrap_or("unknown")
}

fn format_amsha_variation_label(amsha: dhruv_vedic_base::Amsha, variation_code: u8) -> String {
    if variation_code == dhruv_vedic_base::default_amsha_variation(amsha) {
        String::new()
    } else {
        format!(" ({})", amsha_variation_name(amsha, variation_code))
    }
}

fn write_amsha_transform_rows(
    w: &mut impl std::io::Write,
    rows: &[AmshaTransformRow],
    output: AmshaOutputMode,
    format: AmshaOutputFormat,
) -> std::io::Result<()> {
    match format {
        AmshaOutputFormat::Text => match output {
            AmshaOutputMode::Longitude => {
                for row in rows {
                    writeln!(
                        w,
                        "{}{}: {:.6}°",
                        row.amsha.name(),
                        format_amsha_variation_label(row.amsha, row.variation_code),
                        row.longitude
                    )?;
                }
            }
            AmshaOutputMode::Rashi => {
                for row in rows {
                    writeln!(
                        w,
                        "{}{}: {:?} {:02}°{:02}'{:05.2}\"  ({:.6}°)",
                        row.amsha.name(),
                        format_amsha_variation_label(row.amsha, row.variation_code),
                        row.info.rashi,
                        row.info.dms.degrees,
                        row.info.dms.minutes,
                        row.info.dms.seconds,
                        row.longitude,
                    )?;
                }
            }
        },
        AmshaOutputFormat::Tsv => match output {
            AmshaOutputMode::Longitude => {
                writeln!(w, "amsha\tvariation\tlongitude_deg")?;
                for row in rows {
                    writeln!(
                        w,
                        "D{}\t{}\t{:.6}",
                        row.amsha.code(),
                        row.variation_code,
                        row.longitude
                    )?;
                }
            }
            AmshaOutputMode::Rashi => {
                writeln!(
                    w,
                    "amsha\tvariation\tlongitude_deg\trashi_index\trashi\tdegrees_in_rashi\tdms_degrees\tdms_minutes\tdms_seconds"
                )?;
                for row in rows {
                    writeln!(
                        w,
                        "D{}\t{}\t{:.6}\t{}\t{}\t{:.6}\t{}\t{}\t{:.2}",
                        row.amsha.code(),
                        row.variation_code,
                        row.longitude,
                        row.info.rashi_index,
                        row.info.rashi.name(),
                        row.info.degrees_in_rashi,
                        row.info.dms.degrees,
                        row.info.dms.minutes,
                        row.info.dms.seconds,
                    )?;
                }
            }
        },
    }
    Ok(())
}

fn parse_amsha_specs(s: &str) -> Vec<dhruv_vedic_base::AmshaRequest> {
    s.split(',')
        .map(|spec| {
            let spec = spec.trim();
            let (amsha_part, var_part) = match spec.find(':') {
                Some(idx) => (&spec[..idx], Some(&spec[idx + 1..])),
                None => (spec, None),
            };
            // Parse D-number
            let d_str = amsha_part
                .strip_prefix('D')
                .or_else(|| amsha_part.strip_prefix('d'));
            let code: u16 = match d_str {
                Some(num) => num.parse().unwrap_or_else(|_| {
                    eprintln!("Invalid amsha number: {amsha_part}");
                    std::process::exit(1);
                }),
                None => {
                    eprintln!("Amsha must start with D (e.g. D9): {amsha_part}");
                    std::process::exit(1);
                }
            };
            let amsha = match dhruv_vedic_base::Amsha::from_code(code) {
                Some(a) => a,
                None => {
                    eprintln!("Unknown amsha code: D{code}");
                    std::process::exit(1);
                }
            };
            let variation = match var_part {
                None | Some("default") => None,
                Some(other) => match dhruv_vedic_base::amsha_variation_by_name(amsha, other) {
                    Some(info) if info.is_default => None,
                    Some(info) => Some(info.variation_code),
                    None => {
                        eprintln!("Unknown variation '{other}' for D{code}");
                        std::process::exit(1);
                    }
                },
            };
            match variation {
                Some(v) => dhruv_vedic_base::AmshaRequest::with_variation(amsha, v),
                None => dhruv_vedic_base::AmshaRequest::new(amsha),
            }
        })
        .collect()
}

fn parse_amsha_list(s: &str) -> Vec<dhruv_vedic_base::Amsha> {
    s.split(',')
        .map(|spec| {
            let spec = spec.trim();
            if spec.contains(':') {
                eprintln!("amsha-variations expects D-codes only (for example D2,D9)");
                std::process::exit(1);
            }
            let code: u16 = spec
                .strip_prefix('D')
                .or_else(|| spec.strip_prefix('d'))
                .unwrap_or_else(|| {
                    eprintln!("Amsha must start with D (e.g. D9): {spec}");
                    std::process::exit(1);
                })
                .parse()
                .unwrap_or_else(|_| {
                    eprintln!("Invalid amsha number: {spec}");
                    std::process::exit(1);
                });
            dhruv_vedic_base::Amsha::from_code(code).unwrap_or_else(|| {
                eprintln!("Unknown amsha code: D{code}");
                std::process::exit(1);
            })
        })
        .collect()
}

fn write_amsha_variation_catalogs(
    w: &mut impl std::io::Write,
    amshas: &[dhruv_vedic_base::Amsha],
    format: AmshaOutputFormat,
) -> std::io::Result<()> {
    match format {
        AmshaOutputFormat::Text => {
            for (index, amsha) in amshas.iter().enumerate() {
                if index > 0 {
                    writeln!(w)?;
                }
                writeln!(w, "{} (D{}):", amsha.name(), amsha.code())?;
                for info in dhruv_vedic_base::amsha_variations(*amsha) {
                    let default_suffix = if info.is_default { " [default]" } else { "" };
                    writeln!(
                        w,
                        "  {:<3} {:<20} {}{}",
                        info.variation_code, info.name, info.label, default_suffix
                    )?;
                }
            }
        }
        AmshaOutputFormat::Tsv => {
            writeln!(w, "amsha\tvariation\tname\tlabel\tis_default\tdescription")?;
            for amsha in amshas {
                for info in dhruv_vedic_base::amsha_variations(*amsha) {
                    writeln!(
                        w,
                        "D{}\t{}\t{}\t{}\t{}\t{}",
                        amsha.code(),
                        info.variation_code,
                        info.name,
                        info.label,
                        info.is_default as u8,
                        info.description
                    )?;
                }
            }
        }
    }
    Ok(())
}

fn amsha_selection_from_requests(
    requests: &[dhruv_vedic_base::AmshaRequest],
) -> dhruv_search::AmshaSelectionConfig {
    if requests.len() > dhruv_search::MAX_AMSHA_REQUESTS {
        eprintln!(
            "Too many amsha requests: {} (maximum {})",
            requests.len(),
            dhruv_search::MAX_AMSHA_REQUESTS
        );
        std::process::exit(1);
    }
    let mut selection = dhruv_search::AmshaSelectionConfig {
        count: requests.len() as u8,
        ..Default::default()
    };
    for (index, request) in requests.iter().enumerate() {
        selection.codes[index] = request.amsha.code();
        selection.variations[index] = request.effective_variation();
    }
    selection
}

fn amsha_scope(
    include_bhava_cusps: bool,
    include_arudha_padas: bool,
    include_upagrahas: bool,
    include_sphutas: bool,
    include_special_lagnas: bool,
    include_outer_planets: bool,
) -> dhruv_search::AmshaChartScope {
    dhruv_search::AmshaChartScope {
        include_bhava_cusps,
        include_arudha_padas,
        include_upagrahas,
        include_sphutas,
        include_special_lagnas,
        include_outer_planets,
    }
}

fn has_amsha_scope(scope: &dhruv_search::AmshaChartScope) -> bool {
    scope.include_bhava_cusps
        || scope.include_arudha_padas
        || scope.include_upagrahas
        || scope.include_sphutas
        || scope.include_special_lagnas
}

fn print_conjunction_event(label: &str, ev: &ConjunctionEvent) {
    println!(
        "{}: UTC {}  JD TDB {:.6}  sep: {:.6}°",
        label, ev.utc, ev.jd_tdb, ev.actual_separation_deg
    );
    println!(
        "  Body1 lon: {:.6}°  Body2 lon: {:.6}°",
        ev.body1_longitude_deg, ev.body2_longitude_deg
    );
}

fn print_chandra_grahan(label: &str, ev: &dhruv_search::grahan_types::ChandraGrahan) {
    println!(
        "{}: {:?}  mag: {:.4}  penumbral mag: {:.4}",
        label, ev.grahan_type, ev.magnitude, ev.penumbral_magnitude
    );
    println!(
        "  Greatest: UTC {}  JD TDB {:.6}",
        ev.greatest_grahan_utc, ev.greatest_grahan_jd
    );
    println!("  P1: UTC {}  JD TDB {:.6}", ev.p1_utc, ev.p1_jd);
    if let Some(u1) = ev.u1_jd {
        if let Some(u1_utc) = ev.u1_utc {
            println!("  U1: UTC {}  JD TDB {:.6}", u1_utc, u1);
        }
    }
    if let Some(u2) = ev.u2_jd {
        if let Some(u2_utc) = ev.u2_utc {
            println!("  U2: UTC {}  JD TDB {:.6}", u2_utc, u2);
        }
    }
}

fn print_surya_grahan(label: &str, ev: &dhruv_search::grahan_types::SuryaGrahan) {
    println!("{}: {:?}  mag: {:.4}", label, ev.grahan_type, ev.magnitude);
    println!(
        "  Greatest: UTC {}  JD TDB {:.6}",
        ev.greatest_grahan_utc, ev.greatest_grahan_jd
    );
    if let Some(c1) = ev.c1_jd {
        if let Some(c1_utc) = ev.c1_utc {
            println!("  C1: UTC {}  JD TDB {:.6}", c1_utc, c1);
        }
    }
    if let Some(c2) = ev.c2_jd {
        if let Some(c2_utc) = ev.c2_utc {
            println!("  C2: UTC {}  JD TDB {:.6}", c2_utc, c2);
        }
    }
    if let Some(c3) = ev.c3_jd {
        if let Some(c3_utc) = ev.c3_utc {
            println!("  C3: UTC {}  JD TDB {:.6}", c3_utc, c3);
        }
    }
    if let Some(c4) = ev.c4_jd {
        if let Some(c4_utc) = ev.c4_utc {
            println!("  C4: UTC {}  JD TDB {:.6}", c4_utc, c4);
        }
    }
}

fn print_stationary_event(label: &str, ev: &dhruv_search::stationary_types::StationaryEvent) {
    println!(
        "{}: {:?} {:?} at UTC {} (JD TDB {:.6})",
        label, ev.body, ev.station_type, ev.utc, ev.jd_tdb
    );
    println!(
        "  Longitude: {:.6}°  Latitude: {:.6}°",
        ev.longitude_deg, ev.latitude_deg
    );
}

fn print_shadbala_entry(entry: &dhruv_search::ShadbalaEntry) {
    println!("  Sthana Bala:     {:>8.2}", entry.sthana.total);
    println!("    Uchcha:        {:>8.2}", entry.sthana.uchcha);
    println!("    Saptavargaja:  {:>8.2}", entry.sthana.saptavargaja);
    println!("    Ojhayugma:     {:>8.2}", entry.sthana.ojhayugma);
    println!("    Kendradi:      {:>8.2}", entry.sthana.kendradi);
    println!("    Drekkana:      {:>8.2}", entry.sthana.drekkana);
    println!("  Dig Bala:        {:>8.2}", entry.dig);
    println!("  Kala Bala:       {:>8.2}", entry.kala.total);
    println!("    Nathonnatha:   {:>8.2}", entry.kala.nathonnatha);
    println!("    Paksha:        {:>8.2}", entry.kala.paksha);
    println!("    Tribhaga:      {:>8.2}", entry.kala.tribhaga);
    println!("    Abda:          {:>8.2}", entry.kala.abda);
    println!("    Masa:          {:>8.2}", entry.kala.masa);
    println!("    Vara:          {:>8.2}", entry.kala.vara);
    println!("    Hora:          {:>8.2}", entry.kala.hora);
    println!("    Ayana:         {:>8.2}", entry.kala.ayana);
    println!("    Yuddha:        {:>8.2}", entry.kala.yuddha);
    println!("  Cheshta Bala:    {:>8.2}", entry.cheshta);
    println!("  Naisargika Bala: {:>8.2}", entry.naisargika);
    println!("  Drik Bala:       {:>8.2}", entry.drik);
    println!("  ─────────────────────────");
    println!(
        "  Total:           {:>8.2} shashtiamsas ({:.2} rupas)",
        entry.total_shashtiamsas, entry.total_rupas
    );
    println!("  Required:        {:>8.2}", entry.required_strength);
    println!(
        "  Strong:          {}",
        if entry.is_strong { "Yes" } else { "No" }
    );
}

fn print_bhavabala_entry(entry: &dhruv_search::BhavaBalaEntry) {
    println!(
        "  Cusp:            {}",
        format_rashi_dms(entry.cusp_sidereal_lon)
    );
    println!("  House Lord:      {}", entry.lord.name());
    println!("  Bhavadhipati:    {:>8.2}", entry.bhavadhipati);
    println!("  Dig Bala:        {:>8.2}", entry.dig);
    println!("  Drishti Bala:    {:>8.2}", entry.drishti);
    println!("  Occupation:      {:>8.2}", entry.occupation_bonus);
    println!("  Rising Bonus:    {:>8.2}", entry.rising_bonus);
    println!("  ─────────────────────────");
    println!(
        "  Total:           {:>8.2} virupas ({:.2} rupas)",
        entry.total_virupas, entry.total_rupas
    );
}

fn format_deeptadi_states(entry: &dhruv_vedic_base::GrahaAvasthas) -> String {
    let names = entry.deeptadi_states.as_names();
    let count = entry.deeptadi_states.count();
    if count == 0 {
        return entry.deeptadi.name().to_string();
    }
    names[..count].join(",")
}

fn format_lajjitadi_states(entry: &dhruv_vedic_base::GrahaAvasthas) -> String {
    let names = entry.lajjitadi_states.as_names();
    let count = entry.lajjitadi_states.count();
    if count == 0 {
        return "None".to_string();
    }
    names[..count].join(",")
}

fn print_graha_avastha(entry: &dhruv_vedic_base::GrahaAvasthas) {
    println!(
        "  Baladi:     {} (strength {:.2})",
        entry.baladi.name(),
        entry.baladi.strength_factor()
    );
    println!(
        "  Jagradadi:  {} (strength {:.2})",
        entry.jagradadi.name(),
        entry.jagradadi.strength_factor()
    );
    println!(
        "  Deeptadi:   {} (primary {}, strength {:.2})",
        format_deeptadi_states(entry),
        entry.deeptadi.name(),
        entry.deeptadi.strength_factor()
    );
    if let Some(primary) = entry.lajjitadi {
        println!(
            "  Lajjitadi:  {} (primary {}, strength {:.2})",
            format_lajjitadi_states(entry),
            primary.name(),
            primary.strength_factor()
        );
    } else {
        println!("  Lajjitadi:  None");
    }
    println!("  Sayanadi:   {}", entry.sayanadi.avastha.name());
    let group_names = ["Ka", "Cha", "Ta(r)", "Ta(d)", "Pa"];
    for (i, ss) in entry.sayanadi.sub_states.iter().enumerate() {
        println!(
            "    {}-varga:  {} (strength {:.2})",
            group_names[i],
            ss.name(),
            ss.strength_factor()
        );
    }
}

fn print_max_speed_event(label: &str, ev: &dhruv_search::stationary_types::MaxSpeedEvent) {
    println!(
        "{}: {:?} {:?} at UTC {} (JD TDB {:.6})",
        label, ev.body, ev.speed_type, ev.utc, ev.jd_tdb
    );
    println!(
        "  Longitude: {:.6}°  Speed: {:.6} deg/day",
        ev.longitude_deg, ev.speed_deg_per_day
    );
}

// ---------------------------------------------------------------------------
// Kundali flag resolution and config construction helpers
// ---------------------------------------------------------------------------

struct ResolvedKundaliFlags {
    include_bhava_cusps: bool,
    include_graha: bool,
    include_bindus: bool,
    include_drishti: bool,
    include_ashtakavarga: bool,
    include_upagrahas: bool,
    include_special_lagnas: bool,
    include_amshas: bool,
    include_shadbala: bool,
    include_bhavabala: bool,
    include_vimsopaka: bool,
    include_avastha: bool,
    include_charakaraka: bool,
    include_panchang: bool,
    include_calendar: bool,
}

#[allow(clippy::too_many_arguments)]
fn resolve_kundali_flags(
    all: bool,
    include_graha: bool,
    include_bindus: bool,
    include_drishti: bool,
    include_ashtakavarga: bool,
    include_upagrahas: bool,
    include_special_lagnas: bool,
    include_amshas: bool,
    include_shadbala: bool,
    include_bhavabala: bool,
    include_vimsopaka: bool,
    include_avastha: bool,
    include_charakaraka: bool,
    include_panchang: bool,
    include_calendar: bool,
) -> ResolvedKundaliFlags {
    let any_include_flag = include_graha
        || include_bindus
        || include_drishti
        || include_ashtakavarga
        || include_upagrahas
        || include_special_lagnas
        || include_amshas
        || include_shadbala
        || include_bhavabala
        || include_vimsopaka
        || include_avastha
        || include_charakaraka
        || include_panchang
        || include_calendar;

    if all {
        ResolvedKundaliFlags {
            include_bhava_cusps: true,
            include_graha: true,
            include_bindus: true,
            include_drishti: true,
            include_ashtakavarga: true,
            include_upagrahas: true,
            include_special_lagnas: true,
            include_amshas: true,
            include_shadbala: true,
            include_bhavabala: true,
            include_vimsopaka: true,
            include_avastha: true,
            include_charakaraka: true,
            include_panchang: true,
            include_calendar: true,
        }
    } else if any_include_flag {
        ResolvedKundaliFlags {
            include_bhava_cusps: include_graha,
            include_graha,
            include_bindus,
            include_drishti,
            include_ashtakavarga,
            include_upagrahas,
            include_special_lagnas,
            include_amshas,
            include_shadbala,
            include_bhavabala,
            include_vimsopaka,
            include_avastha,
            include_charakaraka,
            include_panchang: include_panchang || include_calendar,
            include_calendar,
        }
    } else {
        // Backwards-compatible default: original 6 sections + bhava cusps
        ResolvedKundaliFlags {
            include_bhava_cusps: true,
            include_graha: true,
            include_bindus: true,
            include_drishti: true,
            include_ashtakavarga: true,
            include_upagrahas: true,
            include_special_lagnas: true,
            include_amshas: false,
            include_shadbala: false,
            include_bhavabala: false,
            include_vimsopaka: false,
            include_avastha: false,
            include_charakaraka: false,
            include_panchang: false,
            include_calendar: false,
        }
    }
}

fn shodasavarga_amsha_selection() -> dhruv_search::AmshaSelectionConfig {
    let d_numbers: [u16; 16] = [1, 2, 3, 4, 7, 9, 10, 12, 16, 20, 24, 27, 30, 40, 45, 60];
    let mut sel = dhruv_search::AmshaSelectionConfig {
        count: 16,
        ..Default::default()
    };
    for (i, &d) in d_numbers.iter().enumerate() {
        sel.codes[i] = d;
    }
    sel
}

fn build_kundali_config(
    resolved: &ResolvedKundaliFlags,
    dasha_systems: Option<&str>,
    dasha_max_level: u8,
    dasha_snapshot_time: Option<dhruv_search::DashaSnapshotTime>,
    node_policy: NodeDignityPolicy,
    charakaraka_scheme: dhruv_vedic_base::CharakarakaScheme,
    requested_amsha_selection: Option<&dhruv_search::AmshaSelectionConfig>,
    requested_amsha_scope: &dhruv_search::AmshaChartScope,
    upagraha_config: TimeUpagrahaConfig,
    include_outer_planets: bool,
) -> dhruv_search::FullKundaliConfig {
    // Compute-vs-print: force graha_positions + lagna when amshas need it
    let compute_graha = resolved.include_graha || resolved.include_amshas;
    let mut gp_config = if resolved.include_graha {
        dhruv_search::GrahaPositionsConfig {
            include_nakshatra: true,
            include_lagna: true,
            include_outer_planets,
            include_bhava: true,
        }
    } else {
        dhruv_search::GrahaPositionsConfig::default()
    };
    if resolved.include_amshas {
        gp_config.include_lagna = true;
    }

    // Dasha: controlled solely by dasha_systems presence
    let (include_dasha, dasha_config) = if let Some(sys_str) = dasha_systems {
        let mut cfg = parse_dasha_systems_config(sys_str, dasha_max_level);
        cfg.snapshot_time = dasha_snapshot_time;
        (true, cfg)
    } else {
        (false, dhruv_search::DashaSelectionConfig::default())
    };

    let force_bhava_cusps = requested_amsha_scope.include_bhava_cusps;
    let force_bindus = requested_amsha_scope.include_arudha_padas;
    let force_upagrahas = requested_amsha_scope.include_upagrahas;
    let force_sphutas = requested_amsha_scope.include_sphutas;
    let force_special_lagnas = requested_amsha_scope.include_special_lagnas;

    // Amsha: use explicit selection when present, otherwise populate Shodasavarga by default.
    let amsha_selection = if resolved.include_amshas {
        requested_amsha_selection
            .copied()
            .unwrap_or_else(shodasavarga_amsha_selection)
    } else {
        dhruv_search::AmshaSelectionConfig::default()
    };

    dhruv_search::FullKundaliConfig {
        include_bhava_cusps: resolved.include_bhava_cusps || force_bhava_cusps,
        include_graha_positions: compute_graha,
        graha_positions_config: gp_config,
        include_bindus: resolved.include_bindus || force_bindus,
        include_drishti: resolved.include_drishti,
        include_ashtakavarga: resolved.include_ashtakavarga,
        include_upagrahas: resolved.include_upagrahas || force_upagrahas,
        include_sphutas: resolved.include_bindus || force_sphutas,
        include_special_lagnas: resolved.include_special_lagnas || force_special_lagnas,
        include_amshas: resolved.include_amshas,
        amsha_selection,
        include_shadbala: resolved.include_shadbala,
        include_bhavabala: resolved.include_bhavabala,
        include_vimsopaka: resolved.include_vimsopaka,
        include_avastha: resolved.include_avastha,
        include_charakaraka: resolved.include_charakaraka,
        charakaraka_scheme,
        include_panchang: resolved.include_panchang,
        include_calendar: resolved.include_calendar,
        include_dasha,
        dasha_config,
        node_dignity_policy: node_policy,
        upagraha_config,
        bindus_config: dhruv_search::BindusConfig {
            include_nakshatra: resolved.include_bindus,
            include_bhava: resolved.include_bindus,
            upagraha_config,
        },
        drishti_config: dhruv_search::DrishtiConfig {
            include_bhava: resolved.include_drishti,
            include_lagna: resolved.include_drishti,
            include_bindus: resolved.include_drishti,
        },
        amsha_scope: *requested_amsha_scope,
    }
}

fn build_time_upagraha_config(args: &TimeUpagrahaArgs) -> TimeUpagrahaConfig {
    let mut config = TimeUpagrahaConfig::default();
    if let Some(point) = args.gulika_point {
        config.gulika_point = point.into();
    }
    if let Some(point) = args.maandi_point {
        config.maandi_point = point.into();
    }
    if let Some(point) = args.other_upagraha_point {
        config.other_point = point.into();
    }
    if let Some(planet) = args.gulika_planet {
        config.gulika_planet = planet.into();
    }
    if let Some(planet) = args.maandi_planet {
        config.maandi_planet = planet.into();
    }
    config
}

fn format_rashi_dms(sidereal_lon: f64) -> String {
    let info = rashi_from_longitude(sidereal_lon);
    let mut degs = info.dms.degrees;
    let mut mins = info.dms.minutes;
    let mut secs = info.dms.seconds;
    let mut rashi_name = info.rashi.name();

    // Carry normalization for display (seconds rounding to 60)
    if secs >= 59.5 {
        secs = 0.0;
        mins += 1;
        if mins >= 60 {
            mins = 0;
            degs += 1;
        }
    }
    // If carry pushed past 30°, show next rashi at 0°
    if degs >= 30 {
        let next = rashi_from_longitude(sidereal_lon + 0.001);
        rashi_name = next.rashi.name();
        degs = 0;
    }

    format!("{:<10} {:02}°{:02}'{:02.0}\"", rashi_name, degs, mins, secs)
}

fn write_amsha_chart(
    w: &mut impl std::io::Write,
    chart: &dhruv_search::AmshaChart,
    base_indent: &str,
) -> std::io::Result<()> {
    let graha_names = [
        "Sun", "Moon", "Mars", "Mercury", "Jupiter", "Venus", "Saturn", "Rahu", "Ketu",
    ];
    let variation_suffix =
        if chart.variation_code == dhruv_vedic_base::default_amsha_variation(chart.amsha) {
            String::new()
        } else {
            format!(
                " [{}]",
                amsha_variation_name(chart.amsha, chart.variation_code)
            )
        };

    writeln!(
        w,
        "{base_indent}{} (D{}){variation_suffix}:",
        chart.amsha.name(),
        chart.amsha.code(),
    )?;
    for (index, entry) in chart.grahas.iter().enumerate() {
        writeln!(
            w,
            "{base_indent}  {:<14} {}",
            graha_names[index],
            format_rashi_dms(entry.sidereal_longitude)
        )?;
    }
    if let Some(ref outer_planets) = chart.outer_planets {
        let names = ["Uranus", "Neptune", "Pluto"];
        writeln!(w, "{base_indent}  Outer Grahas:")?;
        for (name, entry) in names.iter().zip(outer_planets.iter()) {
            writeln!(
                w,
                "{base_indent}    {:<12} {}",
                name,
                format_rashi_dms(entry.sidereal_longitude)
            )?;
        }
    }
    writeln!(
        w,
        "{base_indent}  {:<14} {}",
        "Lagna",
        format_rashi_dms(chart.lagna.sidereal_longitude)
    )?;

    if let Some(ref cusps) = chart.bhava_cusps {
        writeln!(w, "{base_indent}  Bhava Cusps:")?;
        for (index, entry) in cusps.iter().enumerate() {
            writeln!(
                w,
                "{base_indent}    Bhava {:>2}      {}",
                index + 1,
                format_rashi_dms(entry.sidereal_longitude)
            )?;
        }
    }

    if let Some(ref cusps) = chart.rashi_bhava_cusps {
        writeln!(w, "{base_indent}  Rashi-Bhava Cusps:")?;
        for (index, entry) in cusps.iter().enumerate() {
            writeln!(
                w,
                "{base_indent}    Bhava {:>2}      {}",
                index + 1,
                format_rashi_dms(entry.sidereal_longitude)
            )?;
        }
    }

    if let Some(ref arudha_padas) = chart.arudha_padas {
        writeln!(w, "{base_indent}  Arudha Padas:")?;
        for (index, entry) in arudha_padas.iter().enumerate() {
            writeln!(
                w,
                "{base_indent}    A{:<2}          {}",
                index + 1,
                format_rashi_dms(entry.sidereal_longitude)
            )?;
        }
    }

    if let Some(ref arudha_padas) = chart.rashi_bhava_arudha_padas {
        writeln!(w, "{base_indent}  Rashi-Bhava Arudha Padas:")?;
        for (index, entry) in arudha_padas.iter().enumerate() {
            writeln!(
                w,
                "{base_indent}    A{:<2}          {}",
                index + 1,
                format_rashi_dms(entry.sidereal_longitude)
            )?;
        }
    }

    if let Some(ref upagrahas) = chart.upagrahas {
        let names = [
            "Gulika",
            "Maandi",
            "Kaala",
            "Mrityu",
            "Artha Prahara",
            "Yama Ghantaka",
            "Dhooma",
            "Vyatipata",
            "Parivesha",
            "Indra Chapa",
            "Upaketu",
        ];
        writeln!(w, "{base_indent}  Upagrahas:")?;
        for (name, entry) in names.iter().zip(upagrahas.iter()) {
            writeln!(
                w,
                "{base_indent}    {:<14} {}",
                name,
                format_rashi_dms(entry.sidereal_longitude)
            )?;
        }
    }

    if let Some(ref sphutas) = chart.sphutas {
        writeln!(w, "{base_indent}  Sphutas:")?;
        for (sphuta, entry) in dhruv_vedic_base::ALL_SPHUTAS.iter().zip(sphutas.iter()) {
            writeln!(
                w,
                "{base_indent}    {:<22} {}",
                sphuta.name(),
                format_rashi_dms(entry.sidereal_longitude)
            )?;
        }
    }

    if let Some(ref special_lagnas) = chart.special_lagnas {
        let names = [
            "Bhava Lagna",
            "Hora Lagna",
            "Ghati Lagna",
            "Vighati Lagna",
            "Varnada Lagna",
            "Sree Lagna",
            "Pranapada Lagna",
            "Indu Lagna",
        ];
        writeln!(w, "{base_indent}  Special Lagnas:")?;
        for (name, entry) in names.iter().zip(special_lagnas.iter()) {
            writeln!(
                w,
                "{base_indent}    {:<16} {}",
                name,
                format_rashi_dms(entry.sidereal_longitude)
            )?;
        }
    }

    Ok(())
}

fn print_kundali(
    w: &mut impl std::io::Write,
    result: &dhruv_search::FullKundaliResult,
    flags: &ResolvedKundaliFlags,
) -> std::io::Result<()> {
    let graha_names = [
        "Sun", "Moon", "Mars", "Mercury", "Jupiter", "Venus", "Saturn", "Rahu", "Ketu",
    ];

    if flags.include_graha
        && let Some(ref g) = result.graha_positions
    {
        writeln!(w, "Graha Positions:")?;
        for (i, entry) in g.grahas.iter().enumerate() {
            writeln!(
                w,
                "  {:<8} {}  Nakshatra: {:<12} Pada: {} Bhava: {}",
                graha_names[i],
                format_rashi_dms(entry.sidereal_longitude),
                entry.nakshatra.name(),
                entry.pada,
                entry.bhava_number,
            )?;
            if entry.rashi_bhava_number > 0 {
                writeln!(w, "           Rashi-Bhava: {}", entry.rashi_bhava_number)?;
            }
        }
        writeln!(
            w,
            "  {:<8} {}  Nakshatra: {:<12} Pada: {} Bhava: {}",
            "Lagna",
            format_rashi_dms(g.lagna.sidereal_longitude),
            g.lagna.nakshatra.name(),
            g.lagna.pada,
            g.lagna.bhava_number,
        )?;
        if g.lagna.rashi_bhava_number > 0 {
            writeln!(w, "           Rashi-Bhava: {}", g.lagna.rashi_bhava_number)?;
        }
        if g.outer_planets
            .iter()
            .any(|entry| entry.sidereal_longitude != 0.0)
        {
            let outer_names = ["Uranus", "Neptune", "Pluto"];
            writeln!(w, "  Outer Grahas:")?;
            for (name, entry) in outer_names.iter().zip(g.outer_planets.iter()) {
                writeln!(
                    w,
                    "    {:<8} {}  Nakshatra: {:<12} Pada: {} Bhava: {}",
                    name,
                    format_rashi_dms(entry.sidereal_longitude),
                    entry.nakshatra.name(),
                    entry.pada,
                    entry.bhava_number,
                )?;
                if entry.rashi_bhava_number > 0 {
                    writeln!(w, "             Rashi-Bhava: {}", entry.rashi_bhava_number)?;
                }
            }
        }
        writeln!(w)?;
    }

    if flags.include_bhava_cusps
        && let Some(ref bh) = result.bhava_cusps
    {
        let aya = result.ayanamsha_deg;
        writeln!(w, "Bhava Cusps:")?;
        for b in &bh.bhavas {
            let sid = (b.cusp_deg - aya).rem_euclid(360.0);
            let nk = nakshatra_from_longitude(sid);
            writeln!(
                w,
                "  Bhava {:>2}  {}  Nakshatra: {:<12} Pada: {}",
                b.number,
                format_rashi_dms(sid),
                nk.nakshatra.name(),
                nk.pada,
            )?;
        }
        let mc_sid = (bh.mc_deg - aya).rem_euclid(360.0);
        writeln!(w, "  MC        {}", format_rashi_dms(mc_sid))?;
        writeln!(w)?;
    }

    if flags.include_bhava_cusps
        && let Some(ref bh) = result.rashi_bhava_cusps
    {
        writeln!(w, "Rashi-Bhava Cusps:")?;
        for b in &bh.bhavas {
            let nk = nakshatra_from_longitude(b.cusp_deg);
            writeln!(
                w,
                "  Bhava {:>2}  {}  Nakshatra: {:<12} Pada: {}",
                b.number,
                format_rashi_dms(b.cusp_deg),
                nk.nakshatra.name(),
                nk.pada,
            )?;
        }
        writeln!(w, "  Synthetic MC {}", format_rashi_dms(bh.mc_deg))?;
        writeln!(w)?;
    }

    if flags.include_special_lagnas
        && let Some(ref s) = result.special_lagnas
    {
        writeln!(w, "Special Lagnas:")?;
        writeln!(w, "  Bhava Lagna:     {}", format_rashi_dms(s.bhava_lagna))?;
        writeln!(w, "  Hora Lagna:      {}", format_rashi_dms(s.hora_lagna))?;
        writeln!(w, "  Ghati Lagna:     {}", format_rashi_dms(s.ghati_lagna))?;
        writeln!(
            w,
            "  Vighati Lagna:   {}",
            format_rashi_dms(s.vighati_lagna)
        )?;
        writeln!(
            w,
            "  Varnada Lagna:   {}",
            format_rashi_dms(s.varnada_lagna)
        )?;
        writeln!(w, "  Sree Lagna:      {}", format_rashi_dms(s.sree_lagna))?;
        writeln!(
            w,
            "  Pranapada Lagna: {}",
            format_rashi_dms(s.pranapada_lagna)
        )?;
        writeln!(w, "  Indu Lagna:      {}", format_rashi_dms(s.indu_lagna))?;
        writeln!(w)?;
    }

    if flags.include_bindus
        && let Some(ref b) = result.bindus
    {
        let pada_names = [
            "A1", "A2", "A3", "A4", "A5", "A6", "A7", "A8", "A9", "A10", "A11", "A12",
        ];
        writeln!(w, "Core Bindus:")?;
        for (i, entry) in b.arudha_padas.iter().enumerate() {
            writeln!(
                w,
                "  {:<6} {}",
                pada_names[i],
                format_rashi_dms(entry.sidereal_longitude)
            )?;
        }
        for (name, entry) in [
            ("Bhrigu", &b.bhrigu_bindu),
            ("Prana", &b.pranapada_lagna),
            ("Gulika", &b.gulika),
            ("Maandi", &b.maandi),
            ("Hora", &b.hora_lagna),
            ("Ghati", &b.ghati_lagna),
            ("Sree", &b.sree_lagna),
        ] {
            writeln!(
                w,
                "  {:<6} {}",
                name,
                format_rashi_dms(entry.sidereal_longitude)
            )?;
        }
        writeln!(w)?;
    }

    if flags.include_drishti
        && let Some(ref d) = result.drishti
    {
        let short_names = [
            "Sun", "Moon", "Mars", "Merc", "Jup", "Ven", "Sat", "Rahu", "Ketu",
        ];
        writeln!(w, "Graha-to-Graha Drishti (total virupa):")?;
        write!(w, "{:<8}", "From\\To")?;
        for name in &short_names {
            write!(w, "{:>8}", name)?;
        }
        writeln!(w)?;
        writeln!(w, "{}", "-".repeat(8 + 8 * 9))?;
        for (i, short_name) in short_names.iter().enumerate() {
            write!(w, "{:<8}", short_name)?;
            for j in 0..9 {
                let v = d.graha_to_graha.entries[i][j].total_virupa;
                if i == j {
                    write!(w, "{:>8}", "-")?;
                } else {
                    write!(w, "{:>8.1}", v)?;
                }
            }
            writeln!(w)?;
        }
        writeln!(w)?;
    }

    if flags.include_ashtakavarga
        && let Some(ref a) = result.ashtakavarga
    {
        writeln!(w, "Sarva Ashtakavarga totals:")?;
        write!(w, "  Total      ")?;
        for &p in &a.sav.total_points {
            write!(w, "{:>4}", p)?;
        }
        writeln!(w)?;
        write!(w, "  Trikona    ")?;
        for &p in &a.sav.after_trikona {
            write!(w, "{:>4}", p)?;
        }
        writeln!(w)?;
        write!(w, "  Ekadhipatya")?;
        for &p in &a.sav.after_ekadhipatya {
            write!(w, "{:>4}", p)?;
        }
        writeln!(w, "\n")?;
    }

    if flags.include_upagrahas
        && let Some(ref u) = result.upagrahas
    {
        writeln!(w, "Upagrahas:")?;
        for (name, lon) in [
            ("Gulika", u.gulika),
            ("Maandi", u.maandi),
            ("Kaala", u.kaala),
            ("Mrityu", u.mrityu),
            ("Artha Prahr", u.artha_prahara),
            ("Yama Ghant", u.yama_ghantaka),
            ("Dhooma", u.dhooma),
            ("Vyatipata", u.vyatipata),
            ("Parivesha", u.parivesha),
            ("Indra Chapa", u.indra_chapa),
            ("Upaketu", u.upaketu),
        ] {
            writeln!(w, "  {:<13} {}", name, format_rashi_dms(lon))?;
        }
        writeln!(w)?;
    }

    if let Some(ref sphutas) = result.sphutas {
        writeln!(w, "Sphutas:")?;
        for (i, sphuta) in dhruv_vedic_base::ALL_SPHUTAS.iter().enumerate() {
            writeln!(
                w,
                "  {:<22} {}",
                sphuta.name(),
                format_rashi_dms(sphutas.longitudes[i])
            )?;
        }
        writeln!(w)?;
    }

    if flags.include_amshas
        && let Some(ref am) = result.amshas
    {
        writeln!(w, "Amsha Charts ({} chart(s)):", am.charts.len())?;
        for chart in &am.charts {
            writeln!(w)?;
            write_amsha_chart(w, chart, "  ")?;
        }
        writeln!(w)?;
    }

    if flags.include_shadbala
        && let Some(ref sb) = result.shadbala
    {
        writeln!(w, "Shadbala:")?;
        writeln!(
            w,
            "  {:<8} {:>8} {:>6} {:>8} {:>8} {:>6} {:>6} {:>8} {:>6} {:>6}",
            "Graha", "Sthana", "Dig", "Kala", "Cheshta", "Nais", "Drik", "Total", "Reqd", "OK?"
        )?;
        writeln!(w, "  {}", "-".repeat(78))?;
        for e in &sb.entries {
            writeln!(
                w,
                "  {:<8} {:>8.2} {:>6.2} {:>8.2} {:>8.2} {:>6.2} {:>6.2} {:>8.2} {:>6.2} {:>6}",
                e.graha.name(),
                e.sthana.total,
                e.dig,
                e.kala.total,
                e.cheshta,
                e.naisargika,
                e.drik,
                e.total_shashtiamsas,
                e.required_strength,
                if e.is_strong { "Yes" } else { "No" },
            )?;
        }
        writeln!(w)?;
    }

    if flags.include_bhavabala
        && let Some(ref bb) = result.bhavabala
    {
        writeln!(w, "Bhava Bala:")?;
        writeln!(
            w,
            "  {:<6} {:<10} {:<8} {:>11} {:>8} {:>8} {:>8} {:>8} {:>8}",
            "Bhava", "Rashi", "Lord", "Bhavadhip", "Dig", "Drishti", "Occup", "Rise", "Total"
        )?;
        writeln!(w, "  {}", "-".repeat(76))?;
        for e in &bb.entries {
            writeln!(
                w,
                "  {:<6} {:<10} {:<8} {:>11.2} {:>8.2} {:>8.2} {:>8.2} {:>8.2} {:>8.2}",
                e.bhava_number,
                dhruv_vedic_base::ALL_RASHIS[e.rashi_index as usize].name(),
                e.lord.name(),
                e.bhavadhipati,
                e.dig,
                e.drishti,
                e.occupation_bonus,
                e.rising_bonus,
                e.total_virupas,
            )?;
        }
        writeln!(w)?;
    }

    if flags.include_vimsopaka
        && let Some(ref vm) = result.vimsopaka
    {
        writeln!(w, "Vimsopaka Bala:")?;
        writeln!(
            w,
            "  {:<8} {:>10} {:>11} {:>10} {:>13}",
            "Graha", "Shadvarga", "Saptavarga", "Dashavarga", "Shodasavarga"
        )?;
        writeln!(w, "  {}", "-".repeat(56))?;
        for e in &vm.entries {
            writeln!(
                w,
                "  {:<8} {:>10.2} {:>11.2} {:>10.2} {:>13.2}",
                e.graha.name(),
                e.shadvarga,
                e.saptavarga,
                e.dashavarga,
                e.shodasavarga,
            )?;
        }
        writeln!(w)?;
    }

    if flags.include_avastha
        && let Some(ref av) = result.avastha
    {
        writeln!(w, "Graha Avasthas:")?;
        writeln!(
            w,
            "  {:<8} {:<12} {:<12} {:<30} {:<12} {:<12}",
            "Graha", "Baladi", "Jagradadi", "Deeptadi", "Lajjitadi", "Sayanadi"
        )?;
        writeln!(w, "  {}", "-".repeat(90))?;
        for (i, e) in av.entries.iter().enumerate() {
            writeln!(
                w,
                "  {:<8} {:<12} {:<12} {:<30} {:<12} {:<12}",
                graha_names[i],
                e.baladi.name(),
                e.jagradadi.name(),
                format_deeptadi_states(e),
                format_lajjitadi_states(e),
                e.sayanadi.avastha.name(),
            )?;
        }
        writeln!(w)?;
    }

    if flags.include_charakaraka
        && let Some(ref ck) = result.charakaraka
    {
        writeln!(w, "Charakaraka:")?;
        writeln!(
            w,
            "  {:<4} {:<14} {:<8} {:<26} {:>8}",
            "Rank", "Role", "Graha", "Longitude", "Eff°"
        )?;
        writeln!(w, "  {}", "-".repeat(70))?;
        for e in &ck.entries {
            writeln!(
                w,
                "  {:<4} {:<14} {:<8} {:<26} {:>8.3}",
                e.rank,
                charakaraka_role_name(e.role),
                e.graha.name(),
                format_rashi_dms(e.longitude_deg),
                e.effective_degrees_in_rashi
            )?;
        }
        if ck.scheme == dhruv_vedic_base::CharakarakaScheme::MixedParashara {
            writeln!(
                w,
                "  Mixed resolved to {}-karaka set",
                if ck.used_eight_karakas { 8 } else { 7 }
            )?;
        }
        writeln!(w)?;
    }

    if flags.include_panchang
        && let Some(ref p) = result.panchang
    {
        writeln!(w, "Panchang:")?;
        writeln!(
            w,
            "  Tithi:     {} (index {})",
            p.tithi.tithi.name(),
            p.tithi.tithi_index
        )?;
        writeln!(
            w,
            "    Paksha: {}  Tithi in paksha: {}",
            p.tithi.paksha.name(),
            p.tithi.tithi_in_paksha
        )?;
        writeln!(w, "    Start:  {}  End: {}", p.tithi.start, p.tithi.end)?;
        writeln!(
            w,
            "  Karana:    {} (sequence {})",
            p.karana.karana.name(),
            p.karana.karana_index
        )?;
        writeln!(w, "    Start:  {}  End: {}", p.karana.start, p.karana.end)?;
        writeln!(
            w,
            "  Yoga:      {} (index {})",
            p.yoga.yoga.name(),
            p.yoga.yoga_index
        )?;
        writeln!(w, "    Start:  {}  End: {}", p.yoga.start, p.yoga.end)?;
        writeln!(w, "  Vaar:      {}", p.vaar.vaar.name())?;
        writeln!(w, "    Start:  {}  End: {}", p.vaar.start, p.vaar.end)?;
        writeln!(
            w,
            "  Hora:      {} (position {} of 24)",
            p.hora.hora.name(),
            p.hora.hora_index
        )?;
        writeln!(w, "    Start:  {}  End: {}", p.hora.start, p.hora.end)?;
        writeln!(w, "  Ghatika:   {}/60", p.ghatika.value)?;
        writeln!(w, "    Start:  {}  End: {}", p.ghatika.start, p.ghatika.end)?;
        writeln!(
            w,
            "  Nakshatra: {} (index {}, pada {})",
            p.nakshatra.nakshatra.name(),
            p.nakshatra.nakshatra_index,
            p.nakshatra.pada
        )?;
        writeln!(
            w,
            "    Start:  {}  End: {}",
            p.nakshatra.start, p.nakshatra.end
        )?;
        if flags.include_calendar {
            if let Some(ref m) = p.masa {
                let adhika_str = if m.adhika { " (Adhika)" } else { "" };
                writeln!(w, "  Masa:      {}{}", m.masa.name(), adhika_str)?;
                writeln!(w, "    Start:  {}  End: {}", m.start, m.end)?;
            }
            if let Some(ref a) = p.ayana {
                writeln!(w, "  Ayana:     {}", a.ayana.name())?;
                writeln!(w, "    Start:  {}  End: {}", a.start, a.end)?;
            }
            if let Some(ref v) = p.varsha {
                writeln!(
                    w,
                    "  Varsha:    {} (order {} of 60)",
                    v.samvatsara.name(),
                    v.order
                )?;
                writeln!(w, "    Start:  {}  End: {}", v.start, v.end)?;
            }
        }
        writeln!(w)?;
    }

    // Dasha is always printed if present (controlled by --dasha-systems, not flags)
    if let Some(ref dasha_vec) = result.dasha {
        writeln!(w, "Dasha Hierarchies ({} system(s)):", dasha_vec.len())?;
        for hierarchy in dasha_vec {
            writeln!(
                w,
                "\n  {} ({} levels):",
                hierarchy.system.name(),
                hierarchy.levels.len()
            )?;
            for (lvl_idx, level) in hierarchy.levels.iter().enumerate() {
                let level_name = dhruv_vedic_base::dasha::DashaLevel::from_u8(lvl_idx as u8)
                    .map(|l| l.name())
                    .unwrap_or("Unknown");
                let display_count = level.len().min(20);
                for period in &level[..display_count] {
                    let indent = "    ".to_string() + &"  ".repeat(lvl_idx);
                    writeln!(
                        w,
                        "{}[{}] {} ({:.1} days)",
                        indent,
                        level_name,
                        format_dasha_entity(&period.entity),
                        period.duration_days(),
                    )?;
                }
                if level.len() > display_count {
                    let indent = "    ".to_string() + &"  ".repeat(lvl_idx);
                    writeln!(w, "{}... and {} more", indent, level.len() - display_count)?;
                }
            }
        }
        writeln!(w)?;
    }

    if let Some(ref snap_vec) = result.dasha_snapshots {
        writeln!(w, "Dasha Snapshots:")?;
        for snap in snap_vec {
            writeln!(
                w,
                "  {} at UTC {} (JD {:.4}):",
                snap.system.name(),
                jd_utc_to_iso_string(snap.query_jd),
                snap.query_jd
            )?;
            for period in &snap.periods {
                let indent = "    ".to_string() + &"  ".repeat(period.level as usize);
                writeln!(
                    w,
                    "{}{}: {} ({:.1} days)",
                    indent,
                    period.level.name(),
                    format_dasha_entity(&period.entity),
                    period.duration_days(),
                )?;
            }
        }
        writeln!(w)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_charakaraka_scheme() -> dhruv_vedic_base::CharakarakaScheme {
        dhruv_vedic_base::CharakarakaScheme::default()
    }

    fn default_bhava_behavior_args() -> BhavaBehaviorArgs {
        BhavaBehaviorArgs {
            use_rashi_bhava_for_bala_avastha: false,
            use_configured_bhava_for_bala_avastha: false,
            include_node_aspects_for_drik_bala: false,
            exclude_node_aspects_for_drik_bala: false,
            include_special_bhavabala_rules: false,
            exclude_special_bhavabala_rules: false,
            divide_guru_buddh_drishti_by_4_for_drik_bala: false,
            add_full_guru_buddh_drishti_for_drik_bala: false,
            chandra_benefic_rule: None,
            sayanadi_ghatika_rounding: None,
            include_rashi_bhava_results: false,
            no_rashi_bhava_results: false,
        }
    }

    #[test]
    fn test_bhava_config_from_cli_special_bhavabala_default_and_opt_out() {
        let default_cfg = bhava_config_from_cli(&default_bhava_behavior_args());
        assert!(default_cfg.include_special_bhavabala_rules);

        let mut args = default_bhava_behavior_args();
        args.exclude_special_bhavabala_rules = true;
        let opt_out_cfg = bhava_config_from_cli(&args);
        assert!(!opt_out_cfg.include_special_bhavabala_rules);
    }

    #[test]
    fn test_resolve_kundali_flags_default() {
        let f = resolve_kundali_flags(
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false,
        );
        assert!(f.include_bhava_cusps);
        assert!(f.include_graha);
        assert!(f.include_bindus);
        assert!(f.include_drishti);
        assert!(f.include_ashtakavarga);
        assert!(f.include_upagrahas);
        assert!(f.include_special_lagnas);
        assert!(!f.include_amshas);
        assert!(!f.include_shadbala);
        assert!(!f.include_vimsopaka);
        assert!(!f.include_avastha);
        assert!(!f.include_panchang);
        assert!(!f.include_calendar);
    }

    #[test]
    fn test_resolve_kundali_flags_all() {
        let f = resolve_kundali_flags(
            true, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false,
        );
        assert!(f.include_bhava_cusps);
        assert!(f.include_graha);
        assert!(f.include_bindus);
        assert!(f.include_amshas);
        assert!(f.include_shadbala);
        assert!(f.include_vimsopaka);
        assert!(f.include_avastha);
        assert!(f.include_panchang);
        assert!(f.include_calendar);
    }

    #[test]
    fn test_resolve_kundali_flags_graha_only() {
        let f = resolve_kundali_flags(
            false, true, false, false, false, false, false, false, false, false, false, false,
            false, false, false,
        );
        assert!(
            f.include_bhava_cusps,
            "bhava cusps follows graha in any_flag branch"
        );
        assert!(f.include_graha);
        assert!(!f.include_bindus);
        assert!(!f.include_drishti);
        assert!(!f.include_amshas);
        assert!(!f.include_panchang);
    }

    #[test]
    fn test_resolve_kundali_flags_calendar_implies_panchang() {
        let f = resolve_kundali_flags(
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, true,
        );
        assert!(f.include_panchang);
        assert!(f.include_calendar);
        assert!(!f.include_graha);
        assert!(
            !f.include_bhava_cusps,
            "bhava cusps off when only calendar selected"
        );
    }

    #[test]
    fn test_resolve_kundali_flags_panchang_only_no_bhava() {
        // When only panchang is selected, bhava cusps should be off
        // (follows include_graha which is false)
        //                   all   graha bindus drishti ashtak upagr  splgn amsha shadb vimso avast panch calen
        let f = resolve_kundali_flags(
            false, false, false, false, false, false, false, false, false, false, false, true,
            false, true, false,
        );
        assert!(f.include_panchang);
        assert!(!f.include_graha);
        assert!(!f.include_bhava_cusps);
    }

    #[test]
    fn test_shodasavarga_amsha_selection() {
        let sel = shodasavarga_amsha_selection();
        assert_eq!(sel.count, 16);
        let expected: [u16; 16] = [1, 2, 3, 4, 7, 9, 10, 12, 16, 20, 24, 27, 30, 40, 45, 60];
        for i in 0..16 {
            assert_eq!(sel.codes[i], expected[i]);
            assert_eq!(sel.variations[i], 0);
        }
    }

    #[test]
    fn test_build_kundali_config_defaults_with_dasha() {
        let resolved = resolve_kundali_flags(
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false,
        );
        let cfg = build_kundali_config(
            &resolved,
            Some("vimshottari"),
            2,
            None,
            NodeDignityPolicy::default(),
            default_charakaraka_scheme(),
            None,
            &dhruv_search::AmshaChartScope::default(),
            TimeUpagrahaConfig::default(),
            true,
        );
        assert!(cfg.include_bhava_cusps);
        assert!(cfg.include_graha_positions);
        assert!(cfg.include_bindus);
        assert!(cfg.include_drishti);
        assert!(cfg.include_ashtakavarga);
        assert!(cfg.include_upagrahas);
        assert!(cfg.include_special_lagnas);
        assert!(cfg.include_dasha);
        assert!(cfg.dasha_config.count > 0);
        assert!(!cfg.include_amshas);
        assert!(!cfg.include_shadbala);
        assert!(!cfg.include_panchang);
    }

    #[test]
    fn test_build_kundali_config_bhava_cusps_off_when_panchang_only() {
        //                   all   graha bindus drishti ashtak upagr  splgn amsha shadb vimso avast panch calen
        let resolved = resolve_kundali_flags(
            false, false, false, false, false, false, false, false, false, false, false, true,
            false, true, false,
        );
        let cfg = build_kundali_config(
            &resolved,
            None,
            2,
            None,
            NodeDignityPolicy::default(),
            default_charakaraka_scheme(),
            None,
            &dhruv_search::AmshaChartScope::default(),
            TimeUpagrahaConfig::default(),
            true,
        );
        assert!(!cfg.include_bhava_cusps);
        assert!(cfg.include_panchang);
        assert!(!cfg.include_graha_positions);
    }

    #[test]
    fn test_build_kundali_config_graha_with_dasha() {
        let resolved = resolve_kundali_flags(
            false, true, false, false, false, false, false, false, false, false, false, false,
            false, false, false,
        );
        let cfg = build_kundali_config(
            &resolved,
            Some("vimshottari"),
            2,
            None,
            NodeDignityPolicy::default(),
            default_charakaraka_scheme(),
            None,
            &dhruv_search::AmshaChartScope::default(),
            TimeUpagrahaConfig::default(),
            true,
        );
        assert!(cfg.include_graha_positions);
        assert!(!cfg.include_bindus);
        assert!(cfg.include_dasha);
    }

    #[test]
    fn test_build_kundali_config_all_with_dasha() {
        let resolved = resolve_kundali_flags(
            true, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false,
        );
        let cfg = build_kundali_config(
            &resolved,
            Some("vimshottari"),
            2,
            None,
            NodeDignityPolicy::default(),
            default_charakaraka_scheme(),
            None,
            &dhruv_search::AmshaChartScope::default(),
            TimeUpagrahaConfig::default(),
            true,
        );
        assert!(cfg.include_graha_positions);
        assert!(cfg.include_panchang);
        assert!(cfg.include_calendar);
        assert!(cfg.include_amshas);
        assert_eq!(cfg.amsha_selection.count, 16);
        assert!(cfg.include_dasha);
    }

    #[test]
    fn test_build_kundali_config_no_dasha_without_systems() {
        let resolved = resolve_kundali_flags(
            true, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false,
        );
        let cfg = build_kundali_config(
            &resolved,
            None,
            2,
            None,
            NodeDignityPolicy::default(),
            default_charakaraka_scheme(),
            None,
            &dhruv_search::AmshaChartScope::default(),
            TimeUpagrahaConfig::default(),
            true,
        );
        assert!(!cfg.include_dasha);
        assert_eq!(cfg.dasha_config.count, 0);
        assert!(cfg.include_graha_positions);
        assert!(cfg.include_amshas);
    }

    #[test]
    fn test_build_kundali_config_amshas_force_graha() {
        // --include-amshas alone (no --include-graha)
        let resolved = resolve_kundali_flags(
            false, false, false, false, false, false, false, true, false, false, false, false,
            false, false, false,
        );
        let cfg = build_kundali_config(
            &resolved,
            None,
            2,
            None,
            NodeDignityPolicy::default(),
            default_charakaraka_scheme(),
            None,
            &dhruv_search::AmshaChartScope::default(),
            TimeUpagrahaConfig::default(),
            true,
        );
        // Graha must be force-computed for amshas
        assert!(cfg.include_graha_positions);
        assert!(cfg.graha_positions_config.include_lagna);
        assert!(cfg.include_amshas);
        assert_eq!(cfg.amsha_selection.count, 16);
    }

    #[test]
    fn test_build_kundali_config_node_policy() {
        let resolved = resolve_kundali_flags(
            false, true, false, false, false, false, false, false, false, false, false, false,
            false, false, false,
        );
        let cfg = build_kundali_config(
            &resolved,
            None,
            2,
            None,
            NodeDignityPolicy::AlwaysSama,
            default_charakaraka_scheme(),
            None,
            &dhruv_search::AmshaChartScope::default(),
            TimeUpagrahaConfig::default(),
            true,
        );
        assert_eq!(cfg.node_dignity_policy, NodeDignityPolicy::AlwaysSama);
    }

    #[test]
    fn test_amsha_selection_from_requests_preserves_codes_and_variations() {
        let requests = vec![
            dhruv_vedic_base::AmshaRequest::new(dhruv_vedic_base::Amsha::D9),
            dhruv_vedic_base::AmshaRequest::with_variation(
                dhruv_vedic_base::Amsha::D2,
                dhruv_vedic_base::D2_CANCER_LEO_ONLY_VARIATION_CODE,
            ),
        ];
        let selection = amsha_selection_from_requests(&requests);
        assert_eq!(selection.count, 2);
        assert_eq!(selection.codes[0], 9);
        assert_eq!(selection.variations[0], 0);
        assert_eq!(selection.codes[1], 2);
        assert_eq!(selection.variations[1], 1);
    }

    #[test]
    fn test_build_kundali_config_uses_explicit_amsha_selection_and_scope_dependencies() {
        let resolved = resolve_kundali_flags(
            false, false, false, false, false, false, false, true, false, false, false, false,
            false, false, false,
        );
        let requests = vec![
            dhruv_vedic_base::AmshaRequest::new(dhruv_vedic_base::Amsha::D9),
            dhruv_vedic_base::AmshaRequest::new(dhruv_vedic_base::Amsha::D10),
        ];
        let selection = amsha_selection_from_requests(&requests);
        let scope = amsha_scope(true, true, true, true, true, true);
        let cfg = build_kundali_config(
            &resolved,
            None,
            2,
            None,
            NodeDignityPolicy::default(),
            default_charakaraka_scheme(),
            Some(&selection),
            &scope,
            TimeUpagrahaConfig::default(),
            true,
        );
        assert_eq!(cfg.amsha_selection.count, 2);
        assert_eq!(cfg.amsha_selection.codes[0], 9);
        assert_eq!(cfg.amsha_selection.codes[1], 10);
        assert!(cfg.amsha_scope.include_bhava_cusps);
        assert!(cfg.amsha_scope.include_arudha_padas);
        assert!(cfg.amsha_scope.include_upagrahas);
        assert!(cfg.amsha_scope.include_sphutas);
        assert!(cfg.amsha_scope.include_special_lagnas);
        assert!(cfg.include_bhava_cusps);
        assert!(cfg.include_bindus);
        assert!(cfg.include_upagrahas);
        assert!(cfg.include_sphutas);
        assert!(cfg.include_special_lagnas);
    }

    #[test]
    fn test_write_amsha_transform_rows_text_longitude() {
        let requests = vec![
            dhruv_vedic_base::AmshaRequest::new(dhruv_vedic_base::Amsha::D9),
            dhruv_vedic_base::AmshaRequest::with_variation(
                dhruv_vedic_base::Amsha::D2,
                dhruv_vedic_base::D2_CANCER_LEO_ONLY_VARIATION_CODE,
            ),
        ];
        let rows = compute_amsha_transform_rows(45.0, &requests);
        let mut out = Vec::new();
        write_amsha_transform_rows(
            &mut out,
            &rows,
            AmshaOutputMode::Longitude,
            AmshaOutputFormat::Text,
        )
        .expect("write should succeed");
        let rendered = String::from_utf8(out).expect("utf8");
        assert!(rendered.contains("Navamsha:"));
        assert!(rendered.contains("(cancer-leo-only):"));
        assert_eq!(rendered.lines().count(), 2);
        assert!(!rendered.contains("Mesha"));
    }

    #[test]
    fn test_write_amsha_transform_rows_tsv_rashi() {
        let requests = vec![dhruv_vedic_base::AmshaRequest::new(
            dhruv_vedic_base::Amsha::D9,
        )];
        let rows = compute_amsha_transform_rows(45.0, &requests);
        let mut out = Vec::new();
        write_amsha_transform_rows(
            &mut out,
            &rows,
            AmshaOutputMode::Rashi,
            AmshaOutputFormat::Tsv,
        )
        .expect("write should succeed");
        let rendered = String::from_utf8(out).expect("utf8");
        assert!(
            rendered.starts_with(
                "amsha\tvariation\tlongitude_deg\trashi_index\trashi\tdegrees_in_rashi"
            )
        );
        assert!(rendered.contains("\nD9\t0\t"));
        assert_eq!(rendered.lines().count(), 2);
    }

    #[test]
    fn test_format_rashi_dms_normal() {
        let s = format_rashi_dms(45.0);
        assert!(s.contains("Vrishabha"));
        assert!(s.contains("15°"));
        assert!(s.contains("00'"));
        assert!(s.contains("00\""));
    }

    #[test]
    fn test_format_rashi_dms_zero() {
        let s = format_rashi_dms(0.0);
        assert!(s.contains("Mesha"));
        assert!(s.contains("00°"));
    }

    #[test]
    fn test_format_rashi_dms_near_360() {
        let s = format_rashi_dms(359.9999);
        // Should not panic and should produce valid output
        assert!(!s.is_empty());
        assert!(!s.contains("30°"));
    }

    fn empty_kundali_result() -> dhruv_search::FullKundaliResult {
        dhruv_search::FullKundaliResult {
            ayanamsha_deg: 24.0,
            bhava_cusps: None,
            rashi_bhava_cusps: None,
            graha_positions: None,
            bindus: None,
            drishti: None,
            ashtakavarga: None,
            upagrahas: None,
            sphutas: None,
            special_lagnas: None,
            amshas: None,
            shadbala: None,
            bhavabala: None,
            vimsopaka: None,
            avastha: None,
            charakaraka: None,
            panchang: None,
            dasha: None,
            dasha_snapshots: None,
        }
    }

    fn make_bhava_result() -> dhruv_vedic_base::BhavaResult {
        let mut bhavas = [dhruv_vedic_base::Bhava {
            number: 1,
            cusp_deg: 0.0,
            start_deg: 0.0,
            end_deg: 30.0,
        }; 12];
        for i in 0..12 {
            bhavas[i].number = (i + 1) as u8;
            bhavas[i].cusp_deg = (i as f64) * 30.0;
            bhavas[i].start_deg = (i as f64) * 30.0;
            bhavas[i].end_deg = ((i + 1) as f64) * 30.0;
        }
        dhruv_vedic_base::BhavaResult {
            bhavas,
            lagna_deg: 0.0,
            mc_deg: 90.0,
        }
    }

    #[test]
    fn test_print_kundali_bhava_shown_when_flag_on() {
        // make_bhava_result: cusps at 0°,30°,...,330° tropical, ayanamsha=24.0
        // Cusp 1: 0° tropical → (0-24) mod 360 = 336° sidereal → Meena (330-360°) at 06°00'00"
        // Cusp 2: 30° tropical → 6° sidereal → Mesha (0-30°) at 06°00'00"
        // MC: 90° tropical → 66° sidereal → Mithuna (60-90°) at 06°00'00"
        let mut result = empty_kundali_result();
        result.bhava_cusps = Some(make_bhava_result());
        let flags = ResolvedKundaliFlags {
            include_bhava_cusps: true,
            include_graha: false,
            include_bindus: false,
            include_drishti: false,
            include_ashtakavarga: false,
            include_upagrahas: false,
            include_special_lagnas: false,
            include_amshas: false,
            include_shadbala: false,
            include_bhavabala: false,
            include_vimsopaka: false,
            include_avastha: false,
            include_charakaraka: false,
            include_panchang: false,
            include_calendar: false,
        };
        let mut buf = Vec::new();
        print_kundali(&mut buf, &result, &flags).unwrap();
        let output = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines[0], "Bhava Cusps:");

        // Cusp 1: 0° tropical − 24° aya = 336° sid → Meena 06°00'00"
        let bhava1 = lines
            .iter()
            .find(|l| l.contains("Bhava  1"))
            .expect("no Bhava 1 line");
        assert!(
            bhava1.contains("Meena") && bhava1.contains("06°00'00\""),
            "Bhava 1 should be Meena 06°00'00\", got: {bhava1}"
        );

        // Cusp 2: 30° tropical − 24° = 6° sid → Mesha 06°00'00"
        let bhava2 = lines
            .iter()
            .find(|l| l.contains("Bhava  2"))
            .expect("no Bhava 2 line");
        assert!(
            bhava2.contains("Mesha") && bhava2.contains("06°00'00\""),
            "Bhava 2 should be Mesha 06°00'00\", got: {bhava2}"
        );

        // Cusp 4: 90° tropical − 24° = 66° sid → Mithuna 06°00'00"
        let bhava4 = lines
            .iter()
            .find(|l| l.contains("Bhava  4"))
            .expect("no Bhava 4 line");
        assert!(
            bhava4.contains("Mithuna") && bhava4.contains("06°00'00\""),
            "Bhava 4 should be Mithuna 06°00'00\", got: {bhava4}"
        );

        // MC: 90° tropical − 24° = 66° sid → Mithuna 06°00'00"
        let mc_line = lines
            .iter()
            .find(|l| l.starts_with("  MC"))
            .expect("no MC line");
        assert!(
            mc_line.contains("Mithuna") && mc_line.contains("06°00'00\""),
            "MC should be Mithuna 06°00'00\", got: {mc_line}"
        );
    }

    #[test]
    fn test_print_kundali_bhava_hidden_when_flag_off() {
        let mut result = empty_kundali_result();
        result.bhava_cusps = Some(make_bhava_result());
        let flags = ResolvedKundaliFlags {
            include_bhava_cusps: false,
            include_graha: false,
            include_bindus: false,
            include_drishti: false,
            include_ashtakavarga: false,
            include_upagrahas: false,
            include_special_lagnas: false,
            include_amshas: false,
            include_shadbala: false,
            include_bhavabala: false,
            include_vimsopaka: false,
            include_avastha: false,
            include_charakaraka: false,
            include_panchang: false,
            include_calendar: false,
        };
        let mut buf = Vec::new();
        print_kundali(&mut buf, &result, &flags).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(
            !output.contains("Bhava Cusps:"),
            "should NOT print bhava cusps when flag is off"
        );
    }

    #[test]
    fn test_print_kundali_bhava_hidden_when_none() {
        let result = empty_kundali_result();
        let flags = ResolvedKundaliFlags {
            include_bhava_cusps: true,
            include_graha: false,
            include_bindus: false,
            include_drishti: false,
            include_ashtakavarga: false,
            include_upagrahas: false,
            include_special_lagnas: false,
            include_amshas: false,
            include_shadbala: false,
            include_bhavabala: false,
            include_vimsopaka: false,
            include_avastha: false,
            include_charakaraka: false,
            include_panchang: false,
            include_calendar: false,
        };
        let mut buf = Vec::new();
        print_kundali(&mut buf, &result, &flags).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(
            !output.contains("Bhava Cusps:"),
            "should NOT print when bhava_cusps is None"
        );
    }

    #[test]
    fn tropical_rejects_incompatible_flags() {
        let base = [
            "dhruv",
            "graha-positions",
            "--tropical",
            "--date",
            "2000-01-01T00:00:00Z",
            "--lat",
            "0",
            "--lon",
            "0",
            "--bsp",
            "x",
            "--lsk",
            "x",
            "--eop",
            "x",
        ];
        for flag in &["--nakshatra", "--lagna", "--outer", "--bhava"] {
            let mut args: Vec<&str> = base.to_vec();
            args.push(flag);
            let result = Cli::try_parse_from(&args);
            assert!(result.is_err(), "--tropical should conflict with {flag}");
        }
    }

    #[test]
    fn tropical_alone_parses_ok() {
        let args = [
            "dhruv",
            "graha-positions",
            "--tropical",
            "--date",
            "2000-01-01T00:00:00Z",
            "--lat",
            "0",
            "--lon",
            "0",
            "--bsp",
            "x",
            "--lsk",
            "x",
            "--eop",
            "x",
        ];
        let result = Cli::try_parse_from(&args);
        assert!(result.is_ok(), "--tropical alone should parse successfully");
    }

    #[test]
    fn delta_t_model_parser_accepts_supported_values() {
        assert_eq!(
            parse_delta_t_model("legacy-em2006"),
            DeltaTModel::LegacyEspenakMeeus2006
        );
        assert_eq!(
            parse_delta_t_model("legacy"),
            DeltaTModel::LegacyEspenakMeeus2006
        );
        assert_eq!(
            parse_delta_t_model("smh2016"),
            DeltaTModel::Smh2016WithPre720Quadratic
        );
    }

    #[test]
    fn parse_time_policy_wires_selected_delta_t_model() {
        let out = parse_time_policy(
            "hybrid-deltat",
            DeltaTModel::Smh2016WithPre720Quadratic,
            SmhFutureParabolaFamily::ConstantCMinus17p52,
            FutureDeltaTTransition::BridgeFromModernEndpoint,
            false,
            true,
            Some(0.25),
            Some(25.0),
        );
        match out {
            TimeConversionPolicy::HybridDeltaT(opts) => {
                assert!(!opts.warn_on_fallback);
                assert_eq!(opts.delta_t_model, DeltaTModel::Smh2016WithPre720Quadratic);
                assert_eq!(
                    opts.smh_future_family,
                    SmhFutureParabolaFamily::ConstantCMinus17p52
                );
                assert_eq!(
                    opts.future_delta_t_transition,
                    FutureDeltaTTransition::BridgeFromModernEndpoint
                );
                assert_eq!(opts.pre_range_dut1, 0.25);
                assert_eq!(opts.future_transition_years, 25.0);
            }
            TimeConversionPolicy::StrictLsk => panic!("expected hybrid policy"),
        }
    }

    #[test]
    fn future_delta_t_transition_parser_accepts_supported_values() {
        assert_eq!(
            parse_future_delta_t_transition("legacy-tt-utc-blend"),
            FutureDeltaTTransition::LegacyTtUtcBlend
        );
        assert_eq!(
            parse_future_delta_t_transition("bridge-modern-endpoint"),
            FutureDeltaTTransition::BridgeFromModernEndpoint
        );
    }

    #[test]
    fn smh_future_family_parser_accepts_supported_values() {
        assert_eq!(
            parse_smh_future_family("addendum2020"),
            SmhFutureParabolaFamily::Addendum2020Piecewise
        );
        assert_eq!(
            parse_smh_future_family("c-20"),
            SmhFutureParabolaFamily::ConstantCMinus20
        );
        assert_eq!(
            parse_smh_future_family("c-17.52"),
            SmhFutureParabolaFamily::ConstantCMinus17p52
        );
        assert_eq!(
            parse_smh_future_family("c-15.32"),
            SmhFutureParabolaFamily::ConstantCMinus15p32
        );
        assert_eq!(
            parse_smh_future_family("stephenson1997"),
            SmhFutureParabolaFamily::Stephenson1997
        );
        assert_eq!(
            parse_smh_future_family("swiss-stephenson1997"),
            SmhFutureParabolaFamily::Stephenson1997
        );
        assert_eq!(
            parse_smh_future_family("stephenson2016"),
            SmhFutureParabolaFamily::Stephenson2016
        );
        assert_eq!(
            parse_smh_future_family("st2016"),
            SmhFutureParabolaFamily::Stephenson2016
        );
    }
}
