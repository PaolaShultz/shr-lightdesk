# Development and validation

Use Rust 1.97.1, edition 2024 and Cargo.lock; the package has no external Rust
dependencies or sibling path dependencies. The default binary is always offline.
No build/test command discovers or opens MIDI, audio, DMX, display or network devices.

```sh
rustc -vV
CARGO_INCREMENTAL=0 cargo fmt --all -- --check
CARGO_INCREMENTAL=0 cargo test --locked --all-targets
CARGO_INCREMENTAL=0 cargo clippy --locked --all-targets -- -D warnings
CARGO_INCREMENTAL=0 cargo build --locked --release
./target/release/shr-lightdesk --version
./target/release/shr-lightdesk --help
./target/release/shr-lightdesk gallery artifacts/screens
./target/release/shr-lightdesk simulate
```

Normal tests cover selection/capability atomicity, programmer/hold/storage,
intensity/color arbitration, masters/blackout, mode/grant continuity, explicit
release, proposals, request/revision/retry/reconnect, MIDI identity/edges/pickup,
all-page viewport/font coverage and end-to-end operator/fault CLI workflows.
The agent selects tests. Run focused checks while editing; full normal suite
for shared models, rendering, controller routing, recovery and safety changes.

No historical/exhaustive tests exist yet. Physical device/display tests, combined
CPU/GPU/load/thermal trials, real fixture acceptance and long research/auditions
are separate opt-in sessions, not default tests or implicit CLI side effects.
Existing GigPies and sibling suites need not rerun for this documentation-only
integration change. Hardware verification cannot be inferred from software tests.

## Reproducible previews and interactive review

`gallery artifacts/screens` creates seven original 1920×1080 SVGs and a local
HTML index using the actual bundled bitmap glyph paths. The index opens no
connection, script or external asset. The page itself is only a gallery.
`simulate artifacts/live.svg` updates one SVG after every line command. Refresh
it in a viewer; the future native renderer will consume the same scene primitives.
No viewer is auto-launched and no host font/display configuration is touched.

The optional command `render artifacts/view.ppm` in the loop writes a software
raster of the same scene without external dependencies. SVG is smaller and is
the normal deliverable. For local review this task rendered SVGs to small PNGs
using temporary CairoSVG tooling; these images remain under ignored artifacts.
Do not hand-edit previews to hide a rendering defect. Neither export cost nor
software raster cost is a GPU performance measurement.

All live values reset on exit; save/load is intentionally absent. Do not author a
real show in this simulator expecting it to survive restart. Library IDs/names
are synthetic, and files written by `render`/`gallery` are review artifacts only.
The loop does not change terminal modes, capture the mouse or require a keyboard
raw-mode restoration handler. EOF exits normally.

## Scope, artifacts and publication

The new project is local only, with no remote, installation or deployment. Keep
Cargo.lock alongside source. Keep generated gallery/raster outputs, temporary
research tools, private controller assignments, logs and show files in ignored
`artifacts/` or `user/`. The font asset has explicit OFL provenance; manufacturer
images/manuals are linked instead of bundled. No private recordings were used.

Before any future commit, inspect status and named staged contents, confirm no
private/generated files entered the index, and run `git diff --cached --check`.
Publication requires explicit scope and a reviewed allowlist including the font;
GigPies' publication guard applies to GigPies changes, not automatically to this
new independent repository. No publication guard or speculative CI scripts were
copied here. New reusable scripts need their own review rather than a one-off
runner entering source control.

Check free space/output sizes before substantial builds. Use CARGO_INCREMENTAL=0
and normal target/. Retain useful small drafts and required executables; clean
only known task-owned disposable research/build artifacts after checking processes,
open paths and locks. Never blanket-delete another project's target or user data.
