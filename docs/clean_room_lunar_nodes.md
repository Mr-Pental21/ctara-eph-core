# Clean-Room Documentation: Lunar Nodes (Rahu/Ketu)

## What is Computed

Ecliptic longitude of the Moon's ascending node (Rahu) and descending node
(Ketu = Rahu + 180 deg), in both mean (polynomial) and true (polynomial +
short-period perturbation) modes.

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

### True Node (perturbation corrections)

**Source:** Meeus, Jean. *Astronomical Algorithms*, 2nd edition (1998),
Chapter 47 ("Position of the Moon"), specifically the apparent longitude
perturbation terms that affect the node position.

- Published textbook, widely cited in astronomical software.
- 13 sinusoidal terms, each a function of the five Delaunay arguments.
- Largest term: -1.4979 deg * sin(Omega), corresponding to the 18.6-year
  nutation period.

### Supplementary Reference

**Source:** Chapront-Touze, M. & Chapront, J. (1991). "Lunar Tables and
Programs from 4000 B.C. to A.D. 8000." Willmann-Bell.

- ELP 2000-82 series, from which the Delaunay argument polynomials derive.
- Peer-reviewed, published research.

## Implementation Notes

- Mean node: convert `fundamental_arguments(t)[4]` from radians to degrees,
  normalize to [0, 360).
- True node: mean + sum of 13 sinusoidal perturbation terms.
- Ketu: always Rahu + 180 deg (exact geometric relationship).
- All outputs normalized to [0, 360).
- No kernel files required (pure mathematical computation).
- **Default mode is True** (`NodeMode::True`), matching standard Vedic/jyotish
  practice. The jyotish pipeline (`graha_sidereal_longitudes`) uses true nodes.
  Mean nodes remain available for research/comparison via the `NodeMode` parameter.

## Denylisted Sources

No code from Swiss Ephemeris, IMCCE closed-source implementations, or any
GPL/AGPL-licensed software was consulted during implementation.
