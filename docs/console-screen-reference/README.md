# Lighting console screen references

Retained design-reference library, reviewed **2026-10-06**: **16 images**, each individually
inspected and accompanied by a detailed description. Open [the offline gallery](index.html)
to browse pictures and all observations together, or use the per-screen Markdown links below.
The gallery is static HTML with local images; it needs no server, build, scripts or external fonts.

[Audio companion library](https://github.com/PaolaShultz/shr-desk/tree/main/docs/console-screen-reference) · [existing console study](../CONSOLE_STUDY.md) ·
[owning screen plan](../SCREENS.md) · [source register](SOURCES.md) · [manifest](manifest.json)

## How to use this library

1. Find a workflow in the comparison table and open the relevant full-resolution figure.
2. Read its visible-layout/control description before drawing conclusions from colour alone.
3. Use the design implications as hypotheses for original SHR Lightdesk layouts, checked against
   the owning screen plan, runtime capabilities and actual controller/display review.
4. Preserve the difference between observed pixels, source-documented context and our proposals.

The relevant planned areas are Overview, Fixtures, Programmer, Playback, Palettes, Patch/Health and future effect editors. No product code or screen plan
is changed by this collection. The historical study's statement that no screenshots were retained
describes the October 4 review; this separately requested October 6 folder now retains selected figures.

## Workflow comparison

| Design question | Screen IDs | Pattern to consider |
|---|---|---|
| Fixture identity/capabilities | 01, 09, 10 | Fixed IDs and capability-aware attributes; never invent a universal fixture personality. |
| Programmer and empty state | 04, 05, 10, 11 | Selection, active data, unset values, explicit zero and disconnected state are different. |
| Output inspection | 06 | Base/post-master/submitted/physical feedback need separate names and provenance. |
| Colour and discrete choices | 07, 08, 13, 14 | Graphic tools need exact values, capability scope and a complete keyboard route. |
| Effects | 02, 03, 11 | Separate shape, timing and contribution; Lux owns evaluation, phase and physical output. |
| Cues and playback | 12, 15, 16 | Active, selected and next cue are separate; selection/scrolling must never imply GO. |

## Screen catalogue

| ID | Console / screen | Native size | Reference role |
|---|---|---|---|
| 01 | [MA Lighting grandMA3 — Fixture Sheet, absolute mode](screens/01-grandma3-fixture-sheet.md) | 1401×684 | Inspect fixture identities and multiple attribute families in a dense table, with a separately chosen value layer. |
| 02 | [MA Lighting grandMA3 — Phaser Editor, Auto view](screens/02-grandma3-phaser-auto.md) | 1600×751 | Inspect a multi-fixture movement effect as both a two-dimensional path and separate parameter curves. |
| 03 | [MA Lighting grandMA3 — Phaser Editor, 2D view](screens/03-grandma3-phaser-2d.md) | 1600×776 | Compare a focused two-dimensional movement editor with the split Auto view of the same example. |
| 04 | [ChamSys MagicQ — Empty Programmer, Levels view](screens/04-magicq-empty-programmer.md) | 1225×253 | Document a deliberate empty programmer state, including the commands that remain visible around it. |
| 05 | [ChamSys MagicQ — Programmer Times](screens/05-magicq-programmer-times.md) | 1074×592 | Show general timing and individual attribute timing in the same programmer context. |
| 06 | [ChamSys MagicQ — Head output values](screens/06-magicq-head-output.md) | 1057×250 | Inspect current per-head attribute values separately from the programmer’s editable data. |
| 07 | [Avolites Titan — HSI/RGB/CMY Attribute Editor](screens/07-avolites-colour-mix.md) | 702×390 | Combine an intuitive colour picker with separately named component controls. |
| 08 | [Avolites Titan — Gobo Attribute Editor](screens/08-avolites-gobo-selection.md) | 461×299 | Show fixture-specific discrete choices as labelled thumbnails rather than unexplained numeric ranges. |
| 09 | [Avolites Titan — Channel Grid](screens/09-avolites-channel-grid.md) | 1147×604 | Inspect fixture attribute values in a table with quick filters for attribute families and fixture type. |
| 10 | [Obsidian ONYX — Programmer, fixture-specific attribute groups](screens/10-onyx-programmer-capabilities.md) | 456×429 | Demonstrate that the programmer’s table structure can differ by fixture type within one selected set. |
| 11 | [Obsidian ONYX — Programmer, populated example](screens/11-onyx-programmer-values.md) | 1749×873 | Inspect a full programmer window where base fixture values and effect details appear together. |
| 12 | [ETC Eos — Blind cue tracking sheet](screens/12-eos-tracking-sheet.md) | 1921×612 | Inspect how cue changes and carried values can be compared across channels in an explicitly Blind view. |
| 13 | [ETC Eos — Color Picker and gel swatches](screens/13-eos-colour-picker.md) | 1429×432 | Compare a chromaticity-based colour editor and named gel/preset swatches with the circular Titan picker. |
| 14 | [ETC Eos — Moving-light Controls](screens/14-eos-ml-controls.md) | 1133×548 | Study an attribute-rich moving-light editor that mixes continuous controls, discrete choices and graphical tools. |
| 15 | [MA Lighting grandMA3 — Sequence Sheet, normal view](screens/15-grandma3-sequence-sheet.md) | 1758×738 | Inspect cue order, trigger/timing fields and the active cue while retaining an optional recipe-detail region. |
| 16 | [Avolites Titan — Virtual Playbacks](screens/16-avolites-virtual-playbacks.md) | 1000×322 | Study an on-screen playback bank that mirrors physical faders and momentary playback buttons. |

## Scope and limits

These are complete available source figures, sometimes cropped windows, teaching annotations or
promotional composites. They are not all full-desktop captures. Small original images remain small;
no invented detail or upscaling fills missing text. Versions identify source material, not latest
firmware. No reference console was physically operated. No screenshot proves our own engine,
measurement, device connection, cue timing or output capability. Manufacturer images remain
third-party reference evidence; see [source status](SOURCES.md).

The ONYX images are historical 3.71-named illustrations, and the MagicQ Times and Eos workbook
figures visibly reuse older builds. Individual notes record that provenance. No current-version
appearance, physical colour reproduction or fixture personality is inferred from those images.
