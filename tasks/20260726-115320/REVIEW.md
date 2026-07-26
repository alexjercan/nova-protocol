# Review: NOVA OS monitor shell and visual treatment

- TASK: 20260726-115320
- BRANCH: feature/nova-os-monitor-shell

## Round 1

- VERDICT: REQUEST_CHANGES
- REVIEWER: out-of-context

- [ ] R1.1 (MAJOR) crates/nova_gameplay/src/hud/drawer.rs:883 - the scanline and vignette/glass nodes are spawned before `spawn_nova_os_terminal_content`, so with equal UI z Bevy renders the later terminal content in front of them. The CRT treatment is therefore behind the content it is supposed to affect, and the marker-only test would still pass. Spawn the overlay nodes after the terminal content or give them a higher local `ZIndex`, then add a regression that proves the overlay stack is above the terminal body.
  - Response: fixed by spawning terminal content before overlays, assigning `ZIndex(0)` to terminal content and `ZIndex(1)` to CRT overlays, and extending `drawer_spawns_single_nova_os_monitor` to assert overlay z is above content z.
- [ ] R1.2 (NIT) crates/nova_gameplay/src/hud/mod.rs:368 - the comment still says drawer-exempt chrome includes "the status strip + keybind hints", but this branch intentionally makes key hints non-exempt. Rewrite this comment to say diagnostic/status chrome only.
  - Response: fixed by rewriting the `lift_exempt_chrome_over_drawer` comment to describe diagnostic/status chrome only.

Pending manual check: compare a real run or screenshot against
`examples/ui/nova_os_terminal_poc.html`.

## Round 2

- VERDICT: APPROVE
- REVIEWER: out-of-context

- [x] R1.1 (MAJOR) crates/nova_gameplay/src/hud/drawer.rs:883 - resolved.
  Verified terminal content now spawns before overlays, overlays carry higher
  local `ZIndex`, and `drawer_spawns_single_nova_os_monitor` asserts overlay z
  above content z.
- [x] R1.2 (NIT) crates/nova_gameplay/src/hud/mod.rs:368 - resolved. Verified
  the stale "status strip + keybind hints" comment was rewritten to
  diagnostic/status chrome only.

No new findings.

Pending manual check: the manual visual comparison against
`examples/ui/nova_os_terminal_poc.html` remains pending user acceptance.
