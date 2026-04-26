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

Nine states determined by independently applicable conditions. A graha can
carry multiple Deeptadi states at the same time when multiple conditions are
true. Public result surfaces therefore expose both a primary compatibility
state and the full Deeptadi state list.

| Primary order | Condition                  | State      |
|----------|----------------------------|------------|
| 1        | Exalted                    | Deepta     |
| 2        | OwnSign/Moolatrikone       | Swastha    |
| 3        | Conjunct Surya             | Kopa       |
| 4        | Conjunct a malefic graha except Surya | Vikala |
| 5        | Inauspicious sign          | Khala      |
| 6        | Extreme friend sign        | Pramudita  |
| 7        | Friend sign                | Shanta     |
| 8        | Neutral sign               | Deena      |
| 9        | Enemy/debilitation sign    | Dukhita    |

The primary order above is used only when an older single-value surface needs
one Deeptadi state. The full Deeptadi state set is the authoritative result.
Surya conjunction contributes Kopa, so Surya is not also reused as the generic
malefic-conjunction trigger for Vikala. Chandra and Buddh use the dynamic
benefic/malefic rules from `BhavaConfig.chandra_benefic_rule`; Buddh also uses
the same association-based rule used by Shadbala Drik Bala.

**Inauspicious signs:** Mesha, Vrishabha, Karka, Dhanu, Makara.

**Strength factors:** Deepta=1.0, Swastha=0.9, Pramudita=0.8,
Shanta=0.75, Deena=0.6, Dukhita=0.4, Khala=0.3, Kopa=0.2,
Vikala=0.1.

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

Six states determined by independently applicable house placement,
conjunction, dignity, and aspect conditions. A graha may have multiple
Lajjitadi states, or no Lajjitadi state when no condition applies.

| Primary order | Condition                                      | State     |
|---------------|------------------------------------------------|-----------|
| 1             | 5th bhava + conjunct Rahu/Ketu/Mangal/Shani    | Lajjita   |
| 2             | Exalted or Moolatrikone                        | Garvita   |
| 3             | Natural enemy sign/conjunction, or conjunct Shani and aspected by natural enemy | Kshudhita |
| 4             | Water sign + natural enemy aspect + no benefic aspect | Trushita  |
| 5             | Conjunct Surya + malefic aspect + natural enemy aspect | Kshobhita |
| 6             | Natural friend sign/conjunction, or conjunct Guru and aspected by natural friend | Mudita    |

**Definitions:**
- Malefic/benefic: Chandra and Buddh use the dynamic rules from
  `BhavaConfig.chandra_benefic_rule`; other grahas use
  `natural_benefic_malefic()`.
- Natural friend/enemy: via `naisargika_maitri(graha, other)`.
- Aspected by: grahas with `total_virupa > 45.0` from drishti matrix.
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
- `ghatikas` = configured rounded ghatikas since sunrise (u16); default is floor,
  with ceil available through BhavaConfig.
- `lagna_rashi` = lagna rashi number (1-12)

**12 Avasthas:** Nidra(0), Sayana(1), Upavesha(2), Netrapani(3), Prakasha(4),
Gamana(5), Agamana(6), Sabha(7), Agama(8), Bhojana(9), NrityaLipsa(10), Kautuka(11).

### Sayanadi Sub-States

Each avastha has 3 possible sub-states (Drishti, Chestha, Vicheshta) computed
for 5 BPHS name-group variants:

```
R = (avastha_number^2 + name_anka) % 12
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
- **SignLordBased** (default): compound dignity based on the node's direct
  naisargika maitri with the target sign lord, plus node-to-lord temporary
  relationship. Rahu in Kumbha and Ketu in Vrischika are treated as own-sign
  before sign-lord friendship is evaluated.
- **AlwaysSama**: always returns `Dignity::Sama` for nodes

This affects Jagradadi, Deeptadi, and Lajjitadi avasthas for Rahu/Ketu.

## Implementation Notes

- All avastha functions are pure math — no engine queries
- Orchestration layer (`assemble_avastha_inputs`) gathers all inputs from
  engine queries and JyotishContext cache
- `birth_ghatikas` uses BhavaConfig `sayanadi_ghatika_rounding`; default is `floor`,
  optional `ceil` counts the current partial ghatika.
- Navamsa number: `((sidereal_lon / (360/108)) % 9).floor() + 1`
