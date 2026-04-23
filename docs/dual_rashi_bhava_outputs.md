# Dual Bhava Outputs and Bala/Avastha Basis

Dhruv exposes two bhava bases on high-level jyotish surfaces:

- **Configured bhava system**: the result of `BhavaConfig.system` and related configured bhava fields. Existing fields keep this meaning, for example `bhava_cusps`, `bhava_number`, `graha_to_bhava`, and amsha-chart `bhava_cusps`.
- **Rashi-bhava / equal-house basis**: a whole-sign/equal-house companion basis. New sibling fields use this basis, for example `rashi_bhava_cusps`, `rashi_bhava_number`, `graha_to_rashi_bhava`, and amsha-chart `rashi_bhava_cusps`.

`BhavaConfig` has these behavior flags across Rust, C ABI, CLI, Python, Go, Node, and Elixir:

- `use_rashi_bhava_for_bala_avastha` defaults to `true`. When true, shadbala, bhavabala, bundled balas, and avastha use the rashi-bhava basis. When false, they use the configured bhava-system basis.
- `include_node_aspects_for_drik_bala` defaults to `false`. When true, Shadbala Drik Bala includes Rahu/Ketu incoming aspect contributions. Standalone drishti matrices are unaffected and always report node aspects.
- `divide_guru_buddh_drishti_by_4_for_drik_bala` defaults to `true`. When true, Guru/Buddh incoming aspects participate in the divided Drik Bala balance. When false, their signed incoming aspects are added at full strength after the divided balance.
- `include_rashi_bhava_results` defaults to `true`. When true, high-level result surfaces and public bhava computation results include rashi-bhava sibling fields where that surface exposes bhava-derived results. When false, those sibling fields are suppressed.

Rashi-bhava cusps are synthetic: bhava 1 is the lagna rashi at the lagna degree, and each following bhava advances one rashi while preserving the same degree/minute/second within the sign. The synthetic 10th cusp is used as the meridian equivalent for bhavabala when the rashi-bhava basis is selected.

Shadbala Dig Bala now uses exact sidereal angular distance from each graha to its max-strength cusp:

- Sun and Mars use the 10th cusp.
- Moon and Venus use the 4th cusp.
- Mercury and Jupiter use the 1st cusp.
- Saturn uses the 7th cusp.
- `dig = 60 * (1 - smaller_angle / 180)`, clamped to `0..=60`.

CLI flags:

- `--use-rashi-bhava-for-bala-avastha`
- `--use-configured-bhava-for-bala-avastha`
- `--include-node-aspects-for-drik-bala`
- `--exclude-node-aspects-for-drik-bala`
- `--divide-guru-buddh-drishti-by-4-for-drik-bala`
- `--add-full-guru-buddh-drishti-for-drik-bala`
- `--include-rashi-bhava-results`
- `--no-rashi-bhava-results`
