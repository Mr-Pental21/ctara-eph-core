# Clean-Room Record: Ayanamsha Module

## Precession Polynomial

**Source**: Capitaine, N., Wallace, P.T. & Chapront, J. (2003).
"Expressions for the Celestial Intermediate Pole and Celestial Ephemeris
Origin consistent with the IAU 2000A precession-nutation model."
_Astronomy & Astrophysics_, 412, 567-586. Table 1.

Also published as: IERS Conventions (2010), Chapter 5, Table 5.1.

**License**: Public domain (IAU standard, intergovernmental scientific body).

**Implementation**: `dhruv_frames::precession::general_precession_longitude_arcsec()`
implements the 5th-order polynomial directly from the published coefficients.

---

## Ayanamsha Reference Values

Each system's J2000.0 reference value was independently derived from the
system's published definition. No values were copied from any copyleft or
denylisted source code.

### Star-Anchored Systems

Star positions at J2000.0 from the Hipparcos catalog (ESA, 1997, public
domain) converted to ecliptic coordinates using IAU 2006 obliquity.

| System | Anchor Star | Sidereal Position | J2000.0 Ecliptic Lon | Ayanamsha |
|--------|-------------|-------------------|---------------------|-----------|
| Lahiri | Spica (alpha Vir) | 0 deg Libra (180 deg) | ~203.83 deg | 23.853 deg (Indian govt gazette) |
| TrueLahiri | Same as Lahiri | Same | Same | 23.853 deg (nutation applied separately) |
| FaganBradley | SVP calibration | Empirical | Empirical | 24.736 deg (published SVP tables) |
| PushyaPaksha | delta Cancri | 16 deg Cancer (106 deg) | ~127 deg | 21.000 deg |
| RohiniPaksha | Aldebaran (alpha Tau) | 15 deg 47 min Taurus (45.783 deg) | ~69.87 deg | 24.087 deg |
| GalacticCenter0Sag | Galactic Center | 0 deg Sagittarius (240 deg) | ~266.86 deg | 26.860 deg |
| Aldebaran15Tau | Aldebaran | 15 deg Taurus (45 deg) | ~69.87 deg | 24.870 deg |

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
| ChandraHari | Chandra Hari | 23.250 deg | Published Chandra Hari tables |
| Jagganatha | Jagganatha | 23.250 deg | Published Jagganatha tables |
| SuryaSiddhanta | Ancient Indian treatise | 22.459 deg | Back-computed with IAU precession |

### Design Decision: IAU Precession for All Systems

All 20 systems use the IAU 2006 general precession polynomial for time
propagation. This ensures mathematical consistency across systems. Some
traditional systems (e.g., Surya Siddhanta) historically used a fixed
54 arcsec/year rate, which diverges from the IAU model over centuries.
Our reference values are calibrated so that the IAU-based formula matches
published tables at J2000.0.

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
