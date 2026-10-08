# Implementation status

## Task0020 configured analysis and operator workflow — 2026-10-08

Implemented configured `lx05-v2` strict bindings, current and retained provenance,
explicit calibration start/finish, selected-fixture intensity cap/TTL grants,
reviewed AUTO entry and ASSIST exit/revoke. Native K/L/A/T/X use the same
semantic authority path as the headless commands. Source/calibration identity is
frozen into each material review independently of the engine revision; changed
identity refuses confirmation after maintenance. ACKs validate requested scope,
cap, exact expiry, issue revision and source basis. Legacy LX05 v1 stays supported.

Actual configured synthetic GigPies -> Lux -> semantic Lightdesk episode passed:
17 logical inputs, nondefault bindings including input17, custom show/two fixtures,
calibration, explicit scoped AUTO, unrelated fixture unchanged, source loss,
retained provenance, fresh source epoch without automatic grant, programmer zero,
Hold, explicit revoke and disarmed durable restart. Full current-source default/
native gates and final independent review are recorded in the private task handoff.
No physical output, display or controller acceptance is claimed.


The native LD-02 software frontend, actual Lux snapshot/control client and
read-only LX05 analysis compatibility are implemented and software-validated against
accepted provider executables. Validation covers default/native builds, strict
capability checks, source loss/reattachment, human controls and durable disarmed
recovery. Current counts are recorded below. No physical output, operator display,
MIDI/LED or combined hardware/load acceptance is claimed. The dated foundation
evidence is preserved.

## Read-only observation and draft recovery fixes — 2026-10-06

The frontend continues automatic snapshot observation without GP09 write authority.
Role loss retires local authority once; grants and mutations remain fenced. Explicit
disconnect and transport failures still require explicit reconnect. Snapshot reads
preserve transport/session checks while bypassing the role-only write guard.
Focus and resize retain command text and detached semantic drafts while invalidating
held input, queued commands and review progress. Fresh validation and a new complete
review precede confirmation. Synthetic private-socket regressions exercise unbound
polling, write refusal, role-loss reads, disconnect and retained-draft review.
No live Lux, broker, native-window or physical acceptance is implied.


Local offline validation: **109 default / 110 native tests passed**,
with 4/5 opt-ins intentionally skipped. Both warning-denied Clippy
configurations, formatting and 9 Python tests passed. Reproduce with
`CARGO_INCREMENTAL=0 cargo test --locked -j1 --all-targets`, then the same command
with `--features native`, under the shared build lock. Historical, physical,
external-artifact and explicit rendering campaigns were not run. No publication
or physical/native-window activation was performed.

## Original offline foundation

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
LLVM 22.1.6. This dated dependency-free foundation predates the accepted codec/client
and optional native graphics dependencies; no sibling path dependencies exist.
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

## GigPies integration planning — 2026-10-04

[Owning GigPies plan](GIGPIES_IMPLEMENTATION.md) records scoped tasks, contract dependencies,
validation and launch instructions. This is planned work; existing implementation
and hardware status above are unchanged.

## Task0008 headless operator increment — 2026-10-04 / rpi4

LD-01 review defects corrected; coordinator accepted corrective R2 source.
LD-02 headless operator model is implemented: bounded event dispatch, independent
latest-state LED mailbox, generation-checked lighting role observations, explicit
pending/confirmed authority path, focus/key/device/overflow recovery, and abstract
renderer loss plus resize/hit-selection continuity. See
[owning progress](GIGPIES_IMPLEMENTATION.md) and `src/operator.rs`.
Native winit/wgpu, actual workers/device bindings and hardware validation remain
pending. Simulator remains explicit; this is no production Lux replacement.

Task0008 offline validation: **44 normal tests passed** (24 existing contracts/
rendering, 11 interaction, 5 event model, 4 CLI), none ignored. Focused suite
40 passed; formatting, warning-denied Clippy, release/version/help and both
invalid-draft CLI reproductions passed under the required P4-B build turn.
Physical/native/production integration classes remain intentionally unrun.

Task0008 pass2 LD02 corrections: keyboard-focus recovery preserves assigned MIDI
queue/edges, invalidated modal intent refuses visibly, pad feedback shares the
input action mapping, and exhausted role generations fail closed. **44 focused
normal tests passed**, formatting passed; coordinator R2 accepted; complete
post-adapter validation remains pending. LD03 waits for accepted LX03 wire bytes.
No physical/native verification or endpoint opening occurred.

Task0008 LD03 real read-only consumer is implemented: complete bounded framed JSON
validation/page assembly before trusted state, stable IDs/advertised capabilities,
visible perattribute provenance, and explicit file/private-UDS headless CLI.
The accepted LX03-R2 corpus is retained byte-for-byte with root/original provenance;
private executable is outside Git. Complete normal **58 tests passed** in the final
batch; formatting, warning-denied Clippy, release/version/help and private service
read-only/wrong-epoch acceptance passed. All own service children exited and
temporary endpoints were removed.
No command authority, engine release, durable restart, native or physical verification
is claimed. Simulator remains independently selectable and explicitly labeled.

Task0008 pass3 LD03 is coordinator-accepted offline: current62 normal tests, fmt,
Clippy, release, release extrema regression and actual accepted LX03 private IPC
passed. Explicit Fresh/Stale/Unavailable retains cached values through malformed,
partial, expired, timeout and disconnected observations; only a validated complete
snapshot restores Fresh. Contributor inventory follows actual Lux convention.

LD04 real null-provider client/operator implementation is under coordinator review.
75 normal tests plus Clippy/release and first actual accepted LX04 private IPC batch
passed; latest status export and regression additions await current-source build.
Actual provider touch/record/GO/zero/Hold, engine preview/release, renew, checkpoint,
two crashed disarmed restarts and exact lost-ACK retry have evidence. Final acceptance
and precise current-source validation follow in the owning implementation plan.
No native/physical/shared-load verification or production publication occurred.

LD04 corrective R2 current-source validation now completes: **80 normal tests**,
fmt, Clippy and release passed. Expanded real accepted LX04 IPC verifies sampled
release progression/endpoint, renewal, status export, copied cue/GO/zero/Hold,
checkpoint/two crashed disarmed restarts, exact lost-ACK retry one mutation,
reconnect without intent replay and held Enter refusal plus engine preview cancel.
Children joined/endpoints removed; coordinator R2 review is pending. Historical,
exhaustive, native/GPU, physical MIDI/LED/DMX/audio and shared load were skipped.

LD04 final R3 review input: **81 normal tests**, fmt, Clippy, release and expanded
actual accepted LX04 IPC passed on the corrected completed-frame deadline path.
Independent coordinator final acceptance is pending; software implementation and
offline evidence are complete, native/physical/shared-load gates remain unchanged.

### Task0009 LD02 native software continuation (validation in progress)

Optional pinned winit0.30.12 X11/rwh_06, wgpu0.20.1 WGSL and pollster0.3.0 were
reviewed by root with a4GiB growth budget and one helper-held jobs1 build slot.
`native` routes ordinary typed keyboard intents to the accepted Lux operator on
an independent worker. `offscreen` exercises real Lux commands and produces the
same scene/font SVG/PPM and trusted JSON without opening an adapter or display.
Capabilities, programmer, human Hold, playback, engine release current/transition
and final intent are provider data; no lighting algorithms are duplicated.
Input is bounded16, provider updates coalesce independently, lifecycle generation
invalidation bypasses queue saturation, and expired UI views retain values as
Stale while hiding reviews/writer availability. Device/surface loss recreates
graphics resources while preserving provider state and discarding detached input.
GP09's exact root-accepted executable/protocol/corpus is consumed by a private
per-desk child/pipes client. Role verification/loss gates writes and generation
fences input/provider retries/LED mailbox; no competing role registry or physical
LED encoder/driver is implemented. Semantic keyboard actions use the existing
Action/Draft vocabulary and actual provider client. Complete wrapped review pages
must all be presented successfully before confirmation; console confirmation
uses the same context gate.
Current tests/build results will be appended after completion. The10Hz CPU-raster
frontend is useful software, not a measured60Hz or production hardware claim.

Task0009 pass1 current-source software evidence: **91 default normal tests passed**
(three producer tests explicitly ignored in the default run), then **three actual
accepted GP09/Lux integration tests passed**. Native-feature all-target Clippy with
`-D warnings` passed. The explicit CPU-only Vulkan test passed the shared native
texture/font upload, shader/pipeline and64x36 readback: Mesa llvmpipe CPU, exact
left/right pixels102/223/211/255 and16/21/29/255, byte sum1281024. Real-provider
worker offscreen state and complete engine-review pages were generated and
visually inspected. All own producer children/endpoints were joined/removed.
Native full normal feature-suite completion and native release are **pending**
continuation; the earlier first compile/test errors were repaired and remain
private evidence, not successful validation. No actual window, hardware display,
physical MIDI/LED/DMX/audio or shared-load acceptance occurred. The complete LD02
software gate is not yet claimed at this bounded handoff.

Native-feature focused frontend suite: **six passed**, including portrait/narrow/
minimized/integer viewport, complete per-context review coverage, provider identity,
all pages/font bounds and stalled-view expiration. Final formatting check passed.

Task0009 pass2 native continuation verified all77 frozen pass1 files byte-for-byte.
The frozen full native-feature all-target suite passed **92 tests**, with four
explicit opt-in tests ignored. This precedes pass2 readability corrections: native
queued zero-size redraw refuses presentation, fatal renderer initialization/recovery
errors propagate to CLI, and material operator review/value/provenance text replaces
raw mandatory token JSON. Review lines freeze with the exact confirmation context;
current-source validation and refreshed actual-provider visual acceptance are pending.
LX05 compatibility remains pending root-accepted producer artifacts/authorization.

Task0009 pass2 LD02 software validation is complete on the root-reviewed material
source: **96 default / 97 native normal tests passed**; three actual accepted
GP09/Lux independent-process tests and the explicit CPU-only wgpu64x36 readback
passed (DeviceType::Cpu, checksum1281024). Default/native warning-denied all-target
Clippy, fmt check, both release builds/version/help and final private offscreen
exports passed. Root independently accepted the material review source and visually
accepted all seven real-provider frames. Successful-frame coverage remains bound
to every frozen material line; raw native console mutations share that gate.
All owned children joined and temporary endpoints were removed. No physical
endpoint, operator window, deployment, publication or shared-load test occurred.

LX05 root-accepted opt-in wire/command/feature data has been hash-verified and
retained in `tests/fixtures/lx05/`. **Consumer implementation remains pending:** the
current decoder deliberately still rejects that new schema. A new bounded pass
must add strict read-only analysis status and retained per-fixture AUTO provenance,
then run focused/default/native/fmt/Clippy/releases and actual accepted new-service
interop. This describes the frozen pass2 handoff; pass3 progress follows below.
No calibration or writable AUTO grant UI was added. Earlier validation above covers
the legacy-compatible LD02 increment, not LX05 compatibility.

### Task0009 pass3 LX05 read-only compatibility

The strict consumer gates `lx05-v1`; legacy LX01–04 fields, framing, page staging,
identity, depth, duplicate-key and counter checks remain unchanged. Analysis
metadata, exact C-ANALYSIS source descriptors and per-fixture AUTO contributions
are validated before a complete snapshot becomes trusted. Contribution provenance
is retained independently of current analysis source identity. Unknown musical
estimates remain null/unavailable. Health carries detailed provenance; ordinary
pages use human units and distinguish current analysis from retained contributions.
No writable calibration, analysis grants or AUTO workflow was added. Existing
human leases, finite engine release and advertised checkpoint/recovery are retained.

Accepted Lux producer source: `1f0a5b40d660f31971b1c8666d4c45723f0aadde`.
Exact owner-generated command/feature fixtures stay under `tests/fixtures/lx05`;
provider algorithms and executables are not part of consumer source.
Software validation complete: **104 default / 105 native normal tests**, with
three/four explicit opt-ins ignored in those normal runs; **18 focused tests in
each configuration**, including the final exact untimed owner fixture refinement.
Final fmt and both all-target warning-denied Clippy checks passed; both default
and native releases passed. The test-only exact-fixture refinement did not alter
production source or binaries and received focused/style checks after full suites.
Three explicit actual GP09/Lux process tests and one forced llvmpipe CPU Vulkan
readback passed separately (DeviceType::Cpu, checksum1281024).

Actual GP04→LX05→production CLI/offscreen checks passed on final binaries:
calibration, active/lost analysis, reattached source epoch10 while retained
contribution keeps epoch9 provenance, programmer/record/GO/zero/Hold, engine
release and fresh-lease checkpoint, crash/restart into the actual intended Hold.
Actual untimed LX05 read-only receipt and locally refused timed preview passed
without changing the look. Root accepted the readable retained-provenance Health
frame. Owned children/pipes were joined/closed before private directory cleanup.
Analysis-only LX05 may omit timed release and checkpoint capabilities. Both are
validated strictly when advertised; an untimed snapshot cannot enable preview.
The separately accepted amendment/data is under `tests/fixtures/lx05-untimed`.
Physical output, controllers/displays, GPU hardware and shared-load acceptance
remain unverified. Historical media/exhaustive/long and hardware tests are outside
this bounded software pass.
