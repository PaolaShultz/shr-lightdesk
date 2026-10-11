# Authored scene maps

Lightdesk reads the opted-in `lx06-scenes-v1` / `lx06-scenes-v2` Lux inventory.
Lux owns evidence interpretation, scene selection, transitions, fixture evaluation
and actual arbitration. Earlier LX01–05 schemas remain strict; scene commands are
unavailable on those versions. The sole shared work card is
[GP-SCENES](../../gigpies/docs/archive/records/2026-10-11-gp-scenes.md).

`page scenes` (native F7) shows current/proposed scene intent, measured evidence,
inferred change, elapsed fade progress, authority scope and actual fixture values
with their winning sources. Beat/downbeat alignment remains unavailable. Manual
programmer and Hold can keep actual values different from selected scene intent.
A stale view retains last-known values and disables writer actions.

Lux supplies six editable defaults: Intimate, Floating, Driving, Building,
Open/Lift and Release. Mood/palette describe independently authored RGB values;
lift controls intensity independently. Keys and modes do not prescribe emotions
or colours. Scene role bindings use advertised fixture identities. Movement and
strobe are unsupported; dimmer RGB limitations are visible in owner previews.

## Inspect, edit and preview

Use an explicit endpoint/show/epoch with `lux-control`, or the existing role-bound
native/offscreen frontend. The client never starts Lux or opens lighting hardware.
Native/offscreen writes require the existing live GP09 `lighting-desk` role.
F7 and automatic observation remain available without that role. Supply the
explicit `--role-provider`, `--role-dir` and `--role-request` arguments described
in the README; injected descriptors do not verify physical controller identity.

```text
grant
page scenes
scene map
scene edit driving mood dark
confirm
release-input
scene preview driving
```

`scene edit` prepares an atomic whole-map replacement for exact review. It never
sends an unreviewed edit. Its next map revision is the current owner revision +1;
the existing envelope also binds engine revision, lease, epoch and review context.
Authored-map reviews show exact old/new changed fields and allow up to 30 seconds,
capped to the remaining inherited 25-second connection session; fresh
state, a continuously renewed live lease and the original engine revision remain
required. Reconnect before starting edits when little session time remains. Native
scene drafts survive context loss or failed confirmation; release input, obtain
fresh authority and review again. Import files remain unchanged and can be
explicitly imported again after reconnect. There is no automatic replay.
Actuation/grant reviews retain their existing 2-second deadline.
Native S opens `ID FIELD VALUES...`; Enter reviews, release Enter and inspect all
pages, then Enter confirms. V opens an owner-preview scene-ID editor. Preview is
nonmutating and shows owner target values, actual resolved values and winning
sources, plus unsupported fixture attributes. Its captured map revision remains
visible; request another preview after edits. Reconnect drops captured previews.

Fields: `label`, `family`, `mood`, `palette`, `lift`, `fade_ms`, `min_dwell_ms`,
`confirm_ms`, `variation`, `repetition_penalty`, `min_confidence`; `energy`,
`density`, `trend` take MIN MAX; `compatible_from` takes IDs or `none`;
`rgb` takes ROLE R G B; `intensity` takes ROLE VALUE. Levels are integer logical
0..1000 tenths-percent. RGB values are independent of palette labels.

`scene export PATH` saves the full owner map. Edit role bindings or full authoring
fields there, increment the decimal-string map revision once, then
`scene import PATH` requests exact review. Imports are bounded to 64 KiB, regular
files only, no symlinks; strict integer JSON rejects duplicate keys, excess depth,
unknown fields, duplicate/dangling identities and invalid ranges. Invalid drafts
leave the owner map untouched. Selection/focus/resize/role loss invalidates review;
retained native draft content requires fresh authority and a complete new review.

`scene grant INTENSITY_CAP COLOR_CAP TTL_MS` reviews the exact selected fixtures
and bounded independent intensity/RGB caps; AUTO and ready calibrated Lux evidence
are required. TTL is 10..2000 ms in 10 ms steps with no automatic extension.
Existing `auto revoke` returns to ASSIST and existing manual/Hold workflows remain
available. Map editing revokes automation according to Lux's owner policy.

The full map is authored state; actual restart recovery, durability and source-loss
semantics belong to Lux. UI receipt, ACK and preview establish logical evidence
only. Physical output, controllers, native windows and shared load remain separate
acceptance gates.
