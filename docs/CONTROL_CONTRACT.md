# Lighting control and observation contract

2026-10-04. **Proposed integrated contract**, with a deliberately smaller in-process
simulator implementing the marked subset. SHR Lux owns lighting authority; GigPies
owns cross-module compatibility and integration. No Lux API has been agreed yet.
GPC1's audio test scalar and GPA1 source frames are not a lighting control API.

## State words that must stay distinct

| State | Meaning / owner |
|---|---|
| Selected | Surface-local fixture/group edit scope; selecting sends no output command |
| Programmed | Touched attributes in the live programmer; eligible for Record; explicit override, including zero |
| Stored | Named cue/palette data; existence alone neither plays it nor clears programmer |
| Playing | Engine-owned active playback instance, cue ID/revision, timing and source scope |
| Overridden / Hold | Persistent per-attribute human ownership; no implicit expiry; may mask playback or automation |
| Resolved | Engine's pre-master intent and winning source, plus relevant competing contributors |
| Final intent | Post-master/blackout/fixture-policy state, still not proof of delivery |
| Submitted | Encoded frame handed to the declared output transport, with time/sequence/error evidence |
| Observed | Actual available fixture/receiver feedback or separate measurement; unknown unless established |

The drafts display synthetic resolved/final values; DMX, submitted and observed
are unavailable. Connection, command ACK and physical output are independent.
Stale state remains labelled and is never replaced by healthy-looking zeroes.

## Manual, Assist and Auto

MANUAL needs no analyzer: select, set intensity/color/position where supported,
store/recall looks, play cues and operate masters. Automation writes are disabled;
observations may continue in the final engine. The mock refuses proposals in MANUAL.

ASSIST receives scoped proposals with reasons, validity and bounds. Nothing
changes until the operator accepts. Acceptance becomes a human programmer edit,
with a visible override; it does not silently grant future automation.

AUTO requires separate grants naming fixtures, attributes, allowed ranges, allowed
effects and rate/slew constraints. Mode alone grants nothing. Manual edits override
the same attribute, starting from the resolved value; holds survive subsequent
proposals and UI loss. Re-entering AUTO does not release them. The mock has bounded
scalar grants only; it clamps the applied automation candidate and keeps the raw
proposal separately. It has no analysis, proposal expiry or rate/slew capability.

Changing mode freezes the current pre-master look for attributes not already in
the programmer. Global master and blackout remain unchanged. Grant/bounds changes
also capture affected current values into Hold before changing future permission.
Returning any scope to playback/automation is a separate reviewed transition.
In the real engine this includes capturing current effect/fade output and freezing
its phase/continuation policy, not merely storing its endpoint. This is missing
Lux work; the static mock cannot validate continuity of moving effects.

## Arbitration proposal for Lux

Do not use audio fader takeover or general last-message-wins semantics here.

1. Validate fixture capability, ranges, access scope and safety policy.
2. Evaluate each cue/playback and its **own** effect/timing layer in Lux.
3. Among normal equal-priority playback intensity contributors, use HTP (highest
   scaled intensity). A playback level affects its intensity contribution; at
   level zero its non-intensity values remain until explicit Off/release.
4. For color, position, zoom and discrete beam attributes, use the most recently
   **activated** eligible playback at the declared priority, not network arrival
   time. Components of a coherent color/position group must activate atomically.
   Zero intensity does not automatically home a mover. Off exposes the next
   eligible source or a validated default.
5. Explicit AUTO targets form a named authority layer over eligible playback
   attributes, only within grants. This is a visible takeover, not HTP against
   the operator; default grants exclude performer key light, blackout, unsafe
   fixture controls and strobe. ASSIST is never an authority layer.
6. A human Hold masks underlying contributors. The live programmer masks Hold
   on touched attributes; an intentional 0% must actually lower a light even
   when a playback is at 100%. No HTP between programmer and background.
7. Apply group/inhibitive masters and the lighting grand master to intensity.
   Fixture-specific virtual intensity and calibrated color conversion belong to
   Lux; do not multiply arbitrary DMX slots in Lightdesk.
8. Apply the persistent lighting blackout latch and fixture/output protection.
   Blackout suppresses emitted intensity through the fixture's verified dimmer/
   shutter contract. Preserve color/position unless the declared policy says
   otherwise; never assume an all-zero universe is safe.

Show the winning source **per attribute**, its value before and after masters,
and why the other sources lost. Final engine telemetry should identify all HTP
contributors/ties, priority, cue/effect references, clamps and inhibits. The mock
reports one deterministic winner (newer activation breaks HTP ties), two static
slots, scalar RGB channels and no effect/priority/group-master evaluation. Real
color grouping, non-LTP attribute classes and fixture defaults require Lux review.

Effect generation never runs in a view. Planned effects are owned children of a
playback/programmer/automation scope, with declared replace/additive behavior,
amplitude bounds, phase, clock and expiry. Manual takeover suppresses the effect
on that attribute without stopping unrelated attributes. Releasing it previews
the then-current engine destination. Avoid layering anonymous LTP effects that
cannot explain their contribution.

## Clear, hold and release

| Action | Explicit result |
|---|---|
| Deselect | Clear selection only; preserve programmer, recordable content and output |
| Clear to Hold | Move all touched programmer values into persistent human Hold; empty recordable programmer; preserve look |
| Return to Playback | Preview target IDs/attributes, destination source/value, delta, timing; commit clears those programmer/hold entries and excludes AUTO on that scope |
| Return to Automation | Requires AUTO, grant and a valid current candidate; same preview/commit; clears human entries and enables AUTO only for that scope |
| Cancel | Keep all applied state; discard only the release draft |
| Record | Store the recordable mask with explicit destination; do not change playing values, release programmer or capture unrelated Hold values |
| Update | Separate destination and mode (replace/merge/cue-only planned); show diff and affected references before real engine application |

The simulation supports immediate **0 s** release only, explicitly labelled before
confirmation. Selection/page/revision changes cancel its preview. The native/live
version must default to an engine-approved finite release time, show motion/color
path implications, and revalidate target revisions before applying. A zero-time
live release must be an intentional operator choice; simulation timing is not a
safe hardware default. No automatic return timer is planned.

Stored cues in the mock are static touched-attribute snapshots, with IDs 1..32.
`cue update` explicitly replaces a stored snapshot; active playback retains the
copy captured at GO until another GO. Palettes are fixture-specific value copies,
not references: changing one does not update cues. Cue lists, tracking, selective
update, palette propagation, undo and persistence remain engine work.

## Grand master, blackout and bump

Grand master always names **lighting intensity only**. It cannot alter audio,
recording, movement or control channels. Its absolute controller needs pickup.
Display its influence next to the base intensity. Blackout-on is a latched discrete
action; release of the pressed key/pad does nothing. Blackout-off is a deliberate
separate action with a reveal preview in the native UI; a mode/page/reconnect
cannot release it. The loop uses explicit `blackout off` as that intent.

Flash/bump is **planned and unavailable** in the mock. Proposed first version:
press creates a named temporary intensity contribution for the displayed playback
or selected group; release removes that token and restores the current background.
It respects grand master, blackout and safety bounds, never solos/kills other
playbacks, and never means strobe. No color/position flash by default. Capture the
scope on press; page/selection changes cannot retarget a held flash. Input overflow,
focus loss, device loss and client death end the token; Lux owns a bounded expiry
lease so a missing release cannot leave it latched. Duplicate presses reuse the
token; a new press requires a release. No flash enabled before this is implemented.

## Proposed live transport messages

Use an authenticated/versioned control service outside audio/render threads.
Local IPC versus dedicated network transport is a GigPies/Lux integration decision.
Neither this package nor its simulator opens a socket.

| Message | Minimum content |
|---|---|
| Capabilities | Show UUID; module/build/schema versions; epoch; stable fixture/group/attribute IDs; patch revision; physical units/range/step; supported actions; output profile and authority capabilities |
| Snapshot | Revision; programmer mask; holds; mode/grants; cue/palette/playback identities; base/final values and provenance; master/blackout; output arm/fault state |
| Grant | Authenticated writer, lighting scope, lease generation/expiry; separate automation target bounds and effect permissions |
| Command | Unique writer/session/request ID, epoch, expected target revisions, explicit scope, absolute intent, validated action/transaction size |
| Preview / commit | Engine-produced before/after, target mask, transition and token tied to revision/expiry; commit must reject stale preview |
| ACK | ID/epoch, accepted/conflict/rejected/busy, reason, authoritative revision and values, effective engine tick when known |
| Proposal | Producer/version, targets, before/candidate/bounds, reason/evidence, confidence/freshness, validity revision and transition |
| Observation | Sequence/timestamp, resolved/encoded/submitted/observed stage, driver/node identity, freshness, faults and unknowns |

Snapshots are not commands and sending is not confirmation. Bound payload/target
counts, retry window and queues. The mock has one pending command, ≤64-target
transactions, 32 cues and 32 palettes, a 64-reply cache and high-water ID refusal
for a **single writer for its lifetime**. Duplicates return the original response;
reused IDs with different payloads and expired IDs cannot become new writes.
A real multi-writer protocol needs negotiated writer identity/lease and dedup
windows. The public Rust structs are trusted in-process types, not a hardened
untrusted decoder. Wire parsing, authorization and full snapshot validation are
missing and must precede connection to Lux.

## Loss, restart and output recovery

Disconnect cancels queued intent, disables writes, releases transient input
modifiers and preserves stale last-known values. A lost ACK may mean applied;
reconnect takes a fresh authority snapshot and never replays the old queue. A
surface restart also needs a fresh writer session/grant. Reconnect requires fresh
button release and rearms pickup. Permanent manual holds/blackout are engine-owned.

Output-loss policy is a **show/patch contract**, not a universal blackout rule:
for a validated static dimmer rig a reviewed bounded hold may be appropriate;
for a mover or special channel an explicit safe-state transition may be required.
Specify behavior per failure: UI gone, control link gone, engine stopped, transport
failed, dongle power lost, fixture receiver lost signal. Firmware may independently
hold, blackout or run internal programs; verify those settings on the real fixture.
Do not promise a USB worker can undo a cable/power loss. Emergency independent
house/work lighting is outside this control contract.

No automatic rearm after output failure, epoch/patch mismatch or engine restart.
Refresh capabilities/patch, show current physical uncertainty, preview the recovery
look, cancel expired transient effects and require operator rearm. Recover from
known current state with an engine-controlled transition. Never replay a buffered
strobe/flash or yesterday's programmer. Physical confirmation is separate from a
successful reconnect. Lux owns and tests this path; Lightdesk displays it.

## Coordinated implementation baseline — 2026-10-04

The [GP-2026-10-04.1 contract decisions](../../gigpies/docs/MODULE_CONTRACTS.md)
now fix the first implementation subset, examples, bounds and unresolved gates.
They supersede undecided integration choices in this earlier proposal for that
subset; the simulator remains in-process scaffolding with the limits above.
See [the task plan](GIGPIES_IMPLEMENTATION.md) for provider replacement gates.

Task0008 LD03 adds a real **read-only** C-LIGHT:1 consumer, separate from the
Simulator Authority seam. Strict bounded frames and complete page/schema/patch/
capability/source validation precede trusted state; failures retain the previous
complete snapshot. Stable IDs and declared ranges are presented without computing
arbitration, release destinations, lighting output or authority grants. Explicit
file/private-UDS CLI selection is required. LX03 is volatile/null-disarmed and
physical unknown; native display, command writes, timed release and durable restart
await later reviewed providers/gates. Pinned codec dependencies do not import Lux
source or lighting algorithms. See tests/fixtures/lx03/PROVENANCE.json for exact
accepted producer source/data identities.
