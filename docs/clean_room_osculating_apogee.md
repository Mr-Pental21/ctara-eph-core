# Clean-Room Documentation: Moving Osculating Apogee

## Scope

Dhruv exposes heliocentric moving osculating apogee longitudes for Mangal,
Buddh, Guru, Shukra, and Shani. Surya, Chandra, Rahu, and Ketu are
intentionally not valid endpoint inputs.

The endpoint reports:

- `reference_plane_longitude`: aphelion longitude on the requested
  reference plane before ayanamsha subtraction.
- `ayanamsha_deg`: ayanamsha used for the sidereal conversion.
- `sidereal_longitude`: normalized reference-plane longitude minus ayanamsha.

## Algorithm

For each supported graha, query its heliocentric ICRF/J2000 state vector at the
requested TDB epoch:

```text
r = position vector of graha relative to Surya
v = velocity vector of graha relative to Surya
h = r x v
e = (v x h) / mu_sun - r / |r|
apogee_vector = -e
```

The aphelion/anti-periapsis vector is rotated to the configured reference plane:

- Ecliptic: ICRF/J2000 to ecliptic/J2000, then precessed to ecliptic-of-date.
- Invariable: ICRF/J2000 to the invariable plane.

The resulting vector is converted to spherical longitude and normalized to
`[0, 360)`. The selected ayanamsha is then subtracted using the same
ayanamsha, nutation, precession model, and reference-plane semantics as
`graha_longitudes`.

The same state-vector recovery helper also derives a modern osculating mean
longitude for Cheshta Bala:

```text
L_mean = longitude_of_periapsis + mean_anomaly
L_aphelion = longitude_of_periapsis + 180°
```

`L_mean` is an instantaneous osculating mean longitude, not a traditional
epoch-and-daily-motion table value. Cheshta Bala maps this modern mean longitude
through the interior/exterior correction model documented in
`docs/clean_room_shadbala.md`; it does not consume `L_aphelion` directly.

Bound instantaneous conics use the anti-periapsis direction from the
eccentricity vector. Zero-length eccentricity vectors and non-bound osculating
states are treated as non-convergent input.

## Provenance

The vector formula is the standard two-body osculating eccentricity-vector
definition from classical orbital mechanics.

The implementation follows the same clean-room concept as NAIF's public
`oscelt_c` documentation: osculating conic elements are derived from an inertial
state vector and a primary-body gravitational parameter. JPL SPK kernels provide
state vectors; they do not provide these classical elements as the primary stored
data.

The interior/exterior correction-model context for Cheshta Bala is based on the
public IITGN paper "The traditional Indian planetary model and its revision by
Nilakantha Somayaji", which describes the different treatment of Mercury/Venus
and Mars/Jupiter/Saturn in the traditional correction model.

The solar gravitational parameter used by the implementation is:

```text
mu_sun = 132712440000 km^3/s^2
```

This is the IAU 2015 nominal solar mass parameter. The value is
published as part of IAU Resolution B3 nominal conversion constants and is a
standards constant, not copied from ephemeris or astrology source code.

No denylisted/source-available astrology implementation was referenced or
derived for this feature.
