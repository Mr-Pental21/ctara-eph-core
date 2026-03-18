# Clean-Room Record: Ayanamsha Module

## Precession Models

Three precession models are supported via `PrecessionModel`:

| Model | Source | Polynomial Order |
|-------|--------|-----------------|
| Lieske 1977 (IAU 1976) | Lieske et al. 1977, A&A 58; Lieske 1979, A&A 73, 282 | 3rd-order |
| IAU 2006 | Capitaine, Wallace & Chapront 2003, A&A 412, 567-586, Table 1; IERS Conventions 2010, Ch.5, Table 5.1 | 5th-order |
| Vondrák 2011 | Vondrák, Capitaine & Wallace 2011, A&A 534, A22 | Long-term periodic series |

**License**: All public domain (IAU standards, peer-reviewed journals).

**Default**: `Vondrak2011` (best long-term accuracy).

Each model provides three ecliptic precession parameters:
- p_A: general precession in ecliptic longitude
- π_A: inclination of ecliptic-of-date to J2000 ecliptic
- Π_A: longitude of ascending node of ecliptic-of-date on J2000 ecliptic

---

## Ecliptic-of-Date Consistency

Both the tropical longitude and the ayanamsha are computed on the ecliptic-of-
date using the full 3D precession matrix. This ensures `sidereal = tropical −
ayanamsha` is geometrically consistent.

### Tropical longitude

**Coordinate chain:**
```
ICRF J2000 (engine output)
  → Ecliptic J2000  [icrf_to_ecliptic()]
  → Ecliptic of Date  [precess_ecliptic_j2000_to_date(v, t)]
  → tropical longitude = atan2(y, x)
```

### Ayanamsha (ecliptic-of-date)

The sidereal zero point is tracked as a 3D direction vector through the same
precession matrix. Its longitude on the ecliptic-of-date IS the ayanamsha:

```
v_j2000 = [cos(ref), sin(ref), lat_component]   (on J2000 ecliptic)
v_date  = P(t) · v_j2000                         (precessed to ecliptic-of-date)
ayanamsha = atan2(v_date.y, v_date.x)
```

The earlier scalar formula `ayanamsha = ref + p_A(t)` ignored the ecliptic tilt
(π_A ≈ 47"/century), creating a small but growing inconsistency with the 3D
tropical longitude. The 3D approach eliminates this.

### Precession rotation

```
P = R3(-(Π_A + p_A)) · R1(π_A) · R3(Π_A)
```
where π_A is the inclination of the ecliptic of date on the ecliptic of J2000,
Π_A is the longitude of the ascending node of the ecliptic of date on J2000,
and p_A is the general precession in longitude.

**Implementations**:
- `dhruv_frames::precession::precess_ecliptic_j2000_to_date(v, t)`
- `dhruv_frames::precession::precess_ecliptic_date_to_j2000(v, t)` (inverse)

**Velocity correction**: d(P·r)/dt = P·(dr/dt) + Ṗ·r. The Ṗ·r cross-term
is automatically captured by finite-differencing the fully-precessed longitude
at t ± 1 minute, rather than rotating the raw velocity vector.

---

## Ayanamsha Reference Values

Each system's J2000.0 reference value was independently derived from the
system's published definition. No values were copied from any copyleft or
denylisted source code.

### Star/Anchor-Based Systems

Star positions at J2000.0 derived from the HGCA EDR3 star catalog (Brandt
2021, ApJS 254, 42), propagated from J2016.0 to J2000.0 via proper motion
(Butkevich & Lindegren 2014), then converted to ecliptic coordinates using
IAU 2006 obliquity (ε = 23.439291°).

The embedded HGCA catalog (`kernels/data/hgca_tara.json`) is compiled into
the binary and used as the **default** computation path for all star-anchored
systems. Static hardcoded coordinates serve as a fallback and validation path.

For named star-anchor definitions, the target sidereal longitudes come from the
published definition of each system. In particular:

- RohiniPaksha uses Aldebaran fixed at the middle of Rohini nakshatra,
  `16Ta40`, following P. V. R. Narasimha Rao's published Rohini-paksha
  definition.
- PushyaPaksha uses delta Cancri fixed at `16Cn00`, matching the corresponding
  P. V. R. Narasimha Rao Pushya-paksha definition.

| System | Anchor | Sidereal Position | J2000.0 Ecliptic Lon | Ayanamsha |
|--------|--------|-------------------|---------------------|-----------|
| Lahiri | Sidereal zero at 1956 anchor | 0 deg (sidereal zero) | 23.862 deg | 23°15'00.658" at 1956-03-21 (IAE gazette) |
| TrueLahiri | Spica (alpha Vir) | 0 deg Libra (180 deg), star-locked | 203.841 deg | anchor-relative (star-locked) |
| FaganBradley | SVP calibration | Empirical | Empirical | 24.736 deg (published SVP tables) |
| PushyaPaksha | delta Cancri | 16 deg Cancer (106 deg) | 128.722 deg | 22.722 deg |
| RohiniPaksha | Aldebaran (alpha Tau) | 16 deg 40 min Taurus (46.667 deg) | 69.789 deg | 23.123 deg |
| GalacticCenter0Sag | Galactic Center | 0 deg Sagittarius (240 deg) | 266.840 deg | 26.840 deg |
| Aldebaran15Tau | Aldebaran | 15 deg Taurus (45 deg) | 69.789 deg | 24.789 deg |

### Anchor Star Coordinate Provenance

Source catalog: HGCA EDR3 (Brandt 2021, ApJS 254, 42)
Reference epoch: J2016.0 (ICRS)
Propagation: Butkevich & Lindegren (2014) to J2000.0
Ecliptic conversion: IAU 2006 obliquity (ε = 23.439291°)

Embedded catalog: `kernels/data/hgca_tara.json` (compiled into binary)
Derivation test: `ayanamsha_anchor::tests::derive_anchor_coordinates_from_catalog()`

Published definition references for target sidereal longitudes:
- P. V. R. Narasimha Rao, "Correct Siddhamsa Chart of Parasara" (Rohini-paksha
  anchor at Aldebaran = `16Ta40`)
- P. V. R. Narasimha Rao, "Introducing Pushya-paksha Ayanamsa"
  (Pushya-paksha anchor at delta Cancri = `16Cn00`)

### Epoch-Defined Systems

For systems defined by a zero-ayanamsha epoch, the reference was derived
from published tables or adopted values for that system at J2000.0.

| System | Definition | J2000.0 Value | Source |
|--------|-----------|---------------|--------|
| KP | Krishnamurti Paddhati | 23.850 deg | Published KP tables, minimal offset from Lahiri |
| Raman | Zero year ~397 CE | 22.370 deg | B.V. Raman "Hindu Predictive Astrology" tables |
| DeLuce | Robert DeLuce (1930s) | 21.619 deg | Published DeLuce tables |
| DjwalKhul | Alice Bailey tradition | 22.883 deg | Published esoteric astrology tables |
| Hipparchos | Hipparchus ~128 BCE | 21.176 deg | Derived from historical observations |
| Sassanian | Sassanid Persian | 19.765 deg | Published Sassanian tradition tables |
| DevaDutta | Deva-Dutta | 22.474 deg | Published Deva-Dutta tables |
| UshaShashi | Usha-Shashi | 20.103 deg | Published Usha-Shashi tables |
| Yukteshwar | Sri Yukteshwar (1894) | 22.376 deg | "The Holy Science" adopted value |
| JnBhasin | J.N. Bhasin | 22.376 deg | Published J.N. Bhasin tables |
| ChandraHari | λ Sco at 240° sidereal | 24.586 deg | Anchor-relative (star-locked) |
| Jagganatha | Spica at 180° on invariable plane | 23.841 deg | Anchor-relative, invariable plane (see `clean_room_invariable_plane.md`) |
| SuryaSiddhanta | Ancient Indian treatise | 22.459 deg | Back-computed with IAU precession |

### Design Decision: 3D Precession for All Systems

All 20 systems use the full 3D ecliptic precession matrix (default: Vondrák
2011) for time propagation. The precession model is selectable via
`PrecessionModel` (Lieske1977, Iau2006, Vondrak2011). Some traditional
systems (e.g., Surya Siddhanta) historically used a fixed 54 arcsec/year
rate, which diverges from modern models over centuries. Our reference values
are calibrated so that the modern formula matches published tables at J2000.0.

### Anchor-Relative Systems

Eight systems are evaluated by tracking a 3D anchor direction through the
precession matrix (or invariable-plane rotation for Jagganatha), preserving
full consistency including the ecliptic tilt (π_A):

- Lahiri (sidereal zero point at 0 deg, back-precessed from 1956 anchor)
- TrueLahiri (Spica at 180 deg)
- PushyaPaksha (Pushya anchor at 106 deg)
- RohiniPaksha (Aldebaran at 16 deg 40 min Taurus)
- Aldebaran15Tau (Aldebaran at 15 deg Taurus)
- GalacticCenter0Sag (Galactic center at 240 deg)
- ChandraHari (λ Sco at 240 deg)
- Jagganatha (Spica at 180 deg, invariable plane — see below)

For these systems, the implementation stores the anchor's J2000 ecliptic
longitude and latitude, precesses to ecliptic-of-date, and derives:

`ayanamsha = anchor_tropical_longitude - target_sidereal_longitude`

Lahiri was converted to anchor-relative because its 1956 anchor epoch is
on the ecliptic-of-1956 (not J2000). Back-precessing to J2000 produces a
small ecliptic latitude (0.0027 deg) that must be preserved through the
round-trip to recover the anchor value exactly.

When `use_nutation` is true, nutation in longitude (Δψ) is added to the mean
ayanamsha for all systems uniformly. The Lahiri anchor is stored as a MEAN
value (IAE gazette 23°15'00.658" at 1956-03-21 minus IAU 2000B nutation at
that epoch, Δψ ≈ 16.78"). This ensures `ayanamsha_deg(Lahiri, t_1956, true)`
recovers the original gazette value.

### Calibration Record (Lahiri Mean Anchor)

```
Epoch:         1956-03-21 00:00 TDT (JD 2435553.5)
Time-scale:    TDT ≈ TDB at 1956 (ΔTT ≈ 0)
Gazette value: 23°15'00.658" (TRUE, includes nutation)
Nutation model: IAU 2000B (77 lunisolar terms)
Δψ(1956):      16.779932" (arcsec)
Mean anchor:   23°15'00.658" − 16.780" = 23°14'43.878"
Precession:    Vondrák 2011 (3D ecliptic matrix)
J2000 lon:     23.857052898247307°
J2000 lat:     0.002727754076653°
```

### Non-Anchor Systems (3D Vector Precession)

The remaining 15 systems store a J2000 ecliptic longitude (`reference_j2000_deg`)
for their sidereal zero point. The ayanamsha is computed by placing a unit vector
at that longitude on the J2000 ecliptic (latitude = 0), precessing to ecliptic-of-
date via the 3D matrix, and reading off the longitude:

```
v = [cos(ref_rad), sin(ref_rad), 0]
v_date = P(t) · v
ayanamsha = atan2(v_date.y, v_date.x)
```

This replaces the earlier scalar `ref + p_A(t)` formula. At J2000 (t=0), the
result is identical. At distant epochs the difference is sub-arcsecond but
ensures consistency with the 3D tropical longitude.

### Jagganatha: Invariable Plane System

The Jagganatha ayanamsha differs from all other systems in that it operates on
the **invariable plane** instead of the ecliptic. It is defined as "True Lahiri
on the invariable plane": Spica (Chitra) locked to 180° sidereal, with all
positional measurements on the invariable plane.

**Reference plane**: Invariable (Souami & Souchay 2012, A&A 543, A133)

**Anchor**: Spica at 180° sidereal (same star as TrueLahiri)

**Coordinate chain for ayanamsha**:
```
Spica ICRF position (from catalog)
  → Invariable plane [icrf_to_invariable()]
  → spherical longitude (no precession — plane is fixed)
  → ayanamsha = star_invariable_lon - 180°
```

**Coordinate chain for planet sidereal longitude**:
```
Planet ICRF position (from engine)
  → Invariable plane [icrf_to_invariable()]
  → spherical longitude (no precession)
  → sidereal_lon = planet_invariable_lon - ayanamsha
```

**Key differences from ecliptic systems**:
- No precession correction needed (invariable plane is fixed)
- Nutation has no effect (`use_nutation` is ignored)
- Lagna/bhava cusps projected from ecliptic to invariable plane before
  sidereal conversion
- Lunar node computed via angular momentum on the invariable plane

See `docs/clean_room_invariable_plane.md` for full details on the invariable
plane definition, constants, and rotation matrix.

---

## Denylisted Sources NOT Referenced

- Swiss Ephemeris (GPL)
- Any GPL/AGPL/copyleft ayanamsha implementations
- No source code from any denylisted project was inspected

## Validation

Golden tests compare against:
- Indian Astronomical Ephemeris (Lahiri at J2000.0)
- Rashtriya Panchang 2024 (Lahiri at 2024)
- Published Western sidereal tables (Fagan-Bradley)
- B.V. Raman's published tables (Raman)

All comparisons are black-box I/O validation against published values.
