# Clean-Room Documentation: Upagrahas

## Algorithm Provenance

Upagrahas ("shadow planets") are sensitive mathematical points used in Vedic
jyotish. Their formulas are standard across BPHS (Brihat Parashara Hora
Shastra) and other classical texts.

## Source Material

1. **BPHS (Brihat Parashara Hora Shastra)** — Primary source for all 11 upagrahas
2. **Jataka Parijata** — Confirms sun-based chain and portion divisions
3. **K.S. Charak, "Subtleties of Medical Astrology"** — Modern exposition of Gulika/Maandi

## The 11 Upagrahas

### Sun-based (5) — Pure math chain from Sun longitude

| # | Name | Formula |
|---|------|---------|
| 1 | Dhooma | Sun + 133°20' |
| 2 | Vyatipata | 360° - Dhooma |
| 3 | Parivesha | Vyatipata + 180° |
| 4 | Indra Chapa | 360° - Parivesha |
| 5 | Upaketu | Indra Chapa + 16°40' |

All results normalized to [0°, 360°).

### Time-based (6) — Lagna at a planet's portion of day/night

| # | Name | Planet | Lagna at |
|---|------|--------|----------|
| 1 | Gulika | Rahu | Start of portion |
| 2 | Maandi | Rahu | End of portion |
| 3 | Kaala | Sun | Start of portion |
| 4 | Mrityu | Mars | Start of portion |
| 5 | Artha Prahara | Mercury | Start of portion |
| 6 | Yama Ghantaka | Jupiter | Start of portion |

## Portion Division System

Day (sunrise to sunset) and night (sunset to next sunrise) are each divided
into 8 equal temporal portions. Each portion is ruled by a planet in the
Chaldean sequence: Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn, Rahu.

### Day portions

The sequence starts with the weekday ruler:
- Sunday: Sun(0), Moon(1), Mars(2), Mercury(3), Jupiter(4), Venus(5), Saturn(6), Rahu(7)
- Monday: Moon, Mars, Mercury, Jupiter, Venus, Saturn, Rahu, Sun
- Tuesday: Mars, Mercury, Jupiter, Venus, Saturn, Rahu, Sun, Moon
- etc.

Formula: `day_portion_index(weekday, planet) = (planet - weekday + 8) % 8`

### Night portions

The sequence starts from an offset planet (weekday + 4) % 7:
- Sunday night: Jupiter(4), Venus, Saturn, Rahu, Sun, Moon, Mars, Mercury
- Monday night: Venus(5), Saturn, Rahu, Sun, Moon, Mars, Mercury, Jupiter
- etc.

Formula: `night_start = (weekday + 4) % 7; night_portion_index = (planet - night_start + 8) % 8`

### Weekday convention

Uses Vaar convention: 0=Sunday (Ravivaar) through 6=Saturday (Shanivaar).
Planet indices: 0=Sun, 1=Moon, 2=Mars, 3=Mercury, 4=Jupiter, 5=Venus, 6=Saturn, 7=Rahu.

## Computation Steps

For time-based upagrahas:
1. Get sunrise, sunset, next sunrise for the vedic day
2. Determine if birth is day (sunrise ≤ t < sunset) or night
3. Compute weekday from sunrise JD
4. For each upagraha: get portion index → compute portion JD range → evaluate
   ascendant (lagna) at portion start (or end for Maandi)
5. Convert tropical lagna to sidereal by subtracting ayanamsha

For sun-based upagrahas:
1. Get Sun's sidereal longitude
2. Apply chain formula

## Implementation Notes

- All longitudes are sidereal degrees [0, 360)
- Portion indices are computed algorithmically (no lookup tables needed)
- Lagna uses `lagna_longitude_rad()` from the lagna module
- Vedic day boundaries from `vedic_day_sunrises()` (existing infrastructure)
- Sunset computed via `compute_rise_set()` with `RiseSetEvent::Sunset`

## Verification

Unit tests verify:
- Enum indexing and naming for all 11 upagrahas
- Time-based classification
- Sun-based chain calculation (algebraic identity checks)
- Day/night portion indices match independently derived lookup tables
- Portion JD range division into 8 equal parts
