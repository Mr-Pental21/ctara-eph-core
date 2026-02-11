# Clean-Room Documentation: Ashtakavarga

## Algorithm Provenance

Ashtakavarga is a Vedic jyotish system that assigns benefic points to zodiac
signs based on the relative positions of 7 grahas (Sun through Saturn) and the
Ascendant (Lagna). The rules are standard across classical texts.

## Source Material

1. **BPHS (Brihat Parashara Hora Shastra)** — Primary source for all rules
2. **Jataka Parijata** — Confirms the standard offset tables
3. **B.V. Raman, "Graha and Bhava Balas"** — Modern exposition with worked examples

## System Overview

### Bhinna Ashtakavarga (BAV)

For each of 7 target grahas, 8 contributors (7 grahas + Lagna) assign
benefic points based on which rashi offsets from the contributor's position
are considered favorable. Offsets are 1-based (1 = same sign, 2 = next sign,
..., 12 = previous sign).

Each combination of target graha × contributor has a fixed list of favorable
offsets defined in BPHS. A point is awarded to a rashi if the offset from
the contributor to that rashi matches any favorable offset.

### Mathematical Invariants

The total points per graha across all 12 rashis is constant for all charts
(depends only on the rules, not on planetary positions):

| Graha    | Total |
|----------|-------|
| Sun      | 48    |
| Moon     | 49    |
| Mars     | 39    |
| Mercury  | 54    |
| Jupiter  | 56    |
| Venus    | 52    |
| Saturn   | 39    |

SAV total (sum of all BAVs): **337** (constant for all charts).

### Sarvashtakavarga (SAV)

SAV is the sum of all 7 BAV tables, giving a combined benefic score
per rashi (range: 0-56 per rashi).

### Trikona Sodhana

Reduces SAV by subtracting the minimum value within each element triangle:

- Fire:  Mesha (0), Simha (4), Dhanu (8)
- Earth: Vrishabha (1), Kanya (5), Makara (9)
- Air:   Mithuna (2), Tula (6), Kumbha (10)
- Water: Karka (3), Vrischika (7), Meena (11)

For each group, find the minimum and subtract it from all three members.

### Ekadhipatya Sodhana

Further reduces the Trikona-reduced SAV by subtracting the minimum within
same-lord sign pairs that were NOT already in the same trikona group:

- Mercury: Mithuna (2), Kanya (5)
- Jupiter: Dhanu (8), Meena (11)

Mars (Mesha/Vrischika), Venus (Vrishabha/Tula), and Saturn (Makara/Kumbha)
pairs are in different trikona groups and were already reduced. Sun and Moon
each rule a single sign, so no pairs exist.

## Implementation Notes

- Rules encoded as bitmasks (u16) for efficient offset checking
- No floating-point involved — all operations are integer (u8/u16)
- Pure math: only needs rashi indices (0-11) for 7 grahas + Lagna
- Orchestration function queries engine for sidereal positions at a date

## Verification

Unit tests verify:
- BAV totals match invariants for multiple chart positions
- SAV total = 337 for arbitrary positions
- Trikona and Ekadhipatya sodhana produce correct reductions
- BAV point values are in valid range (0-8 per rashi)
- Rules table bit counts match expected BAV totals
