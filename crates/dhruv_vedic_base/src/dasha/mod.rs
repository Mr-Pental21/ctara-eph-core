//! Dasha (planetary period) calculations for Vedic astrology.
//!
//! Implements 23 dasha systems from BPHS across 4 categories:
//! - Nakshatra-based (10 systems): Vimshottari, Ashtottari, etc.
//! - Yogini (1 system)
//! - Rashi-based (10 systems): Chara, Sthira, Kendradi, etc.
//! - Special (2 systems): Kala, Kaal Chakra
//!
//! Each system supports 5 hierarchical levels (Mahadasha through Pranadasha)
//! and 6 computation tiers from simple level-0 generation to efficient
//! snapshot queries.
//!
//! See `docs/clean_room_dasha.md` for algorithm provenance.

pub mod balance;
pub mod nakshatra;
pub mod nakshatra_data;
pub mod query;
pub mod rashi_util;
pub mod subperiod;
pub mod types;
pub mod variation;
pub mod yogini;
pub mod yogini_data;

// Rashi-based dasha modules (Phase 18c)
pub mod chakra;
pub mod chara;
pub mod driga;
pub mod kendradi;
pub mod mandooka;
pub mod rashi_dasha;
pub mod rashi_strength;
pub mod shoola;
pub mod sthira;
pub mod yogardha;

pub use balance::{nakshatra_birth_balance, rashi_birth_balance};
pub use nakshatra::{
    nakshatra_child_period, nakshatra_children, nakshatra_complete_level, nakshatra_hierarchy,
    nakshatra_level0, nakshatra_level0_entity, nakshatra_snapshot,
};
pub use nakshatra_data::{NakshatraDashaConfig, nakshatra_config_for_system, vimshottari_config};
pub use query::{find_active_period, snapshot_from_hierarchy};
pub use rashi_util::{
    SignType, count_signs_forward, count_signs_reverse, is_odd_sign, jump_rashi, next_rashi,
    sign_type,
};
pub use subperiod::{
    equal_children, generate_children, proportional_children, snap_last_child_end,
};
pub use types::{
    ALL_DASHA_SYSTEMS, DAYS_PER_YEAR, DEFAULT_DASHA_LEVEL, DashaEntity, DashaHierarchy, DashaLevel,
    DashaPeriod, DashaSnapshot, DashaSystem, MAX_DASHA_LEVEL, MAX_DASHA_SYSTEMS,
    MAX_PERIODS_PER_LEVEL,
};
pub use variation::{DashaVariationConfig, SubPeriodMethod, YoginiScheme};
pub use yogini::{
    yogini_child_period, yogini_children, yogini_complete_level, yogini_hierarchy, yogini_level0,
    yogini_level0_entity, yogini_snapshot,
};
pub use yogini_data::{YoginiDashaConfig, yogini_config, yogini_graha, yogini_name};

// Rashi-based dasha re-exports
pub use chakra::{BirthPeriod, chakra_hierarchy, chakra_level0, chakra_snapshot};
pub use chara::{chara_hierarchy, chara_level0, chara_period_years, chara_snapshot};
pub use driga::{driga_hierarchy, driga_level0, driga_snapshot};
pub use kendradi::{
    karaka_kendradi_graha_hierarchy, karaka_kendradi_graha_snapshot, karaka_kendradi_hierarchy,
    karaka_kendradi_snapshot, kendradi_hierarchy, kendradi_level0, kendradi_snapshot,
};
pub use mandooka::{mandooka_hierarchy, mandooka_level0, mandooka_snapshot};
pub use rashi_strength::RashiDashaInputs;
pub use shoola::{shoola_hierarchy, shoola_level0, shoola_snapshot};
pub use sthira::{sthira_hierarchy, sthira_level0, sthira_snapshot};
pub use yogardha::{yogardha_hierarchy, yogardha_level0, yogardha_snapshot};
