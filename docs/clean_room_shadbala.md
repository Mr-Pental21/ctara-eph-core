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

Sum across 7 vargas. Each varga uses its own rashi positions for temporal friendship
(not D1 positions reused).

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

Male graha in 1st decanate (0-10°) = 15. Female in 2nd (10-20°) = 15.
Neuter in 3rd (20-30°) = 15. Otherwise 0.

Gender: Male = Sun, Mars, Jupiter. Female = Moon, Venus. Neuter = Mercury, Saturn.

## 2. Dig Bala (Directional Strength)

Formula: `60 × (1 − dist/6)` where dist = min(|bhava − max_bhava|, 12 − |bhava − max_bhava|), capped at 6.

Maximum strength bhava:

| Graha | Max Bhava |
|-------|----------|
| Sun, Mars | 10 (South/MC) |
| Moon, Venus | 4 (North/IC) |
| Mercury, Jupiter | 1 (East/Asc) |
| Saturn | 7 (West/Desc) |

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

Retrograde = 60. Direct = (|speed| / max_speed) × 60, capped at 60.
Sun and Moon always = 0 (they never retrograde; their motion is handled elsewhere).

Max speeds (deg/day): Sun=1.0, Moon=15.0, Mars=0.8, Mercury=2.2, Jupiter=0.25, Venus=1.6, Saturn=0.13.

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

### Dignity Hierarchy

Check order: Exalted → Debilitated → Moolatrikone → Own Sign → Compound friendship.
