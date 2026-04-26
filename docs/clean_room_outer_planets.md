# Clean-Room Record: Outer Graha Display Sections

## Purpose

Expose Uranus, Neptune, and Pluto as modern positional/display entities in sibling
`outer_planets` sections without changing the traditional 9-graha arrays.

## Provenance

- Positions are derived from the repository's existing JPL-kernel body support for
  `Body::Uranus`, `Body::Neptune`, and `Body::Pluto`.
- No astrology-library source or denylisted implementation was consulted.
- The implementation reuses Dhruv's existing frame, nutation, precession, and
  ayanamsha plumbing used for graha longitude display.

## Design Notes

- Existing navagraha arrays remain length 9.
- Outer planets are returned as `[Uranus, Neptune, Pluto]`.
- Outer planets are display-only in this change. They do not participate in
  Shadbala, Vimsopaka, Ashtakavarga, Avastha, Drishti, Dasha, lordship, or other
  traditional navagraha calculations.
- Amsha charts transform outer-planet D1 longitudes with the same resolved amsha
  variation as the chart, but the transformed entries remain in the sibling
  `outer_planets` section.
