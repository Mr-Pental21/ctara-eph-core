# Clean-Room Documentation: Dasha (Planetary Period) Calculations

## Overview

Dashas are hierarchical time-period systems from Vedic astrology that divide a person's
life into planetary periods. This implementation covers 23 dasha systems described in
Brihat Parashara Hora Shastra (BPHS).

## Phase 18a: Core Types + Vimshottari

### Sources

- **BPHS**: Brihat Parashara Hora Shastra, Chapters 46-53 (dasha systems)
- **Lahiri's Tables of Ascendants**: Reference for Vimshottari dasha calculations
- **K.S. Krishnamurti**: Stellar astrology, Vimshottari period calculations
- **B.V. Raman**: Hindu Predictive Astrology, Chapter on Dashas

### Vimshottari Dasha System

**Sequence and Periods** (BPHS Ch.46):

| Graha | Period (years) | Total |
|-------|---------------|-------|
| Ketu | 7 | 7 |
| Shukra | 20 | 27 |
| Surya | 6 | 33 |
| Chandra | 10 | 43 |
| Mangal | 7 | 50 |
| Rahu | 18 | 68 |
| Guru | 16 | 84 |
| Shani | 19 | 103 |
| Buddh | 17 | 120 |

Total cycle: 120 years.

**Nakshatra-to-Graha mapping**: Each of the 27 nakshatras maps to a graha lord.
Every 3rd nakshatra shares the same lord (9 grahas × 3 = 27):

- Ketu: Ashwini(0), Magha(9), Mula(18)
- Shukra: Bharani(1), P.Phalguni(10), P.Ashadha(19)
- Surya: Krittika(2), U.Phalguni(11), U.Ashadha(20)
- Chandra: Rohini(3), Hasta(12), Shravana(21)
- Mangal: Mrigashira(4), Chitra(13), Dhanishtha(22)
- Rahu: Ardra(5), Swati(14), Shatabhisha(23)
- Guru: Punarvasu(6), Vishakha(15), P.Bhadrapada(24)
- Shani: Pushya(7), Anuradha(16), U.Bhadrapada(25)
- Buddh: Ashlesha(8), Jyeshtha(17), Revati(26)

### Birth Balance Algorithm

The birth balance determines how much of the first mahadasha remains at birth:

```
nakshatra_span = 360° / 27 = 13.3333°
nakshatra_index = floor(moon_sidereal_lon / nakshatra_span)
position_in_nakshatra = moon_sidereal_lon mod nakshatra_span
elapsed_fraction = position_in_nakshatra / nakshatra_span
balance_days = graha_period_days × (1 - elapsed_fraction)
```

The remaining 8 mahadashas follow in sequence after the partial first period,
each at their full duration.

### Sub-Period (Antardasha) Calculation

**Proportional from Parent** (default for Vimshottari, BPHS Ch.46):

Within each parent period, sub-periods are generated for all 9 grahas in the
cyclic sequence starting from the parent's graha:

```
parent_duration = parent.end_jd - parent.start_jd
For each child_graha in cyclic_sequence(starting_from=parent_graha):
    child_duration = (child_full_period / total_cycle_period) × parent_duration
```

The last child's end_jd is snapped to the parent's end_jd to absorb
floating-point drift.

### Hierarchical Levels

| Level | Name | Depth |
|-------|------|-------|
| 0 | Mahadasha | Top-level |
| 1 | Antardasha | Sub-period |
| 2 | Pratyantardasha | Sub-sub-period |
| 3 | Sookshmadasha | 4th level |
| 4 | Pranadasha | 5th level (finest) |

Each deeper level applies the same proportional sub-period algorithm recursively.

### Interval Convention

- Periods use `[start_jd, end_jd)` — start is inclusive, end is exclusive
- Adjacent periods share boundaries: `period[n].end_jd == period[n+1].start_jd`
- No gaps, no overlaps

### Time Constants

- `DAYS_PER_YEAR = 365.25` (Julian year, standard astronomical convention)
- All times are JD UTC (calendar Julian Date, not TDB)

### Safety Limits

- `MAX_DASHA_LEVEL = 4` (levels 0-4)
- `MAX_PERIODS_PER_LEVEL = 100,000` (prevents runaway allocation)
- At depth 4, Vimshottari produces 9^5 = 59,049 periods (within limit)

### Snapshot-Only Path

For efficient deep-level queries, the snapshot path avoids materializing the
full hierarchy. Instead of generating all periods at each level, it:

1. Generates level-0 periods
2. Binary searches for the active period at query_jd
3. Generates children of only that active period
4. Repeats until max_level

Complexity: O(depth × sequence_length) instead of O(sequence_length^depth).

## Phase 18b: Remaining Nakshatra-Based Systems + Yogini

### Sources

Same BPHS sources as Phase 18a, plus:
- **BPHS Ch.47**: Ashtottari Dasha
- **BPHS Ch.48**: Shodsottari Dasha
- **BPHS Ch.49**: Dwadashottari Dasha
- **BPHS Ch.50-53**: Panchottari, Shatabdika, Chaturashiti, Dwisaptati, Shashtihayani, Shat-Trimsha
- **B.V. Raman**: Hindu Predictive Astrology, dasha system summaries

### Nakshatra-Based Systems (10 total)

| System | Total Years | Grahas | Cycles | Starting Nakshatra | Special |
|--------|-------------|--------|--------|--------------------|---------|
| Vimshottari | 120 | 9 | 1 | Ashwini (0) | Phase 18a |
| Ashtottari | 108 | 8 (no Ketu) | 1 | Ardra (5) | Abhijit detection TBD |
| Shodsottari | 116 | 8 | 1 | Pushya (7) | Arithmetic 11-18y |
| Dwadashottari | 112 | 8 | 1 | Bharani (1) | Odd 7-21y |
| Panchottari | 105 | 7 | 1 | Anuradha (16) | Arithmetic 12-18y |
| Shatabdika | 100 | 7 | 1 | Revati (26) | Paired 5,5,10,10,20,20,30 |
| Chaturashiti | 84 | 7 | 2 | Swati (14) | Equal 12y each |
| Dwisaptati Sama | 72 | 8 | 2 | Mula (18) | Equal 9y each |
| Shashtihayani | 60 | 8 | 2 | Ashwini (0) | Period ÷ nakshatra count |
| Shat-Trimsha Sama | 36 | 8 | 3 | Shravana (21) | Arithmetic 1-8y |

### Cycle Count Logic

Systems with `cycle_count > 1` repeat the full graha sequence multiple times to
fill the total period. For example, Chaturashiti (84y, 7 grahas, 2 cycles) generates
14 mahadasha periods (7 × 2), each 12 years.

### Shashtihayani Special Balance

Unlike other systems where `entry_period = graha_full_period`, Shashtihayani divides
each graha's total period among the nakshatras assigned to that graha:

```
entry_period = graha_period / count_of_nakshatras_for_that_graha
```

This ensures the birth balance is proportional to the per-nakshatra share.

### Yogini Dasha System

**Sequence and Periods** (BPHS):

| Index | Yogini | Graha Lord | Period (years) |
|-------|--------|------------|----------------|
| 0 | Mangala | Chandra | 1 |
| 1 | Pingala | Surya | 2 |
| 2 | Dhanya | Guru | 3 |
| 3 | Bhramari | Mangal | 4 |
| 4 | Bhadrika | Buddh | 5 |
| 5 | Ulka | Shani | 6 |
| 6 | Siddha | Shukra | 7 |
| 7 | Sankata | Rahu | 8 |

Total cycle: 36 years. 8 yoginis.

**Nakshatra-to-Yogini mapping**:

Formula: `yogini_idx = ((nakshatra_1_indexed + 3) % 8)`, where result 0 maps to index 7.

The pattern repeats every 8 nakshatras starting from Ardra (index 5) → Mangala (index 0).

**Sub-period method**: ProportionalFromParent (same as Vimshottari).

**Entity type**: Uses `DashaEntity::Yogini(u8)` (0-7) instead of `DashaEntity::Graha`.

## Phase 18c: Rashi-Based Dasha Systems (10 systems)

### Sources

- **BPHS Ch.46-53**: Rashi dasha systems (Chara, Sthira, Yogardha, Driga, Shoola, Mandooka, Chakra, Kendradi)
- **Jaimini Sutras**: Chara dasha (primary source for rashi-based period calculation)
- **B.V. Raman**: Hindu Predictive Astrology, rashi dasha descriptions
- **K.N. Rao**: Jaimini's Chara Dasha, practical applications

### Key Differences from Nakshatra-Based Systems

Rashi-based dashas differ from nakshatra-based in several ways:

1. **Input data**: Require full chart (9 graha sidereal longitudes + lagna) instead of just Moon longitude
2. **Entity type**: Use `DashaEntity::Rashi(0..11)` instead of `DashaEntity::Graha`
3. **12-period cycle**: All systems generate 12 mahadasha periods (one per rashi)
4. **Birth balance**: Based on lagna degree within its rashi (not Moon in nakshatra)
5. **Direction**: Odd signs traverse forward, even signs traverse reverse (for most systems)

### Rashi Birth Balance

```
rashi_index = floor(lagna_sidereal_lon / 30)
position_in_rashi = lagna_sidereal_lon - rashi_index * 30
elapsed_fraction = position_in_rashi / 30
balance_days = entry_period_days * (1 - elapsed_fraction)
```

### RashiDashaInputs

All rashi-based systems share a common input struct assembled by the orchestration layer:

- `graha_sidereal_lons`: 9 sidereal longitudes (Sun through Ketu)
- `lagna_sidereal_lon`: sidereal lagna longitude
- `lagna_rashi_index`: whole-sign house of lagna (0-11)
- `bhava_rashi_indices`: whole-sign house indices for all 12 bhavas

### Rashi Strength (6-Rule Hierarchy)

Several systems need to determine the "stronger" of two rashis. Rules applied in order
(first decisive rule wins):

1. **Occupant count**: More grahas present in the rashi
2. **Benefic association**: Lord aspected by/conjunct Jupiter or Mercury
3. **Exaltation proximity**: Lord closer to its exaltation degree
4. **Odd/even preference**: Odd signs preferred for odd pairs, even for even
5. **Lord longitude**: Higher sidereal longitude of the rashi lord
6. **Index fallback**: Higher rashi index

### System Details

#### Chara Dasha (Jaimini)

The most complex rashi system. Period (in years) for each rashi:

```
For odd signs: count_forward(rashi, lord_rashi) - 1
For even signs: count_reverse(rashi, lord_rashi) - 1
If result = 0: period = 12 years (lord in own sign)
```

- **Starting rashi**: Lagna rashi
- **Direction**: Odd lagna → forward, even lagna → reverse
- **Sub-period**: EqualFromNext (12 equal sub-periods starting from next rashi)
- **Total cycle**: Variable, depends on chart

#### Sthira Dasha

Fixed periods based on sign type:

| Sign Type | Examples | Period |
|-----------|----------|--------|
| Chara (Movable) | Aries, Cancer, Libra, Capricorn | 7 years |
| Sthira (Fixed) | Taurus, Leo, Scorpio, Aquarius | 8 years |
| Dvisvabhava (Dual) | Gemini, Virgo, Sagittarius, Pisces | 9 years |

- **Total cycle**: 4×7 + 4×8 + 4×9 = 96 years
- **Starting rashi**: Sign of the Brahma Graha (Venus/Jupiter/Saturn in odd sign, houses 1-7)
- **Sub-period**: ProportionalFromNext

#### Yogardha Dasha

Average of Chara and Sthira periods:

```
yogardha_period(rashi) = (chara_period(rashi) + sthira_period(rashi)) / 2
```

- **Starting rashi**: Stronger of lagna rashi or 7th house rashi
- **Sub-period**: ProportionalFromNext

#### Driga Dasha

Signs grouped by type (4 each), traversed as three groups:

```
Group 1: All 4 Chara (movable) signs
Group 2: All 4 Sthira (fixed) signs
Group 3: All 4 Dvisvabhava (dual) signs
```

Within each group, signs are traversed forward (odd start) or reverse (even start).

- **Fixed periods**: Same as Sthira (C=7/S=8/D=9), total 96 years
- **Sub-period**: ProportionalFromParent

#### Shoola Dasha

- **Fixed periods**: C=7/S=8/D=9, total 96 years
- **Starting rashi**: Stronger of 2nd house rashi or 8th house rashi
- **Sub-period**: ProportionalFromParent

#### Mandooka (Frog) Dasha

Distinctive movement pattern — jumps ±3 signs instead of sequential traversal:

```
Sequence from Mesha(0), forward: 0, 3, 6, 9, 2, 5, 8, 11, 4, 7, 10, 1
(each step adds or subtracts 3, wrapping around the zodiac)
```

- **Fixed periods**: C=7/S=8/D=9, total 96 years
- **Starting rashi**: Stronger of lagna rashi or 7th house rashi
- **Sub-period**: ProportionalFromParent

#### Chakra Dasha

Simplest rashi-based system:

- **Fixed period**: 10 years per rashi, 120 years total
- **Direction**: Always forward (regardless of sign parity)
- **Starting sign**: Depends on birth period:
  - Day birth: lagna rashi
  - Night birth: 7th from lagna
  - Twilight: 9th from lagna
- **Sub-period**: EqualFromSame (12 equal sub-periods starting from same rashi)

#### Kendradi Dasha (3 variants)

Signs traversed in Kendra→Panapara→Apoklima groups:

```
Kendra:   offsets 0, 3, 6, 9 from starting rashi
Panapara: offsets 1, 4, 7, 10
Apoklima: offsets 2, 5, 8, 11
```

Three variants differ only in starting rashi determination:

| Variant | Starting Rashi |
|---------|---------------|
| Kendradi | Stronger of lagna or 7th house |
| Karaka Kendradi | Atmakaraka's rashi |
| Karaka Kendradi Graha | Atmakaraka's rashi (graha-based sub-periods) |

- **Periods**: Chara period years (variable, chart-dependent)
- **Sub-period**: ProportionalFromParent

### Special Computations

#### Brahma Graha (for Sthira Dasha)

The Brahma Graha is determined by:
1. Consider Venus, Jupiter, Saturn only
2. Filter to those in odd signs
3. Filter to those in houses 1 through 7 (from lagna)
4. Among remaining, select the one with the highest degree-in-sign
5. Fallback: lord of the 6th house

#### Atmakaraka (for Kendradi variants)

The Atmakaraka is the graha with the highest degree within its sign (degree-in-sign),
considering only the 7 sapta grahas (Sun through Saturn, excluding Rahu and Ketu).

### Sub-Period Methods for Rashi Systems

| Method | Description |
|--------|-------------|
| ProportionalFromParent | Duration ∝ period/total, sequence starts from parent rashi |
| ProportionalFromNext | Duration ∝ period/total, sequence starts from next rashi |
| EqualFromSame | Parent duration ÷ 12, sequence starts from parent rashi |
| EqualFromNext | Parent duration ÷ 12, sequence starts from next rashi |

For all methods, the 12-rashi sub-sequence direction follows the parent's sign parity
(odd → forward, even → reverse).

## Data Provenance

All dasha sequences, periods, and algorithms are derived from:
- BPHS text (multiple translations/commentaries cross-referenced)
- Published Vimshottari tables in standard Jyotish reference works
- Jaimini Sutras for Chara dasha rashi-based period calculations
- No copyleft or proprietary source code was referenced
