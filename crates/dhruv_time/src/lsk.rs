//! NAIF Leapseconds Kernel (LSK) text file parser.
//!
//! Parses the `\begindata` section of an LSK file to extract the
//! DELTET/* variables needed for UTC ↔ TDB conversion.
//!
//! Reference: NAIF Time Required Reading (public domain, US Government work product).
//! Implementation is original, written from the public specification.

use crate::error::TimeError;
use crate::julian::{calendar_to_jd, jd_to_tdb_seconds, month_from_abbrev};

/// Parsed contents of an LSK file.
#[derive(Debug, Clone)]
pub struct LskData {
    /// TT - TAI offset in seconds (DELTET/DELTA_T_A).
    pub delta_t_a: f64,
    /// Amplitude of dominant TDB-TT term in seconds (DELTET/K).
    pub k: f64,
    /// Earth orbital eccentricity for Kepler equation (DELTET/EB).
    pub eb: f64,
    /// Mean anomaly at J2000.0 in radians (DELTET/M[0]).
    pub m0: f64,
    /// Mean anomaly rate in rad/s (DELTET/M[1]).
    pub m1: f64,
    /// Leap second table: (delta_AT, epoch_tdb_seconds_past_j2000), sorted by epoch.
    pub leap_seconds: Vec<(f64, f64)>,
}

/// Parse an LSK file from its text content.
pub fn parse_lsk(content: &str) -> Result<LskData, TimeError> {
    // Extract the data section between \begindata and \begintext (or EOF).
    let data_text = extract_data_section(content)?;

    // Parse variable assignments from the data section.
    let pool = parse_kernel_pool(&data_text)?;

    let delta_t_a = get_scalar(&pool, "DELTET/DELTA_T_A")?;
    let k = get_scalar(&pool, "DELTET/K")?;
    let eb = get_scalar(&pool, "DELTET/EB")?;

    let m_vals = pool
        .get("DELTET/M")
        .ok_or_else(|| TimeError::LskParse("missing DELTET/M".into()))?;
    if m_vals.len() < 2 {
        return Err(TimeError::LskParse("DELTET/M needs 2 values".into()));
    }

    let delta_at_vals = pool
        .get("DELTET/DELTA_AT")
        .ok_or_else(|| TimeError::LskParse("missing DELTET/DELTA_AT".into()))?;

    let leap_seconds = build_leap_table(delta_at_vals)?;

    Ok(LskData {
        delta_t_a,
        k,
        eb,
        m0: m_vals[0],
        m1: m_vals[1],
        leap_seconds,
    })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Pool of named variables: name → list of f64 values.
type KernelPool = std::collections::HashMap<String, Vec<f64>>;

/// Find text between `\begindata` and `\begintext` (or EOF).
fn extract_data_section(content: &str) -> Result<String, TimeError> {
    let mut in_data = false;
    let mut data_lines = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("\\begindata") {
            in_data = true;
            continue;
        }
        if trimmed.eq_ignore_ascii_case("\\begintext") {
            in_data = false;
            continue;
        }
        if in_data {
            data_lines.push(line);
        }
    }

    if data_lines.is_empty() {
        return Err(TimeError::LskParse("no \\begindata section found".into()));
    }

    Ok(data_lines.join("\n"))
}

/// Parse NAIF kernel pool variable assignments.
///
/// Handles scalar and array values, Fortran `D` exponent notation,
/// and `@YYYY-MON-DD` date literals.
fn parse_kernel_pool(text: &str) -> Result<KernelPool, TimeError> {
    let mut pool = KernelPool::new();

    // Join continuation lines and split on variable names.
    // Strategy: find "NAME = VALUE" patterns where VALUE may span lines.
    let mut current_name: Option<String> = None;
    let mut current_values: Vec<f64> = Vec::new();
    let mut in_array = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Check for new assignment: "NAME = VALUE" or "NAME = ( VALUES )"
        if let Some(eq_pos) = trimmed.find('=') {
            // Save previous variable if any.
            if let Some(name) = current_name.take() {
                pool.insert(name, std::mem::take(&mut current_values));
                in_array = false;
            }

            let name = trimmed[..eq_pos].trim().to_string();
            let rhs = trimmed[eq_pos + 1..].trim();

            current_name = Some(name);
            current_values.clear();

            // Check if RHS starts with '('
            let rhs = if let Some(stripped) = rhs.strip_prefix('(') {
                in_array = true;
                stripped
            } else {
                rhs
            };

            // Check if RHS ends with ')'
            let rhs = if let Some(stripped) = rhs.strip_suffix(')') {
                in_array = false;
                stripped
            } else {
                rhs
            };

            parse_values(rhs, &mut current_values)?;
        } else if in_array {
            // Continuation line of an array value.
            let line_data = if let Some(stripped) = trimmed.strip_suffix(')') {
                in_array = false;
                stripped
            } else {
                trimmed
            };
            parse_values(line_data, &mut current_values)?;
        }
    }

    // Save last variable.
    if let Some(name) = current_name {
        pool.insert(name, current_values);
    }

    Ok(pool)
}

/// Parse whitespace/comma-separated values from a line.
///
/// Handles:
/// - Regular floats: `32.184`
/// - Fortran D notation: `1.657D-3` → `1.657e-3`
/// - Date literals: `@1972-JAN-1` → seconds past J2000
fn parse_values(text: &str, out: &mut Vec<f64>) -> Result<(), TimeError> {
    for token in text.split([' ', ',', '\t']).filter(|t| !t.is_empty()) {
        if let Some(date_str) = token.strip_prefix('@') {
            out.push(parse_naif_date(date_str)?);
        } else {
            // Replace Fortran 'D' exponent with 'E'.
            let normalized = token.replace('D', "E").replace('d', "e");
            let val: f64 = normalized
                .parse()
                .map_err(|e| TimeError::LskParse(format!("cannot parse '{token}' as f64: {e}")))?;
            out.push(val);
        }
    }
    Ok(())
}

/// Parse a NAIF date literal like `1972-JAN-1` into TDB seconds past J2000.
fn parse_naif_date(s: &str) -> Result<f64, TimeError> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return Err(TimeError::LskParse(format!("bad date literal: @{s}")));
    }

    let year: i32 = parts[0]
        .parse()
        .map_err(|_| TimeError::LskParse(format!("bad year in @{s}")))?;
    let month = month_from_abbrev(parts[1])
        .ok_or_else(|| TimeError::LskParse(format!("bad month in @{s}")))?;
    let day: f64 = parts[2]
        .parse()
        .map_err(|_| TimeError::LskParse(format!("bad day in @{s}")))?;

    let jd = calendar_to_jd(year, month, day);
    Ok(jd_to_tdb_seconds(jd))
}

/// Build the leap second table from the flat DELTET/DELTA_AT array.
///
/// The array is pairs: [delta_at_1, epoch_1, delta_at_2, epoch_2, ...].
fn build_leap_table(flat: &[f64]) -> Result<Vec<(f64, f64)>, TimeError> {
    if !flat.len().is_multiple_of(2) {
        return Err(TimeError::LskParse(
            "DELTET/DELTA_AT must have even number of values".into(),
        ));
    }

    let table: Vec<(f64, f64)> = flat.chunks(2).map(|pair| (pair[0], pair[1])).collect();

    Ok(table)
}

fn get_scalar(pool: &KernelPool, name: &str) -> Result<f64, TimeError> {
    let vals = pool
        .get(name)
        .ok_or_else(|| TimeError::LskParse(format!("missing {name}")))?;
    if vals.is_empty() {
        return Err(TimeError::LskParse(format!("{name} has no values")));
    }
    Ok(vals[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_LSK: &str = r#"
\begintext
Some comments here.
\begindata

DELTET/DELTA_T_A       =   32.184
DELTET/K               =    1.657D-3
DELTET/EB              =    1.671D-2
DELTET/M               = (  6.239996   1.99096871D-7  )

DELTET/DELTA_AT        = ( 10,   @1972-JAN-1
                           11,   @1972-JUL-1
                           37,   @2017-JAN-1  )

\begintext
"#;

    #[test]
    fn parse_sample_lsk() {
        let data = parse_lsk(SAMPLE_LSK).expect("should parse");
        assert!((data.delta_t_a - 32.184).abs() < 1e-10);
        assert!((data.k - 1.657e-3).abs() < 1e-15);
        assert!((data.eb - 1.671e-2).abs() < 1e-15);
        assert!((data.m0 - 6.239996).abs() < 1e-10);
        assert!((data.m1 - 1.99096871e-7).abs() < 1e-20);
        assert_eq!(data.leap_seconds.len(), 3);
        assert!((data.leap_seconds[0].0 - 10.0).abs() < 1e-10);
        assert!((data.leap_seconds[2].0 - 37.0).abs() < 1e-10);
    }

    #[test]
    fn naif_date_1972_jan_1() {
        let s = parse_naif_date("1972-JAN-1").unwrap();
        // 1972-Jan-01 00:00 = JD 2441317.5
        // seconds past J2000 = (2441317.5 - 2451545.0) * 86400
        let expected = (2_441_317.5 - 2_451_545.0) * 86_400.0;
        assert!((s - expected).abs() < 1.0, "got {s}, expected {expected}");
    }

    #[test]
    fn naif_date_2017_jan_1() {
        let s = parse_naif_date("2017-JAN-1").unwrap();
        // 2017-Jan-01 00:00 = JD 2457754.5
        let expected = (2_457_754.5 - 2_451_545.0) * 86_400.0;
        assert!((s - expected).abs() < 1.0, "got {s}, expected {expected}");
    }
}
