# Clean-Room Provenance: Stationary Point & Max-Speed Search

## Feature
Detection of planetary stationary points (retrograde/direct stations) and
peak-speed events via geocentric ecliptic longitude velocity analysis.

## Algorithm Description

### Stationary Points
A geocentric stationary point occurs when a planet's ecliptic longitude
velocity crosses zero. The algorithm:

1. **Coarse scan**: step through time evaluating the ecliptic longitude
   speed (deg/day) at each point. Detect sign changes in the speed.
2. **Bisection refinement**: when a sign change is found between two
   adjacent samples, bisect the interval to converge on the zero crossing.
3. **Classification**: speed positive→negative = StationRetrograde (planet
   begins apparent backward motion), negative→positive = StationDirect
   (planet resumes forward motion).

### Max-Speed Events
A max-speed event occurs when the planet's ecliptic longitude acceleration
crosses zero (velocity reaches a local extremum). The algorithm:

1. **Numerical acceleration**: computed via central difference
   `a(t) = (v(t+h) - v(t-h)) / (2h)` with `h = 0.01` days.
2. **Coarse scan**: step through time evaluating acceleration. Detect
   sign changes.
3. **Bisection refinement**: refine the zero crossing of acceleration.
4. **Classification**: speed > 0 at extremum = MaxDirect (peak forward
   speed), speed < 0 = MaxRetrograde (peak retrograde speed).

### Velocity Pipeline
The ecliptic longitude speed is obtained from the existing engine pipeline:
- Chebyshev polynomial evaluation yields Cartesian position and velocity
- Engine applies ICRF→Ecliptic rotation to both position and velocity
  vectors when `Frame::EclipticJ2000` is requested
- `cartesian_state_to_spherical_state()` converts to spherical coordinates
  including `lon_speed` in rad/s
- Final conversion: `deg/day = lon_speed × (180/π) × 86400`

## Sources

- **Numerical bisection**: standard numerical root-finding method, textbook
  material (e.g., Burden & Faires, "Numerical Analysis").
- **Central difference**: standard numerical differentiation formula,
  O(h²) accuracy.
- **Retrograde motion**: standard geocentric observational astronomy. A
  planet appears to move retrograde when Earth overtakes it (superior
  planets) or it overtakes Earth (inferior planets). This is apparent
  motion only, caused by the relative orbital velocities.
- **Ecliptic longitude velocity**: derivative of the standard ecliptic
  longitude coordinate, a direct output of the spherical coordinate
  transformation applied to Cartesian state vectors.

## What Was NOT Referenced

- No Swiss Ephemeris code or algorithms
- No Astro.com code
- No GPL/AGPL/copyleft implementations
- Classification and body validation rules derived from first principles
  of orbital mechanics (Sun and Moon never retrograde geocentrically)

## Validation

Golden values compared against widely published retrograde dates for
Mercury and Mars (public astronomical almanac data, e.g., USNO/HMNAO).
