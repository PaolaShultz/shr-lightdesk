# Capability and integration matrix

2026-10-04. **E** = existing owner implementation inspected/documented;
**S** = implemented/offline-validated surface simulation; **M** = missing for a
usable integrated manual console; **L** = useful later; **X** = outside initial
small-band scope. An E is not fresh hardware certification.

| Capability | Evidence today | Required work / owner | Class |
|---|---|---|---|
| Complete GigPies dual desk | Audio transport/stereo bench plus separate Desk offline foundation | GigPies shared show/roles/compatibility; independent launches/display recovery | E/M |
| Stage selection, groups and attributes | Lightdesk 8 synthetic fixtures; 3 groups; fixture/group key layers, multi/reset | Native focus/editor; real stable IDs and group authoring via Lux | S/M |
| Programmer and source explanation | Touched mask, Hold, simulated base/final, per-attribute winner | Lux authoritative programmer, source trace, group/coherent attribute handling | S/M |
| Manual static looks | Intensity, RGB, mover pan/tilt/zoom declared in synthetic rig | Lux verified personalities and fixture-specific evaluation/calibration | S/M |
| Palettes | 32 fixture-specific copied-value snapshots, record/update/recall | Lux persistence, masks and palette references/update propagation | S/M |
| Cue playback | 32 snapshots, 2 static playback slots, HTP intensity and activation-ordered non-intensity | Lux cue list/GO/back/release, engine timing, reviewed update/tracking policy | S/M |
| Grand master / blackout | Simulated intensity master and persistent in-memory latch | Lux fixture-aware inhibit, persistence, permission and output tests | S/M |
| ASSIST/AUTO/manual hold | Synthetic proposals, scoped bounded grants, explicit return preview | Lux arbitration, proposal reason/expiry, effect/fade continuation and slew limits | S/M |
| Recorded source analysis | Lux `analysis`, `replay`, `simulation`: source activity, energy, kick/pulse candidates | Lux/GigPies live source subscription and freshness; no duplicate analyzer here | E/M |
| Music-aware looks | Lux `show` and `preview`; composition/color/timing research | Lux hold-first direction, role geometry and defined color/intensity representation | E/L |
| Section/downbeat/harmony/solo identification | Explicitly unavailable in Lux's analysis notes | Lux labelled evaluation, if useful; never infer from an active-source meter | L |
| Effects and movement timelines | Lux preview motifs; proposed engine design | Lux effect descriptors, phase/clock, bounded execution, per-attribute overrides | M for basic fades; L for richer FX |
| Flash/bump | Planned scope/release/expiry policy only | Lux transient token/expiry; Lightdesk held-input tracking | L |
| Patch and capability validation | Lux DMX address/range primitives; Lightdesk shows synthetic capabilities | Lux versioned venue patch, overlap/mode/default validation, atomic change | E/M |
| Physical output | Lux USB descriptor evidence and pure uDMX request encoding; no DMX driver | Lux bounded single output worker, known fixtures, arm/blackout/loss/rearm acceptance | M |
| Controller input | DAW/FX learned profiles and pickup references; Lightdesk pure synthetic translator | Verify second controller independently; learn/map both banks, input worker | E/S/M |
| Pad LEDs | Lux opt-in mkII pad preview; DAW proposal; no Lightdesk driver | Sole per-desk owner, protocol capability, coalesced feedback; Lux integrated mode no competing writes | E/M |
| Licensed font / scene grammar | Desk Terminus OFL + MIT primitives adapted narrowly | Native backend measured with both consumers; future small shared package | E/S/M |
| Output health | Offline state ladder and no-driver indicators | Lux transport observation/fault API; fixture feedback remains unknown where unsupported | S/M |
| Show storage / crash recovery | GigPies frozen offline audio identities; no shared live show schema | GigPies manifest; Lux atomic durable show, schema migrations and recovery policy | M |
| Cross-system cues | No lighting/audio cue integration | GigPies named opt-in event scopes, expiry and partial-failure feedback | L |
| Live read-only integration | `Authority` trait with only a Simulator implementation | Agree Lux/GigPies contract, implement adapter and version/refusal fixtures | M |
| Physical two-monitor/controller/load acceptance | Not performed | Separately scoped GigPies H1/H3 stages | M |
| Tracking theatre hundreds of actors / multi-user merge / timecode touring | Research references only | No requirement for first band console | X |
| Media servers, pixel video, lasers, pyrotechnics, rigging automation | No capability | Excluded; no generic macro escape hatch into hardware | X |
| Photometric 3D previsualization and performer tracking | Logical stage draft only | Useful specialized future integration if justified; not a preview claim | L/X |

No sibling implementation task is marked completed merely by its entry here.
The owning backlog is [delivery gates](BLUEPRINT.md#delivery-gates-and-required-sibling-work)
and [GigPies integration](../../gigpies/docs/BRAIN_CONSOLE_PLAN.md). The latter
relative link requires a local sibling checkout; it is not a Cargo dependency.
