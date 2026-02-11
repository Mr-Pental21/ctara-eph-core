//! Integration tests for rashi and nakshatra computation.
//!
//! Pure-math tests (no kernel files needed).

use dhruv_vedic_base::{
    AyanamshaSystem, Nakshatra, Nakshatra28, Rashi, deg_to_dms, nakshatra28_from_longitude,
    nakshatra_from_longitude, nakshatra_from_tropical, rashi_from_longitude, rashi_from_tropical,
};

// ---------------------------------------------------------------------------
// Rashi integration tests
// ---------------------------------------------------------------------------

#[test]
fn rashi_sweep_all_12() {
    let expected = [
        Rashi::Mesha,
        Rashi::Vrishabha,
        Rashi::Mithuna,
        Rashi::Karka,
        Rashi::Simha,
        Rashi::Kanya,
        Rashi::Tula,
        Rashi::Vrischika,
        Rashi::Dhanu,
        Rashi::Makara,
        Rashi::Kumbha,
        Rashi::Meena,
    ];
    for (i, r) in expected.iter().enumerate() {
        let lon = i as f64 * 30.0 + 15.0; // midpoint of each rashi
        let info = rashi_from_longitude(lon);
        assert_eq!(info.rashi, *r, "rashi at {lon} deg");
        assert_eq!(info.rashi_index, i as u8);
    }
}

#[test]
fn rashi_dms_precision() {
    // 45 deg 30' 15.5" within Vrishabha → degrees_in_rashi = 15.504306...
    let lon = 45.0 + 30.0 / 60.0 + 15.5 / 3600.0;
    let info = rashi_from_longitude(lon);
    assert_eq!(info.rashi, Rashi::Vrishabha);
    assert_eq!(info.dms.degrees, 15);
    assert_eq!(info.dms.minutes, 30);
    assert!((info.dms.seconds - 15.5).abs() < 0.01, "seconds = {}", info.dms.seconds);
}

#[test]
fn rashi_from_tropical_with_lahiri() {
    // At J2000.0 (JD 2451545.0), Lahiri ayanamsha ~23.853
    // Tropical 280.5 → sidereal ~256.647 → Dhanu (index 8, starts at 240)
    let info = rashi_from_tropical(280.5, AyanamshaSystem::Lahiri, 2_451_545.0, false);
    assert_eq!(info.rashi, Rashi::Dhanu);
    assert_eq!(info.rashi_index, 8);
    assert!((info.degrees_in_rashi - 16.647).abs() < 0.01);
}

#[test]
fn dms_round_trip() {
    // 23.853 deg → 23 deg 51' 10.8"
    let d = deg_to_dms(23.853);
    let reconstructed = d.degrees as f64 + d.minutes as f64 / 60.0 + d.seconds / 3600.0;
    assert!((reconstructed - 23.853).abs() < 1e-10, "reconstructed = {reconstructed}");
}

// ---------------------------------------------------------------------------
// Nakshatra integration tests
// ---------------------------------------------------------------------------

#[test]
fn nakshatra_sweep_all_27() {
    let span = 360.0 / 27.0;
    for i in 0..27u8 {
        let lon = i as f64 * span + span / 2.0; // midpoint
        let info = nakshatra_from_longitude(lon);
        assert_eq!(info.nakshatra_index, i, "nakshatra at {lon} deg");
    }
}

#[test]
fn nakshatra_pada_boundaries() {
    let span = 360.0 / 27.0;
    let pada_span = span / 4.0;

    // Pada 1: [0, 3.333)
    let info = nakshatra_from_longitude(1.0);
    assert_eq!(info.pada, 1);

    // Pada 2: [3.333, 6.667)
    let info = nakshatra_from_longitude(pada_span + 0.5);
    assert_eq!(info.pada, 2);

    // Pada 3: [6.667, 10.0)
    let info = nakshatra_from_longitude(2.0 * pada_span + 0.5);
    assert_eq!(info.pada, 3);

    // Pada 4: [10.0, 13.333)
    let info = nakshatra_from_longitude(3.0 * pada_span + 0.5);
    assert_eq!(info.pada, 4);
}

#[test]
fn nakshatra_from_tropical_test() {
    // Sun at J2000: tropical ~280.5, Lahiri ~23.853 → sidereal ~256.647
    // 256.647 / 13.333 = 19.25 → index 19 = Purva Ashadha
    let info = nakshatra_from_tropical(280.5, AyanamshaSystem::Lahiri, 2_451_545.0, false);
    assert_eq!(info.nakshatra, Nakshatra::PurvaAshadha);
}

// ---------------------------------------------------------------------------
// 28-scheme tests
// ---------------------------------------------------------------------------

#[test]
fn nakshatra28_abhijit_region() {
    // Abhijit spans ~276.667 to ~280.889
    let test_points = [277.0, 278.0, 279.0, 280.0, 280.5];
    for lon in test_points {
        let info = nakshatra28_from_longitude(lon);
        assert_eq!(
            info.nakshatra,
            Nakshatra28::Abhijit,
            "lon {lon} should be Abhijit"
        );
        assert_eq!(info.pada, 0, "Abhijit pada should be 0");
    }
}

#[test]
fn nakshatra28_around_abhijit() {
    // Just before Abhijit: 276.0 → Uttara Ashadha
    let info = nakshatra28_from_longitude(276.0);
    assert_eq!(info.nakshatra, Nakshatra28::UttaraAshadha);

    // Just after Abhijit: 281.0 → Shravana
    let info = nakshatra28_from_longitude(281.0);
    assert_eq!(info.nakshatra, Nakshatra28::Shravana);
}

#[test]
fn nakshatra28_non_abhijit_matches_27() {
    // For nakshatras far from the Abhijit region, 28-scheme should match 27-scheme names
    let test_lons = [5.0, 50.0, 100.0, 150.0, 200.0, 350.0];
    for lon in test_lons {
        let n27 = nakshatra_from_longitude(lon);
        let n28 = nakshatra28_from_longitude(lon);
        assert_eq!(
            n27.nakshatra.name(),
            n28.nakshatra.name(),
            "at {lon} deg: 27-scheme {} vs 28-scheme {}",
            n27.nakshatra.name(),
            n28.nakshatra.name()
        );
    }
}

// ---------------------------------------------------------------------------
// Spot-check from plan
// ---------------------------------------------------------------------------

#[test]
fn spot_check_sun_j2000() {
    // Sun at J2000: tropical ~280.5 deg
    // Lahiri ayanamsha at J2000 ~23.853 deg
    // Sidereal ~256.647 deg
    let sidereal = 280.5 - 23.853;
    let rashi = rashi_from_longitude(sidereal);
    assert_eq!(rashi.rashi, Rashi::Dhanu); // index 8, [240, 270)

    let nak = nakshatra_from_longitude(sidereal);
    // 256.647 / 13.333 = 19.25 → Purva Ashadha (index 19)
    assert_eq!(nak.nakshatra, Nakshatra::PurvaAshadha);
    // Pada: (256.647 - 253.333) / 3.333 = 0.99 → pada 1? Let's check
    // 19 * 13.333 = 253.333, offset = 3.314, pada_span = 3.333 → pada 1
    assert_eq!(nak.pada, 1);
}
