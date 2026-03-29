# Layered Configuration Spec (v1)

Date: 2026-03-04  
Applies to: `dhruv_cli`, `dhruv_ffi_c`, `dhruv_rs`, `dhruv_config`

## Goal

Provide optional file-based configuration while keeping core computation crates stateless.

This spec covers policy/default configuration only. Per-call input variations should be carried by typed request/context values, not by layered config files or suffixed public function names.

## Precedence

Highest to lowest:

1. Explicit request/context inputs, function arguments, CLI flags, or non-null FFI config pointers
2. Operation-specific config section from file (`operations.<family>`)
3. Common config section from file (`common`)
4. Recommended defaults (when enabled)

## Defaults Mode

- `recommended` (default): use built-in recommended defaults when values are not provided.
- `none`: unresolved required values produce strict errors.

## File Formats

- TOML and JSON are both supported.
- Unknown keys are strict errors (`deny_unknown_fields`).
- `version` must be `1` (or omitted/`0`, which normalizes to `1`).

## Discovery Order

`--config <path>` / explicit path is highest priority.  
If not provided and config is not disabled:

1. `DHRUV_CONFIG_FILE`
2. Platform user config paths
3. Platform system config paths

### Linux / Unix (non-macOS)

1. `$XDG_CONFIG_HOME/dhruv/config.toml`
2. `$XDG_CONFIG_HOME/dhruv/config.json`
3. `$HOME/.config/dhruv/config.toml` (fallback when `XDG_CONFIG_HOME` unset)
4. `$HOME/.config/dhruv/config.json` (fallback when `XDG_CONFIG_HOME` unset)
5. `/etc/xdg/dhruv/config.toml`
6. `/etc/xdg/dhruv/config.json`
7. `/etc/dhruv/config.toml`
8. `/etc/dhruv/config.json`

### macOS

1. `$HOME/Library/Application Support/dhruv/config.toml`
2. `$HOME/Library/Application Support/dhruv/config.json`
3. `/Library/Application Support/dhruv/config.toml`
4. `/Library/Application Support/dhruv/config.json`

### Windows

1. `%APPDATA%\\dhruv\\config.toml`
2. `%APPDATA%\\dhruv\\config.json`
3. `%PROGRAMDATA%\\dhruv\\config.toml`
4. `%PROGRAMDATA%\\dhruv\\config.json`

### Ambiguity Rule

If both `config.toml` and `config.json` exist in the same candidate directory, loading fails with an ambiguity error.

## CLI Behavior

- Global flags:
  - `--config <path>`
  - `--no-config`
  - `--defaults-mode <recommended|none>`
- `config-show-effective` prints resolved effective config per family.
- Engine kernel paths (`--bsp`, `--lsk`) are now optional when provided via layered config.

## Rust API Behavior (`dhruv_rs`)

- High-level API is context-first via `DhruvContext`.
- Global singleton APIs were removed from the public surface.
- Context can hold an optional `ConfigResolver`.
- `DhruvContext` or per-operation request values carry invocation-specific inputs; layered config remains for defaults and behavior/policy knobs.

## C ABI Behavior (`dhruv_ffi_c`)

- Added resolver lifecycle APIs:
  - `dhruv_config_load(path_utf8, defaults_mode, out_handle)`
  - `dhruv_config_free(handle)`
  - `dhruv_config_clear_active()`
- For operation families, config pointers are nullable.
  - `NULL` uses resolver + layered precedence.
- ABI request/context structs should carry invocation-specific inputs; config handles/pointers should carry behavior and defaults rather than replacing per-call request data.
  - Non-null pointer remains explicit highest-priority override.
