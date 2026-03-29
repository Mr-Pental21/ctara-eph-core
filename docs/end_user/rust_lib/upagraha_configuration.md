# Rust Library Upagraha Configuration

These examples use the Rust wrapper surface and `TimeUpagrahaConfig`.

Default behavior:

- Gulika: `Rahu` + `Start`
- Maandi: `Rahu` + `End`
- Other time-based upagrahas: `Start`

```rust
use dhruv_rs::{
    DhruvContext, EngineConfig, GeoLocation, GulikaMaandiPlanet, RiseSetConfig,
    SankrantiConfig, TimeInput, TimeUpagrahaConfig, TimeUpagrahaPoint,
    UpagrahaRequest, UtcDate, upagraha_op,
};

let config = TimeUpagrahaConfig {
    gulika_point: TimeUpagrahaPoint::Middle,
    gulika_planet: GulikaMaandiPlanet::Saturn,
    maandi_point: TimeUpagrahaPoint::End,
    maandi_planet: GulikaMaandiPlanet::Rahu,
    other_point: TimeUpagrahaPoint::Start,
};

let ctx = DhruvContext::new(engine_config)?;
let request = UpagrahaRequest {
    at: TimeInput::Utc(UtcDate::new(2024, 1, 15, 12, 0, 0.0)),
    location: GeoLocation::new(28.6139, 77.2090, 0.0),
    riseset_config: Some(RiseSetConfig::default()),
    sankranti_config: Some(SankrantiConfig::default_lahiri()),
    upagraha_config: Some(config),
};

let upagrahas = upagraha_op(&ctx, &eop, &request)?;
```

If you are building `BindusConfig` or `FullKundaliConfig`, pass the same
`TimeUpagrahaConfig` into those config structs so Gulika and Maandi stay
consistent across bindus and full-kundali output.
