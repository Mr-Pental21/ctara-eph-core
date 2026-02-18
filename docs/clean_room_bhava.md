# Clean-Room Documentation: Bhava (House) Systems

## Module

`dhruv_vedic_base::ascendant`, `dhruv_vedic_base::bhava`, `dhruv_vedic_base::bhava_types`

## Purpose

Compute the Ascendant (Lagna), MC (Midheaven), and 12 bhava (house) cusps
for 10 house division systems used in Vedic and Western astrology.

## Algorithms and Provenance

### Ascendant and MC

**Formula (Ascendant):**
```
Asc = atan2(cos(LST), -(sin(LST)*cos(eps) + tan(phi)*sin(eps)))
```

**Formula (MC):**
```
MC = atan2(sin(LST), cos(LST)*cos(eps))
```

**RAMC:** Equal to LST by definition.

**Source:** Meeus, Jean. *Astronomical Algorithms* (2nd ed.), Chapter 13.
Standard spherical astronomy textbook formula. Independently derivable from
the spherical triangle (pole, zenith, vernal equinox).

**Obliquity:** J2000.0 mean obliquity constant (`OBLIQUITY_J2000_RAD` from
`dhruv_frames`), standard for astrological computation.

**Sidereal time chain:** UTC -> UT1 (via IERS EOP DUT1) -> GMST (Capitaine 2003)
-> LST (GMST + east longitude). Reuses existing `dhruv_time` functions.

### House Systems

#### 1. Equal

Cusp[i] = starting_point + i * 30 degrees (i = 0..11).

**Source:** Elementary division, no external reference needed. Standard in
Vedic astrology (Parashari equal house).

#### 2. Surya Siddhanta

Same algorithm as Equal. The system enum variant exists for semantic clarity
(some software distinguishes the two by name).

#### 3. Sripati (Porphyry)

Angular cusps: Asc (1), IC (4), Desc (7), MC (10).
Intermediate cusps: trisect the four quadrant arcs.

**Source:** Porphyry of Tyre (3rd century CE). Described in:
- Holden, James H. *A History of Horoscopic Astrology*
- Standard astrological reference (arc trisection of quadrants)

#### 4. KP (Placidus)

Angular cusps: Asc (1), IC (4), Desc (7), MC (10).
Intermediate cusps: iterative semi-arc time trisection.

The diurnal/nocturnal semi-arc is divided into thirds in terms of time
(right ascension), then the ecliptic longitude at each division point
is computed iteratively.

**Source:**
- Placidus de Titis (17th century). Described in:
- Montenbruck, Oliver and Pfleger, Thomas. *Astronomy on the Personal Computer*
  (4th ed.), Chapter 11 (house systems).
- The iterative method converges in ~5-10 iterations to < 0.001 arcsecond.

**Latitude limit:** |lat| <= 66.5 degrees (circumpolar issues beyond Arctic/Antarctic circles).

#### 5. Koch

Uses the time for the MC degree to rise from the horizon to the meridian,
divided into thirds.

**Source:**
- Koch, Walter. Original 1960s methodology.
- Described in: Montenbruck & Pfleger, *Astronomy on the Personal Computer*

**Latitude limit:** |lat| <= 66.5 degrees.

#### 6. Regiomontanus

Divides the celestial equator into 30-degree segments from the East Point
(RAMC + 90 deg), then projects each division point to the ecliptic via
the local horizon.

**Source:**
- Regiomontanus (Johannes Müller von Königsberg), 15th century.
- Described in: Meeus, *Astronomical Algorithms*; Montenbruck & Pfleger.

**Latitude limit:** None (works at all latitudes).

#### 7. Campanus

Divides the prime vertical into 30-degree arcs, then projects each division
point onto the ecliptic.

**Source:**
- Campanus of Novara, 13th century.
- Standard spherical astronomy projection (prime vertical -> ecliptic).
- Described in: Montenbruck & Pfleger.

**Latitude limit:** None.

#### 8. Axial Rotation (Meridian)

RAMC + 30-degree equator arcs, projected to ecliptic. Independent of latitude.

**Source:**
- Elementary equator-to-ecliptic projection. Standard spherical astronomy.
- Described in: Montenbruck & Pfleger.

**Latitude limit:** None.

#### 9. Topocentric (Polich-Page)

Similar to Placidus but uses tangent-ratio interpolation with modified
latitudes for intermediate cusps:
- House 11: tan(phi') = tan(phi) / 3
- House 12: tan(phi') = 2*tan(phi) / 3

**Source:**
- Polich, Wendel and Page, A.P. Nelson. "The Topocentric System of Houses."
  *Astrologer's Quarterly*, 1961.
- The original methodology is public domain (published 1961, standard
  reference in astrological literature).

**Latitude limit:** |lat| <= 66.5 degrees.

#### 10. Alcabitus

Divides the diurnal and nocturnal semi-arcs of the Ascendant on the
celestial equator into thirds, then projects to the ecliptic.

**Source:**
- Al-Qabisi (Alcabitius), 10th century.
- Described in: Holden, *A History of Horoscopic Astrology*.
- Standard semi-arc equator division method.

**Latitude limit:** |lat| <= 66.5 degrees.

### Supporting Formulas

**Semi-arc:**
```
cos(H) = -tan(dec) * tan(lat)
semi_arc_diurnal = acos(cos(H))
semi_arc_nocturnal = pi - semi_arc_diurnal
```
Source: Standard spherical astronomy (hour angle at rising/setting).

**Equator to ecliptic projection:**
```
dec = asin(sin(eps) * sin(RA))
lon = atan2(sin(RA)*cos(eps) + tan(dec)*sin(eps), cos(RA))
```
Source: Standard coordinate transformation (Meeus Ch. 13).

**Arc forward:** `(b - a) mod 360` — elementary modular arithmetic.

## Configuration

- **BhavaStartingPoint::Lagna** — default, cusp 1 at Lagna (Ascendant) longitude
- **BhavaStartingPoint::BodyLongitude(body)** — cusp 1 at body's ecliptic longitude
- **BhavaStartingPoint::CustomDeg(deg)** — cusp 1 at arbitrary ecliptic degree
- **BhavaReferenceMode::StartOfFirst** — starting point is cusp 1 (default)
- **BhavaReferenceMode::MiddleOfFirst** — starting point is midpoint of bhava 1
  (shifts all cusps back by half the width of bhava 1)

## Denylisted Sources

No code was referenced from:
- Swiss Ephemeris (GPL)
- Any GPL/AGPL/copyleft implementation

All algorithms were implemented from the public-domain mathematical formulas
described in the sources listed above.
