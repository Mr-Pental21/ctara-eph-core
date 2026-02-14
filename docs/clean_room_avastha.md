# Clean-Room Reference: Graha Avasthas (Planetary States)

## Sources

- **BPHS** (Brihat Parashara Hora Shastra), Chapters 44-45
- **Saravali** by Kalyana Varma
- Standard Vedic astrology textbooks on planetary states

## Overview

Avasthas (planetary states) indicate a graha's ability to deliver results.
Five classification systems are implemented, each providing a different lens
on planetary condition.

## 1. Baladi Avastha (Age-Based)

Five states based on degree position within the sign (30 deg / 5 = 6 deg bands):

| Band (deg in sign) | Odd Signs      | Even Signs     |
|---------------------|----------------|----------------|
| 0-6                 | Bala (infant)  | Mrita (dead)   |
| 6-12                | Kumara (youth) | Vriddha (old)  |
| 12-18               | Yuva (prime)   | Yuva (prime)   |
| 18-24               | Vriddha (old)  | Kumara (youth) |
| 24-30               | Mrita (dead)   | Bala (infant)  |

Odd signs: indices 0,2,4,6,8,10 (Mesha, Mithuna, Simha, Tula, Dhanu, Kumbha).
Even signs: indices 1,3,5,7,9,11 (Vrishabha, Karka, Kanya, Vrischika, Makara, Meena).

**Strength factors:** Bala=0.25, Kumara=0.50, Yuva=1.00, Vriddha=0.50, Mrita=0.00.

## 2. Jagradadi Avastha (Wakefulness-Based)

Three states based on dignity (relationship with sign):

| Dignity                                | State    |
|----------------------------------------|----------|
| Exalted, Moolatrikone, OwnSign         | Jagrat   |
| AdhiMitra, Mitra                       | Swapna   |
| Sama, Shatru, AdhiShatru, Debilitated  | Sushupta |

**Strength factors:** Jagrat=1.0, Swapna=0.5, Sushupta=0.25.

## 3. Deeptadi Avastha (Condition-Based)

Nine states determined by priority-ordered conditions:

| Priority | Condition           | State    |
|----------|---------------------|----------|
| 1        | Exalted             | Deepta   |
| 2        | Lost planetary war  | Peedita  |
| 3        | Combust             | Deena    |
| 4        | Debilitated         | Vikala   |
| 5        | Retrograde          | Shakta   |
| 6        | OwnSign/Moolat      | Swastha  |
| 7        | Friend sign         | Mudita   |
| 8        | Enemy sign          | Khala    |
| 9        | Default             | Shanta   |

**Strength factors:** Deepta=1.0, Swastha=0.9, Shakta=0.8, Mudita=0.75,
Shanta=0.6, Khala=0.4, Kshobhita=0.35, Peedita=0.3, Kshudhita=0.25,
Deena=0.2, Vikala=0.1.

### Combustion Thresholds (BPHS)

| Graha   | Direct | Retrograde |
|---------|--------|------------|
| Moon    | 12     | 12         |
| Mars    | 17     | 17         |
| Mercury | 14     | 12         |
| Jupiter | 11     | 11         |
| Venus   | 10     | 8          |
| Saturn  | 15     | 15         |

Boundary rule: strictly less-than (`<`). Exactly at threshold = not combust.
Sun, Rahu, Ketu are never combust.

### Planetary War

Two grahas within 1 degree of each other; the one with lower absolute
declination loses. Only Mars, Mercury, Jupiter, Venus, Saturn participate.

## 4. Lajjitadi Avastha (Emotional-Based)

Six states determined by house placement, conjunctions, and aspects:

| Priority | Condition                                         | State     |
|----------|---------------------------------------------------|-----------|
| 1        | In 5th house + conjunct a malefic                 | Lajjita   |
| 2        | Exalted or Moolatrikone                           | Garvita   |
| 3        | In enemy sign (Shatru/AdhiShatru/Debilitated)     | Kshudhita |
| 4        | In water sign + enemy aspect - no benefic          | Trushita  |
| 5        | Conjunct Sun + aspected by malefic                | Kshobhita |
| 6        | Default                                           | Mudita    |

**Definitions:**
- Malefic: via `natural_benefic_malefic()` (Mars, Saturn, Rahu, Ketu, plus waning Moon)
- Aspected by: grahas with `total_virupa >= 45.0` from drishti matrix
- Water signs: Karka (3), Vrischika (7), Meena (11)
- Same rashi = conjunction

**Strength factors:** Garvita=1.0, Mudita=0.8, Kshobhita=0.35, Kshudhita=0.3,
Trushita=0.25, Lajjita=0.2.

## 5. Sayanadi Avastha (BPHS Ch.45)

Twelve states from a mathematical formula:

```
R = ((nk + 1) * planet_number * navamsa + (janma_nk + 1) + ghatikas + lagna_rashi) % 12
```

Where:
- `nk` = graha's nakshatra index (0-26)
- `planet_number` = Sun=1, Moon=2, ... Ketu=9
- `navamsa` = navamsa number (1-9) from sidereal longitude
- `janma_nk` = Moon's nakshatra index at birth
- `ghatikas` = floor of ghatikas since sunrise (u16)
- `lagna_rashi` = lagna rashi number (1-12)

**12 Avasthas:** Sayana(0), Upavesha(1), Netrapani(2), Prakasha(3), Gamana(4),
Agamana(5), Sabha(6), Agama(7), Bhojana(8), NrityaLipsa(9), Kautuka(10), Nidra(11).

### Sayanadi Sub-States

Each avastha has 3 possible sub-states (Drishti, Chestha, Vicheshta) computed
for 5 BPHS name-group variants:

```
R = ((avastha_index + 1)^2 + name_anka) % 12
sub_state = (R + planet_constant) % 3
```

| Name Group     | Anka |
|----------------|------|
| Ka-varga       | 1    |
| Cha-varga      | 6    |
| Ta(retroflex)  | 11   |
| Ta(dental)     | 16   |
| Pa-varga       | 21   |

**Planet constants:** Sun=5, Moon=2, Mars=2, Mercury=3, Jupiter=5, Venus=3,
Saturn=3, Rahu=4, Ketu=4.

**Sub-state mapping:** remainder 1=Drishti, 2=Chestha, 0=Vicheshta.

**Strength factors:** Drishti=0.5, Chestha=1.0, Vicheshta=0.0.

## Node Dignity Policy

Rahu and Ketu dignity is computed via `node_dignity_in_rashi()` with configurable
`NodeDignityPolicy`:
- **SignLordBased** (default): dignity based on naisargika maitri with sign lord
- **AlwaysSama**: always returns `Dignity::Sama` for nodes

This affects Jagradadi, Deeptadi, and Lajjitadi avasthas for Rahu/Ketu.

## Implementation Notes

- All avastha functions are pure math — no engine queries
- Orchestration layer (`assemble_avastha_inputs`) gathers all inputs from
  engine queries and JyotishContext cache
- `birth_ghatikas` uses explicit `floor()` for deterministic rounding
- Navamsa number: `((sidereal_lon / (360/108)) % 9).floor() + 1`
