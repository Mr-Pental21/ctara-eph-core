//! Types for panchang classification results.

use dhruv_time::UtcTime;
use dhruv_vedic_base::{
    Ayana, Hora, Karana, Masa, Nakshatra, Paksha, Samvatsara, Tithi, Vaar, Yoga,
};

/// Masa (lunar month) classification result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MasaInfo {
    /// The masa (lunar month).
    pub masa: Masa,
    /// Whether this is an adhika (intercalary) month.
    pub adhika: bool,
    /// Start of the masa (previous new moon).
    pub start: UtcTime,
    /// End of the masa (next new moon).
    pub end: UtcTime,
}

/// Ayana (solstice period) classification result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AyanaInfo {
    /// The ayana (Uttarayana or Dakshinayana).
    pub ayana: Ayana,
    /// Start of this ayana period (Sankranti).
    pub start: UtcTime,
    /// End of this ayana period (next Sankranti).
    pub end: UtcTime,
}

/// Varsha (year in 60-year cycle) classification result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VarshaInfo {
    /// The samvatsara name.
    pub samvatsara: Samvatsara,
    /// Order in the 60-year cycle (1-60).
    pub order: u8,
    /// Start of the Vedic year (Chaitra Pratipada).
    pub start: UtcTime,
    /// End of the Vedic year (next Chaitra Pratipada).
    pub end: UtcTime,
}

/// Tithi (lunar day) classification result with start/end times.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TithiInfo {
    /// The tithi.
    pub tithi: Tithi,
    /// 0-based tithi index (0..29).
    pub tithi_index: u8,
    /// Paksha (Shukla or Krishna).
    pub paksha: Paksha,
    /// 1-based tithi number within the paksha (1-15).
    pub tithi_in_paksha: u8,
    /// Start of this tithi (UTC).
    pub start: UtcTime,
    /// End of this tithi (UTC).
    pub end: UtcTime,
}

/// Karana (half-tithi) classification result with start/end times.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KaranaInfo {
    /// The karana name.
    pub karana: Karana,
    /// 0-based karana sequence index within the synodic month (0..59).
    pub karana_index: u8,
    /// Start of this karana (UTC).
    pub start: UtcTime,
    /// End of this karana (UTC).
    pub end: UtcTime,
}

/// Yoga (luni-solar yoga) classification result with start/end times.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct YogaInfo {
    /// The yoga.
    pub yoga: Yoga,
    /// 0-based yoga index (0..26).
    pub yoga_index: u8,
    /// Start of this yoga (UTC).
    pub start: UtcTime,
    /// End of this yoga (UTC).
    pub end: UtcTime,
}

/// Vaar (weekday) classification result with sunrise boundaries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VaarInfo {
    /// The vaar (weekday).
    pub vaar: Vaar,
    /// Start of this Vedic day (sunrise).
    pub start: UtcTime,
    /// End of this Vedic day (next sunrise).
    pub end: UtcTime,
}

/// Hora (planetary hour) classification result with start/end times.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HoraInfo {
    /// The hora lord (planet).
    pub hora: Hora,
    /// 0-based hora index within the Vedic day (0..23).
    pub hora_index: u8,
    /// Start of this hora (UTC).
    pub start: UtcTime,
    /// End of this hora (UTC).
    pub end: UtcTime,
}

/// Ghatika classification result with start/end times.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GhatikaInfo {
    /// Ghatika value (1-60).
    pub value: u8,
    /// Start of this ghatika (UTC).
    pub start: UtcTime,
    /// End of this ghatika (UTC).
    pub end: UtcTime,
}

/// Moon's nakshatra classification result with start/end times.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanchangNakshatraInfo {
    /// The nakshatra.
    pub nakshatra: Nakshatra,
    /// 0-based nakshatra index (0=Ashwini .. 26=Revati).
    pub nakshatra_index: u8,
    /// Pada (quarter) within the nakshatra, 1-4.
    pub pada: u8,
    /// Start of this nakshatra (UTC).
    pub start: UtcTime,
    /// End of this nakshatra (UTC).
    pub end: UtcTime,
}

/// Combined daily panchang: all seven elements for a single moment,
/// with optional calendar elements (masa, ayana, varsha).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanchangInfo {
    /// Tithi (lunar day).
    pub tithi: TithiInfo,
    /// Karana (half-tithi).
    pub karana: KaranaInfo,
    /// Yoga (luni-solar yoga).
    pub yoga: YogaInfo,
    /// Vaar (Vedic weekday).
    pub vaar: VaarInfo,
    /// Hora (planetary hour).
    pub hora: HoraInfo,
    /// Ghatika (1-60 division of Vedic day).
    pub ghatika: GhatikaInfo,
    /// Moon's nakshatra.
    pub nakshatra: PanchangNakshatraInfo,
    /// Masa (lunar month). Present when `include_calendar` is true.
    pub masa: Option<MasaInfo>,
    /// Ayana (solstice period). Present when `include_calendar` is true.
    pub ayana: Option<AyanaInfo>,
    /// Varsha (60-year samvatsara). Present when `include_calendar` is true.
    pub varsha: Option<VarshaInfo>,
}
