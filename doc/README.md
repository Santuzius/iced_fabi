# Documentation

Conventions, test setups, release steps and research results for iced_fabi go here, so every agent and person reads the same.

- [architecture.md](architecture.md) — what each module is for and the dependency direction of the crate.
- [iced-notes.md](iced-notes.md) — the iced 0.14 drawing and layout behaviour the cabinet works around.
- [list-performance.md](list-performance.md) — what the per-frame cost of a big listing is, what iced 0.14 does about it, and what other projects do (research, 2026-10-27).
- [material-icons.md](material-icons.md) — the Material Icon Theme cut in the demo: where each file came from, how the extension mapping was derived, and why it is a vendored cut rather than a submodule or a dependency crate.
