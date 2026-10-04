# Development and validation

Use Rust 1.97.1, edition 2024 and Cargo.lock. Pinned serde/serde_json/libc support
the accepted Lux protocol. Optional `native` pins winit0.30.12, wgpu0.20.1 and
pollster0.3.0; no sibling path dependencies exist. The default startup stays offline;
real provider/native modes require explicit selection. Normal tests never discover
or open physical MIDI, audio, DMX or display endpoints. Private UDS fixtures and
an explicitly selected CPU Vulkan ICD are software acceptance only.

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
Sibling suites run when their own behavior changes. Hardware verification cannot
be inferred from software tests.

## Reproducible previews and interactive review

`gallery artifacts/screens` creates seven original 1920×1080 SVGs and a local
HTML index using the actual bundled bitmap glyph paths. The index opens no
connection, script or external asset. The page itself is only a gallery.
`simulate artifacts/live.svg` updates one SVG after every line command. Refresh
it in a viewer; the optional native renderer consumes the same scene primitives.
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

Reviewed source publication is authorized. The independent versioned
[publication policy](PUBLICATION.md) defines complete-index and outgoing-history
checks, hooks and the exact font/licence exception. Enable the hooks after checking
existing configuration, inspect named staged content and run the publication guard.
New reusable scripts require an explicit policy entry and review. Keep Cargo.lock
with source. Provider binaries, private roles, generated renders and worker evidence
stay outside Git. Source pushes do not imply a binary release, installer, service
deployment or hardware acceptance. The licensed font remains unmodified.

Check free space/output sizes before substantial builds. Use CARGO_INCREMENTAL=0
and normal target/. Retain useful small drafts and required executables; clean
only known task-owned disposable research/build artifacts after checking processes,
open paths and locks. Never blanket-delete another project's target or user data.


## Task0009 native/offscreen checks

Follow the central [one-build-slot-per-host procedure](https://github.com/PaolaShultz/gigpies/blob/main/docs/PARALLEL_WORK_PLAN.md#one-build-slot-per-host).
A parent process holds the shared nonblocking lock throughout each Cargo command;
if occupied, continue independent source work and retry later. With the host lock
path set by the coordinator, standalone commands are:

```sh
export GIGPIES_BUILD_LOCK=/home/shome/p/.gigpies-build.lock
export CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1
flock -n "$GIGPIES_BUILD_LOCK" cargo +1.97.1 test --locked -j1 --all-targets
flock -n "$GIGPIES_BUILD_LOCK" cargo +1.97.1 test --locked -j1 --all-targets --features native
flock -n "$GIGPIES_BUILD_LOCK" cargo +1.97.1 clippy --locked -j1 --all-targets --features native -- -D warnings
flock -n "$GIGPIES_BUILD_LOCK" cargo +1.97.1 build --locked -j1 --release --features native
flock -n "$GIGPIES_BUILD_LOCK" cargo +1.97.1 fmt --all -- --check
# Explicit CPU-only software Vulkan: refuses missing selection or non-CPU adapter.
env -u DISPLAY -u WAYLAND_DISPLAY VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json flock -n "$GIGPIES_BUILD_LOCK" cargo +1.97.1 test --locked -j1 --features native --lib software_wgpu_upload_shader_readback -- --ignored --nocapture
```

No test launches an operator window. Default native tests leave the CPU-Vulkan
check ignored until the explicit ICD command above is run. It shares the actual
native texture/font-upload, shader and pipeline creation functions, renders64x36
and reads back two exact pixels/checksum. This does not measure physical GPU/HDMI
behavior. The standard framebuffer remains1920x1080 with nearest sampling.
The frontend currently refreshes at10Hz; provider maintenance runs independently
at500ms and uses the accepted25s connection lifetime. Use explicit reconnect for
longer interaction. Shared load/thermal/60Hz and physical role identity tests are
intentionally not part of these software checks.


Real producer integration is opt-in because it requires exact root-accepted
executables, not device discovery. Set explicit absolute artifact paths:

```sh
export GP_ROLE_BROKER=/absolute/reviewed/gigpies-role-lease
export GP_LUX_PROVIDER=/absolute/reviewed/lux-service
flock -n "$GIGPIES_BUILD_LOCK" cargo +1.97.1 test --locked -j1 --features native --test native_provider -- --ignored --nocapture
```

These tests start only owned synthetic/null-output children and private UDS/pipes,
join their children and remove their own temporary directories. They exercise
actual semantic programming, record/GO/zero/Hold/release, reviewed mode/blackout,
complete-review refusal (including console whitespace aliases), role child loss,
role timeout, distinct global CAS/live generations and recovery without input
replay. GP09 intended assignments remain durable during ordinary release/crash;
tests remove only their own disposable private registry after all children exit.
The ignored software GPU test and ignored producer tests must be reported
separately from the default normal suite and from physical acceptance.

Offscreen accepts explicit injected semantic scripts (`@grant`, `@select 1`,
`@edit`, `@text 70`, `@confirm`, `@up`, `@review-all`, `@confirm`, `@record cue`,
`@go`, `@clear`, `@preview`, `@mode`, `@blackout`, `@page programmer`). Values in
semantic drafts use percent/degrees; legacy raw `touch` uses integer tenths.
`@review-all` is an explicit headless acceptance gesture, not a physical operator
observation. The same bounded worker/semantic action path is used by the window.
Supply the same `--role-provider`, `--role-dir`, `--role-request` arguments as the
native frontend to authorize writer actions. Without a role, offscreen is read-only.
Exports include trusted provider state, the scene/font raster, and every complete
review page when a modal is present; no target text is silently truncated.
