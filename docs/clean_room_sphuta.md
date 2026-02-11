# Clean-Room Documentation: Sphutas & Graha Foundation

## Overview

Phase 10a adds the Graha (Vedic planet) foundation and 16 sphuta (sensitive point)
formulas. All computations are pure arithmetic on sidereal longitudes.

## Graha Enum

Nine Vedic grahas: Surya (Sun), Chandra (Moon), Mangal (Mars), Buddh (Mercury),
Guru (Jupiter), Shukra (Venus), Shani (Saturn), Rahu, Ketu.

### Rashi Lordship

Standard Parashari lordship assignment:
- Mesha/Vrischika → Mangal
- Vrishabha/Tula → Shukra
- Mithuna/Kanya → Buddh
- Karka → Chandra
- Simha → Surya
- Dhanu/Meena → Guru
- Makara/Kumbha → Shani

Source: Brihat Parashara Hora Shastra (BPHS), universally agreed in Jyotish.

### NAIF Body Codes

Sun=10, Moon=301, Mars=499, Mercury=199, Jupiter=599, Venus=299, Saturn=699.
Rahu and Ketu are mathematical points (lunar nodes), not physical bodies.

### Kaksha Values

For Indu Lagna computation: Sun=30, Moon=16, Mars=6, Mercury=8,
Jupiter=10, Venus=12, Saturn=1.

Source: Jataka Parijata, Uttara Kalamrita.

## Sphuta Formulas

All formulas take sidereal longitudes in degrees and return degrees [0, 360).

### 1. Bhrigu Bindu
`BB = normalize(rahu + normalize(moon - rahu) / 2)`

Midpoint of the shorter arc from Rahu to Moon.
Source: Common Jyotish reference, attributed to Bhrigu.

### 2. Prana Sphuta
`PS = normalize(lagna * 5 + moon)`

Source: BPHS, Saravali.

### 3. Deha Sphuta
`DS = normalize(moon * 8 + lagna)`

Source: BPHS, Saravali.

### 4. Mrityu Sphuta
`MS = normalize(eighth_lord * 8 + lagna)`

Source: BPHS.

### 5. Tithi Sphuta (Sookshma Tithi Sphuta)
`TS = normalize(normalize(moon - sun) / 12 + lagna)`

Elongation divided by 12, offset by lagna.
Source: Uttara Kalamrita.

### 6. Yoga Sphuta
`YS = normalize(sun + moon)`

Sum of Sun and Moon longitudes.
Source: Standard panchang definition.

### 7. Yoga Sphuta Normalized
`YSN = normalize(sun + moon) mod 13.333...`

Position within the current yoga segment (13°20').

### 8. Rahu Tithi Sphuta
`RTS = normalize(normalize(rahu - sun) / 12 + lagna)`

Same as tithi sphuta but with Rahu replacing Moon.

### 9. Kshetra Sphuta
`KS = normalize(venus + moon + mars + jupiter + lagna)`

Five-body sum for fertility/marriage analysis.
Source: BPHS.

### 10. Beeja Sphuta
`BS = normalize(sun + venus + jupiter)`

Three-body sum for fertility/progeny analysis.
Source: BPHS.

### 11. Trisphuta
`TS = normalize(lagna + moon + gulika)`

Three-point sum using Gulika (a time-based upagraha).
Source: BPHS.

### 12. Chatussphuta
`CS = normalize(trisphuta + sun)`

Trisphuta plus Sun longitude.
Source: BPHS.

### 13. Panchasphuta
`PS = normalize(chatussphuta + rahu)`

Chatussphuta plus Rahu.
Source: BPHS.

### 14. Sookshma Trisphuta
`STS = normalize((lagna + moon + gulika + sun) / 4)`

Average of four points.
Source: BPHS.

### 15. Avayoga Sphuta
`AS = normalize(360 - yoga_sphuta(sun, moon))`

Complement of yoga sphuta.

### 16. Kunda
`K = normalize(lagna + moon + mars)`

Three-point sum.

## Graha Sidereal Longitudes

Engine-dependent orchestration that:
1. Queries tropical ecliptic longitude of each graha via `body_ecliptic_lon_lat()`
2. Computes Rahu/Ketu via `lunar_node_deg()` (mean or true mode)
3. Subtracts ayanamsha to get sidereal longitude
4. Normalizes all results to [0, 360)

Ketu = normalize(Rahu + 180°).

## Algorithm Provenance

| Component | Source |
|---|---|
| Graha enum & lordship | BPHS (Parashari standard) |
| Kaksha values | Jataka Parijata, Uttara Kalamrita |
| Sphuta formulas | BPHS, Saravali, Uttara Kalamrita |
| NAIF body codes | NASA/NAIF SPICE convention |
| Ayanamsha subtraction | Standard sidereal conversion |
| Lunar nodes | Mean/true node from dhruv_vedic_base |

## Implementation Notes

- All sphuta functions are `const`-friendly pure math (no engine dependency)
- `SphutalInputs` struct bundles all required longitudes for batch computation
- `all_sphutas()` returns fixed-size array `[(Sphuta, f64); 16]`
- Gulika longitude requires upagraha computation (Phase 10d); set to 0.0 until then
- `normalize_360()` centralized in `util.rs` for consistent wrapping
