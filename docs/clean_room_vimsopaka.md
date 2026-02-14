# Clean-Room Documentation: Vimsopaka Bala (20-Point Varga Dignity Strength)

## Source

Formulas from **Brihat Parashara Hora Shastra (BPHS)**, chapters on Varga
classification and strength. Weight tables cross-checked against standard
commentaries.

No code-level reference to any copyleft implementation.

## Scope

**Navagraha (all 9)** — Sun through Ketu. Rahu/Ketu dignity handled via
configurable `NodeDignityPolicy` (see below).

## Concept

Vimsopaka Bala evaluates a planet's dignity across multiple divisional charts
(vargas) and produces a weighted score out of 20. Higher scores indicate
stronger placement across the varga spectrum.

## Varga Groupings & Weights

BPHS defines four groupings. Weights in each grouping sum to 20.

### Shadvarga (6 divisions)

| Amsha | Weight |
|-------|--------|
| D1 (Rashi) | 6.0 |
| D2 (Hora) | 2.0 |
| D3 (Drekkana) | 4.0 |
| D9 (Navamsha) | 5.0 |
| D12 (Dwadashamsha) | 2.0 |
| D30 (Trimshamsha) | 1.0 |

### Saptavarga (7 divisions)

| Amsha | Weight |
|-------|--------|
| D1 | 5.0 |
| D2 | 2.0 |
| D3 | 3.0 |
| D7 (Saptamsha) | 2.5 |
| D9 | 4.5 |
| D12 | 2.0 |
| D30 | 1.0 |

### Dashavarga (10 divisions)

| Amsha | Weight |
|-------|--------|
| D1 | 3.0 |
| D2 | 1.5 |
| D3 | 1.5 |
| D7 | 1.5 |
| D9 | 3.0 |
| D10 (Dashamsha) | 1.5 |
| D12 | 1.5 |
| D16 (Shodashamsha) | 2.0 |
| D30 | 1.0 |
| D60 (Shashtiamsha) | 3.5 |

### Shodasavarga (16 divisions)

| Amsha | Weight |
|-------|--------|
| D1 | 3.5 |
| D2 | 1.0 |
| D3 | 1.0 |
| D4 (Chaturthamsha) | 0.5 |
| D7 | 0.5 |
| D9 | 3.0 |
| D10 | 0.5 |
| D12 | 0.5 |
| D16 | 2.0 |
| D20 (Vimshamsha) | 0.5 |
| D24 (Chaturvimshamsha) | 0.5 |
| D27 (Saptavimshamsha) | 0.5 |
| D30 | 1.0 |
| D40 (Khavedamsha) | 0.5 |
| D45 (Akshavedamsha) | 0.5 |
| D60 | 4.0 |

## Dignity Points

For each varga, the planet's dignity in that varga's rashi determines points:

| Dignity | Points |
|---------|--------|
| Exalted | 20 |
| Moolatrikone | 18 |
| Own Sign | 15 |
| Adhi Mitra | 12 |
| Mitra | 10 |
| Sama | 7 |
| Shatru | 5 |
| Adhi Shatru | 3 |
| Debilitated | 2 |

## Computation

For each varga grouping:

```
score = Σ(weight_i × points_i) / Σ(weight_i)
```

Since all grouping weights sum to 20, this simplifies to:

```
score = Σ(weight_i × points_i) / 20
```

Result is in [0, 20].

## Per-Varga Temporal Friendship

For each varga, the planet's temporal friendship is determined by its rashi
position **within that varga** (i.e., the amsha-transformed position), not by
the D1 rashi position. This is important because a planet may be in different
rashis in different vargas, leading to different temporal friendship patterns.

## Node Dignity Policy (Rahu/Ketu Extension)

BPHS does not define exaltation, moolatrikone, or friendship tables for Rahu
and Ketu in a universally agreed way. This implementation provides a
configurable `NodeDignityPolicy`:

### SignLordBased (default)

The node's dignity in a rashi is determined by the relationship between the
node's dispositor (sign lord of the node's D1 rashi) and the target rashi's
lord. Temporal component uses the node's rashi position vs other grahas.

### AlwaysSama

Conservative choice: Rahu and Ketu receive Sama (7 points) in every varga.
This is the safest option when no definitive BPHS source is available.

**Note**: This policy is explicitly marked as "extension beyond strict BPHS"
in the codebase, with documented rationale for auditability.
