# NOTES: NOVA OS ship computer

## What shipped

- Bare `ship` now LAUNCHES a schematic viewer app (peer to `map`); the old
  status summary moved to the `ship view` CLI subcommand. New arg-bearing verbs:
  `ship section <id>`, `ship reload <id>`, `ship repair <id>`.
- Sections carry a new `SectionCode` (`HULL-1`, `PDC-1`, `TRB-1`, ...) assigned
  per session from kind + a stable index; it is the human/CLI/label handle and
  the Tab-completion candidate set. The grid `EntityId` stays the real identity.
- The viewer renders the ship as unlit green-phosphor proxy BLOCKS built from
  each section's authored `SectionCollider` + local transform, on a dedicated
  `Camera3d` / `RenderLayers` RTT composited through the existing CRT shader
  (the `map` app pattern). Sections are selected via projected clickable UI
  BLIPS labelled with the code, or `[`/`]`; block/blip colour tracks live status.
- Actions (`reload`/`repair`) are instant + free, applied through one
  `ShipSectionCommand` seam.

## The command-invocation pathway (the load-bearing new mechanism)

The pre-existing CLI dispatch printed pre-built rows keyed by a STATIC command
name and dropped parsed arguments - fine for `log`/`ship view`, useless for
`ship repair HULL-3` which needs the typed `<id>` in the ECS.

Added `CommandDispatch::Gameplay` (+ `CommandBody::Gameplay`,
`TerminalCommand::gameplay`). On submit, an arg-bearing gameplay command records
a `NovaOsCommandInvocation { name, args }` on the terminal instead of printing;
`nova_gameplay`'s `apply_ship_cli_commands` drains it with
`take_pending_invocation`, applies it against the live world, and appends the
result rows via `extend_scrollback`. The pure `nova_os` crate stays ECS-free.

Tab completion of the `<id>` argument: the terminal cannot enumerate live
section codes, so `nova_gameplay` injects them with `set_arg_completions`
(keyed by verb); `completion_matches` offers them once the player is past the
command name. `sync_ship_arg_completions` only writes on a real change.

## The queued/resource extension seam (DECISION fork 4)

Actions are arcade-instant now but MUST become the owner's queued/over-time,
hull-resource-costed model without a rewrite. Both entry points (CLI verb ->
invocation, in-app `L`/`P` key -> `ShipSectionCommand` message) converge on
`apply_action_to_section`. That single function is where a future job model
plugs in: instead of mutating `Health`/`SectionAmmo` in place it would enqueue a
job component on the section, check/decrement resources stored in hull sections,
and tick the job while the drawer is closed. A ship inventory panel would live
in this same app. Nothing in `nova_os` or the callers would change.

## Difficulties / diagnosis

- Query conflict: `apply_ship_cli_commands` first used the `ShipSections`
  system-param (which reads `Health`/`SectionAmmo` immutably) alongside
  `&mut Health`/`&mut SectionAmmo` -> scheduler panic. Fixed by resolving the
  code with a query that does NOT touch health/ammo, then reading integrity/ammo
  back through the mutable queries' `get`.
- Query tuple cap: the section query exceeded 15 top-level items; grouped the
  kind-marker columns into a nested `SectionKindQuery` tuple.
- Section codes are assigned from a SYSTEM (not an `Add` observer) because
  sections are inserted by the deferred spawn inside the ship-root `Add`
  observer (`require-default-lands-after-root-add-observer`).
- Input-edge test: pressing `]` then `app.update()` let InputPlugin's PreUpdate
  clear the edge before `ship_input` read it; drove the system with
  `run_system_once` instead so the pressed edge survives
  (`nextstate-input-test-needs-clear-and-two-updates`).
- Real player ships use auto grid-coord `EntityId`s (`cube_i0_j0_k0`), which is
  exactly why a readable `SectionCode` was needed rather than reusing `EntityId`.

## Self-reflection

- Reusing the `map` app wholesale (RTT + orbit + projected-blip picking) removed
  all the render/pick risk up front; the exploration that found that precedent
  before planning was worth it.
- The biggest single decision was making arg delivery a first-class `Gameplay`
  dispatch rather than smuggling args through the snapshot. It kept `nova_os`
  pure and gave the queued-job future a clean seam - worth the extra type.
- Next time: factor the shared RTT/orbit scaffolding out of `nova_os_map.rs` and
  `nova_os_ship.rs` into a small helper; the two files now duplicate the camera,
  `orbit_eye`, `new_rtt_image`, `unlit` and blip-projection shapes. Left as a
  follow-up to avoid destabilising the landed map app in this task.

## Owner manual acceptance (DoD)

The in-game 3D schematic look/feel is a `manual:` DoD item - run
`NOVA_SHOT_DIR=target/reel BCS_AUTOPILOT=1 BCS_REEL=1 cargo run --example
screenshot_nova_os --features debug` and check `nova-os-ship.png` (the example
now drives `ship view`, then launches the `ship` app and captures it).
