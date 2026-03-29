# Clean-Room Documentation: Lunar Nodes (Rahu/Ketu)

## What is Computed

Ecliptic longitude of the Moon's ascending node (Rahu) and descending node
(Ketu = Rahu + 180 deg), in both mean and true modes.

## Sources

### Mean Node (Omega polynomial)

**Source:** IERS Conventions 2010, Table 5.2e — "Fundamental arguments of
nutation," fifth argument (mean longitude of the ascending node).

- Public-domain IAU standard.
- Polynomial: Omega(T) = 450160.398036 - 6962890.5431*T + 7.4722*T^2
  + 0.007702*T^3 - 0.00005939*T^4 (arcseconds).
- T = Julian centuries of TDB since J2000.0.
- Already implemented in `dhruv_frames::nutation::fundamental_arguments()`
  as the 5th Delaunay argument. We reuse it directly (made `pub`).

### True Node (perturbation corrections — pure-math path)

**Source:** Self-derived coefficients via black-box matching pursuit against
the osculating node output of our own DE442s engine (1900–2100, every 3 days,
~24 350 data points).

- 50 sin+cos terms, each a linear combination of the five Delaunay arguments
  (IERS 2010).
- Argument combinations identified by exhaustive projection over
  |nl|≤3, |nl'|≤2, |nF|≤4, |nD|≤4, |nΩ|≤2 (~14 000 candidates).
- Iterative matching pursuit: fit the largest term, subtract, repeat.
- RMS residual ≈ 5″, max residual ≈ 20″ over the fitting interval.
- Largest term: -1.498 deg * sin(-2F + 2D), period ≈ 173 days (half of the
  eclipse year, the dominant oscillation of the osculating node).
- No external coefficient tables or GPL/copyleft code were used; the
  amplitudes are entirely self-derived from our own engine output.

### True Node (osculating state-vector mode)

**Sources:** Standard celestial mechanics geometry (public domain / textbook):

- Orbital angular momentum vector: `h = r × v`
- Ascending node direction in the reference ecliptic plane: `N = k × h`
  where `k = (0, 0, 1)`
- Node longitude: `lambda_node = atan2(Ny, Nx)`

Implementation uses:

- Moon geocentric state (`r`, `v`) from the JPL SPK queried through
  `dhruv_core::Engine` in ICRF/J2000.
- Frame rotation to J2000 ecliptic.
- 3D ecliptic precession (`precess_ecliptic_j2000_to_date_with_model`) to
  express the orbital normal in ecliptic-of-date coordinates before extracting
  the node longitude.
- The precession model is explicit in model-aware APIs and defaults to the
  crate default model (`dhruv_frames::DEFAULT_PRECESSION_MODEL`) in wrapper APIs.

### Supplementary Reference

**Source:** Chapront-Touze, M. & Chapront, J. (1991). "Lunar Tables and
Programs from 4000 B.C. to A.D. 8000." Willmann-Bell.

- ELP 2000-82 series, from which the Delaunay argument polynomials derive.
- Peer-reviewed, published research.

## Implementation Notes

- Mean node: convert `fundamental_arguments(t)[4]` from radians to degrees,
  normalize to [0, 360).
- True node:
  - Pure-math API (`lunar_node_deg`): mean + 50-term sin/cos perturbation
    series (fitted against DE442s osculating node).
  - Engine-aware API (`lunar_node_deg_for_epoch`): osculating node from
    Moon state vector geometry.
- Ketu: always Rahu + 180 deg (exact geometric relationship).
- All outputs normalized to [0, 360).
- Mean mode requires no kernel files (pure mathematical computation).
- Osculating true mode requires kernel-backed Moon state queries.
- **Default mode is True** (`NodeMode::True`), matching standard Vedic/jyotish
  practice. The jyotish longitude pipeline (`graha_longitudes` with sidereal config) uses true nodes.
  Mean nodes remain available for research/comparison via the `NodeMode` parameter.

## Denylisted Sources

No code from Swiss Ephemeris, IMCCE closed-source implementations, or any
GPL/AGPL-licensed software was consulted during implementation.
