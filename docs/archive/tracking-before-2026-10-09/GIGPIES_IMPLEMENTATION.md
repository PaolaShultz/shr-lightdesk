> Historical snapshot, retired as an active tracker. Relative links were
> adjusted for relocation; source text and dated evidence retain their original scope.

# SHR Lightdesk: GigPies implementation

Planning baseline **2026-10-04 / GP-2026-10-04.1**. All new tasks below are
**planned**, not implemented by this document. [Central inventory](https://github.com/PaolaShultz/gigpies/blob/main/docs/MODULE_IMPLEMENTATION_MAP.md) ·
[Agreed contracts](https://github.com/PaolaShultz/gigpies/blob/main/docs/MODULE_CONTRACTS.md). Existing product roadmaps remain authoritative for
unrelated work; this plan owns only the GigPies integration increments below.

## Objective and boundary

Own lighting interaction/selection/pending-state display and independent controller role. Lux owns fixture evaluation, programmer/cue/effect arbitration, timing, output and safety. Preserve manual-first workflow and keep mock authority explicitly separate; no production engine algorithm in this repository.

## Source and evidence reviewed

Repository: `/home/shome/p/shr-lightdesk`. Inspected HEAD: `94402b42aae434a158657b7de9a73f013e27e352`.
Clean at inspection; recheck before editing. This dated observation is not a future ownership claim.

Owning documents: README.md; docs/STATUS.md, BLUEPRINT.md, CONTROL_CONTRACT.md, CAPABILITIES.md, CONTROLLER.md, SCREENS.md, DEVELOPMENT.md.

Source inspected: `src/model.rs::{Authority,Attribute,Snapshot}`, `src/surface.rs::{Surface,ReleasePreview}`, `controller.rs`, `simulator.rs`, `render.rs`, `main.rs`; tests/contracts.rs and CLI tests.

`Authority` has only `Simulator`; trusted in-process structs are not a wire
protocol. Eight fixture fixtures/two playbacks, static manual/hold/arbitration,
32 copied cues/palettes, immediate mock releases, synthetic MIDI and seven
full-HD drafts exist. Twenty-seven normal tests previously passed. No live Lux,
clock/fade/effect/output or durable show state exists here.

These are source inspection and previously recorded results, not fresh builds or
physical acceptance. The planning session runs documentation checks only.

## Milestones and tasks

First: complete typed editor/focus/modal action loop against explicit mock. Next: native window; validated read-only Lux snapshots; engine-generated preview/commit; separately reserved physical output/usability.

Task states are execution dependencies: READY has no missing software provider;
WAITING names its precise prerequisite; DEFERRED has an activation condition.
Source delivery and build reservation are additional launch prerequisites on a
peer. Every row has one owner, the repository named in its Owner column. A later
task starts only after the previous artifact is reviewed, never merely delivered.

| Task / priority / state | Owner | Work area, inputs and required artifact | Output and measurable acceptance |
|---|---|---|---|
| LD-01 / P0 / READY | SHR Lightdesk | `src/surface.rs`, `controller.rs`, `main.rs`, new `src/actions.rs`, interaction/CLI tests. Existing L1 workflow and C-LIGHT presentation rules; no Lux provider needed. | Shared keyboard/controller actions, typed attribute draft, record destination, confirm/cancel/back, selection/page focus and explicit blackout-off confirmation. Context loss clears drafts/held inputs. Mock remains visible; no fixture/fade calculations added to surface. |
| LD-02 / P1 / WAITING | SHR Lightdesk | LD-01; C-ROLE:1 E02 abstract events. `render.rs`, native frontend and bounded worker seams. | winit/wgpu over existing scenes/font with resize/device-loss/selection recovery; separate queues and role-gated LED output abstraction. No shared hot queue with Desk. Real binding waits GP-09 and GP-H2. |
| LD-03 / P1 / WAITING | SHR Lightdesk | LX-01 C-LIGHT:1 capability/E04 corpus, LX-03 read-only endpoint; GP-01 C-SHOW:1. `model.rs`, new adapter/codec. | Read-only real Lux adapter; complete bounded snapshot validation, output ladder and per-attribute contributor display. Synthetic adapter stays explicit and selectable; mismatches produce diagnostics. |
| LD-04 / P1 / WAITING | SHR Lightdesk | LX-02 accepted E04 authority, LX-04 accepted E05 timed release and LX-03 command binding; LD-03. | Route commands and engine preview tokens through Authority seam; reject stale/expired preview, lost ACK/reconnect and lease conflict visibly. No surface-calculated release destination. Timing/restart capability follows the accepted LX-04 provider. |
| LD-H1 / P2 / DEFERRED | SHR Lightdesk | LD-02/04, GP-H2/H3 and LX-06 under fresh physical authorization. | Verified second controller/displays, sole LED ownership and fixture/output loss presentation; null-output ACK never marks physical light observed. |

## Validation and failure behavior

Focused `CARGO_INCREMENTAL=0 cargo +1.97.1 test --locked --test contracts -j 1`; full `cargo +1.97.1 test --locked --all-targets -j 1` with the same environment after shared state/controller/render changes. Format `--all -- --check`; Clippy `--locked --all-targets -j 1 -- -D warnings`. Existing CLI workflow tests stay default. Gallery generation is optional bounded visual evidence after layout changes; no current historical suite.

Protect intentional intensity zero, selection vs output, Clear-to-Hold vs release, mode/hold continuity, record vs GO and blackout latch. LD-01 tests semantic actions, not new engine arbitration. Test stale revisions, unsupported coherent groups, duplicate requests and UI loss. Replacement gate is accepted Lux provider corpus plus real null authority; simulator success alone cannot pass.

Historical research, auditions, exhaustive matrices, long soaks, full-show renders
and physical/combined-load checks are intentionally outside the normal software
milestones unless their protected behavior changes. Retain their owning documented
on-demand commands; no private media download or test hardware side effect.
Independent builds retain lockfiles and existing repository editions; this plan
does not upgrade dependencies/editions or replace existing intra-repository workspace
paths. The ban is on new sibling-repository path dependencies.

## Resources, review and recovery of work

Peer lane P4-B, rpi4 microSD/4 GiB class, isolated `/home/shome/p/gigpies-module-planning-0006/shr-lightdesk`. Existing dependency-free core is suitable for bounded work. Estimate <512 MiB compiler RSS and <256 MiB target growth for LD-01; one shared jobs=1 slot with P4-A, never parallel Cargo. Native dependency build needs a new resource review, preferably NVMe host after lane transfer.

Independent fallback: LD-01 remaining focus/confirmation/recovery cases and docs while build slot is occupied. No unbounded render or research assignment.
Before builds check free space and target size; below 20 GiB free or above 5 GiB
output is a review, not permission to delete another task's cache. No reduced
coverage/debug information to make a budget appear to pass.

Handoff: exact changed files, commit plus patch hashes or bounded source manifest
if uncommitted, contract IDs/versions and provider-fixture hashes, commands/results,
intentional skipped classes, remaining limits and next task/owner. Stage only named
owned changes if a later implementation session commits; no public push is implied.
Receiving owner reviews independently and writes an immutable private-ledger
acknowledgement. Interrupted work stays visible with last completed acceptance
criterion; never reset/stash/clean another session or replay an uncertain mutation.


## Implementation launch prompt

Host/cwd assignments and source preparation are in GigPies PARALLEL_WORK_PLAN.md.
This is a prompt for a later user-started session; no implementation worker has
been started by the planning pass.

```text
Work only in the current shr-lightdesk checkout. Read AGENTS.md (if present), the
owning docs and docs/GIGPIES_IMPLEMENTATION.md, then the referenced GP-2026-10-04.1 contracts.
Implement only LD-01; keep progress and evidence in this plan. Check hostname,
HEAD/source manifest, live Git state, active ownership and current contract hashes
before edits; preserve other sessions and unrelated work. If this is a delivered
snapshot, verify its handoff manifest and separate receiving acknowledgement first.
Reserve this host's one build slot as described in GigPies PARALLEL_WORK_PLAN.md;
use Rust 1.97.1, Cargo.lock, CARGO_INCREMENTAL=0 and cargo -j 1 in normal target/.
Run focused checks during work and the required normal suite for changed behavior.
No sibling writes, sibling path dependencies or unilateral contract changes.
No audio/MIDI/DMX/playback/device/display/service changes or shared load tests.
When blocked report exact provider/task/version mismatch and continue only the
independent fallback LD-01 remaining focus/confirmation/recovery cases and docs while build slot is occupied within this repository; do not fake acceptance.
Stop after the scoped task and reviewable handoff, before later milestones,
physical operations, publication or deployment. Do not stage unrelated files or
claim mock, planned or incomplete behavior is a finished engine.
```

## Progress

- 2026-10-04: source and owner documents inspected; plan written. Implementation
  tasks remain in the states above. Physical evidence retains its original limits.

### LD-01 implementation handoff — 2026-10-04 / P4-B

Implemented and offline-validated against baseline
`94402b42aae434a158657b7de9a73f013e27e352`, contract GP-2026-10-04.1 SHA-256
`54a887f4a300d049d3ba67077a2e0446f7a92c57c41cef9d3b7da13735a092ac`.
State: **ready-for-review**, awaiting GigPies acceptance; later rows above remain
unstarted. The original planning baseline and CONTROL_CONTRACT/STATUS appendices
are preserved.

- `src/actions.rs` is exported through `surface::actions` to keep the assigned
  file ownership exact. Keyboard and abstract controller `Input::Action` share
  page/selection/layer/attribute focus, typed drafts, record/update destinations,
  Clear Hold, blackout and confirm/cancel/back semantics. No device binding added.
- Typed scalar drafts use percent/degrees at exact tenths, refuse malformed or
  out-of-range input and unavailable complete selections, and retain intentional
  zero. Drafts do not write until confirmation. Record destination is 1..32;
  replacement is explicit, and recording never means GO or release.
- Blackout-off opens a revision-bound confirmation. CLI `status` reviews the
  authority's pre-master intensity/source and master, without surface-computed
  output. Cancel, selection/page changes, ACK, refresh/disconnect and input-context
  loss invalidate drafts. Lost input clears fine/pickup and requires fresh releases.
- Synthetic pads open record/editor panels and confirm/cancel them; modal keys
  enter digits and rotary 1 adjusts only the draft. Outside a modal, keys retain
  fixture/group selection. Complete navigation is available as abstract semantic
  events; real controller profiles and native keyboard/display bindings await LD-02
  and their owning prerequisites. Attribute rotary edits use the same typed action
  validation and command path as keyboard entry.

Validation under the shared nonblocking build lock, after P4-A's explicit release:
Rust +1.97.1, jobs=1, incremental=0, normal target and unchanged profiles/lockfile.
Focused `test --locked --test interaction --test contracts -j 1`: **31 passed**.
Full `test --locked --all-targets -j 1`: **35 passed** (24 contracts/rendering,
7 interaction, 4 CLI), none ignored. `fmt --all -- --check`,
`clippy --locked --all-targets -j 1 -- -D warnings`, and
`build --locked --release -j 1`: passed. First build shell time real 12.286 s,
user 11.200 s, sys 1.338 s; peak RSS unavailable. Target absent → 71,694,879 bytes;
free space 38,348,353,536 → 38,275,837,952 bytes (host-wide delta includes other
activity, not attributed wholly to this build). No deletion performed.

Synthetic CLI fixture `tests/fixtures/ld01-operator.txt` SHA-256:
`4df9a5f698ec29440acb0df0048fb2ef6563c7a74100b123b8cccd1c01793221`.
Existing simulator/rig SHA-256:
`c5d43d83403c3226d67da3f114eb0edfac0fe5b573d20e2e6340472c58e5d43e`.
No engine algorithm, model, renderer, dependency or lockfile changes. Scalar mock
RGB editing does not implement the future real coherent-group contract. Stored
state remains volatile; Simulator stays explicit and physical output unknown.
Typed setters now reject more than one fractional digit instead of float rounding;
legacy explicit `cue/palette record/update ID` remains a complete keyboard action,
while `record cue|palette` opens the detached destination panel.

Intentionally skipped: historical/exhaustive matrices, gallery/media generation,
long benchmarks/soaks, sibling suites, native display/GPU, physical MIDI/LED/DMX/audio,
shared hardware/load and real Lux integration. No hardware verification or live I/O.
Source remains uncommitted for review. Private P4-B handoff includes exact file
hashes and validation logs. Next owner: GigPies reviews LD-01; Lightdesk may begin
LD-02 only after acceptance and its separately scoped prerequisites.

### Task0008 LD-01 corrections and LD-02 headless — 2026-10-04 / P4-B

LD-01 corrective source accepted by coordinator R2; the original handoff above
remains historical evidence. Invalid nonempty drafts refuse rotary adjustment
before conversion, including record IDs at i32 boundaries and attribute tenths
at i32 boundaries. Controller math widens to i64 and rejects invalid/degenerate
ranges. Empty attribute drafts start at a range-valid zero clamp (including Zoom).
Text/backspace/navigation rearm pickup; rotary draft edits retain pickup. Context
cancellation consistently closes the menu as well as the detached draft.

**LD-02 HEADLESS implemented; native LD-02 remains partial/pending.** New
`src/operator.rs` owns a 64-event lighting input queue, fixed 16-event pump budget,
separate latest-only LED mailbox and injected-time maximum 20 LED batches/s.
Unchanged desired LED state is suppressed. These are pure worker seams, not native
threads, OS device locks, endpoint claims or a MIDI LED encoder. Role-owner
observations must have a new generation after revoke/reconnect; foreign audio
roles, stale inputs and stale feedback cannot act. Assignment changes, overflow,
focus/context loss and reconnect discard queued intent, rearm pickup and require
fresh key/pad releases. Raw MIDI is decoded only at bounded dispatch. Keyboard
key-edge events coexist with already interpreted text/menu actions; MIDI remains
role-routed when desktop keyboard focus is absent.

The pump drives existing Surface semantic actions, returns requests to an
independent authority worker and preserves pending until an explicit reply.
Input loss does not manufacture an authority ACK. Renderer loss preserves pending
and confirmed authority state and selection; stale restoration/layout tokens refuse.
Resize uses letterboxed logical coordinates and rendering's shared fixture geometry
for hit selection, with stale queued hits refused. Zero-size rendering suspends;
software scenes remain available after abstract restoration. No redraw advances
lighting time. Simulator is still the only authority and is explicitly labelled.

GP-01 provider E01/E02 example corpus retained byte-for-byte with PROVENANCE under
`tests/fixtures/gp01/`: contract SHA-256
`54a887f4a300d049d3ba67077a2e0446f7a92c57c41cef9d3b7da13735a092ac`;
E02 SHA-256 `055c8d501cde17fcc168fd83071f53c6f731d32f0554bdc4846de056f75a4f5b`.
Role tests exercise trusted observations with the E02 prior-state/conflict scenario;
this is not a JSON decoder or provider replacement acceptance. Native identity,
profile checks and OS locks remain GP-09/physical prerequisites. Lux wire decode,
arbitration, timed releases and real provider binding remain LD-03/04 next-pass work
following exact provider acceptance. No Lux source or algorithms copied.

Validation evidence and final source hashes are in the private task0008 P4-B
handoff. Focused interaction/operator/contracts suite: **40 passed**. Required
complete normal suite, fmt, Clippy and release results are recorded at final handoff.
Physical audio/MIDI/LED/DMX, native winit/wgpu/display, production sockets, services,
combined load, long/exhaustive research and sibling suites are intentionally skipped.
No hardware verification, source commit, publication or deployment occurred.

Final task0008 pass1 checks: full normal all-targets **44 passed**, none ignored;
fmt check, warning-denied all-target Clippy, release build/version/help passed.
Both supplied invalid-value CLI reproductions returned range refusals without
panic. Rust +1.97.1, jobs1/incremental0, normal target; parent nonblocking build
lock for every producing check. P4-A pass1 release preceded this batch. Target
71,694,879 → 86,020,942 bytes; free 38,261,891,072 → 38,247,350,272 bytes,
well within review thresholds (host-wide delta, not exclusive attribution).
Source is **ready-for-review** for LD-02 headless; coordinator owns acceptance
and the next provider-dependent pass. No further build before reassigned turn.

### Task0008 pass2 LD-02 review corrections — 2026-10-04 / P4-B

Coordinator accepted correction R2 source and its 44 focused tests. Keyboard
focus transitions now discard keyboard/pointer intents and detached drafts while
preserving assigned controller events and press edges. Queued modal controller
intents whose panel was invalidated refuse visibly after edge decoding; releases
remain effective. An unchanged action-menu blackout press survives either focus
transition without an artificial controller release. Role generation exhaustion
fails closed without wrapping or retaining LED/input authority.

Pad input and desired abstract LED feedback share one semantic layout. Base pads
are Stage, Programmer, Playbacks, Library, Multi, Deselect, Actions and Back;
Automation/Patch/Health do not falsely highlight fixed controls. Menu and modal
contexts use their actual actions, with unavailable modal pads off. There is no
hardware LED codec or output.

Focused offline validation: **44 passed** (24 contracts/rendering, 11 interaction,
9 operator tests), none ignored, plus rustfmt. Complete normal suite, Clippy and
release are pending the subsequent LD-03 implementation batch. The correction
build turn has been released while awaiting the exact root-reviewed LX-03 wire
provider. LD-03 decoder implementation has not begun; its endpoint remains a
provider dependency. Native/display/physical/hardware and long research remain
outside this pass. Prior handoffs and plans above remain preserved.

### Task0008 pass2 LD-03 real read-only adapter — 2026-10-04 / P4-B

Root accepted LX03-R2 at 14:13 UTC, delivering exact corpus and a private synthetic
service executable. All transferred data/documentation and executable hashes were
verified before use. Exact producer data, original/root provenance and acceptance
are copied under `tests/fixtures/lx03/`; executable stays outside product Git.

LD03 source implements strict bounded UTF8/integer JSON with duplicate keys,
unknown/missing fields and depth refusal; four-byte framing; coherent page staging
with identity/counter/deadline checks; complete capability/range/patch/inventory/
source-reference checks before one trusted-state commit. Rejections retain the
last valid snapshot. Stable string IDs and advertised angular ranges are retained,
independent of Simulator's u16 IDs/eight fixtures. No consumer lighting evaluation,
Authority command implementation, release token or grant exists here.

Explicit `--provider lux --snapshot-file PATH --show-id UUID --epoch DECIMAL`
routes real framed producer observations through validation and visible perattribute
provenance. Linux `--socket-path` instead explicitly requests a read-only snapshot
from a same-UID private 0700/0600 Unix endpoint with bounded reads; startup and
Simulator never auto-connect. Programmer, Hold, stored, playing, proposal, resolved,
final intent, contributors/winners, inhibit/clamp and durability remain distinct;
null submitted/observed and physical unknown stay unknown. Native rendering remains
pending. Offline validation passed: **58 normal tests** (24 contracts/rendering,
11 interaction, 9 operator, 9 provider, 5 CLI), none ignored. Correction-focused
44 passed; initial adapter-focused17 passed, final full suite includes9 provider
cases. Format check, warning-denied all-target Clippy, release/version/help passed.
The final release CLI decoded the accepted real private Lux service and rejected
wrong epoch without partial output; own child joined and endpoint cleaned.

All Cargo commands held the parent nonblocking host build lock, jobs=1,
incremental=0, Rust1.97.1, normal target/profiles. Pinned serde1.0.229,
serde_json1.0.151 and libc0.2.189 lock additions only; no sibling dependency or
native GPU build. Final target318249346 bytes (pass input86394351), free37785956352
bytes; no cleanup. Final full suite27.67s, Clippy7.60s, release38.31s wall time;
RSS unavailable. Build turn released after all children exited.

Status: **implemented/offline-validated, ready for root review**. Native LD02
remains partial; physical/hardware/combined-load, long research/exhaustive and
sibling suites intentionally unrun. LD04 command/release authority waits accepted
LX04 in the next coordinated pass. Source remains uncommitted, no publication.

### Task0008 pass3 LD03 corrective checkpoint — rpi4

Implemented explicit Fresh/Stale/Unavailable cached logical state. Invalid or
incomplete pages, receipt expiry (2000ms), read timeout and disconnect preserve
trusted values as stale; only a fully validated compatible complete snapshot
restores Fresh. Physical output remains unknown. `advance` is the event-clock seam
for idle expiry; one-shot CLI reports freshness at receipt without subscription.
Contributor identity inventory follows Lux: present programmer/Hold and all
playing sources, default only when none, with chosen source winner-marked.
Playing intensity is Lux-supplied level-adjusted state, distinct from stored raw
playback values; contributor values agree with advertised playing values. No
surface arbitration or intensity scaling is added. Address bounds precede
footprint addition, covering i32 extrema in debug and release.
Current validation is recorded in the private pass3 checkpoint; prior 58-test
pass2 evidence precedes the last address edits and remains distinct. LD04 still
requires coordinator acceptance and exact reviewed LX04 schema/executable bytes.

### Task0008 pass3 LD04 real null-provider implementation checkpoint

Implemented a separate stable-ID `LightingAuthority` seam and `LuxAuthority`
lifecycle in `src/lux_control.rs`, with secure bounded real private UDS transport.
The original u16 trusted in-process Simulator Authority is unchanged. The real
headless operator in `src/lux_operator.rs` and `lux-control` CLI route real grant,
programmer touch including zero, copied record/update, GO, Clear-to-Hold, master,
blackout, mode, engine preview/commit/cancel and advertised durable checkpoint.
No Lux arbitration, fade interpolation, destination or recovery algorithms are
copied. Exact accepted LX04 data is retained under tests/fixtures/lx04; executable
stays private outside Git (SHA256 cfb47410053418b8eec2d396a8dc4e78639b2c8f76ef52e2d21b6c083e0922ee).

Strict timed/durable schema validates transition masks/ranges/ticks, distinct release
provenance, preview selection/revision/lease/epoch/patch/tick/token and durability.
One pending mutation, increasing writer IDs, conservative first-send lease deadlines,
exact-envelope100/250/500ms retries and bounded uncertain/fresh-read recovery prevent
operation replay. Cached old ACKs cannot regress current state. Renew always reads
fresh state and permits one bounded new-ID retry only after a correlated revision
conflict. Confirmation maintenance refuses any reviewed revision/lease change.
Input loss/reconnect discards confirmations and requires release/pickup.

Current checkpoint evidence: 75 normal tests passed before three final regressions
and status-export addition; warning-denied Clippy and release passed after style
repair. Actual accepted executable IPC passed real CLI touch/record/GO/zero/Hold,
preview/500ms release plus renewal, master/blackout/checkpoint, two forced restarts
with epochs1/2/3, retained disarmed intent/no playback, old-epoch refusal and exact
lost-ACK retry proving one mutation. Final current-source normal/fmt/Clippy/release,
additional sampled-progress/client-reconnect/status-export IPC and independent root
review are pending the next build turn. No final LD04 acceptance is claimed here.
Native display/controller bindings, physical light/output and shared load remain
unimplemented/unverified and outside this pass. See README for exact usage/bounds.

LD04 corrective R2 validation completed in this pass: **80 normal tests passed**,
none failed/ignored; formatting, warning-denied all-target Clippy and normal release
passed on Rust1.97.1/jobs1/incremental0 under the assigned nonblocking host lock.
Reviewed corrections additionally bound every script/transport session to25s,
latch Enter-down with no confirmation, and enforce one500ms first-response-byte
frame deadline across retry windows. Regressions cover malformed/adversarial actual
LX04 inventories/tokens, delayed grant/renew, identity/counter exhaustion, uncertain
recovery/no replay, cached ACK regression, correlated renew conflict, confirmed ACK
with failed refresh, confirmation revision changes and held-input/frame expiry.

Expanded actual accepted LX04 IPC passed with the current release binary: sampled
intermediate release/current distinct from Hold and exact final700 intent, renew
through the fade, status export, copied cue/GO/intentional zero, master/blackout,
checkpoint and two forced restarts (epochs1,2,3) with actual-client refresh of retained
disarmed intent/no GO. A same-UID private proxy dropped one touch ACK; exact retry
produced only revision1. Disconnect/reconnect dropped confirmations without changing
Hold/blackout; Enter held before any preview could not commit a later target; engine
cancel cleared the unused preview. Invalid version/show and old epoch refused.
All own children joined and temporary endpoints removed. Exact logs/manifests and
reproducible harness remain in the private pass3/P4-B run. State: ready-for-review,
awaiting independent coordinator R2 acceptance, not publication or physical evidence.

LD04 R3 closes the final independent-review frame-ordering defect: deadline expiry
is checked before delivering even a completed buffered frame. A valid complete JSON
buffer past500ms now refuses before delivery; its explicit regression joins the
partial-frame/retry-window test. Current **81 normal tests**, fmt, warning-denied
Clippy and normal release passed, followed by the complete expanded actual LX04 IPC
harness again on this final binary. R2 evidence remains historical; R3 source hashes
and checksummed validation/IPC/handoff are the final review input. No source commit,
publication, hardware/native/GPU/load operation or extra feature was introduced.


### Task0009 LD02 native source checkpoint

Task0008 LD01/headless-LD02/LD03/LD04 are accepted (root final acceptance2026-10-04
15:30:35Z, source manifest36d1497d4f43bed2a523459a5d354740ef4ff6e4e2167568ce845f6932d2d8c0).
The historical planning/task rows above do not undo that acceptance. This pass owns
only LD02 native software and truthful documentation over that unchanged authority.
`frontend.rs` presents real validated Lux data and bounded worker/input seams;
`native.rs` provides opt-in winit/wgpu with the existing bitmap font and scenes.
Root accepted the pinned optional dependency/resource review and initial lockfile
resolution exception; all compilation uses locked jobs1 normal profiles/target.
Native/offscreen/input tests and real accepted Lux IPC are in progress. GP09 binding consumes accepted broker/protocol/corpus
(source ac8f19cc58b5386bf1f617cbd7e080996582615e188a75e334a588e25b20d01e,
private executable9206f9945ab4696175d27da87cb06588c2011802c7d3d0d0a06afc3596b94dc3).
Native semantic selection/attribute/edit/record/GO/Hold/release/mode/blackout
routes to Lux with complete presented-review gating. Role loss fences sends and
queued intent; private pipes and latest LED mailbox stay independent. No central contract changes, provider algorithm
copying, source commits/pushes or physical/shared-load work occur in this worker.


Task0009 pass1 bounded handoff: current91 default tests, three actual accepted
broker/Lux tests, native-feature warning-denied Clippy and explicit MesaCPU shader/
readback passed. Real worker offscreen programmer/Hold/playback/current-intent and
complete release-review pages were rendered/visually inspected. GP09 globalCAS
change preserves a live lighting lease; child loss/timeout fences input/provider/
LED generation; fresh reacquisition retains the look without input replay. Native
full feature suite and release remain pending root's immediate continuation. No
source commits/pushes, physical endpoints/windows, service installation, copied
provider algorithms or central contract changes. Final private checkpoint carries
exact owned hashes/commands/artifacts and current limits; original task0008 evidence
remains preserved.

### Task0009 pass2 LD02 software handoff

Root accepted the final material-review source and all seven actual-provider
frames. Record/update review names programmer-only requested values, matching
stored values being overwritten and untouched retained values. GO review names
both requested cue and replaced playback target union, including removals and
current contributions. Effective results await Lux resolution; no consumer
arbitration or output math is introduced. Readable review lines freeze with the
existing command context, while the original opaque commit token stays internal.
Native console mutations use the same coverage gate. Expired contexts and
notice-only pages cannot certify review; zero-size queued redraw is suspended and
fatal renderer failures propagate through the CLI.

Final software checks:96 default,97 native normal tests;3 actual accepted broker/
Lux process tests;1 explicit llvmpipe DeviceType::Cpu readback with checksum1281024;
both all-target warning-denied Clippy configurations, fmt and both release builds.
No physical window/device/output or combined-load acceptance is inferred.

The accepted LX05 wire/command/feature corpus is retained byte-for-byte under
`tests/fixtures/lx05` with root ACK. Its decoder/provenance/analysis-health consumer
and new-runtime interoperability are the next bounded continuation, not completed
here. Legacy LX01..04 schemas/corpora and all prior evidence remain preserved.
Private pass2/P4-B-native final checkpoint fixes source/artifact hashes, commands,
actual results and this explicit compatibility limit. Root owns source integration
and publication; this worker performs no source commits/pushes.

### Task0009 pass3 LX05 compatibility continuation

LD-02 software is accepted on frozen pass2 source; its protected review/layout
workflow is retained. This increment consumes accepted Lux LX05 source
`1f0a5b40d660f31971b1c8666d4c45723f0aadde` and exact owner-generated wire data.
The opt-in decoder accepts bounded analysis health/features/proposals and AUTO
intensity/source metadata, including original per-fixture provenance across source
reconnect. Human actions still use the existing strict leases and Lux destinations.
Optional timed release and durable checkpoint capabilities use the existing
contracts; analysis-only snapshots cannot enable timed preview. Exact accepted
untimed owner data/amendment is separate under `tests/fixtures/lx05-untimed`.
No new consumer arbitration, provider DSP, calibration/grant UI or role
workflow is introduced. Central contract ownership stays with GigPies.

Current state: **software complete and validated**, independently reviewed.
Complete normal suites passed 104 default/105 native; final focused suites passed 18
in each configuration, including the exact accepted untimed fixture refinement.
Fmt, both all-target warning-denied Clippy configurations, both release variants,
three actual GP09/Lux process tests and one forced CPU Vulkan readback passed.
The final test-only fixture refinement leaves production code/binaries unchanged.

Actual accepted GP04→LX05→production CLI/offscreen acceptance passed calibration,
active/lost analysis, new source epoch10 with retained epoch9 provenance, human
programming/storage/GO/zero/Hold, engine release, fresh-lease checkpoint and
crash/restart into the exact actual intended Hold. Untimed read-only receipt and
locally refused preview passed; the look remains unchanged. Root accepted the
Health render and current source; all owned children joined before cleanup.

Physical output, operator displays/controllers, hardware GPU and combined-load
verification remain separate. Historical/media/exhaustive/long/hardware tests
were intentionally skipped. Private handoff fixes source/executable/artifact
hashes and reproducible commands. Root owns final integration/publication; this
worker makes no source commits/pushes or central-contract edits.
