//! Compatibility shim over `dhruv_vedic_math` and `dhruv_vedic_engine`.

pub mod amsha {
    pub use dhruv_vedic_math::amsha::*;
}
pub mod arudha {
    pub use dhruv_vedic_math::arudha::*;
}
pub mod ashtakavarga {
    pub use dhruv_vedic_math::ashtakavarga::*;
}
pub mod avastha {
    pub use dhruv_vedic_math::avastha::*;
}
pub mod ayana_type {
    pub use dhruv_vedic_math::ayana_type::*;
}
pub mod ayanamsha {
    pub use dhruv_vedic_engine::ayanamsha::*;
}
pub mod bhava {
    pub use dhruv_vedic_engine::bhava::*;
}
pub mod bhava_types {
    pub use dhruv_vedic_engine::bhava_types::*;
}
pub mod charakaraka {
    pub use dhruv_vedic_math::charakaraka::*;
}
pub mod combustion {
    pub use dhruv_vedic_math::combustion::*;
}
pub mod dasha {
    pub use dhruv_vedic_math::dasha::*;
}
pub mod drishti {
    pub use dhruv_vedic_math::drishti::*;
}
pub mod error {
    pub use dhruv_vedic_engine::error::*;
}
pub mod ghatika {
    pub use dhruv_vedic_math::ghatika::*;
}
pub mod graha {
    pub use dhruv_vedic_math::graha::*;
}
pub mod graha_relationships {
    pub use dhruv_vedic_math::graha_relationships::*;
}
pub mod hora {
    pub use dhruv_vedic_math::hora::*;
}
pub mod karana {
    pub use dhruv_vedic_math::karana::*;
}
pub mod lagna {
    pub use dhruv_vedic_engine::lagna::*;
}
pub mod lunar_nodes {
    pub use dhruv_vedic_engine::lunar_nodes::*;
}
pub mod masa {
    pub use dhruv_vedic_math::masa::*;
}
pub mod nakshatra {
    pub use dhruv_vedic_math::nakshatra::*;

    use dhruv_vedic_engine::{AyanamshaSystem, ayanamsha_deg, jd_tdb_to_centuries};

    pub fn nakshatra_from_tropical(
        tropical_lon_deg: f64,
        system: AyanamshaSystem,
        jd_tdb: f64,
        use_nutation: bool,
    ) -> NakshatraInfo {
        let t = jd_tdb_to_centuries(jd_tdb);
        let aya = ayanamsha_deg(system, t, use_nutation);
        nakshatra_from_longitude(tropical_lon_deg - aya)
    }

    pub fn nakshatra28_from_tropical(
        tropical_lon_deg: f64,
        system: AyanamshaSystem,
        jd_tdb: f64,
        use_nutation: bool,
    ) -> Nakshatra28Info {
        let t = jd_tdb_to_centuries(jd_tdb);
        let aya = ayanamsha_deg(system, t, use_nutation);
        nakshatra28_from_longitude(tropical_lon_deg - aya)
    }
}
pub mod rashi {
    pub use dhruv_vedic_math::rashi::*;

    use dhruv_vedic_engine::{AyanamshaSystem, ayanamsha_deg, jd_tdb_to_centuries};

    pub fn rashi_from_tropical(
        tropical_lon_deg: f64,
        system: AyanamshaSystem,
        jd_tdb: f64,
        use_nutation: bool,
    ) -> RashiInfo {
        let t = jd_tdb_to_centuries(jd_tdb);
        let aya = ayanamsha_deg(system, t, use_nutation);
        rashi_from_longitude(tropical_lon_deg - aya)
    }
}
pub mod riseset {
    pub use dhruv_vedic_engine::riseset::*;
}
pub mod riseset_types {
    pub use dhruv_vedic_engine::riseset_types::*;
}
pub mod samvatsara {
    pub use dhruv_vedic_math::samvatsara::*;
}
pub mod shadbala {
    pub use dhruv_vedic_math::shadbala::*;
}
pub mod special_lagna {
    pub use dhruv_vedic_math::special_lagna::*;
}
pub mod sphuta {
    pub use dhruv_vedic_math::sphuta::*;
}
pub mod time_policy {
    pub use dhruv_vedic_engine::time_policy::*;
}
pub mod tithi {
    pub use dhruv_vedic_math::tithi::*;
}
pub mod upagraha {
    pub use dhruv_vedic_math::upagraha::*;
}
pub mod util {
    pub use dhruv_vedic_math::util::*;
}
pub mod vaar {
    pub use dhruv_vedic_math::vaar::*;
}
pub mod vimsopaka {
    pub use dhruv_vedic_math::vimsopaka::*;
}
pub mod yoga {
    pub use dhruv_vedic_math::yoga::*;
}

pub use dhruv_vedic_engine::VedicError;
pub use dhruv_vedic_engine::*;
pub use dhruv_vedic_math::*;
pub use nakshatra::{nakshatra_from_tropical, nakshatra28_from_tropical};
pub use rashi::rashi_from_tropical;
