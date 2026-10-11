# Capability and integration matrix

## Current software boundary

Capabilities describe supported contracts, not task status. Plans and progress
live only in [the owning plan](GIGPIES_IMPLEMENTATION.md); dated evidence is in
[Status](STATUS.md). A mock capability is never a physical output claim.

| Capability | Current implementation | Remaining boundary |
|---|---|---|
| Lux authority and control | Strict C-LIGHT decoding, writer leases, programmer/Hold, cue/palette/playback commands and engine-owned release previews | Trusted private local IPC; physical output unknown |
| Timing and recovery | LX04 timed release and durable restart state consumed from Lux | No automatic scene replay or physical rearm |
| Analysis | LX05 calibration, bounded AUTO and retained-contribution provenance; opt-in music v1/v2 chord/key/Camelot, fifths motion and tentative section metadata from Lux | Beat/downbeat remain unavailable; legacy harmony field remains null; estimates do not establish musical truth |
| Native surface | Optional winit/wgpu frontend, separate bounded provider/input queues, semantic actions and complete material review | Software/offscreen validation; physical display/controller acceptance separate |
| Role ownership | Explicit GP09 process-held bindings and loss fencing | Injected descriptors; hardware discovery/profile qualification separate |

## Provider boundaries

Fixture personalities/venue patch authoring, fuller cue lists and effects/movement
must be implemented and advertised by Lux before their production UI is enabled.
Physical output depends on Lux LX-06. Real controller/display/LED acceptance and
whole-show load use the shared GP-H2/H3 cards. Music metadata requires the explicit LX05 music schema; beat/downbeat remain unavailable.
No source meter or successful command implies physical light.

[Historical foundation matrix](archive/tracking-before-2026-10-09/CAPABILITIES.md) is retired as a current assessment.

LX06 scene maps: strict bounded versioned consumption, real owner-backed map edits,
nonmutating previews and reviewed intensity/RGB scopes are implemented software.
See [Scene maps](SCENE_MAP.md); physical acceptance is separate.
