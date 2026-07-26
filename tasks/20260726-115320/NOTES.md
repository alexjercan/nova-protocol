# Notes: NOVA OS monitor shell and visual treatment

- TASK: 20260726-115320

## What changed

`crates/nova_gameplay/src/hud/drawer.rs` now spawns one inset `NovaOsMonitor`
root instead of two left/right drawer panels. The monitor keeps the existing
drawer toggle, `PauseStates::Drawer` freeze/cursor path, backdrop, z tier,
`DrawerFlightLog` data, objective rebuilding and scroll handling.

The monitor tree ports the visual direction from
`examples/ui/nova_os_terminal_poc.html` into Bevy UI nodes: dark blue-black
casing, physical bezel, green phosphor screen, orange/yellow accent slots,
terminal top bar, scrollable flight log, objectives block, prompt placeholder,
scanline overlay and vignette/glass overlay.

`crates/nova_gameplay/src/hud/mod.rs` now treats lower-left key hints as ordinary
flight chrome while NOVA OS is open. Diagnostic/status chrome tagged
`HudDrawerExempt` still remains visible and z-lifted above the drawer backdrop.

Player-facing docs moved with the behavior in `web/src/wiki/hud.md` and
`CHANGELOG.md`.

## Why this adaptation

The PoC used CSS gradients, pseudo-elements and blend modes. The Bevy version
uses ordinary UI nodes, translucent fills and borders because that is the most
reliable path in the current Bevy UI stack and is easy to assert headlessly. A
custom CRT shader would be more flexible, but this slice needed a stable monitor
shell that future terminal input and app tasks can build on.

The existing drawer log/objective resources stayed in place intentionally. The
next terminal-output task can turn those live feeds into command output without
rebuilding their data plumbing.

## Difficulties

The first red test run failed at compile time because the new monitor markers
did not exist yet, which was the expected test-first failure for the structure
change. I also initially ran two `cargo test` commands in parallel; they
contended on Cargo locks, so the rest of verification ran one Cargo command at a
time.

The web check initially failed because `node_modules` was absent in the sprout
worktree. Running `npm ci` installed the toolchain; `npm run ci` then passed.
`npm ci` reported existing audit vulnerabilities, but they were unrelated to
this change.

## Self-reflection

The plan correctly forced the artifact decision before implementation: a
drawer-owned monitor tree, not shared status chrome or restyled side panels. The
main thing to improve next time is to avoid parallel Cargo commands in a fresh
worktree. The contention cost was small but unnecessary, and this repo already
has enough compile work without adding lock waits.
