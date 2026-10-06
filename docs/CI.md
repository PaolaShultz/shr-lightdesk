# Continuous integration

[The workflow](../.github/workflows/ci.yml) runs on pushes to `main`, pull
requests and manual dispatch. It uses read-only repository permissions, cancels
superseded runs and selects exact Rust 1.97.1 with locked Cargo dependencies.

Default and native feature jobs run the normal headless suites serially. The native job additionally selects Mesa lavapipe for the bounded ignored software_wgpu_upload_shader_readback test. Its hard-coded ICD path is normalized on the disposable runner. No window or physical GPU is used. Ignored real-provider integration requires separately accepted sibling executables and remains excluded.

The workflow contains the exact reproducible commands. Historical/exhaustive
auditions and benchmarks remain opt-in. No physical audio, MIDI, DMX, playback,
service activation, media download or deployment is part of these checks.
Compilation and synthetic tests do not establish Raspberry Pi hardware acceptance.
Clippy with warnings denied and release builds are not added as new CI gates.
