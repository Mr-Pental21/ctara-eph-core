# Clean-Room Documentation: Amsha (Divisional Charts)

## Provenance

Divisional charts (amsha / varga) are described in the *Brihat Parashara Hora
Shastra* (BPHS), chapters on Shodashavarga (16 divisional charts) and other
varga classifications. The mapping rules below are derived solely from public
BPHS translations and standard Jyotish reference texts.

No code or algorithms from copyleft or source-available implementations were
consulted.

## Algorithm

An amsha chart transforms a **sidereal longitude** through a divisional
mapping. Each amsha divides the 30-degree rashi span into *N* equal parts
(where *N* = the amsha's division count) and maps each part to a target rashi
based on predefined rules.

### Core transformation

Given sidereal longitude *L* (degrees, [0, 360)):

1. `rashi_idx = floor(L / 30)` (0-11, the natal rashi index)
2. `pos_in_rashi = L - rashi_idx * 30` (degrees within the rashi, [0, 30))
3. `deg_per_div = 30 / N` (span of each division)
4. `div_idx = min(floor(pos_in_rashi / deg_per_div), N - 1)` (clamp for boundary)
5. `target_rashi_idx = lookup(amsha, rashi_idx, div_idx)` (see sequence rules)
6. `scaled_pos = (pos_in_rashi mod deg_per_div) / deg_per_div * 30` (scale to rashi)
7. `result = (target_rashi_idx * 30 + scaled_pos) mod 360`

D1 (Rashi) is the identity: output equals input.

## Supported Amshas (34)

| Code | Name | Divisions | Sanskrit |
|------|------|-----------|----------|
| D1 | Rashi | 1 | Rashi |
| D2 | Hora | 2 | Hora |
| D3 | Drekkana | 3 | Drekkana |
| D4 | Chaturthamsha | 4 | Chaturthamsha |
| D5 | Panchamsha | 5 | Panchamsha |
| D6 | Shashthamsha | 6 | Shashthamsha |
| D7 | Saptamsha | 7 | Saptamsha |
| D8 | Ashtamsha | 8 | Ashtamsha |
| D9 | Navamsha | 9 | Navamsha |
| D10 | Dashamsha | 10 | Dashamsha |
| D11 | Ekadashamsha | 11 | Ekadashamsha |
| D12 | Dwadashamsha | 12 | Dwadashamsha |
| D15 | Panchadashamsha | 15 | Panchadashamsha |
| D16 | Shodashamsha | 16 | Shodashamsha |
| D18 | Ashtadashamsha | 18 | Ashtadashamsha |
| D20 | Vimshamsha | 20 | Vimshamsha |
| D21 | Ekavimshamsha | 21 | Ekavimshamsha |
| D22 | Dwavimshamsha | 22 | Dwavimshamsha |
| D24 | Chaturvimshamsha | 24 | Chaturvimshamsha |
| D25 | Panchavimshamsha | 25 | Panchavimshamsha |
| D27 | Saptavimshamsha | 27 | Bhamsha |
| D28 | Ashtavimshamsha | 28 | Ashtavimshamsha |
| D30 | Trimshamsha | 30 | Trimshamsha |
| D36 | Shashtyamsha | 36 | Chatvarimsha |
| D40 | Khavedamsha | 40 | Khavedamsha |
| D45 | Akshavedamsha | 45 | Akshavedamsha |
| D48 | Ashtachatvarimsha | 48 | Ashtachatvarimsha |
| D50 | Panchashtamsha | 50 | Panchashtamsha |
| D54 | Chaturpanchashamsha | 54 | Chaturpanchashamsha |
| D60 | Shashtyamsha | 60 | Shashtyamsha |
| D72 | Dwasaptatimsha | 72 | Dwasaptatimsha |
| D81 | Ekasheetimsha | 81 | Ekasheetimsha |
| D108 | Ashtottaramsha | 108 | Ashtottaramsha |
| D144 | Dwadas-dwadashamsha | 144 | Dwadas-dwadashamsha |

## Shodashavarga (16 standard)

D1, D2, D3, D4, D7, D9, D10, D12, D16, D20, D24, D27, D30, D40, D45, D60

## Sequence Rules

### Starting Rashi

| Rule Type | Description | Used By |
|-----------|-------------|---------|
| NATAL | Start from natal rashi | D3-D6, D8, D11-D12, D15, D18, D21-D22, D25, D27-D28, D36, D45, D48, D50, D54, D72, D81, D108, D144 |
| DOUBLE_MOD | `(rashi * 2) % 12` | D2 (standard) |
| CANCER_LEO | Odd rashi → Cancer(3), Even → Leo(4) | D2 (HoraCancerLeoOnly variation) |
| INCREMENT | Odd rashi → natal, Even → natal + offset | D7(+6), D10(+8), D24(+4), D40(+6) |
| FEAW | Element-based fixed start | D9, D16, D20, D60 |
| ODD_EVEN_FIXED | Odd → Mesha(0), Even → Meena(11) | D30 |

### Progression

- **Sequential** (+1 mod 12): All amshas except D3
- **Trine** (+4 mod 12): D3 (Drekkana)

### FEAW (Fire/Earth/Air/Water) Values

Rashi element classification:
- Fire: Mesha(0), Simha(4), Dhanu(8)
- Earth: Vrishabha(1), Kanya(5), Makara(9)
- Air: Mithuna(2), Tula(6), Kumbha(10)
- Water: Karka(3), Vrischika(7), Meena(11)

FEAW starting rashi by amsha:
| Amsha | Fire | Earth | Air | Water |
|-------|------|-------|-----|-------|
| D9 | Mesha(0) | Makara(9) | Tula(6) | Karka(3) |
| D16 | Mesha(0) | Simha(4) | Dhanu(8) | Mesha(0) |
| D20 | Mesha(0) | Dhanu(8) | Simha(4) | Mesha(0) |
| D60 | Mesha(0) | Makara(9) | Tula(6) | Karka(3) |

## Variations

| Code | Name | Applicable To | Description |
|------|------|---------------|-------------|
| 0 | TraditionalParashari | All | Standard BPHS divisional mapping |
| 1 | HoraCancerLeoOnly | D2 only | Hora with only Cancer/Leo as target rashis |

## Validation

- Unknown amsha code → error
- Unknown variation code → error
- Variation not applicable to amsha → error
- Fail-fast: first invalid entry stops processing
- Duplicate amsha+variation pairs allowed (no dedup)

## References

- BPHS: Brihat Parashara Hora Shastra (public domain translation)
- Standard Jyotish textbooks on Shodashavarga and higher divisional charts
