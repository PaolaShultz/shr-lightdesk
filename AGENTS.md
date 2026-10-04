# SHR Lightdesk working agreements

Read README.md, docs/STATUS.md, docs/BLUEPRINT.md and the relevant owning document.
Follow ../AGENTS.md for GigPies peer coordination. Inspect hostname and live Git
state. Preserve interactive sessions and unrelated edits. Existing siblings are
read-only unless separately authorized. GigPies owns final integration; SHR Desk
owns audio UI; Lightdesk owns lighting UI; SHR Lux owns lighting algorithms,
fixture evaluation, cue/effect execution, arbitration and physical output.

Rust 1.97.1, edition 2024, Cargo.lock in source control, no sibling path dependencies.
Use CARGO_INCREMENTAL=0 and normal target/. Review disk space before major builds;
20 GiB free and 5 GiB output are review thresholds, not permission for blanket cleanup.
Keep source, user data and useful evidence. Remove only this task's disposable output.

Distinguish planned, implemented, offline-validated and hardware-verified. The
simulator is disposable acceptance scaffolding, never a production Lux substitute.
No automatic audio/MIDI/DMX/socket/service/display/host-font operations. Physical
output and shared load acceptance require explicit session scope. Never infer
physical light from a preview, descriptor, ACK or successful packet submission.

Fast state, control contract, recovery, controller and rendering tests stay default.
Run focused tests while implementing and the complete normal suite for shared
models/rendering/routing/safety. Long research/exhaustive/hardware tests are opt-in.
Report run and intentionally skipped classes. Keep artifacts/ and user/ ignored.
Preserve superseded owning plans in docs/archive/ and update current status.

No publication or deployment is implied. Before any future commit, inspect live
status and only named staged content; preserve unrelated edits. Fonts retain OFL
and the MIT rendering adaptation retains attribution. See docs/DEVELOPMENT.md.
