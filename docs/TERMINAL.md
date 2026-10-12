# Ratatui instrument console

Run from this repository root so Cargo finds the checksum-verified shared package
in `.cargo/config.toml`. Build with Rust 1.97.1, `CARGO_INCREMENTAL=0`, `--locked`,
one job and `/home/shome/p/.gigpies-build.lock`. No native feature is needed.

```
shr-lightdesk terminal --socket-path ENDPOINT --show-id SHOW_UUID --epoch EPOCH
```

Use the existing native command's explicit provider and role options. Startup is
read-only; no role/grant is implied. All existing provider workers, semantic
actions, exact review context and freshness checks remain the authority boundary.
Ctrl+C closes the console and invalidates local input. Existing page/control keys
are preserved. Resize and focus loss invalidate held/review input. Below 160x45
cells the console shows a recovery message and refuses input; it does not mark a
clipped review as presented. Terminal release-event support is queried. Held
safety controls/confirmations that require release edges are refused on an
unsupported path rather than generating a false release.

The [shared presentation contract](../../gigpies/docs/reference/INSTRUMENT_PANEL.md)
owns the Ratatui controls, font licence/authoring, primary Rio/font proof and
fallback rules. Set `SHR_PANEL_FONT='SHR Panel Mono:0.3'` only with that verified
application-local font configuration. Unset NO_COLOR for the colour presentation.
Ordinary Unicode is the readable fallback. No provider spectrum is synthesized.

The shared source lives in GigPies `src/instrument-panel/`. The local vendor copy
is a generated exact-version distribution, refreshed by GigPies'
`scripts/package_panel.py`; edit the owner source, not this copy. No path
dependency, network registry or publication is required for this checkout.

[GP-PANEL](../../gigpies/docs/development/MODULE_IMPLEMENTATION_PLAN.md#gp-panel--shared-physical-instrument-panel)
owns the complete plan, validation results and remaining acceptance gates. Native
and offscreen compatibility frontends remain available. Display screenshots prove
software terminal rendering, not physical audio, lighting or HDMI qualification.
