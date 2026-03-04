# Unified Operations Tracker

Status key:
- `TODO`
- `IN_PROGRESS`
- `DONE`

## Phase S0: Spec

- `DONE` Add `docs/UNIFIED_OPERATIONS_SPEC.md`
- `DONE` Add this tracker

## Phase S1: dhruv_search canonical ops

- `DONE` Conjunction operation structs + dispatcher
- `DONE` Grahan operation structs + dispatcher
- `DONE` Motion operation structs + dispatcher
- `DONE` Lunar phase operation structs + dispatcher
- `DONE` Sankranti operation structs + dispatcher
- `DONE` Ayanamsha operation structs + dispatcher
- `DONE` Tara operation structs + dispatcher
- `DONE` Panchang operation structs + include-mask dispatcher
- `DONE` Node operation backend selector

## Phase S2: dhruv_rs migration

- `DONE` Add `ops` module for conjunction operation
- `DONE` Add op modules for lunar phase and sankranti
- `DONE` Add op modules for remaining families (Tara)
- `DONE` Replace split re-exports with operation-centric API

## Phase S3: CLI migration

- `DONE` Add grouped conjunction command with `--mode`
- `DONE` Add grouped grahan command
- `DONE` Add grouped motion command
- `DONE` Add grouped lunar-phase command
- `DONE` Add grouped sankranti command
- `DONE` Add unified ayanamsha compute command
- `DONE` Add panchang include-mask command path

## Phase S4: C ABI migration

- `DONE` Add unified conjunction C ABI request/response API
- `DONE` Add unified grahan C ABI request/response API
- `DONE` Add unified motion C ABI request/response API
- `DONE` Add unified lunar-phase C ABI request/response API
- `DONE` Add unified sankranti C ABI request/response API
- `DONE` Add unified ayanamsha C ABI request/response API
- `DONE` Add unified tara C ABI request/response API
- `DONE` Add unified panchang C ABI request/response API

## Phase S5: Cleanup

- `TODO` Remove legacy split APIs/commands
- `TODO` Update ABI version and release docs
- `TODO` Complete migration reference docs
