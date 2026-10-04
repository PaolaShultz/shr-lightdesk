# Implementation status

Checkpoint **2026-10-04**, developed and validated on actual hostname **rpi5**.
This is the independently buildable offline foundation of GigPies' lighting
operator surface. The native window and real lighting engine integration remain
planned. No physical output or shared hardware/load acceptance occurred.

## Implemented and offline-validated

- Eight synthetic fixtures with declared capabilities: two dimmers, four RGB
  fixtures and two RGB movers with pan, tilt and zoom; three fixed groups.
- Independent selection, fixture/group key layers, additive selection and reset;
  capability-checked scalar programmer edits with explicit units.
- A separate mock authority behind `Authority`: touched programmer values, holds,
  copied-value cue/palette storage, two static playback slots, intensity HTP,
  activation-ordered non-intensity playback and manual override including zero.
- MANUAL/ASSIST/AUTO, injected proposals, scoped bounds and explicit acceptance;
  mode/grant changes preserve the current look. Clear to Hold and reviewed,
  per-attribute Return Playback/Auto are distinct operations.
- Intensity-only simulated grand master/blackout; pending, confirmed, rejected
  and disconnected command states; revision/epoch checks, duplicate suppression,
  stale-preview refusal and fresh snapshot recovery without replaying lost input.
- Pure synthetic MIDI translation: verified-injection identity, distinct keys
  and pads, press/release edges, absolute pickup, three relative encoder modes,
  fine adjustment, sixteen visible rotary positions and eight pad positions.
- A mouse-free line-command interaction loop and seven original, state-driven
  1920×1080 SVG/PPM screen drafts using Desk's licensed Terminus font. Rendering
  distinguishes selection, programmer, holds, playback and simulated final state;
  physical DMX remains unavailable/unknown.

The mock has no fixture output, clock, fades, cue tracking, effects or persistence.
Its bounded static semantics are acceptance scaffolding for the UI, not production
Lux code. Request IDs assume one in-process writer lifetime. Snapshot structs are
trusted in-process values, not a validated network protocol. See the
[control contract](CONTROL_CONTRACT.md) for these limits.

## Validation evidence

Rust **1.97.1** (`8bab26f4f`, 2026-07-14), edition 2024, aarch64-unknown-linux-gnu;
LLVM 22.1.6. No external Rust dependencies or sibling path dependencies.
Commands are reproducible in [DEVELOPMENT.md](DEVELOPMENT.md).

| Check | Result |
|---|---|
| Complete normal suite, `cargo test --locked --all-targets` | **27 passed:** 24 state/contract/controller/recovery/rendering tests, 3 CLI workflow tests; none ignored |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --locked --all-targets -- -D warnings` | Passed |
| `cargo build --locked --release`, binary version/help | Passed |
| Gallery generation and visual inspection | All seven Full-HD screens inspected; actual font paths and software raster dimensions tested |
| Manual programming and uncertain-delivery workflows | Automated CLI coverage; no device discovery/open side effects |
| GigPies documentation whitespace / publication guard | Diff check passed; existing complete index passed 167 entries; integration edits remain unstaged |
| Existing sibling preservation | Before/after source hashes unchanged for Desk, Lux, DAW and FX |

Intentionally not run: physical MIDI/LED/DMX/audio tests, native GPU/HDMI tests,
combined load/thermal/latency trials, historical/exhaustive research and private
media auditions. They need their own relevance and session scope. GigPies and
sibling production suites were not rerun because their production code did not
change. Combined CPU/GPU/memory/response figures in the blueprint are unmeasured
acceptance targets; this suite does not certify them.

Useful local evidence remains under ignored `artifacts/screens/`: seven SVGs,
seven small PNGs and a local HTML index (about 944 KiB total). Source hashes and
the updated GigPies architecture preview are also retained. Temporary manufacturer
downloads and the review Python environment were removed after checking open
process paths and locks, recovering about **104 MiB**. Normal `target/` is about
67 MiB and is retained for development; about 31 GiB disk space remained. Larger
pre-existing GigPies/DAW build directories were reviewed and left untouched.

## Reference state and ownership

GigPies started at `eea5267` with existing uncommitted integration work, which was
preserved. Desk was an uncommitted independent project; its current files and
font/render source hashes are recorded locally and in [notices](../THIRD_PARTY.md).
Read-only reference revisions:

| Project | Inspected revision |
|---|---|
| SHR Lux | `ba4ccd92656e6d2a3cbc6424cdc2a017d3e4f14d` |
| SHR DAW | `d93447d1ae391e7910c54965f886ed39baedab1c` |
| SHR FX | `cd943c6a5cbe3c06be4e9968f54f685e97baf84f` |

The superseded GigPies Brain console plan was preserved byte-for-byte in its
archive before the dual-desk revision. Existing interactive sessions were left
running. No peer worker, service change, host display/font configuration,
publication or deployment was started. The new project has no remote.

## Next concrete implementation

1. **Lightdesk L1:** add a native window over the existing scene/font model;
   implement keyboard focus, typed editors, selection navigation and controller
   modal actions. Keep the mock explicit and test renderer loss/recreation.
2. **Lux LX1, separate write authorization:** agree fixture capabilities and
   source/arbitration snapshots, engine-owned programmer/holds and static cues;
   implement an authoritative null-output path and versioned contract fixtures.
   Lightdesk can then add a read-only adapter before enabling commands.
3. **GigPies GI1:** show/module compatibility and persistent display/controller
   role assignment with independent workers, ambiguity workflow and sole LED
   ownership. No ALSA port-number identity or generic shared input queue.
4. **Lux timing/recovery, then separately scoped hardware gates:** explicit
   release timing, persistence, output-loss/rearm policy; second-device mapping
   and two-monitor usability; fixture output; combined workload acceptance.

Full ownership, acceptance conditions and useful later work are in the
[blueprint](BLUEPRINT.md), [capability matrix](CAPABILITIES.md) and
[GigPies integration backlog](../../gigpies/docs/BRAIN_CONSOLE_PLAN.md).
