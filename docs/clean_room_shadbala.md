# Clean-Room Documentation: Shadbala (Six-Fold Planetary Strength)

## Source

All formulas derived from **Brihat Parashara Hora Shastra (BPHS)**, chapters on
planetary strength. Constants cross-checked against standard Jyotish textbooks
(Graha & Bhava Balas by B.V. Raman, freely published tables).

No code-level reference to any copyleft implementation.

## Scope

**Sapta grahas only** (Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn).
Rahu and Ketu are excluded — BPHS does not define Shadbala for shadow planets.

## 1. Sthana Bala (Positional Strength)

### 1a. Uchcha Bala (Exaltation Strength)

Formula: `60 × (1 − |distance_from_exaltation| / 180)`

Exaltation degrees (BPHS ch.3):

| Graha   | Exaltation | Degree |
|---------|-----------|--------|
| Sun     | 10° Aries | 10°    |
| Moon    | 3° Taurus | 33°    |
| Mars    | 28° Capricorn | 298° |
| Mercury | 15° Virgo | 165°  |
| Jupiter | 5° Cancer | 95°   |
| Venus   | 27° Pisces | 357° |
| Saturn  | 20° Libra | 200°  |

Debilitation = exaltation + 180° mod 360°.

### 1a.1 Moolatrikona And Own-House Portions

When a planet's own sign also contains a moolatrikona or exaltation portion,
only the specified degree span receives that dignity:

| Graha | Sign | Portion |
|-------|------|---------|
| Sun | Leo | 0°-20° moolatrikona; 20°-30° own house |
| Moon | Taurus | 0°-3° exaltation; 3°-30° moolatrikona |
| Mars | Aries | 0°-12° moolatrikona; 12°-30° own house |
| Mercury | Virgo | 0°-15° exaltation; 15°-20° moolatrikona; 20°-30° own house |
| Jupiter | Sagittarius | 0°-10° moolatrikona; 10°-30° own house |
| Venus | Libra | 0°-15° moolatrikona; 15°-30° own house |
| Saturn | Aquarius | 0°-20° moolatrikona; 20°-30° own house |

### 1b. Saptavargaja Bala (Seven-Division Dignity Strength)

Dignity in each of 7 vargas (D1, D2, D3, D7, D9, D12, D30) contributes points:

| Dignity | Points |
|---------|--------|
| Moolatrikone | 45 |
| Own Sign | 30 |
| Adhi Mitra | 22.5 |
| Mitra | 15 |
| Sama | 7.5 |
| Shatru | 3.75 |
| Adhi Shatru | 1.875 |

Sum across 7 vargas. D1/Rashi may use degree-specific moolatrikona and
own-house portions. The other vargas use own sign where applicable, otherwise
compound friendship against the varga sign lord; temporary friendship for that
compound relationship is always computed from D1 rashi positions. Exaltation
and debilitation are not separate Saptavargaja categories.

### 1c. Ojhayugma Bala (Odd/Even Sign-Navamsa Strength)

Male grahas (Sun, Mars, Jupiter) get 15 points for being in odd rashi + 15 for odd navamsa.
Female grahas (Moon, Venus) get 15 for even rashi + 15 for even navamsa.
Mercury and Saturn: 15 for odd rashi + 15 for even navamsa (or vice versa).

### 1d. Kendradi Bala (Angular House Strength)

| Bhava position | Points |
|---------------|--------|
| Kendra (1,4,7,10) | 60 |
| Panaphara (2,5,8,11) | 30 |
| Apoklima (3,6,9,12) | 15 |

### 1e. Drekkana Bala (Decanate Strength)

Male graha in 1st decanate (0-10°) = 15. Neuter in 2nd (10-20°) = 15.
Female in 3rd (20-30°) = 15. Otherwise 0.

Gender: Male = Sun, Mars, Jupiter. Female = Moon, Venus. Neuter = Mercury, Saturn.

## 2. Dig Bala (Directional Strength)

Formula: `60 × (1 − smaller_angle/180)`, where `smaller_angle` is the exact sidereal angular distance in degrees between the graha and its max-strength cusp, clamped to `[0, 180]`.

Maximum strength bhava:

| Graha | Max Bhava |
|-------|----------|
| Sun, Mars | 10 (South/MC) |
| Moon, Venus | 4 (North/IC) |
| Mercury, Jupiter | 1 (East/Asc) |
| Saturn | 7 (West/Desc) |

The cusp set is chosen by `BhavaConfig.use_rashi_bhava_for_bala_avastha`: rashi-bhava/equal-house cusps by default, or configured bhava-system cusps when explicitly disabled.

## 3. Kala Bala (Temporal Strength)

### 3a. Nathonnatha Bala (Day/Night Strength)

Malefics (Sun, Mars, Saturn) strong by day = 60, night = 0.
Benefics (Moon, Mercury, Jupiter, Venus) strong by night = 60, day = 0.
Mercury classified by Moon-Sun elongation.

### 3b. Paksha Bala (Fortnight Strength)

phase_angle = min(elongation, 360 − elongation). Range [0, 180].
Benefic paksha = phase_angle / 3 (0 at new moon, 60 at full moon).
Malefic paksha = 60 − benefic_paksha.

### 3c. Tribhaga Bala (Day/Night Third Strength)

Day: 1st third = Jupiter (60), 2nd = Mercury (60), 3rd = Saturn (60).
Night: 1st third = Moon (60), 2nd = Venus (60), 3rd = Mars (60).
Sun always gets 60.

### 3d. Lord Balas

| Component | Points | Lord of |
|----------|--------|---------|
| Abda Bala | 15 | Year (samvatsara lord) |
| Masa Bala | 30 | Month (masa lord) |
| Vara Bala | 45 | Weekday (vaar lord) |
| Hora Bala | 60 | Planetary hour (hora lord) |

### 3e. Ayana Bala (Declination Strength)

Per-graha declination formula (full BPHS, not simplified Sun-only).
Benefic: `(24 + declination_deg) / 48 × 60`.
Malefic: `(24 − declination_deg) / 48 × 60`.
Capped to [0, 60].

### 3f. Yuddha Bala (Planetary War Strength)

Only Mars, Mercury, Jupiter, Venus, Saturn participate.
War occurs when two star-planets are within 1° longitude.
Winner = planet with more northerly declination. Winner gets +60, loser gets −60.

## 4. Cheshta Bala (Motional Strength)

Mangal, Buddh, Guru, Shukra, and Shani use a modern correction-model
interpretation of the sloka's madhyama, sphuta, and chaloccha quantities. This
does not use the physical moving osculating apogee endpoint directly.

```text
mid = circular_midpoint(madhyama_longitude, sphuta_longitude)
raw_cheshta_kendra = normalize_360(chaloccha_longitude - mid)

if raw_cheshta_kendra <= 180:
    cheshta_kendra = raw_cheshta_kendra
else:
    cheshta_kendra = 360 - raw_cheshta_kendra

cheshta_bala = cheshta_kendra / 3
```

Interior/exterior mapping:

- Exterior grahas Mangal, Guru, Shani: madhyama = graha heliocentric
  osculating mean longitude, chaloccha = modern mean Sun longitude.
- Interior grahas Buddh, Shukra: madhyama = modern mean Sun longitude,
  chaloccha = graha heliocentric osculating mean longitude.

Surya and Chandra always receive 0 Cheshta Bala. Rahu and Ketu are outside the
Shadbala graha set.

The physical moving osculating apogee helper and the osculating-element
provenance used for the modern mean-longitude approximation are documented in
`docs/clean_room_osculating_apogee.md`.

## 5. Naisargika Bala (Natural Strength)

Fixed constants (BPHS):

| Graha | Shashtiamsas |
|-------|-------------|
| Sun | 60.00 |
| Moon | 51.43 |
| Mars | 17.14 |
| Mercury | 25.71 |
| Jupiter | 34.29 |
| Venus | 42.86 |
| Saturn | 8.57 |

## 6. Drik Bala (Aspectual Strength)

(benefic_virupa_sum − malefic_virupa_sum) / 4.
Uses existing virupa-based drishti system. Benefic/malefic classification from Moon-Sun elongation.

## 7. Total Shadbala

Total shashtiamsas = sum of all 6 components.
Total rupas = total shashtiamsas / 60.

Required strength (BPHS):

| Graha | Required (shashtiamsas) |
|-------|----------------------|
| Sun | 390 |
| Moon | 360 |
| Mars | 300 |
| Mercury | 420 |
| Jupiter | 390 |
| Venus | 330 |
| Saturn | 300 |

is_strong = total_shashtiamsas >= required_strength.

## Graha Relationships (Foundation)

### Naisargika Maitri (Natural Friendship)

| Graha | Friends | Enemies | Neutral |
|-------|---------|---------|---------|
| Sun | Moon, Mars, Jupiter | Venus, Saturn | Mercury |
| Moon | Sun, Mercury | — | Mars, Jupiter, Venus, Saturn |
| Mars | Sun, Moon, Jupiter | Mercury | Venus, Saturn |
| Mercury | Sun, Venus | Moon | Mars, Jupiter, Saturn |
| Jupiter | Sun, Moon, Mars | Mercury, Venus | Saturn |
| Venus | Mercury, Saturn | Sun, Moon | Mars, Jupiter |
| Saturn | Mercury, Venus | Sun, Moon, Mars | Jupiter |
| Rahu | Venus, Saturn | Sun, Moon, Mars | Jupiter, Mercury |
| Ketu | Venus, Saturn | Sun, Moon, Mars | Jupiter, Mercury |

### Tatkalika Maitri (Temporal Friendship)

Friend if other planet is in 2nd, 3rd, 4th, 10th, 11th, or 12th house from planet.
Enemy otherwise.

### Panchadha Maitri (Five-Fold Compound)

| Natural + Temporal | Result |
|-------------------|--------|
| Friend + Friend | Adhi Mitra |
| Friend + Enemy | Sama |
| Neutral + Friend | Mitra |
| Neutral + Enemy | Shatru |
| Enemy + Friend | Sama |
| Enemy + Enemy | Adhi Shatru |

### General Dignity Hierarchy

Check order: Exalted → Debilitated → Moolatrikone → Own Sign → Compound friendship.
