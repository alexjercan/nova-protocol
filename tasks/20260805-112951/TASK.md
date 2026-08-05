# Point screenshot_nova_os at the web names: terminal and apps beats

- PRIORITY: 67
- TAGS: v0.10.0,screenshot,examples
- ACTIVITY: -
- GATES: -
- RESOLUTION: -
- PARENT: 20260805-105154
- DEPENDS ON: 20260805-112749

## Context

The two NOVA OS shots of the refresh (`20260805-105154`):
`news-090-nova-os-terminal.png` and `news-090-nova-os-apps.png`. Neither has a
manifest slot, so the report lists both as `capturable` with no producer.

`screenshot_nova_os` already exists and does most of the work: it boots a
one-ship range, opens the ship computer with Tab, drives commands through the
real keyboard path, and captures the screen. It was built for an HTML fidelity
comparison against `web/design/nova_os_terminal_poc.html` (task
20260726-180807), not for the website - so its beats are diagnostic, and its
output names are not the web names.

Smallest task of the six. Mostly naming and one new beat for the apps shot
(`crates/nova_gameplay/src/hud/nova_os_map/` is the map app; the terminal shot
already exists in spirit).

Depends on the photo kit only for sequencing - the look here is CRT chrome, not
a lit 3D set.

## Steps

- [ ] Decide which existing beat becomes `news-090-nova-os-terminal` (the
      terminal with a command run, inline completion visible) and re-name it.
- [ ] Add an apps beat: open the map/app surface so the shot shows NOVA OS as
      more than a text prompt.
- [ ] Keep or drop the diagnostic beats deliberately: the fidelity-comparison
      shots served task 20260726-180807 and may have no reader left. Decide:
      keep, or delete with the reason recorded.
- [ ] Give both web names FIGURES slots in `scripts/gen-web-screenshots.py`
      naming `screenshot_nova_os`.
- [ ] Hand it to the owner: run plainly, open the computer, verdict on
      contrast and the CRT treatment.

## Definition of Done

- The example builds and the catalog agrees with disk.
  (cmd: `nix develop --command cargo check --examples --features debug`)
- Every beat resolves headless - a step that stalls fails the run naming
  itself. (test: `screenshots_reach_playing_without_panic`)
- The report names `screenshot_nova_os` for both shots, which have no slot
  today.
  (cmd: `nix develop --command python3 scripts/gen-web-screenshots.py --report`)
- The owner opens the computer and accepts both states as good enough to shoot.
  (manual: `cargo run --example screenshot_nova_os --features debug`, no NOVA_REEL)

## Notes

- No PNG is captured or committed in this task.
- The CRT treatment already applies its own bloom
  (`crates/nova_gameplay/src/hud/nova_os/crt.rs`), so the scene's lighting is
  nearly irrelevant to these two shots.
