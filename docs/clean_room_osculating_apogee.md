# Clean-Room Documentation: Moving Osculating Apogee

## Scope

Dhruv exposes geocentric moving osculating apogee longitudes for Mangal, Buddh,
Guru, Shukra, and Shani. Surya, Chandra, Rahu, and Ketu are intentionally not
valid endpoint inputs.

The endpoint reports:

- `reference_plane_longitude`: anti-periapsis longitude on the requested
  reference plane before ayanamsha subtraction.
- `ayanamsha_deg`: ayanamsha used for the sidereal conversion.
- `sidereal_longitude`: normalized reference-plane longitude minus ayanamsha.

## Algorithm

For each supported graha, query its geocentric ICRF/J2000 state vector at the
requested TDB epoch:

```text
r = position vector of graha relative to Earth
v = velocity vector of graha relative to Earth
h = r x v
e = (v x h) / mu_earth - r / |r|
apogee_vector = -e
```

The anti-periapsis vector is rotated to the configured reference plane:

- Ecliptic: ICRF/J2000 to ecliptic/J2000, then precessed to ecliptic-of-date.
- Invariable: ICRF/J2000 to the invariable plane.

The resulting vector is converted to spherical longitude and normalized to
`[0, 360)`. The selected ayanamsha is then subtracted using the same
ayanamsha, nutation, precession model, and reference-plane semantics as
`graha_longitudes`.

For instantaneous non-bound conics, Dhruv still reports the anti-periapsis
direction from the eccentricity vector. A zero-length eccentricity vector is
treated as non-convergent input.

## Provenance

The vector formula is the standard two-body osculating eccentricity-vector
definition from classical orbital mechanics.

The Earth gravitational parameter used by the implementation is:

```text
mu_earth = 398600.435436 km^3/s^2
```

This is the IAU 2015 nominal Earth gravitational parameter. The value is
published as part of IAU Resolution B3 nominal conversion constants and is a
standards constant, not copied from ephemeris or astrology source code.

No denylisted/source-available astrology implementation was referenced or
derived for this feature.
