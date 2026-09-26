# Replace TAB NOVA OS with the themed UI and migrate its apps and commands

- STATUS: OPEN
- PRIORITY: 80
- TAGS: v0.15.0,ui,novaos,migration

## User facts
- The owner approves the visual direction of the example-only `ui_app_variants` sketch in PR #77. Ship the new themed UI as the real game interface: TAB opens its panels, never the old NOVA OS computer.
- Keep `:` working as the command entry, with the existing command capabilities. Migrate NOVA OS apps and commands into the new UI/command experience rather than deleting their useful behaviors. This is a breaking replacement: no backward-compatible NOVA OS screen, parallel app hosts, legacy defaults, adapter, or fallback.
- This task is the UI/command migration, not station, cargo, credit, repair-economy, or persistence implementation. Those belong to `20260926-174806`; the sketch's sample stock and transactions are not real game state.

## Agent findings (scout baseline, recheck before implementation)
- `crates/nova_os_ui/src/bindings.rs:48-51` binds `novaos_toggle` to TAB (and gamepad Right Thumb); `terminal/input.rs:49-85` opens the NOVA OS shell on TAB. The command/app registry is `crates/nova_os/src/command.rs:22-52,186-251`, with map and ship apps registered from `crates/nova_os_ui/src/map/mod.rs` and `ship/mod.rs`. `NovaOsUiPlugin` enters AppBuilder at `crates/nova_core/src/lib.rs:445`.
- The themed PR #77 example is in `examples/playable/ui_app_variants.rs`; it uses real 3D map/ship views but example-local inventory, station and repair fixtures. It is NOT a ready-to-wire production screen. The command shell's `:` entry, menu access, shell/app subcommands, pause ownership, input contexts, sound, help/log/objectives, viewer actions, and gamepad behavior need a current consumer audit before choosing interfaces.

## Delivery (requires a separate reviewed code-backed plan)
- Inventory every existing NOVA OS app and command, including nested commands, gameplay effects, input bindings, direct callers, menu/editor entry, content references and tests. Classify each as themed panel, `:` command action, or removed obsolete shell-only behavior. Define how a command that currently launches `map`/`ship` reaches the new panel, and what `commands`, `exit`, `clear` and completion mean without the old monitor. Do not silently remove a capability.
- Specify the replacement screen lifecycle, pause/input/cursor ownership, TAB/Escape/back-out precedence, colon entry inside/outside panels, and command-output presentation. Preserve the working `:` behavior and command effects; replace the old NOVA OS UI, app-host interface and `novaos_toggle` wiring rather than layering an optional new screen over them. Decide gamepad and rebinding semantics before editing.
- Migrate production map/ship functionality, not just the visual sample. The inventory panel may initially be an explicitly non-service surface until real inventory exists; do not present mock credits or trades as gameplay. Delete obsolete CRT/terminal-app paths and update docs, hints, defaults, help and callers in one breaking migration. Do not retain compatibility with unshipped interfaces.

## Verification to plan
- Assert TAB opens/closes the new themed UI and cannot open the old computer; `:` still enters commands and runs representative core, map and ship actions with the same game effects (including menu/editor entry if currently supported). Prove typing, completion and Escape do not trigger panel/flight actions. Prove paused world, input/cursor ownership, rebinds and gamepad navigation with focused tests and a rendered player flow. Verify no old host/default/alias survives; only named, stable behaviors get permanent tests.

## Done when
- The owner approves the exact UI/command migration interfaces and deletion policy; the new themed UI is wired into the real app, `:` command capabilities remain working, TAB no longer opens the old NOVA OS, all obsolete paths are removed, docs are current, and affected checks plus rendered/player proofs pass. PR #77 alone does not complete this task.

## Origin
- Split from `20260824-125943` after owner acceptance of PR #77's UI look on 2026-09-26. Coordinate station/inventory surfaces with `20260926-174806`; do not invent its mechanics here.
