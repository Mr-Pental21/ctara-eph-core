# Clean-Room: Rashi (Zodiac Sign) & Nakshatra (Lunar Mansion)

## Provenance

All formulas in `dhruv_vedic_base::rashi` and `dhruv_vedic_base::nakshatra`
are derived from universal Vedic astronomical convention (Surya Siddhanta
and standard Jyotish texts). These are pure geometric divisions of the
360-degree ecliptic circle — no copyleft or proprietary sources used.

## Rashi (12-fold Division)

The sidereal ecliptic is divided into 12 equal signs of 30 degrees each:

| Index | Sanskrit    | Western     | Start (deg) | End (deg) |
|-------|-------------|-------------|-------------|-----------|
| 0     | Mesha       | Aries       | 0           | 30        |
| 1     | Vrishabha   | Taurus      | 30          | 60        |
| 2     | Mithuna     | Gemini      | 60          | 90        |
| 3     | Karka       | Cancer      | 90          | 120       |
| 4     | Simha       | Leo         | 120         | 150       |
| 5     | Kanya       | Virgo       | 150         | 180       |
| 6     | Tula        | Libra       | 180         | 210       |
| 7     | Vrischika   | Scorpio     | 210         | 240       |
| 8     | Dhanu       | Sagittarius | 240         | 270       |
| 9     | Makara      | Capricorn   | 270         | 300       |
| 10    | Kumbha      | Aquarius    | 300         | 330       |
| 11    | Meena       | Pisces      | 330         | 360       |

### Formula

```
rashi_index = floor(sidereal_longitude / 30)
degrees_in_rashi = sidereal_longitude - rashi_index * 30
```

Where `sidereal_longitude = tropical_longitude - ayanamsha`.

### DMS Conversion

```
degrees = floor(angle)
remainder = (angle - degrees) * 60
minutes = floor(remainder)
seconds = (remainder - minutes) * 60
```

## Nakshatra — 27-Scheme (Uniform)

The ecliptic is divided into 27 equal nakshatras of 13 deg 20'
(13.3333... deg) each. Each nakshatra has 4 padas (quarters) of
3 deg 20' (3.3333... deg).

### Formula

```
NAKSHATRA_SPAN = 360 / 27 = 13.3333...
PADA_SPAN = NAKSHATRA_SPAN / 4 = 3.3333...

nakshatra_index = floor(sidereal_longitude / NAKSHATRA_SPAN)
degrees_in_nakshatra = sidereal_longitude - nakshatra_index * NAKSHATRA_SPAN
pada = floor(degrees_in_nakshatra / PADA_SPAN) + 1   (1-based, 1-4)
```

### 27 Nakshatras

| Index | Name               | Start (deg) |
|-------|--------------------|-------------|
| 0     | Ashwini            | 0.0000      |
| 1     | Bharani            | 13.3333     |
| 2     | Krittika           | 26.6667     |
| 3     | Rohini             | 40.0000     |
| 4     | Mrigashira         | 53.3333     |
| 5     | Ardra              | 66.6667     |
| 6     | Punarvasu          | 80.0000     |
| 7     | Pushya             | 93.3333     |
| 8     | Ashlesha           | 106.6667    |
| 9     | Magha              | 120.0000    |
| 10    | Purva Phalguni     | 133.3333    |
| 11    | Uttara Phalguni    | 146.6667    |
| 12    | Hasta              | 160.0000    |
| 13    | Chitra             | 173.3333    |
| 14    | Swati              | 186.6667    |
| 15    | Vishakha           | 200.0000    |
| 16    | Anuradha           | 213.3333    |
| 17    | Jyeshtha           | 226.6667    |
| 18    | Mula               | 240.0000    |
| 19    | Purva Ashadha      | 253.3333    |
| 20    | Uttara Ashadha     | 266.6667    |
| 21    | Shravana           | 280.0000    |
| 22    | Dhanishtha         | 293.3333    |
| 23    | Shatabhisha        | 306.6667    |
| 24    | Purva Bhadrapada   | 320.0000    |
| 25    | Uttara Bhadrapada  | 333.3333    |
| 26    | Revati             | 346.6667    |

## Nakshatra — 28-Scheme (With Abhijit)

When Abhijit is included, the region around Uttara Ashadha / Shravana
is divided non-uniformly:

| Index | Name            | Start (deg)   | End (deg)     | Span         |
|-------|-----------------|---------------|---------------|--------------|
| 20    | Uttara Ashadha  | 266 deg 40'   | 276 deg 40'   | 10 deg 00'   |
| 21    | Abhijit         | 276 deg 40'   | 280 deg 53'20"| 4 deg 13'20" |
| 22    | Shravana        | 280 deg 53'20"| 293 deg 20'   | 12 deg 26'40"|

All other nakshatras retain their standard 13 deg 20' span.

### Abhijit Boundaries

The boundaries are derived from the traditional assignment:
- Abhijit starts at the last quarter of Uttara Ashadha (27-scheme):
  20 * 13.3333 + 10.0 = 276.6667 deg = 276 deg 40'
- Abhijit ends at 280 deg 53' 20" (280.8889 deg), traditionally
  associated with the star Vega (alpha Lyrae).
- Shravana then runs from 280.8889 deg to 293.3333 deg.

### Pada for Abhijit

Abhijit pada is set to 0 (not applicable). The number of padas for
Abhijit is debated in Vedic tradition and not standardized. Some
authorities assign 4 padas, others none.

## Sources

- Surya Siddhanta (c. 4th century CE): defines 12 rashis and 27 nakshatras
  as equal divisions of the ecliptic.
- Standard Jyotish textbooks (Brihat Parashara Hora Shastra, Brihat
  Jataka): 12 x 30 deg = 360 deg, 27 x 13 deg 20' = 360 deg.
- Abhijit boundaries: from Atharvaveda Nakshatra lists and traditional
  astronomical texts that include the 28th nakshatra.
- No copyleft or proprietary ephemeris software was referenced.
