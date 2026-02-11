//! Types for Vedic jyotish orchestration (graha longitudes, etc.).

use dhruv_vedic_base::Graha;

/// Sidereal longitudes of all 9 grahas.
#[derive(Debug, Clone, Copy)]
pub struct GrahaLongitudes {
    /// Sidereal longitudes indexed by `Graha::index()` (0-8).
    pub longitudes: [f64; 9],
}

impl GrahaLongitudes {
    /// Get the sidereal longitude for a specific graha.
    pub fn longitude(&self, graha: Graha) -> f64 {
        self.longitudes[graha.index() as usize]
    }

    /// Get the 0-based rashi index (0-11) for a specific graha.
    pub fn rashi_index(&self, graha: Graha) -> u8 {
        let lon = self.longitude(graha);
        ((lon / 30.0).floor() as u8).min(11)
    }

    /// Get rashi indices for all 9 grahas.
    pub fn all_rashi_indices(&self) -> [u8; 9] {
        let mut indices = [0u8; 9];
        for (i, &lon) in self.longitudes.iter().enumerate() {
            indices[i] = ((lon / 30.0).floor() as u8).min(11);
        }
        indices
    }
}
