# Clean-Room Documentation: Special Lagnas

## Overview

8 special ascendant variants used in Vedic jyotish. All core formulas are
pure arithmetic on sidereal longitudes and time offsets.

## Time-Based Lagnas

These advance from the Sun's longitude at rates proportional to ghatikas
(24-minute periods) elapsed since sunrise.

### 1. Bhava Lagna
`BL = (Sun + ghatikas * 6) % 360`

Advances 1 sign (30 deg) per 5 ghatikas.
Source: BPHS, Jaimini Sutras.

### 2. Hora Lagna
`HL = (Sun + ghatikas * 12) % 360`

Advances 1 sign per 2.5 ghatikas.
Source: BPHS, Jaimini Sutras.

### 3. Ghati Lagna
`GL = (Sun + ghatikas * 30) % 360`

Advances 1 sign per ghatika.
Source: BPHS, Jaimini Sutras.

### 4. Vighati Lagna
`VL = (Lagna + vighatikas * 0.5) % 360`

Based on birth lagna, advances at vighati rate (60 vighatis per ghatika).
Source: BPHS.

## Composite Lagnas

### 5. Varnada Lagna

Parity-based combination of Lagna and Hora Lagna (rashi 1-12):
- Both odd rashis: add longitudes
- Both even rashis: add complements (360 - longitude)
- Lagna odd, Hora even: absolute difference
- Lagna even, Hora odd: 360 - absolute difference

Source: Jaimini Sutras, BPHS commentary.

### 6. Pranapada Lagna

`base = (Sun + ghatikas * 120) % 360`

Then adjusted by Sun's rashi type:
- Movable (1,4,7,10): no addition
- Fixed (2,5,8,11): +240 deg
- Dual (3,6,9,12): +120 deg

Source: BPHS, Jaimini Sutras.

## Moon-Based Lagna

### 7. Sree Lagna

Moon's fraction within its current nakshatra, scaled to 360 deg, added to Lagna:
```
nakshatra_span = 360/27 = 13.333... deg
fraction = (Moon % nakshatra_span) / nakshatra_span
Sree = (Lagna + fraction * 360) % 360
```

Source: BPHS, Jaimini commentary.

## Wealth Lagna

### 8. Indu Lagna

Uses kaksha (portion) values assigned to each planet:
- Sun=30, Moon=16, Mars=6, Mercury=8, Jupiter=10, Venus=12, Saturn=1

Formula:
1. Find Lagna lord and 9th lord from Moon
2. total = lagna_lord_kaksha + moon_9th_lord_kaksha
3. remainder = total % 12; if 0, use 12
4. Indu = Moon + (remainder - 1) * 30 deg

Source: Jataka Parijata, Uttara Kalamrita.

## Orchestration

The engine-dependent function `special_lagnas_for_date()`:
1. Queries Sun and Moon tropical longitudes via `body_ecliptic_lon_lat()`
2. Subtracts ayanamsha for sidereal positions
3. Computes Lagna via `lagna_longitude_rad()` (Meeus Ch. 13)
4. Gets sunrise pair via `vedic_day_sunrises()` for ghatika calculation
5. Determines lords via `rashi_lord_by_index()` for Indu Lagna
6. Delegates to pure-math `all_special_lagnas()`

## Algorithm Provenance

| Component | Source |
|---|---|
| Time-based lagnas (BL, HL, GL, VL) | BPHS, Jaimini Sutras |
| Varnada Lagna (parity rules) | Jaimini Sutras |
| Pranapada Lagna (sign-type) | BPHS, Jaimini Sutras |
| Sree Lagna (nakshatra fraction) | BPHS |
| Indu Lagna (kaksha values) | Jataka Parijata, Uttara Kalamrita |
| Ghatikas computation | Standard Vedic time division (60 per day) |
| Ascendant formula | Meeus, Astronomical Algorithms Ch. 13 |
