use dhruv_search::{SankrantiConfig, amsha_charts_for_date};
use dhruv_time::{EopKernel, UtcTime};
pub use dhruv_vedic_base::{
    ALL_AMSHAS, Amsha, AmshaRequest, AmshaVariationCatalog, AmshaVariationCode, AmshaVariationInfo,
    D2_CANCER_LEO_ONLY_VARIATION_CODE, DEFAULT_AMSHA_VARIATION_CODE, RashiElement, RashiInfo,
    SHODASHAVARGA, amsha_from_rashi_position, amsha_longitude, amsha_longitudes, amsha_rashi_info,
    amsha_rashi_infos, amsha_variation_by_name, amsha_variation_catalog, amsha_variation_info,
    amsha_variations, default_amsha_variation, rashi_element, rashi_position_to_longitude,
};

use crate::date::UtcDate;
use crate::{AyanamshaSystem, BhavaConfig, DhruvContext, DhruvError, GeoLocation, RiseSetConfig};

pub use dhruv_search::{AmshaChart, AmshaChartScope, AmshaResult, AmshaSelectionConfig};

pub fn amsha_variations_many(amshas: &[Amsha]) -> Vec<AmshaVariationCatalog> {
    amshas
        .iter()
        .copied()
        .map(amsha_variation_catalog)
        .collect()
}

/// Compute amsha charts for a given date and location using explicit configs.
pub fn charts_for_date(
    ctx: &DhruvContext,
    eop: &EopKernel,
    date: UtcDate,
    location: &GeoLocation,
    bhava_config: &BhavaConfig,
    riseset_config: &RiseSetConfig,
    system: AyanamshaSystem,
    use_nutation: bool,
    requests: &[AmshaRequest],
    scope: &AmshaChartScope,
) -> Result<AmshaResult, DhruvError> {
    let utc: UtcTime = date.into();
    let aya_config = SankrantiConfig::new(system, use_nutation);
    Ok(amsha_charts_for_date(
        ctx.engine(),
        eop,
        &utc,
        location,
        bhava_config,
        riseset_config,
        &aya_config,
        requests,
        scope,
    )?)
}

/// Compute amsha charts for a given date and location using default bhava and rise/set configs.
pub fn charts(
    ctx: &DhruvContext,
    eop: &EopKernel,
    date: UtcDate,
    location: &GeoLocation,
    system: AyanamshaSystem,
    use_nutation: bool,
    requests: &[AmshaRequest],
    scope: &AmshaChartScope,
) -> Result<AmshaResult, DhruvError> {
    charts_for_date(
        ctx,
        eop,
        date,
        location,
        &BhavaConfig::default(),
        &RiseSetConfig::default(),
        system,
        use_nutation,
        requests,
        scope,
    )
}

/// Compute a single amsha chart using explicit configs.
pub fn chart_for_date(
    ctx: &DhruvContext,
    eop: &EopKernel,
    date: UtcDate,
    location: &GeoLocation,
    bhava_config: &BhavaConfig,
    riseset_config: &RiseSetConfig,
    system: AyanamshaSystem,
    use_nutation: bool,
    amsha: Amsha,
    variation_code: AmshaVariationCode,
    scope: &AmshaChartScope,
) -> Result<AmshaChart, DhruvError> {
    let requests = [AmshaRequest::with_variation(amsha, variation_code)];
    let mut result = charts_for_date(
        ctx,
        eop,
        date,
        location,
        bhava_config,
        riseset_config,
        system,
        use_nutation,
        &requests,
        scope,
    )?;
    result.charts.pop().ok_or(DhruvError::Config(
        "amsha chart result unexpectedly empty".to_string(),
    ))
}

/// Compute a single amsha chart using default bhava and rise/set configs.
pub fn chart(
    ctx: &DhruvContext,
    eop: &EopKernel,
    date: UtcDate,
    location: &GeoLocation,
    system: AyanamshaSystem,
    use_nutation: bool,
    amsha: Amsha,
    variation_code: AmshaVariationCode,
    scope: &AmshaChartScope,
) -> Result<AmshaChart, DhruvError> {
    chart_for_date(
        ctx,
        eop,
        date,
        location,
        &BhavaConfig::default(),
        &RiseSetConfig::default(),
        system,
        use_nutation,
        amsha,
        variation_code,
        scope,
    )
}

pub use dhruv_vedic_base::{
    ALL_AMSHAS as ALL, SHODASHAVARGA as SHODASHA, amsha_longitude as longitude,
    amsha_longitudes as longitudes, amsha_rashi_info as rashi_info,
};
