/*
 * dhruv.h -- Canonical C header for ctara-dhruv-core FFI
 *
 * SPDX-License-Identifier: MIT
 *
 * This file mirrors every #[repr(C)] struct and #[unsafe(no_mangle)]
 * function exported by dhruv_ffi_c.  Keep it in sync with lib.rs and
 * bindings/python-open/src/ctara_dhruv/_cdef.py.
 */

#ifndef DHRUV_H
#define DHRUV_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ===================================================================
 * Constants
 * =================================================================== */

/* API version */
#define DHRUV_API_VERSION       51
#define DHRUV_PATH_CAPACITY     512
#define DHRUV_MAX_SPK_PATHS     8

/* DhruvStatus (repr(i32)) */
typedef int32_t DhruvStatus;

/* Status codes */
#define DHRUV_STATUS_OK                  0
#define DHRUV_STATUS_INVALID_CONFIG      1
#define DHRUV_STATUS_INVALID_QUERY       2
#define DHRUV_STATUS_KERNEL_LOAD         3
#define DHRUV_STATUS_TIME_CONVERSION     4
#define DHRUV_STATUS_UNSUPPORTED_QUERY   5
#define DHRUV_STATUS_EPOCH_OUT_OF_RANGE  6
#define DHRUV_STATUS_NULL_POINTER        7
#define DHRUV_STATUS_EOP_LOAD            8
#define DHRUV_STATUS_EOP_OUT_OF_RANGE    9
#define DHRUV_STATUS_INVALID_LOCATION   10
#define DHRUV_STATUS_NO_CONVERGENCE     11
#define DHRUV_STATUS_INVALID_SEARCH_CONFIG 12
#define DHRUV_STATUS_INVALID_INPUT      13
#define DHRUV_STATUS_INTERNAL          255

/* DhruvReferencePlane (repr(i32)) */
#define DHRUV_REFERENCE_PLANE_ECLIPTIC   0
#define DHRUV_REFERENCE_PLANE_INVARIABLE 1

/* Precession model selector */
#define DHRUV_PRECESSION_MODEL_NEWCOMB1895 0
#define DHRUV_PRECESSION_MODEL_LIESKE1977  1
#define DHRUV_PRECESSION_MODEL_IAU2006     2
#define DHRUV_PRECESSION_MODEL_VONDRAK2011 3

/* Graha longitude selector */
#define DHRUV_GRAHA_LONGITUDE_KIND_SIDEREAL 0
#define DHRUV_GRAHA_LONGITUDE_KIND_TROPICAL 1

/* Query time selector */
#define DHRUV_QUERY_TIME_JD_TDB 0
#define DHRUV_QUERY_TIME_UTC    1

/* Query output selector */
#define DHRUV_QUERY_OUTPUT_CARTESIAN 0
#define DHRUV_QUERY_OUTPUT_SPHERICAL 1
#define DHRUV_QUERY_OUTPUT_BOTH      2

/* Time policy */
#define DHRUV_TIME_POLICY_STRICT_LSK      0
#define DHRUV_TIME_POLICY_HYBRID_DELTA_T  1

/* Delta-T model */
#define DHRUV_DELTA_T_MODEL_LEGACY_ESPENAK_MEEUS_2006       0
#define DHRUV_DELTA_T_MODEL_SMH2016_WITH_PRE720_QUADRATIC   1

/* Future Delta-T transition */
#define DHRUV_FUTURE_DELTA_T_TRANSITION_LEGACY_TT_UTC_BLEND          0
#define DHRUV_FUTURE_DELTA_T_TRANSITION_BRIDGE_FROM_MODERN_ENDPOINT  1

/* SMH future-family selector */
#define DHRUV_SMH_FUTURE_FAMILY_ADDENDUM_2020_PIECEWISE 0
#define DHRUV_SMH_FUTURE_FAMILY_CONSTANT_C_MINUS20      1
#define DHRUV_SMH_FUTURE_FAMILY_CONSTANT_C_MINUS17P52   2
#define DHRUV_SMH_FUTURE_FAMILY_CONSTANT_C_MINUS15P32   3
#define DHRUV_SMH_FUTURE_FAMILY_STEPHENSON_1997         4
#define DHRUV_SMH_FUTURE_FAMILY_STEPHENSON_2016         5

/* TT-UTC diagnostic source */
#define DHRUV_TT_UTC_SOURCE_LSK_DELTA_AT  0
#define DHRUV_TT_UTC_SOURCE_DELTA_T_MODEL 1

/* Time warning kinds */
#define DHRUV_TIME_WARNING_LSK_FUTURE_FROZEN      0
#define DHRUV_TIME_WARNING_LSK_PRE_RANGE_FALLBACK 1
#define DHRUV_TIME_WARNING_EOP_FUTURE_FROZEN      2
#define DHRUV_TIME_WARNING_EOP_PRE_RANGE_FALLBACK 3
#define DHRUV_TIME_WARNING_DELTA_T_MODEL_USED     4

/* Delta-T segment codes */
#define DHRUV_DELTA_T_SEGMENT_PRE_MINUS720_QUADRATIC  0
#define DHRUV_DELTA_T_SEGMENT_SMH2016_RECONSTRUCTION  1
#define DHRUV_DELTA_T_SEGMENT_SMH_ASYMPTOTIC_FUTURE   2
#define DHRUV_DELTA_T_SEGMENT_BEFORE_MINUS500         3
#define DHRUV_DELTA_T_SEGMENT_MINUS500_TO_500         4
#define DHRUV_DELTA_T_SEGMENT_YEAR500_TO1600          5
#define DHRUV_DELTA_T_SEGMENT_YEAR1600_TO1700         6
#define DHRUV_DELTA_T_SEGMENT_YEAR1700_TO1800         7
#define DHRUV_DELTA_T_SEGMENT_YEAR1800_TO1860         8
#define DHRUV_DELTA_T_SEGMENT_YEAR1860_TO1900         9
#define DHRUV_DELTA_T_SEGMENT_YEAR1900_TO1920         10
#define DHRUV_DELTA_T_SEGMENT_YEAR1920_TO1941         11
#define DHRUV_DELTA_T_SEGMENT_YEAR1941_TO1961         12
#define DHRUV_DELTA_T_SEGMENT_YEAR1961_TO1986         13
#define DHRUV_DELTA_T_SEGMENT_YEAR1986_TO2005         14
#define DHRUV_DELTA_T_SEGMENT_YEAR2005_TO2050         15
#define DHRUV_DELTA_T_SEGMENT_YEAR2050_TO2150         16
#define DHRUV_DELTA_T_SEGMENT_AFTER2150               17

#define DHRUV_DASHA_TIME_NONE   -1
#define DHRUV_DASHA_TIME_JD_UTC 0
#define DHRUV_DASHA_TIME_UTC    1

#define DHRUV_MAX_TIME_WARNINGS 8

/* Sun limb */
#define DHRUV_SUN_LIMB_UPPER     0
#define DHRUV_SUN_LIMB_CENTER    1
#define DHRUV_SUN_LIMB_LOWER     2

/* Rise/set result type */
#define DHRUV_RISESET_EVENT       0
#define DHRUV_RISESET_NEVER_RISES 1
#define DHRUV_RISESET_NEVER_SETS  2

/* Rise/set event codes */
#define DHRUV_EVENT_SUNRISE             0
#define DHRUV_EVENT_SUNSET              1
#define DHRUV_EVENT_CIVIL_DAWN          2
#define DHRUV_EVENT_CIVIL_DUSK          3
#define DHRUV_EVENT_NAUTICAL_DAWN       4
#define DHRUV_EVENT_NAUTICAL_DUSK       5
#define DHRUV_EVENT_ASTRONOMICAL_DAWN   6
#define DHRUV_EVENT_ASTRONOMICAL_DUSK   7

/* Ayanamsha mode */
#define DHRUV_AYANAMSHA_MODE_MEAN    0
#define DHRUV_AYANAMSHA_MODE_TRUE    1
#define DHRUV_AYANAMSHA_MODE_UNIFIED 2

/* Ayanamsha time */
#define DHRUV_AYANAMSHA_TIME_JD_TDB 0
#define DHRUV_AYANAMSHA_TIME_UTC    1

/* Bhava system codes */
#define DHRUV_BHAVA_EQUAL           0
#define DHRUV_BHAVA_SURYA_SIDDHANTA 1
#define DHRUV_BHAVA_SRIPATI         2
#define DHRUV_BHAVA_KP              3
#define DHRUV_BHAVA_KOCH            4
#define DHRUV_BHAVA_REGIOMONTANUS   5
#define DHRUV_BHAVA_CAMPANUS        6
#define DHRUV_BHAVA_AXIAL_ROTATION  7
#define DHRUV_BHAVA_TOPOCENTRIC     8
#define DHRUV_BHAVA_ALCABITUS       9

/* Bhava reference mode */
#define DHRUV_BHAVA_REF_START   0
#define DHRUV_BHAVA_REF_MIDDLE  1

/* Bhava starting point */
#define DHRUV_BHAVA_START_LAGNA  -1
#define DHRUV_BHAVA_START_CUSTOM -2

/* Lunar node codes */
#define DHRUV_NODE_RAHU  0
#define DHRUV_NODE_KETU  1

/* Lunar node mode */
#define DHRUV_NODE_MODE_MEAN 0
#define DHRUV_NODE_MODE_TRUE 1

/* Lunar node backend */
#define DHRUV_NODE_BACKEND_ANALYTIC 0
#define DHRUV_NODE_BACKEND_ENGINE   1

/* Lunar node time */
#define DHRUV_NODE_TIME_JD_TDB 0
#define DHRUV_NODE_TIME_UTC    1

/* Conjunction query mode */
#define DHRUV_CONJUNCTION_QUERY_MODE_NEXT  0
#define DHRUV_CONJUNCTION_QUERY_MODE_PREV  1
#define DHRUV_CONJUNCTION_QUERY_MODE_RANGE 2

/* Sentinel */
#define DHRUV_JD_ABSENT (-1.0)

/* Eclipse type constants */
#define DHRUV_CHANDRA_GRAHAN_PENUMBRAL 0
#define DHRUV_CHANDRA_GRAHAN_PARTIAL   1
#define DHRUV_CHANDRA_GRAHAN_TOTAL     2

#define DHRUV_SURYA_GRAHAN_PARTIAL  0
#define DHRUV_SURYA_GRAHAN_ANNULAR  1
#define DHRUV_SURYA_GRAHAN_TOTAL    2
#define DHRUV_SURYA_GRAHAN_HYBRID   3

/* Eclipse query */
#define DHRUV_GRAHAN_KIND_CHANDRA 0
#define DHRUV_GRAHAN_KIND_SURYA   1

#define DHRUV_GRAHAN_QUERY_MODE_NEXT  0
#define DHRUV_GRAHAN_QUERY_MODE_PREV  1
#define DHRUV_GRAHAN_QUERY_MODE_RANGE 2

/* Station / max-speed */
#define DHRUV_STATION_RETROGRADE 0
#define DHRUV_STATION_DIRECT     1

#define DHRUV_MAX_SPEED_DIRECT     0
#define DHRUV_MAX_SPEED_RETROGRADE 1

/* Motion query */
#define DHRUV_MOTION_KIND_STATIONARY 0
#define DHRUV_MOTION_KIND_MAX_SPEED  1

#define DHRUV_MOTION_QUERY_MODE_NEXT  0
#define DHRUV_MOTION_QUERY_MODE_PREV  1
#define DHRUV_MOTION_QUERY_MODE_RANGE 2

/* Lunar phase */
#define DHRUV_LUNAR_PHASE_NEW_MOON  0
#define DHRUV_LUNAR_PHASE_FULL_MOON 1

#define DHRUV_LUNAR_PHASE_KIND_AMAVASYA 0
#define DHRUV_LUNAR_PHASE_KIND_PURNIMA  1

#define DHRUV_LUNAR_PHASE_QUERY_MODE_NEXT  0
#define DHRUV_LUNAR_PHASE_QUERY_MODE_PREV  1
#define DHRUV_LUNAR_PHASE_QUERY_MODE_RANGE 2

/* Ayana */
#define DHRUV_AYANA_UTTARAYANA   0
#define DHRUV_AYANA_DAKSHINAYANA 1

/* Sankranti */
#define DHRUV_SANKRANTI_TARGET_ANY      0
#define DHRUV_SANKRANTI_TARGET_SPECIFIC 1

#define DHRUV_SANKRANTI_QUERY_MODE_NEXT  0
#define DHRUV_SANKRANTI_QUERY_MODE_PREV  1
#define DHRUV_SANKRANTI_QUERY_MODE_RANGE 2

/* Panchang time */
#define DHRUV_PANCHANG_TIME_JD_TDB 0
#define DHRUV_PANCHANG_TIME_UTC    1

/* Panchang include masks */
#define DHRUV_PANCHANG_INCLUDE_TITHI      (1U << 0)
#define DHRUV_PANCHANG_INCLUDE_KARANA     (1U << 1)
#define DHRUV_PANCHANG_INCLUDE_YOGA       (1U << 2)
#define DHRUV_PANCHANG_INCLUDE_VAAR       (1U << 3)
#define DHRUV_PANCHANG_INCLUDE_HORA       (1U << 4)
#define DHRUV_PANCHANG_INCLUDE_GHATIKA    (1U << 5)
#define DHRUV_PANCHANG_INCLUDE_NAKSHATRA  (1U << 6)
#define DHRUV_PANCHANG_INCLUDE_MASA       (1U << 7)
#define DHRUV_PANCHANG_INCLUDE_AYANA      (1U << 8)
#define DHRUV_PANCHANG_INCLUDE_VARSHA     (1U << 9)
#define DHRUV_PANCHANG_INCLUDE_ALL_CORE     \
    (DHRUV_PANCHANG_INCLUDE_TITHI     |     \
     DHRUV_PANCHANG_INCLUDE_KARANA    |     \
     DHRUV_PANCHANG_INCLUDE_YOGA      |     \
     DHRUV_PANCHANG_INCLUDE_VAAR      |     \
     DHRUV_PANCHANG_INCLUDE_HORA      |     \
     DHRUV_PANCHANG_INCLUDE_GHATIKA   |     \
     DHRUV_PANCHANG_INCLUDE_NAKSHATRA)
#define DHRUV_PANCHANG_INCLUDE_ALL_CALENDAR \
    (DHRUV_PANCHANG_INCLUDE_MASA  |         \
     DHRUV_PANCHANG_INCLUDE_AYANA |         \
     DHRUV_PANCHANG_INCLUDE_VARSHA)
#define DHRUV_PANCHANG_INCLUDE_ALL          \
    (DHRUV_PANCHANG_INCLUDE_ALL_CORE |      \
     DHRUV_PANCHANG_INCLUDE_ALL_CALENDAR)

/* Count constants */
#define DHRUV_GRAHA_COUNT              9
#define DHRUV_SAPTA_GRAHA_COUNT        7
#define DHRUV_SPHUTA_COUNT            16
#define DHRUV_SPECIAL_LAGNA_COUNT      8
#define DHRUV_ARUDHA_PADA_COUNT       12
#define DHRUV_UPAGRAHA_COUNT          11
#define DHRUV_ASHTAKAVARGA_GRAHA_COUNT 7
#define DHRUV_MAX_AMSHA_REQUESTS      40
#define DHRUV_MAX_DASHA_SYSTEMS       23
#define DHRUV_MAX_CHARAKARAKA_ENTRIES 8

/* Charakaraka schemes */
#define DHRUV_CHARAKARAKA_SCHEME_EIGHT             0
#define DHRUV_CHARAKARAKA_SCHEME_SEVEN_NO_PITRI    1
#define DHRUV_CHARAKARAKA_SCHEME_SEVEN_PK_MERGED_MK 2
#define DHRUV_CHARAKARAKA_SCHEME_MIXED_PARASHARA   3

/* Charakaraka role codes */
#define DHRUV_CHARAKARAKA_ROLE_ATMA         0
#define DHRUV_CHARAKARAKA_ROLE_AMATYA       1
#define DHRUV_CHARAKARAKA_ROLE_BHRATRI      2
#define DHRUV_CHARAKARAKA_ROLE_MATRI        3
#define DHRUV_CHARAKARAKA_ROLE_PITRI        4
#define DHRUV_CHARAKARAKA_ROLE_PUTRA        5
#define DHRUV_CHARAKARAKA_ROLE_GNATI        6
#define DHRUV_CHARAKARAKA_ROLE_DARA         7
#define DHRUV_CHARAKARAKA_ROLE_MATRI_PUTRA  8

/* Tara output selectors */
#define DHRUV_TARA_OUTPUT_EQUATORIAL 0
#define DHRUV_TARA_OUTPUT_ECLIPTIC   1
#define DHRUV_TARA_OUTPUT_SIDEREAL   2

/* ===================================================================
 * Opaque handles
 * =================================================================== */

typedef struct DhruvEngineHandle    DhruvEngineHandle;
typedef struct DhruvLskHandle       DhruvLskHandle;
typedef struct DhruvEopHandle       DhruvEopHandle;
typedef struct DhruvConfigHandle    DhruvConfigHandle;
typedef struct DhruvTaraCatalogHandle DhruvTaraCatalogHandle;

/* DhruvDashaHierarchyHandle is void* */
typedef void *DhruvDashaHierarchyHandle;
/* DhruvDashaPeriodListHandle is void* */
typedef void *DhruvDashaPeriodListHandle;

/* ===================================================================
 * Structs
 * =================================================================== */

typedef struct {
    uint32_t spk_path_count;
    uint8_t  spk_paths_utf8[8][512];
    uint8_t  lsk_path_utf8[512];
    uint64_t cache_capacity;
    uint8_t  strict_validation;
} DhruvEngineConfig;

typedef struct {
    int32_t target;
    int32_t observer;
    int32_t frame;
    double  epoch_tdb_jd;
} DhruvQuery;

typedef struct {
    double position_km[3];
    double velocity_km_s[3];
} DhruvStateVector;

typedef struct {
    double lon_deg;
    double lat_deg;
    double distance_km;
} DhruvSphericalCoords;

typedef struct {
    double lon_deg;
    double lat_deg;
    double distance_km;
    double lon_speed;
    double lat_speed;
    double distance_speed;
} DhruvSphericalState;

typedef struct {
    DhruvStateVector    state_vector;
    DhruvSphericalState spherical_state;
} DhruvQueryResult;

typedef struct {
    uint8_t warn_on_fallback;
    int32_t delta_t_model;
    uint8_t freeze_future_dut1;
    double  pre_range_dut1;
    int32_t future_delta_t_transition;
    double  future_transition_years;
    int32_t smh_future_family;
} DhruvTimeConversionOptions;

typedef struct {
    int32_t                   mode;
    DhruvTimeConversionOptions options;
} DhruvTimePolicy;

typedef struct {
    int32_t  year;
    uint32_t month;
    uint32_t day;
    uint32_t hour;
    uint32_t minute;
    double   second;
} DhruvUtcTime;

typedef struct {
    int32_t kind;
    double  utc_seconds;
    double  first_entry_utc_seconds;
    double  last_entry_utc_seconds;
    double  used_delta_at_seconds;
    double  mjd;
    double  first_entry_mjd;
    double  last_entry_mjd;
    double  used_dut1_seconds;
    int32_t delta_t_model;
    int32_t delta_t_segment;
} DhruvTimeWarning;

typedef struct {
    int32_t          source;
    double           tt_minus_utc_s;
    uint32_t         warning_count;
    DhruvTimeWarning warnings[DHRUV_MAX_TIME_WARNINGS];
} DhruvTimeDiagnostics;

typedef struct {
    DhruvUtcTime    utc;
    DhruvTimePolicy policy;
} DhruvUtcToTdbRequest;

typedef struct {
    double               jd_tdb;
    DhruvTimeDiagnostics diagnostics;
} DhruvUtcToTdbResult;

typedef struct {
    int32_t      target;
    int32_t      observer;
    int32_t      frame;
    int32_t      time_kind;
    double       epoch_tdb_jd;
    DhruvUtcTime utc;
    int32_t      output_mode;
} DhruvQueryRequest;

typedef struct {
    double latitude_deg;
    double longitude_deg;
    double altitude_m;
} DhruvGeoLocation;

typedef struct {
    uint8_t use_refraction;
    int32_t sun_limb;
    uint8_t altitude_correction;
} DhruvRiseSetConfig;

typedef struct {
    int32_t result_type;
    int32_t event_code;
    double  jd_tdb;
} DhruvRiseSetResult;

typedef struct {
    int32_t system_code;
    int32_t mode;
    int32_t time_kind;
    double  jd_tdb;
    DhruvUtcTime utc;
    uint8_t use_nutation;
    double  delta_psi_arcsec;
} DhruvAyanamshaComputeRequest;

typedef struct {
    uint16_t degrees;
    uint8_t  minutes;
    double   seconds;
} DhruvDms;

typedef struct {
    uint8_t  rashi_index;
    DhruvDms dms;
    double   degrees_in_rashi;
} DhruvRashiInfo;

typedef struct {
    uint8_t nakshatra_index;
    uint8_t pada;
    double  degrees_in_nakshatra;
    double  degrees_in_pada;
} DhruvNakshatraInfo;

typedef struct {
    uint8_t nakshatra_index;
    uint8_t pada;
    double  degrees_in_nakshatra;
} DhruvNakshatra28Info;

/* --- Bhava --- */

typedef struct {
    int32_t system;
    int32_t starting_point;
    double  custom_start_deg;
    int32_t reference_mode;
    int32_t output_mode;
    int32_t ayanamsha_system;
    uint8_t use_nutation;
    int32_t reference_plane;
} DhruvBhavaConfig;

typedef struct {
    uint8_t number;
    double  cusp_deg;
    double  start_deg;
    double  end_deg;
} DhruvBhava;

typedef struct {
    DhruvBhava bhavas[12];
    double lagna_deg;
    double mc_deg;
} DhruvBhavaResult;

/* --- Lunar node --- */

typedef struct {
    int32_t      node_code;
    int32_t      mode_code;
    int32_t      backend;
    int32_t      time_kind;
    double       jd_tdb;
    DhruvUtcTime utc;
} DhruvLunarNodeRequest;

/* --- Conjunction --- */

typedef struct {
    double   target_separation_deg;
    double   step_size_days;
    uint32_t max_iterations;
    double   convergence_days;
} DhruvConjunctionConfig;

typedef struct {
    int32_t body1_code;
    int32_t body2_code;
    int32_t query_mode;
    double  at_jd_tdb;
    double  start_jd_tdb;
    double  end_jd_tdb;
    DhruvConjunctionConfig config;
} DhruvConjunctionSearchRequest;

typedef struct {
    double  jd_tdb;
    double  actual_separation_deg;
    double  body1_longitude_deg;
    double  body2_longitude_deg;
    double  body1_latitude_deg;
    double  body2_latitude_deg;
    int32_t body1_code;
    int32_t body2_code;
} DhruvConjunctionEvent;

/* --- Grahan (eclipse) --- */

typedef struct {
    uint8_t include_penumbral;
    uint8_t include_peak_details;
} DhruvGrahanConfig;

typedef struct {
    int32_t grahan_kind;
    int32_t query_mode;
    double  at_jd_tdb;
    double  start_jd_tdb;
    double  end_jd_tdb;
    DhruvGrahanConfig config;
} DhruvGrahanSearchRequest;

typedef struct {
    int32_t grahan_type;
    double  magnitude;
    double  penumbral_magnitude;
    double  greatest_grahan_jd;
    double  p1_jd;
    double  u1_jd;
    double  u2_jd;
    double  u3_jd;
    double  u4_jd;
    double  p4_jd;
    double  moon_ecliptic_lat_deg;
    double  angular_separation_deg;
} DhruvChandraGrahanResult;

typedef struct {
    int32_t grahan_type;
    double  magnitude;
    double  greatest_grahan_jd;
    double  c1_jd;
    double  c2_jd;
    double  c3_jd;
    double  c4_jd;
    double  moon_ecliptic_lat_deg;
    double  angular_separation_deg;
} DhruvSuryaGrahanResult;

/* --- Stationary / max-speed --- */

typedef struct {
    double   step_size_days;
    uint32_t max_iterations;
    double   convergence_days;
    double   numerical_step_days;
} DhruvStationaryConfig;

typedef struct {
    int32_t body_code;
    int32_t motion_kind;
    int32_t query_mode;
    double  at_jd_tdb;
    double  start_jd_tdb;
    double  end_jd_tdb;
    DhruvStationaryConfig config;
} DhruvMotionSearchRequest;

typedef struct {
    double  jd_tdb;
    int32_t body_code;
    double  longitude_deg;
    double  latitude_deg;
    int32_t station_type;
} DhruvStationaryEvent;

typedef struct {
    double  jd_tdb;
    int32_t body_code;
    double  longitude_deg;
    double  latitude_deg;
    double  speed_deg_per_day;
    int32_t speed_type;
} DhruvMaxSpeedEvent;

/* --- Sankranti / Lunar phase --- */

typedef struct {
    int32_t  ayanamsha_system;
    uint8_t  use_nutation;
    int32_t  reference_plane;
    double   step_size_days;
    uint32_t max_iterations;
    double   convergence_days;
} DhruvSankrantiConfig;

typedef struct {
    DhruvUtcTime utc;
    int32_t rashi_index;
    double  sun_sidereal_longitude_deg;
    double  sun_tropical_longitude_deg;
} DhruvSankrantiEvent;

typedef struct {
    int32_t target_kind;
    int32_t query_mode;
    int32_t rashi_index;
    double  at_jd_tdb;
    double  start_jd_tdb;
    double  end_jd_tdb;
    DhruvSankrantiConfig config;
} DhruvSankrantiSearchRequest;

typedef struct {
    int32_t phase_kind;
    int32_t query_mode;
    double  at_jd_tdb;
    double  start_jd_tdb;
    double  end_jd_tdb;
} DhruvLunarPhaseSearchRequest;

typedef struct {
    DhruvUtcTime utc;
    int32_t phase;
    double  moon_longitude_deg;
    double  sun_longitude_deg;
} DhruvLunarPhaseEvent;

/* --- Panchang types --- */

typedef struct {
    int32_t      tithi_index;
    int32_t      paksha;
    int32_t      tithi_in_paksha;
    double       degrees_in_tithi;
} DhruvTithiPosition;

typedef struct {
    int32_t karana_index;
    double  degrees_in_karana;
} DhruvKaranaPosition;

typedef struct {
    int32_t yoga_index;
    double  degrees_in_yoga;
} DhruvYogaPosition;

typedef struct {
    int32_t samvatsara_index;
    int32_t cycle_position;
} DhruvSamvatsaraResult;

typedef struct {
    int32_t      tithi_index;
    int32_t      paksha;
    int32_t      tithi_in_paksha;
    DhruvUtcTime start;
    DhruvUtcTime end;
} DhruvTithiInfo;

typedef struct {
    int32_t      karana_index;
    int32_t      karana_name_index;
    DhruvUtcTime start;
    DhruvUtcTime end;
} DhruvKaranaInfo;

typedef struct {
    int32_t      yoga_index;
    DhruvUtcTime start;
    DhruvUtcTime end;
} DhruvYogaInfo;

typedef struct {
    int32_t      vaar_index;
    DhruvUtcTime start;
    DhruvUtcTime end;
} DhruvVaarInfo;

typedef struct {
    int32_t      hora_index;
    int32_t      hora_position;
    DhruvUtcTime start;
    DhruvUtcTime end;
} DhruvHoraInfo;

typedef struct {
    int32_t      value;
    DhruvUtcTime start;
    DhruvUtcTime end;
} DhruvGhatikaInfo;

typedef struct {
    int32_t      nakshatra_index;
    int32_t      pada;
    DhruvUtcTime start;
    DhruvUtcTime end;
} DhruvPanchangNakshatraInfo;

typedef struct {
    int32_t      masa_index;
    uint8_t      adhika;
    DhruvUtcTime start;
    DhruvUtcTime end;
} DhruvMasaInfo;

typedef struct {
    int32_t      ayana;
    DhruvUtcTime start;
    DhruvUtcTime end;
} DhruvAyanaInfo;

typedef struct {
    int32_t      samvatsara_index;
    int32_t      order;
    DhruvUtcTime start;
    DhruvUtcTime end;
} DhruvVarshaInfo;

typedef struct {
    int32_t          time_kind;
    double           jd_tdb;
    DhruvUtcTime     utc;
    uint32_t         include_mask;
    DhruvGeoLocation location;
    DhruvRiseSetConfig  riseset_config;
    DhruvSankrantiConfig sankranti_config;
} DhruvPanchangComputeRequest;

typedef struct {
    uint8_t                  tithi_valid;
    DhruvTithiInfo           tithi;
    uint8_t                  karana_valid;
    DhruvKaranaInfo          karana;
    uint8_t                  yoga_valid;
    DhruvYogaInfo            yoga;
    uint8_t                  vaar_valid;
    DhruvVaarInfo            vaar;
    uint8_t                  hora_valid;
    DhruvHoraInfo            hora;
    uint8_t                  ghatika_valid;
    DhruvGhatikaInfo         ghatika;
    uint8_t                  nakshatra_valid;
    DhruvPanchangNakshatraInfo nakshatra;
    uint8_t                  masa_valid;
    DhruvMasaInfo            masa;
    uint8_t                  ayana_valid;
    DhruvAyanaInfo           ayana;
    uint8_t                  varsha_valid;
    DhruvVarshaInfo          varsha;
} DhruvPanchangOperationResult;

typedef struct {
    DhruvTithiInfo              tithi;
    DhruvKaranaInfo             karana;
    DhruvYogaInfo               yoga;
    DhruvVaarInfo               vaar;
    DhruvHoraInfo               hora;
    DhruvGhatikaInfo            ghatika;
    DhruvPanchangNakshatraInfo  nakshatra;
    uint8_t                     calendar_valid;
    DhruvMasaInfo               masa;
    DhruvAyanaInfo              ayana;
    DhruvVarshaInfo             varsha;
} DhruvPanchangInfo;

/* --- UTC event variants --- */

typedef struct {
    DhruvUtcTime utc;
    double  actual_separation_deg;
    double  body1_longitude_deg;
    double  body2_longitude_deg;
    double  body1_latitude_deg;
    double  body2_latitude_deg;
    int32_t body1_code;
    int32_t body2_code;
} DhruvConjunctionEventUtc;

typedef struct {
    DhruvUtcTime utc;
    int32_t body_code;
    double  longitude_deg;
    double  latitude_deg;
    int32_t station_type;
} DhruvStationaryEventUtc;

typedef struct {
    DhruvUtcTime utc;
    int32_t body_code;
    double  longitude_deg;
    double  latitude_deg;
    double  speed_deg_per_day;
    int32_t speed_type;
} DhruvMaxSpeedEventUtc;

typedef struct {
    int32_t      result_type;
    int32_t      event_code;
    DhruvUtcTime utc;
} DhruvRiseSetResultUtc;

typedef struct {
    int32_t      grahan_type;
    double       magnitude;
    double       penumbral_magnitude;
    DhruvUtcTime greatest_grahan;
    DhruvUtcTime p1;
    DhruvUtcTime u1;
    DhruvUtcTime u2;
    DhruvUtcTime u3;
    DhruvUtcTime u4;
    DhruvUtcTime p4;
    double       moon_ecliptic_lat_deg;
    double       angular_separation_deg;
    uint8_t      u1_valid;
    uint8_t      u2_valid;
    uint8_t      u3_valid;
    uint8_t      u4_valid;
} DhruvChandraGrahanResultUtc;

typedef struct {
    int32_t      grahan_type;
    double       magnitude;
    DhruvUtcTime greatest_grahan;
    DhruvUtcTime c1;
    DhruvUtcTime c2;
    DhruvUtcTime c3;
    DhruvUtcTime c4;
    double       moon_ecliptic_lat_deg;
    double       angular_separation_deg;
    uint8_t      c1_valid;
    uint8_t      c2_valid;
    uint8_t      c3_valid;
    uint8_t      c4_valid;
} DhruvSuryaGrahanResultUtc;

/* --- Sphuta --- */

typedef struct {
    double sun;
    double moon;
    double mars;
    double jupiter;
    double venus;
    double rahu;
    double lagna;
    double eighth_lord;
    double gulika;
} DhruvSphutalInputs;

typedef struct {
    double longitudes[16];
} DhruvSphutalResult;

/* --- Special Lagnas --- */

typedef struct {
    double bhava_lagna;
    double hora_lagna;
    double ghati_lagna;
    double vighati_lagna;
    double varnada_lagna;
    double sree_lagna;
    double pranapada_lagna;
    double indu_lagna;
} DhruvSpecialLagnas;

/* --- Arudha --- */

typedef struct {
    uint8_t bhava_number;
    double  longitude_deg;
    uint8_t rashi_index;
} DhruvArudhaResult;

/* --- Upagrahas --- */

enum {
    DHRUV_UPAGRAHA_POINT_START = 0,
    DHRUV_UPAGRAHA_POINT_MIDDLE = 1,
    DHRUV_UPAGRAHA_POINT_END = 2
};

enum {
    DHRUV_GULIKA_MAANDI_PLANET_RAHU = 0,
    DHRUV_GULIKA_MAANDI_PLANET_SATURN = 1
};

typedef struct {
    int32_t gulika_point;
    int32_t maandi_point;
    int32_t other_point;
    int32_t gulika_planet;
    int32_t maandi_planet;
} DhruvTimeUpagrahaConfig;

typedef struct {
    double gulika;
    double maandi;
    double kaala;
    double mrityu;
    double artha_prahara;
    double yama_ghantaka;
    double dhooma;
    double vyatipata;
    double parivesha;
    double indra_chapa;
    double upaketu;
} DhruvAllUpagrahas;

/* --- Ashtakavarga --- */

typedef struct {
    uint8_t graha_index;
    uint8_t points[12];
    uint8_t contributors[12][8];
} DhruvBhinnaAshtakavarga;

typedef struct {
    uint8_t total_points[12];
    uint8_t after_trikona[12];
    uint8_t after_ekadhipatya[12];
} DhruvSarvaAshtakavarga;

typedef struct {
    DhruvBhinnaAshtakavarga bavs[7];
    DhruvSarvaAshtakavarga  sav;
} DhruvAshtakavargaResult;

/* --- Drishti --- */

typedef struct {
    double angular_distance;
    double base_virupa;
    double special_virupa;
    double total_virupa;
} DhruvDrishtiEntry;

typedef struct {
    DhruvDrishtiEntry entries[9][9];
} DhruvGrahaDrishtiMatrix;

typedef struct {
    uint8_t include_bhava;
    uint8_t include_lagna;
    uint8_t include_bindus;
} DhruvDrishtiConfig;

typedef struct {
    DhruvDrishtiEntry graha_to_graha[9][9];
    DhruvDrishtiEntry graha_to_bhava[9][12];
    DhruvDrishtiEntry graha_to_lagna[9];
    DhruvDrishtiEntry graha_to_bindus[9][19];
} DhruvDrishtiResult;

/* --- Graha positions --- */

typedef struct {
    uint8_t include_nakshatra;
    uint8_t include_lagna;
    uint8_t include_outer_planets;
    uint8_t include_bhava;
} DhruvGrahaPositionsConfig;

typedef struct {
    double  sidereal_longitude;
    uint8_t rashi_index;
    uint8_t nakshatra_index;
    uint8_t pada;
    uint8_t bhava_number;
} DhruvGrahaEntry;

typedef struct {
    DhruvGrahaEntry grahas[9];
    DhruvGrahaEntry lagna;
    DhruvGrahaEntry outer_planets[3];
} DhruvGrahaPositions;

/* --- Core Bindus --- */

typedef struct {
    uint8_t include_nakshatra;
    uint8_t include_bhava;
    DhruvTimeUpagrahaConfig upagraha_config;
} DhruvBindusConfig;

typedef struct {
    DhruvGrahaEntry arudha_padas[12];
    DhruvGrahaEntry bhrigu_bindu;
    DhruvGrahaEntry pranapada_lagna;
    DhruvGrahaEntry gulika;
    DhruvGrahaEntry maandi;
    DhruvGrahaEntry hora_lagna;
    DhruvGrahaEntry ghati_lagna;
    DhruvGrahaEntry sree_lagna;
} DhruvBindusResult;

/* --- Graha longitudes --- */

typedef struct {
    double longitudes[9];
} DhruvGrahaLongitudes;

typedef struct {
    int32_t kind;
    int32_t ayanamsha_system;
    uint8_t use_nutation;
    int32_t precession_model;
    int32_t reference_plane;
} DhruvGrahaLongitudesConfig;

/* --- Amsha (divisional chart) --- */

typedef struct {
    double   sidereal_longitude;
    uint8_t  rashi_index;
    uint16_t dms_degrees;
    uint8_t  dms_minutes;
    double   dms_seconds;
    double   degrees_in_rashi;
} DhruvAmshaEntry;

typedef struct {
    uint8_t include_bhava_cusps;
    uint8_t include_arudha_padas;
    uint8_t include_upagrahas;
    uint8_t include_sphutas;
    uint8_t include_special_lagnas;
} DhruvAmshaChartScope;

typedef struct {
    uint8_t  count;
    uint16_t codes[40];
    uint8_t  variations[40];
} DhruvAmshaSelectionConfig;

typedef struct {
    uint16_t        amsha_code;
    uint8_t         variation_code;
    DhruvAmshaEntry grahas[9];
    DhruvAmshaEntry lagna;
    uint8_t         bhava_cusps_valid;
    DhruvAmshaEntry bhava_cusps[12];
    uint8_t         arudha_padas_valid;
    DhruvAmshaEntry arudha_padas[12];
    uint8_t         upagrahas_valid;
    DhruvAmshaEntry upagrahas[11];
    uint8_t         sphutas_valid;
    DhruvAmshaEntry sphutas[16];
    uint8_t         special_lagnas_valid;
    DhruvAmshaEntry special_lagnas[8];
} DhruvAmshaChart;

/* --- Charakaraka --- */

typedef struct {
    uint8_t role_code;
    uint8_t graha_index;
    uint8_t rank;
    double  longitude_deg;
    double  degrees_in_rashi;
    double  effective_degrees_in_rashi;
} DhruvCharakarakaEntry;

typedef struct {
    uint8_t              scheme;
    uint8_t              used_eight_karakas;
    uint8_t              count;
    DhruvCharakarakaEntry entries[8];
} DhruvCharakarakaResult;

/* --- Shadbala & Vimsopaka --- */

typedef struct {
    double uchcha;
    double saptavargaja;
    double ojhayugma;
    double kendradi;
    double drekkana;
    double total;
} DhruvSthanaBalaBreakdown;

typedef struct {
    double nathonnatha;
    double paksha;
    double tribhaga;
    double abda;
    double masa;
    double vara;
    double hora;
    double ayana;
    double yuddha;
    double total;
} DhruvKalaBalaBreakdown;

typedef struct {
    uint8_t                  graha_index;
    DhruvSthanaBalaBreakdown sthana;
    double                   dig;
    DhruvKalaBalaBreakdown   kala;
    double                   cheshta;
    double                   naisargika;
    double                   drik;
    double                   total_shashtiamsas;
    double                   total_rupas;
    double                   required_strength;
    uint8_t                  is_strong;
} DhruvShadbalaEntry;

typedef struct {
    DhruvShadbalaEntry entries[7];
} DhruvShadbalaResult;

typedef struct {
    uint8_t bhava_number;
    double  cusp_sidereal_lon;
    uint8_t rashi_index;
    uint8_t lord_graha_index;
    double  bhavadhipati;
    double  dig;
    double  drishti;
    double  occupation_bonus;
    double  rising_bonus;
    double  total_virupas;
    double  total_rupas;
} DhruvBhavaBalaEntry;

typedef struct {
    DhruvBhavaBalaEntry entries[12];
} DhruvBhavaBalaResult;

typedef struct {
    double   cusp_sidereal_lons[12];
    double   ascendant_sidereal_lon;
    double   meridian_sidereal_lon;
    uint8_t  graha_bhava_numbers[9];
    double   house_lord_strengths[12];
    double   aspect_virupas[9][12];
    uint32_t birth_period;
} DhruvBhavaBalaInputs;

typedef struct {
    uint8_t graha_index;
    double  shadvarga;
    double  saptavarga;
    double  dashavarga;
    double  shodasavarga;
} DhruvVimsopakaEntry;

typedef struct {
    DhruvVimsopakaEntry entries[9];
} DhruvVimsopakaResult;

typedef struct {
    DhruvShadbalaResult     shadbala;
    DhruvVimsopakaResult    vimsopaka;
    DhruvAshtakavargaResult ashtakavarga;
    DhruvBhavaBalaResult    bhavabala;
} DhruvBalaBundleResult;

/* --- Avastha --- */

typedef struct {
    uint8_t avastha;
    uint8_t sub_states[5];
} DhruvSayanadiResult;

typedef struct {
    uint8_t             baladi;
    uint8_t             jagradadi;
    uint8_t             deeptadi;
    uint8_t             lajjitadi;
    DhruvSayanadiResult sayanadi;
} DhruvGrahaAvasthas;

typedef struct {
    DhruvGrahaAvasthas entries[9];
} DhruvAllGrahaAvasthas;

/* --- Dasha --- */

typedef struct {
    uint8_t  entity_type;
    uint8_t  entity_index;
    const char *entity_name;
    double   start_jd;
    double   end_jd;
    uint8_t  level;
    uint16_t order;
    uint32_t parent_idx;
} DhruvDashaPeriod;

typedef struct {
    uint8_t          system;
    double           query_jd;
    uint8_t          count;
    DhruvDashaPeriod periods[5];
} DhruvDashaSnapshot;

typedef struct {
    int32_t      time_kind;
    double       jd_utc;
    DhruvUtcTime utc;
} DhruvDashaSnapshotTime;

typedef struct {
    uint8_t count;
    uint8_t systems[DHRUV_MAX_DASHA_SYSTEMS];
    uint8_t max_levels[DHRUV_MAX_DASHA_SYSTEMS];
    uint8_t max_level;
    uint8_t level_methods[5];
    uint8_t yogini_scheme;
    uint8_t use_abhijit;
    DhruvDashaSnapshotTime snapshot_time;
} DhruvDashaSelectionConfig;

typedef struct {
    uint8_t level_methods[5];
    uint8_t yogini_scheme;
    uint8_t use_abhijit;
} DhruvDashaVariationConfig;

typedef struct {
    double graha_sidereal_lons[9];
    double lagna_sidereal_lon;
} DhruvRashiDashaInputs;

typedef struct {
    uint8_t               has_moon_sid_lon;
    double                moon_sid_lon;
    uint8_t               has_rashi_inputs;
    DhruvRashiDashaInputs rashi_inputs;
    uint8_t               has_sunrise_sunset;
    double                sunrise_jd;
    double                sunset_jd;
} DhruvDashaInputs;

typedef struct {
    int32_t              time_kind;
    double               birth_jd;
    DhruvUtcTime         birth_utc;
    uint8_t              has_location;
    DhruvGeoLocation     location;
    DhruvBhavaConfig     bhava_config;
    DhruvRiseSetConfig   riseset_config;
    DhruvSankrantiConfig sankranti_config;
    uint8_t              has_inputs;
    DhruvDashaInputs     inputs;
} DhruvDashaBirthContext;

typedef struct {
    DhruvDashaBirthContext   birth;
    uint8_t                  system;
    uint8_t                  max_level;
    DhruvDashaVariationConfig variation;
} DhruvDashaHierarchyRequest;

typedef struct {
    DhruvDashaBirthContext   birth;
    int32_t                  query_time_kind;
    double                   query_jd;
    DhruvUtcTime             query_utc;
    uint8_t                  system;
    uint8_t                  max_level;
    DhruvDashaVariationConfig variation;
} DhruvDashaSnapshotRequest;

typedef struct {
    DhruvDashaBirthContext birth;
    uint8_t                system;
} DhruvDashaLevel0Request;

typedef struct {
    DhruvDashaBirthContext birth;
    uint8_t                system;
    uint8_t                entity_type;
    uint8_t                entity_index;
} DhruvDashaLevel0EntityRequest;

typedef struct {
    DhruvDashaBirthContext   birth;
    uint8_t                  system;
    DhruvDashaVariationConfig variation;
    DhruvDashaPeriod         parent;
} DhruvDashaChildrenRequest;

typedef struct {
    DhruvDashaBirthContext   birth;
    uint8_t                  system;
    DhruvDashaVariationConfig variation;
    DhruvDashaPeriod         parent;
    uint8_t                  child_entity_type;
    uint8_t                  child_entity_index;
} DhruvDashaChildPeriodRequest;

typedef struct {
    DhruvDashaBirthContext   birth;
    uint8_t                  system;
    DhruvDashaVariationConfig variation;
    uint8_t                  child_level;
} DhruvDashaCompleteLevelRequest;

/* --- Full Kundali --- */

typedef struct {
    uint8_t  include_bhava_cusps;
    uint8_t  include_graha_positions;
    uint8_t  include_bindus;
    uint8_t  include_drishti;
    uint8_t  include_ashtakavarga;
    uint8_t  include_upagrahas;
    uint8_t  include_sphutas;
    uint8_t  include_special_lagnas;
    uint8_t  include_amshas;
    uint8_t  include_shadbala;
    uint8_t  include_bhavabala;
    uint8_t  include_vimsopaka;
    uint8_t  include_avastha;
    uint8_t  include_charakaraka;
    uint8_t  charakaraka_scheme;
    uint32_t node_dignity_policy;
    DhruvTimeUpagrahaConfig  upagraha_config;
    DhruvGrahaPositionsConfig graha_positions_config;
    DhruvBindusConfig         bindus_config;
    DhruvDrishtiConfig        drishti_config;
    DhruvAmshaChartScope      amsha_scope;
    DhruvAmshaSelectionConfig amsha_selection;
    uint8_t  include_panchang;
    uint8_t  include_calendar;
    uint8_t  include_dasha;
    DhruvDashaSelectionConfig dasha_config;
} DhruvFullKundaliConfig;

typedef struct {
    double                    ayanamsha_deg;
    uint8_t                   bhava_cusps_valid;
    DhruvBhavaResult          bhava_cusps;
    uint8_t                   graha_positions_valid;
    DhruvGrahaPositions       graha_positions;
    uint8_t                   bindus_valid;
    DhruvBindusResult         bindus;
    uint8_t                   drishti_valid;
    DhruvDrishtiResult        drishti;
    uint8_t                   ashtakavarga_valid;
    DhruvAshtakavargaResult   ashtakavarga;
    uint8_t                   upagrahas_valid;
    DhruvAllUpagrahas         upagrahas;
    uint8_t                   sphutas_valid;
    DhruvSphutalResult        sphutas;
    uint8_t                   special_lagnas_valid;
    DhruvSpecialLagnas        special_lagnas;
    uint8_t                   amshas_valid;
    uint8_t                   amshas_count;
    DhruvAmshaChart           amshas[40];
    uint8_t                   shadbala_valid;
    DhruvShadbalaResult       shadbala;
    uint8_t                   bhavabala_valid;
    DhruvBhavaBalaResult      bhavabala;
    uint8_t                   vimsopaka_valid;
    DhruvVimsopakaResult      vimsopaka;
    uint8_t                   avastha_valid;
    DhruvAllGrahaAvasthas     avastha;
    uint8_t                   charakaraka_valid;
    DhruvCharakarakaResult    charakaraka;
    uint8_t                   panchang_valid;
    DhruvPanchangInfo         panchang;
    uint8_t                   dasha_count;
    DhruvDashaHierarchyHandle dasha_handles[DHRUV_MAX_DASHA_SYSTEMS];
    uint8_t                   dasha_systems[DHRUV_MAX_DASHA_SYSTEMS];
    uint8_t                   dasha_snapshot_count;
    DhruvDashaSnapshot        dasha_snapshots[DHRUV_MAX_DASHA_SYSTEMS];
} DhruvFullKundaliResult;

/* --- Tara (fixed star) --- */

typedef struct {
    double ra_deg;
    double dec_deg;
    double distance_au;
} DhruvEquatorialPosition;

typedef struct {
    double position_au[3];
    double velocity_au_day[3];
} DhruvEarthState;

typedef struct {
    int32_t accuracy;
    uint8_t apply_parallax;
} DhruvTaraConfig;

typedef struct {
    int32_t            tara_id;
    int32_t            output_kind;
    double             jd_tdb;
    double             ayanamsha_deg;
    DhruvTaraConfig    config;
    uint8_t            earth_state_valid;
    DhruvEarthState    earth_state;
} DhruvTaraComputeRequest;

typedef struct {
    int32_t                 output_kind;
    DhruvEquatorialPosition equatorial;
    DhruvSphericalCoords    ecliptic;
    double                  sidereal_longitude_deg;
} DhruvTaraComputeResult;

/* ===================================================================
 * Functions
 * =================================================================== */

/* --- Versioning --- */
uint32_t dhruv_api_version(void);

/* --- Config --- */
DhruvStatus dhruv_config_load(
    const char *path_utf8,
    int32_t defaults_mode,
    DhruvConfigHandle **out_handle);
DhruvStatus dhruv_config_free(DhruvConfigHandle *handle);
DhruvStatus dhruv_config_clear_active(void);

/* --- Engine lifecycle --- */
DhruvStatus dhruv_engine_new(
    const DhruvEngineConfig *config,
    DhruvEngineHandle **out);
DhruvStatus dhruv_engine_query(
    const DhruvEngineHandle *engine,
    const DhruvQuery *query,
    DhruvStateVector *out);
DhruvStatus dhruv_engine_query_request(
    const DhruvEngineHandle *engine,
    const DhruvQueryRequest *request,
    DhruvQueryResult *out);
DhruvStatus dhruv_engine_free(DhruvEngineHandle *engine);
DhruvStatus dhruv_query_once(
    const DhruvEngineConfig *config,
    const DhruvQuery *query,
    DhruvStateVector *out);

/* --- LSK (leap-second kernel) --- */
DhruvStatus dhruv_lsk_load(const char *path, DhruvLskHandle **out);
DhruvStatus dhruv_lsk_free(DhruvLskHandle *lsk);

/* --- Time conversion --- */
DhruvStatus dhruv_utc_to_tdb_jd(
    const DhruvLskHandle *lsk,
    const DhruvEopHandle *eop,
    const DhruvUtcToTdbRequest *request,
    DhruvUtcToTdbResult *out);

/* --- Coordinate transform --- */
DhruvStatus dhruv_cartesian_to_spherical(
    const double *position_km,
    DhruvSphericalCoords *out);

/* --- EOP --- */
DhruvStatus dhruv_eop_load(const char *path, DhruvEopHandle **out);
DhruvStatus dhruv_eop_free(DhruvEopHandle *eop);

/* --- Ayanamsha --- */
uint32_t dhruv_ayanamsha_system_count(void);
int32_t  dhruv_reference_plane_default(int32_t system_code);
DhruvStatus dhruv_ayanamsha_compute_ex(
    const DhruvLskHandle *lsk,
    const DhruvAyanamshaComputeRequest *request,
    const DhruvEopHandle *eop,
    double *out);

/* --- Nutation --- */
DhruvStatus dhruv_nutation_iau2000b(double jd_tdb, double *dpsi, double *deps);

/* --- UTC time helpers --- */
DhruvStatus dhruv_jd_tdb_to_utc(
    const DhruvLskHandle *lsk, double jd_tdb,
    DhruvUtcTime *out);
DhruvStatus dhruv_riseset_result_to_utc(
    const DhruvLskHandle *lsk,
    const DhruvRiseSetResult *result,
    DhruvUtcTime *out);

/* --- Rise/set --- */
DhruvRiseSetConfig dhruv_riseset_config_default(void);
DhruvStatus dhruv_compute_rise_set(
    const DhruvEngineHandle *engine,
    const DhruvLskHandle *lsk,
    const DhruvEopHandle *eop,
    const DhruvGeoLocation *location,
    int32_t event_code,
    double  jd_tdb_approx,
    const DhruvRiseSetConfig *config,
    DhruvRiseSetResult *out);
DhruvStatus dhruv_compute_all_events(
    const DhruvEngineHandle *engine,
    const DhruvLskHandle *lsk,
    const DhruvEopHandle *eop,
    const DhruvGeoLocation *location,
    double jd_tdb_approx,
    const DhruvRiseSetConfig *config,
    DhruvRiseSetResult *out);
double dhruv_approximate_local_noon_jd(double jd_ut_midnight, double longitude_deg);

/* --- Bhava --- */
DhruvBhavaConfig dhruv_bhava_config_default(void);
uint32_t dhruv_bhava_system_count(void);
DhruvStatus dhruv_compute_bhavas(
    const DhruvEngineHandle *engine,
    const DhruvLskHandle *lsk,
    const DhruvEopHandle *eop,
    const DhruvGeoLocation *location,
    double jd_tdb,
    const DhruvBhavaConfig *config,
    DhruvBhavaResult *out);
DhruvStatus dhruv_lagna_deg(
    const DhruvLskHandle *lsk,
    const DhruvEopHandle *eop,
    const DhruvGeoLocation *location,
    double jd_tdb,
    double *out);
DhruvStatus dhruv_lagna_deg_with_config(
    const DhruvLskHandle *lsk,
    const DhruvEopHandle *eop,
    const DhruvGeoLocation *location,
    double jd_tdb,
    const DhruvBhavaConfig *config,
    double *out);
DhruvStatus dhruv_mc_deg(
    const DhruvLskHandle *lsk,
    const DhruvEopHandle *eop,
    const DhruvGeoLocation *location,
    double jd_tdb,
    double *out);
DhruvStatus dhruv_mc_deg_with_config(
    const DhruvLskHandle *lsk,
    const DhruvEopHandle *eop,
    const DhruvGeoLocation *location,
    double jd_tdb,
    const DhruvBhavaConfig *config,
    double *out);
DhruvStatus dhruv_ramc_deg(
    const DhruvLskHandle *lsk,
    const DhruvEopHandle *eop,
    const DhruvGeoLocation *location,
    double jd_tdb,
    double *out);

/* --- Lunar node --- */
DhruvStatus dhruv_lunar_node_deg(
    int32_t node_code, int32_t mode_code,
    double jd_tdb, double *out);
DhruvStatus dhruv_lunar_node_deg_with_engine(
    const DhruvEngineHandle *engine,
    int32_t node_code, int32_t mode_code,
    double jd_tdb, double *out);
DhruvStatus dhruv_lunar_node_compute_ex(
    const DhruvLskHandle *lsk,
    const DhruvEopHandle *eop,
    const DhruvLunarNodeRequest *request,
    double *out);
uint32_t dhruv_lunar_node_count(void);

/* --- Conjunction --- */
DhruvConjunctionConfig dhruv_conjunction_config_default(void);
DhruvStatus dhruv_conjunction_search_ex(
    const DhruvEngineHandle *engine,
    const DhruvConjunctionSearchRequest *request,
    DhruvConjunctionEvent *out_event,
    uint8_t *out_found,
    DhruvConjunctionEvent *out_events,
    uint32_t out_capacity,
    uint32_t *out_count);

/* --- Grahan (eclipse) --- */
DhruvGrahanConfig dhruv_grahan_config_default(void);
DhruvStatus dhruv_grahan_search_ex(
    const DhruvEngineHandle *engine,
    const DhruvGrahanSearchRequest *request,
    DhruvChandraGrahanResult *out_chandra,
    DhruvSuryaGrahanResult *out_surya,
    uint8_t *out_found,
    DhruvChandraGrahanResult *out_chandra_events,
    DhruvSuryaGrahanResult *out_surya_events,
    uint32_t out_capacity,
    uint32_t *out_count);

/* --- Stationary / max-speed --- */
DhruvStationaryConfig dhruv_stationary_config_default(void);
DhruvStatus dhruv_motion_search_ex(
    const DhruvEngineHandle *engine,
    const DhruvMotionSearchRequest *request,
    DhruvStationaryEvent *out_stationary,
    DhruvMaxSpeedEvent *out_max_speed,
    uint8_t *out_found,
    DhruvStationaryEvent *out_stationary_events,
    DhruvMaxSpeedEvent *out_max_speed_events,
    uint32_t out_capacity,
    uint32_t *out_count);

/* --- Rashi / Nakshatra --- */
DhruvStatus dhruv_deg_to_dms(double degrees, DhruvDms *out);
DhruvStatus dhruv_rashi_from_longitude(double sidereal_lon, DhruvRashiInfo *out);
DhruvStatus dhruv_nakshatra_from_longitude(double sidereal_lon, DhruvNakshatraInfo *out);
DhruvStatus dhruv_nakshatra28_from_longitude(double sidereal_lon, DhruvNakshatra28Info *out);
DhruvStatus dhruv_rashi_from_tropical(
    double tropical_lon, uint32_t ayanamsha_system,
    double jd_tdb, uint8_t use_nutation,
    DhruvRashiInfo *out);
DhruvStatus dhruv_nakshatra_from_tropical(
    double tropical_lon, uint32_t ayanamsha_system,
    double jd_tdb, uint8_t use_nutation,
    DhruvNakshatraInfo *out);
DhruvStatus dhruv_nakshatra28_from_tropical(
    double tropical_lon, uint32_t ayanamsha_system,
    double jd_tdb, uint8_t use_nutation,
    DhruvNakshatra28Info *out);
uint32_t dhruv_rashi_count(void);
uint32_t dhruv_nakshatra_count(uint32_t scheme_code);
const char *dhruv_rashi_name(uint32_t index);
const char *dhruv_nakshatra_name(uint32_t index);
const char *dhruv_nakshatra28_name(uint32_t index);

/* --- Sankranti / Lunar phase --- */
DhruvSankrantiConfig dhruv_sankranti_config_default(void);
DhruvStatus dhruv_lunar_phase_search_ex(
    const DhruvEngineHandle *engine,
    const DhruvLunarPhaseSearchRequest *request,
    DhruvLunarPhaseEvent *out_event,
    uint8_t *out_found,
    DhruvLunarPhaseEvent *out_events,
    uint32_t out_capacity,
    uint32_t *out_count);
DhruvStatus dhruv_sankranti_search_ex(
    const DhruvEngineHandle *engine,
    const DhruvSankrantiSearchRequest *request,
    DhruvSankrantiEvent *out_event,
    uint8_t *out_found,
    DhruvSankrantiEvent *out_events,
    uint32_t out_capacity,
    uint32_t *out_count);

/* --- Calendar --- */
DhruvStatus dhruv_masa_for_date(
    const DhruvEngineHandle *engine,
    const DhruvUtcTime *utc,
    const DhruvSankrantiConfig *config,
    DhruvMasaInfo *out);
DhruvStatus dhruv_ayana_for_date(
    const DhruvEngineHandle *engine,
    const DhruvUtcTime *utc,
    const DhruvSankrantiConfig *config,
    DhruvAyanaInfo *out);
DhruvStatus dhruv_varsha_for_date(
    const DhruvEngineHandle *engine,
    const DhruvUtcTime *utc,
    const DhruvSankrantiConfig *config,
    DhruvVarshaInfo *out);
const char *dhruv_masa_name(uint32_t index);
const char *dhruv_ayana_name(uint32_t index);
const char *dhruv_samvatsara_name(uint32_t index);

/* --- Pure-math panchang classifiers --- */
DhruvStatus dhruv_tithi_from_elongation(double elongation_deg, DhruvTithiPosition *out);
DhruvStatus dhruv_karana_from_elongation(double elongation_deg, DhruvKaranaPosition *out);
DhruvStatus dhruv_yoga_from_sum(double sum_deg, DhruvYogaPosition *out);
int32_t dhruv_vaar_from_jd(double jd);
int32_t dhruv_masa_from_rashi_index(uint32_t rashi_index);
int32_t dhruv_ayana_from_sidereal_longitude(double lon_deg);
DhruvStatus dhruv_samvatsara_from_year(int32_t ce_year, DhruvSamvatsaraResult *out);
int32_t dhruv_nth_rashi_from(uint32_t rashi_index, uint32_t offset);

/* --- UTC wrapper functions --- */
DhruvStatus dhruv_compute_rise_set_utc(
    const DhruvEngineHandle *engine,
    const DhruvLskHandle *lsk,
    const DhruvEopHandle *eop,
    const DhruvGeoLocation *location,
    int32_t event_code,
    const DhruvUtcTime *utc,
    const DhruvRiseSetConfig *config,
    DhruvRiseSetResultUtc *out);
DhruvStatus dhruv_compute_all_events_utc(
    const DhruvEngineHandle *engine,
    const DhruvLskHandle *lsk,
    const DhruvEopHandle *eop,
    const DhruvGeoLocation *location,
    const DhruvUtcTime *utc,
    const DhruvRiseSetConfig *config,
    DhruvRiseSetResultUtc *out);
DhruvStatus dhruv_compute_bhavas_utc(
    const DhruvEngineHandle *engine,
    const DhruvLskHandle *lsk,
    const DhruvEopHandle *eop,
    const DhruvGeoLocation *location,
    const DhruvUtcTime *utc,
    const DhruvBhavaConfig *config,
    DhruvBhavaResult *out);
DhruvStatus dhruv_lagna_deg_utc(
    const DhruvLskHandle *lsk,
    const DhruvEopHandle *eop,
    const DhruvGeoLocation *location,
    const DhruvUtcTime *utc,
    double *out);
DhruvStatus dhruv_lagna_deg_utc_with_config(
    const DhruvLskHandle *lsk,
    const DhruvEopHandle *eop,
    const DhruvGeoLocation *location,
    const DhruvUtcTime *utc,
    const DhruvBhavaConfig *config,
    double *out);
DhruvStatus dhruv_mc_deg_utc(
    const DhruvLskHandle *lsk,
    const DhruvEopHandle *eop,
    const DhruvGeoLocation *location,
    const DhruvUtcTime *utc,
    double *out);
DhruvStatus dhruv_mc_deg_utc_with_config(
    const DhruvLskHandle *lsk,
    const DhruvEopHandle *eop,
    const DhruvGeoLocation *location,
    const DhruvUtcTime *utc,
    const DhruvBhavaConfig *config,
    double *out);
DhruvStatus dhruv_ramc_deg_utc(
    const DhruvLskHandle *lsk,
    const DhruvEopHandle *eop,
    const DhruvGeoLocation *location,
    const DhruvUtcTime *utc,
    double *out);
DhruvStatus dhruv_nutation_iau2000b_utc(
    const DhruvLskHandle *lsk,
    const DhruvUtcTime *utc,
    double *dpsi, double *deps);
DhruvStatus dhruv_lunar_node_deg_utc(
    const DhruvLskHandle *lsk,
    int32_t node_code, int32_t mode_code,
    const DhruvUtcTime *utc,
    double *out);
DhruvStatus dhruv_lunar_node_deg_utc_with_engine(
    const DhruvEngineHandle *engine,
    const DhruvLskHandle *lsk,
    int32_t node_code, int32_t mode_code,
    const DhruvUtcTime *utc,
    double *out);
DhruvStatus dhruv_rashi_from_tropical_utc(
    const DhruvLskHandle *lsk,
    double tropical_lon, uint32_t ayanamsha_system,
    const DhruvUtcTime *utc, uint8_t use_nutation,
    DhruvRashiInfo *out);
DhruvStatus dhruv_nakshatra_from_tropical_utc(
    const DhruvLskHandle *lsk,
    double tropical_lon, uint32_t ayanamsha_system,
    const DhruvUtcTime *utc, uint8_t use_nutation,
    DhruvNakshatraInfo *out);
DhruvStatus dhruv_nakshatra28_from_tropical_utc(
    const DhruvLskHandle *lsk,
    double tropical_lon, uint32_t ayanamsha_system,
    const DhruvUtcTime *utc, uint8_t use_nutation,
    DhruvNakshatra28Info *out);
/* --- Panchang for-date functions --- */
DhruvStatus dhruv_tithi_for_date(
    const DhruvEngineHandle *engine,
    const DhruvUtcTime *utc,
    DhruvTithiInfo *out);
DhruvStatus dhruv_karana_for_date(
    const DhruvEngineHandle *engine,
    const DhruvUtcTime *utc,
    DhruvKaranaInfo *out);
DhruvStatus dhruv_yoga_for_date(
    const DhruvEngineHandle *engine,
    const DhruvUtcTime *utc,
    const DhruvSankrantiConfig *config,
    DhruvYogaInfo *out);
DhruvStatus dhruv_nakshatra_for_date(
    const DhruvEngineHandle *engine,
    const DhruvUtcTime *utc,
    const DhruvSankrantiConfig *config,
    DhruvPanchangNakshatraInfo *out);
DhruvStatus dhruv_vaar_for_date(
    const DhruvEngineHandle *engine,
    const DhruvEopHandle *eop,
    const DhruvUtcTime *utc,
    const DhruvGeoLocation *location,
    const DhruvRiseSetConfig *riseset_config,
    DhruvVaarInfo *out);
DhruvStatus dhruv_hora_for_date(
    const DhruvEngineHandle *engine,
    const DhruvEopHandle *eop,
    const DhruvUtcTime *utc,
    const DhruvGeoLocation *location,
    const DhruvRiseSetConfig *riseset_config,
    DhruvHoraInfo *out);
DhruvStatus dhruv_ghatika_for_date(
    const DhruvEngineHandle *engine,
    const DhruvEopHandle *eop,
    const DhruvUtcTime *utc,
    const DhruvGeoLocation *location,
    const DhruvRiseSetConfig *riseset_config,
    DhruvGhatikaInfo *out);

/* --- Unified panchang --- */
DhruvStatus dhruv_panchang_compute_ex(
    const DhruvEngineHandle *engine,
    const DhruvEopHandle *eop,
    const DhruvLskHandle *lsk,
    const DhruvPanchangComputeRequest *request,
    DhruvPanchangOperationResult *out);

/* --- Panchang name lookups --- */
const char *dhruv_tithi_name(uint32_t index);
const char *dhruv_karana_name(uint32_t index);
const char *dhruv_yoga_name(uint32_t index);
const char *dhruv_vaar_name(uint32_t index);
const char *dhruv_hora_name(uint32_t index);

/* --- Panchang composable intermediates --- */
DhruvStatus dhruv_elongation_at(
    const DhruvEngineHandle *engine,
    double jd_tdb, double *out);
DhruvStatus dhruv_sidereal_sum_at(
    const DhruvEngineHandle *engine,
    double jd_tdb,
    const DhruvSankrantiConfig *config,
    double *out);
DhruvStatus dhruv_vedic_day_sunrises(
    const DhruvEngineHandle *engine,
    const DhruvEopHandle *eop,
    const DhruvUtcTime *utc,
    const DhruvGeoLocation *location,
    const DhruvRiseSetConfig *config,
    double *out_sunrise,
    double *out_next_sunrise);
DhruvStatus dhruv_body_ecliptic_lon_lat(
    const DhruvEngineHandle *engine,
    int32_t body_code, double jd_tdb,
    double *out_lon, double *out_lat);
DhruvStatus dhruv_tithi_at(
    const DhruvEngineHandle *engine,
    double jd_tdb, double sunrise_jd,
    DhruvTithiInfo *out);
DhruvStatus dhruv_karana_at(
    const DhruvEngineHandle *engine,
    double jd_tdb, double sunrise_jd,
    DhruvKaranaInfo *out);
DhruvStatus dhruv_yoga_at(
    const DhruvEngineHandle *engine,
    double jd_tdb, double sunrise_jd,
    const DhruvSankrantiConfig *config,
    DhruvYogaInfo *out);
DhruvStatus dhruv_vaar_from_sunrises(
    const DhruvLskHandle *lsk,
    double sunrise_jd, double next_sunrise_jd,
    DhruvVaarInfo *out);
DhruvStatus dhruv_hora_from_sunrises(
    const DhruvLskHandle *lsk,
    double query_jd,
    double sunrise_jd, double next_sunrise_jd,
    DhruvHoraInfo *out);
DhruvStatus dhruv_ghatika_from_sunrises(
    const DhruvLskHandle *lsk,
    double query_jd,
    double sunrise_jd, double next_sunrise_jd,
    DhruvGhatikaInfo *out);

/* --- Graha identifiers --- */
const char *dhruv_graha_name(uint32_t index);
const char *dhruv_yogini_name(uint32_t index);
int32_t dhruv_rashi_lord(uint32_t rashi_index);
int32_t dhruv_hora_lord(uint32_t vaar_index, uint32_t hora_index);
int32_t dhruv_masa_lord(uint32_t masa_index);
int32_t dhruv_samvatsara_lord(uint32_t samvatsara_index);

/* --- Graha relationship / dignity / combustion helper codes --- */
#define DHRUV_NAISARGIKA_FRIEND 0
#define DHRUV_NAISARGIKA_ENEMY 1
#define DHRUV_NAISARGIKA_NEUTRAL 2

#define DHRUV_TATKALIKA_FRIEND 0
#define DHRUV_TATKALIKA_ENEMY 1

#define DHRUV_PANCHADHA_ADHI_SHATRU 0
#define DHRUV_PANCHADHA_SHATRU 1
#define DHRUV_PANCHADHA_SAMA 2
#define DHRUV_PANCHADHA_MITRA 3
#define DHRUV_PANCHADHA_ADHI_MITRA 4

#define DHRUV_DIGNITY_EXALTED 0
#define DHRUV_DIGNITY_MOOLATRIKONE 1
#define DHRUV_DIGNITY_OWN_SIGN 2
#define DHRUV_DIGNITY_ADHI_MITRA 3
#define DHRUV_DIGNITY_MITRA 4
#define DHRUV_DIGNITY_SAMA 5
#define DHRUV_DIGNITY_SHATRU 6
#define DHRUV_DIGNITY_ADHI_SHATRU 7
#define DHRUV_DIGNITY_DEBILITATED 8

#define DHRUV_NODE_DIGNITY_SIGN_LORD_BASED 0
#define DHRUV_NODE_DIGNITY_ALWAYS_SAMA 1

#define DHRUV_BENEFIC_NATURE_BENEFIC 0
#define DHRUV_BENEFIC_NATURE_MALEFIC 1

#define DHRUV_GRAHA_GENDER_MALE 0
#define DHRUV_GRAHA_GENDER_FEMALE 1
#define DHRUV_GRAHA_GENDER_NEUTER 2

/* --- Graha relationship / dignity / combustion helpers --- */
DhruvStatus dhruv_exaltation_degree(
    uint32_t graha_index,
    uint8_t *out_has_value,
    double *out_value);
DhruvStatus dhruv_debilitation_degree(
    uint32_t graha_index,
    uint8_t *out_has_value,
    double *out_value);
DhruvStatus dhruv_moolatrikone_range(
    uint32_t graha_index,
    uint8_t *out_has_value,
    uint8_t *out_rashi_index,
    double *out_start_deg,
    double *out_end_deg);
DhruvStatus dhruv_combustion_threshold(
    uint32_t graha_index,
    uint8_t is_retrograde,
    uint8_t *out_has_value,
    double *out_threshold_deg);
DhruvStatus dhruv_is_combust(
    uint32_t graha_index,
    double graha_sid_lon,
    double sun_sid_lon,
    uint8_t is_retrograde,
    uint8_t *out_is_combust);
DhruvStatus dhruv_all_combustion_status(
    const double *sidereal_lons_9,
    const uint8_t *retrograde_flags_9,
    uint8_t *out_combust_flags_9);
DhruvStatus dhruv_naisargika_maitri(
    uint32_t graha_index,
    uint32_t other_index,
    int32_t *out_code);
DhruvStatus dhruv_tatkalika_maitri(
    uint32_t graha_rashi_index,
    uint32_t other_rashi_index,
    int32_t *out_code);
DhruvStatus dhruv_panchadha_maitri(
    int32_t naisargika_code,
    int32_t tatkalika_code,
    int32_t *out_code);
DhruvStatus dhruv_dignity_in_rashi(
    uint32_t graha_index,
    double sidereal_lon,
    uint32_t rashi_index,
    int32_t *out_code);
DhruvStatus dhruv_dignity_in_rashi_with_positions(
    uint32_t graha_index,
    double sidereal_lon,
    uint32_t rashi_index,
    const uint8_t *sapta_rashi_indices_7,
    int32_t *out_code);
DhruvStatus dhruv_node_dignity_in_rashi(
    uint32_t graha_index,
    uint32_t rashi_index,
    const uint8_t *graha_rashi_indices_9,
    int32_t policy_code,
    int32_t *out_code);
DhruvStatus dhruv_natural_benefic_malefic(
    uint32_t graha_index,
    int32_t *out_code);
DhruvStatus dhruv_moon_benefic_nature(
    double moon_sun_elongation,
    int32_t *out_code);
DhruvStatus dhruv_graha_gender(
    uint32_t graha_index,
    int32_t *out_code);

/* --- Sphuta --- */
const char *dhruv_sphuta_name(uint32_t index);
DhruvStatus dhruv_all_sphutas(
    const DhruvSphutalInputs *inputs,
    DhruvSphutalResult *out);
double dhruv_bhrigu_bindu(double rahu, double moon);
double dhruv_prana_sphuta(double lagna, double moon);
double dhruv_deha_sphuta(double moon, double lagna);
double dhruv_mrityu_sphuta(double eighth_lord, double lagna);
double dhruv_tithi_sphuta(double moon, double sun, double lagna);
double dhruv_yoga_sphuta(double sun, double moon);
double dhruv_yoga_sphuta_normalized(double sun, double moon);
double dhruv_rahu_tithi_sphuta(double rahu, double sun, double lagna);
double dhruv_kshetra_sphuta(
    double venus, double moon, double mars,
    double jupiter, double lagna);
double dhruv_beeja_sphuta(double sun, double venus, double jupiter);
double dhruv_trisphuta(double lagna, double moon, double gulika);
double dhruv_chatussphuta(double trisphuta_val, double sun);
double dhruv_panchasphuta(double chatussphuta_val, double rahu);
double dhruv_sookshma_trisphuta(
    double lagna, double moon, double gulika, double sun);
double dhruv_avayoga_sphuta(double sun, double moon);
double dhruv_kunda(double lagna, double moon, double mars);

/* --- Special Lagnas --- */
const char *dhruv_special_lagna_name(uint32_t index);
double dhruv_bhava_lagna(double sun_lon, double ghatikas);
double dhruv_hora_lagna(double sun_lon, double ghatikas);
double dhruv_ghati_lagna(double sun_lon, double ghatikas);
double dhruv_vighati_lagna(double lagna_lon, double vighatikas);
double dhruv_varnada_lagna(double lagna_lon, double hora_lagna_lon);
double dhruv_sree_lagna(double moon_lon, double lagna_lon);
double dhruv_pranapada_lagna(double sun_lon, double ghatikas);
double dhruv_indu_lagna(double moon_lon, uint32_t lagna_lord, uint32_t moon_9th_lord);
DhruvStatus dhruv_special_lagnas_for_date(
    const DhruvEngineHandle *engine,
    const DhruvEopHandle *eop,
    const DhruvUtcTime *utc,
    const DhruvGeoLocation *location,
    const DhruvRiseSetConfig *riseset_config,
    uint32_t ayanamsha_system,
    uint8_t use_nutation,
    DhruvSpecialLagnas *out);

/* --- Arudha Padas --- */
const char *dhruv_arudha_pada_name(uint32_t index);
double dhruv_arudha_pada(
    double bhava_cusp_lon,
    double lord_lon,
    uint8_t *out_rashi);
DhruvStatus dhruv_arudha_padas_for_date(
    const DhruvEngineHandle *engine,
    const DhruvEopHandle *eop,
    const DhruvUtcTime *utc,
    const DhruvGeoLocation *location,
    uint32_t ayanamsha_system,
    uint8_t use_nutation,
    DhruvArudhaResult *out);

/* --- Upagrahas --- */
const char *dhruv_upagraha_name(uint32_t index);
DhruvTimeUpagrahaConfig dhruv_time_upagraha_config_default(void);
DhruvStatus dhruv_sun_based_upagrahas(
    double sun_sid_lon,
    DhruvAllUpagrahas *out);
DhruvStatus dhruv_time_upagraha_jd(
    uint32_t upagraha_index,
    uint32_t weekday,
    uint8_t is_day,
    double sunrise_jd,
    double sunset_jd,
    double next_sunrise_jd,
    double *out_jd);
DhruvStatus dhruv_time_upagraha_jd_with_config(
    uint32_t upagraha_index,
    uint32_t weekday,
    uint8_t is_day,
    double sunrise_jd,
    double sunset_jd,
    double next_sunrise_jd,
    const DhruvTimeUpagrahaConfig *upagraha_config,
    double *out_jd);
DhruvStatus dhruv_time_upagraha_jd_utc(
    const DhruvEngineHandle *engine,
    const DhruvEopHandle *eop,
    const DhruvUtcTime *utc,
    const DhruvGeoLocation *location,
    const DhruvRiseSetConfig *riseset_config,
    uint32_t upagraha_index,
    double *out_jd);
DhruvStatus dhruv_time_upagraha_jd_utc_with_config(
    const DhruvEngineHandle *engine,
    const DhruvEopHandle *eop,
    const DhruvUtcTime *utc,
    const DhruvGeoLocation *location,
    const DhruvRiseSetConfig *riseset_config,
    const DhruvTimeUpagrahaConfig *upagraha_config,
    uint32_t upagraha_index,
    double *out_jd);
DhruvStatus dhruv_all_upagrahas_for_date(
    const DhruvEngineHandle *engine,
    const DhruvEopHandle *eop,
    const DhruvUtcTime *utc,
    const DhruvGeoLocation *location,
    uint32_t ayanamsha_system,
    uint8_t use_nutation,
    DhruvAllUpagrahas *out);
DhruvStatus dhruv_all_upagrahas_for_date_with_config(
    const DhruvEngineHandle *engine,
    const DhruvEopHandle *eop,
    const DhruvUtcTime *utc,
    const DhruvGeoLocation *location,
    uint32_t ayanamsha_system,
    uint8_t use_nutation,
    const DhruvTimeUpagrahaConfig *upagraha_config,
    DhruvAllUpagrahas *out);

/* --- Ashtakavarga --- */
DhruvStatus dhruv_calculate_ashtakavarga(
    const uint8_t *graha_rashis, uint8_t lagna_rashi,
    DhruvAshtakavargaResult *out);
DhruvStatus dhruv_calculate_bav(
    uint8_t graha_index,
    const uint8_t *graha_rashis, uint8_t lagna_rashi,
    DhruvBhinnaAshtakavarga *out);
DhruvStatus dhruv_calculate_all_bav(
    const uint8_t *graha_rashis, uint8_t lagna_rashi,
    DhruvBhinnaAshtakavarga *out);
DhruvStatus dhruv_calculate_sav(
    const DhruvBhinnaAshtakavarga *bavs,
    DhruvSarvaAshtakavarga *out);
DhruvStatus dhruv_trikona_sodhana(const uint8_t *totals, uint8_t *out);
DhruvStatus dhruv_ekadhipatya_sodhana(
    const uint8_t *totals,
    const uint8_t *graha_rashis, uint8_t lagna_rashi,
    uint8_t *out);
DhruvStatus dhruv_ashtakavarga_for_date(
    const DhruvEngineHandle *engine,
    const DhruvEopHandle *eop,
    const DhruvUtcTime *utc,
    const DhruvGeoLocation *location,
    uint32_t ayanamsha_system,
    uint8_t use_nutation,
    DhruvAshtakavargaResult *out);

/* --- Drishti --- */
DhruvStatus dhruv_graha_drishti(
    uint32_t graha_index,
    double source_lon, double target_lon,
    DhruvDrishtiEntry *out);
DhruvStatus dhruv_graha_drishti_matrix(
    const double *sidereal_lons,
    DhruvGrahaDrishtiMatrix *out);
DhruvStatus dhruv_drishti(
    const DhruvEngineHandle *engine,
    const DhruvEopHandle *eop,
    const DhruvUtcTime *utc,
    const DhruvGeoLocation *location,
    const DhruvBhavaConfig *bhava_config,
    const DhruvRiseSetConfig *riseset_config,
    uint32_t ayanamsha_system,
    uint8_t use_nutation,
    const DhruvDrishtiConfig *config,
    DhruvDrishtiResult *out);

/* --- Ghatika / hora helpers --- */
DhruvStatus dhruv_ghatika_from_elapsed(
    double query_jd, double sunrise_jd, double next_sunrise_jd,
    int32_t *out);
DhruvStatus dhruv_ghatikas_since_sunrise(
    double query_jd, double sunrise_jd,
    double *out);
int32_t dhruv_hora_at(uint32_t vaar_index, uint32_t hora_index);

/* --- Graha positions --- */
DhruvStatus dhruv_graha_positions(
    const DhruvEngineHandle *engine,
    const DhruvEopHandle *eop,
    const DhruvUtcTime *utc,
    const DhruvGeoLocation *location,
    const DhruvBhavaConfig *bhava_config,
    uint32_t ayanamsha_system,
    uint8_t use_nutation,
    const DhruvGrahaPositionsConfig *config,
    DhruvGrahaPositions *out);

/* --- Core Bindus --- */
DhruvStatus dhruv_core_bindus(
    const DhruvEngineHandle *engine,
    const DhruvEopHandle *eop,
    const DhruvUtcTime *utc,
    const DhruvGeoLocation *location,
    const DhruvBhavaConfig *bhava_config,
    const DhruvRiseSetConfig *riseset_config,
    uint32_t ayanamsha_system,
    uint8_t use_nutation,
    const DhruvBindusConfig *config,
    DhruvBindusResult *out);

/* --- Graha longitudes --- */
DhruvGrahaLongitudesConfig dhruv_graha_longitudes_config_default(void);
DhruvStatus dhruv_graha_longitudes(
    const DhruvEngineHandle *engine,
    double jd_tdb,
    const DhruvGrahaLongitudesConfig *config,
    DhruvGrahaLongitudes *out);

/* --- Nakshatra at --- */
DhruvStatus dhruv_nakshatra_at(
    const DhruvEngineHandle *engine,
    double jd_tdb,
    double moon_sidereal_deg,
    const DhruvSankrantiConfig *config,
    DhruvPanchangNakshatraInfo *out);

/* --- Amsha (divisional chart) --- */
DhruvStatus dhruv_amsha_longitude(
    double sidereal_lon,
    uint16_t amsha_code,
    uint8_t variation_code,
    double *out);
DhruvStatus dhruv_amsha_rashi_info(
    double sidereal_lon,
    uint16_t amsha_code,
    uint8_t variation_code,
    DhruvRashiInfo *out);
DhruvStatus dhruv_amsha_longitudes(
    double sidereal_lon,
    const uint16_t *amsha_codes,
    const uint8_t *variation_codes,
    uint32_t count,
    double *out);
DhruvStatus dhruv_amsha_chart_for_date(
    const DhruvEngineHandle *engine,
    const DhruvEopHandle *eop,
    const DhruvUtcTime *utc,
    const DhruvGeoLocation *location,
    const DhruvBhavaConfig *bhava_config,
    const DhruvRiseSetConfig *riseset_config,
    uint32_t ayanamsha_system,
    uint8_t use_nutation,
    uint16_t amsha_code,
    uint8_t variation_code,
    const DhruvAmshaChartScope *scope,
    DhruvAmshaChart *out);

/* --- Charakaraka --- */
DhruvStatus dhruv_charakaraka_for_date(
    const DhruvEngineHandle *engine,
    const DhruvEopHandle *eop,
    const DhruvUtcTime *utc,
    uint32_t ayanamsha_system,
    uint8_t use_nutation,
    uint8_t scheme,
    DhruvCharakarakaResult *out);

/* --- Shadbala --- */
DhruvStatus dhruv_shadbala_for_date(
    const DhruvEngineHandle *engine,
    const DhruvEopHandle *eop,
    const DhruvUtcTime *utc,
    const DhruvGeoLocation *location,
    const DhruvBhavaConfig *bhava_config,
    const DhruvRiseSetConfig *riseset_config,
    uint32_t ayanamsha_system,
    uint8_t use_nutation,
    DhruvShadbalaResult *out);

/* --- Bhava Bala --- */
DhruvStatus dhruv_calculate_bhavabala(
    const DhruvBhavaBalaInputs *inputs,
    DhruvBhavaBalaResult *out);
DhruvStatus dhruv_bhavabala_for_date(
    const DhruvEngineHandle *engine,
    const DhruvEopHandle *eop,
    const DhruvUtcTime *utc,
    const DhruvGeoLocation *location,
    const DhruvBhavaConfig *bhava_config,
    const DhruvRiseSetConfig *riseset_config,
    uint32_t ayanamsha_system,
    uint8_t use_nutation,
    DhruvBhavaBalaResult *out);

/* --- Vimsopaka --- */
DhruvStatus dhruv_vimsopaka_for_date(
    const DhruvEngineHandle *engine,
    const DhruvEopHandle *eop,
    const DhruvUtcTime *utc,
    const DhruvGeoLocation *location,
    uint32_t ayanamsha_system,
    uint8_t use_nutation,
    uint32_t node_dignity_policy,
    DhruvVimsopakaResult *out);

/* --- Bala Bundle --- */
DhruvStatus dhruv_balas_for_date(
    const DhruvEngineHandle *engine,
    const DhruvEopHandle *eop,
    const DhruvUtcTime *utc,
    const DhruvGeoLocation *location,
    const DhruvBhavaConfig *bhava_config,
    const DhruvRiseSetConfig *riseset_config,
    uint32_t ayanamsha_system,
    uint8_t use_nutation,
    uint32_t node_dignity_policy,
    DhruvBalaBundleResult *out);

/* --- Avastha --- */
DhruvStatus dhruv_avastha_for_date(
    const DhruvEngineHandle *engine,
    const DhruvEopHandle *eop,
    const DhruvUtcTime *utc,
    const DhruvGeoLocation *location,
    const DhruvBhavaConfig *bhava_config,
    const DhruvRiseSetConfig *riseset_config,
    uint32_t ayanamsha_system,
    uint8_t use_nutation,
    uint32_t node_dignity_policy,
    DhruvAllGrahaAvasthas *out);

/* --- Dasha --- */
DhruvDashaVariationConfig dhruv_dasha_variation_config_default(void);
DhruvDashaSelectionConfig dhruv_dasha_selection_config_default(void);
DhruvStatus dhruv_dasha_hierarchy_level_count(
    DhruvDashaHierarchyHandle handle, uint8_t *out);
DhruvStatus dhruv_dasha_hierarchy_period_count(
    DhruvDashaHierarchyHandle handle,
    uint8_t level, uint32_t *out);
DhruvStatus dhruv_dasha_hierarchy_period_at(
    DhruvDashaHierarchyHandle handle,
    uint8_t level, uint32_t idx,
    DhruvDashaPeriod *out);
void dhruv_dasha_hierarchy_free(DhruvDashaHierarchyHandle handle);
DhruvStatus dhruv_dasha_period_list_count(
    DhruvDashaPeriodListHandle handle, uint32_t *out);
DhruvStatus dhruv_dasha_period_list_at(
    DhruvDashaPeriodListHandle handle,
    uint32_t idx,
    DhruvDashaPeriod *out);
void dhruv_dasha_period_list_free(DhruvDashaPeriodListHandle handle);
DhruvStatus dhruv_dasha_hierarchy(
    const DhruvEngineHandle *engine,
    const DhruvEopHandle *eop,
    const DhruvDashaHierarchyRequest *request,
    DhruvDashaHierarchyHandle *out);
DhruvStatus dhruv_dasha_snapshot(
    const DhruvEngineHandle *engine,
    const DhruvEopHandle *eop,
    const DhruvDashaSnapshotRequest *request,
    DhruvDashaSnapshot *out);
DhruvStatus dhruv_dasha_level0(
    const DhruvEngineHandle *engine,
    const DhruvEopHandle *eop,
    const DhruvDashaLevel0Request *request,
    DhruvDashaPeriodListHandle *out);
DhruvStatus dhruv_dasha_level0_entity(
    const DhruvEngineHandle *engine,
    const DhruvEopHandle *eop,
    const DhruvDashaLevel0EntityRequest *request,
    uint8_t *out_found,
    DhruvDashaPeriod *out);
DhruvStatus dhruv_dasha_children(
    const DhruvEngineHandle *engine,
    const DhruvEopHandle *eop,
    const DhruvDashaChildrenRequest *request,
    DhruvDashaPeriodListHandle *out);
DhruvStatus dhruv_dasha_child_period(
    const DhruvEngineHandle *engine,
    const DhruvEopHandle *eop,
    const DhruvDashaChildPeriodRequest *request,
    uint8_t *out_found,
    DhruvDashaPeriod *out);
DhruvStatus dhruv_dasha_complete_level(
    const DhruvEngineHandle *engine,
    const DhruvEopHandle *eop,
    const DhruvDashaCompleteLevelRequest *request,
    const DhruvDashaPeriod *parent_periods,
    uint32_t parent_count,
    DhruvDashaPeriodListHandle *out);

/* --- Full Kundali --- */
DhruvFullKundaliConfig dhruv_full_kundali_config_default(void);
DhruvStatus dhruv_full_kundali_for_date(
    const DhruvEngineHandle *engine,
    const DhruvEopHandle *eop,
    const DhruvUtcTime *utc,
    const DhruvGeoLocation *location,
    const DhruvBhavaConfig *bhava_config,
    const DhruvRiseSetConfig *riseset_config,
    uint32_t ayanamsha_system,
    uint8_t use_nutation,
    const DhruvFullKundaliConfig *config,
    DhruvFullKundaliResult *out);
void dhruv_full_kundali_result_free(DhruvFullKundaliResult *result);

/* --- Tara (fixed star) --- */
DhruvStatus dhruv_tara_catalog_load(
    const uint8_t *path_utf8, uint32_t path_len,
    DhruvTaraCatalogHandle **out_handle);
void dhruv_tara_catalog_free(DhruvTaraCatalogHandle *handle);
DhruvStatus dhruv_tara_compute_ex(
    const DhruvTaraCatalogHandle *handle,
    const DhruvTaraComputeRequest *request,
    DhruvTaraComputeResult *out);
DhruvStatus dhruv_tara_galactic_center_ecliptic(
    const DhruvTaraCatalogHandle *handle,
    double jd_tdb,
    DhruvSphericalCoords *out);
DhruvStatus dhruv_tara_propagate_position(
    double ra_deg,
    double dec_deg,
    double parallax_mas,
    double pm_ra_mas_yr,
    double pm_dec_mas_yr,
    double rv_km_s,
    double dt_years,
    DhruvEquatorialPosition *out);
DhruvStatus dhruv_tara_apply_aberration(
    const double *direction_3,
    const double *earth_vel_au_day_3,
    double *out_direction_3);
DhruvStatus dhruv_tara_apply_light_deflection(
    const double *direction_3,
    const double *sun_to_observer_3,
    double sun_observer_distance_au,
    double *out_direction_3);
DhruvStatus dhruv_tara_galactic_anticenter_icrs(double *out_direction_3);

#ifdef __cplusplus
}
#endif

#endif /* DHRUV_H */
