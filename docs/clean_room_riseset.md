# Clean-Room Record: Sunrise/Sunset Module

## IERS Earth Orientation Parameters (EOP)

**Data file**: IERS finals2000A.all
- Public domain (intergovernmental data, IERS Rapid Service/Prediction Center)
- Available from https://datacenter.iers.org/ and https://maia.usno.navy.mil/ser7/
- Format specification: IERS Technical Note 36 (public)

**Implementation**: `dhruv_time::eop::EopData::parse_finals()` parses the
fixed-width format (MJD from col 8-15, DUT1 from col 59-68) with linear
interpolation between daily values.

---

## Earth Rotation Angle (ERA)

**Source**: IERS Conventions (2010), Equation 5.15.
Public domain (IAU/IERS standard).

**Formula**:
```
theta(JD_UT1) = 2*pi * (0.7790572732640 + 1.00273781191135448 * Du)
Du = JD_UT1 - 2451545.0
```

**Implementation**: `dhruv_time::sidereal::earth_rotation_angle_rad()`

---

## Greenwich Mean Sidereal Time (GMST)

**Source**: Capitaine, N., Wallace, P.T. & Chapront, J. (2003).
Table 2. Public domain (IAU standard).

**Formula**: GMST = ERA + polynomial(T) where the polynomial coefficients
are published in the cited paper.

**Implementation**: `dhruv_time::sidereal::gmst_rad()`

---

## Sunrise/Sunset Algorithm

**Source**: Standard astronomical spherical trigonometry formulas, published
in numerous public-domain textbooks:

- Meeus, J. "Astronomical Algorithms" (Willmann-Bell) — Chapter 15
- US Naval Observatory publications (public domain, US government)
- Montenbruck, O. & Pfleger, T. "Astronomy on the Personal Computer"

**This is an original implementation** from the fundamental spherical
astronomy formulas, not a port of any existing codebase.

### Hour Angle Formula

```
cos(H0) = [sin(h0) - sin(phi) * sin(delta)] / [cos(phi) * cos(delta)]
```

Where:
- H0 = hour angle at target altitude
- h0 = target altitude (negative for below horizon)
- phi = observer latitude
- delta = Sun declination

### Atmospheric Refraction

Standard values:
- Atmospheric refraction at horizon: 34 arcmin (configurable)
- Solar angular semi-diameter: 16 arcmin (configurable)
- Total depression for sunrise/sunset: 50 arcmin = 0.8333 deg

### Geometric Dip

For elevated observers:
```
dip = sqrt(2 * h / R_earth) radians
```

Where R_earth = 6,371,000 m (IAU nominal).

### Twilight Depression Angles

Standard IAU definitions:
- Civil: 6 deg
- Nautical: 12 deg
- Astronomical: 18 deg

### Iterative Refinement

The algorithm iterates up to 5 times, recomputing the Sun's position at
each refined estimate, converging to ~0.1 second (1e-6 days).

---

## Denylisted Sources NOT Referenced

- Swiss Ephemeris (GPL)
- Any GPL/AGPL/copyleft sunrise/sunset implementations
- No source code from any denylisted project was inspected

## Validation

Golden tests compare against:
- USNO Solar Calculator (black-box I/O comparison)
- Known polar behavior (Tromso: midnight sun in June, polar night in December)
- New Delhi equinox sunrise/sunset times

All comparisons are black-box I/O validation against published values
from authoritative sources.
