# SHR Lightdesk

**The lighting operator surface for GigPies.** Manual look building and playback
come first; ASSIST and AUTO are optional operating modes. It pairs a second
1920×1080 monitor and an independently assigned MIDI keyboard controller with
SHR Desk's audio console on the same Brain. SHR Lux remains the lighting engine.

**Implemented software:** real private-IPC Lux snapshot/control clients, typed
operator actions and reviewed engine release, plus an optional winit/wgpu native
frontend over the bundled scene/font renderer. The explicit simulator remains
acceptance scaffolding. Native compilation and CPU/offscreen acceptance are
tracked in [Status](docs/STATUS.md); physical output, operator displays and
controllers have not been verified.

```sh
CARGO_INCREMENTAL=0 cargo run --locked -- simulate
CARGO_INCREMENTAL=0 cargo run --locked -- gallery artifacts/screens
# Open artifacts/screens/index.html to inspect the screen drafts.
```

Try `select 11 12`, `set INT 45`, `set RED 15`, `cue record 1`, `go 1 1`,
`clear hold`, `return playback INT`, then `confirm`. `status` explains each
attribute's source; `help` lists the complete keyboard workflow. A mode change
keeps the current look. Show state is in memory and disappears on exit.

`simulate artifacts/live.svg` redraws a state-driven SVG after each command
(refresh that file in a viewer). It is a review backend, not a browser application
and is separate from the optional native GUI. The package builds independently with Rust 1.97.1,
edition 2024 and pinned serde/serde_json plus Linux credential-checking libc dependencies.

| Read | Purpose |
|---|---|
| [Blueprint](docs/BLUEPRINT.md) | Product, ownership, dual-desk operation and delivery gates |
| [Console study](docs/CONSOLE_STUDY.md) | Manufacturer screens and workflows, versions and limits |
| [Screens and workflow](docs/SCREENS.md) | Full screen map, drafts and a reproducible operator exercise |
| [Controller plan](docs/CONTROLLER.md) | Sixteen rotaries, eight pads, selection layers and separate ownership |
| [Control contract](docs/CONTROL_CONTRACT.md) | Lighting arbitration, release, recovery and engine obligations |
| [Capability matrix](docs/CAPABILITIES.md) | Existing, missing, later and excluded functions |
| [Status](docs/STATUS.md) | Evidence, limitations and concrete next implementation |
| [Development](docs/DEVELOPMENT.md) | Reproducible build/checks, artifacts and publication boundary |

Code MIT; bundled Terminus Font SIL OFL 1.1. See [third-party notices](THIRD_PARTY.md).

Read a real reviewed Lux snapshot with explicit identity:

```sh
shr-lightdesk --provider lux --snapshot-file snapshot.frames --show-id 11111111-1111-4111-8111-111111111111 --epoch 9
# Explicit private same-UID endpoint instead of file input:
shr-lightdesk --provider lux --socket-path /absolute/owned-private-dir/lux.sock --show-id 11111111-1111-4111-8111-111111111111 --epoch 1
```

The file contains four-byte big-endian length-prefixed C-LIGHT:1 producer pages.
Provider mode validates the complete inventory before displaying provenance and
remains read-only. No startup auto-connect occurs. The accepted LX03 provider is
synthetic, volatile, null/disarmed; submitted/observed and physical light stay
unknown. Simulator commands remain independently available under `simulate`.

Saved cues, palettes and playing looks may span every advertised fixture attribute,
even when assembled through several edits. The 64-target limit applies to each
incoming edit; it does not truncate or reject a complete accumulated look.

Cached logical state is explicitly `Fresh`, `Stale`, or `Unavailable`, independently
of always-unknown physical output. A complete compatible receipt is Fresh for
2000ms; invalid/incomplete pages, expiry, timeout, or link loss retain last-known
values as Stale. Before any complete snapshot it is Unavailable. A file/one-shot
socket report describes freshness at receipt, not a live subscription.

Real null-output Lux control is an explicit headless mode:

```sh
shr-lightdesk lux-control --socket-path /absolute/owned-private-dir/lux.sock \
  --show-id 11111111-1111-4111-8111-111111111111 --epoch 1 --script operator.txt
```

Use the actual current provider epoch (restart reserves a new one). The client
reads a compatible complete snapshot before `grant`, uses a new writer identity,
renews at 500ms, and holds at most one mutation. A private same-UID 0700 parent and
0600 Unix socket are required. No endpoint is discovered or started automatically.
The default Simulator and `--provider lux` read-only mode remain explicit.

Example script (values are integer tenths in the advertised unit):

```text
grant
select fixture-11
touch intensity 700
record cue look-a
go look-a playback-a
touch intensity 0
clearHold
preview intensity
release
release-input
wait 750
status
checkpoint
quit
```

`record` copies programmer values; it does not GO or release. `clearHold` preserves
Lux's current look. `preview` displays Lux's exact destination/token; `release`
commits that reviewed finite 500ms transition. Hold and engine release current
are displayed separately. `cancel` cancels an unused engine preview. Preview
support requires accepted `lx04-v1` or `lx04-durable-v1`; checkpoint requires the
latter's advertised capability. Durability is volatile/checkpointed/error as Lux
reports it. No ACK, transition or checkpoint establishes physical light.

`blackout off` and `update cue|palette ID` require a context/revision/lease-bound
confirmation. `confirm`/`release` are Enter-down edges: `release-input` or `enter up`
is required before another confirmation. Selection/page/mode/focus/input loss
invalidate confirmations; reconnect drops intents and requires release/pickup,
a fresh complete snapshot and new grant. Writes stop on output/control loss while
last-known values remain stale; the client never clears the engine look or forces
blackout. `status-json PATH` exports cached state, last ACK and inventory with null
submitted/observed separately from physical unknown.

Scripts are UTF-8, at most 64KiB/256 lines and 4096 bytes per line. `wait` is bounded
0..2000ms and renews while waiting. Stdin renews during idle input; each invocation
is bounded to 25s/256 commands against the provider's 30s connection lifetime.
Use a new explicit invocation/reconnect for longer work. Absolute typed edits are
sent synchronously; there is no unsent command queue or discrete-command coalescing.
An unconfirmed operation retries only the same bytes/ID at100/250/500ms within the
lease, then reads fresh state and disables the old writer. It is never replayed
under a new ID after reconnect. A confirmed ACK survives a later failed state read,
with the cached view marked stale and writes disabled.

The native frontend is optional and connects only to the explicitly supplied Lux
endpoint; it does not start a provider or discover devices:

```sh
cargo +1.97.1 build --locked -j1 --release --features native
# Explicit window launch requires an independently authorized display session:
shr-lightdesk native --socket-path /absolute/private/lux.sock --show-id UUID --epoch 1
# Device-free real-provider actions and the same scene/font raster:
shr-lightdesk offscreen --socket-path /absolute/private/lux.sock --show-id UUID --epoch 1 --script operator.txt --output artifacts/lux
```

The native window has semantic controls: Left/Right focus fixture slots; Space
selects; Up/Down focus attributes; E opens a detached typed value editor (percent
or degrees); R opens a cue record destination; G opens CUE_ID PLAYBACK_ID; C
reviews Clear-to-Hold; P requests the engine's finite release preview; B reviews
blackout; M reviews manual/assist mode. F1–F5 select the real provider pages and
F6 requests a writer grant. Enter reviews a detached draft, then a new Enter edge
confirms only after the full exact review has been shown. PageUp/PageDown scroll
all targets; F12 shows help; Escape cancels; Backspace edits. RGB and position
editors take three/two coherent component values. Unadvertised effects/groups
remain unavailable. `:` opens the secondary command console; its confirmation
uses the same review gate. Focus/resize/device loss/overflow preserve detached
content while invalidating queued input, held actions and confirmations. Provider retries check cancellation before every send, preserving an
already-submitted uncertain outcome and requiring explicit reconnect.

Native and worker-offscreen modes require a live accepted GP09 `lighting-desk`
lease before writer actions. Without binding they remain keyboard-only read-only.
Supply the reviewed provider executable, private role registry and explicit
injected acquisition JSON (physical identity remains unverified):

```sh
shr-lightdesk native --socket-path /absolute/private/lux.sock --show-id UUID --epoch 1 --role-provider /absolute/gigpies-role-lease --role-dir /absolute/private/roles --role-request lighting-acquire.json
```

Each desk owns a private broker child/pipes. Verification and reply deadlines are
500ms; lease/child/identity loss fences new input, Lux sends/retries and the separate
latest LED mailbox. The mailbox is an injected desired-state seam; it never opens
MIDI or encodes physical LED bytes. Reacquisition requires a fresh request/new
process, release of held input and a new Lux grant. Provider reconnect takes a
fresh writer and never replays old intent. Cached state and physical uncertainty
remain visible after failure. The CPU raster refreshes at10Hz with aspect/integer
letterboxing; 60Hz responsiveness and physical GPU/display behavior are unverified.

Opt-in Lux `lx05-v1` snapshots are supported as read-only metadata. Current
analysis health/confidence and intensity proposals are distinct from each retained
AUTO contribution. Health shows that contribution's original source, window and
calibration provenance, even after source reconnect. Beat, downbeat and harmony
remain unavailable. Lux supplies resolved values and winning sources; Lightdesk
does not calculate AUTO arbitration or expose calibration/grant controls.
Existing leased human commands remain available with this schema. Timed release
and durable recovery require their validated advertised capabilities; analysis
alone does not enable them. Legacy LX01–04 decoding remains strict.

Automatic read-only observation continues without a GP09 lighting role and after
role loss. Grants, renewals and mutations still require the live role. Explicit
disconnect and transport failure stop polling until an explicit reconnect; a role
loss during an already submitted operation may therefore leave the surface unavailable.
Focus and resize preserve typed commands and detached semantic editors in memory,
while invalidating held input, queued commands and confirmations. Retained semantic
drafts require released input, fresh authority and a new complete review before
confirmation; changed selections refuse review. Esc remains explicit cancellation.
