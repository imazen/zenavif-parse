# zenavif-parse → moved to [imazen/zenavif](https://github.com/imazen/zenavif)

**This repository is archived.** On 2026-07-16 it was absorbed — full git
history included — into the [imazen/zenavif](https://github.com/imazen/zenavif)
cargo workspace, where development continues.

- **Code**: [`zenavif-parse/`](https://github.com/imazen/zenavif/tree/main/zenavif-parse)
  in imazen/zenavif. `git log -- zenavif-parse/` there walks this repository's
  entire lineage (Mozilla mp4parse → kornelski/avif-parse → this fork).
- **Releases and tags**: imported crate-prefixed — this fork's releases as
  [`zenavif-parse-v*`](https://github.com/imazen/zenavif/tags), the inherited
  upstream lineages as `avif-parse-v*` and `mp4parse-v*`. GitHub releases were
  recreated there with provenance notes.
- **Issues / PRs**: please file against
  [imazen/zenavif](https://github.com/imazen/zenavif/issues).
- **The crate is unaffected**: [crates.io](https://crates.io/crates/zenavif-parse) /
  [docs.rs](https://docs.rs/zenavif-parse) /
  [lib.rs](https://lib.rs/crates/zenavif-parse) continue as before; versions
  from 0.7.0 on publish from the workspace.

zenavif-parse is an AVIF container parser (ISOBMFF/MIAF demuxer) with a
zero-copy `AvifParser` API, grid images, animation, and resource limits —
a fork of the battle-tested [kornelski/avif-parse](https://github.com/kornelski/avif-parse).
MPL-2.0, as before.
