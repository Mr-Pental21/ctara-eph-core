# Clean-Room Documentation: Arudha Padas

## Algorithm Provenance

Arudha Pada ("reflected image") is a Jaimini concept from the Jaimini Sutras
and further elaborated in BPHS (Brihat Parashara Hora Shastra). The formulas
are standard across multiple public Vedic astrology textbooks.

## Source Material

1. **Jaimini Sutras** — Original description of arudha computation
2. **BPHS (Brihat Parashara Hora Shastra)** — Extended treatment of all 12 padas
3. **Hart de Fouw & Robert Svoboda, "Light on Life"** — Modern English exposition
4. **P.S. Sastri, "Jaimini Sutras"** — Annotated translation with examples

## Core Formula

For each bhava (house 1-12):

1. **Arc**: Compute the arc from bhava cusp to its lord:
   `arc = normalize_360(lord_longitude - cusp_longitude)`

2. **Projection**: Project the same arc forward from the lord:
   `arudha = normalize_360(lord_longitude + arc)`

3. **Exception rule**: If the arudha falls in the same rashi as the bhava cusp
   OR in the 7th rashi from it, take the 10th from the result:
   `arudha = normalize_360(arudha + 270)`

## The 12 Arudha Padas

| Index | Pada | Abbreviation | Bhava |
|-------|------|-------------|-------|
| 0 | Arudha Lagna | A1/AL | 1st |
| 1 | Dhana Pada | A2 | 2nd |
| 2 | Vikrama Pada | A3 | 3rd |
| 3 | Matri Pada | A4 | 4th |
| 4 | Mantra Pada | A5 | 5th |
| 5 | Roga Pada | A6 | 6th |
| 6 | Dara Pada | A7 | 7th |
| 7 | Mrityu Pada | A8 | 8th |
| 8 | Pitri Pada | A9 | 9th |
| 9 | Rajya Pada | A10 | 10th |
| 10 | Labha Pada | A11 | 11th |
| 11 | Upapada | A12/UL | 12th |

## Prerequisites

- **Bhava cusps** (sidereal): from `compute_bhavas()` with ayanamsha subtracted
- **Rashi lordship**: standard mapping from `rashi_lord_by_index()`
- **Graha longitudes**: from `graha_longitudes()` with sidereal config for lord positions

## Exception Rule Rationale

The exception prevents the arudha from falling in its own house or opposite house,
as this would create a degenerate reflection. The "10th from result" rule is
universally accepted across Jaimini tradition commentaries.

## Implementation Notes

- All longitudes in sidereal degrees [0, 360)
- `normalize_360()` handles wrap-around at 0/360 boundary
- Rashi index = floor(longitude / 30) → 0-based (0=Mesha, 11=Meena)
- 7th from rashi: `(rashi_index + 6) % 12`
- 10th from arudha: add 270° (= 9 signs × 30°)
- Orchestration: cusps come from bhava computation (tropical), converted to sidereal
  by subtracting ayanamsha

## Verification

Unit tests cover:
- Basic arc projection without exception
- Exception trigger on same-rashi
- Exception trigger on 7th-from-rashi
- Wrap-around at 360° boundary
- All 12 padas computed for equal-house example
