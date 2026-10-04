# Screens, drafts and operating workflow

2026-10-04. Native graphical application with a dense TUI appearance is the target.
Today's implemented backend writes state-driven SVG/PPM drafts; the interaction
loop is ordinary line input. There is no native window, mouse dependency, active
HDMI session or physical stage preview. [Generate the gallery](DEVELOPMENT.md).

## Full-HD frame and visual grammar

Use Desk's 1920×1080 canvas, unmodified Terminus 12×24 glyphs, charcoal panels,
pale text, cyan selection, amber pending/hold and red faults. Selection borders,
`[SEL]`, `PROGRAMMER`, `HOLD`, `PLAYING`, `--` and `STALE` carry meaning without
color. Fixed positions: 48 px status header, page/layer bar at y64, content
between y108 and y912, 144 px controller/result footer at y936. Two actual rows
of eight rotaries and all eight pads are always visible; assignments stay stable.

Stage/programming pages use a 1164 px left area and 684 px inspector with gutters.
The stage plan uses simple fixture blocks and labelled RGB swatches. They are
logical display approximations, not color calibration or photometry. Base and
post-master values are separate. The inspector explicitly shows the first selected
fixture; the selected set is visible, and the loop's `status` lists all selected
attributes. Mixed-value/group-delta graphical editing is next-stage work.

Output status never says LIVE or physical OK in the simulator. `CONFIRMED SIM`
means acknowledged by the mock. The persistent `DMX --` and Health evidence ladder
show no driver, no transport submission and unknown fixture response. Fault/release
panels retain target and command context. Unsupported rotaries are dimmed and
marked; attempting one gives a reason instead of changing a different attribute.

## Full screen map

P0 = necessary for a usable integrated manual console, P1 = useful later.
Rendered means an offline draft, not completion of its planned controls.

| Page | Contents and operator task | Current draft / code | Priority |
|---|---|---|---|
| Stage | Fixture layout/selection; intensity/color; winning source; master/blackout; next/playing cue summary | Rendered; selection and intensity/source ledger work in loop; no physical readback | P0 |
| Programmer | Selected attribute fields, touched mask, mixed/unavailable, record/clear/return, candidate versus confirmed | Rendered; capability-gated scalar edits, global Clear to Hold and scoped release | P0 |
| Library: groups/palettes | Ordered groups; intensity/color/position palettes; selection distinct from applying values | Rendered; three fixed groups, copied-value palette storage/recall; group authoring absent | P0 |
| Playbacks / cue lists | Active versus stored cues, GO/Back/Pause/Off, time remaining, tracked data, playback level/master | Rendered; two static slots and record/update/GO/Off only; no cue list or timing | P0 |
| Effects / timing | Owned FX source, waveform/path, speed/size/phase, fade/delay, effects masked by manual holds | Planned page; footer positions visibly unavailable; no FX generation | P0 basic fades; P1 richer effects |
| Patch / capabilities | Fixture ID, make/model/mode, universe/address, footprint, roles, default/blackout behavior, conflicts | Rendered; synthetic capability table, all actual patch/output identities `--` | P0 |
| Automation | MANUAL/ASSIST/AUTO; proposal/reason/freshness; grants/ranges; holds and return preview | Rendered; injected scalar proposals/bounds/explicit accept; no audio feed | P0 |
| Output health | Control connection, authority freshness, encoding/submission/receiver stages, fault history | Rendered; pending/rejected/disconnected and explicit no-driver evidence | P0 |
| Show / recovery | Shared show UUID, save/load/backup/diff, output safes, mismatch/rearm, module compatibility | Combined Health draft shows limits; no persisted show | P0 |
| Controller / display setup | Separate role identities, learned profile, endpoint ambiguity, test input/feedback, monitor recovery | Planned page; pure identity/profile validation only | P0 before hardware |

Do not hide setup failures inside a stage drawing. An operator must reach Patch
and Health while output is degraded. Planned high-value pixel surfaces are stage
geometry, capability-aware color picker, movement destination path, cue fade
progress and source-activity history. All need labelled basis/freshness; absence
is `--`. Effects/timing controls remain disabled until their owner exposes them.

## Reproducible manual show exercise

Start `cargo run --locked -- simulate`. All commands below work with no automation
and no mouse. Values are percent or degrees. Each command prints its result;
`status` gives the full per-attribute source explanation. `autoack off` makes
pending state visible until a separate `ack`, `reject` or `dropack`.

```text
select 1 2
set INT 60
cue record 1
go 1 1
clear hold
return playback INT
confirm
select 11 12 13 14
set INT 35
set RED 15
set GREEN 30
set BLUE 90
palette record 1
cue record 2
go 2 2
clear hold
return playback INT
confirm
select 11
set INT 0
status
return playback INT
confirm
page playbacks
render artifacts/playbacks.svg
```

The last manual zero wins over PB2. Return previews 0→35% and only changes it on
confirm. Color values remain held after returning intensity; release masks are
per attribute. Use `return playback RED`, etc., deliberately. Deselect never
removes those values. To edit a cue, touch desired attributes then explicitly
`cue update ID`: this **replaces** its stored contents in the mock. Existing
playback keeps its captured snapshot until another GO. Selective merge is planned.
Palette recall enters programmer values for matching selected fixture IDs; it is
not an automatic playback or a universal fixture-type palette.

## Assisted and automatic exercise

```text
select 11
mode assist
propose 11 INT 55
status
accept INT
mode auto
grant INT 10 60
propose 11 INT 90
status
return auto INT
confirm
set INT 42
propose 11 INT 20
status
clear hold
mode manual
```

The 90% proposal is visible separately from the bounded 60% automatic candidate.
The held/manual value stays until Return Auto. A subsequent manual 42% overrides
new proposals. Mode changes do not jump the look. The final Clear empties the
recordable mask but preserves 42% in Hold. This is injected state, not Lux analysis.

## Pending, refusal and uncertain delivery exercise

```text
autoack off
master 50
status
ack
master 70
reject
master 80
dropack
status
reconnect
status
autoack on
```

`dropack` applies at the simulator then disconnects without delivering its reply.
The display keeps stale 50% until reconnect retrieves authoritative 80%; there is
no resend. Disconnect with an unapplied request discards it. `refresh` is an
explicit snapshot resynchronization and also resets input gestures. No live
output-loss/fixture-rearm policy is exercised by these local operations.

## Keyboard and controller drafting limits

Every implemented command is in `help`: selection/group/multi/layer, all seven
pages, supported attributes, cue/palette storage, masters, modes/grants/proposals,
release preview, MIDI injection and recovery. This is a usable offline authoring
loop, not yet a complete controller-only/native-console workflow. Native function
keys, focus/scroll editors, typed modal prompts, graphical color editing and
movement/cue timelines are planned. The current gallery labels pages by name
rather than advertising working F-key shortcuts.

The simulated control map is in [CONTROLLER.md](CONTROLLER.md). In the loop,
`key 48` selects fixture slot 1; `key 60` addresses slot 13 (empty in this rig).
`layer groups` makes `key 48` select FRONT. `pad 5` toggles MULTI; `pad 6` deselects.
`midi light-sim 176 16 76` injects K01 absolute input; actual devices are never
opened. Use `encoder 1 twos` then value 1 or 127 for relative steps, after choosing
a supported uniform selection. The gallery is reproducible original artwork,
not an image of a running commercial console or future physical installation.
