# Notes: Match NOVA OS drawer to terminal PoC

- TASK: 20260726-134738

## POC deltas found before editing

`examples/ui/nova_os_terminal_poc.html` uses one full monitor stack:
`.hud -> .bezel -> .screen -> .screen-inner`, then a topbar, one terminal
surface, a prompt row and footer hints. The current `drawer.rs` had the same
outer monitor idea, but the screen body still rendered three permanent blocks:
`FLIGHT LOG`, `OBJECTIVES` and `TERMINAL`. That made the in-game drawer feel
like a dense dashboard inside the monitor instead of the PoC's single terminal.

The PoC also had a separate lit lamp beside the brand, right-side status text,
footer hint text and a shader-like CRT treatment. The current Bevy version had
marker-node scanlines and vignette, but no shader/material overlay.

## What changed

`crates/nova_gameplay/src/hud/drawer.rs` now spawns the PoC-shaped screen:
topbar brand/status, lit lamp, one terminal surface, scrollback viewport,
separate amber `nova>` prompt prefix, input text, completion hint and footer
hints. The visible permanent Flight Log and Objectives panes were removed from
the monitor tree.

The drawer still keeps `DrawerFlightLog`, objective row builders and the data
tests. Those are backing logic for the later command-output task, not visible
NOVA OS panes.

The terminal command registry remains exactly `help` and `clear`. POC-only or
planned commands (`log`, `objectives`, `ship`, `map`, `ship viewer`, `exit`,
`reload`, `repair`) still parse as unknown commands.

## CRT material

The CRT treatment now has a drawer-specific Bevy `UiMaterial` and WGSL shader:
`NovaOsCrtMaterial` in `drawer.rs` and
`assets/shaders/nova_os_crt.wgsl`. Render-capable apps get one
`MaterialNode<NovaOsCrtMaterial>` overlay above the terminal content. Headless
or minimal rigs that do not initialize asset resources still spawn the existing
scanline/vignette UI fallback nodes, so widget-tree tests can run without a GPU.

The shader is intentionally simple: horizontal scanline darkening, green
phosphor tint/glow and edge vignette, with no derivatives so it stays safe on
the WebGL2 path.

## Verification path

- Visual reference: open `examples/ui/nova_os_terminal_poc.html` in a browser.
- Running comparison path: `nix develop --command cargo run --features dev`,
  start a playable scenario, then press `Tab` to open NOVA OS. Compare the
  casing, bezel, topbar, single terminal, prompt row, footer hints and CRT
  treatment against the HTML PoC.
- Screenshot path if a capture is needed later: use the game's debug screenshot
  flow after opening NOVA OS, then compare the captured PNG against the PoC.
  This task records that path but does not add a new automated pixel capture,
  because the repo's GPU capture notes call out software-GPU flakiness for
  layout verification and the task already pins the widget tree headlessly.

## Difficulties

The first test run after the structural edit failed in the useful places: two
tests still expected visible Flight Log/Objectives viewports, and the prompt UI
test expected the old combined `nova> input` text. I converted those tests to
the new contract: single terminal scrollback is the visible viewport, the old
data surfaces are backing-only, and the prompt prefix is its own PoC-style node.

The material path was straightforward because `lock_dwell_ring.rs` already had a
working `UiMaterial` pattern. The only adaptation was making the material asset
optional in the drawer setup observer so minimal headless tests can spawn the
same widget tree without render assets.

## Self-reflection

The plan correctly isolated UI fidelity before command/app work. The main
improvement next time is to update the old tests before the first compile when
the desired red is already known. The compile caught the right failures, but the
test edits were predictable from the plan.
