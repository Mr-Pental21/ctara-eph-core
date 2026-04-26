# Clean-Room Documentation: Bhava Bala (House Strength)

## Source

Primary conceptual source:

- **Brihat Parashara Hora Shastra (BPHS), Chapter 27** public translation passages covering Bhava Bala:
  - base bhava directional strength by sign-group anchor and angular separation
  - Bhavadhipati Bala (strength of the house lord)
  - Bhava Drishti Bala (house aspect strength)
  - special occupation and rising-type bonuses

Supporting conceptual cross-checks:

- Standard traditional sign classifications:
  - Shirshodaya / Prishtodaya / Ubhayodaya

No code-level reference to any copyleft or source-available implementation.

## Scope

This implementation computes Bhava Bala for all 12 houses and exposes:

- Bhavadhipati Bala
- Bhava Dig Bala
- Bhava Drishti Bala
- occupation bonus/penalty
- rising-type bonus
- total Bhava Bala

## Formula Summary

### 1. Bhavadhipati Bala

For each bhava, add the Shadbala total of the lord of the sign occupied by the bhava cusp.

### 2. Bhava Dig Bala

Per BPHS Chapter 27, choose the reference angle to subtract from the bhava cusp by cusp sign:

- Descendant anchor:
  - Virgo, Gemini, Libra, Aquarius
  - first half of Sagittarius
- Nadir anchor:
  - Aries, Taurus, Leo
  - second half of Sagittarius
  - first half of Capricorn
- Ascendant anchor:
  - Cancer, Scorpio
- Meridian anchor:
  - Pisces
  - second half of Capricorn

Then:

1. subtract the chosen anchor from the bhava cusp longitude
2. reduce to the smaller angular separation (`<= 180°`)
3. divide by 3

This yields Bhava Dig Bala in virupas.

## 3. Bhava Drishti Bala

Per BPHS Chapter 27:

- add full aspect strength from Mercury
- add full aspect strength from Jupiter
- add one fourth of aspect strength from other benefics
- subtract one fourth of aspect strength from malefics

Implementation policy:

- Jupiter is always treated as a full positive addition.
- Mercury is full strength, but signed dynamically: benefic Mercury adds its
  full aspect and malefic Mercury subtracts its full aspect.
- Moon is quarter strength and signed dynamically by `BhavaConfig.chandra_benefic_rule`.
- Venus is quarter-positive; Sun, Mars, and Saturn are quarter-negative.
- Rahu/Ketu are quarter-negative only when
  `BhavaConfig.include_node_aspects_for_drik_bala` is enabled.
- Mercury's dynamic nature uses the same association rule as Shadbala Drik
  Bala, including the configured Chandra benefic/malefic rule when Mercury is
  with Chandra.

## 4. Occupation Rule

Per BPHS Chapter 27:

- add 1 rupa (`60` virupas) if Jupiter or Mercury occupies the bhava
- subtract 1 rupa (`60` virupas) if Saturn, Mars, or Sun occupies the bhava

Multiple occupying grahas stack additively.

## 5. Rising-Type Rule

Per BPHS Chapter 27:

- add `15` virupas for Shirshodaya signs in day births
- add `15` virupas for Ubhayodaya signs in twilight births
- add `15` virupas for Prishtodaya signs in night births

Sign grouping used:

- Shirshodaya: Gemini, Leo, Virgo, Libra, Scorpio, Aquarius
- Prishtodaya: Aries, Taurus, Cancer, Sagittarius, Capricorn
- Ubhayodaya: Pisces only

Birth-period classification uses Sandhya windows of `2.5` ghatis before and
after both sunrise and sunset. This gives `5` ghatis around sunrise plus `5`
ghatis around sunset, i.e. `10` total ghatis of Sandhya per civil day. Day is
from sunrise + `2.5` ghatis until sunset - `2.5` ghatis; night is from sunset +
`2.5` ghatis until next sunrise - `2.5` ghatis.

## Implementation Notes

- Bhava sign and lord are determined from the **sidereal** bhava cusp longitude.
- Half-sign rules for Sagittarius and Capricorn use the cusp's degrees within sign:
  - first half: `< 15°`
  - second half: `>= 15°`
- High-level date-based Bhava Bala uses:
  - sidereal bhava cusp positions
  - house-lord Shadbala totals
  - graha-to-bhava drishti virupas
  - graha bhava occupancy
  - day/twilight/night birth period classification
- `BhavaConfig.include_special_bhavabala_rules` defaults to true. When false,
  occupation and rising-rule values are still returned but excluded from
  `total_virupas`.
- `BhavaConfig.chandra_benefic_rule` controls Chandra in Bhava Drishti Bala
  and also controls Chandra's nature inside Mercury's association rule.

## Explicitly Excluded Sources

- Denylisted projects reviewed: `None`
- Source-available/proprietary projects reviewed: `None`

## Contributor Declaration

- This implementation is clean-room and does not derive from denylisted/source-available code.
