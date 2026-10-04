# Third-party notices

## Terminus Font

`assets/fonts/Uni2-TerminusBold24x12.psf` is an unmodified copy of SHR Desk's
bundled PSF, originally decompressed from this host's Debian console-setup-linux
`/usr/share/consolefonts/Uni2-TerminusBold24x12.psf.gz`. No font was installed or
changed on the host. Copyright (c) 2010 Dimitar Toshkov Zhekov, with Reserved
Font Name "Terminus Font". SIL Open Font License 1.1 text is preserved in
[assets/fonts/OFL.txt](assets/fonts/OFL.txt), from Debian's corresponding font
copyright record. Font software remains OFL; application code is MIT.
The license text is byte-for-byte preserved from Desk; its upstream trailing
whitespace has a narrowly scoped `.gitattributes` exemption.

SHA-256: `76cbb7a30085000dab63323650d2296486f8af5528f51eead1519dbfce96b1f9`.

## SHR Desk renderer adaptation

The scene primitives, palette constants, PSF Unicode/glyph parsing and SVG glyph
backend in `src/render.rs` are adapted from the local `../shr-desk/src/render.rs`
read on 2026-10-04. That newly created repository had no commit at review; exact
source SHA-256: `d6add0da9ff6614ccfd2d0a526c26867093a83e7ef3565c2b0bf92da8931a9e2`.
Copyright (c) 2026 GigPies contributors, MIT; its licence is retained as [LICENSE](LICENSE).
The Lightdesk layouts, lighting model, mock authority, controller translation and
raster review backend are new work. No sibling path dependency is used.

This deliberately narrow adaptation lets the two real consumers establish a
common renderer boundary before extracting a maintained shared package. It does
not copy Desk's audio model or Lux's algorithms. No Lux source was copied.

## Research references

Manufacturer manuals/images are linked in [the study](docs/CONSOLE_STUDY.md);
no screenshot, manual, skin or console implementation is redistributed here.
Original generated Lightdesk drafts use synthetic state and the licensed font.
Existing controller protocol evidence remains attributed to Desk/Lux/DAW sources
in [the controller plan](docs/CONTROLLER.md). No hardware output driver or borrowed
private controller configuration is included.
