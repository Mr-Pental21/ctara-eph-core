# Clean-Room Documentation: IAU 2000B Nutation Model

## Overview

This document establishes the provenance of the IAU 2000B nutation
implementation in `crates/dhruv_frames/src/nutation.rs`.

## Source Material

### Primary Source

**IERS Conventions (2010)**, IERS Technical Note No. 36.
- Chapter 5: "Transformation between the Celestial and Terrestrial Reference Systems"
- Table 5.2e: Fundamental arguments of nutation (Delaunay variables)
- Table 5.3b: IAU 2000B lunisolar nutation coefficients (77 terms)

This is a public-domain IAU standard published by the International Earth
Rotation and Reference Systems Service. Available at:
https://www.iers.org/IERS/EN/Publications/TechnicalNotes/tn36.html

### Supplementary Reference

**Capitaine, N., Wallace, P.T., & Chapront, J.** (2003).
"Expressions for IAU 2000 precession quantities."
*Astronomy & Astrophysics*, 412, 567-586.

## Implementation Details

### Fundamental Arguments (Delaunay Variables)

Five fundamental arguments are computed as polynomials in T (Julian
centuries of TDB since J2000.0):

| Symbol | Name                                      |
|--------|-------------------------------------------|
| l      | Mean anomaly of the Moon                  |
| l'     | Mean anomaly of the Sun                   |
| F      | Mean argument of latitude of the Moon     |
| D      | Mean elongation of the Moon from the Sun  |
| Omega  | Mean longitude of ascending node of Moon  |

Polynomial coefficients taken directly from IERS 2010 Table 5.2e.
Units: arcseconds, converted to radians internally.

### Nutation Series

77 lunisolar terms from Table 5.3b. Each term has:
- Integer multipliers for the 5 Delaunay arguments
- Sine amplitude (S_i + S'_i * T) for nutation in longitude (Delta psi)
- Cosine amplitude (C_i + C'_i * T) for nutation in obliquity (Delta epsilon)

Amplitudes stored in units of 0.1 microarcsecond (1e-7 arcsec).

### Fixed Offset Corrections

The IAU 2000B model includes small fixed offsets to approximate the
full IAU 2000A model:
- Delta psi offset: -0.135 milliarcseconds (-0.000135 arcsec)
- Delta epsilon offset: -0.388 milliarcseconds (-0.000388 arcsec)

### Solar Radius

For solar semidiameter computation in rise/set calculations:
- **IAU 2015 Resolution B3**: Nominal solar radius R_N = 696,000 km
- This is a defined constant (not measured), public domain.

## Accuracy

The IAU 2000B model provides ~1 milliarcsecond accuracy compared to
the full IAU 2000A model (which has 1365 terms). This is more than
sufficient for:
- Ayanamsha computation (nutation contribution is ~0.005 degrees max)
- Sunrise/sunset calculations (timing uncertainty from nutation is <0.01s)

## Clean-Room Statement

This implementation was derived directly from the published IAU/IERS
standards listed above. No code from any copyleft or source-available
implementation was consulted. The nutation coefficients are reproduced
from the public-domain IERS Technical Note tables.
