//! Dasha variation configuration: sub-period method selection per level.

/// How child periods are divided within a parent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SubPeriodMethod {
    /// child_duration = (child_full_period / total_period) * parent_duration.
    /// Sequence starts from parent's entity in the cyclic order.
    ProportionalFromParent = 0,
    /// child_duration = parent_duration / num_children.
    /// Sequence starts from entity after parent.
    EqualFromNext = 1,
    /// child_duration = parent_duration / num_children.
    /// Sequence starts from parent's entity.
    EqualFromSame = 2,
    /// Proportional but sequence starts from next entity after parent.
    ProportionalFromNext = 3,
}

impl SubPeriodMethod {
    /// Create from raw u8 value.
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::ProportionalFromParent),
            1 => Some(Self::EqualFromNext),
            2 => Some(Self::EqualFromSame),
            3 => Some(Self::ProportionalFromNext),
            _ => None,
        }
    }
}

/// Yogini dasha scheme variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum YoginiScheme {
    #[default]
    Default = 0,
    LaDeepanshuGiri = 1,
}

impl YoginiScheme {
    /// Create from raw u8 value.
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Default),
            1 => Some(Self::LaDeepanshuGiri),
            _ => None,
        }
    }
}

/// Per-level variation overrides.
///
/// Array indices 0-4 correspond to DashaLevel 0-4.
/// Index i controls how children of level-i are generated.
/// Index 4 (Pranadasha) is reserved (it has no children).
#[derive(Debug, Clone, Copy)]
pub struct DashaVariationConfig {
    /// Override sub-period method per level.
    /// None = use system default for that level.
    pub level_methods: [Option<SubPeriodMethod>; 5],
    /// Yogini scheme variant.
    pub yogini_scheme: YoginiScheme,
    /// For Ashtottari: use 28-nakshatra Abhijit detection.
    pub use_abhijit: bool,
}

impl Default for DashaVariationConfig {
    fn default() -> Self {
        Self {
            level_methods: [None; 5],
            yogini_scheme: YoginiScheme::Default,
            use_abhijit: true,
        }
    }
}

impl DashaVariationConfig {
    /// Get effective sub-period method for a level, with system default fallback.
    pub fn method_for_level(&self, level: u8, system_default: SubPeriodMethod) -> SubPeriodMethod {
        if level <= 4 {
            self.level_methods[level as usize].unwrap_or(system_default)
        } else {
            system_default
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sub_period_method_from_u8() {
        assert_eq!(
            SubPeriodMethod::from_u8(0),
            Some(SubPeriodMethod::ProportionalFromParent)
        );
        assert_eq!(SubPeriodMethod::from_u8(4), None);
    }

    #[test]
    fn yogini_scheme_from_u8() {
        assert_eq!(YoginiScheme::from_u8(0), Some(YoginiScheme::Default));
        assert_eq!(YoginiScheme::from_u8(2), None);
    }

    #[test]
    fn default_variation_uses_system_default() {
        let cfg = DashaVariationConfig::default();
        assert_eq!(
            cfg.method_for_level(0, SubPeriodMethod::ProportionalFromParent),
            SubPeriodMethod::ProportionalFromParent,
        );
    }

    #[test]
    fn override_works() {
        let mut cfg = DashaVariationConfig::default();
        cfg.level_methods[1] = Some(SubPeriodMethod::EqualFromNext);
        assert_eq!(
            cfg.method_for_level(1, SubPeriodMethod::ProportionalFromParent),
            SubPeriodMethod::EqualFromNext,
        );
    }
}
