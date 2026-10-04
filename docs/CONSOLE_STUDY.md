# Lighting-console screen and workflow study

Reviewed 2026-10-04. This is a bounded design study, not a feature-parity or buying
comparison. Primary manufacturer manuals and actual manufacturer screenshots were
opened. No commercial software was installed or console physically operated.
Version numbers below identify the material reviewed, not a claim of latest firmware.
Conclusions labelled **Lightdesk** are our design choices.

## grandMA3 — large-system organization

Reviewed MA Lighting **grandMA3 2.4**, [Fixture Sheet](https://help.malighting.com/grandMA3/2.4/HTML/operate_fixture_sheet.html)
and its [actual fixture-sheet image](https://help.malighting.com/grandMA3/2.4/Storage/grandma3-user-manual-publication/img/window_fixture-sheet_absolute-mode.PNG).
The screenshot visibly separates fixture name/ID columns from Dimmer, PanTilt,
Gobo, RGB and Beam; layers occupy a stable bottom strip. Selection and active
values have different markings. It supports a dense attribute table without
pretending every fixture has every attribute.

The [Programmer chapter](https://help.malighting.com/grandMA3/2.4/HTML/operate_programmer.html)
distinguishes selection, active values and deactivated values. Successive Clear
presses deselect, deactivate recordability, then release. Deactivated values may
still affect output. Freeze changes programmer priority relative to sequences;
we must not assume every console's programmer has unconditional priority. Blind
and Preview also differ; entering Blind with data can change output.

[Presets](https://help.malighting.com/grandMA3/2.4/HTML/presets.html) are references,
so updates can reach cues using them. [Store Cues](https://help.malighting.com/grandMA3/2.4/HTML/cue_store.html)
connects programmer values to a sequence/executor and makes overwrite/merge choices
explicit. Operators select fixtures, edit attributes or recall presets, then
store into a named playback structure.

**Lightdesk:** borrow per-attribute state and stable identity/layer presentation.
Use explicit Deselect/Clear to Hold/Return actions instead of an invisible Clear
press count. Palette references and engine-generated release previews need Lux.
Multi-user programmers, recipes, worlds and large phaser editing are unnecessary
for the first band rig. The screenshot is one sheet, not a full multi-monitor
show review; no claims about tested MA timing or our ability to reproduce it.

## ChamSys MagicQ — compact programming and output inspection

Reviewed **MagicQ 1.9.7.x**, [Programmer](https://docs.chamsys.co.uk/magicq/1.9.7.x/manual/programmer.html),
including actual [programmer screenshot](https://docs.chamsys.co.uk/magicq/1.9.7.x/manual/_images/progwindow.png).
Levels, Times and FX occupy adjacent top tabs with stable surrounding actions.
An empty programmer is explicitly identified. The workflow selects heads, edits
values, records to a playback, then clears the programmer to test that playback.
Palette references, activation masks and cue times affect what is stored.
The programmer overrides playbacks; clearing is more than deselection.

[Output Windows](https://docs.chamsys.co.uk/magicq/1.9.7.x/manual/outputs_windows.html)
and the [inspected head-output screenshot](https://docs.chamsys.co.uk/magicq/1.9.7.x/manual/_images/outputsheads.png)
show identity plus compact attribute columns. The manual separates value, raw,
controlling-playback and cue-ID views. Head values precede patch adjustments;
DMX view follows those adjustments. The [Cue Stack Options](https://docs.chamsys.co.uk/magicq/1.9.7.x/manual/cue_stack_settings.html)
make fader activation/release and HTP/LTP control configurable. A fader position
alone therefore cannot explain ownership.

**Lightdesk:** keep Programmer and Output/source inspection distinct, but put
both within easy reach. Label base, post-master, transport-submitted and physical
feedback separately. Adopt a small declared playback policy rather than exposing
all configuration permutations. Separate fixture selection from GO. Lux must
supply timing, attribute masks and source traces. Pixel mapping and remote
multi-console features are later/outside initial scope. Local HTML initially
returned 403; browser-indexed manual text and direct image downloads succeeded.
Screenshots were inspected locally; no installed MagicQ project was exercised.

## Avolites Titan — attribute editing and busking

Reviewed **Titan 17.0** [Changing Fixture Attributes](https://manual.avolites.com/docs/17.0/controlling-fixtures/changing-fixture-attributes/),
including the [actual HSI/RGB/CMY editor image](https://manual.avolites.com/assets/images/Attribute-Editor-HSI-RGB-CMY-7b1c54520b9a1a19f6ff9f58eb750be9.png).
The color disk sits beside labelled component sliders; the editor combines a
spatial picker with exact attribute controls. Available attributes follow the
selected fixture; wheels are visibly labelled and programmer membership marked.

[Viewing and Editing Fixture Values](https://manual.avolites.com/docs/17.0/controlling-fixtures/viewing-and-editing-fixture-values/)
provides Channel Grid filters and levels/playbacks/shapes/times views. We read this
chapter but did not inspect every linked image. [Selecting Fixtures](https://manual.avolites.com/docs/controlling-fixtures/)
was reviewed in the unversioned manual (page updated July 8, 2026): Clear options
include selection/programmer precedence and freeze-current versus return-to-playback
for non-intensity attributes. [Playback controls](https://manual.avolites.com/docs/running-the-show/playback-controls/)
from that same current documentation describes palettes applied to selected
fixtures as programmer overrides, while Quick Palettes have different takeover
behavior. These current pages are supplementary, not silently pinned to 17.0.

**Lightdesk:** make color selection graphical only where capabilities justify it,
with units and a complete keyboard path. Keep programmer versus playback source
visible while busking. Never copy ambiguous context-dependent palette recall;
our initial Recall explicitly enters the programmer. Engine-side color conversion,
palette reference semantics, time and release are missing Lux capabilities.
Virtual dimmers, fixture modes and control/reset channels cannot be guessed by
the surface. Large pixel/video/timeline programming is unnecessary here.

## ETC Eos — explicit command scope and return to background

Reviewed ETC **Eos v3.3 Level 1 Essentials**, [manufacturer training PDF](https://www.etcconnect.com/uploadedFiles/Main_Site/Documents/Public/Video_Tutorial/Eos_Family_L1_Essentials_v3.3.pdf).
PDF leaf 8 (printed page 8) was rendered and inspected: the channel tiles separate
number, intensity and focus/color/beam/effect data. The display teaching pages
contrast Live and Blind and retain a visible command line. This is tile/layout
teaching evidence, not a whole-show screenshot. The training workflow uses explicit
channel ranges, levels, Record/Cue, timing, palettes and playback.

The online [Sneak chapter](https://www.etcconnect.com/WebDocs/Controls/EosFamilyOnlineHelp/en/Content/08_Manual_Control/Sneak.htm)
reported **v3.3.10 Rev A** when opened (the search excerpt still said 3.3.6): selected
manual values can return to background cue/submaster state with timing, or home
if there is no background. Making data unmanual can retain values while changing
recordability. [About Address](https://www.etcconnect.com/WebDocs/Controls/EosFamilyOnlineHelp/en/Content/17_Using_About/%5BAbout%5D_Address.htm)
exposes patch identity, output source and available device faults; feedback depends
on supported connected equipment.

**Lightdesk:** show scope before executing, and show the destination of a release.
Our Clear to Hold takes inspiration from retaining values while separating
recordability; our naming/policy is deliberately our own. Timed return and device
feedback need Lux. Blind cue editing is useful later but must not be confused with
an offline simulator. Theatre tracking, many cue parts and elaborate marking are
not necessary for initial band use. Failed direct web screenshot extraction was
replaced with local rendering of the same official PDF; no paid material was used.

## Obsidian ONYX — compact fixture-aware programmer

Reviewed the current, **unversioned ONYX web manual**, [The Programmer](https://support.obsidiancontrol.com/Content/Onyx_Manual/Programming/Manipulating_Fixtures/The_Programmer.htm),
with its actual [Programmer image](https://support.obsidiancontrol.com/Content/assets/images/Screenshots/3.71/Programmer%203.71.png).
The image filename is **3.71**; it is historical illustration reused by the current
manual, not proof of a 2026 build's appearance. Fixture types have different
attribute columns; cells show named presets and a dash for null. The last selected
fixture determines editing context. Hidden/filter-excluded data can still be in
the programmer and recordable.

[Clearing Attributes](https://support.obsidiancontrol.com/Content/Onyx_Manual/Programming/Manipulating_Fixtures/Clearing_Attributes.htm)
documents single-clear deselection, double-clear full release and scoped options.
[Recording a Simple Cue](https://support.obsidiancontrol.com/Content/Onyx_Manual/Playback/Cues_and_Cuelists/Recording_a_simple_cue.htm)
records the programmer into a cuelist attached to a playback. [Re-recording](https://support.obsidiancontrol.com/Content/Onyx_Manual/Playback/Cues_and_Cuelists/Modifying_Cues/Re-Recording_a_Cue.htm)
separates merge, replace and cancel. [Creating Cuelists](https://support.obsidiancontrol.com/Content/Onyx_Manual/Playback/Cues_and_Cuelists/Creating_Cuelists.htm)
explains tracking and the difference between null and a stored value.

**Lightdesk:** never render an unsupported/null attribute as a healthy zero.
Show selected scope and complete record mask even when a view filters details.
Use a simple cue-store/playback relationship first. Lux must implement real cue
execution, references and editing semantics; the mock's copied static cues do not
meet ONYX tracking behavior. Media/pixel engines and complex cuelist types are
unneeded. Hardware wing ergonomics and output/network screens were not operated.

## Task-by-task design synthesis

These are our proposed workflow decisions, derived from the reviewed chapters;
none is a claim of commercial feature parity.

| Operator task | Lightdesk choice | Dependency / deliberately deferred |
|---|---|---|
| Select fixtures/groups | Stable IDs, stage/table selection, explicit key layer and MULTI/reset | Group authoring and ordered selection via Lux; no Locate-on-select |
| Set intensity/color/position | Named capability fields and physical 2×8 rotary legend; unsupported disabled | Real color/position units, coupled attributes and safe ranges from Lux |
| Build a look | Live programmer with touched mask and per-attribute source, including 0% | Blind authoring separate future state, not implicit mode toggle |
| Store/recall palettes | Explicit record mask/destination; recall into programmer | Referenced palettes and fixture-type reuse need engine semantics |
| Record/edit/play cues | Record separate from GO and release; show cue versus playback instance | Tracking, list timing, merge/cue-only update and engine-owned execution |
| Masters/overrides | Post-resolution intensity master; distinct blackout; visible human Hold | Fixture-aware inhibit, timed releases and momentary bump leases |
| Find who controls output | Winning source, base/final, proposal and hold; competing sources later | Lux source trace, transport status, physical feedback where available |
| Clear safely | Deselect, Clear to Hold, Return to Playback/Auto are separate names | Engine preview, revision token and timed transition for live use |
| Find patch/faults | Capability/patch page and persistent no-driver/stale/output-health state | Validated personalities, transport arm/fault/recovery and device telemetry |

Native stage geometry, color picker, movement preview and cue timeline help when
they explain scope or timing. They cannot establish coverage, spectrum, glare,
dimmer quality, stage position calibration or physical DMX delivery. Prefer a
readable band look with deliberate held changes, consistent with Lux's existing
composition research, over automatic motion to make the display appear active.

No manufacturer image/manual is bundled into the product or committed. Temporary
source downloads were used for the visual review, then removed. Exact source
links and the inspected version/image/page limits above remain reproducible.
