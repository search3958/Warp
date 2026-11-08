# Vendored stb_truetype_rust

This directory vendors the `stb_truetype_rust` 1.26.1 source supplied in the
BARAM reference archive.

Upstream: https://github.com/StbRust/stb_truetype_rust
License declared by upstream Cargo.toml: Unlicense.

The crate is kept local so MB5's glyph rasterization is independent of
platform font frameworks. Its only Cargo dependency is `libm = "0.2"`.
The small C-runtime allocation shim uses the target C runtime on Unix and
Windows; it does not use any platform text or graphics API.
