# SHR Lightdesk

**The lighting operator surface for GigPies.** Manual look building and playback
come first; ASSIST and AUTO are optional operating modes. It pairs a second
1920×1080 monitor and an independently assigned MIDI keyboard controller with
SHR Desk's audio console on the same Brain. SHR Lux remains the lighting engine.

**Implemented, offline-validated foundation:** eight synthetic fixtures, selection,
programmer/hold/playback/automation state, controller translation, an interactive
keyboard command loop, and seven full-HD screen drafts using the same licensed
Terminus font as SHR Desk. No physical outputs or network connections exist.
The native graphical window, real Lux adapter and physical acceptance are planned.
This is not yet a production lighting console.

```sh
CARGO_INCREMENTAL=0 cargo run --locked -- simulate
CARGO_INCREMENTAL=0 cargo run --locked -- gallery artifacts/screens
# Open artifacts/screens/index.html to inspect the screen drafts.
```

Try `select 11 12`, `set INT 45`, `set RED 15`, `cue record 1`, `go 1 1`,
`clear hold`, `return playback INT`, then `confirm`. `status` explains each
attribute's source; `help` lists the complete keyboard workflow. A mode change
keeps the current look. Show state is in memory and disappears on exit.

`simulate artifacts/live.svg` redraws a state-driven SVG after each command
(refresh that file in a viewer). It is a review backend, not a browser application
or the planned native GUI. The package builds independently with Rust 1.97.1,
edition 2024 and a dependency-free Cargo.lock.

| Read | Purpose |
|---|---|
| [Blueprint](docs/BLUEPRINT.md) | Product, ownership, dual-desk operation and delivery gates |
| [Console study](docs/CONSOLE_STUDY.md) | Manufacturer screens and workflows, versions and limits |
| [Screens and workflow](docs/SCREENS.md) | Full screen map, drafts and a reproducible operator exercise |
| [Controller plan](docs/CONTROLLER.md) | Sixteen rotaries, eight pads, selection layers and separate ownership |
| [Control contract](docs/CONTROL_CONTRACT.md) | Lighting arbitration, release, recovery and engine obligations |
| [Capability matrix](docs/CAPABILITIES.md) | Existing, missing, later and excluded functions |
| [Status](docs/STATUS.md) | Evidence, limitations and concrete next implementation |
| [Development](docs/DEVELOPMENT.md) | Reproducible build/checks, artifacts and publication boundary |

Code MIT; bundled Terminus Font SIL OFL 1.1. See [third-party notices](THIRD_PARTY.md).
