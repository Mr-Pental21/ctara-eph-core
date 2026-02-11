//! Vedic jyotish orchestration: queries engine for graha positions.
//!
//! Provides the bridge between the ephemeris engine and the pure-math
//! Vedic calculation modules. Queries all 9 graha positions at a given
//! epoch and converts to sidereal longitudes.

use dhruv_core::{Body, Engine};
use dhruv_vedic_base::{
    AyanamshaSystem, Graha, LunarNode, NodeMode, ALL_GRAHAS,
    ayanamsha_deg, jd_tdb_to_centuries, lunar_node_deg,
};

use crate::conjunction::body_ecliptic_lon_lat;
use crate::error::SearchError;
use crate::jyotish_types::GrahaLongitudes;

/// Map a Graha to its dhruv_core::Body for engine queries.
fn graha_to_body(graha: Graha) -> Option<Body> {
    match graha {
        Graha::Surya => Some(Body::Sun),
        Graha::Chandra => Some(Body::Moon),
        Graha::Mangal => Some(Body::Mars),
        Graha::Buddh => Some(Body::Mercury),
        Graha::Guru => Some(Body::Jupiter),
        Graha::Shukra => Some(Body::Venus),
        Graha::Shani => Some(Body::Saturn),
        Graha::Rahu | Graha::Ketu => None,
    }
}

/// Query all 9 graha sidereal longitudes at a given TDB epoch.
///
/// For the 7 physical planets, queries the engine for tropical ecliptic
/// longitude and subtracts ayanamsha. For Rahu/Ketu, uses the mean/true
/// node mathematical formulas.
pub fn graha_sidereal_longitudes(
    engine: &Engine,
    jd_tdb: f64,
    system: AyanamshaSystem,
    use_nutation: bool,
) -> Result<GrahaLongitudes, SearchError> {
    let t = jd_tdb_to_centuries(jd_tdb);
    let aya = ayanamsha_deg(system, t, use_nutation);

    let mut longitudes = [0.0f64; 9];

    for graha in ALL_GRAHAS {
        let idx = graha.index() as usize;
        match graha {
            Graha::Rahu => {
                let rahu_tropical = lunar_node_deg(LunarNode::Rahu, t, NodeMode::True);
                longitudes[idx] = normalize(rahu_tropical - aya);
            }
            Graha::Ketu => {
                let ketu_tropical = lunar_node_deg(LunarNode::Ketu, t, NodeMode::True);
                longitudes[idx] = normalize(ketu_tropical - aya);
            }
            _ => {
                let body = graha_to_body(graha).expect("sapta graha has body");
                let (lon_tropical, _lat) = body_ecliptic_lon_lat(engine, body, jd_tdb)?;
                longitudes[idx] = normalize(lon_tropical - aya);
            }
        }
    }

    Ok(GrahaLongitudes { longitudes })
}

/// Normalize longitude to [0, 360).
fn normalize(deg: f64) -> f64 {
    let r = deg % 360.0;
    if r < 0.0 { r + 360.0 } else { r }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graha_to_body_mapping() {
        assert_eq!(graha_to_body(Graha::Surya), Some(Body::Sun));
        assert_eq!(graha_to_body(Graha::Chandra), Some(Body::Moon));
        assert_eq!(graha_to_body(Graha::Mangal), Some(Body::Mars));
        assert_eq!(graha_to_body(Graha::Buddh), Some(Body::Mercury));
        assert_eq!(graha_to_body(Graha::Guru), Some(Body::Jupiter));
        assert_eq!(graha_to_body(Graha::Shukra), Some(Body::Venus));
        assert_eq!(graha_to_body(Graha::Shani), Some(Body::Saturn));
        assert_eq!(graha_to_body(Graha::Rahu), None);
        assert_eq!(graha_to_body(Graha::Ketu), None);
    }
}
