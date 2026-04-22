# Clean-Room: Graha Drishti (Planetary Aspects)

## What

Virupa-based graha drishti: computes how strongly each graha aspects any
sidereal point. Includes 9×9 graha-to-graha matrix, optional extensions
to bhava cusps, lagna, and core bindus.

## Sources

- **BPHS (Brihat Parashara Hora Shastra)**: planetary aspect rules, special
  aspects for Mars (4th/8th houses), Jupiter (5th/9th), Saturn (3rd/10th).
- **Jataka Parijata**: virupa strength quantification for aspects.
- **Standard Vedic jyotish convention**: piecewise virupa formula mapping
  angular separation to aspect strength.

## Algorithm

### Base Virupa (piecewise formula)

Given angular distance A = normalize_360(target - source):

| Range       | Formula                | Virupa range |
|-------------|------------------------|--------------|
| [0, 30)     | 0                         | 0            |
| [30, 60)    | (A − 30) / 2             | 0 → 15       |
| [60, 90)    | A − 45                   | 15 → 45      |
| [90, 120)   | 30 + (120 − A) / 2       | 45 → 30      |
| [120, 150)  | 150 − A                  | 30 → 0       |
| [150, 180)  | (A − 150) × 2            | 0 → 60       |
| [180, 300)  | (300 − A) × 0.5          | 60 → 0       |
| [300, 360)  | 0                         | 0            |

### Special Virupa (planet-specific bonuses)

- **Mars (Mangal)**: +15 virupa if A ∈ [90, 120) or [210, 240)
- **Jupiter (Guru)**: +30 virupa if A ∈ [120, 150) or [240, 270)
- **Saturn (Shani)**: +45 virupa if A ∈ [60, 90) or [270, 300)
- All other grahas (Sun, Moon, Mercury, Venus, Rahu, Ketu): 0

### Total Virupa

total = base_virupa + special_virupa

### Orchestration

1. Compute 9 graha sidereal longitudes
2. Build 9×9 matrix (diagonal zeroed for self-aspect)
3. Optionally: graha-to-lagna (9×1), graha-to-bhava-cusps (9×12),
   graha-to-core-bindus (9×19)

## Denylisted References

No Swiss Ephemeris or GPL/copyleft implementations were consulted.

## Verification

- Unit tests: 38 tests covering boundary values, special aspects, matrix properties
- Integration tests: 6 tests with real ephemeris data validating all config flag combinations
- Total virupa = base + special identity verified across all matrix entries
- Angular distance symmetry: d(i,j) + d(j,i) = 360° for all pairs
- Non-special grahas confirmed to have zero special_virupa everywhere
