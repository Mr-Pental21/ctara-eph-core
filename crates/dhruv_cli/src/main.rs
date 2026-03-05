use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use clap::{Parser, Subcommand};
use dhruv_config::{ConfigResolver, DefaultsMode, EngineConfigPatch, load_with_discovery};
use dhruv_core::{Body, Engine, EngineConfig, Frame, Observer, Query};
use dhruv_frames::{
    PrecessionModel, cartesian_to_spherical, icrf_to_ecliptic, nutation_iau2000b,
    precess_ecliptic_j2000_to_date,
};
use dhruv_search::conjunction_types::{ConjunctionConfig, ConjunctionEvent};
use dhruv_search::grahan_types::GrahanConfig;
use dhruv_search::sankranti_types::SankrantiConfig;
use dhruv_search::stationary_types::StationaryConfig;
use dhruv_search::{
    ConjunctionOperation, ConjunctionQuery, ConjunctionResult, GrahanKind, GrahanOperation,
    GrahanQuery, GrahanResult, LunarPhaseKind, LunarPhaseOperation, LunarPhaseQuery,
    LunarPhaseResult, MotionKind, MotionOperation, MotionQuery, MotionResult, NodeBackend,
    NodeOperation, PANCHANG_INCLUDE_ALL, PANCHANG_INCLUDE_ALL_CALENDAR, PANCHANG_INCLUDE_ALL_CORE,
    PANCHANG_INCLUDE_AYANA, PANCHANG_INCLUDE_GHATIKA, PANCHANG_INCLUDE_HORA,
    PANCHANG_INCLUDE_KARANA, PANCHANG_INCLUDE_MASA, PANCHANG_INCLUDE_NAKSHATRA,
    PANCHANG_INCLUDE_TITHI, PANCHANG_INCLUDE_VAAR, PANCHANG_INCLUDE_VARSHA, PANCHANG_INCLUDE_YOGA,
    PanchangOperation, SankrantiOperation, SankrantiQuery, SankrantiResult, SankrantiTarget,
    TaraOperation, TaraOutputKind, TaraResult,
};
use dhruv_tara::{EarthState, TaraAccuracy, TaraCatalog, TaraConfig, TaraId};
use dhruv_time::{
    DeltaTModel, EopKernel, FutureDeltaTTransition, LeapSecondKernel, SmhFutureParabolaFamily,
    TimeConversionOptions, TimeConversionPolicy, TimeWarning, UtcTime, calendar_to_jd,
    install_smh2016_reconstruction, jd_to_calendar, jd_to_tdb_seconds,
    parse_smh2016_reconstruction, smh2016_reconstruction_installed, tdb_seconds_to_jd,
};
use dhruv_vedic_base::BhavaConfig;
use dhruv_vedic_base::riseset_types::{GeoLocation, RiseSetConfig, RiseSetResult};
use dhruv_vedic_base::{
    ALL_GRAHAS, AyanamshaSystem, Graha, LunarNode, NodeDignityPolicy, NodeMode, Rashi,
    ayanamsha_deg, ayanamsha_deg_with_catalog, ayanamsha_mean_deg_with_catalog, ayanamsha_true_deg,
    deg_to_dms, jd_tdb_to_centuries, nakshatra_from_longitude, nakshatra_from_tropical,
    nakshatra28_from_longitude, nakshatra28_from_tropical, rashi_from_longitude,
    rashi_from_tropical,
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
    /// Optional path to SMH2016 reconstruction table.
    /// Accepted formats: `year delta_t_seconds` points or
    /// cubic segments `Ki Ki+1 a0 a1 a2 a3`.
    #[arg(long, global = true)]
    delta_t_smh_table: Option<PathBuf>,
    /// Future Delta-T transition strategy for UTC beyond LSK coverage.
    /// Values: legacy-tt-utc-blend, bridge-modern-endpoint.
    #[arg(long, global = true, default_value = "legacy-tt-utc-blend")]
    future_delta_t_transition: String,
    /// For hybrid-deltat policy: do not freeze DUT1 after EOP coverage.
    /// By default DUT1 is frozen to last known EOP value.
    #[arg(long, global = true, default_value_t = false)]
    no_freeze_future_dut1: bool,
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
    #[arg(long)]
    outer: bool,
    /// Include bhava placement
    #[arg(long)]
    bhava: bool,
    /// Output tropical (ecliptic-of-date) longitudes instead of sidereal
    #[arg(long, conflicts_with_all = ["nakshatra", "lagna", "outer", "bhava"])]
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
    /// Include amsha (divisional charts)
    #[arg(long)]
    include_amshas: bool,
    /// Include shadbala
    #[arg(long)]
    include_shadbala: bool,
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
    /// Birth UTC datetime (YYYY-MM-DDThh:mm:ssZ)
    #[arg(long)]
    birth_date: String,
    /// Query UTC datetime for snapshot mode (omit for hierarchy-only)
    #[arg(long)]
    query_date: Option<String>,
    /// Latitude in degrees (north positive)
    #[arg(long)]
    lat: f64,
    /// Longitude in degrees (east positive)
    #[arg(long)]
    lon: f64,
    /// Altitude in meters (default 0)
    #[arg(long, default_value = "0")]
    alt: f64,
    /// Maximum dasha depth (0-4, default 2)
    #[arg(long, default_value = "2")]
    max_level: u8,
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
    /// Compute Vimsopaka Bala (20-point varga dignity strength) for a date and location
    Vimsopaka(VimsopakaArgs),
    /// Compute Chara Karaka assignments for a date
    Charakaraka(CharakarakaArgs),
    /// Transform a sidereal longitude through amsha (divisional chart) mappings
    Amsha {
        /// Sidereal longitude in degrees
        #[arg(long)]
        lon: f64,
        /// Comma-separated amsha specs: D<n>[:variation], e.g. D9,D10,D2:cancer-leo-only
        #[arg(long)]
        amsha: String,
    },
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
    future_transition_years: Option<f64>,
) -> TimeConversionPolicy {
    match s {
        "strict-lsk" => TimeConversionPolicy::StrictLsk,
        "hybrid-deltat" => {
            let mut opts = TimeConversionOptions::default();
            opts.delta_t_model = delta_t_model;
            opts.smh_future_family = smh_future_family;
            opts.future_delta_t_transition = future_delta_t_transition;
            opts.freeze_future_dut1 = !no_freeze_future_dut1;
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

fn find_default_smh2016_table() -> Option<PathBuf> {
    let candidates = [
        "kernels/data/time/smh2016_reconstruction.tsv",
        "kernels/data/time/smh2016_reconstruction.txt",
        "kernels/data/time/smh2016_reconstruction.csv",
    ];
    candidates
        .iter()
        .map(PathBuf::from)
        .find(|p| p.exists() && p.is_file())
}

fn maybe_warn_smh_manifest_pending() {
    let manifest = Path::new("kernels/data/time/time_assets_manifest.json");
    let Ok(content) = std::fs::read_to_string(manifest) else {
        return;
    };
    if content.contains("\"id\": \"smh2016_reconstruction\"")
        && content.contains("\"status\": \"pending_import\"")
    {
        eprintln!(
            "Warning: SMH2016 manifest status is pending_import; provide --delta-t-smh-table or import the canonical table under kernels/data/time."
        );
    }
}

fn maybe_install_smh2016_table(path: Option<&Path>) {
    let selected = path
        .map(|p| p.to_path_buf())
        .or_else(find_default_smh2016_table);
    let Some(path) = selected else {
        maybe_warn_smh_manifest_pending();
        return;
    };

    let content = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!(
            "Failed to read SMH2016 reconstruction table '{}': {e}",
            path.display()
        );
        std::process::exit(1);
    });
    let table = parse_smh2016_reconstruction(&content).unwrap_or_else(|e| {
        eprintln!(
            "Failed to parse SMH2016 reconstruction table '{}': {e}",
            path.display()
        );
        std::process::exit(1);
    });
    if !install_smh2016_reconstruction(table) {
        eprintln!(
            "Warning: SMH2016 reconstruction table was already installed; keeping first-loaded table."
        );
    } else {
        eprintln!("Loaded SMH2016 reconstruction table: {}", path.display());
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

    maybe_install_smh2016_table(cli.delta_t_smh_table.as_deref());
    let delta_t_model = parse_delta_t_model(&cli.delta_t_model);
    let smh_future_family = parse_smh_future_family(&cli.smh_future_family);
    let future_delta_t_transition = parse_future_delta_t_transition(&cli.future_delta_t_transition);
    if delta_t_model == DeltaTModel::Smh2016WithPre720Quadratic
        && !smh2016_reconstruction_installed()
    {
        eprintln!(
            "Warning: delta-t model 'smh2016' selected but no reconstruction table is installed; \
             years -720..1961 currently fall back to legacy piecewise segments."
        );
    }
    let time_policy = parse_time_policy(
        &cli.time_policy,
        delta_t_model,
        smh_future_family,
        future_delta_t_transition,
        cli.no_freeze_future_dut1,
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
                "{} ({}) - {} deg {} min {:.1} sec ({:.6} deg in rashi)",
                info.rashi.name(),
                info.rashi.western_name(),
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
                "{} ({}) - {} deg {} min {:.1} sec ({:.6} deg in rashi)",
                info.rashi.name(),
                info.rashi.western_name(),
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
                    println!(
                        "Next Sankranti: {} ({})",
                        ev.rashi.name(),
                        ev.rashi.western_name()
                    );
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
            let graha_lons =
                dhruv_search::graha_sidereal_longitudes(&engine, jd_tdb, system, args.nutation)
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
            let bhava_config = BhavaConfig::default();
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
            match dhruv_search::panchang(&engine, &eop_kernel, &op) {
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

            let result = dhruv_search::all_upagrahas_for_date(
                &engine,
                &eop_kernel,
                &utc,
                &location,
                &rs_config,
                &config,
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
                let result = dhruv_search::graha_tropical_longitudes_with_model(
                    &engine,
                    jd_tdb,
                    prec,
                    args.nutation,
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
                let bhava_config = BhavaConfig::default();
                let prec = parse_precession_model(&args.precession);
                let aya_config = SankrantiConfig::new_with_model(system, args.nutation, prec);
                let gp_config = dhruv_search::GrahaPositionsConfig {
                    include_nakshatra: args.nakshatra,
                    include_lagna: args.lagna,
                    include_outer_planets: args.outer,
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

                if args.outer {
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
            let bhava_config = BhavaConfig::default();
            let rs_config = RiseSetConfig::default();
            let aya_config = SankrantiConfig::new(system, args.nutation);
            let bindus_config = dhruv_search::BindusConfig {
                include_nakshatra: args.nakshatra,
                include_bhava: args.bhava,
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
            let bhava_config = BhavaConfig::default();
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
            let bhava_config = BhavaConfig::default();
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

            let resolved = resolve_kundali_flags(
                args.all,
                args.include_graha,
                args.include_bindus,
                args.include_drishti,
                args.include_ashtakavarga,
                args.include_upagrahas,
                args.include_special_lagnas,
                args.include_amshas,
                args.include_shadbala,
                args.include_vimsopaka,
                args.include_avastha,
                args.include_charakaraka,
                args.include_panchang,
                args.include_calendar,
            );

            let snapshot_jd = args.dasha_snapshot_date.as_ref().map(|d| {
                let snap_utc = parse_utc(d).unwrap_or_else(|e| {
                    eprintln!("{e}");
                    std::process::exit(1);
                });
                utc_to_jd_utc(&snap_utc)
            });

            let full_config = build_kundali_config(
                &resolved,
                args.dasha_systems.as_deref(),
                args.dasha_max_level,
                snapshot_jd,
                node_dignity_policy,
                parse_charakaraka_scheme(&args.charakaraka_scheme),
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
                    println!(
                        "Previous Sankranti: {} ({})",
                        ev.rashi.name(),
                        ev.rashi.western_name()
                    );
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
                            "  {} ({}) at {}  sid: {:.6}°  trop: {:.6}°",
                            ev.rashi.name(),
                            ev.rashi.western_name(),
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
            let jd_noon =
                dhruv_vedic_base::approximate_local_noon_jd(jd_utc.floor(), location.longitude_deg);

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
            let bhava_config = BhavaConfig::default();
            let jd_utc = utc_to_jd_utc(&utc);

            let result = dhruv_vedic_base::compute_bhavas(
                &engine,
                engine.lsk(),
                &eop_kernel,
                &location,
                jd_utc,
                &bhava_config,
            )
            .unwrap_or_else(|e| {
                eprintln!("Error: {e}");
                std::process::exit(1);
            });

            println!(
                "Bhavas for {} at {:.6}°N, {:.6}°E\n",
                args.date, args.lat, args.lon
            );
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

        Commands::LagnaCompute(args) => {
            let utc = parse_utc(&args.date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&args.bsp, &args.lsk);
            let eop_kernel = load_eop(&args.eop);
            let location = GeoLocation::new(args.lat, args.lon, args.alt);
            let jd_utc = utc_to_jd_utc(&utc);

            let lagna =
                dhruv_vedic_base::lagna_longitude_rad(engine.lsk(), &eop_kernel, &location, jd_utc)
                    .unwrap_or_else(|e| {
                        eprintln!("Error: {e}");
                        std::process::exit(1);
                    });
            let mc =
                dhruv_vedic_base::mc_longitude_rad(engine.lsk(), &eop_kernel, &location, jd_utc)
                    .unwrap_or_else(|e| {
                        eprintln!("Error: {e}");
                        std::process::exit(1);
                    });
            let ramc = dhruv_vedic_base::ramc_rad(engine.lsk(), &eop_kernel, &location, jd_utc)
                .unwrap_or_else(|e| {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                });

            println!(
                "Lagna (tropical): {:.6}°",
                lagna.to_degrees().rem_euclid(360.0)
            );
            println!(
                "MC (tropical):    {:.6}°",
                mc.to_degrees().rem_euclid(360.0)
            );
            println!(
                "RAMC:             {:.6}°",
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
            let lon = dhruv_search::lunar_node(&engine, &op).unwrap_or_else(|e| {
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
                    println!("{label}: {} ({})", ev.rashi.name(), ev.rashi.western_name());
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
                            "  {} ({}) at {}  sid: {:.6}°  trop: {:.6}°",
                            ev.rashi.name(),
                            ev.rashi.western_name(),
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
            let lons =
                dhruv_search::graha_sidereal_longitudes(&engine, jd_tdb, system, args.nutation)
                    .unwrap_or_else(|e| {
                        eprintln!("Error: {e}");
                        std::process::exit(1);
                    });

            println!(
                "Graha sidereal longitudes ({:?}{}):\n",
                system,
                if args.nutation { " +nutation" } else { "" }
            );
            let graha_names = [
                "Surya", "Chandra", "Mangal", "Budha", "Guru", "Shukra", "Shani", "Rahu", "Ketu",
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
            println!("{} ({})", vaar.name(), vaar.english_name());
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
            let bhava_config = BhavaConfig::default();
            let rs_config = RiseSetConfig::default();
            let aya_config = SankrantiConfig::new(system, args.nutation);

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
                    g,
                )
                .unwrap_or_else(|e| {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                });
                println!("Shadbala for {} on {}\n", g.english_name(), args.date);
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
                    e.graha.english_name(),
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
                    g,
                )
                .unwrap_or_else(|e| {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                });
                println!("Vimsopaka for {} on {}\n", g.english_name(), args.date);
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
        Commands::Amsha { lon, amsha } => {
            let requests = parse_amsha_specs(&amsha);
            for req in &requests {
                let variation = req.effective_variation();
                let info = dhruv_vedic_base::amsha_rashi_info(lon, req.amsha, Some(variation));
                let rashi = dhruv_vedic_base::ALL_RASHIS[info.rashi_index as usize];
                let var_label = match variation {
                    dhruv_vedic_base::AmshaVariation::TraditionalParashari => "",
                    dhruv_vedic_base::AmshaVariation::HoraCancerLeoOnly => " (cancer-leo-only)",
                };
                println!(
                    "{}{}: {:?} {:02}°{:02}'{:05.2}\"  ({:.6}°)",
                    req.amsha.name(),
                    var_label,
                    rashi,
                    info.dms.degrees,
                    info.dms.minutes,
                    info.dms.seconds,
                    info.rashi_index as f64 * 30.0 + info.degrees_in_rashi,
                );
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
            let bhava_config = BhavaConfig::default();
            let rs_config = RiseSetConfig::default();
            let aya_config = SankrantiConfig::new(system, args.nutation);
            let policy = parse_node_policy(&args.node_policy);

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
                    g,
                )
                .unwrap_or_else(|e| {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                });
                println!("Avasthas for {} on {}\n", g.english_name(), args.date);
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
                    "{:<8} {:>10} {:>10} {:>10} {:>10} {:>12}",
                    "Graha", "Baladi", "Jagradadi", "Deeptadi", "Lajjitadi", "Sayanadi"
                );
                println!("{}", "-".repeat(68));
                for (i, entry) in result.entries.iter().enumerate() {
                    println!(
                        "{:<8} {:>10} {:>10} {:>10} {:>10} {:>12}",
                        graha_names[i],
                        entry.baladi.name(),
                        entry.jagradadi.name(),
                        entry.deeptadi.name(),
                        entry.lajjitadi.name(),
                        entry.sayanadi.avastha.name(),
                    );
                }
            }
        }
        Commands::Dasha(args) => {
            let aya_system = require_aya_system(args.ayanamsha);
            let birth_utc = parse_utc(&args.birth_date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&args.bsp, &args.lsk);
            let eop_kernel = load_eop(&args.eop);
            let location = GeoLocation::new(args.lat, args.lon, args.alt);
            let bhava_config = BhavaConfig::default();
            let rs_config = RiseSetConfig::default();
            let aya_config = SankrantiConfig::new(aya_system, args.nutation);
            let dasha_system = parse_dasha_system(&args.system);
            let variation = dhruv_vedic_base::dasha::DashaVariationConfig::default();
            let clamped_level = args.max_level.min(dhruv_vedic_base::dasha::MAX_DASHA_LEVEL);

            if let Some(q_date) = args.query_date {
                let query_utc = parse_utc(&q_date).unwrap_or_else(|e| {
                    eprintln!("{e}");
                    std::process::exit(1);
                });
                let snapshot = dhruv_search::dasha_snapshot_at(
                    &engine,
                    &eop_kernel,
                    &birth_utc,
                    &query_utc,
                    &location,
                    dasha_system,
                    clamped_level,
                    &bhava_config,
                    &rs_config,
                    &aya_config,
                    &variation,
                )
                .unwrap_or_else(|e| {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                });
                println!(
                    "Dasha Snapshot ({}) at {} for birth {}\n",
                    dasha_system.name(),
                    q_date,
                    args.birth_date
                );
                for period in &snapshot.periods {
                    let indent = "  ".repeat(period.level as usize);
                    println!(
                        "{}{}: {} (JD {:.4} - {:.4}, {:.1} days)",
                        indent,
                        period.level.name(),
                        format_dasha_entity(&period.entity),
                        period.start_jd,
                        period.end_jd,
                        period.duration_days(),
                    );
                }
            } else {
                let hierarchy = dhruv_search::dasha_hierarchy_for_birth(
                    &engine,
                    &eop_kernel,
                    &birth_utc,
                    &location,
                    dasha_system,
                    clamped_level,
                    &bhava_config,
                    &rs_config,
                    &aya_config,
                    &variation,
                )
                .unwrap_or_else(|e| {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                });
                println!(
                    "Dasha Hierarchy ({}) for birth {} ({} levels)\n",
                    dasha_system.name(),
                    args.birth_date,
                    hierarchy.levels.len()
                );
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
                    let display_count = level.len().min(50);
                    for period in &level[..display_count] {
                        let indent = "  ".repeat(lvl_idx + 1);
                        println!(
                            "{}[{}] {} (JD {:.4} - {:.4}, {:.1} days)",
                            indent,
                            period.order,
                            format_dasha_entity(&period.entity),
                            period.start_jd,
                            period.end_jd,
                            period.duration_days(),
                        );
                    }
                    if level.len() > display_count {
                        println!("  ... and {} more periods", level.len() - display_count);
                    }
                    println!();
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
            match dhruv_search::tara(&cat, &op_equatorial) {
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
            match dhruv_search::tara(&cat, &op_ecliptic) {
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
            match dhruv_search::tara(&cat, &op_sidereal) {
                Ok(TaraResult::Sidereal(lon)) => {
                    let rashi_info = rashi_from_longitude(lon);
                    let nak_info = nakshatra_from_longitude(lon);
                    println!("Sidereal ({:?}, nutation={}):", system, args.nutation);
                    println!("  Longitude: {:.6}°", lon);
                    println!(
                        "  Rashi:     {} ({})",
                        rashi_info.rashi.name(),
                        rashi_info.rashi.western_name()
                    );
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
    let mut systems = [0xFFu8; 8];
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
        if count >= 8 {
            eprintln!("Error: too many dasha systems (max 8)");
            std::process::exit(1);
        }
        systems[count] = code;
        count += 1;
    }

    dhruv_search::DashaSelectionConfig {
        count: count as u8,
        systems,
        max_level,
        snapshot_jd: None,
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
    match entity {
        dhruv_vedic_base::dasha::DashaEntity::Graha(g) => g.english_name().to_string(),
        dhruv_vedic_base::dasha::DashaEntity::Rashi(r) => format!("Rashi {r}"),
        dhruv_vedic_base::dasha::DashaEntity::Yogini(y) => format!("Yogini {y}"),
    }
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
                Some("cancer-leo-only") => {
                    let v = dhruv_vedic_base::AmshaVariation::HoraCancerLeoOnly;
                    if !v.is_applicable_to(amsha) {
                        eprintln!("Variation 'cancer-leo-only' not applicable to D{code}");
                        std::process::exit(1);
                    }
                    Some(v)
                }
                Some(other) => {
                    eprintln!("Unknown variation: {other}  (valid: default, cancer-leo-only)");
                    std::process::exit(1);
                }
            };
            match variation {
                Some(v) => dhruv_vedic_base::AmshaRequest::with_variation(amsha, v),
                None => dhruv_vedic_base::AmshaRequest::new(amsha),
            }
        })
        .collect()
}

fn print_conjunction_event(label: &str, ev: &ConjunctionEvent) {
    println!(
        "{}: JD TDB {:.6}  sep: {:.6}°",
        label, ev.jd_tdb, ev.actual_separation_deg
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
    println!("  Greatest: JD TDB {:.6}", ev.greatest_grahan_jd);
    println!("  P1: JD TDB {:.6}", ev.p1_jd);
    if let Some(u1) = ev.u1_jd {
        println!("  U1: JD TDB {:.6}", u1);
    }
    if let Some(u2) = ev.u2_jd {
        println!("  U2: JD TDB {:.6}", u2);
    }
}

fn print_surya_grahan(label: &str, ev: &dhruv_search::grahan_types::SuryaGrahan) {
    println!("{}: {:?}  mag: {:.4}", label, ev.grahan_type, ev.magnitude);
    println!("  Greatest: JD TDB {:.6}", ev.greatest_grahan_jd);
    if let Some(c1) = ev.c1_jd {
        println!("  C1: JD TDB {:.6}", c1);
    }
    if let Some(c2) = ev.c2_jd {
        println!("  C2: JD TDB {:.6}", c2);
    }
    if let Some(c3) = ev.c3_jd {
        println!("  C3: JD TDB {:.6}", c3);
    }
    if let Some(c4) = ev.c4_jd {
        println!("  C4: JD TDB {:.6}", c4);
    }
}

fn print_stationary_event(label: &str, ev: &dhruv_search::stationary_types::StationaryEvent) {
    println!(
        "{}: {:?} {:?} at JD TDB {:.6}",
        label, ev.body, ev.station_type, ev.jd_tdb
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
        "  Deeptadi:   {} (strength {:.2})",
        entry.deeptadi.name(),
        entry.deeptadi.strength_factor()
    );
    println!(
        "  Lajjitadi:  {} (strength {:.2})",
        entry.lajjitadi.name(),
        entry.lajjitadi.strength_factor()
    );
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
        "{}: {:?} {:?} at JD TDB {:.6}",
        label, ev.body, ev.speed_type, ev.jd_tdb
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
    dasha_snapshot_jd: Option<f64>,
    node_policy: NodeDignityPolicy,
    charakaraka_scheme: dhruv_vedic_base::CharakarakaScheme,
) -> dhruv_search::FullKundaliConfig {
    // Compute-vs-print: force graha_positions + lagna when amshas need it
    let compute_graha = resolved.include_graha || resolved.include_amshas;
    let mut gp_config = if resolved.include_graha {
        dhruv_search::GrahaPositionsConfig {
            include_nakshatra: true,
            include_lagna: true,
            include_outer_planets: false,
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
        cfg.snapshot_jd = dasha_snapshot_jd;
        (true, cfg)
    } else {
        (false, dhruv_search::DashaSelectionConfig::default())
    };

    // Amsha: populate Shodasavarga default if enabled with count==0
    let amsha_selection = if resolved.include_amshas {
        shodasavarga_amsha_selection()
    } else {
        dhruv_search::AmshaSelectionConfig::default()
    };

    dhruv_search::FullKundaliConfig {
        include_bhava_cusps: resolved.include_bhava_cusps,
        include_graha_positions: compute_graha,
        graha_positions_config: gp_config,
        include_bindus: resolved.include_bindus,
        include_drishti: resolved.include_drishti,
        include_ashtakavarga: resolved.include_ashtakavarga,
        include_upagrahas: resolved.include_upagrahas,
        include_special_lagnas: resolved.include_special_lagnas,
        include_amshas: resolved.include_amshas,
        amsha_selection,
        include_shadbala: resolved.include_shadbala,
        include_vimsopaka: resolved.include_vimsopaka,
        include_avastha: resolved.include_avastha,
        include_charakaraka: resolved.include_charakaraka,
        charakaraka_scheme,
        include_panchang: resolved.include_panchang,
        include_calendar: resolved.include_calendar,
        include_dasha,
        dasha_config,
        node_dignity_policy: node_policy,
        bindus_config: dhruv_search::BindusConfig {
            include_nakshatra: resolved.include_bindus,
            include_bhava: resolved.include_bindus,
        },
        drishti_config: dhruv_search::DrishtiConfig {
            include_bhava: resolved.include_drishti,
            include_lagna: resolved.include_drishti,
            include_bindus: resolved.include_drishti,
        },
        amsha_scope: dhruv_search::AmshaChartScope::default(),
    }
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

    if flags.include_amshas
        && let Some(ref am) = result.amshas
    {
        writeln!(w, "Amsha Charts ({} chart(s)):", am.charts.len())?;
        for chart in &am.charts {
            writeln!(w, "\n  {} (D{}):", chart.amsha.name(), chart.amsha.code())?;
            for (i, entry) in chart.grahas.iter().enumerate() {
                writeln!(
                    w,
                    "    {:<8} {}",
                    graha_names[i],
                    format_rashi_dms(entry.sidereal_longitude)
                )?;
            }
            writeln!(
                w,
                "    {:<8} {}",
                "Lagna",
                format_rashi_dms(chart.lagna.sidereal_longitude)
            )?;
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
                e.graha.english_name(),
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
                e.graha.english_name(),
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
            "  {:<8} {:<12} {:<12} {:<12} {:<12} {:<12}",
            "Graha", "Baladi", "Jagradadi", "Deeptadi", "Lajjitadi", "Sayanadi"
        )?;
        writeln!(w, "  {}", "-".repeat(72))?;
        for (i, e) in av.entries.iter().enumerate() {
            writeln!(
                w,
                "  {:<8} {:<12} {:<12} {:<12} {:<12} {:<12}",
                graha_names[i],
                e.baladi.name(),
                e.jagradadi.name(),
                e.deeptadi.name(),
                e.lajjitadi.name(),
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
                e.graha.english_name(),
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
            writeln!(w, "  {} at JD {:.4}:", snap.system.name(), snap.query_jd)?;
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

    #[test]
    fn test_resolve_kundali_flags_default() {
        let f = resolve_kundali_flags(
            false, false, false, false, false, false, false, false, false, false, false, false,
            false,
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
            false,
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
            false,
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
            true,
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
            false,
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
            false,
        );
        let cfg = build_kundali_config(
            &resolved,
            Some("vimshottari"),
            2,
            None,
            NodeDignityPolicy::default(),
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
            false,
        );
        let cfg = build_kundali_config(&resolved, None, 2, None, NodeDignityPolicy::default());
        assert!(!cfg.include_bhava_cusps);
        assert!(cfg.include_panchang);
        assert!(!cfg.include_graha_positions);
    }

    #[test]
    fn test_build_kundali_config_graha_with_dasha() {
        let resolved = resolve_kundali_flags(
            false, true, false, false, false, false, false, false, false, false, false, false,
            false,
        );
        let cfg = build_kundali_config(
            &resolved,
            Some("vimshottari"),
            2,
            None,
            NodeDignityPolicy::default(),
        );
        assert!(cfg.include_graha_positions);
        assert!(!cfg.include_bindus);
        assert!(cfg.include_dasha);
    }

    #[test]
    fn test_build_kundali_config_all_with_dasha() {
        let resolved = resolve_kundali_flags(
            true, false, false, false, false, false, false, false, false, false, false, false,
            false,
        );
        let cfg = build_kundali_config(
            &resolved,
            Some("vimshottari"),
            2,
            None,
            NodeDignityPolicy::default(),
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
            false,
        );
        let cfg = build_kundali_config(&resolved, None, 2, None, NodeDignityPolicy::default());
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
            false,
        );
        let cfg = build_kundali_config(&resolved, None, 2, None, NodeDignityPolicy::default());
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
            false,
        );
        let cfg = build_kundali_config(&resolved, None, 2, None, NodeDignityPolicy::AlwaysSama);
        assert_eq!(cfg.node_dignity_policy, NodeDignityPolicy::AlwaysSama);
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
            graha_positions: None,
            bindus: None,
            drishti: None,
            ashtakavarga: None,
            upagrahas: None,
            special_lagnas: None,
            amshas: None,
            shadbala: None,
            vimsopaka: None,
            avastha: None,
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
            include_vimsopaka: false,
            include_avastha: false,
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
            include_vimsopaka: false,
            include_avastha: false,
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
            include_vimsopaka: false,
            include_avastha: false,
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
            Some(25.0),
        );
        match out {
            TimeConversionPolicy::HybridDeltaT(opts) => {
                assert_eq!(opts.delta_t_model, DeltaTModel::Smh2016WithPre720Quadratic);
                assert_eq!(
                    opts.smh_future_family,
                    SmhFutureParabolaFamily::ConstantCMinus17p52
                );
                assert_eq!(
                    opts.future_delta_t_transition,
                    FutureDeltaTTransition::BridgeFromModernEndpoint
                );
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
