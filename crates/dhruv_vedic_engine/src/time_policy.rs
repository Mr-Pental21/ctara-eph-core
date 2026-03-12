//! Global UTC->TDB time-conversion policy for `dhruv_vedic_base`.

use dhruv_time::TimeConversionPolicy;
use std::sync::{LazyLock, RwLock};

static TIME_POLICY: LazyLock<RwLock<TimeConversionPolicy>> =
    LazyLock::new(|| RwLock::new(TimeConversionPolicy::default()));

/// Set crate-level UTC->TDB conversion policy used by internal date->TDB conversions.
pub fn set_time_conversion_policy(policy: TimeConversionPolicy) {
    match TIME_POLICY.write() {
        Ok(mut guard) => *guard = policy,
        Err(poisoned) => {
            let mut guard = poisoned.into_inner();
            *guard = policy;
        }
    }
}

/// Current crate-level UTC->TDB conversion policy.
pub fn time_conversion_policy() -> TimeConversionPolicy {
    match TIME_POLICY.read() {
        Ok(guard) => *guard,
        Err(poisoned) => *poisoned.into_inner(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_roundtrip_set_get() {
        let p = TimeConversionPolicy::StrictLsk;
        set_time_conversion_policy(p);
        assert_eq!(time_conversion_policy(), p);
    }
}
