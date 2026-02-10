//! IERS Earth Orientation Parameters (EOP) — UT1−UTC lookup.
//!
//! Parses the IERS finals2000A.all fixed-width format to extract daily
//! DUT1 = UT1−UTC values. Provides linear interpolation for any epoch
//! within the table range.
//!
//! Data file: IERS finals2000A.all (~3 MB, public domain).
//! Available from <https://datacenter.iers.org/> and
//! <https://maia.usno.navy.mil/ser7/>.
//!
//! File format (relevant columns, 1-indexed):
//! - Col  8-15: Modified Julian Date (F8.2)
//! - Col 58:    Flag 'I' (IERS final) or 'P' (prediction)
//! - Col 59-68: UT1−UTC in seconds (F10.7)

use crate::error::TimeError;

/// Parsed IERS Earth Orientation Parameters (DUT1 lookup table).
#[derive(Debug, Clone)]
pub struct EopData {
    /// Daily (MJD, DUT1) pairs, sorted ascending by MJD.
    entries: Vec<(f64, f64)>,
}

impl EopData {
    /// Parse IERS finals2000A.all fixed-width format.
    ///
    /// Extracts MJD (col 8-15) and DUT1 (col 59-68) from each line.
    /// Lines with blank or unparseable DUT1 fields are skipped.
    pub fn parse_finals(content: &str) -> Result<Self, TimeError> {
        let mut entries = Vec::new();

        for line in content.lines() {
            let bytes = line.as_bytes();
            // Need at least 68 characters for DUT1 field
            if bytes.len() < 68 {
                continue;
            }

            // Col 8-15 (0-indexed: 7..15): MJD
            let mjd_str = &line[7..15];
            let mjd: f64 = match mjd_str.trim().parse() {
                Ok(v) => v,
                Err(_) => continue,
            };

            // Col 59-68 (0-indexed: 58..68): UT1-UTC
            let dut1_str = &line[58..68];
            let dut1: f64 = match dut1_str.trim().parse() {
                Ok(v) => v,
                Err(_) => continue,
            };

            entries.push((mjd, dut1));
        }

        if entries.is_empty() {
            return Err(TimeError::EopParse(
                "no valid DUT1 entries found".to_string(),
            ));
        }

        // Ensure sorted by MJD
        entries.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        Ok(Self { entries })
    }

    /// Number of entries in the table.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the table is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// MJD range covered by the table: (first, last).
    pub fn range(&self) -> (f64, f64) {
        (self.entries[0].0, self.entries[self.entries.len() - 1].0)
    }

    /// DUT1 (UT1−UTC) in seconds at a given MJD, linearly interpolated.
    pub fn dut1_at_mjd(&self, mjd: f64) -> Result<f64, TimeError> {
        let (mjd_start, mjd_end) = self.range();
        if mjd < mjd_start || mjd > mjd_end {
            return Err(TimeError::EopOutOfRange);
        }

        // Binary search for the bracketing interval
        let idx = self
            .entries
            .partition_point(|&(m, _)| m < mjd)
            .saturating_sub(1);

        // Exact match or last entry
        if idx + 1 >= self.entries.len() {
            return Ok(self.entries[idx].1);
        }

        let (m0, d0) = self.entries[idx];
        let (m1, d1) = self.entries[idx + 1];

        if (m1 - m0).abs() < 1e-12 {
            return Ok(d0);
        }

        // Linear interpolation
        let frac = (mjd - m0) / (m1 - m0);
        Ok(d0 + frac * (d1 - d0))
    }

    /// Convert a UTC Julian Date to a UT1 Julian Date.
    ///
    /// `jd_ut1 = jd_utc + dut1 / 86400.0`
    pub fn utc_to_ut1_jd(&self, jd_utc: f64) -> Result<f64, TimeError> {
        let mjd = jd_utc - 2_400_000.5;
        let dut1 = self.dut1_at_mjd(mjd)?;
        Ok(jd_utc + dut1 / 86_400.0)
    }
}

/// Loaded IERS EOP file, ready for UT1 conversions.
///
/// Follows the same load-from-file pattern as [`crate::LeapSecondKernel`].
#[derive(Debug, Clone)]
pub struct EopKernel {
    data: EopData,
}

impl EopKernel {
    /// Load a finals2000A.all file from disk.
    pub fn load(path: &std::path::Path) -> Result<Self, TimeError> {
        let content = std::fs::read_to_string(path)?;
        Self::parse(&content)
    }

    /// Parse finals2000A.all from string content.
    pub fn parse(content: &str) -> Result<Self, TimeError> {
        let data = EopData::parse_finals(content)?;
        Ok(Self { data })
    }

    /// Access the parsed EOP data.
    pub fn data(&self) -> &EopData {
        &self.data
    }

    /// Convert a UTC Julian Date to a UT1 Julian Date.
    pub fn utc_to_ut1_jd(&self, jd_utc: f64) -> Result<f64, TimeError> {
        self.data.utc_to_ut1_jd(jd_utc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A minimal hand-crafted finals2000A.all snippet (3 lines).
    // Format: cols 1-6=date, 7=blank, 8-15=MJD, ... 58=flag, 59-68=DUT1
    // We pad to 68+ chars with the DUT1 value at the right position.
    fn test_snippet() -> String {
        // MJD 60000.00, DUT1 = +0.1234567 s
        // MJD 60001.00, DUT1 = +0.2345678 s
        // MJD 60002.00, DUT1 = -0.1000000 s
        let mut lines = Vec::new();
        for &(mjd, dut1) in &[
            (60000.00, 0.1234567),
            (60001.00, 0.2345678),
            (60002.00, -0.1000000),
        ] {
            // Build a 68-char line with MJD at col 8-15 and DUT1 at col 59-68
            let mut line = vec![b' '; 70];
            let mjd_s = format!("{mjd:8.2}");
            line[7..15].copy_from_slice(mjd_s.as_bytes());
            line[57] = b'I'; // col 58: flag
            let dut1_s = format!("{dut1:10.7}");
            line[58..68].copy_from_slice(dut1_s.as_bytes());
            lines.push(String::from_utf8(line).unwrap());
        }
        lines.join("\n")
    }

    #[test]
    fn parse_small_snippet() {
        let data = EopData::parse_finals(&test_snippet()).unwrap();
        assert_eq!(data.len(), 3);
        let (start, end) = data.range();
        assert!((start - 60000.0).abs() < 0.01);
        assert!((end - 60002.0).abs() < 0.01);
    }

    #[test]
    fn interpolation_exact() {
        let data = EopData::parse_finals(&test_snippet()).unwrap();
        let dut1 = data.dut1_at_mjd(60000.0).unwrap();
        assert!((dut1 - 0.1234567).abs() < 1e-7);
    }

    #[test]
    fn interpolation_midpoint() {
        let data = EopData::parse_finals(&test_snippet()).unwrap();
        let dut1 = data.dut1_at_mjd(60000.5).unwrap();
        let expected = (0.1234567 + 0.2345678) / 2.0;
        assert!(
            (dut1 - expected).abs() < 1e-7,
            "midpoint: got {dut1}, expected {expected}"
        );
    }

    #[test]
    fn out_of_range_before() {
        let data = EopData::parse_finals(&test_snippet()).unwrap();
        assert_eq!(data.dut1_at_mjd(59999.0), Err(TimeError::EopOutOfRange));
    }

    #[test]
    fn out_of_range_after() {
        let data = EopData::parse_finals(&test_snippet()).unwrap();
        assert_eq!(data.dut1_at_mjd(60003.0), Err(TimeError::EopOutOfRange));
    }

    #[test]
    fn blank_dut1_skipped() {
        // Line with blank DUT1 field should be skipped
        let mut snippet = test_snippet();
        // Add a line with blank DUT1
        let mut blank_line = vec![b' '; 70];
        let mjd_s = format!("{:8.2}", 60003.00);
        blank_line[7..15].copy_from_slice(mjd_s.as_bytes());
        // col 59-68 left as spaces (blank DUT1)
        snippet.push('\n');
        snippet.push_str(&String::from_utf8(blank_line).unwrap());

        let data = EopData::parse_finals(&snippet).unwrap();
        // Should still have only 3 entries (blank line skipped)
        assert_eq!(data.len(), 3);
    }

    #[test]
    fn utc_to_ut1_offset() {
        let data = EopData::parse_finals(&test_snippet()).unwrap();
        // Verify DUT1 at first entry directly
        let dut1 = data.dut1_at_mjd(60000.0).unwrap();
        assert!((dut1 - 0.1234567).abs() < 1e-7, "dut1 = {dut1}");
        // Verify JD conversion applies the offset correctly
        let jd_utc = 2_460_000.5;
        let jd_ut1 = data.utc_to_ut1_jd(jd_utc).unwrap();
        // Offset should be dut1/86400 ≈ 1.43e-6 days
        assert!(
            (jd_ut1 - jd_utc).abs() < 1e-5,
            "UT1 offset too large: {}",
            jd_ut1 - jd_utc
        );
        assert!(jd_ut1 > jd_utc, "UT1 should be ahead of UTC when DUT1 > 0");
    }
}
