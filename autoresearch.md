# Autoresearch: Minimize Binary Size

## Objective
Reduce the release binary size of `splitwise` CLI from ~5.1 MB to as small as possible, ideally under 2 MB. The binary is a Rust CLI tool with a TUI mode (ratatui) that talks to the Splitwise API over HTTPS.

## Metrics
- **Primary**: binary_size_kb (KB, lower is better) — the release binary file size
- **Secondary**: dep_count — total dependency tree lines (proxy for bloat)

## How to Run
`bash autoresearch.sh` — builds release, measures binary size, verifies functionality.

## Files in Scope
- `Cargo.toml` — dependencies, features, profile settings
- `src/client.rs` — HTTP client (currently uses reqwest blocking)
- `src/main.rs` — entry point, clap CLI
- `src/tui.rs` — TUI mode (ratatui + crossterm)
- `src/models.rs` — serde data models
- `src/display.rs` — CLI output formatting
- `src/config.rs` — config file handling

## Off Limits
- Do not remove any CLI commands or TUI features
- Do not break the `--help` output
- Do not remove the `--tui` flag
- All existing functionality must be preserved

## Constraints
- Must compile without errors
- Binary must be functional (--help works, all commands present)
- No cheating (e.g., UPX compression, external strippers not in cargo)

## What's Been Tried
(auto-populated)

## Ideas
- Replace `reqwest` (blocking) with `ureq` — pure sync HTTP, no tokio/async runtime, way fewer deps
- Add `[profile.release]` optimizations: `strip = true`, `lto = true`, `opt-level = "z"`, `codegen-units = 1`
- Minimize clap features (avoid `derive` if savings are significant, though likely not worth the code churn)
- Use `minreq` instead of `ureq` for even smaller HTTP
- Disable default features on ratatui/crossterm where possible
- Use `serde_json` without `serde` derive — manual impls (probably not worth it)
- Consider `opt-level = "s"` vs `"z"` tradeoffs
