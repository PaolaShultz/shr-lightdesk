# Lighting controller and keyboard plan

2026-10-04. Second physical controller identity/preset/messages are **unverified**.
No MIDI device was opened. All byte injection below uses an explicitly synthetic
profile. Sixteen rotaries, eight physical pads and octave-transposed keys describe
the requested vocabulary; they do not identify a particular hardware memory.

## Existing evidence and separate ownership

Lux notes 0009/0018 document a MiniLab mkII USB `1c75:0289`, historical enumeration
`hw:2,0,0`, an identity reply and opt-in pad-color work. Enumeration is not a
persistent identity. DAW `CONTROLLER_PROFILES.md` includes MiniLab 3 factory
channel-10 pads and a captured User 1 conflict on channel 1; its current handoff
also names mkII hardware. These records are device/session-specific, not a single
universal map. FX's corrected layout describes two rows of eight rotaries and
clicks only on 1 and 9. Neither clicks nor the same LED protocol are assumed for
the second controller. Do not import a sibling's private controller configuration.

Official [MiniLab mkII manual](https://downloads.arturia.net/products/minilab-mkII/manual/minilab-mkii_Manual_1_1_EN.pdf)
is a linked hardware reference already reviewed in Desk/Lux; no new physical
acceptance of its modes is claimed. The [Desk plan](../../shr-desk/docs/CONTROLLER.md),
[DAW profiles](../../shr-daw/docs/CONTROLLER_PROFILES.md),
[DAW LEDs](../../shr-daw/docs/CONTROLLER_LED_FEEDBACK.md) and
[FX interface](../../shr-fx/docs/INTERFACE.md) own the inspected local evidence.

GigPies assigns `audio-desk` and `lighting-desk` separately, using verified
manufacturer/product/interface, serial where unique, and explicit physical
assignment. Do not persist ALSA port/card numbers or match the first name.
VID/PID alone cannot distinguish identical controllers. If serials are absent
or duplicated, present both unresolved devices, ask the operator to press one
labelled key on the intended lighting controller, consume that gesture without
performing an action, then confirm the observed endpoint and role. Bind a
verified USB topology path only as a documented fallback; moving ports makes
that assignment unresolved. Reassignment is atomic: release old role first,
claim only the chosen endpoint, and keep the other desk unchanged on failure.

A per-device owner token covers input and associated feedback endpoint. Never
connect either controller's notes to a synth/DAW/instrument. Feedback output must
match the same verified device, not an arbitrary MIDI output. Lux's standalone
pad preview must be disabled/unclaimed in integrated mode; it can supply semantic
state to Lightdesk but cannot write the same LEDs. Independent nonblocking workers
and bounded queues prevent lighting input/output stalls from blocking audio input.
The mock enforces the exact injected identity `light-sim`; `audio-sim` is ignored.
Its pure duplicate-identity check rejects ambiguity; it does not discover devices.

## Keys select an explicit layer

Always display `KEYS: FIXTURES` or `KEYS: GROUPS`, anchor, ordered slot map and
REPLACE/MULTI. A key selects only; it never recalls a cue, changes intensity,
locates a mover or sounds a note. All twelve chromatic pitches are selectable.

```text
slot = note - learned_anchor_C
ordered_entities[slot] -> stable fixture/group ID
C  C# D  D# E  F  F# G  G# A  A# B    slots 1..12
next octave                           slots 13..24
next octave                           slots 25..36
```

Octave buttons already transpose transmitted notes. Do not add a second software
octave offset. Upper end key is an ordinary slot. Out-of-range notes are ignored.
The sample rig has eight occupied fixture slots (IDs 1,2,11,12,13,14,21,22), and
three group slots; higher octave slots are currently empty. Pure translation tests
exercise the next octave independently of that small rig.

REPLACE selects the fixture or entire group. MULTI toggles membership; group
members toggle individually, with resulting membership visible. Selection reset
is always P6 / `deselect`, distinct from programmer Clear. Held notes are edge
tracked; note-off/velocity-zero releases do not select. Chords add selection only
when MULTI is intentionally enabled. Selection/layer/page changes cancel release
previews and rearm pickup. A held command pad does not acquire a new meaning just
because its screen changed.

## Physical rotary map

Positions are exactly two rows of eight. The first slice keeps assignments stable
on all seven pages; action menus/release panels suppress rotary edits. Unsupported
controls show `--` and emit no command. Mixed fixture selections permit only
capabilities supported by **every** selected fixture; a rejected edit changes none.
Mixed values currently require explicit keyboard `set` before absolute/relative
rotary adjustment. Future relative group-offset editing needs bounds per fixture.

| Top row | K01 | K02 | K03 | K04 | K05 | K06 | K07 | K08 |
|---|---|---|---|---|---|---|---|---|
| Selected fixtures | Intensity % | Red % | Green % | Blue % | Pan deg | Tilt deg | Zoom deg | Strobe -- |

| Bottom row | K09 | K10 | K11 | K12 | K13 | K14 | K15 | K16 |
|---|---|---|---|---|---|---|---|---|
| Show controls | PB1 level % | PB2 level % | FX rate -- | FX amount -- | Fade -- | Delay -- | Page -- | Lighting grand master % |

The synthetic RGB model does not imply real fixture color temperature or calibrated
color. Future attribute pages derive descriptors from Lux and show the exact page,
units and target. Candidate K11/K12 effects and K13/K14 timing remain unavailable
until engine support. K15 may browse pages only after explicit profile assignment;
currently use pads/keyboard. No unguarded rotary for patch, lamp/reset, blackout,
mode switching or recalling a show. Optional learned clicks 1/9 may mean Open/Back;
they remain unimplemented and must not be required.

## Eight pads and feedback

Both logical pad banks, if verified, should mirror these eight actions. The mock
models one synthetic input bank only; no bank or preset write is performed.

| Pad | Base action | Actions menu (P7 opens) |
|---|---|---|
| P1 | Stage | Record next free cue from touched programmer values |
| P2 | Programmer | Clear all programmer values to Hold |
| P3 | Playbacks | Preview selected intensity Return to Playback |
| P4 | Library | Automation view |
| P5 | Toggle MULTI selection | Blackout-on latch; if latched, show explicit release instruction |
| P6 | Deselect | Health |
| P7 | Actions menu | Confirm the current release preview |
| P8 | Back to Stage | Cancel / close menu |

Pad press acts once, release does nothing except reset its edge. The actions menu
is always labelled; record/clear/blackout cannot fire from the base-page meanings.
This is an initial subset. Full controller-only mode/grant editors, cue/palette
browsing/GO, non-intensity release masks, blackout-off preview and show management
are L1 work; all implemented operations are reachable from the ordinary keyboard
now. There is no hidden pad flash/bump implementation.

Planned LED state comes from the same action table: selected page cyan, available
blue, human Hold amber/yellow, confirmed active playback green, pending yellow,
blackout/fault red, unavailable off. Labels/borders distinguish states that share
a color. Never display a pad's action as active from raw input before engine ACK.

mkII's documented local research uses a discrete eight-color SysEx palette, not
arbitrary RGB brightness. It is not MiniLab 3's protocol. Lightdesk implements
**no LED encoder or driver** yet; reuse a reviewed owner/protocol package when
identity is accepted, not a second guessed implementation. Planned worker: one
latest-state mailbox, changed-pad coalescing, ≤20 update batches/s, bounded writes,
input independent of LED failures. Repaint after reconnect; restore only known
transient state. No firmware/preset/global-backlight writes. A successful MIDI
write does not establish physical pad color or state persistence.

## Pickup, encoding and held controls

Profile validation rejects duplicate rotary/pad messages, invalid MIDI bytes and
key/pad note-channel overlap. The fixture profile uses keys channel 1 / anchor 48,
pads channel 10 notes 36..43, rotaries channel 1 CC16..31, all absolute. These are
**invented simulation inputs**, not a claimed MiniLab memory.

Absolute pickup acquires within two raw CC steps of the confirmed value or on
crossing it. No stale previous position crosses a new target after selection,
page, mode, external update or reconnect. The current conservative simulator
rearms after each ACK; production gesture continuity across one's own confirmed
updates is L1 work. `?` in the footer means waiting/unacquired, `=` acquired,
`-` unavailable. Numeric confirmed targets remain in the inspector. Native pickup
arrows and individual mixed-value ranges are planned.

Relative two's complement, binary offset and signed-bit encodings are explicit
per rotary, never guessed from an endless shaft. Deltas clamp to eight steps;
neutral messages do nothing. Ordinary step is 1% or 1 degree, fine step 0.1.
Absolute fine mode uses subsequent raw differences after pickup. No acceleration
beyond that bound. `fine on|off` is explicit; a future held fine modifier must
clear on focus/loss. Repeated relative messages represent repeated turns and
must not be deduplicated as if they were repeated pad presses.

Disconnect/overflow drops backlog, clears fine/pickup and requires button/key
release before a new press. No buffered relative deltas replay after reconnect.
Engine reconnect obtains a fresh snapshot before edits. Unknown SysEx, MIDI clock,
aftertouch and unrelated channels/identities are ignored, with no forwarding.

## Complete keyboard path and acceptance

The current loop's `help` is the complete implemented command map; see the
[workflow](SCREENS.md). It uses standard cooked input, no terminal raw mode or
mouse capture. Future native map: F1–F7 pages, Tab/Shift-Tab field focus,
arrows/value entry, Enter accept, Esc cancel/back; named Select/Group/Record/GO,
Clear Hold/Return/Mode actions reachable from an always-visible menu. Text editing
owns ordinary keypresses so naming cannot fire GO or blackout. Do not show those
native shortcuts as working in today's line loop.

Hardware gate: record model/firmware/serial/preset for both devices, capture all
16 rotations and eight pad press/releases plus supported banks/clicks/Shift,
prove encodings and LED protocol independently, test identical-device assignment,
replug/port swap, focus changes, lost release and simultaneous operation. Observe
that lighting notes/LEDs never reach audio or an instrument. This task did none
of those operations; no automatic device probe or mapping installation is present.
